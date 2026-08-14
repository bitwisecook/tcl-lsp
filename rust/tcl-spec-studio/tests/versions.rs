// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Version ranges derived from a synthetic package's three releases.
//!
//! One fixture package carries every case the importer has to get right: a
//! command that never changes, one that arrives, one that leaves, one that
//! leaves and comes back, an option and a closed value that join late, a doc
//! comment that starts saying "deprecated", and a signature that grows.

use serde_json::{Value, json};
use tcl_spec_studio::infer::{Inferred, SourceFile, import_package};
use tcl_spec_studio::versions::{
    VERSION_GATE_NOTE, VersionedImport, VersionedImportOptions, VersionedSnapshot,
    import_package_versions,
};

/// 1.0 — the earliest release we have sources for.
const V1_0: &str = r"package provide fakepkg 1.0
# Present in every analysed release.
proc fakepkg::stable {a} { return $a }
# Removed in 1.2.
proc fakepkg::doomed {} { return 1 }
# Removed in 1.1, back in 1.2.
proc fakepkg::flaky {} { return 1 }
# Grows an optional parameter in 1.2.
proc fakepkg::grew {a} { return $a }
# Options come from the body's dispatch.
proc fakepkg::render {args} {
    foreach opt $args {
        switch -- $opt {
            -quiet { set quiet 1 }
        }
    }
}
# A closed value set that gains a member in 1.2.
proc fakepkg::encode {mode text} {
    switch -- $mode {
        ascii { return $text }
        latin1 { return $text }
    }
}
";

/// 1.1 — `flaky` gone, `fresh` new, `-width` new, `stable` now says deprecated.
const V1_1: &str = r"package provide fakepkg 1.1
# Deprecated: use fakepkg::fresh instead.
proc fakepkg::stable {a} { return $a }
# Removed in 1.2.
proc fakepkg::doomed {} { return 1 }
# Grows an optional parameter in 1.2.
proc fakepkg::grew {a} { return $a }
# Brand new in this release.
proc fakepkg::fresh {} { return 1 }
# Options come from the body's dispatch.
proc fakepkg::render {args} {
    foreach opt $args {
        switch -- $opt {
            -quiet { set quiet 1 }
            -width { set w [lindex $args [incr i]] }
        }
    }
}
# A closed value set that gains a member in 1.2.
proc fakepkg::encode {mode text} {
    switch -- $mode {
        ascii { return $text }
        latin1 { return $text }
    }
}
";

/// 1.2 — `doomed` gone, `flaky` back, `utf-8` accepted, `grew` grew.
const V1_2: &str = r"package provide fakepkg 1.2
# Deprecated: use fakepkg::fresh instead.
proc fakepkg::stable {a} { return $a }
# Removed in 1.1, back in 1.2.
proc fakepkg::flaky {} { return 1 }
# Grows an optional parameter in 1.2.
proc fakepkg::grew {a {b 1}} { return $a$b }
# Brand new in 1.1.
proc fakepkg::fresh {} { return 1 }
# Options come from the body's dispatch.
proc fakepkg::render {args} {
    foreach opt $args {
        switch -- $opt {
            -quiet { set quiet 1 }
            -width { set w [lindex $args [incr i]] }
        }
    }
}
# A closed value set that gains a member in 1.2.
proc fakepkg::encode {mode text} {
    switch -- $mode {
        ascii { return $text }
        latin1 { return $text }
        utf-8 { return $text }
    }
}
";

fn snapshot(version: &str, text: &str) -> VersionedSnapshot {
    VersionedSnapshot {
        version: version.to_owned(),
        files: vec![SourceFile {
            name: "pkg.tcl".to_owned(),
            text: text.to_owned(),
        }],
    }
}

fn releases() -> Vec<VersionedSnapshot> {
    vec![
        snapshot("1.0", V1_0),
        snapshot("1.1", V1_1),
        snapshot("1.2", V1_2),
    ]
}

fn imported(complete_history: bool) -> VersionedImport {
    import_package_versions(
        &releases(),
        "tcl9.0",
        &VersionedImportOptions { complete_history },
    )
}

fn find<'a>(import: &'a VersionedImport, name: &str) -> &'a Inferred {
    import
        .commands
        .iter()
        .find(|command| command.name == name)
        .unwrap_or_else(|| {
            panic!(
                "no draft for {name} in {:?}",
                import.commands.iter().map(|c| &c.name).collect::<Vec<_>>()
            )
        })
}

