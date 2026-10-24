// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual protocol control for physical and inherited interpretation owners.

use crate::common::Lsp;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(30);
const PARENT: &str = "namespace eval lib {proc héλper {} {return GOOD}}\nnamespace eval app {\n namespace path ::lib\n source [file join [file dirname [info script]] .. shared.tcl]\n}\nputs [::app::run]\n";
const CHILD: &str = "proc run {} {héλper}\n";
const RETIRED: &str = "# retired source edge\n";

struct Fixture {
    root: PathBuf,
    a: String,
    b: String,
    child: String,
}

fn uri(path: &Path) -> String {
    tcl_lsp_server::canonical_file_uri(path)
        .expect("fixture URI")
        .to_string()
}

fn oracle(path: &Path, version: tcl_dialect::TclVersion, output: &[u8]) {
    let engine = tcl_test_support::witness_tclsh(version).expect("matching C Tcl is required");
    let result = tcl_test_support::run_script_file(&engine.path, path).expect("C Tcl control");
    assert!(result.success(), "{:?}", result.stderr);
    assert_eq!(result.stdout, output);
    assert!(result.stderr.is_empty(), "{:?}", result.stderr);
}

impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tcl-lsp-source-owners-{}-{}",
            std::process::id(),
            line!()
        ));
        for (folder, dialect) in [("A", "tcl8.6"), ("B", "tcl9.0")] {
            std::fs::create_dir_all(root.join(folder)).expect("parent directory");
            std::fs::write(
                root.join(folder).join(".tcl-lsp.ini"),
                format!("[project]\ndialect = {dialect}\n"),
            )
            .expect("parent context");
            std::fs::write(root.join(folder).join("root.tcl"), PARENT)
                .expect("same original parent bytes");
        }
        std::fs::write(root.join("shared.tcl"), CHILD).expect("physical child");
        oracle(
            &root.join("A/root.tcl"),
            tcl_dialect::TclVersion::V8_6,
            b"GOOD\n",
        );
        oracle(
            &root.join("B/root.tcl"),
            tcl_dialect::TclVersion::V9_0,
            b"GOOD\n",
        );
        Self {
            a: uri(&root.join("A/root.tcl")),
            b: uri(&root.join("B/root.tcl")),
            child: uri(&root.join("shared.tcl")),
            root,
        }
    }

    fn ready(&self, lsp: &mut Lsp, a_version: i64, a_source: &str) {
        let _ = lsp.current_complete_diagnostics(&self.a, a_version, a_source, TIMEOUT);
        let _ = lsp.current_complete_diagnostics(&self.b, 1, PARENT, TIMEOUT);
        assert_eq!(lsp.effective_config(&self.a)["dialect_id"], "tcl8.6");
        assert_eq!(lsp.effective_config(&self.b)["dialect_id"], "tcl9.0");
        // The unopened child has its own ordinary physical context. Neither
        // inherited interpretation may replace it or borrow it as its input.
        assert_eq!(lsp.effective_config(&self.child)["dialect_id"], "tcl8.5");
    }
}

fn outgoing_parent(lsp: &mut Lsp, item: &Value) -> String {
    let result = lsp.outgoing_calls(item.clone());
    let rows = result.as_array().expect("positive source relationships");
    assert_eq!(rows.len(), 1, "{result}");
    assert_eq!(rows[0]["to"]["name"], "héλper");
    assert_eq!(
        rows[0]["to"]["selectionRange"]["start"],
        json!({"line":0,"character":25})
    );
    assert_eq!(
        rows[0]["fromRanges"],
        json!([{"start":{"line":0,"character":13},"end":{"line":0,"character":19}}])
    );
    rows[0]["to"]["uri"]
        .as_str()
        .expect("actual parent URI")
        .to_owned()
}

