//! The history of a Lesson over versions made in memory from the fixture: who wrote each, the
//! events of every kind, the groups by run, the summary, and the patch of a write.

use knowqore_stores::{Amendment, Curation, LessonVersion, Support};

use super::*;

const INGEST: &str = "Intake 0001792653461143-0000000008-0000040607-ingest";

/// A Lesson seen in one container, as its first version.
fn first() -> Lesson {
    Lesson {
        id: Some("l0a1b2c3d4e5f00020a1b2c3d4e5f0002".to_owned()),
        statement: Some("Format Go code before every commit.".to_owned()),
        revision: Some(1),
        status: Some("candidate".to_owned()),
        members_count: Some(1),
        support: Support {
            origins: vec!["gitlab.com/team/tool".to_owned()],
            containers: Some(1),
            source_containers: Some(1),
            ..Support::default()
        }
        .into(),
        ..Lesson::default()
    }
}

fn version(write_id: &str, at: &str, note: &str, lesson: Option<Lesson>) -> LessonVersion {
    LessonVersion {
        write_id: Some(write_id.to_owned()),
        at: Some(at.to_owned()),
        note: (!note.is_empty()).then(|| note.to_owned()),
        lesson: lesson.into(),
        ..LessonVersion::default()
    }
}

fn titles(change: &Change) -> Vec<&str> {
    change
        .events
        .iter()
        .map(|event| event.title.as_str())
        .collect()
}

/// Eight writes of one Lesson, oldest first: created, rewritten, revised, curated, checked and
/// seen elsewhere, chosen and amended, removed, and a version that cannot be read.
fn versions() -> Vec<LessonVersion> {
    let one = first();
    let mut quiet = one.clone();
    quiet.updated_at = Some("2026-10-06T07:00:00.000Z".to_owned());
    let mut revised = quiet.clone();
    revised.revision = Some(2);
    revised.statement = Some("Format Go code with teamfmt before every commit.".to_owned());
    revised.members_count = Some(3);
    revised.status = Some("active".to_owned());
    revised.status_reasons = vec!["seen in two containers".to_owned()];
    let mut curated = revised.clone();
    curated.curation = Curation {
        verdict: Some("duplicate".to_owned()),
        other: Some("l0a1b2c3d4e5f00010a1b2c3d4e5f0001".to_owned()),
        reason: Some("Says again what the asset says.".to_owned()),
        ..Curation::default()
    }
    .into();
    let mut checked = curated.clone();
    let support = checked.support.get_or_insert_default();
    support.origins.push("github.com/acme/app".to_owned());
    support.containers = Some(2);
    support.source_containers = Some(2);
    support.present_containers = Some(1);
    let mut chosen = checked.clone();
    let support = chosen.support.get_or_insert_default();
    support.choices = Some(2);
    support.amendment = Amendment {
        observation: Some("o0a1b2c3d4e5f00ee0a1b2c3d4e5f00ee".to_owned()),
        container: Some("github.com/acme/app".to_owned()),
        at: Some("2026-10-08T10:00:00.000Z".to_owned()),
        ..Amendment::default()
    }
    .into();
    let mut unreadable = version("w8", "2026-10-09T10:00:00.000Z", "learn", None);
    unreadable.unreadable = Some("record version 2 is not 1".to_owned());
    let mut oldest_first = vec![
        version("w1aaaaaaaa", "2026-10-05T07:00:00.000Z", "learn", Some(one)),
        version("w2", "2026-10-06T07:00:00.000Z", "learn", Some(quiet)),
        version("w3", "2026-10-06T08:00:00.000Z", "learn", Some(revised)),
        version("w4", "2026-10-07T09:00:00.000Z", INGEST, Some(curated)),
        version("w5", "2026-10-07T09:30:00.000Z", INGEST, Some(checked)),
        version("w6", "2026-10-08T10:00:00.000Z", "", Some(chosen)),
        version("w7", "2026-10-08T11:00:00.000Z", "learn", None),
        unreadable,
    ];
    oldest_first.reverse();
    oldest_first
}

#[test]
fn the_note_of_a_write_names_who_wrote_it() {
    assert_eq!(
        writer_of(Some("learn")),
        Writer {
            origin: Origin::Learn,
            run: None,
            note: "learn".to_owned()
        }
    );
    let ingest = writer_of(Some(INGEST));
    assert_eq!(
        (ingest.origin, ingest.run.as_deref()),
        (Origin::Ingest, Some("40607"))
    );
    let learn = writer_of(Some("Intake 0001-0002-0000000000-learn"));
    assert_eq!(
        (learn.origin, learn.run.as_deref()),
        (Origin::Learn, Some("0"))
    );
    // Any other shape is shown as it is.
    for note in [
        "Fixture Lessons",
        "Intake x-1-2-learn",
        "Intake 1-2-3-other",
        "Intake 1-2-learn",
    ] {
        let writer = writer_of(Some(note));
        assert_eq!((writer.origin, writer.note.as_str()), (Origin::Other, note));
    }
    assert_eq!(writer_of(None).note, "");
}