fn option_row<'a>(command: &'a Inferred, name: &str) -> &'a Value {
    command.draft["options"]
        .as_array()
        .expect("option rows")
        .iter()
        .find(|row| row["name"] == json!(name))
        .unwrap_or_else(|| panic!("no `{name}` row in {:?}", command.draft["options"]))
}

fn has_note(command: &Inferred, needle: &str) -> bool {
    command.notes.iter().any(|note| note.contains(needle))
}

#[test]
fn the_analysed_releases_are_reported_in_version_order() {
    let import = imported(false);
    assert_eq!(import.package.as_deref(), Some("fakepkg"));
    assert_eq!(import.versions, vec!["1.0", "1.1", "1.2"]);
}

#[test]
fn a_command_present_throughout_a_partial_history_has_no_introduction() {
    let import = imported(false);
    let stable = find(&import, "fakepkg::stable");
    assert_eq!(
        stable.draft["introduced_version"],
        Value::Null,
        "a partial history cannot witness an introduction"
    );
    assert_eq!(stable.draft["retired_version"], Value::Null);
    assert!(
        has_note(
            stable,
            "present in earliest analysed version 1.0; introduction unknown"
        ),
        "{:?}",
        stable.notes
    );
}

#[test]
fn a_complete_history_makes_the_earliest_release_the_introduction() {
    let import = imported(true);
    let stable = find(&import, "fakepkg::stable");
    assert_eq!(stable.draft["introduced_version"], json!("1.0"));
    assert_eq!(stable.draft["retired_version"], Value::Null);
}

#[test]
fn a_command_that_appears_later_is_introduced_there() {
    let import = imported(false);
    let fresh = find(&import, "fakepkg::fresh");
    assert_eq!(fresh.draft["introduced_version"], json!("1.1"));
    assert!(
        has_note(fresh, "absent in 1.0, present in 1.1 (pkg.tcl)"),
        "the evidence must name both releases and the file: {:?}",
        fresh.notes
    );
}

#[test]
fn a_command_that_disappears_is_retired_at_the_release_that_lacks_it() {
    let import = imported(true);
    let doomed = find(&import, "fakepkg::doomed");
    assert_eq!(doomed.draft["introduced_version"], json!("1.0"));
    assert_eq!(
        doomed.draft["retired_version"],
        json!("1.2"),
        "retirement is the exclusive bound — the first release without it"
    );
    assert!(
        has_note(doomed, "present in 1.1, absent in 1.2"),
        "{:?}",
        doomed.notes
    );
}

#[test]
fn a_command_that_comes_back_is_left_unbounded_with_a_warning() {
    let import = imported(true);
    let flaky = find(&import, "fakepkg::flaky");
    assert_eq!(
        flaky.draft["introduced_version"],
        Value::Null,
        "a hole in the presence pattern is not a range"
    );
    assert_eq!(flaky.draft["retired_version"], Value::Null);
    assert!(
        import
            .warnings
            .iter()
            .any(|w| w.contains("fakepkg::flaky") && w.contains("absent in 1.1")),
        "the gap release must be named: {:?}",
        import.warnings
    );
    assert!(has_note(flaky, "warning:"), "{:?}", flaky.notes);
}

#[test]
fn the_merged_draft_keeps_the_newest_shape_and_reports_the_change() {
    let import = imported(true);
    let grew = find(&import, "fakepkg::grew");
    assert_eq!(grew.draft["arity"]["min"], json!(1));
    assert_eq!(grew.draft["arity"]["max"], json!(2));
    assert!(
        has_note(grew, "arity 1..2 since 1.2; was 1 in 1.0–1.1"),
        "{:?}",
        grew.notes
    );
}

#[test]
fn an_option_that_appears_later_gets_its_own_introduction() {
    let import = imported(true);
    let render = find(&import, "fakepkg::render");
    assert_eq!(
        option_row(render, "-quiet")["introduced_version"],
        Value::Null
    );
    assert_eq!(
        option_row(render, "-width")["introduced_version"],
        json!("1.1")
    );
    assert_eq!(option_row(render, "-width")["retired_version"], Value::Null);
    assert!(
        has_note(
            render,
            "option `-width` of `fakepkg::render` introduced 1.1"
        ),
        "{:?}",
        render.notes
    );
}