fn initial_handles(lsp: &mut Lsp, fixture: &Fixture) -> (Value, Value) {
    let result = lsp.prepare_call_hierarchy(&fixture.child, 0, 6);
    let rows = result
        .as_array()
        .expect("two retained child interpretations");
    assert_eq!(rows.len(), 2, "{result}");
    assert_ne!(rows[0]["data"], rows[1]["data"]);
    let mut a = None;
    let mut b = None;
    for item in rows {
        assert_eq!(item["uri"], fixture.child);
        assert_eq!(
            item["selectionRange"],
            json!({"start":{"line":0,"character":5},"end":{"line":0,"character":8}})
        );
        let parent = outgoing_parent(lsp, item);
        if parent == fixture.a {
            assert!(a.replace(item.clone()).is_none());
        } else {
            assert_eq!(parent, fixture.b);
            assert!(b.replace(item.clone()).is_none());
        }
    }
    (a.expect("actual A view"), b.expect("actual B view"))
}

fn assert_retired(lsp: &mut Lsp, item: &Value) {
    for result in [
        lsp.incoming_calls(item.clone()),
        lsp.outgoing_calls(item.clone()),
    ] {
        assert!(
            result.is_null() || result.as_array().is_some_and(Vec::is_empty),
            "retired A cannot borrow B: {result}"
        );
    }
}

#[test]
fn retained_child_views_keep_distinct_parent_inputs_and_refuse_retired_owner() {
    let fixture = Fixture::new();
    let mut lsp = Lsp::with_config_at_root(json!({"dialect":"tcl8.5"}), &fixture.root);
    lsp.await_log(&["workspace_folders_scan"], TIMEOUT, 0);
    lsp.open_document(&fixture.a, PARENT);
    lsp.open_document(&fixture.b, PARENT);
    fixture.ready(&mut lsp, 1, PARENT);
    let (old_a, _) = initial_handles(&mut lsp, &fixture);
    std::fs::write(fixture.root.join("A/root.tcl"), RETIRED).expect("retire only A");
    lsp.replace_document(&fixture.a, 2, RETIRED);
    fixture.ready(&mut lsp, 2, RETIRED);
    // First naming answers after independent completion use saved locators;
    // neither fresh preparation nor a query loop may repair the view table.
    assert_retired(&mut lsp, &old_a);
    let current = lsp.prepare_call_hierarchy(&fixture.child, 0, 6);
    let current = current
        .as_array()
        .expect("remaining actual B interpretation");
    assert_eq!(current.len(), 1);
    assert_eq!(outgoing_parent(&mut lsp, &current[0]), fixture.b);
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("B/root.tcl")).unwrap(),
        PARENT
    );
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("shared.tcl")).unwrap(),
        CHILD
    );
    std::fs::write(fixture.root.join("A/root.tcl"), PARENT).expect("restore exact A bytes");
    lsp.replace_document(&fixture.a, 3, PARENT);
    fixture.ready(&mut lsp, 3, PARENT);
    assert_retired(&mut lsp, &old_a);
    let (new_a, _) = initial_handles(&mut lsp, &fixture);
    assert_ne!(old_a["data"], new_a["data"]);
    drop(lsp);
    std::fs::remove_dir_all(fixture.root).expect("remove source-view controls");
}

const UNSUPPORTED_DECODE: &str =
    "source -encoding utf-8 [file join [file dirname [info script]] held.tcl]\n";
const INDEPENDENT: &str = "proc héλper {} {return GOOD}\nputs [héλper]\n";
const HELD_CHILD: &str = "proc held {} {return HELD}\nputs [held]\n";

struct FrontierFixture {
    root: PathBuf,
    pending: String,
    independent: String,
    held: String,
    potential_leaf: String,
}

