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
//! sorts, filters by a node of the tree of containers, and cuts the windows
//! itself. When the store answers windows,
//! `lessons_page` calls it instead, and the frontend does not change.

use std::cmp::Ordering;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Mutex, PoisonError};

use futures_lite::StreamExt as _;
use knowqore_stores::{Empty, Error, Lesson, LessonKey, StoreAsync as _, StoreRef};
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
#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
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
    /// The key of a node of the tree ([`LessonTreeNode::key`]): only the
    /// Lessons seen under it are listed and counted. `None` lists every
    /// Lesson; a key no node has lists none.
    #[serde(default)]
    pub node: Option<String>,
    /// Which values the Lessons must have; empty narrows nothing.
    #[serde(default)]
    pub filters: LessonFilters,
    /// The keys of the groups folded shut ([`PageItem::Group::key`]). Groups
    /// exist only when `node` names a node that has children; the keys of a
    /// folded group's Lessons are left out of the page, its header stays.
    #[serde(default)]
    pub collapsed: Vec<String>,
}

/// The values a Lesson may have, per field. The values of one field are
/// joined by "or", the fields by "and"; an empty field does not narrow.
#[derive(Clone, Debug, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LessonFilters {
    /// The kinds wanted.
    #[serde(default)]
    pub kind: Vec<String>,
    /// The statuses wanted.
    #[serde(default)]
    pub status: Vec<String>,
    /// The transfers wanted.
    #[serde(default)]
    pub transfer: Vec<String>,
    /// The polarities wanted.
    #[serde(default)]
    pub polarity: Vec<String>,
}

impl LessonFilters {
    /// Whether no field narrows.
    fn is_empty(&self) -> bool {
        self.kind.is_empty()
            && self.status.is_empty()
            && self.transfer.is_empty()
            && self.polarity.is_empty()
    }

    /// Whether `row` has one of the values wanted in every field that asks.
    fn admits(&self, row: &LessonRow) -> bool {
        let one_of = |wanted: &[String], value: &str| {
            wanted.is_empty() || wanted.iter().any(|item| item == value)
        };
        one_of(&self.kind, &row.kind)
            && one_of(&self.status, &row.status)
            && one_of(&self.transfer, &row.transfer)
            && one_of(&self.polarity, &row.polarity)
    }
}

/// How many Lessons have one value of a field.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FacetCount {
    /// The value, as the contract spells it.
    pub value: String,
    /// How many Lessons have it.
    pub lessons: usize,
}

/// The values the Lessons under a node have, with how many Lessons have each.
/// Every value that occurs is listed, in no particular order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonFacets {
    /// The kinds.
    pub kind: Vec<FacetCount>,
    /// The statuses.
    pub status: Vec<FacetCount>,
    /// The transfers.
    pub transfer: Vec<FacetCount>,
    /// The polarities.
    pub polarity: Vec<FacetCount>,
}

/// The counts of the values `values` yields.
fn facet_counts<'a>(values: impl Iterator<Item = &'a str>) -> Vec<FacetCount> {
    let mut counts: BTreeMap<&str, usize> = BTreeMap::new();
    for value in values {
        *counts.entry(value).or_default() += 1;
    }
    counts
        .into_iter()
        .map(|(value, lessons)| FacetCount {
            value: value.to_owned(),
            lessons,
        })
        .collect()
}

/// One item of the table: a Lesson's row, or the header of a group of them.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(
    tag = "type",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum PageItem {
    /// The header that opens a group: the Lessons under one child of the
    /// chosen node.
    Group {
        /// The child node's key; the chosen node's own key for the group of
        /// Lessons seen in no child.
        key: String,
        /// The child's label, or the chosen node's own label.
        label: String,
        /// How many Lessons the group holds after the filters, folded or not.
        lessons: usize,
        /// Whether the group is folded shut: its Lessons are not listed.
        collapsed: bool,
    },
    /// A Lesson of the group above it, or of the whole list without groups.
    Lesson(LessonRow),
}

/// One window of sorted items.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonsPage {
    /// How many items there are in all: Lessons, and group headers when the
    /// node is grouped, without the Lessons of folded groups.
    pub total: usize,
    /// How many distinct Lessons the node and the filters admit, however
    /// they are grouped or folded: the count the person reads.
    pub lessons: usize,
    /// The position of the first item among all of them; at most `total`.
    pub offset: usize,
    /// The items of the window: fewer than asked for at the end.
    pub items: Vec<PageItem>,
}

/// One node of the tree of containers: a kind of source, or a prefix of the
/// container ids of that kind split at `/` (host, owner, repository).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonTreeNode {
    /// `kind:prefix`, as `git:` for a kind and `git:github.com/owner` for a
    /// prefix; what [`LessonsQuery::node`] takes.
    pub key: String,
    /// The kind, or the segment this node adds to its parent's prefix.
    pub label: String,
    /// How many distinct Lessons are seen in a container of this kind that
    /// is this node's prefix or under it. A Lesson seen in two repositories
    /// of one owner counts once at the owner.
    pub lessons: usize,
    /// The nodes below, most Lessons first, then by label.
    pub children: Vec<LessonTreeNode>,
}

/// The containers the Lessons are seen in, as a tree.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LessonTree {
    /// How many distinct containers the Lessons name.
    pub containers: usize,
    /// The nodes of the kinds, most Lessons first, then by label.
    pub nodes: Vec<LessonTreeNode>,
}