#[test]
fn every_version_becomes_the_events_of_what_it_changed() {
    let history = History::new(&versions());
    let changes = history.changes;
    assert_eq!(changes.len(), 8);
    // Newest first, as the store gives them.
    let newest = &changes[0];
    assert_eq!(titles(newest), ["Version cannot be read"]);
    assert_eq!(
        newest.events[0].reason.as_deref(),
        Some("record version 2 is not 1")
    );
    assert_eq!(titles(&changes[1]), ["Removed"]);
    assert_eq!(titles(&changes[2]), ["Your choice", "Amended by an agent"]);
    assert_eq!(
        changes[2].events[0].sub.as_deref(),
        Some("You chose it as the lead 2 times")
    );
    assert_eq!(
        titles(&changes[3]),
        ["Seen in more containers", "Evidence added"]
    );
    let chips = |event: &Event| -> Vec<String> {
        event.chips.iter().map(|chip| chip.text.clone()).collect()
    };
    assert_eq!(chips(&changes[3].events[0]), ["+ github.com/acme/app"]);
    assert_eq!(chips(&changes[3].events[1]), ["+1 source", "+1 present"]);
    assert_eq!(changes[3].events[1].chips[0].tone, Tone::Added);

    let curated = &changes[4].events[0];
    assert_eq!(curated.title, "Curator: duplicate");
    assert_eq!(
        curated.other,
        Some(Other {
            label: "Duplicate of",
            id: "l0a1b2c3d4e5f00010a1b2c3d4e5f0001".to_owned()
        })
    );
    assert_eq!(
        curated.reason.as_deref(),
        Some("Says again what the asset says.")
    );

    let revised = &changes[5];
    assert_eq!(
        titles(revised),
        ["Revision 1 \u{2192} 2", "Status: candidate \u{2192} active"]
    );
    assert_eq!(
        revised.events[0].sub.as_deref(),
        Some("Changed: statement \u{b7} candidates 1 \u{2192} 3")
    );
    let statements = revised.events[0]
        .statements
        .clone()
        .expect("the statements");
    assert_ne!(statements.old, statements.new);
    assert_eq!(
        statements.new,
        "Format Go code with teamfmt before every commit."
    );
    assert_eq!(
        revised.events[1].sub.as_deref(),
        Some("seen in two containers")
    );

    assert!(changes[6].quiet);
    assert_eq!(titles(&changes[6]), ["Rewritten, no change"]);
    let created = &changes[7];
    assert_eq!(
        titles(created),
        ["Created \u{b7} revision 1 \u{b7} candidate"]
    );
    assert_eq!(
        created.events[0].sub.as_deref(),
        Some("Reduced from 1 candidate seen in gitlab.com/team/tool")
    );
    assert_eq!(
        (
            created.short_id.as_str(),
            created.day.as_str(),
            created.at.as_str()
        ),
        ("w1aaaaa", "2026-10-05", "2026-10-05T07:00:00.000Z")
    );
}

#[test]
fn a_curation_cleared_and_a_verdict_that_changes_say_so() {
    let mut keep = first();
    keep.curation = Curation {
        verdict: Some("keep".to_owned()),
        reason: Some("Fine.".to_owned()),
        ..Curation::default()
    }
    .into();
    let mut outdated = keep.clone();
    outdated.curation = Curation {
        verdict: Some("outdated".to_owned()),
        other: Some("l0a1b2c3d4e5f00040a1b2c3d4e5f0004".to_owned()),
        reason: Some("Replaced.".to_owned()),
        ..Curation::default()
    }
    .into();
    let mut cleared = outdated.clone();
    cleared.curation = Default::default();
    let history = History::new(&[
        version("c", "2026-10-07T00:00:00.000Z", "learn", Some(cleared)),
        version("b", "2026-10-06T00:00:00.000Z", "learn", Some(outdated)),
        version("a", "2026-10-05T00:00:00.000Z", "learn", Some(keep)),
    ]);
    assert_eq!(titles(&history.changes[0]), ["Curation cleared"]);
    let changed = &history.changes[1].events[0];
    assert_eq!(changed.title, "Curator: keep \u{2192} outdated");
    assert_eq!(
        changed.other.as_ref().map(|other| other.label),
        Some("Replaced by")
    );
}

