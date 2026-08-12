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

//! SpecTcl spec packs, end to end (`docs/design/spec-packs.md`).
//!
//! The claim under test is the whole point of the feature: **a user with a
//! private Tcl library drops a `.tclspec` in their workspace and their own
//! commands start behaving like shipped ones** — no Rust toolchain, no server
//! rebuild. So these tests drive the real binary over JSON-RPC against a real
//! directory, and assert on what the editor would show: hover text, the
//! absence of an unknown-command diagnostic, and load notices squiggled on the
//! pack file itself.

use crate::common::helpers::hover_text;
use crate::common::Lsp;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// A pack declaring one command with enough detail to be recognisable in
/// hover: a summary, a synopsis, and a real arity.
const MYLIB_PACK: &str = r"
speclib mylib 1 {
    command mylib::with_var {
        arity 2..3
        arg 0 -role varwrite
        arg 1 -role body
        hover {
            summary  {Run a script with a caller variable bound.}
            synopsis {mylib::with_var varName script ?mode?}
            returns  {The script's result.}
        }
    }
}
";

fn workspace(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-spec-packs-{name}-{}-{:?}",
        std::process::id(),
        std::thread::current().id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).expect("workspace root");
    root
}

fn write(path: &Path, body: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent dir");
    }
    std::fs::write(path, body).expect("write");
}

fn file_uri(path: &Path) -> String {
    format!("file://{}", path.to_string_lossy())
}

/// Tell the server a pack file moved, exactly as a client's file watcher would.
fn notify_pack_changed(lsp: &mut Lsp, path: &Path, kind: i64) {
    lsp.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": file_uri(path), "type": kind }] }),
    );
}

const CREATED: i64 = 1;
const CHANGED: i64 = 2;

fn messages(diagnostics: &[Value]) -> Vec<String> {
    diagnostics
        .iter()
        .filter_map(|d| d.get("message").and_then(Value::as_str))
        .map(str::to_owned)
        .collect()
}

