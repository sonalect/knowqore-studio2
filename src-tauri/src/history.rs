//! The history of a Lesson as the detail shows it: the versions the store keeps of its record,
//! newest first, each turned into events by comparing it with the version before, and grouped by
//! the run of Knowqore that wrote them.
//!
//! Who wrote a version is read from the note of its write, Knowqore's convention: learn writes
//! `learn`, and the intake of a session writes `Intake <ms>-<seq>-<pid>-<origin>`, `origin` being
//! `learn` or `ingest`. A note of another shape is shown as it is. Everything here is plain Rust
//! over the versions; nothing calls the store or the interface.

use std::collections::BTreeSet;

use futures_lite::StreamExt as _;
use knowqore_stores::{Curation, Lesson, LessonRef, LessonVersion, StoreAsync as _, Support};
use serde::Serialize;
use tauri::State;

use crate::lessons::{containers_of, record_yaml};
use crate::workspace::OpenStore;

/// What wrote a version, by the note of its write.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Origin {
    /// The learning engine.
    Learn,
    /// The intake of the person's sessions with an agent.
    Ingest,
    /// A write whose note Studio does not know.
    Other,
}

/// Who wrote a version: the origin, and the process of the run when the note names one.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Writer {
    /// What wrote it.
    pub origin: Origin,
    /// The process id of the run, without its leading zeros, when the note names it.
    pub run: Option<String>,
    /// The note as the write gave it, empty when it had none.
    pub note: String,
}

/// Reads who wrote a version from the note of its write.
pub fn writer_of(note: Option<&str>) -> Writer {
    let note = note.unwrap_or_default();
    let writer = |origin, run| Writer {
        origin,
        run,
        note: note.to_owned(),
    };
    if note == "learn" {
        return writer(Origin::Learn, None);
    }
    let intake = note.strip_prefix("Intake ").and_then(|stem| {
        let parts: Vec<&str> = stem.split('-').collect();
        let [ms, seq, pid, origin] = parts.as_slice() else {
            return None;
        };
        let digits = |part: &str| !part.is_empty() && part.bytes().all(|b| b.is_ascii_digit());
        if !(digits(ms) && digits(seq) && digits(pid)) {
            return None;
        }
        let origin = match *origin {
            "learn" => Origin::Learn,
            "ingest" => Origin::Ingest,
            _ => return None,
        };
        let run = pid.trim_start_matches('0');
        Some((origin, if run.is_empty() { "0" } else { run }.to_owned()))
    });
    match intake {
        Some((origin, run)) => writer(origin, Some(run)),
        None => writer(Origin::Other, None),
    }
}

/// How a chip of an event reads: something added, something taken away, or a plain fact.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Tone {
    /// Something added or that holds.
    Added,
    /// Something taken away or that does not hold.
    Removed,
    /// A plain fact.
    Neutral,
}

/// A short fact under an event, such as `+1 source` or `+ github.com/acme/app`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Chip {
    /// The text of the chip.
    pub text: String,
    /// How it reads.
    pub tone: Tone,
}

impl Chip {
    fn new(text: impl Into<String>, tone: Tone) -> Self {
        Self {
            text: text.into(),
            tone,
        }
    }
}

/// What kind of change an event is, which picks its mark.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum EventKind {
    /// The first version of the record.
    Created,
    /// The write removed the record.
    Removed,
    /// The version could not be read as a Lesson.
    Unreadable,
    /// The revision moved: what the Lesson says changed.
    Revision,
    /// The status moved.
    Status,
    /// The curator's verdict changed or was cleared.
    Curation,
    /// The containers it is seen in changed.
    Containers,
    /// The numbers of what stands behind it changed.
    Evidence,
    /// A person chose it as the lead.
    Choice,
    /// An agent amended a lead of it.
    Amendment,
    /// Whether a search may find it changed.
    Search,
    /// The phrasings of its task changed.
    Tasks,
    /// The places it is anchored to changed.
    Anchors,
    /// Nothing the history shows changed.
    Quiet,
}

/// One event of a version.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    /// What kind of change it is.
    pub kind: EventKind,
    /// The line that says what happened.
    pub title: String,
    /// A line under it, when there is more to say.
    pub sub: Option<String>,
    /// Short facts under it.
    pub chips: Vec<Chip>,
    /// The curator's reason, or why a version could not be read.
    pub reason: Option<String>,
    /// The other Lesson a verdict names, with the words that lead to it.
    pub other: Option<Other>,
    /// The statement before a revision that changed it, and after.
    pub statements: Option<Statements>,
}

