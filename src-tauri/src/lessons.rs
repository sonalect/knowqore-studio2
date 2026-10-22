//! The Lessons table: sorted windows of the Vault's Lessons, as the frontend
//! scrolls through them.
//!
//! The frontend asks for one window at a time (`lessons_page`): a sort, an
//! offset, and a number of rows, and gets back those rows and the total. It
//! never holds every Lesson, and the store's full records stay in the store.
//!
//! The contract lists the Lessons only as one stream (`Lessons`), in the
//! store's own order, with no count, sort, or window. Until it has a call for
//! that, this module stands in for it: it reads the stream once per open
//! store, keeps of each Lesson only what a row shows ([`LessonRow`]), and
//! sorts and cuts the windows itself. When the store answers windows,
//! `lessons_page` calls it instead, and the frontend does not change.

use std::cmp::Ordering;
use std::sync::{Mutex, PoisonError};

use futures_lite::StreamExt as _;
use knowqore_stores::{Empty, Error, Lesson, StoreAsync as _, StoreRef};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};

use crate::workspace::OpenStore;

/// The most rows one page holds; a larger `limit` gets this many.
pub const MAX_PAGE_ROWS: usize = 500;

/// How many characters of the statement a row keeps: what a table cell can
/// show, not the whole statement (up to 1024 bytes), so the rows of a large
/// Vault stay small.
const STATEMENT_CHARS: usize = 240;

/// One row of the Lessons table: what the table shows of a Lesson. The store
/// checks that a Lesson it reads has every field the row takes.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonRow {
    /// The Lesson id.
    pub id: String,
    /// The start of the statement, ending in `…` when it was cut.
    pub statement: String,
    /// `asset`, `practice`, `decision`, `pitfall`, `project_fact`, or
    /// `preference`.
    pub kind: String,
    /// `candidate`, `active`, or `retired`.
    pub status: String,
    /// `global` or `local`.
    pub transfer: String,
    /// `prefer` or `avoid`.
    pub polarity: String,
    /// When the record was written, RFC 3339 UTC.
    pub updated_at: String,
}

impl LessonRow {
    /// The row of `lesson`.
    fn of(lesson: &Lesson) -> Self {
        let full = lesson.statement.as_deref().unwrap_or_default();
        let mut statement: String = full.chars().take(STATEMENT_CHARS).collect();
        if statement.len() < full.len() {
            statement.push('…');
        }
        Self {
            id: lesson.id.clone().unwrap_or_default(),
            statement,
            kind: lesson.kind.clone().unwrap_or_default(),
            status: lesson.status.clone().unwrap_or_default(),
            transfer: lesson.transfer.clone().unwrap_or_default(),
            polarity: lesson.polarity.clone().unwrap_or_default(),
            updated_at: lesson.updated_at.clone().unwrap_or_default(),
        }
    }
}

/// The column the rows are sorted by.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LessonSortKey {
    /// The statement, ignoring case.
    Statement,
    /// The kind.
    Kind,
    /// The status.
    Status,
    /// The transfer.
    Transfer,
    /// When the record was written.
    UpdatedAt,
}

/// Which way the rows are sorted.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SortDirection {
    /// Smallest first.
    Ascending,
    /// Largest first.
    Descending,
}

/// The window of rows the table asks for.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LessonsQuery {
    /// The column to sort by. Rows equal in it are ordered by id, so a sort
    /// is the same on every call.
    pub sort_key: LessonSortKey,
    /// Which way to sort.
    pub direction: SortDirection,
    /// How many sorted rows come before the window.
    pub offset: usize,
    /// How many rows the window holds, at most [`MAX_PAGE_ROWS`].
    pub limit: usize,
}

/// One window of sorted rows.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonsPage {
    /// How many Lessons there are in all.
    pub total: usize,
    /// The position of the first row among all of them; at most `total`.
    pub offset: usize,
    /// The rows of the window: fewer than asked for at the end.
    pub rows: Vec<LessonRow>,
}

/// The rows of every Lesson of one store, in the last order asked for.
struct Lessons {
    rows: Vec<LessonRow>,
    order: Option<(LessonSortKey, SortDirection)>,
}

impl Lessons {
    fn new(rows: Vec<LessonRow>) -> Self {
        Self { rows, order: None }
    }

