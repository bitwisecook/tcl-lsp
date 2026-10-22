// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native protocol proof for inherited source state, deep views and retirement.

use crate::common::Lsp;
use crate::common::helpers::locations;
use serde_json::{Value, json};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Duration;

const TIMEOUT: Duration = Duration::from_secs(30);
const DEPTH: usize = 12;

fn file_uri(path: &Path) -> String {
    tcl_lsp_server::canonical_file_uri(path)
        .expect("representable fixture file URI")
        .to_string()
}

fn source_root() -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-deep-source-{}-{}",
        std::process::id(),
        line!()
    ));
    std::fs::create_dir_all(&root).expect("create deep-source workspace");
    root
}

fn namespace(root: &str) -> String {
    format!("::{root}{}", "::part".repeat(DEPTH))
}

fn entry_source(include_app: bool) -> String {
    let mut source = String::from(
        "namespace eval lib {proc héλper {} {return GOOD}}\nset ::sourceDir [file dirname [info script]]\n",
    );
    for root in ["app", "other"] {
        if root == "app" && !include_app {
            continue;
        }
        writeln!(source,
            "namespace eval {root} {{namespace path ::lib; source [file join $::sourceDir flat.tcl]; source [file join $::sourceDir hop1.tcl]}}"
        ).expect("writing a fixture String cannot fail");
        writeln!(
            source,
            "puts [::{root}::flat]\nputs [{}::run]",
            namespace(root)
        )
        .expect("writing a fixture String cannot fail");
    }
    source
}

fn write_source_chain(root: &Path) {
    std::fs::write(root.join("flat.tcl"), "proc flat {} {héλper}\n")
        .expect("write direct inherited-path child");
    std::fs::write(root.join("leaf.tcl"), "proc run {} {héλper}\n").expect("write deep leaf");
    for hop in 1..=DEPTH {
        let child = if hop == DEPTH {
            "leaf.tcl".to_owned()
        } else {
            format!("hop{}.tcl", hop + 1)
        };
        let load = if hop == 6 {
            // The global directory cell must cross more than five source hops.
            format!("source $::sourceDir/{child}")
        } else {
            format!("source {{{}}}", root.join(child).display())
        };
        let source = format!("namespace eval part {{namespace path ::lib; {load}}}\n");
        std::fs::write(root.join(format!("hop{hop}.tcl")), source)
            .expect("write composed source hop");
    }
}

fn run_fixture_oracle(entry: &Path, expected: &[u8]) {
    if let Some(oracle) = tcl_test_support::witness_tclsh(tcl_dialect::TclVersion::V8_6) {
        let result = tcl_test_support::run_script_file(&oracle.path, entry)
            .expect("matching C Tcl source-fixture execution");
        assert!(result.success(), "{:?}", result.stderr);
        assert_eq!(result.stdout, expected);
        assert!(result.stderr.is_empty(), "{:?}", result.stderr);
    }
}

fn assert_run_views(result: &Value, leaf_uri: &str, roots: &[&str]) {
    let mut containers = result
        .as_array()
        .expect("workspace symbols array")
        .iter()
        .filter(|symbol| symbol["name"] == "run")
        .map(|symbol| {
            assert_eq!(symbol["location"]["uri"], leaf_uri, "{symbol}");
            assert_eq!(
                symbol["location"]["range"]["start"],
                json!({"line":0,"character":5})
            );
            symbol["containerName"]
                .as_str()
                .expect("source-view namespace container")
                .to_owned()
        })
        .collect::<Vec<_>>();
    containers.sort();
    let mut expected = roots.iter().map(|root| namespace(root)).collect::<Vec<_>>();
    expected.sort();
    assert_eq!(containers, expected, "first complete source-view inventory");
}

fn assert_definition(result: &Value, target_uri: &str, line: u32, character: u32) {
    let targets = locations(result);
    assert_eq!(targets.len(), 1, "{result}");
    assert_eq!(targets[0].uri, target_uri);
    assert_eq!(
        targets[0].range["start"],
        json!({"line":line,"character":character})
    );
}

fn position(source: &str, needle: &str) -> (u32, u32) {
    let offset = source.find(needle).expect("fixture occurrence");
    let prefix = &source[..offset];
    (
        u32::try_from(prefix.bytes().filter(|&byte| byte == b'\n').count()).unwrap(),
        u32::try_from(
            prefix
                .rsplit_once('\n')
                .map_or(prefix, |(_, tail)| tail)
                .encode_utf16()
                .count(),
        )
        .unwrap(),
    )
}

#[test]
fn first_source_inventory_carries_twelve_composed_views_and_retires_removed_edge() {
    let root = source_root();
    write_source_chain(&root);
    let entry = root.join("root.tcl");
    let original = entry_source(true);
    std::fs::write(&entry, &original).expect("write source entry point");
    run_fixture_oracle(&entry, b"GOOD\nGOOD\nGOOD\nGOOD\n");
    let entry_uri = file_uri(&entry);
    let flat_uri = file_uri(&root.join("flat.tcl"));
    let leaf_uri = file_uri(&root.join("leaf.tcl"));
    let mut lsp = Lsp::with_config_at_root(json!({"dialect":"tcl8.6"}), &root);
    lsp.await_log(&["workspace_folders_scan"], TIMEOUT, 0);
    lsp.open_document(&entry_uri, &original);
    let _ = lsp.current_complete_diagnostics(&entry_uri, 1, &original, TIMEOUT);

    // The first naming query must see the complete inventory. Repeating it
    // would let another four-round reconciliation hide a depth limit.
    assert_run_views(&lsp.workspace_symbols("run"), &leaf_uri, &["app", "other"]);
    for root in ["app", "other"] {
        let call = format!("{}::run", namespace(root));
        let (line, character) = position(&original, &call);
        assert_definition(
            &lsp.definition(&entry_uri, line, character),
            &leaf_uri,
            0,
            5,
        );
    }
    // flat.tcl stays unopened; both source views share the root's inherited
    // path and therefore the same Unicode helper declaration.
    assert_definition(&lsp.definition(&flat_uri, 0, 15), &entry_uri, 0, 25);

    let changed = entry_source(false);
    std::fs::write(&entry, &changed).expect("write changed source entry point");
    run_fixture_oracle(&entry, b"GOOD\nGOOD\n");
    lsp.replace_document(&entry_uri, 2, &changed);
    let _ = lsp.current_complete_diagnostics(&entry_uri, 2, &changed, TIMEOUT);
    assert_run_views(&lsp.workspace_symbols("run"), &leaf_uri, &["other"]);
    let call = format!("{}::run", namespace("other"));
    let (line, character) = position(&changed, &call);
    assert_definition(
        &lsp.definition(&entry_uri, line, character),
        &leaf_uri,
        0,
        5,
    );
    lsp.close_document(&entry_uri);
    drop(lsp);
    std::fs::remove_dir_all(root).expect("remove source fixture files");
}