/// The other Lesson a curator's verdict names.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Other {
    /// The words that lead to it: "Duplicate of", "Replaced by", or "See".
    pub label: &'static str,
    /// Its id.
    pub id: String,
}

/// The statement of a Lesson before a revision and after it.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Statements {
    /// Before the revision.
    pub old: String,
    /// After it.
    pub new: String,
}

impl Event {
    fn new(kind: EventKind, title: impl Into<String>) -> Self {
        Self {
            kind,
            title: title.into(),
            sub: None,
            chips: Vec::new(),
            reason: None,
            other: None,
            statements: None,
        }
    }

    fn sub(mut self, sub: impl Into<String>) -> Self {
        let sub = sub.into();
        if !sub.is_empty() {
            self.sub = Some(sub);
        }
        self
    }
}

/// One version of the record with what it changed.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Change {
    /// The id of the write that made it, as the store names it.
    pub write_id: String,
    /// The first seven characters of the id, as the list shows it.
    pub short_id: String,
    /// The time of the write as the store gives it, RFC 3339 UTC.
    pub at: String,
    /// The day of the write, `YYYY-MM-DD`.
    pub day: String,
    /// Who wrote it.
    pub writer: Writer,
    /// Whether the write changed nothing the history shows.
    pub quiet: bool,
    /// What it changed. Never empty.
    pub events: Vec<Event>,
}

/// Versions written one after another by the same run on the same day.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Run {
    /// Who wrote them.
    pub writer: Writer,
    /// What the group is called: "Learning run", "Agent sessions", or the note.
    pub title: String,
    /// The day, `YYYY-MM-DD`.
    pub day: String,
    /// The changes of the group, newest first, as indices into [`History::changes`].
    pub changes: Vec<usize>,
}

/// The history of one Lesson, newest first, as the frontend draws it: every change with its
/// events, and the groups of them with the quiet changes left out and kept.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct History {
    /// Every change, newest first.
    pub changes: Vec<Change>,
    /// The groups without the quiet changes.
    pub runs: Vec<Run>,
    /// The groups with the quiet changes: they can join or split groups.
    pub runs_with_quiet: Vec<Run>,
    /// How many changes changed something the history shows.
    pub meaningful: usize,
    /// How many changes changed nothing the history shows.
    pub quiet: usize,
    /// The day of the oldest change, `YYYY-MM-DD`.
    pub since: Option<String>,
}

impl History {
    /// Turns the versions of a record, newest first, into its changes.
    pub fn new(versions: &[LessonVersion]) -> Self {
        let mut changes = Vec::with_capacity(versions.len());
        let mut before: Option<&Lesson> = None;
        for version in versions.iter().rev() {
            let at = version.at.as_deref().unwrap_or_default();
            let write_id = version.write_id.clone().unwrap_or_default();
            let events = match (version.lesson.as_option(), version.unreadable.as_deref()) {
                (Some(lesson), _) => {
                    let events = events_between(before, lesson);
                    before = Some(lesson);
                    events
                }
                (None, Some(reason)) => {
                    let mut event = Event::new(EventKind::Unreadable, "Version cannot be read");
                    event.reason = Some(reason.to_owned());
                    vec![event]
                }
                (None, None) => {
                    before = None;
                    vec![
                        Event::new(EventKind::Removed, "Removed")
                            .sub("The write removed the record"),
                    ]
                }
            };
            changes.push(Change {
                short_id: write_id.chars().take(7).collect(),
                write_id,
                at: at.to_owned(),
                day: at.get(..10).unwrap_or(at).to_owned(),
                writer: writer_of(version.note.as_deref()),
                quiet: events.iter().all(|event| event.kind == EventKind::Quiet),
                events,
            });
        }
        changes.reverse();
        let quiet = changes.iter().filter(|change| change.quiet).count();
        Self {
            runs: runs(&changes, false),
            runs_with_quiet: runs(&changes, true),
            meaningful: changes.len() - quiet,
            quiet,
            since: changes.last().map(|first| first.day.clone()),
            changes,
        }
    }
}