impl FrontierFixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!(
            "tcl-lsp-frontier-scopes-{}-{}",
            std::process::id(),
            line!()
        ));
        std::fs::create_dir_all(&root).expect("scope fixture directory");
        for (name, text) in [
            ("pending.tcl", UNSUPPORTED_DECODE),
            ("independent.tcl", INDEPENDENT),
            ("held.tcl", HELD_CHILD),
            (
                "potential-d.tcl",
                "proc dormant {} {source [file join [file dirname [info script]] potential-a.tcl]}\n",
            ),
            (
                "potential-a.tcl",
                "proc unused {} {source [file join [file dirname [info script]] potential-b.tcl]}\n",
            ),
            (
                "potential-b.tcl",
                "proc helper {} {return POTENTIAL}\nputs [helper]\n",
            ),
            (
                "cycle-a.tcl",
                "source [file join [file dirname [info script]] cycle-b.tcl]\n",
            ),
            (
                "cycle-b.tcl",
                "source [file join [file dirname [info script]] cycle-a.tcl]\n",
            ),
        ] {
            std::fs::write(root.join(name), text).expect("scope fixture source");
        }
        // The C Tcl source operation is valid. The current source-only issuer
        // has no decoding receipt for optioned forms and must keep that debt.
        oracle(
            &root.join("pending.tcl"),
            tcl_dialect::TclVersion::V8_6,
            b"HELD\n",
        );
        oracle(
            &root.join("independent.tcl"),
            tcl_dialect::TclVersion::V8_6,
            b"GOOD\n",
        );
        Self {
            pending: uri(&root.join("pending.tcl")),
            independent: uri(&root.join("independent.tcl")),
            held: uri(&root.join("held.tcl")),
            potential_leaf: uri(&root.join("potential-b.tcl")),
            root,
        }
    }
}

fn assert_single_definition(result: &Value, expected_uri: &str, end: u32) {
    let rows = crate::common::helpers::locations(result);
    assert_eq!(rows.len(), 1, "{result}");
    assert_eq!(rows[0].uri, expected_uri);
    assert_eq!(
        rows[0].range,
        json!({
            "start":{"line":0,"character":5}, "end":{"line":0,"character":end}
        })
    );
}

fn assert_no_locations(result: &Value) {
    assert!(
        result.is_null() || result.as_array().is_some_and(Vec::is_empty),
        "{result}"
    );
}

#[test]
fn pending_source_scopes_and_rootless_cycles_keep_independent_first_answers() {
    let fixture = FrontierFixture::new();
    let mut lsp = Lsp::with_config_at_root(json!({"dialect":"tcl8.6"}), &fixture.root);
    lsp.await_log(&["workspace_folders_scan"], TIMEOUT, 0);
    lsp.open_document(&fixture.pending, UNSUPPORTED_DECODE);
    lsp.open_document(&fixture.independent, INDEPENDENT);
    let _ = lsp.current_complete_diagnostics(&fixture.pending, 1, UNSUPPORTED_DECODE, TIMEOUT);
    let _ = lsp.current_complete_diagnostics(&fixture.independent, 1, INDEPENDENT, TIMEOUT);
    // This is the first naming request. Unknown work in a distinct root and
    // a rootless component must not be repaired by a provider-query loop.
    let definition = lsp.definition(&fixture.independent, 1, 7);
    assert_single_definition(&definition, &fixture.independent, 11);
    // A positively associated child cannot borrow its private Fresh candidate
    // when the actual parent entry/decoding obligation is unavailable.
    assert_no_locations(&lsp.definition(&fixture.held, 1, 7));
    // D's deferred association to A supplies no actual A producer/inventory.
    // It cannot discharge private A's deferred association to B and lift B.
    assert_no_locations(&lsp.definition(&fixture.potential_leaf, 1, 7));
    // Prefix scheduling and positive local navigation do not establish whole
    // workspace/negative coverage while the other scopes remain pending.
    assert_no_locations(&lsp.references(&fixture.independent, 1, 7, true));
    std::fs::write(fixture.root.join("pending.tcl"), RETIRED).expect("remove held association");
    lsp.replace_document(&fixture.pending, 2, RETIRED);
    let _ = lsp.current_complete_diagnostics(&fixture.pending, 2, RETIRED, TIMEOUT);
    let definition = lsp.definition(&fixture.held, 1, 7);
    assert_single_definition(&definition, &fixture.held, 9);
    drop(lsp);
    std::fs::remove_dir_all(fixture.root).expect("remove scope controls");
}

mod path_advice_controls {
    use super::*;

    const QUALIFIED: &str = "source $::sourceDir/leaf.tcl\nputs [::path_probe]\n";
    const PURE_WRITE: &str = "set ::next [file join [file dirname [info script]] GOOD]\nsource $::next/leaf.tcl\nputs [::path_probe]\n";
    const MISSING: &str = "# no incoming directory advice\n";