    /// Reads the row of every Lesson of `store`, one Lesson at a time.
    async fn read(store: &StoreRef) -> Result<Self, Error> {
        let mut stream = store.to_async_offload()?.lessons(Empty::default()).await?;
        let mut rows = Vec::new();
        while let Some(lesson) = stream.next().await {
            rows.push(LessonRow::of(&lesson?));
        }
        Ok(Self::new(rows))
    }

    /// The window `query` asks for; sorts the rows first when it asks for
    /// another order than the last one.
    fn page(&mut self, query: &LessonsQuery) -> LessonsPage {
        let order = (query.sort_key, query.direction);
        if self.order != Some(order) {
            self.rows
                .sort_unstable_by(|a, b| compare(a, b, query.sort_key, query.direction));
            self.order = Some(order);
        }
        let total = self.rows.len();
        let start = query.offset.min(total);
        let end = start
            .saturating_add(query.limit.min(MAX_PAGE_ROWS))
            .min(total);
        LessonsPage {
            total,
            offset: start,
            rows: self.rows.get(start..end).unwrap_or_default().to_vec(),
        }
    }
}

/// The order of two rows by `key` and `direction`, then by id.
fn compare(a: &LessonRow, b: &LessonRow, key: LessonSortKey, direction: SortDirection) -> Ordering {
    let by_key = match key {
        LessonSortKey::Statement => folded(&a.statement).cmp(folded(&b.statement)),
        LessonSortKey::Kind => a.kind.cmp(&b.kind),
        LessonSortKey::Status => a.status.cmp(&b.status),
        LessonSortKey::Transfer => a.transfer.cmp(&b.transfer),
        LessonSortKey::UpdatedAt => a.updated_at.cmp(&b.updated_at),
    };
    let ordered = by_key.then_with(|| a.id.cmp(&b.id));
    match direction {
        SortDirection::Ascending => ordered,
        SortDirection::Descending => ordered.reverse(),
    }
}

/// `text` in lower case, without allocating.
fn folded(text: &str) -> impl Iterator<Item = char> + '_ {
    text.chars().flat_map(char::to_lowercase)
}

/// The rows of the open store's Lessons, once read, with the store they were
/// read from.
#[derive(Default)]
pub struct LessonsBuffer(Mutex<Option<(StoreRef, Lessons)>>);

impl LessonsBuffer {
    /// The window of the rows read from `store`, when they are read.
    fn page_of(&self, store: &StoreRef, query: &LessonsQuery) -> Option<LessonsPage> {
        let mut buffer = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        match buffer.as_mut() {
            Some((read_from, lessons)) if read_from == store => Some(lessons.page(query)),
            _ => None,
        }
    }

    /// Keeps the rows read from `store` in place of any others, and answers
    /// the window from them.
    fn keep(&self, store: StoreRef, mut lessons: Lessons, query: &LessonsQuery) -> LessonsPage {
        let page = lessons.page(query);
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some((store, lessons));
        page
    }
}

/// Tauri command `lessons_page`: one sorted window of the Lessons of the open
/// workspace and their total. The first call after a workspace opens reads
/// the Lessons; it fails with the store's error (its code first) when they
/// cannot be read, and when no workspace is open.
#[tauri::command]
pub async fn lessons_page(
    open_store: State<'_, OpenStore>,
    buffer: State<'_, LessonsBuffer>,
    query: LessonsQuery,
) -> Result<LessonsPage, String> {
    let Some(store) = open_store.current() else {
        return Err("No workspace is open.".to_owned());
    };
    if let Some(page) = buffer.page_of(&store, &query) {
        return Ok(page);
    }
    let lessons = Lessons::read(&store)
        .await
        .map_err(|error| error.to_string())?;
    Ok(buffer.keep(store, lessons, &query))
}

/// Registers the buffer of the Lessons' rows.
pub fn manage(app: &AppHandle) {
    app.manage(LessonsBuffer::default());
}

#[cfg(test)]
mod tests {
    use knowqore_stores::Lesson;

    use super::{
        LessonRow, LessonSortKey, Lessons, LessonsQuery, MAX_PAGE_ROWS, STATEMENT_CHARS,
        SortDirection,
    };