/// The kind of a container a Lesson names that the store does not list.
const UNKNOWN_KIND: &str = "unknown";

/// A container of a Lesson: its kind and its id split at `/`, empty
/// segments skipped.
#[derive(Clone, Debug)]
struct Placed {
    kind: String,
    segments: Vec<String>,
}

impl Placed {
    /// The container `id`, of the kind `kinds` lists for it.
    fn of(id: &str, kinds: &HashMap<String, String>) -> Self {
        Self {
            kind: kinds
                .get(id)
                .cloned()
                .unwrap_or_else(|| UNKNOWN_KIND.to_owned()),
            segments: id
                .split('/')
                .filter(|segment| !segment.is_empty())
                .map(str::to_owned)
                .collect(),
        }
    }
}

/// One Lesson in the buffer: its row and the containers it is seen in.
struct Entry {
    row: LessonRow,
    containers: Vec<Placed>,
}

/// The ids of the containers `lesson` is seen in: `support.origins`, or the
/// first container when there are none, or nothing without a support. The
/// contract's `origins` names at most five containers, so a Lesson known in
/// more is found under its first five only, until the store answers by
/// container.
pub fn containers_of(lesson: &Lesson) -> Vec<String> {
    let Some(support) = lesson.support.as_option() else {
        return Vec::new();
    };
    if support.origins.is_empty() {
        support.first_container.iter().cloned().collect()
    } else {
        support.origins.clone()
    }
}

/// A node of the tree while it is built.
#[derive(Default)]
struct Trie {
    /// The position of the last Lesson counted here, so one Lesson seen in
    /// two containers below counts once.
    last: Option<usize>,
    lessons: usize,
    children: BTreeMap<String, Trie>,
}

impl Trie {
    /// Counts the Lesson at `index` here, once.
    fn count(&mut self, index: usize) {
        if self.last != Some(index) {
            self.last = Some(index);
            self.lessons += 1;
        }
    }

    /// The nodes below, with the keys that start with `prefix`.
    fn nodes(&self, kind: &str, prefix: &[&str]) -> Vec<LessonTreeNode> {
        let mut nodes: Vec<LessonTreeNode> = self
            .children
            .iter()
            .map(|(label, child)| {
                let mut path = prefix.to_vec();
                path.push(label);
                LessonTreeNode {
                    key: format!("{kind}:{}", path.join("/")),
                    label: label.clone(),
                    lessons: child.lessons,
                    children: child.nodes(kind, &path),
                }
            })
            .collect();
        nodes.sort_by(|a, b| {
            b.lessons
                .cmp(&a.lessons)
                .then_with(|| a.label.cmp(&b.label))
        });
        nodes
    }
}

/// The tree of the containers of `entries`.
fn build_tree(entries: &[Entry]) -> LessonTree {
    let mut kinds: BTreeMap<&str, Trie> = BTreeMap::new();
    let mut containers = HashSet::new();
    for (index, entry) in entries.iter().enumerate() {
        for placed in &entry.containers {
            containers.insert((placed.kind.as_str(), placed.segments.as_slice()));
            let mut node = kinds.entry(placed.kind.as_str()).or_default();
            node.count(index);
            for segment in &placed.segments {
                node = node.children.entry(segment.clone()).or_default();
                node.count(index);
            }
        }
    }
    let mut nodes: Vec<LessonTreeNode> = kinds
        .iter()
        .map(|(kind, trie)| LessonTreeNode {
            key: format!("{kind}:"),
            label: (*kind).to_owned(),
            lessons: trie.lessons,
            children: trie.nodes(kind, &[]),
        })
        .collect();
    nodes.sort_by(|a, b| {
        b.lessons
            .cmp(&a.lessons)
            .then_with(|| a.label.cmp(&b.label))
    });
    LessonTree {
        containers: containers.len(),
        nodes,
    }
}

/// Whether `entry` is seen in a container of the node `key`: of its kind,
/// equal to its prefix or under it.
fn under_node(entry: &Entry, kind: &str, prefix: &[&str]) -> bool {
    entry.containers.iter().any(|placed| {
        placed.kind == kind
            && placed.segments.len() >= prefix.len()
            && placed.segments.iter().zip(prefix).all(|(a, b)| a == b)
    })
}

/// The positions in `entries` of the Lessons under the node `key`; none for
/// a key that is not `kind:prefix`.
fn positions_under(entries: &[Entry], key: &str) -> Vec<usize> {
    let Some((kind, prefix)) = node_parts(key) else {
        return Vec::new();
    };
    entries
        .iter()
        .enumerate()
        .filter(|(_, entry)| under_node(entry, kind, &prefix))
        .map(|(index, _)| index)
        .collect()
}

/// What the cached sequence was found for: the node, the filters, and the
/// folded groups (sorted, without repeats).
type MatchKey = (Option<String>, LessonFilters, Vec<String>);

/// One place of the flattened sequence a page cuts its window from.
enum Slot {
    /// The header of a group.
    Header {
        key: String,
        label: String,
        lessons: usize,
        collapsed: bool,
    },
    /// The Lesson at this position in `entries`.
    Lesson(usize),
}

/// The parts of a node key `kind:prefix`: the kind and the prefix split at
/// `/`; none for a key that is not of this form.
fn node_parts(key: &str) -> Option<(&str, Vec<&str>)> {
    let (kind, prefix) = key.split_once(':')?;
    Some((
        kind,
        prefix
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect(),
    ))
}

