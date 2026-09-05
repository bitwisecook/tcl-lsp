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
    // macOS exposes its temporary directory through `/var`, a symlink to
    // `/private/var`. Pack discovery canonicalises paths before publishing
    // diagnostics, so keep the client-side fixture URI in that same spelling.
    std::fs::canonicalize(root).expect("canonical workspace root")
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
const DELETED: i64 = 3;

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

/// A pack declaring a file extension nothing in the shipped catalogue owns,
/// alongside one that declares no dialect at all.
const EXTENSION_PACK: &str = r"
speclib extlib 1 {
    file_extension irulex -name {Extension Pack Rules} -dialect f5-irules
    file_extension packplain -name {Plain Pack Script}

    command extlib::noop {
        arity 1
    }
}
";

/// Issue #1626: the server advertises the extensions its discovered packs
/// claim, each resolved to an editor language id a client can actually
/// associate with.
///
/// The server half of lazy registration was already done — pack routing is
/// consulted ahead of the static catalogue by `dialect_from_extension` — but
/// an editor learns associations from a manifest written long before the
/// user's pack existed, so it has to be *told*. This is that channel.
#[test]
fn the_server_advertises_the_extensions_its_packs_claim() {
    let root = workspace("extensions");
    write(&root.join(".tcl-lsp/extlib.tclspec"), EXTENSION_PACK);

    let mut lsp = Lsp::with_config_at_root(json!({}), &root);
    await_pack_named(&mut lsp, "extlib");

    let advertised = lsp
        .effective_config("")
        .get("pack_file_extensions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let row = |ext: &str| -> Option<Value> {
        advertised
            .iter()
            .find(|row| row.get("extension").and_then(Value::as_str) == Some(ext))
            .cloned()
    };

    // A row with a `-dialect` lands on that dialect's own editor language —
    // no new language id can be created at runtime, so this is the whole
    // range of choice a client has.
    let irulex = row("irulex").expect("the pack's `irulex` row must be advertised");
    assert_eq!(irulex["dialect"], json!("f5-irules"));
    assert_eq!(irulex["language_id"], json!("tcl-irule"));
    assert_eq!(irulex["pack"], json!("extlib"));

    // A row with no `-dialect` rides plain `tcl`; the server's own detection
    // decides the dialect once the file is open.
    let plain = row("packplain").expect("a dialect-less row must still be advertised");
    assert_eq!(plain["dialect"], Value::Null);
    assert_eq!(plain["language_id"], json!("tcl"));

    // NEGATIVE control: everything the shipped catalogue owns is already
    // registered statically by every editor, so it is never advertised —
    // otherwise a client could not tell its own dynamic entries from the
    // static ones when a pack goes away.
    for statically_owned in ["irule", "tmsh", "sdc", "upf"] {
        assert!(
            row(statically_owned).is_none(),
            "catalogue-owned .{statically_owned} must not be advertised: {advertised:?}"
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}

/// Review finding P1-3: a pack-claimed extension has to reach the *index*,
/// not only the open document.
///
/// The distinction is the whole finding. Opening a `.irulex` file always
/// worked — `dialect_from_extension` consults pack routing, so the document
/// analyses correctly. But every predicate that decides which files the
/// server reaches on its own read the static `TCL_SOURCE_EXTENSIONS`, so a
/// **closed** `.irulex` file was invisible: its definitions never entered the
/// workspace index, and a reference from an ordinary `.tcl` file could not
/// resolve to them.
///
/// So this asserts on cross-file resolution from a file that is never opened,
/// which is exactly what the static-only predicate could not do.
#[test]
fn a_closed_file_under_a_pack_extension_is_indexed() {
    let root = workspace("extensions-indexed");
    write(&root.join(".tcl-lsp/extlib.tclspec"), EXTENSION_PACK);
    // Never opened by this test — only ever reached by the workspace scan.
    write(
        &root.join("helpers.irulex"),
        "proc extpack_only_helper {a b} {\n    return [expr {$a + $b}]\n}\n",
    );
    let source = "extpack_only_helper 1 2\n";
    let doc = root.join("caller.tcl");
    write(&doc, source);

    let mut lsp = Lsp::with_config_at_root(json!({}), &root);
    let uri = file_uri(&doc);
    await_pack_named(&mut lsp, "extlib");
    lsp.open_ready(&uri, source);

    // The call resolves to a definition that lives in a file nothing opened.
    let definitions = lsp.definition(&uri, 0, 4);
    let text = serde_json::to_string(&definitions).unwrap_or_default();
    assert!(
        text.contains("helpers.irulex"),
        "a closed `.irulex` file must be indexed once a pack claims the \
         extension; definition returned: {text}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// The advertisement follows the packs: retire the pack and the extension
/// stops being advertised, so a client's association is retired with it.
#[test]
fn a_removed_pack_stops_advertising_its_extension() {
    let root = workspace("extensions-removed");
    let pack = root.join(".tcl-lsp/extlib.tclspec");
    write(&pack, EXTENSION_PACK);

    let mut lsp = Lsp::with_config_at_root(json!({}), &root);
    await_pack_named(&mut lsp, "extlib");

    let claims = |lsp: &mut Lsp| -> bool {
        lsp.effective_config("")
            .get("pack_file_extensions")
            .and_then(Value::as_array)
            .is_some_and(|rows| {
                rows.iter()
                    .any(|row| row.get("extension").and_then(Value::as_str) == Some("irulex"))
            })
    };
    assert!(
        claims(&mut lsp),
        "the pack's extension must start advertised"
    );

    std::fs::remove_file(&pack).expect("remove pack");
    notify_pack_changed(&mut lsp, &pack, DELETED);

    let deadline = std::time::Instant::now() + scaled_timeout(std::time::Duration::from_secs(30));
    while claims(&mut lsp) {
        assert!(
            std::time::Instant::now() < deadline,
            "the extension was still advertised after its pack was deleted"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    let _ = std::fs::remove_dir_all(&root);
}

/// A pack that declares an **environment** with a file-extension detection
/// row makes documents with that extension resolve to it — through the
/// ordinary ingress, with no dialect setting, no language id, and nothing in
/// the document saying so.
///
/// This is the production wiring of the environment-registration seam end to
/// end: discovery finds the pack, the load publishes it, publishing registers
/// its `environment` block into the one live registry the ingress resolves
/// against and republishes the extension routing detection reads — so
/// `getEffectiveConfig` on a `.pshx` buffer answers with the pack's own
/// environment id and display name rather than the session default.
///
/// And the other half of the same contract: delete the pack, and the
/// environment retires. A pack-declared environment that outlived its file
/// would be a dialect the user cannot get rid of.
#[test]
fn a_pack_declared_environment_routes_its_documents_through_the_ingress() {
    const ENVIRONMENT_PACK: &str = r"
speclib envprobe 2.0 {
    environment probe-shell-tcl {
        display_name   {Probe Shell}
        core           tcl 8.6
        ambient        ProbeCmds 1.0
        alias          probe-shell
        file_extension pshx -name {Probe Shell Script}
        policy         ambient-plus-require
    }
    command probe_open {
        arity 1
        hover { summary {Open a probe shell session.} }
    }
}
";
    let root = workspace("environment");
    let pack = root.join(".tcl-lsp/envprobe.tclspec");
    write(&pack, ENVIRONMENT_PACK);
    let source = "probe_open session\n";
    let doc = root.join("session.pshx");
    write(&doc, source);

    let mut lsp = Lsp::with_config_at_root(json!({}), &root);
    let uri = file_uri(&doc);
    await_pack_named(&mut lsp, "envprobe");
    lsp.open_ready(&uri, source);

    let dialect = |lsp: &mut Lsp, uri: &str| -> String {
        lsp.effective_config(uri)
            .get("dialect")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    // Resolved at open, from the extension alone: the barrier above means the
    // pack — and so its environment — was live before the document opened.
    assert_eq!(dialect(&mut lsp, &uri), "probe-shell-tcl");

    // The claim is advertised to the client too, so an editor that can
    // register associations at runtime opens `.pshx` as Tcl in the first
    // place — without that, every path but the user's own works.
    let advertised = lsp
        .effective_config("")
        .get("pack_file_extensions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let row = advertised
        .iter()
        .find(|row| row.get("extension").and_then(Value::as_str) == Some("pshx"))
        .unwrap_or_else(|| panic!("the environment's extension is advertised: {advertised:#?}"));
    assert_eq!(
        row.get("dialect").and_then(Value::as_str),
        Some("probe-shell-tcl")
    );
    assert_eq!(
        row.get("display_name").and_then(Value::as_str),
        Some("Probe Shell Script")
    );

    // A plain `.tcl` buffer in the same workspace is untouched — the
    // environment claims its own extension, not the workspace.
    let plain = root.join("plain.tcl");
    write(&plain, source);
    let plain_uri = file_uri(&plain);
    lsp.open_ready(&plain_uri, source);
    assert_ne!(dialect(&mut lsp, &plain_uri), "probe-shell-tcl");

    // Delete the pack: the environment retires with it, and the extension
    // stops routing.
    std::fs::remove_file(&pack).expect("remove pack");
    notify_pack_changed(&mut lsp, &pack, DELETED);
    let deadline = std::time::Instant::now() + scaled_timeout(std::time::Duration::from_secs(30));
    loop {
        // A document's dialect is resolved when it opens, so the retirement
        // is observed the way a user would see it: close the buffer and open
        // it again.
        lsp.close_document(&uri);
        lsp.open_ready(&uri, source);
        if dialect(&mut lsp, &uri) != "probe-shell-tcl" {
            break;
        }
        assert!(
            std::time::Instant::now() < deadline,
            "the environment was still routing after its pack was deleted"
        );
        std::thread::sleep(std::time::Duration::from_millis(50));
    }

    let _ = std::fs::remove_dir_all(&root);
}

// ---------------------------------------------------------------------------
// The pack file as a *document*: what a spec author sees while writing one.
//
// Everything above is about a pack's effect on the Tcl around it. These are
// about the `.tclspec` buffer itself — the Spec Studio's Pack DSL editor is a
// client of this very server, so whatever the server answers here is what a
// pack author gets, in the studio and in their own editor alike.
// ---------------------------------------------------------------------------

/// A pack with one command and enough shape to exercise every document
/// surface: nested blocks to outline and fold, statement words to hover, and
/// braced prose the formatter must not touch.
const AUTHORING_PACK: &str = "speclib mylib 1 {\n\
                              \x20   command mylib::x {\n\
                              \x20       arity 1\n\
                              \x20       hover {\n\
                              \x20           summary {Do a thing.}\n\
                              \x20       }\n\
                              \x20   }\n\
                              }\n";

/// Open a `.tclspec` in a workspace of its own and settle its diagnostics.
fn open_pack(name: &str, body: &str) -> (PathBuf, Lsp, String) {
    let root = workspace(name);
    let pack = root.join(".tcl-lsp/mylib.tclspec");
    write(&pack, body);
    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&pack);
    await_pack_named(&mut lsp, "mylib");
    lsp.open_ready(&uri, body);
    (root, lsp, uri)
}

/// Every symbol name a document-symbol reply carries, parents before children.
fn symbol_names(node: &Value, out: &mut Vec<String>) {
    if let Some(name) = node.get("name").and_then(Value::as_str) {
        out.push(name.to_owned());
    }
    for child in node
        .get("children")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        symbol_names(child, out);
    }
}

/// A pack outlines as its declarations. Its dialect declares a document
/// grammar, so its *blocks* are its structure — the analyser's proc/namespace
/// symbolizer has nothing to find in a file that declares rather than runs.
#[test]
fn a_pack_outlines_as_its_own_declarations() {
    let (root, mut lsp, uri) = open_pack("outline", AUTHORING_PACK);

    let symbols = lsp.document_symbols(&uri);
    let mut names = Vec::new();
    for node in symbols.as_array().into_iter().flatten() {
        symbol_names(node, &mut names);
    }
    assert!(
        names.iter().any(|n| n.contains("speclib mylib")),
        "the pack itself is the outline root: {symbols:#?}"
    );
    assert!(
        names.iter().any(|n| n.contains("command mylib::x")),
        "each declared command is a child: {symbols:#?}"
    );
    assert!(
        names.iter().any(|n| n.contains("hover")),
        "and a nested block is a child of the command: {symbols:#?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Hover on a pack's own statement word answers from the `SpecTcl` command
/// pack — through the real handler, not the provider in isolation.
#[test]
fn a_pack_statement_word_hovers_from_the_spectcl_surface() {
    let (root, mut lsp, uri) = open_pack("authoring-hover", AUTHORING_PACK);

    let speclib = hover_text(&lsp.hover(&uri, 0, 2));
    assert!(
        speclib.contains("Open a SpecTcl command pack."),
        "the document's root word hovers as SpecTcl vocabulary; got:\n{speclib}"
    );
    // …and a *property* word, which only means anything inside a `command`
    // body: the same word in an ordinary Tcl script is not a command at all.
    let arity = hover_text(&lsp.hover(&uri, 2, 9));
    assert!(
        arity.contains("Declare how many argument words the call takes."),
        "a property word inside a command body hovers too; got:\n{arity}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Signature help on a pack statement offers that statement's own form, not
/// an ordinary Tcl command's.
#[test]
fn a_pack_statement_offers_signature_help() {
    let (root, mut lsp, uri) = open_pack("authoring-signature", AUTHORING_PACK);

    let help = lsp.signature_help(&uri, 0, 8);
    let label = help
        .get("signatures")
        .and_then(Value::as_array)
        .and_then(|sigs| sigs.first())
        .and_then(|sig| sig.get("label"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_owned();
    assert!(
        label.contains("speclib"),
        "signature help names the statement being written; got {help:#?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A pack folds on its blocks: the `speclib` body, the `command` body, and the
/// `hover` body inside it.
#[test]
fn a_packs_blocks_are_folding_ranges() {
    let (root, mut lsp, uri) = open_pack("authoring-folding", AUTHORING_PACK);

    let ranges = lsp.folding_range(&uri);
    let starts: Vec<i64> = ranges
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| r.get("startLine").and_then(Value::as_i64))
        .collect();
    for (line, what) in [(0, "speclib"), (1, "command"), (3, "hover")] {
        assert!(
            starts.contains(&line),
            "the `{what}` block must fold (line {line}); got {ranges:#?}"
        );
    }

    let _ = std::fs::remove_dir_all(&root);
}

/// Formatting a pack must leave its byte-significant prose alone.
///
/// `summary` / `description` / `snippet` / `examples` are every byte between
/// their braces, verbatim — that is what lets a ported field compare byte for
/// byte with the `&'static str` it came from — so a formatter that rewrapped
/// one would quietly change what the editor shows for the command the pack
/// describes. (`tcl-spectcl`'s `pack_formatting` suite makes the same claim
/// over the shipped example packs, by reloading them; this one pins that the
/// real `textDocument/formatting` handler is the thing being asked.)
#[test]
fn formatting_a_pack_keeps_its_prose_verbatim() {
    const PROSE: &str = "Two  spaces and\n            an indented line.";
    let body = format!(
        "speclib mylib 1 {{\n\
         \x20   command mylib::x {{\n\
         \x20     arity 1\n\
         \x20       hover {{\n\
         \x20           summary {{{PROSE}}}\n\
         \x20       }}\n\
         \x20   }}\n\
         }}\n"
    );
    let (root, mut lsp, uri) = open_pack("authoring-formatting", &body);

    let edits = lsp.formatting(&uri, 4, true);
    let formatted = edits
        .as_array()
        .into_iter()
        .flatten()
        .next()
        .and_then(|edit| edit.get("newText"))
        .and_then(Value::as_str)
        .map_or_else(|| body.clone(), ToOwned::to_owned);
    assert!(
        formatted.contains(PROSE),
        "the formatter rewrote the pack's prose:\n{formatted}"
    );
    assert!(
        formatted.contains("        arity 1"),
        "…while still doing its job on the declarations:\n{formatted}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// An **open** pack keeps its load notices while it is edited.
///
/// A `.tclspec` is both a spec pack and an analysed Tcl document, and
/// `publishDiagnostics` replaces a URI's whole set — so the two used to
/// overwrite each other and editing a pack showed the analyser's view alone
/// until the next reload put the notices back. The published set is now the
/// union, which is what this asserts: an analyser diagnostic the *edit*
/// introduces and the loader's `SPECTCL` notice, on screen together.
#[test]
fn an_open_pack_keeps_its_notices_while_it_is_edited() {
    /// A word the pack cannot already carry, so the settle barrier below is a
    /// fact of the edited buffer and not of the one it replaced.
    const ADDED: &str = "a_word_only_this_edit_carries";

    let pack = "speclib mylib 1 {\n    command mylib::x {\n        arity 1\n        \
                nonsense_property yes\n    }\n}\n";
    let (root, mut lsp, uri) = open_pack("open-pack-notices", pack);

    let has_notice = |diags: &[Value]| {
        diags.iter().any(|d| {
            d.get("code").and_then(Value::as_str) == Some("SPECTCL")
                && d.get("message")
                    .and_then(Value::as_str)
                    .is_some_and(|m| m.contains("nonsense_property"))
        })
    };
    let settled =
        lsp.await_diagnostics_settled(&uri, std::time::Duration::from_secs(15), |diags| {
            has_notice(diags)
        });
    assert!(has_notice(&settled), "{settled:#?}");

    // Edit the buffer so the *analyser* has something of its own to say that
    // it could not have said before — the settle barrier must be a fact of the
    // edited text, or a stale pre-edit publication satisfies it.
    let edited = format!("{pack}\n{ADDED}\n");
    lsp.replace_document(&uri, 2, &edited);
    let mentions_edit = |diags: &[Value]| {
        diags.iter().any(|d| {
            d.get("code").and_then(Value::as_str) != Some("SPECTCL")
                && d.get("message")
                    .and_then(Value::as_str)
                    .is_some_and(|m| m.contains(ADDED))
        })
    };
    let after = lsp.await_diagnostics_settled(&uri, std::time::Duration::from_secs(20), |diags| {
        mentions_edit(diags)
    });
    assert!(
        mentions_edit(&after),
        "the analyser must have its say on the edited buffer: {after:#?}"
    );
    assert!(
        has_notice(&after),
        "…and the loader's notice must survive alongside it: {after:#?}"
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// A dropped property offers the word the author meant.
///
/// The `SpecTcl` vocabulary is closed — a property word is a member of the
/// grammar in force where it was written — so a notice naming a dropped word
/// is a typo with one computable correction.
#[test]
fn a_dropped_property_offers_a_did_you_mean_fix() {
    let pack = "speclib mylib 1 {\n    command mylib::x {\n        arty 1\n    }\n}\n";
    let (root, mut lsp, uri) = open_pack("did-you-mean", pack);

    let diagnostics =
        lsp.await_diagnostics_settled(&uri, std::time::Duration::from_secs(15), |diags| {
            diags.iter().any(|d| {
                d.get("message")
                    .and_then(Value::as_str)
                    .is_some_and(|m| m.contains("arty"))
            })
        });
    let notice = diagnostics
        .iter()
        .find(|d| {
            d.get("message")
                .and_then(Value::as_str)
                .is_some_and(|m| m.contains("arty"))
        })
        .unwrap_or_else(|| panic!("the dropped-property notice: {diagnostics:#?}"))
        .clone();

    let actions = lsp.code_actions(&uri, notice["range"].clone(), json!([notice]));
    let fix = actions
        .as_array()
        .into_iter()
        .flatten()
        .find(|a| {
            a.get("title")
                .and_then(Value::as_str)
                .is_some_and(|t| t.contains("arity"))
        })
        .unwrap_or_else(|| panic!("a did-you-mean quick fix: {actions:#?}"));
    assert_eq!(fix.get("kind").and_then(Value::as_str), Some("quickfix"));
    let edit = fix["edit"]["changes"][&uri][0].clone();
    assert_eq!(edit.get("newText").and_then(Value::as_str), Some("arity"));
    assert_eq!(edit["range"]["start"]["line"].as_i64(), Some(2));
    assert_eq!(edit["range"]["start"]["character"].as_i64(), Some(8));

    let _ = std::fs::remove_dir_all(&root);
}

/// A pack's `include NAME` row is a document link to the sibling file it
/// names — the same file the loader's include resolver would read.
#[test]
fn a_pack_include_row_is_a_document_link() {
    let pack = "speclib mylib 1 {\n    include shared.tclspec\n    \
                command mylib::x {\n        arity 1\n    }\n}\n";
    let root = workspace("include-link");
    write(&root.join(".tcl-lsp/shared.tclspec"), "");
    let file = root.join(".tcl-lsp/mylib.tclspec");
    write(&file, pack);
    let mut lsp =
        Lsp::with_config_at_root(json!({ "features": { "linkedEditingRange": true } }), &root);
    let uri = file_uri(&file);
    await_pack_named(&mut lsp, "mylib");
    lsp.open_ready(&uri, pack);

    let links = lsp.request(
        "textDocument/documentLink",
        json!({ "textDocument": { "uri": &uri } }),
    );
    let link = links
        .as_array()
        .into_iter()
        .flatten()
        .find(|l| {
            l.get("target")
                .and_then(Value::as_str)
                .is_some_and(|t| t.ends_with("shared.tclspec"))
        })
        .unwrap_or_else(|| panic!("a link to the included pack: {links:#?}"));
    assert_eq!(link["range"]["start"]["line"].as_i64(), Some(1));

    let _ = std::fs::remove_dir_all(&root);
}

/// `tclLsp.features.diagnostics = false` clears a pack's load notices too.
///
/// The notices are a layer the publisher unions into every set a `.tclspec`
/// gets — the pushed one and the pulled one alike — so a master switch that
/// only emptied the *analysed* set left the squiggles standing, which is the
/// one thing that switch promises not to do. Both paths are asserted, and
/// re-enabling has to bring the notice back: a clear that cannot be undone is
/// as wrong as one that never happens.
#[test]
fn the_diagnostics_master_switch_clears_a_packs_notices() {
    let pack = "speclib mylib 1 {\n    command mylib::x {\n        arty 1\n    }\n}\n";
    let (root, mut lsp, uri) = open_pack("master-switch-notices", pack);

    let has_notice = |diags: &[Value]| {
        diags
            .iter()
            .any(|d| d.get("code").and_then(Value::as_str) == Some("SPECTCL"))
    };
    let pushed =
        lsp.await_diagnostics_settled(&uri, std::time::Duration::from_secs(15), has_notice);
    assert!(has_notice(&pushed), "{pushed:#?}");
    let pulled = lsp.pull_diagnostics(&uri);
    assert!(
        has_notice(&pulled),
        "the pull path carries it too: {pulled:#?}"
    );

    let since = lsp.notification_cursor();
    lsp.apply_configuration(json!({ "features": { "diagnostics": false } }));
    assert_eq!(
        lsp.await_diagnostics_master_off(&uri, std::time::Duration::from_secs(20), since),
        Vec::<Value>::new(),
        "the push path must clear the notice"
    );
    let pulled = lsp.pull_diagnostics(&uri);
    assert_eq!(
        pulled,
        Vec::<Value>::new(),
        "…and the pull path must agree with it: {pulled:#?}"
    );

    lsp.apply_configuration(json!({ "features": { "diagnostics": true } }));
    let pushed =
        lsp.await_diagnostics_settled(&uri, std::time::Duration::from_secs(20), has_notice);
    assert!(has_notice(&pushed), "re-enabling restores it: {pushed:#?}");
    let pulled = lsp.pull_diagnostics(&uri);
    assert!(has_notice(&pulled), "…on both paths: {pulled:#?}");

    let _ = std::fs::remove_dir_all(&root);
}