    fn row(id: &str, statement: &str, kind: &str, updated_at: &str) -> LessonRow {
        LessonRow {
            id: id.to_owned(),
            statement: statement.to_owned(),
            kind: kind.to_owned(),
            status: "active".to_owned(),
            transfer: "global".to_owned(),
            polarity: "prefer".to_owned(),
            updated_at: updated_at.to_owned(),
        }
    }

    fn fixture() -> Lessons {
        Lessons::new(vec![
            row("l3", "beta", "pitfall", "2026-10-02T00:00:00.000Z"),
            row("l1", "Alpha", "practice", "2026-10-03T00:00:00.000Z"),
            row("l2", "gamma", "practice", "2026-10-01T00:00:00.000Z"),
            row("l4", "alpha", "asset", "2026-10-03T00:00:00.000Z"),
        ])
    }

    fn query(sort_key: LessonSortKey, direction: SortDirection) -> LessonsQuery {
        LessonsQuery {
            sort_key,
            direction,
            offset: 0,
            limit: 10,
        }
    }

    fn ids(lessons: &mut Lessons, query: &LessonsQuery) -> Vec<String> {
        lessons
            .page(query)
            .rows
            .into_iter()
            .map(|row| row.id)
            .collect()
    }

    #[test]
    fn newest_first_orders_by_the_time_of_writing_then_by_id() {
        let mut lessons = fixture();
        let newest = query(LessonSortKey::UpdatedAt, SortDirection::Descending);
        assert_eq!(ids(&mut lessons, &newest), ["l4", "l1", "l3", "l2"]);
    }

    #[test]
    fn the_statement_sorts_ignoring_case() {
        let mut lessons = fixture();
        let ascending = query(LessonSortKey::Statement, SortDirection::Ascending);
        assert_eq!(ids(&mut lessons, &ascending), ["l1", "l4", "l3", "l2"]);
    }

    #[test]
    fn every_column_sorts_both_ways() {
        for key in [
            LessonSortKey::Statement,
            LessonSortKey::Kind,
            LessonSortKey::Status,
            LessonSortKey::Transfer,
            LessonSortKey::UpdatedAt,
        ] {
            let mut lessons = fixture();
            let mut ascending = ids(&mut lessons, &query(key, SortDirection::Ascending));
            let descending = ids(&mut lessons, &query(key, SortDirection::Descending));
            ascending.reverse();
            assert_eq!(ascending, descending, "{key:?}");
        }
    }

    #[test]
    fn a_window_holds_the_rows_from_its_offset_and_the_total() {
        let mut lessons = fixture();
        let window = LessonsQuery {
            offset: 1,
            limit: 2,
            ..query(LessonSortKey::Kind, SortDirection::Ascending)
        };
        let page = lessons.page(&window);
        assert_eq!(page.total, 4);
        assert_eq!(page.offset, 1);
        let ids: Vec<_> = page.rows.iter().map(|row| row.id.as_str()).collect();
        assert_eq!(ids, ["l3", "l1"]);
    }

    #[test]
    fn a_window_past_the_end_is_empty_and_starts_at_the_total() {
        let mut lessons = fixture();
        let window = LessonsQuery {
            offset: 9,
            ..query(LessonSortKey::Kind, SortDirection::Ascending)
        };
        let page = lessons.page(&window);
        assert_eq!((page.total, page.offset, page.rows.len()), (4, 4, 0));
    }

    #[test]
    fn a_window_holds_at_most_the_largest_page() {
        let rows = (0..MAX_PAGE_ROWS + 5)
            .map(|index| row(&format!("l{index:04}"), "s", "asset", "t"))
            .collect();
        let mut lessons = Lessons::new(rows);
        let window = LessonsQuery {
            limit: usize::MAX,
            ..query(LessonSortKey::Kind, SortDirection::Ascending)
        };
        assert_eq!(lessons.page(&window).rows.len(), MAX_PAGE_ROWS);
    }

    #[test]
    fn a_row_keeps_the_start_of_a_long_statement() {
        let mut lesson = Lesson {
            id: Some("l1".to_owned()),
            statement: Some("é".repeat(STATEMENT_CHARS + 1)),
            ..Lesson::default()
        };
        let row = LessonRow::of(&lesson);
        assert_eq!(row.statement.chars().count(), STATEMENT_CHARS + 1);
        assert!(row.statement.ends_with('…'));

        lesson.statement = Some("short".to_owned());
        assert_eq!(LessonRow::of(&lesson).statement, "short");
    }
}