#[test]
fn a_value_joining_a_closed_set_becomes_a_version_gate_note() {
    let import = imported(true);
    let encode = find(&import, "fakepkg::encode");
    assert!(
        has_note(
            encode,
            &format!(
                "{VERSION_GATE_NOTE} command=fakepkg::encode arg=0 value=utf-8 introduced=1.2"
            )
        ),
        "a command-level value gate has no draft field yet, so it must be a \
         structured note: {:?}",
        encode.notes
    );
    assert!(
        has_note(encode, "value `utf-8`"),
        "the human-readable evidence must be there too: {:?}",
        encode.notes
    );
    assert!(
        !encode
            .notes
            .iter()
            .any(|note| note.contains("value=ascii") || note.contains("value=latin1")),
        "values present in every release are not gated: {:?}",
        encode.notes
    );
}

#[test]
fn a_doc_comment_saying_deprecated_is_a_suggestion_only() {
    let import = imported(true);
    let stable = find(&import, "fakepkg::stable");
    assert_eq!(
        stable.draft["deprecated_version"],
        Value::Null,
        "deprecation is never derived structurally"
    );
    assert!(
        has_note(stable, "suggested deprecated_version 1.1"),
        "{:?}",
        stable.notes
    );
}

#[test]
fn out_of_order_snapshots_are_sorted_with_a_warning() {
    let mut shuffled = releases();
    shuffled.reverse();
    let import = import_package_versions(
        &shuffled,
        "tcl9.0",
        &VersionedImportOptions {
            complete_history: true,
        },
    );
    assert_eq!(import.versions, vec!["1.0", "1.1", "1.2"]);
    assert!(
        import
            .warnings
            .iter()
            .any(|w| w.contains("not given in ascending version order")),
        "{:?}",
        import.warnings
    );
    assert_eq!(
        find(&import, "fakepkg::fresh").draft["introduced_version"],
        json!("1.1"),
        "sorting must happen before anything is derived"
    );
}

#[test]
fn a_duplicate_snapshot_label_is_an_error_entry() {
    let snapshots = vec![snapshot("1.0", V1_0), snapshot("1.0", V1_1)];
    let import = import_package_versions(&snapshots, "tcl9.0", &VersionedImportOptions::default());
    assert_eq!(import.versions, vec!["1.0"]);
    assert!(
        import
            .warnings
            .iter()
            .any(|w| w.starts_with("error: duplicate snapshot version `1.0`")),
        "{:?}",
        import.warnings
    );
}

#[test]
fn a_label_the_sources_contradict_is_warned_about_but_still_trusted() {
    let snapshots = vec![snapshot("2.0", V1_0)];
    let import = import_package_versions(&snapshots, "tcl9.0", &VersionedImportOptions::default());
    assert_eq!(import.versions, vec!["2.0"]);
    assert!(
        import
            .warnings
            .iter()
            .any(|w| w.contains("snapshot labelled 2.0 declares") && w.contains("1.0")),
        "{:?}",
        import.warnings
    );
}

#[test]
fn the_json_carries_the_versions_the_commands_and_the_warnings() {
    let import = imported(true);
    let value = import.to_json();
    assert_eq!(value["package"], json!("fakepkg"));
    assert_eq!(value["versions"], json!(["1.0", "1.1", "1.2"]));
    let commands = value["commands"].as_array().expect("commands array");
    assert_eq!(commands.len(), import.commands.len());
    let doomed = commands
        .iter()
        .find(|c| c["name"] == json!("fakepkg::doomed"))
        .expect("doomed is in the JSON");
    assert_eq!(doomed["draft"]["retired_version"], json!("1.2"));
    assert!(!doomed["notes"].as_array().expect("notes").is_empty());
    assert!(!value["warnings"].as_array().expect("warnings").is_empty());
}

/// The single-snapshot importer is untouched: one snapshot still stamps its own
/// `package provide` version, because that is all one snapshot can say.
#[test]
fn the_single_snapshot_import_path_is_unchanged() {
    let import = import_package(
        &[SourceFile {
            name: "pkg.tcl".to_owned(),
            text: V1_0.to_owned(),
        }],
        "tcl9.0",
    );
    assert_eq!(import.package.as_deref(), Some("fakepkg"));
    assert_eq!(import.version.as_deref(), Some("1.0"));
    let stable = import
        .commands
        .iter()
        .find(|command| command.name == "fakepkg::stable")
        .expect("stable is drafted");
    assert_eq!(stable.draft["introduced_version"], json!("1.0"));
    assert_eq!(stable.draft["required_package"], json!("fakepkg"));
}