/// The groups of changes, newest first: one group for the changes one run wrote one after
/// another on one day. The quiet ones are left out unless `with_quiet`.
fn runs(changes: &[Change], with_quiet: bool) -> Vec<Run> {
    let mut runs: Vec<Run> = Vec::new();
    for (ix, change) in changes.iter().enumerate() {
        if !with_quiet && change.quiet {
            continue;
        }
        match runs.last_mut() {
            Some(run) if run.writer == change.writer && run.day == change.day => {
                run.changes.push(ix);
            }
            _ => runs.push(Run {
                title: match change.writer.origin {
                    Origin::Learn => "Learning run".to_owned(),
                    Origin::Ingest => "Agent sessions".to_owned(),
                    Origin::Other if change.writer.note.is_empty() => "No note".to_owned(),
                    Origin::Other => change.writer.note.clone(),
                },
                writer: change.writer.clone(),
                day: change.day.clone(),
                changes: vec![ix],
            }),
        }
    }
    runs
}

/// The unified patch of the record between two versions, written as YAML, for the kit's diff
/// view: `before` is the version before the write (none for the first), `after` the one it made
/// (none for a removal or a version that cannot be read). `id` names the file of the patch.
pub fn yaml_patch(id: &str, before: Option<&Lesson>, after: Option<&Lesson>) -> String {
    let before = before.map(record_yaml).unwrap_or_default();
    let after = after.map(record_yaml).unwrap_or_default();
    let name = format!("{id}.yaml");
    format!(
        "diff --git a/{name} b/{name}\n{}",
        similar::TextDiff::from_lines(&before, &after)
            .unified_diff()
            .context_radius(3)
            .header(&format!("a/{name}"), &format!("b/{name}"))
    )
}

/// The record before the write of the version at `at` of `versions` (newest first): the nearest
/// older version that reads, or none when the write made the first version or the record was
/// removed in between.
pub fn lesson_before(versions: &[LessonVersion], at: usize) -> Option<&Lesson> {
    for version in versions.get(at + 1..)? {
        match (version.lesson.as_option(), version.unreadable.as_deref()) {
            (Some(lesson), _) => return Some(lesson),
            (None, Some(_)) => {}
            (None, None) => return None,
        }
    }
    None
}

/// The events of a version: the changes from the one before, or its creation.
fn events_between(before: Option<&Lesson>, after: &Lesson) -> Vec<Event> {
    let Some(before) = before else {
        return vec![created(after)];
    };
    let mut events = Vec::new();
    events.extend(revision(before, after));
    events.extend(status(before, after));
    events.extend(curation(before, after));
    events.extend(containers(before, after));
    let (was, is) = (support(before), support(after));
    events.extend(evidence(was, is));
    events.extend(choices(was, is));
    events.extend(amendment(was, is));
    if before.searchable != after.searchable {
        events.push(if after.searchable == Some(true) {
            Event::new(EventKind::Search, "A search may find it")
        } else {
            Event::new(EventKind::Search, "A search no longer finds it")
        });
    }
    if before.revision == after.revision && before.tasks != after.tasks {
        events.push(Event::new(EventKind::Tasks, "Tasks reworded").sub(format!(
            "{} {}",
            after.tasks.len(),
            plural(after.tasks.len(), "task", "tasks")
        )));
    }
    if before.anchors != after.anchors {
        events.push(
            Event::new(EventKind::Anchors, "Anchors changed").sub(format!(
                "{} {}",
                after.anchors.len(),
                plural(after.anchors.len(), "anchor", "anchors")
            )),
        );
    }
    if events.is_empty() {
        events.push(
            Event::new(EventKind::Quiet, "Rewritten, no change")
                .sub("Nothing the history shows changed"),
        );
    }
    events
}

fn created(lesson: &Lesson) -> Event {
    let members = count(lesson.members_count);
    let mut event = Event::new(
        EventKind::Created,
        format!(
            "Created \u{b7} revision {} \u{b7} {}",
            lesson.revision.unwrap_or_default(),
            lesson.status.as_deref().unwrap_or_default()
        ),
    );
    let seen_in = containers_of(lesson).join(", ");
    event = event.sub(format!(
        "Reduced from {members} {} seen in {}",
        plural(members, "candidate", "candidates"),
        if seen_in.is_empty() {
            "no container"
        } else {
            &seen_in
        }
    ));
    event
}