    struct PathFixture {
        root: PathBuf,
        parent: String,
        child: String,
        parent_text: String,
        child_text: String,
        parent_version: i64,
        child_version: i64,
    }

    impl PathFixture {
        fn new(case: &str) -> Self {
            let root = std::env::temp_dir()
                .join(format!("tcl-lsp-path-owner-{}-{case}", std::process::id()));
            std::fs::create_dir_all(&root).expect("path fixture workspace");
            for marker in ["GOOD", "BAD"] {
                std::fs::create_dir_all(root.join(marker)).expect("path provider directory");
                std::fs::write(
                    root.join(marker).join("leaf.tcl"),
                    format!("proc ::path_probe {{}} {{return {marker}}}\n"),
                )
                .expect("distinct actual provider declaration");
            }
            let parent_text = Self::parent_source("GOOD", "");
            std::fs::write(root.join("root.tcl"), &parent_text).expect("physical parent");
            std::fs::write(root.join("shared.tcl"), CHILD).expect("physical child");
            Self {
                parent: uri(&root.join("root.tcl")),
                child: uri(&root.join("shared.tcl")),
                root,
                parent_text,
                child_text: CHILD.to_owned(),
                parent_version: 1,
                child_version: 1,
            }
        }

        fn parent_source(directory: &str, shadow: &str) -> String {
            let assignment = if directory.is_empty() {
                MISSING.to_owned()
            } else {
                format!("set ::sourceDir [file join [file dirname [info script]] {directory}]\n")
            };
            format!(
                "namespace eval lib {{proc héλper {{}} {{return GOOD}}}}\n{assignment}namespace eval app {{\n {shadow}\n namespace path ::lib\n source [file join [file dirname [info script]] shared.tcl]\n}}\nputs [::app::run]\n"
            )
        }

        fn start(&self) -> Lsp {
            let mut lsp = Lsp::with_config_at_root(json!({"dialect":"tcl8.6"}), &self.root);
            lsp.await_log(&["workspace_folders_scan"], TIMEOUT, 0);
            lsp.open_document(&self.parent, &self.parent_text);
            lsp.open_document(&self.child, &self.child_text);
            self.ready(&mut lsp);
            lsp
        }

        fn ready(&self, lsp: &mut Lsp) {
            let _ = lsp.current_complete_diagnostics(
                &self.parent,
                self.parent_version,
                &self.parent_text,
                TIMEOUT,
            );
            let _ = lsp.current_complete_diagnostics(
                &self.child,
                self.child_version,
                &self.child_text,
                TIMEOUT,
            );
        }

        fn change_child(&mut self, lsp: &mut Lsp, body: &str) {
            self.child_text = format!("{CHILD}{body}");
            self.child_version += 1;
            std::fs::write(self.root.join("shared.tcl"), &self.child_text).expect("changed child");
            lsp.replace_document(&self.child, self.child_version, &self.child_text);
            self.ready(lsp);
        }

        fn change_parent(&mut self, lsp: &mut Lsp, directory: &str, shadow: &str) {
            self.parent_text = Self::parent_source(directory, shadow);
            self.parent_version += 1;
            std::fs::write(self.root.join("root.tcl"), &self.parent_text).expect("changed parent");
            lsp.replace_document(&self.parent, self.parent_version, &self.parent_text);
            self.ready(lsp);
        }

        fn oracle(&self, expected: Option<&[u8]>) {
            let engine = tcl_test_support::witness_tclsh(tcl_dialect::TclVersion::V8_6)
                .expect("matching C Tcl8.6 is required");
            let result = std::process::Command::new(&engine.path)
                .arg(self.root.join("root.tcl"))
                .arg(self.root.join("BAD"))
                .output()
                .expect("matching source/argument control");
            if let Some(expected) = expected {
                assert!(result.status.success(), "{:?}", result.stderr);
                assert_eq!(result.stdout, expected);
                assert!(result.stderr.is_empty(), "{:?}", result.stderr);
            } else {
                assert!(
                    !result.status.success(),
                    "missing incoming variable must fail"
                );
                assert_eq!(result.stdout, [] as [u8; 0]);
                assert!(String::from_utf8_lossy(&result.stderr).contains("sourceDir"));
            }
        }