/// The headline case: a workspace pack makes an unknown command resolve, with
/// hover.
///
/// Without the pack, `mylib::with_var` is a command the server has never heard
/// of — W123, no hover. With a `.tclspec` under `.tcl-lsp/`, it hovers with the
/// pack's own summary and synopsis and stops being unknown. Nothing in the
/// document, the settings, or the language id says "pack": discovery finds it
/// by convention.
#[test]
fn a_workspace_pack_makes_an_unknown_command_resolve_with_hover() {
    let root = workspace("hover");
    write(&root.join(".tcl-lsp/mylib.tclspec"), MYLIB_PACK);
    let source = "mylib::with_var counter {\n    set counter 1\n}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let mut lsp = Lsp::with_config_at_root(
        json!({ "features": { "linkedEditingRange": true } }),
        &root,
    );
    let uri = file_uri(&doc);
    lsp.open_ready(&uri, source);

    let hover = hover_text(&lsp.hover(&uri, 0, 4));
    assert!(
        hover.contains("Run a script with a caller variable bound."),
        "hover came from the pack's own summary; got:\n{hover}"
    );
    assert!(
        hover.contains("mylib::with_var varName script ?mode?"),
        "hover carries the pack's synopsis; got:\n{hover}"
    );

    let diagnostics = lsp.await_diagnostics_settled(
        &uri,
        std::time::Duration::from_secs(10),
        |diags| !messages(diags).iter().any(|m| m.contains("with_var")),
    );
    assert!(
        !messages(&diagnostics)
            .iter()
            .any(|m| m.contains("with_var")),
        "a pack-declared command is not unknown: {:#?}",
        messages(&diagnostics)
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// The same command, with **no** pack in the workspace, is unknown — so the
/// test above is measuring the pack and not some pre-existing leniency.
#[test]
fn without_a_pack_the_same_command_is_unknown() {
    let root = workspace("no-pack");
    let source = "mylib::with_var counter {\n    set counter 1\n}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let mut lsp = Lsp::with_config_at_root(
        json!({ "features": { "linkedEditingRange": true } }),
        &root,
    );
    let uri = file_uri(&doc);
    let diagnostics = lsp.open_ready(&uri, source);
    let hover = hover_text(&lsp.hover(&uri, 0, 4));
    assert!(
        !hover.contains("Run a script with a caller variable bound."),
        "no pack, no pack hover; got:\n{hover}"
    );
    assert!(
        messages(&diagnostics)
            .iter()
            .any(|m| m.contains("with_var")),
        "expected an unknown-command diagnostic, got {:#?}",
        messages(&diagnostics)
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A pack file appearing after startup is picked up from the watcher event,
/// and the open document is re-analysed against it.
#[test]
fn a_pack_added_after_startup_reloads_and_re_analyses_open_documents() {
    let root = workspace("reload");
    let source = "mylib::with_var counter {\n    set counter 1\n}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let mut lsp = Lsp::with_config_at_root(
        json!({ "features": { "linkedEditingRange": true } }),
        &root,
    );
    let uri = file_uri(&doc);
    let before = lsp.open_ready(&uri, source);
    assert!(
        messages(&before).iter().any(|m| m.contains("with_var")),
        "unknown before the pack exists: {:#?}",
        messages(&before)
    );

    let pack = root.join(".tcl-lsp/mylib.tclspec");
    write(&pack, MYLIB_PACK);
    notify_pack_changed(&mut lsp, &pack, CREATED);

    let after = lsp.await_diagnostics_settled(
        &uri,
        std::time::Duration::from_secs(15),
        |diags| !messages(diags).iter().any(|m| m.contains("with_var")),
    );
    assert!(
        !messages(&after).iter().any(|m| m.contains("with_var")),
        "the command is known once the pack loads: {:#?}",
        messages(&after)
    );
    let hover = hover_text(&lsp.hover(&uri, 0, 4));
    assert!(
        hover.contains("Run a script with a caller variable bound."),
        "and hovers from the pack; got:\n{hover}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Load notices land as diagnostics **on the pack file**, at the line the
/// author wrote — and clear when the author fixes them.
#[test]
fn pack_load_notices_are_diagnostics_on_the_pack_file() {
    let root = workspace("notices");
    let pack = root.join(".tcl-lsp/mylib.tclspec");
    write(
        &pack,
        "speclib mylib 1 {\n    command mylib::x {\n        arity 1\n        \
         nonsense_property yes\n    }\n}\n",
    );

    let mut lsp = Lsp::with_config_at_root(
        json!({ "features": { "linkedEditingRange": true } }),
        &root,
    );
    let pack_uri = file_uri(&pack);

    let diagnostics = lsp.await_diagnostics_settled(
        &pack_uri,
        std::time::Duration::from_secs(15),
        |diags| !diags.is_empty(),
    );
    let notice = diagnostics
        .iter()
        .find(|d| {
            d.get("message")
                .and_then(Value::as_str)
                .is_some_and(|m| m.contains("nonsense_property"))
        })
        .unwrap_or_else(|| panic!("expected a notice about the unknown property: {diagnostics:#?}"));
    assert_eq!(
        notice.get("code").and_then(Value::as_str),
        Some("SPECTCL"),
        "notices carry the pack-load code"
    );
    assert_eq!(
        notice
            .get("range")
            .and_then(|r| r.get("start"))
            .and_then(|p| p.get("line"))
            .and_then(Value::as_i64),
        Some(3),
        "on the line the author wrote it (1-based line 4)"
    );

    // Fix the pack: the badge must clear, not linger until restart.
    write(
        &pack,
        "speclib mylib 1 {\n    command mylib::x {\n        arity 1\n    }\n}\n",
    );
    notify_pack_changed(&mut lsp, &pack, CHANGED);
    let cleared = lsp.await_diagnostics_settled(
        &pack_uri,
        std::time::Duration::from_secs(15),
        |diags| diags.is_empty(),
    );
    assert!(cleared.is_empty(), "{cleared:#?}");

    let _ = std::fs::remove_dir_all(&root);
}

/// `tclLsp.specPacks` reaches a pack that convention would never find — a
/// directory that is neither `.tcl-lsp/` nor beside a `tclpkg.tcl`.
#[test]
fn the_spec_packs_setting_reaches_a_pack_convention_would_miss() {
    let root = workspace("setting");
    write(&root.join("vendor/specs/mylib.tclspec"), MYLIB_PACK);
    let source = "mylib::with_var counter {\n    set counter 1\n}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let mut lsp = Lsp::with_config_at_root(
        json!({
            "features": { "linkedEditingRange": true },
            "specPacks": ["vendor/specs"],
        }),
        &root,
    );
    let uri = file_uri(&doc);
    lsp.open_ready(&uri, source);

    let hover = hover_text(&lsp.hover(&uri, 0, 4));
    assert!(
        hover.contains("Run a script with a caller variable bound."),
        "the configured directory was discovered; got:\n{hover}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A pack beside a `tclpkg.tcl` manifest is found too — the rule that lets a
/// library ship its own specs next to its code.
#[test]
fn a_pack_beside_a_package_manifest_is_discovered() {
    let root = workspace("manifest");
    write(&root.join("lib/tclpkg.tcl"), "package require Tcl 8.6\n");
    write(&root.join("lib/mylib.tclspec"), MYLIB_PACK);
    let source = "mylib::with_var counter {\n    set counter 1\n}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let mut lsp = Lsp::with_config_at_root(
        json!({ "features": { "linkedEditingRange": true } }),
        &root,
    );
    let uri = file_uri(&doc);
    lsp.open_ready(&uri, source);

    let hover = hover_text(&lsp.hover(&uri, 0, 4));
    assert!(
        hover.contains("Run a script with a caller variable bound."),
        "found beside the manifest; got:\n{hover}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Two files naming one `speclib` merge into one pack, and both commands are
/// live — the "a pack is a logical unit, not a file" rule, observed from the
/// editor rather than from the loader's unit tests.
#[test]
fn a_multi_file_pack_merges_and_both_commands_resolve() {
    let root = workspace("multifile");
    write(
        &root.join(".tcl-lsp/alpha.tclspec"),
        "speclib mylib 1 {\n  command mylib::alpha {\n    arity 1\n    \
         hover { summary {The alpha command.} }\n  }\n}\n",
    );
    write(
        &root.join(".tcl-lsp/beta.tclspec"),
        "speclib mylib 1 {\n  command mylib::beta {\n    arity 1\n    \
         hover { summary {The beta command.} }\n  }\n}\n",
    );
    let source = "mylib::alpha one\nmylib::beta two\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let mut lsp = Lsp::with_config_at_root(
        json!({ "features": { "linkedEditingRange": true } }),
        &root,
    );
    let uri = file_uri(&doc);
    lsp.open_ready(&uri, source);

    assert!(hover_text(&lsp.hover(&uri, 0, 4)).contains("The alpha command."));
    assert!(hover_text(&lsp.hover(&uri, 1, 4)).contains("The beta command."));

    let _ = std::fs::remove_dir_all(&root);
}

/// A pack that redeclares a shipped command does not change it — and says so
/// on the pack file. The one guarantee that keeps a private pack from quietly
/// redefining the standard library.
#[test]
fn a_pack_cannot_silently_redefine_a_shipped_command() {
    let root = workspace("collision");
    let pack = root.join(".tcl-lsp/loud.tclspec");
    write(
        &pack,
        "speclib loud 1 {\n  command lsort {\n    arity 99\n    \
         hover { summary {Definitely not lsort.} }\n  }\n}\n",
    );
    let source = "lsort {c a b}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let mut lsp = Lsp::with_config_at_root(
        json!({ "features": { "linkedEditingRange": true } }),
        &root,
    );
    let uri = file_uri(&doc);
    let diagnostics = lsp.open_ready(&uri, source);

    let hover = hover_text(&lsp.hover(&uri, 0, 1));
    assert!(
        !hover.contains("Definitely not lsort."),
        "the shipped lsort wins; got:\n{hover}"
    );
    assert!(
        !messages(&diagnostics).iter().any(|m| m.contains("99")),
        "and the shipped arity is what `lsort {{c a b}}` is checked against: {:#?}",
        messages(&diagnostics)
    );

    let _ = std::fs::remove_dir_all(&root);
}