fn revision(before: &Lesson, after: &Lesson) -> Option<Event> {
    if before.revision == after.revision {
        return None;
    }
    let mut changed = Vec::new();
    if before.kind != after.kind {
        changed.push("kind");
    }
    if before.statement != after.statement {
        changed.push("statement");
    }
    if before.polarity != after.polarity {
        changed.push("polarity");
    }
    if before.conditions != after.conditions {
        changed.push("conditions");
    }
    if before.transfer != after.transfer {
        changed.push("transfer");
    }
    let mut sub = String::new();
    if !changed.is_empty() {
        sub.push_str(&format!("Changed: {} \u{b7} ", changed.join(", ")));
    }
    sub.push_str(&format!(
        "candidates {} \u{2192} {}",
        count(before.members_count),
        count(after.members_count)
    ));
    let mut event = Event::new(
        EventKind::Revision,
        format!(
            "Revision {} \u{2192} {}",
            before.revision.unwrap_or_default(),
            after.revision.unwrap_or_default()
        ),
    )
    .sub(sub);
    if before.statement != after.statement {
        event.statements = Some(Statements {
            old: before.statement.clone().unwrap_or_default(),
            new: after.statement.clone().unwrap_or_default(),
        });
    }
    Some(event)
}

fn status(before: &Lesson, after: &Lesson) -> Option<Event> {
    if before.status == after.status {
        return None;
    }
    Some(
        Event::new(
            EventKind::Status,
            format!(
                "Status: {} \u{2192} {}",
                before.status.as_deref().unwrap_or_default(),
                after.status.as_deref().unwrap_or_default()
            ),
        )
        .sub(after.status_reasons.join("; ")),
    )
}

fn curation(before: &Lesson, after: &Lesson) -> Option<Event> {
    let (was, is) = (before.curation.as_option(), after.curation.as_option());
    let key = |curation: Option<&Curation>| {
        curation.map(|c| (c.verdict.clone(), c.other.clone(), c.reason.clone()))
    };
    if key(was) == key(is) {
        return None;
    }
    let Some(is) = is else {
        return Some(
            Event::new(EventKind::Curation, "Curation cleared")
                .sub("Reduced again; waits for the next curation"),
        );
    };
    let verdict = is.verdict.as_deref().unwrap_or_default();
    let title = match was.and_then(|was| was.verdict.as_deref()) {
        Some(from) if from != verdict => format!("Curator: {from} \u{2192} {verdict}"),
        _ => format!("Curator: {verdict}"),
    };
    let mut event = Event::new(EventKind::Curation, title);
    event.reason = is.reason.clone();
    event.other = is.other.clone().map(|id| Other {
        label: match verdict {
            "duplicate" => "Duplicate of",
            "outdated" => "Replaced by",
            _ => "See",
        },
        id,
    });
    Some(event)
}

fn containers(before: &Lesson, after: &Lesson) -> Option<Event> {
    let (named_was, named_is) = (containers_of(before), containers_of(after));
    let was: BTreeSet<&str> = named_was.iter().map(String::as_str).collect();
    let is: BTreeSet<&str> = named_is.iter().map(String::as_str).collect();
    let (count_was, count_is) = (
        container_count(before, was.len()),
        container_count(after, is.len()),
    );
    if was == is && count_was == count_is {
        return None;
    }
    let mut chips: Vec<Chip> = is
        .difference(&was)
        .map(|container| Chip::new(format!("+ {container}"), Tone::Added))
        .collect();
    chips.extend(
        was.difference(&is)
            .map(|container| Chip::new(format!("\u{2212} {container}"), Tone::Removed)),
    );
    let unnamed = |count: usize, named: usize| count.saturating_sub(named);
    let (more_was, more_is) = (unnamed(count_was, was.len()), unnamed(count_is, is.len()));
    if more_is != more_was {
        chips.push(Chip::new(
            format!("{more_is} more not named"),
            Tone::Neutral,
        ));
    }
    let title = if count_is >= count_was {
        "Seen in more containers"
    } else {
        "Seen in fewer containers"
    };
    let mut event = Event::new(EventKind::Containers, title);
    event.chips = chips;
    Some(event)
}

fn evidence(was: Option<&Support>, is: Option<&Support>) -> Option<Event> {
    let numbers = |support: Option<&Support>| {
        support.map_or([0; 4], |s| {
            [
                s.source_containers,
                s.record_containers,
                s.present_containers,
                s.absent_containers,
            ]
            .map(|value| i64::from(value.unwrap_or_default()))
        })
    };
    let (was, is) = (numbers(was), numbers(is));
    let names = ["source", "record", "present", "absent"];
    let mut chips = Vec::new();
    let mut only_added = true;
    for ((before, after), name) in was.iter().zip(is).zip(names) {
        let delta = after - before;
        if delta == 0 {
            continue;
        }
        only_added &= delta > 0;
        let tone = match (name, delta > 0) {
            ("absent", true) | (_, false) => Tone::Removed,
            _ => Tone::Added,
        };
        let sign = if delta > 0 { "+" } else { "\u{2212}" };
        chips.push(Chip::new(format!("{sign}{} {name}", delta.abs()), tone));
    }
    if chips.is_empty() {
        return None;
    }
    let only_checks = was[..2] == is[..2];
    let mut event = Event::new(
        EventKind::Evidence,
        if only_added {
            "Evidence added"
        } else {
            "Evidence changed"
        },
    );
    if only_checks {
        event = event.sub("Anchors checked on the current version");
    }
    event.chips = chips;
    Some(event)
}