/// The node of the tree with `key`.
fn find_node<'a>(nodes: &'a [LessonTreeNode], key: &str) -> Option<&'a LessonTreeNode> {
    nodes.iter().find_map(|node| {
        if node.key == key {
            Some(node)
        } else {
            find_node(&node.children, key)
        }
    })
}

/// The Lessons of one store, in the last order asked for.
struct Lessons {
    entries: Vec<Entry>,
    order: Option<(LessonSortKey, SortDirection)>,
    /// The flattened sequence for the last node, filters, and folded groups
    /// asked for, under that key; dropped when the order changes. Not kept
    /// when none of them narrows.
    matching: Option<(MatchKey, Vec<Slot>, usize)>,
    tree: LessonTree,
}

impl Lessons {
    /// Takes each row with the ids of the containers its Lesson is seen in;
    /// `kinds` maps a container id to its kind.
    fn new(rows: Vec<(LessonRow, Vec<String>)>, kinds: &HashMap<String, String>) -> Self {
        let entries: Vec<Entry> = rows
            .into_iter()
            .map(|(row, ids)| Entry {
                row,
                containers: ids.iter().map(|id| Placed::of(id, kinds)).collect(),
            })
            .collect();
        let tree = build_tree(&entries);
        Self {
            entries,
            order: None,
            matching: None,
            tree,
        }
    }

    /// Reads the row of every Lesson of `store`, one Lesson at a time, and
    /// the kind of every container the store lists.
    async fn read(store: &StoreRef) -> Result<Self, Error> {
        let store = store.to_async_offload()?;
        let mut sources = store.containers(Empty::default()).await?;
        let mut kinds = HashMap::new();
        while let Some(source) = sources.next().await {
            let source = source?;
            kinds.insert(
                source.id.unwrap_or_default(),
                source.kind.unwrap_or_default(),
            );
        }
        let mut stream = store.lessons(Empty::default()).await?;
        let mut rows = Vec::new();
        while let Some(lesson) = stream.next().await {
            let lesson = lesson?;
            rows.push((LessonRow::of(&lesson), containers_of(&lesson)));
        }
        Ok(Self::new(rows, &kinds))
    }

    /// The window `query` asks for; sorts the rows first when it asks for
    /// another order than the last one, and flattens the Lessons that pass
    /// its node and filters (with the headers of their groups) when it asks
    /// for another node, other filters, or other folded groups than the last
    /// time.
    fn page(&mut self, query: &LessonsQuery) -> LessonsPage {
        let order = (query.sort_key, query.direction);
        if self.order != Some(order) {
            self.entries
                .sort_unstable_by(|a, b| compare(&a.row, &b.row, query.sort_key, query.direction));
            self.order = Some(order);
            self.matching = None;
        }
        if query.node.is_none() && query.filters.is_empty() {
            let (start, end) = window(self.entries.len(), query);
            return LessonsPage {
                total: self.entries.len(),
                lessons: self.entries.len(),
                offset: start,
                items: self
                    .entries
                    .get(start..end)
                    .unwrap_or_default()
                    .iter()
                    .map(|entry| PageItem::Lesson(entry.row.clone()))
                    .collect(),
            };
        }
        let mut collapsed = query.collapsed.clone();
        collapsed.sort_unstable();
        collapsed.dedup();
        let key: MatchKey = (query.node.clone(), query.filters.clone(), collapsed);
        if self
            .matching
            .as_ref()
            .is_none_or(|(kept, _, _)| *kept != key)
        {
            let (slots, lessons) = self.slots(query.node.as_deref(), &query.filters, &key.2);
            self.matching = Some((key, slots, lessons));
        }
        let (slots, lessons) = self
            .matching
            .as_ref()
            .map_or((&[][..], 0), |(_, s, n)| (s.as_slice(), *n));
        let (start, end) = window(slots.len(), query);
        LessonsPage {
            total: slots.len(),
            lessons,
            offset: start,
            items: slots
                .get(start..end)
                .unwrap_or_default()
                .iter()
                .filter_map(|slot| match slot {
                    Slot::Header {
                        key,
                        label,
                        lessons,
                        collapsed,
                    } => Some(PageItem::Group {
                        key: key.clone(),
                        label: label.clone(),
                        lessons: *lessons,
                        collapsed: *collapsed,
                    }),
                    Slot::Lesson(index) => self
                        .entries
                        .get(*index)
                        .map(|entry| PageItem::Lesson(entry.row.clone())),
                })
                .collect(),
        }
    }

    /// The sequence of the Lessons under `node` (all when `None`) that
    /// `filters` admit. When the node has children it is grouped by them, in
    /// the tree's order, each group after its header and without its Lessons
    /// when its key is in `collapsed`; the Lessons seen in no child form a
    /// last group named for the node itself. Groups left empty by the filters
    /// are dropped. Without children the sequence is the Lessons alone. With
    /// the sequence comes how many distinct Lessons it holds.
    fn slots(
        &self,
        node: Option<&str>,
        filters: &LessonFilters,
        collapsed: &[String],
    ) -> (Vec<Slot>, usize) {
        let positions = self.positions(node, filters);
        let lessons = positions.len();
        let Some(parent) = node
            .and_then(|key| find_node(&self.tree.nodes, key))
            .filter(|parent| !parent.children.is_empty())
        else {
            return (positions.into_iter().map(Slot::Lesson).collect(), lessons);
        };
        let mut groups: Vec<(&LessonTreeNode, Vec<usize>)> = Vec::new();
        let mut placed = HashSet::new();
        for child in &parent.children {
            let Some((kind, prefix)) = node_parts(&child.key) else {
                continue;
            };
            let members: Vec<usize> = positions
                .iter()
                .copied()
                .filter(|&index| {
                    self.entries
                        .get(index)
                        .is_some_and(|entry| under_node(entry, kind, &prefix))
                })
                .collect();
            placed.extend(members.iter().copied());
            groups.push((child, members));
        }
        let own: Vec<usize> = positions
            .iter()
            .copied()
            .filter(|index| !placed.contains(index))
            .collect();
        groups.push((parent, own));
        let mut slots = Vec::new();
        for (group, members) in groups {
            if members.is_empty() {
                continue;
            }
            let folded = collapsed.contains(&group.key);
            slots.push(Slot::Header {
                key: group.key.clone(),
                label: group.label.clone(),
                lessons: members.len(),
                collapsed: folded,
            });
            if !folded {
                slots.extend(members.into_iter().map(Slot::Lesson));
            }
        }
        (slots, lessons)
    }

