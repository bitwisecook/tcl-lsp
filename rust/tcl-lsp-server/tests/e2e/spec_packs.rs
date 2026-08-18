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

//! `SpecTcl` spec packs, end to end (`docs/design/spec-packs.md`).
//!
//! The claim under test is the whole point of the feature: **a user with a
//! private Tcl library drops a `.tclspec` in their workspace and their own
//! commands start behaving like shipped ones** — no Rust toolchain, no server
//! rebuild. So these tests drive the real binary over JSON-RPC against a real
//! directory, and assert on what the editor would show: hover text, the
//! absence of an unknown-command diagnostic, and load notices squiggled on the
//! pack file itself.

use crate::common::helpers::{decode_semantic_tokens, hover_text};
use crate::common::{Lsp, scaled_timeout};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// A pack declaring one command with enough detail to be recognisable in
/// hover: a summary, a synopsis, and a real arity.
const MYLIB_PACK: &str = r"
speclib mylib 1 {
    command mylib::with_var {
        arity 2..3
        arg 0 -role VarWrite
        arg 1 -role Body
        hover {
            summary  {Run a script with a caller variable bound.}
            synopsis {mylib::with_var varName script ?mode?}
            returns  {The script's result.}
        }
    }
}
";

/// A private on-disk workspace for one test.
///
/// The name is deliberately plain ASCII: these paths become `file://` URIs on
/// both sides of the protocol, and the server percent-encodes what
/// `Uri::from_file_path` says to. A directory named after `ThreadId(17)` is
/// then two different strings depending on who built the URI, which is a
/// property of the test fixture and not of anything worth testing.
fn workspace(name: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let unique = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-spec-packs-{name}-{}-{unique}",
        std::process::id()
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

/// The packs the server reports as loaded, from `tcl-lsp.getEffectiveConfig`.
///
/// The settle signal these tests wait on. W123 is deliberately *not* it: the
/// analyser skips the unknown-command check for any name containing `::`
/// ("qualified — defer to per-namespace logic, conservative skip"), so a
/// namespaced pack command never had a W123 to suppress in the first place.
/// What a pack demonstrably changes for a qualified name is what the server
/// *knows* about it — hover, signature, arity — so that is what is asserted,
/// with this as the barrier.
fn loaded_packs(lsp: &mut Lsp) -> Vec<Value> {
    lsp.effective_config("")
        .get("spec_packs_loaded")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// The names of every pack the server reports as loaded.
///
/// A workspace pack is not the only thing in the list: the server also
/// discovers the **bundled** tier (the loadables tcl-lsp ships), so a test
/// about *this* workspace's pack must name it rather than count.
fn pack_names(lsp: &mut Lsp) -> Vec<String> {
    loaded_packs(lsp)
        .iter()
        .filter_map(|pack| pack.get("name").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect()
}

/// Block until the server reports a loaded pack called `name` — the settle
/// barrier for a workspace pack, since the bundled tier makes "any pack at
/// all" true from the first reload.
fn await_pack_named(lsp: &mut Lsp, name: &str) {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while !pack_names(lsp).iter().any(|loaded| loaded == name) {
        assert!(
            std::time::Instant::now() < deadline,
            "the `{name}` pack was never reported as loaded; got {:?}",
            pack_names(lsp)
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
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

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&doc);
    await_pack_named(&mut lsp, "mylib");
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

    // Named, not counted: the bundled tier ships the EDA loadables, so the
    // list is never just this workspace's pack.
    let packs = loaded_packs(&mut lsp);
    let mine = packs
        .iter()
        .find(|pack| pack.get("name").and_then(Value::as_str) == Some("mylib"))
        .unwrap_or_else(|| panic!("the workspace pack: {packs:#?}"));
    assert_eq!(mine.get("tier").and_then(Value::as_str), Some("workspace"));

    let _ = std::fs::remove_dir_all(&root);
}

/// Pack argument roles must reach the memoised semantic-token query too, not
/// only hover and diagnostics. `Body` recurses into the script and `VarWrite`
/// paints the caller variable as a variable rather than a string.
#[test]
fn a_workspace_packs_argument_roles_drive_semantic_tokens() {
    let root = workspace("semantic-tokens");
    write(&root.join(".tcl-lsp/mylib.tclspec"), MYLIB_PACK);
    let source = "mylib::with_var counter {\n    set counter 1\n}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&doc);
    await_pack_named(&mut lsp, "mylib");
    lsp.open_ready(&uri, source);

    let legend: Vec<String> =
        lsp.initialize_result()["capabilities"]["semanticTokensProvider"]["legend"]["tokenTypes"]
            .as_array()
            .expect("semantic-token legend")
            .iter()
            .filter_map(Value::as_str)
            .map(ToOwned::to_owned)
            .collect();
    let tokens = decode_semantic_tokens(&lsp.semantic_tokens_settled(&uri));
    assert!(
        tokens.iter().any(|token| {
            token.line == 1
                && token.char == 4
                && legend[usize::try_from(token.ttype).expect("token type index")] == "function"
        }),
        "the inner `set` must be tokenised through the pack's Body role: {tokens:?}"
    );
    assert!(
        tokens.iter().any(|token| {
            token.line == 0
                && token.char == 16
                && legend[usize::try_from(token.ttype).expect("token type index")] == "variable"
        }),
        "the `counter` argument must use the pack's VarWrite role: {tokens:?}"
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

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&doc);
    lsp.open_ready(&uri, source);
    let hover = hover_text(&lsp.hover(&uri, 0, 4));
    assert!(
        !hover.contains("Run a script with a caller variable bound."),
        "no pack, no pack hover; got:\n{hover}"
    );
    // "No packs" means no *workspace* pack: the bundled tier (the shipped EDA
    // loadables) is always there, and is what this control is controlling for.
    assert!(
        !pack_names(&mut lsp).iter().any(|name| name == "mylib"),
        "and the server reports no `mylib` pack at all; got {:?}",
        pack_names(&mut lsp)
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

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&doc);
    lsp.open_ready(&uri, source);
    assert!(
        !hover_text(&lsp.hover(&uri, 0, 4)).contains("Run a script with a caller variable bound."),
        "nothing is known about the command before the pack exists"
    );
    assert!(!pack_names(&mut lsp).iter().any(|name| name == "mylib"));

    let pack = root.join(".tcl-lsp/mylib.tclspec");
    write(&pack, MYLIB_PACK);
    notify_pack_changed(&mut lsp, &pack, CREATED);

    await_pack_named(&mut lsp, "mylib");
    let hover = hover_text(&lsp.hover(&uri, 0, 4));
    assert!(
        hover.contains("Run a script with a caller variable bound."),
        "and hovers from the pack once it loads; got:\n{hover}"
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

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let pack_uri = file_uri(&pack);

    let diagnostics =
        lsp.await_diagnostics_settled(&pack_uri, std::time::Duration::from_secs(15), |diags| {
            !diags.is_empty()
        });
    let notice = diagnostics
        .iter()
        .find(|d| {
            d.get("message")
                .and_then(Value::as_str)
                .is_some_and(|m| m.contains("nonsense_property"))
        })
        .unwrap_or_else(|| {
            panic!("expected a notice about the unknown property: {diagnostics:#?}")
        });
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
        <[Value]>::is_empty,
    );
    assert!(cleared.is_empty(), "{cleared:#?}");

    let _ = std::fs::remove_dir_all(&root);
}

/// `tclLsp.diagnostics.exclude` (#1556) suppresses pack-load notices too.
///
/// Pack notices publish outside the analyser pipeline's exclusion gate, so
/// they need their own filter: without it, a `.tclspec` matching an exclude
/// glob keeps its `SPECTCL` badge indefinitely — the one diagnostic the
/// "produces no diagnostics at all" promise would still leak. Excluding the
/// pack after the notice is on screen also exercises the clearing path (the
/// config apply reloads packs, and the newly excluded URI joins the stale
/// set).
#[test]
fn diagnostics_exclude_suppresses_pack_load_notices() {
    let root = workspace("notices-excluded");
    let pack = root.join(".tcl-lsp/mylib.tclspec");
    write(
        &pack,
        "speclib mylib 1 {\n    command mylib::x {\n        arity 1\n        \
         nonsense_property yes\n    }\n}\n",
    );

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let pack_uri = file_uri(&pack);

    let diagnostics =
        lsp.await_diagnostics_settled(&pack_uri, std::time::Duration::from_secs(15), |diags| {
            !diags.is_empty()
        });
    assert!(
        diagnostics.iter().any(|d| {
            d.get("message")
                .and_then(Value::as_str)
                .is_some_and(|m| m.contains("nonsense_property"))
        }),
        "expected a notice about the unknown property before excluding: {diagnostics:#?}"
    );

    // Exclude every pack file: the badge must clear without touching the pack.
    lsp.apply_configuration(json!({ "diagnostics": { "exclude": ["*.tclspec"] } }));
    let cleared = lsp.await_diagnostics_settled(
        &pack_uri,
        std::time::Duration::from_secs(15),
        <[Value]>::is_empty,
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

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&doc);
    await_pack_named(&mut lsp, "mylib");
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

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&doc);
    await_pack_named(&mut lsp, "mylib");
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

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&doc);
    let diagnostics = lsp.open_ready(&uri, source);

    let hover = hover_text(&lsp.hover(&uri, 0, 1));
    assert!(
        !hover.contains("Definitely not lsort."),
        "the shipped lsort wins; got:\n{hover}"
    );
    assert!(
        hover.contains("lsort"),
        "and the shipped hover is what is shown; got:\n{hover}"
    );
    // The pack claimed `arity 99`; if it had won, this perfectly ordinary
    // `lsort {c a b}` would be an arity error.
    let arity_complaints: Vec<&Value> = diagnostics
        .iter()
        .filter(|d| {
            d.get("message")
                .and_then(Value::as_str)
                .is_some_and(|m| m.contains("99"))
        })
        .collect();
    assert!(
        arity_complaints.is_empty(),
        "the shipped arity is what the call is checked against: {arity_complaints:#?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A pack whose `const_fold` is a **Tcl body**, folding a real call site in the
/// running server.
///
/// This is the whole hook feature in one file: the loader carries the body,
/// `tcl_spectcl::hooks` binds it to a registry slot, the worker thread that
/// runs the optimiser builds a sandboxed VM host for it, and the optimiser —
/// which knows nothing about packs — reads `spec.const_fold` exactly as it
/// reads a shipped one.
const FOLDER_PACK: &str = r"
speclib folder 1 {
    command folder::strlen {
        arity 1
        arg 0 -role Value
        const_fold -inputs {words} {words ctx} {
            set subject [lindex $words 0]
            if {![string is ascii $subject]} { return }
            fold [string length $subject]
        }
    }
}
";

/// A call site the optimiser will fold if — and only if — the pack's body runs.
const FOLDER_SOURCE: &str = "proc ::show {} {\n    puts [folder::strlen abcde]\n}\n::show\n";

/// The `source` field of an `optimiseDocument` reply.
fn optimised(result: &Value) -> String {
    result
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned()
}

/// Every optimisation offer of `code` in an `optimiseDocument` reply, as
/// `(replacement, message)`.
fn offers(result: &Value, code: &str) -> Vec<(String, String)> {
    result
        .get("optimisations")
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter(|item| item.get("code").and_then(Value::as_str) == Some(code))
                .map(|item| {
                    (
                        item.get("replacement")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                        item.get("message")
                            .and_then(Value::as_str)
                            .unwrap_or_default()
                            .to_owned(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn a_pack_const_fold_body_folds_a_call_site_in_the_optimiser() {
    let root = workspace("const-fold");
    write(&root.join(".tcl-lsp/folder.tclspec"), FOLDER_PACK);
    let doc = root.join("app.tcl");
    write(&doc, FOLDER_SOURCE);

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&doc);
    lsp.open_ready(&uri, FOLDER_SOURCE);
    await_pack_named(&mut lsp, "folder");

    let result = lsp.execute_command("tcl-lsp.optimiseDocument", json!([uri, "full"]));
    assert!(!result.is_null(), "the optimiser answered");
    let folded = offers(&result, "O129");
    assert!(
        folded.iter().any(|(replacement, _)| replacement == "5"),
        "the pack's Tcl body computed `string length abcde`; offers were {folded:#?}\n\
         optimised source:\n{}",
        optimised(&result)
    );
    assert!(
        optimised(&result).contains("puts 5"),
        "and the rewrite applies it; got:\n{}",
        optimised(&result)
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// The control: the identical document with **no** pack in the workspace does
/// not fold, so the test above is measuring the hook body and not some
/// pre-existing constant folder.
#[test]
fn without_the_pack_the_same_call_site_does_not_fold() {
    let root = workspace("const-fold-none");
    let doc = root.join("app.tcl");
    write(&doc, FOLDER_SOURCE);

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&doc);
    lsp.open_ready(&uri, FOLDER_SOURCE);
    assert!(
        !pack_names(&mut lsp).iter().any(|name| name == "folder"),
        "there is no workspace pack here"
    );

    let result = lsp.execute_command("tcl-lsp.optimiseDocument", json!([uri, "full"]));
    assert!(
        offers(&result, "O129")
            .iter()
            .all(|(replacement, _)| replacement != "5"),
        "nothing folds an unknown command: {result:#?}"
    );
    assert!(
        !optimised(&result).contains("puts 5"),
        "and the source is untouched; got:\n{}",
        optimised(&result)
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// The **bundled** tier, end to end: the EDA vendor libraries are `.tclspec`
/// loadables now, not compiled-in Rust (`docs/design/spec-packs.md`), so this
/// is the proof that a shipped pack reaches the analyser in the real server
/// process — a Vivado command has to be discovered on disk, parsed, merged,
/// installed, and its overlay key published to the analyser before
/// `synth_design` can read as anything but an unknown command.
///
/// The negative half matters as much: every vendor's pack is discovered for
/// every dialect, so a rival's command must still be unknown here.
#[test]
fn the_bundled_eda_loadables_make_their_vendor_commands_known() {
    let root = workspace("bundled-eda");
    let source = "# tcl-dialect: xilinx-eda-tcl\nsynth_design -top top\nget_cells *\nvsim -c top\n";
    let doc = root.join("build.tcl");
    write(&doc, source);

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    await_pack_named(&mut lsp, "eda_xilinx");
    assert!(
        pack_names(&mut lsp).iter().any(|name| name == "sdc_base"),
        "the shared SDC library ships alongside it; got {:?}",
        pack_names(&mut lsp)
    );

    let uri = file_uri(&doc);
    lsp.open_ready(&uri, source);

    // Settle rather than take the first publish. The analyser resolves its own
    // registry and only *looks up* the pack-carrying entry by key
    // (`Analyser::with_pack_overlay` / `profile_registry`), falling back to the
    // un-overlaid registry when that entry has not been built yet. So an
    // analysis racing workspace init can legitimately report every vendor
    // command unknown, and does under a loaded runner — the honest answer for
    // the instant it ran, corrected by the re-analysis that follows. What this
    // test is about is the settled answer, so wait for it; `settled` panics on
    // timeout, so a pack that never installs still fails here.
    let unknown_of = |diags: &[Value]| -> Vec<String> {
        diags
            .iter()
            .filter(|d| matches!(d.get("code").and_then(Value::as_str), Some("W123" | "W002")))
            .filter_map(|d| d.get("message").and_then(Value::as_str))
            .map(ToOwned::to_owned)
            .collect()
    };
    let diagnostics =
        lsp.await_diagnostics_settled(&uri, std::time::Duration::from_secs(30), |diags| {
            !unknown_of(diags).iter().any(|m| m.contains("synth_design"))
        });
    let unknown = unknown_of(&diagnostics);

    assert!(
        !unknown.iter().any(|m| m.contains("synth_design")),
        "the bundled Vivado pack must make `synth_design` known: {unknown:?}"
    );
    assert!(
        !unknown.iter().any(|m| m.contains("get_cells")),
        "and the bundled SDC pack `get_cells`: {unknown:?}"
    );
    assert!(
        unknown.iter().any(|m| m.contains("vsim")),
        "but a Questa command must not resolve under Vivado: {diagnostics:#?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A pack that appears **while the startup reload is still running** must not
/// be lost.
///
/// Two reloads overlap here: the one `initialized` starts, which parses the
/// bundled EDA loadables and is the slower of the two, and the one this
/// notification starts. Each takes its own filesystem snapshot, so the startup
/// reload's view predates the file. When it finished second it republished
/// that view and the workspace pack vanished — and because the file was
/// already on disk with its one event consumed, nothing ever re-triggered:
/// the pack stayed missing for the session.
///
/// This is the **fast-path smoke test** for that scenario: no settling step,
/// so the two reloads overlap if the machine lets them, and no artificial
/// timing. It is not by itself a proof — whether they actually overlap depends
/// on how long the bundled load takes on the box running it, and on a fast
/// idle machine the startup reload has already finished by the time the
/// notification arrives, so it passes either way. What was observed in CI is a
/// pack set of exactly the six bundled EDA packs: the startup snapshot,
/// verbatim.
///
/// The proof is its neighbour,
/// [`a_stale_startup_snapshot_cannot_overwrite_a_newer_pack_set`], which forces
/// the interleaving through the server's hold seam and fails without the fix.
/// This one is kept because it is the version with no seam in it: it exercises
/// the real startup path with real timings, so a refactor that moves the pack
/// load out from under the ordering guarantees still has to keep the ordinary
/// case working.
#[test]
fn a_pack_created_during_the_startup_reload_survives_it() {
    let root = workspace("startup-race");
    let source = "mylib::with_var counter {\n    set counter 1\n}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);

    // No settling step: write and notify immediately, so the notification's
    // reload races the one still in flight from `initialized`.
    let pack = root.join(".tcl-lsp/mylib.tclspec");
    write(&pack, MYLIB_PACK);
    notify_pack_changed(&mut lsp, &pack, CREATED);

    await_pack_named(&mut lsp, "mylib");

    let uri = file_uri(&doc);
    lsp.open_ready(&uri, source);
    let hover = hover_text(&lsp.hover(&uri, 0, 4));
    assert!(
        hover.contains("Run a script with a caller variable bound."),
        "the pack that won the race is the one on disk; got:\n{hover}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// How long the server sits on the startup reload's filesystem snapshot, via
/// its `TCL_LSP_TEST_STARTUP_RELOAD_HOLD_MS` seam.
///
/// The window the notification's reload has to land in. A full reload —
/// discovery plus the bundled load — measures ~200ms on this tree, so this is
/// several times what it needs. Not load-scaled: it is a *hold*, not a
/// deadline, and it is anchored to the server's own "snapshot taken" log
/// rather than to wall-clock guesswork, so a busy machine eats into the margin
/// rather than the premise.
const STARTUP_HOLD: std::time::Duration = std::time::Duration::from_millis(1500);

/// The log line the held startup reload emits once its snapshot is fixed —
/// `STARTUP_RELOAD_HELD_LOG` in the server.
const HELD_LOG: &str = "SpecTcl: startup reload holding its snapshot";

/// The same race as the test above, but **forced** — the proof that the
/// ordering fix is load-bearing.
///
/// The server is started with the startup reload holding its snapshot for
/// [`STARTUP_HOLD`]: it discovers and loads packs, announces that its view of
/// the disk is fixed, and only then sleeps before publishing. The pack file is
/// written *after* that announcement and its watcher event sent immediately,
/// so the startup reload is provably sitting on a view that predates the file
/// while the notification's reload reads the true state. That is the
/// interleaving CI hit and that a fast idle box will not reproduce on its own
/// — waiting for the signal is what makes it a proof rather than a hope, since
/// writing the pack too early just gives the startup reload a snapshot that
/// already contains it.
///
/// Two independent protections make it come out right, and either alone
/// suffices. The reload mutex serialises the whole discover → load → publish
/// sequence, so the notification's reload waits out the hold and then re-reads
/// the disk, finding the pack. The generation stamp orders publishes by *when
/// the disk was read*, so even with the two reloads overlapping, the startup
/// one's older snapshot cannot take the place of a set published from a newer
/// one. With neither, the held snapshot publishes last and the pack disappears
/// from a session that had already reported it — which is exactly what this
/// test sees when both are removed.
///
/// Which is why awaiting the pack is not the assertion. The notification's
/// reload publishes it early; the regression is that it is taken *away* again
/// when the stale snapshot finally lands. So this outlives the hold, requiring
/// the pack to still be there, and fails on the first poll that loses it.
#[test]
fn a_stale_startup_snapshot_cannot_overwrite_a_newer_pack_set() {
    let root = workspace("startup-race-forced");
    let source = "mylib::with_var counter {\n    set counter 1\n}\n";
    let doc = root.join("app.tcl");
    write(&doc, source);

    let hold_ms = STARTUP_HOLD.as_millis().to_string();
    let mut lsp = Lsp::with_config_at_root_env(
        json!({ "features": { "linkedEditingRange": true } }),
        &root,
        &[("TCL_LSP_TEST_STARTUP_RELOAD_HOLD_MS", hold_ms.as_str())],
    );

    // The startup reload has read the disk and is now sitting on that reading.
    lsp.await_log(&[HELD_LOG], std::time::Duration::from_secs(30), 0);
    let held_at = std::time::Instant::now();

    // Written strictly inside the hold, so the snapshot upstairs cannot
    // contain it — and notified at once, so the second reload starts while the
    // first is still asleep.
    let pack = root.join(".tcl-lsp/mylib.tclspec");
    write(&pack, MYLIB_PACK);
    notify_pack_changed(&mut lsp, &pack, CREATED);

    await_pack_named(&mut lsp, "mylib");

    // Outlive the held snapshot's publish, which lands one hold after the
    // signal. The slack is load-scaled — the publish is preceded by real work
    // on both sides — while the hold itself is not.
    let window = STARTUP_HOLD + scaled_timeout(std::time::Duration::from_secs(3));
    while held_at.elapsed() < window {
        let names = pack_names(&mut lsp);
        assert!(
            names.iter().any(|loaded| loaded == "mylib"),
            "the workspace pack was published and then lost {:?} into the hold — \
             the stale startup snapshot republished over a newer set; loaded: {names:?}",
            held_at.elapsed()
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    // And the editor-visible consequence, sampled once the race is settled.
    let uri = file_uri(&doc);
    lsp.open_ready(&uri, source);
    let hover = hover_text(&lsp.hover(&uri, 0, 4));
    assert!(
        hover.contains("Run a script with a caller variable bound."),
        "the pack on disk is the one that survives the race; got:\n{hover}"
    );

    let _ = std::fs::remove_dir_all(&root);
}