        fn probe(&self, lsp: &mut Lsp) -> Value {
            let line = self
                .child_text
                .lines()
                .position(|line| line == "puts [::path_probe]")
                .expect("exact original consumer call");
            lsp.definition(&self.child, u32::try_from(line).expect("small fixture"), 9)
        }

        fn target(&self, marker: &str) -> String {
            uri(&self.root.join(marker).join("leaf.tcl"))
        }
    }

    fn actual_child_handle(lsp: &mut Lsp, fixture: &PathFixture) -> Value {
        let result = lsp.prepare_call_hierarchy(&fixture.child, 0, 6);
        let rows = result
            .as_array()
            .expect("actual materialized inherited child");
        assert_eq!(rows.len(), 1, "{result}");
        let item = rows[0].clone();
        assert_eq!(item["uri"], fixture.child);
        assert_eq!(
            item["selectionRange"],
            json!({
                "start":{"line":0,"character":5},"end":{"line":0,"character":8}
            })
        );
        let handle = &item["data"]["originalDeclaration"];
        assert_eq!(handle["schema"], 5);
        assert_eq!(handle["wirePurpose"], "originalSourceViewRelationship");
        let session = handle["session"]
            .as_array()
            .expect("actual 16-byte wire session");
        assert_eq!(session.len(), 16);
        assert!(session.iter().all(|byte| {
            byte.as_u64()
                .is_some_and(|value| value <= u64::from(u8::MAX))
        }));
        assert!(handle["token"].as_u64().is_some_and(|token| token > 0));
        assert_eq!(outgoing_parent(lsp, &item), fixture.parent);
        item
    }

    fn exact_probe_target(result: &Value, target: &str) {
        let rows = crate::common::helpers::locations(result);
        assert_eq!(rows.len(), 1, "{result}");
        assert_eq!(rows[0].uri, target);
        assert_eq!(
            rows[0].range,
            json!({
                "start":{"line":0,"character":5},"end":{"line":0,"character":17}
            })
        );
    }

    fn unknown_or_actual_bad(result: &Value, fixture: &PathFixture) {
        let rows = crate::common::helpers::locations(result);
        assert!(
            rows.iter().all(|row| row.uri != fixture.target("GOOD")),
            "{result}"
        );
        if !rows.is_empty() {
            exact_probe_target(result, &fixture.target("BAD"));
        }
    }

    #[test]
    fn source_path_handles_require_current_complete_same_session_parent() {
        let mut fixture = PathFixture::new("saved-handles");
        fixture.oracle(Some(b"GOOD\n"));
        let mut lsp = fixture.start();
        let old = actual_child_handle(&mut lsp, &fixture);
        let mut foreign = fixture.start();
        let foreign_item = actual_child_handle(&mut foreign, &fixture);
        assert_ne!(old["data"], foreign_item["data"]);
        assert_retired(&mut lsp, &foreign_item);
        let mut missing = old.clone();
        missing["data"]["originalDeclaration"]
            .as_object_mut()
            .expect("wire object")
            .remove("session")
            .expect("actual saved session");
        assert_retired(&mut lsp, &missing);
        // Refused locators cannot retire the genuine current view or renew it.
        assert_eq!(outgoing_parent(&mut lsp, &old), fixture.parent);
        fixture.change_parent(&mut lsp, "BAD", "");
        fixture.oracle(Some(b"GOOD\n"));
        assert_retired(&mut lsp, &old);
        let changed = actual_child_handle(&mut lsp, &fixture);
        assert_ne!(old["data"], changed["data"]);
        drop(foreign);
        drop(lsp);
        std::fs::remove_dir_all(fixture.root).expect("remove saved handle controls");
    }