    /// The positions of the Lessons under `node` (all when `None`) that
    /// `filters` admit.
    fn positions(&self, node: Option<&str>, filters: &LessonFilters) -> Vec<usize> {
        let mut positions = match node {
            Some(key) => positions_under(&self.entries, key),
            None => (0..self.entries.len()).collect(),
        };
        if !filters.is_empty() {
            positions.retain(|&index| {
                self.entries
                    .get(index)
                    .is_some_and(|entry| filters.admits(&entry.row))
            });
        }
        positions
    }

    /// The values of each field of the Lessons under `node` (all when
    /// `None`), with their counts.
    fn facets(&self, node: Option<&str>) -> LessonFacets {
        let rows: Vec<&LessonRow> = match node {
            Some(key) => positions_under(&self.entries, key)
                .into_iter()
                .filter_map(|index| self.entries.get(index))
                .map(|entry| &entry.row)
                .collect(),
            None => self.entries.iter().map(|entry| &entry.row).collect(),
        };
        LessonFacets {
            kind: facet_counts(rows.iter().map(|row| row.kind.as_str())),
            status: facet_counts(rows.iter().map(|row| row.status.as_str())),
            transfer: facet_counts(rows.iter().map(|row| row.transfer.as_str())),
            polarity: facet_counts(rows.iter().map(|row| row.polarity.as_str())),
        }
    }
}

/// The start and end of the window `query` asks for among `total` rows.
fn window(total: usize, query: &LessonsQuery) -> (usize, usize) {
    let start = query.offset.min(total);
    let end = start
        .saturating_add(query.limit.min(MAX_PAGE_ROWS))
        .min(total);
    (start, end)
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

/// The Lessons of the open store, once read, with the store they were read
/// from.
#[derive(Default)]
pub struct LessonsBuffer(Mutex<Option<(StoreRef, Lessons)>>);

impl LessonsBuffer {
    /// What `answer` makes of the Lessons read from `store`, when they are
    /// read.
    fn answer_of<T>(&self, store: &StoreRef, answer: &impl Fn(&mut Lessons) -> T) -> Option<T> {
        let mut buffer = self.0.lock().unwrap_or_else(PoisonError::into_inner);
        match buffer.as_mut() {
            Some((read_from, lessons)) if read_from == store => Some(answer(lessons)),
            _ => None,
        }
    }

    /// Keeps the Lessons read from `store` in place of any others, and
    /// answers from them.
    fn keep<T>(
        &self,
        store: StoreRef,
        mut lessons: Lessons,
        answer: &impl Fn(&mut Lessons) -> T,
    ) -> T {
        let result = answer(&mut lessons);
        *self.0.lock().unwrap_or_else(PoisonError::into_inner) = Some((store, lessons));
        result
    }
}

/// Answers from the Lessons of the open workspace's store, reading them
/// first when this store's are not kept yet. The lock is not held while they
/// are read. Fails with the store's error (its code first) when they cannot
/// be read, and when no workspace is open.
async fn answer_from<T>(
    open_store: &OpenStore,
    buffer: &LessonsBuffer,
    answer: impl Fn(&mut Lessons) -> T,
) -> Result<T, String> {
    let Some(store) = open_store.current() else {
        return Err("No workspace is open.".to_owned());
    };
    if let Some(result) = buffer.answer_of(&store, &answer) {
        return Ok(result);
    }
    let lessons = Lessons::read(&store)
        .await
        .map_err(|error| error.to_string())?;
    Ok(buffer.keep(store, lessons, &answer))
}

/// Tauri command `lessons_page`: one sorted window of the Lessons of the open
/// workspace (or of the node of the tree asked for) and their total. The
/// first call after a workspace opens reads the Lessons; it fails with the
/// store's error (its code first) when they cannot be read, and when no
/// workspace is open.
#[tauri::command]
pub async fn lessons_page(
    open_store: State<'_, OpenStore>,
    buffer: State<'_, LessonsBuffer>,
    query: LessonsQuery,
) -> Result<LessonsPage, String> {
    answer_from(&open_store, &buffer, |lessons| lessons.page(&query)).await
}

/// Tauri command `lesson_tree`: the tree of the containers the Lessons of the
/// open workspace are seen in, with the number of Lessons under each node.
/// Fails like `lessons_page`.
#[tauri::command]
pub async fn lesson_tree(
    open_store: State<'_, OpenStore>,
    buffer: State<'_, LessonsBuffer>,
) -> Result<LessonTree, String> {
    answer_from(&open_store, &buffer, |lessons| lessons.tree.clone()).await
}

/// Tauri command `lesson_facets`: for the Lessons under the node of the tree
/// asked for (all when `None`), how many have each value of kind, status,
/// transfer, and polarity. Fails like `lessons_page`.
#[tauri::command]
pub async fn lesson_facets(
    open_store: State<'_, OpenStore>,
    buffer: State<'_, LessonsBuffer>,
    node: Option<String>,
) -> Result<LessonFacets, String> {
    answer_from(&open_store, &buffer, |lessons| {
        lessons.facets(node.as_deref())
    })
    .await
}

/// The record of a Lesson as YAML, or a comment that says why it cannot be written.
pub fn record_yaml(lesson: &Lesson) -> String {
    yaml_serde::to_string(lesson)
        .unwrap_or_else(|error| format!("# the record could not be written as YAML: {error}\n"))
}

/// The whole record of one Lesson, and the same record written as YAML.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FoundLesson {
    /// The record as the contract spells it (camelCase, empty fields left
    /// out).
    pub lesson: Lesson,
    /// The record as YAML, or a comment that says why it cannot be written.
    pub yaml: String,
}