fn choices(was: Option<&Support>, is: Option<&Support>) -> Option<Event> {
    let of = |support: Option<&Support>| support.and_then(|s| s.choices).unwrap_or_default();
    let delta = of(is) - of(was);
    (delta > 0).then(|| {
        Event::new(EventKind::Choice, "Your choice").sub(format!(
            "You chose it as the lead {delta} {}",
            plural(usize::try_from(delta).unwrap_or_default(), "time", "times")
        ))
    })
}

fn amendment(was: Option<&Support>, is: Option<&Support>) -> Option<Event> {
    let of = |support: Option<&Support>| {
        support
            .and_then(|s| s.amendment.as_option())
            .and_then(|a| a.observation.clone())
    };
    let latest = is.and_then(|s| s.amendment.as_option())?;
    if of(was) == of(is) {
        return None;
    }
    Some(
        Event::new(EventKind::Amendment, "Amended by an agent").sub(format!(
            "In {}",
            latest.container.as_deref().unwrap_or_default()
        )),
    )
}

fn support(lesson: &Lesson) -> Option<&Support> {
    lesson.support.as_option()
}

/// How many containers a Lesson is known in: at least as many as it names.
fn container_count(lesson: &Lesson, named: usize) -> usize {
    let counted = lesson
        .support
        .as_option()
        .and_then(|support| support.containers)
        .and_then(|count| usize::try_from(count).ok())
        .unwrap_or_default();
    counted.max(named)
}

fn count(value: Option<i32>) -> usize {
    value
        .and_then(|value| usize::try_from(value).ok())
        .unwrap_or_default()
}

fn plural<'a>(count: usize, one: &'a str, many: &'a str) -> &'a str {
    if count == 1 { one } else { many }
}

/// The versions of the record of the Lesson with `id` in the open store, newest first.
async fn read_versions(open_store: &OpenStore, id: String) -> Result<Vec<LessonVersion>, String> {
    let Some(store) = open_store.current() else {
        return Err("No workspace is open.".to_owned());
    };
    let store = store
        .to_async_offload()
        .map_err(|error| error.to_string())?;
    let mut stream = store
        .lesson_history(LessonRef {
            lesson: Some(id),
            ..Default::default()
        })
        .await
        .map_err(|error| error.to_string())?;
    let mut versions = Vec::new();
    while let Some(version) = stream.next().await {
        versions.push(version.map_err(|error| error.to_string())?);
    }
    Ok(versions)
}

/// Tauri command `lesson_history`: the history of the Lesson with `id` ([`History`]), empty for
/// a Lesson the store never held. Fails with the store's error (its code first), and when no
/// workspace is open.
#[tauri::command]
pub async fn lesson_history(
    open_store: State<'_, OpenStore>,
    id: String,
) -> Result<History, String> {
    Ok(History::new(&read_versions(&open_store, id).await?))
}

/// Tauri command `lesson_write_diff`: the unified YAML diff of the Lesson with `id` between the
/// write `write_id` and the readable version before it. Not part of [`History`]: a long history
/// would carry the whole record once per write, and a person opens few of them. Fails when the
/// store does not keep that write for the Lesson.
#[tauri::command]
pub async fn lesson_write_diff(
    open_store: State<'_, OpenStore>,
    id: String,
    write_id: String,
) -> Result<String, String> {
    let versions = read_versions(&open_store, id.clone()).await?;
    let Some(at) = versions
        .iter()
        .position(|version| version.write_id.as_deref() == Some(write_id.as_str()))
    else {
        return Err(format!(
            "The store keeps no write {write_id} of this Lesson."
        ));
    };
    Ok(yaml_patch(
        &id,
        lesson_before(&versions, at),
        versions
            .get(at)
            .and_then(|version| version.lesson.as_option()),
    ))
}

#[cfg(test)]
mod tests;