    #[test]
    fn qualified_source_path_uses_current_cut_and_variable_free_own_writes() {
        let mut fixture = PathFixture::new("qualified-positive");
        fixture.oracle(Some(b"GOOD\n"));
        let mut lsp = fixture.start();
        let old = actual_child_handle(&mut lsp, &fixture);
        fixture.change_child(&mut lsp, QUALIFIED);
        fixture.oracle(Some(b"GOOD\nGOOD\n"));
        assert_retired(&mut lsp, &old);
        // These are first answers after independent diagnostics, not retries.
        exact_probe_target(&fixture.probe(&mut lsp), &fixture.target("GOOD"));
        let qualified = actual_child_handle(&mut lsp, &fixture);
        fixture.change_parent(&mut lsp, "BAD", "");
        fixture.oracle(Some(b"BAD\nGOOD\n"));
        assert_retired(&mut lsp, &qualified);
        exact_probe_target(&fixture.probe(&mut lsp), &fixture.target("BAD"));
        let changed = actual_child_handle(&mut lsp, &fixture);
        assert_ne!(qualified["data"], changed["data"]);
        fixture.change_child(&mut lsp, PURE_WRITE);
        fixture.oracle(Some(b"GOOD\nGOOD\n"));
        assert_retired(&mut lsp, &changed);
        exact_probe_target(&fixture.probe(&mut lsp), &fixture.target("GOOD"));
        drop(lsp);
        std::fs::remove_dir_all(fixture.root).expect("remove qualified path controls");
    }

    struct RefusalCase {
        name: &'static str,
        body: &'static str,
        shadow: &'static str,
        directory: &'static str,
        output: Option<&'static [u8]>,
        may_derive_bad: bool,
    }

    const REFUSALS: &[RefusalCase] = &[
        RefusalCase {
            name: "unqualified-shadow",
            directory: "GOOD",
            shadow: "set dir [lindex $::argv 0]",
            body: "set ::dir [file join [file dirname [info script]] GOOD]\nsource $dir/leaf.tcl\nputs [::path_probe]\n",
            output: Some(b"BAD\nGOOD\n"),
            may_derive_bad: true,
        },
        RefusalCase {
            name: "intermediate-shadow",
            directory: "GOOD",
            shadow: "set dir [lindex $::argv 0]",
            body: "set ::dir [file join [file dirname [info script]] GOOD]\nset ::next $dir\nsource $::next/leaf.tcl\nputs [::path_probe]\n",
            output: Some(b"BAD\nGOOD\n"),
            may_derive_bad: true,
        },
        RefusalCase {
            name: "intermediate-absolute",
            directory: "GOOD",
            shadow: "",
            body: "set ::next $::sourceDir\nsource $::next/leaf.tcl\nputs [::path_probe]\n",
            output: Some(b"GOOD\nGOOD\n"),
            may_derive_bad: false,
        },
        RefusalCase {
            name: "effect-barrier",
            directory: "GOOD",
            shadow: "",
            body: "proc ::change {} {set ::sourceDir [file join [file dirname [info script]] BAD]}\n::change\nsource $::sourceDir/leaf.tcl\nputs [::path_probe]\n",
            output: Some(b"BAD\nGOOD\n"),
            may_derive_bad: true,
        },
        RefusalCase {
            name: "missing-incoming",
            directory: "",
            shadow: "",
            body: QUALIFIED,
            output: None,
            may_derive_bad: false,
        },
    ];

    #[test]
    fn source_path_shadows_intermediate_writes_and_effects_cannot_enable_wrong_good() {
        for case in REFUSALS {
            let mut fixture = PathFixture::new(case.name);
            fixture.oracle(Some(b"GOOD\n"));
            let mut lsp = fixture.start();
            let old = actual_child_handle(&mut lsp, &fixture);
            fixture.change_parent(&mut lsp, case.directory, case.shadow);
            fixture.change_child(&mut lsp, case.body);
            fixture.oracle(case.output);
            assert_retired(&mut lsp, &old);
            let result = fixture.probe(&mut lsp);
            if case.may_derive_bad {
                unknown_or_actual_bad(&result, &fixture);
            } else {
                assert_no_locations(&result);
            }
            drop(lsp);
            std::fs::remove_dir_all(fixture.root).expect("remove negative path controls");
        }
    }
}