impl FoundLesson {
    fn of(lesson: Lesson) -> Self {
        let yaml = record_yaml(&lesson);
        Self { lesson, yaml }
    }
}

/// Tauri command `find_lesson`: the whole record of the Lesson with `id`
/// ([`FoundLesson`]), or `None` when the store does not hold it. Reads the
/// store, not the rows' buffer, so nothing is cut. Fails with the store's
/// error (its code first), and when no workspace is open.
#[tauri::command]
pub async fn find_lesson(
    open_store: State<'_, OpenStore>,
    id: String,
) -> Result<Option<FoundLesson>, String> {
    let Some(store) = open_store.current() else {
        return Err("No workspace is open.".to_owned());
    };
    let store = store
        .to_async_offload()
        .map_err(|error| error.to_string())?;
    let key = LessonKey {
        id: Some(id),
        ..Default::default()
    };
    let found = store
        .find_lesson(key)
        .await
        .map_err(|error| error.to_string())?;
    Ok(found.lesson.into_option().map(FoundLesson::of))
}

/// Registers the buffer of the Lessons' rows.
pub fn manage(app: &AppHandle) {
    app.manage(LessonsBuffer::default());
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use knowqore_stores::{Lesson, Support};

    use super::{
        FacetCount, FoundLesson, LessonFilters, LessonRow, LessonSortKey, Lessons, LessonsQuery,
        MAX_PAGE_ROWS, PageItem, STATEMENT_CHARS, SortDirection, containers_of,
    };

    /// Rows seen in no container.
    fn bare(rows: Vec<LessonRow>) -> Lessons {
        Lessons::new(
            rows.into_iter().map(|row| (row, Vec::new())).collect(),
            &HashMap::new(),
        )
    }

    fn git(ids: &[&str]) -> HashMap<String, String> {
        ids.iter()
            .map(|id| ((*id).to_owned(), "git".to_owned()))
            .collect()
    }

    fn seen(id: &str, containers: &[&str]) -> (LessonRow, Vec<String>) {
        (
            row(id, id, "practice", "t"),
            containers.iter().map(|c| (*c).to_owned()).collect(),
        )
    }

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
        bare(vec![
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
            node: None,
            filters: LessonFilters::default(),
            collapsed: Vec::new(),
        }
    }

    fn ids(lessons: &mut Lessons, query: &LessonsQuery) -> Vec<String> {
        ids_of(&lessons.page(query))
            .into_iter()
            .map(str::to_owned)
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
        assert_eq!(ids_of(&page), ["l3", "l1"]);
    }

    #[test]
    fn a_window_past_the_end_is_empty_and_starts_at_the_total() {
        let mut lessons = fixture();
        let window = LessonsQuery {
            offset: 9,
            ..query(LessonSortKey::Kind, SortDirection::Ascending)
        };
        let page = lessons.page(&window);
        assert_eq!((page.total, page.offset, page.items.len()), (4, 4, 0));
    }

    #[test]
    fn a_window_holds_at_most_the_largest_page() {
        let rows = (0..MAX_PAGE_ROWS + 5)
            .map(|index| row(&format!("l{index:04}"), "s", "asset", "t"))
            .collect();
        let mut lessons = bare(rows);
        let window = LessonsQuery {
            limit: usize::MAX,
            ..query(LessonSortKey::Kind, SortDirection::Ascending)
        };
        assert_eq!(lessons.page(&window).items.len(), MAX_PAGE_ROWS);
    }

    #[test]
    fn a_found_lesson_carries_its_record_as_yaml_in_the_contracts_names() {
        let lesson = Lesson {
            id: Some("l1".to_owned()),
            statement: Some("Say it in full.".to_owned()),
            updated_at: Some("2026-10-23T12:00:00.000Z".to_owned()),
            ..Lesson::default()
        };
        let found = FoundLesson::of(lesson);
        assert!(
            found.yaml.contains("statement: Say it in full."),
            "{}",
            found.yaml
        );
        assert!(found.yaml.contains("updatedAt:"), "{}", found.yaml);
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

    fn tree_fixture() -> Lessons {
        let kinds = git(&[
            "github.com/sonalect/knowqore",
            "github.com/sonalect/scheda",
            "github.com/amsokol/mimalloc",
            "/gitlab.com//team/app/",
        ]);
        Lessons::new(
            vec![
                seen("l1", &["github.com/sonalect/knowqore"]),
                seen(
                    "l2",
                    &["github.com/sonalect/knowqore", "github.com/sonalect/scheda"],
                ),
                seen("l3", &["github.com/amsokol/mimalloc"]),
                seen("l4", &["/gitlab.com//team/app/"]),
                seen("l5", &["elsewhere/x"]),
                seen("l6", &[]),
            ],
            &kinds,
        )
    }

    fn labels(nodes: &[super::LessonTreeNode]) -> Vec<(&str, usize)> {
        nodes
            .iter()
            .map(|node| (node.label.as_str(), node.lessons))
            .collect()
    }

    #[test]
    fn the_tree_splits_containers_and_counts_distinct_lessons() {
        let tree = tree_fixture().tree;
        assert_eq!(tree.containers, 5);
        assert_eq!(labels(&tree.nodes), [("git", 4), ("unknown", 1)]);
        let git = &tree.nodes[0];
        assert_eq!(git.key, "git:");
        assert_eq!(
            labels(&git.children),
            [("github.com", 3), ("gitlab.com", 1)]
        );
        let host = &git.children[0];
        assert_eq!(host.key, "git:github.com");
        // l2 is in two repositories of one owner and counts once there.
        assert_eq!(labels(&host.children), [("sonalect", 2), ("amsokol", 1)]);
        let owner = &host.children[0];
        assert_eq!(owner.key, "git:github.com/sonalect");
        assert_eq!(labels(&owner.children), [("knowqore", 2), ("scheda", 1)]);
        assert_eq!(owner.children[0].key, "git:github.com/sonalect/knowqore");
    }

    #[test]
    fn the_tree_skips_empty_segments_and_names_unlisted_containers_unknown() {
        let tree = tree_fixture().tree;
        let gitlab = &tree.nodes[0].children[1];
        assert_eq!(gitlab.children[0].key, "git:gitlab.com/team");
        assert_eq!(
            gitlab.children[0].children[0].key,
            "git:gitlab.com/team/app"
        );
        let unknown = &tree.nodes[1];
        assert_eq!(unknown.key, "unknown:");
        assert_eq!(unknown.children[0].key, "unknown:elsewhere");
    }

    #[test]
    fn a_lesson_without_origins_is_placed_by_its_first_container() {
        let lesson = Lesson {
            support: Support {
                first_container: Some("github.com/a/b".to_owned()),
                ..Support::default()
            }
            .into(),
            ..Lesson::default()
        };
        assert_eq!(containers_of(&lesson), ["github.com/a/b"]);
        assert!(containers_of(&Lesson::default()).is_empty());
        let with_origins = Lesson {
            support: Support {
                first_container: Some("first".to_owned()),
                origins: vec!["x/y".to_owned(), "x/z".to_owned()],
                ..Support::default()
            }
            .into(),
            ..Lesson::default()
        };
        assert_eq!(containers_of(&with_origins), ["x/y", "x/z"]);
    }

    fn under(node: &str) -> LessonsQuery {
        LessonsQuery {
            node: Some(node.to_owned()),
            ..query(LessonSortKey::Statement, SortDirection::Ascending)
        }
    }

    #[test]
    fn a_node_narrows_the_page_and_its_total() {
        let mut lessons = tree_fixture();
        // Two repositories below the owner: two headers and three rows (l2
        // is in both).
        let owner = lessons.page(&under("git:github.com/sonalect"));
        assert_eq!(owner.total, 5);
        assert_eq!(ids_of(&owner), ["l1", "l2", "l2"]);
        // A repository has no children: no groups.
        let repository = lessons.page(&under("git:github.com/sonalect/scheda"));
        assert_eq!((repository.total, ids_of(&repository)), (1, vec!["l2"]));
        let kind = lessons.page(&under("git:"));
        assert_eq!(kind.total, 6);
        let window = LessonsQuery {
            offset: 1,
            limit: 1,
            ..under("git:")
        };
        let page = lessons.page(&window);
        assert_eq!((page.total, page.offset, ids_of(&page)), (6, 1, vec!["l1"]));
        let all = lessons.page(&query(LessonSortKey::Statement, SortDirection::Ascending));
        assert_eq!(all.total, 6);
    }

    #[test]
    fn a_new_order_keeps_the_node_filter_right() {
        let mut lessons = tree_fixture();
        let descending = LessonsQuery {
            direction: SortDirection::Descending,
            ..under("git:github.com/sonalect")
        };
        assert_eq!(
            ids_of(&lessons.page(&under("git:github.com/sonalect"))),
            ["l1", "l2", "l2"]
        );
        assert_eq!(ids_of(&lessons.page(&descending)), ["l2", "l1", "l2"]);
    }

    #[test]
    fn an_unknown_node_key_gives_an_empty_page() {
        let mut lessons = tree_fixture();
        for key in ["git:nowhere", "nokind", "svn:"] {
            let page = lessons.page(&under(key));
            assert_eq!(
                (page.total, page.offset, page.items.len()),
                (0, 0, 0),
                "{key}"
            );
        }
    }

    /// Rows of known kind, status, and polarity, seen in containers.
    fn faceted() -> Lessons {
        let mut kinds = HashMap::new();
        kinds.insert("a/x".to_owned(), "git".to_owned());
        kinds.insert("b/y".to_owned(), "git".to_owned());
        let make = |id: &str, kind: &str, status: &str, polarity: &str, at: &str| {
            let mut row = row(id, id, kind, "t");
            row.status = status.to_owned();
            row.polarity = polarity.to_owned();
            (row, vec![at.to_owned()])
        };
        Lessons::new(
            vec![
                make("l1", "asset", "active", "prefer", "a/x"),
                make("l2", "asset", "retired", "avoid", "a/x"),
                make("l3", "pitfall", "active", "avoid", "b/y"),
                make("l4", "practice", "active", "prefer", "b/y"),
            ],
            &kinds,
        )
    }

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|item| (*item).to_owned()).collect()
    }

    fn filtered(node: Option<&str>, filters: LessonFilters) -> LessonsQuery {
        LessonsQuery {
            node: node.map(str::to_owned),
            filters,
            ..query(LessonSortKey::Statement, SortDirection::Ascending)
        }
    }

    #[test]
    fn values_of_one_field_are_or_and_fields_are_and() {
        let mut lessons = faceted();
        let kinds = LessonFilters {
            kind: strings(&["asset", "pitfall"]),
            ..LessonFilters::default()
        };
        let page = lessons.page(&filtered(None, kinds.clone()));
        assert_eq!((page.total, ids_of(&page)), (3, vec!["l1", "l2", "l3"]));
        let both = LessonFilters {
            status: strings(&["active"]),
            ..kinds
        };
        let page = lessons.page(&filtered(None, both));
        assert_eq!((page.total, ids_of(&page)), (2, vec!["l1", "l3"]));
    }

    #[test]
    fn filters_apply_within_a_node_and_empty_filters_change_nothing() {
        let mut lessons = faceted();
        let avoid = LessonFilters {
            polarity: strings(&["avoid"]),
            ..LessonFilters::default()
        };
        let page = lessons.page(&filtered(Some("git:a"), avoid));
        // One group below `git:a`: its header and the Lesson.
        assert_eq!((page.total, ids_of(&page)), (2, vec!["l2"]));
        let plain = lessons.page(&filtered(Some("git:a"), LessonFilters::default()));
        assert_eq!(plain.total, 3);
        let all = lessons.page(&filtered(None, LessonFilters::default()));
        assert_eq!(all.total, 4);
    }

    #[test]
    fn the_cache_follows_the_filters_alone() {
        let mut lessons = faceted();
        let by = |status: &str| LessonFilters {
            status: strings(&[status]),
            ..LessonFilters::default()
        };
        assert_eq!(lessons.page(&filtered(Some("git:"), by("active"))).total, 5);
        assert_eq!(
            lessons.page(&filtered(Some("git:"), by("retired"))).total,
            2
        );
        assert_eq!(lessons.page(&filtered(Some("git:"), by("active"))).total, 5);
        assert_eq!(lessons.page(&filtered(None, by("retired"))).total, 1);
        assert_eq!(lessons.page(&filtered(None, by("candidate"))).total, 0);
    }

    fn counts(facets: &[FacetCount]) -> Vec<(&str, usize)> {
        facets
            .iter()
            .map(|facet| (facet.value.as_str(), facet.lessons))
            .collect()
    }

    #[test]
    fn facets_count_the_values_under_a_node_and_for_all() {
        let lessons = faceted();
        let all = lessons.facets(None);
        assert_eq!(
            counts(&all.kind),
            [("asset", 2), ("pitfall", 1), ("practice", 1)]
        );
        assert_eq!(counts(&all.status), [("active", 3), ("retired", 1)]);
        let under = lessons.facets(Some("git:a"));
        assert_eq!(counts(&under.kind), [("asset", 2)]);
        assert_eq!(counts(&under.polarity), [("avoid", 1), ("prefer", 1)]);
        assert_eq!(counts(&under.transfer), [("global", 2)]);
        assert_eq!(
            lessons.facets(Some("git:nowhere")),
            super::LessonFacets::default()
        );
    }

    /// The ids of the Lessons of a page, headers left out.
    fn ids_of(page: &super::LessonsPage) -> Vec<&str> {
        page.items
            .iter()
            .filter_map(|item| match item {
                PageItem::Lesson(row) => Some(row.id.as_str()),
                PageItem::Group { .. } => None,
            })
            .collect()
    }

    /// A page as text: `[key (n)]` for an open group, `[key (n)]-` for a
    /// folded one, the id for a Lesson.
    fn shape(page: &super::LessonsPage) -> Vec<String> {
        page.items
            .iter()
            .map(|item| match item {
                PageItem::Lesson(row) => row.id.clone(),
                PageItem::Group {
                    key,
                    lessons,
                    collapsed,
                    ..
                } => format!("[{key} ({lessons})]{}", if *collapsed { "-" } else { "" }),
            })
            .collect()
    }

    fn grouped(node: &str, collapsed: &[&str]) -> LessonsQuery {
        LessonsQuery {
            collapsed: strings(collapsed),
            ..under(node)
        }
    }

    /// l1 knowqore; l2 knowqore and scheda; l3 mimalloc; l4 gitlab; one
    /// Lesson (l7) at the owner `sonalect` itself, none in a repository.
    fn grouping_fixture() -> Lessons {
        let kinds = git(&[
            "github.com/sonalect/knowqore",
            "github.com/sonalect/scheda",
            "github.com/amsokol/mimalloc",
            "gitlab.com/team/app",
            "github.com/sonalect",
        ]);
        Lessons::new(
            vec![
                seen("l1", &["github.com/sonalect/knowqore"]),
                seen(
                    "l2",
                    &["github.com/sonalect/knowqore", "github.com/sonalect/scheda"],
                ),
                seen("l3", &["github.com/amsokol/mimalloc"]),
                seen("l4", &["gitlab.com/team/app"]),
                seen("l7", &["github.com/sonalect"]),
            ],
            &kinds,
        )
    }

    #[test]
    fn a_kind_groups_by_its_hosts() {
        let mut lessons = grouping_fixture();
        let page = lessons.page(&grouped("git:", &[]));
        assert_eq!(
            shape(&page),
            [
                "[git:github.com (4)]",
                "l1",
                "l2",
                "l3",
                "l7",
                "[git:gitlab.com (1)]",
                "l4"
            ]
        );
        assert_eq!(page.total, 7);
    }

    #[test]
    fn a_host_groups_by_owners_and_an_owner_by_repositories() {
        let mut lessons = grouping_fixture();
        assert_eq!(
            shape(&lessons.page(&grouped("git:github.com", &[]))),
            [
                "[git:github.com/sonalect (3)]",
                "l1",
                "l2",
                "l7",
                "[git:github.com/amsokol (1)]",
                "l3"
            ]
        );
        let page = lessons.page(&grouped("git:github.com/sonalect", &[]));
        assert_eq!(
            shape(&page),
            [
                "[git:github.com/sonalect/knowqore (2)]",
                "l1",
                "l2",
                "[git:github.com/sonalect/scheda (1)]",
                "l2",
                "[git:github.com/sonalect (1)]",
                "l7"
            ]
        );
        assert!(
            page.items.iter().any(|item| matches!(
                item,
                PageItem::Group { key, label, .. }
                    if key == "git:github.com/sonalect" && label == "sonalect"
            )),
            "{page:?}"
        );
    }

    #[test]
    fn filters_change_the_counts_and_drop_empty_groups() {
        let mut lessons = grouping_fixture();
        // Only l2 is retired.
        for entry in lessons.entries.iter_mut().filter(|e| e.row.id == "l2") {
            entry.row.status = "retired".to_owned();
        }
        let query = LessonsQuery {
            filters: LessonFilters {
                status: strings(&["retired"]),
                ..LessonFilters::default()
            },
            ..grouped("git:github.com/sonalect", &[])
        };
        assert_eq!(
            shape(&lessons.page(&query)),
            [
                "[git:github.com/sonalect/knowqore (1)]",
                "l2",
                "[git:github.com/sonalect/scheda (1)]",
                "l2"
            ]
        );
    }

    #[test]
    fn a_folded_group_keeps_only_its_header_and_the_total_follows() {
        let mut lessons = grouping_fixture();
        let page = lessons.page(&grouped("git:", &["git:github.com"]));
        assert_eq!(
            shape(&page),
            ["[git:github.com (4)]-", "[git:gitlab.com (1)]", "l4"]
        );
        assert_eq!(page.total, 3);
        let unfolded = lessons.page(&grouped("git:", &[]));
        assert_eq!(unfolded.total, 7);
    }

    #[test]
    fn the_lesson_count_counts_each_lesson_once_however_it_is_grouped() {
        let mut lessons = grouping_fixture();
        // Under `sonalect`, l2 sits in two repositories and l7 in the owner's
        // own group: three Lessons, five rows and headers when open.
        let open = lessons.page(&grouped("git:github.com/sonalect", &[]));
        assert_eq!(open.lessons, 3);
        assert!(open.total > open.lessons);
        let folded = lessons.page(&grouped(
            "git:github.com/sonalect",
            &[
                "git:github.com/sonalect/knowqore",
                "git:github.com/sonalect/scheda",
                "git:github.com/sonalect",
            ],
        ));
        assert_eq!(folded.lessons, 3);
        assert_eq!(folded.total, 3);
    }

    #[test]
    fn a_window_may_start_inside_a_group() {
        let mut lessons = grouping_fixture();
        let window = LessonsQuery {
            offset: 2,
            limit: 3,
            ..grouped("git:", &[])
        };
        let page = lessons.page(&window);
        assert_eq!((page.total, page.offset), (7, 2));
        assert_eq!(shape(&page), ["l2", "l3", "l7"]);
    }

    #[test]
    fn a_leaf_and_no_node_show_no_groups() {
        let mut lessons = grouping_fixture();
        let leaf = lessons.page(&grouped("git:github.com/sonalect/scheda", &[]));
        assert_eq!(shape(&leaf), ["l2"]);
        let all = lessons.page(&query(LessonSortKey::Statement, SortDirection::Ascending));
        assert!(
            all.items
                .iter()
                .all(|item| matches!(item, PageItem::Lesson(_)))
        );
    }

    #[test]
    fn a_page_item_is_tagged_in_the_wire_form() {
        let group = PageItem::Group {
            key: "git:a".to_owned(),
            label: "a".to_owned(),
            lessons: 2,
            collapsed: true,
        };
        let json = serde_json::to_string(&group).unwrap_or_default();
        assert_eq!(
            json,
            r#"{"type":"group","key":"git:a","label":"a","lessons":2,"collapsed":true}"#
        );
        let lesson = PageItem::Lesson(row("l1", "s", "asset", "t"));
        let json = serde_json::to_string(&lesson).unwrap_or_default();
        assert!(json.starts_with(r#"{"type":"lesson","id":"l1""#), "{json}");
        assert!(json.contains(r#""updatedAt":"t""#), "{json}");
    }
}