#[test]
fn the_changes_are_grouped_by_run_and_day_and_the_quiet_ones_hide() {
    let history = History::new(&versions());
    assert_eq!((history.meaningful, history.quiet), (7, 1));
    let runs = |runs: &[Run]| -> Vec<(String, String, Vec<usize>)> {
        runs.iter()
            .map(|run| (run.title.clone(), run.day.clone(), run.changes.clone()))
            .collect()
    };
    let group = |title: &str, day: &str, changes: &[usize]| {
        (title.to_owned(), day.to_owned(), changes.to_vec())
    };
    assert_eq!(
        runs(&history.runs),
        [
            group("Learning run", "2026-10-09", &[0]),
            group("Learning run", "2026-10-08", &[1]),
            group("No note", "2026-10-08", &[2]),
            group("Agent sessions", "2026-10-07", &[3, 4]),
            group("Learning run", "2026-10-06", &[5]),
            group("Learning run", "2026-10-05", &[7]),
        ]
    );
    // With the quiet ones, the rewrite joins the revision of the same day and run.
    assert_eq!(
        runs(&history.runs_with_quiet)[4],
        group("Learning run", "2026-10-06", &[5, 6])
    );
    assert_eq!(history.runs[3].writer.run.as_deref(), Some("40607"));
    assert_eq!(history.since.as_deref(), Some("2026-10-05"));
    let empty = History::new(&[]);
    assert_eq!((empty.meaningful, empty.quiet, empty.since), (0, 0, None));
    assert!(empty.runs.is_empty() && empty.runs_with_quiet.is_empty());
}

#[test]
fn the_version_before_a_write_skips_what_cannot_be_read_and_stops_at_a_removal() {
    let versions = versions();
    // The newest cannot be read; the one before it is the removal.
    assert!(lesson_before(&versions, 0).is_none());
    // The removal: the version before it is the chosen one.
    assert_eq!(
        lesson_before(&versions, 1).and_then(|lesson| lesson.support.as_option()?.choices),
        Some(2)
    );
    // The first version has none before it.
    assert!(lesson_before(&versions, 7).is_none());
    let mut readable_between = versions.clone();
    readable_between.remove(1);
    let mut broken = readable_between[0].clone();
    broken.write_id = Some("broken".to_owned());
    readable_between.insert(1, broken);
    // An unreadable version is skipped on the way to the one before.
    assert!(lesson_before(&readable_between, 0).is_some());
}

#[test]
fn the_patch_of_a_write_is_a_unified_diff_of_the_record() {
    let versions = versions();
    let id = "l0a1b2c3d4e5f00020a1b2c3d4e5f0002";
    // A revision: the old statement goes, the new one comes.
    let patch = yaml_patch(
        id,
        lesson_before(&versions, 5),
        versions[5].lesson.as_option(),
    );
    assert!(patch.starts_with(&format!(
        "diff --git a/{id}.yaml b/{id}.yaml\n--- a/{id}.yaml\n+++ b/{id}.yaml\n"
    )));
    assert!(patch.contains("\n@@ "));
    assert!(patch.contains("+statement: Format Go code with teamfmt before every commit.\n"));
    assert!(patch.contains("-revision: 1\n") && patch.contains("+revision: 2\n"));

    // A creation is all additions, a removal all deletions.
    let created = yaml_patch(id, None, versions[7].lesson.as_option());
    assert!(created.lines().skip(4).all(|line| !line.starts_with('-')));
    assert!(created.contains("\n+id: "));
    let removed = yaml_patch(id, versions[2].lesson.as_option(), None);
    assert!(removed.lines().skip(4).all(|line| !line.starts_with('+')));
    assert!(removed.contains("\n-id: "));
}

#[test]
fn a_history_goes_to_the_frontend_in_camel_case_with_lowercase_names() {
    let value = serde_json::to_value(History::new(&versions())).expect("serializes");
    let newest = &value["changes"][0];
    assert_eq!(newest["events"][0]["kind"], "unreadable");
    assert_eq!(newest["writer"]["origin"], "learn");
    assert_eq!(value["runsWithQuiet"].as_array().map(Vec::len), Some(6));
    assert_eq!(
        value["changes"][4]["events"][0]["other"]["label"],
        "Duplicate of"
    );
    assert_eq!(
        value["changes"][5]["events"][0]["statements"]["old"],
        "Format Go code before every commit."
    );
    assert_eq!(
        value["changes"][3]["events"][0]["chips"][0]["tone"],
        "added"
    );
}
