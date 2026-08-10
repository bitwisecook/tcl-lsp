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

//! Issues #1331 and #1332 — facts that live in **another file**.
//!
//! Both were reported by @nico-robert on #1181 against v2.1.16 / VS Code, and
//! both are invisible to single-file coverage: the existing one-file tests for
//! arity and for W120 pass today and would not have caught either bug. The
//! two-file shape *is* the test, which is why these run over the real protocol
//! with two documents open in one workspace.
//!
//! # #1331 — the diagnostics path ignored the cross-file index
//!
//! ```text
//! # deflib.tcl
//! proc libtest {a b c} { return [expr {$a + $b + $c}] }
//! # plaincaller.tcl
//! libtest 1 2
//! ```
//!
//! `textDocument/definition` on `libtest` resolved to `deflib.tcl`, while
//! `publishDiagnostics` reported `W123 Unknown command 'libtest'` and no
//! arity error — one server giving two contradictory answers about the same
//! name, because navigation consulted the workspace index and diagnostics
//! consulted a different, bare-tail name set that was off by default.
//!
//! The `definition` request is kept here as the **control**, exactly as the
//! issue used it: it is what rules out a mis-scoped fixture (where every
//! cross-file lookup fails for an unrelated reason and looks just like this
//! bug). If the fixture were wrong, definition would fail too.
//!
//! Oracle — C Tcl 9.0.4:
//!
//! ```text
//! % source deflib.tcl ; libtest 1 2
//! wrong # args: should be "libtest a b c"
//! ```
//!
//! # #1332 — `source` was never followed
//!
//! ```text
//! # tkFile.tcl
//! package require Tk
//! # main.tcl
//! source tkFile.tcl
//! winfo exists .l
//! ```
//!
//! `winfo` drew `W120 "winfo" requires package require Tk`, a false positive:
//! Tk *is* loaded by the time `winfo` runs. The dynamic path in the original
//! report was a red herring — the literal form behaved identically, so
//! `source` was not followed however the path was written.
//!
//! Oracle — C Tcl 9.0.4, `package present` around a `source` of a file that
//! requires a package:
//!
//! ```text
//! before: not-loaded
//! after:  1.7.1
//! ```
//!
//! Present only *after* the statement, which is why the TN control below
//! (`winfo` written above the `source`) must still warn.

use crate::common::{Lsp, unique_uri};
use serde_json::{Value, json};
use std::time::Duration;

/// The library half of the #1331 repro.
const DEFLIB: &str = "proc libtest {a b c} { return [expr {$a + $b + $c}] }\n";

/// Diagnostic codes present, in source order.
fn codes(diags: &[Value]) -> Vec<String> {
    diags
        .iter()
        .filter_map(|d| d.get("code").and_then(Value::as_str))
        .map(ToOwned::to_owned)
        .collect()
}

/// Whether any diagnostic carries `code`.
fn has(diags: &[Value], code: &str) -> bool {
    codes(diags).iter().any(|c| c == code)
}

/// The message of the first diagnostic carrying `code`.
fn message_of(diags: &[Value], code: &str) -> Option<String> {
    diags
        .iter()
        .find(|d| d.get("code").and_then(Value::as_str) == Some(code))
        .and_then(|d| d.get("message").and_then(Value::as_str))
        .map(ToOwned::to_owned)
}

fn has_message(diags: &[Value], code: &str, fragment: &str) -> bool {
    diags.iter().any(|diagnostic| {
        diagnostic.get("code").and_then(Value::as_str) == Some(code)
            && diagnostic
                .get("message")
                .and_then(Value::as_str)
                .is_some_and(|message| message.contains(fragment))
    })
}

/// The file name of `uri`, which is what a sibling document writes in its
/// `source` statement.
fn basename(uri: &str) -> String {
    uri.rsplit('/').next().unwrap_or(uri).to_owned()
}

/// Wait until the pushed diagnostics for `uri` satisfy `settled`.
///
/// Only safe with a **positive** predicate — one naming a diagnostic that must
/// be present. The progressive fast tier publishes no W120/W123 and no
/// cross-file arity at all, so a predicate of the form "X is absent" is
/// satisfied by the fast tier before the deep pass has even looked, and would
/// pass whatever the deep pass later concluded. Assertions that something must
/// be *absent* use [`final_diagnostics`] instead.
fn settled(lsp: &Lsp, uri: &str, settled: impl Fn(&[Value]) -> bool) -> Vec<Value> {
    lsp.await_diagnostics_settled(uri, Duration::from_secs(20), settled)
}

/// The document's **complete, converged** diagnostic set, via
/// `textDocument/diagnostic`.
///
/// A deterministic barrier: the pull handler analyses the document's current
/// content and returns the full set, so — unlike the debounced push channel —
/// there is no tier for an "X is absent" assertion to pass against vacuously.
/// It also exercises the *pull* path's own copy of the cross-file resolution,
/// which must reach the same verdict as the push path; a regression that fixed
/// only one of the two would show up here as a disagreement.
fn final_diagnostics(lsp: &mut Lsp, uri: &str) -> Vec<Value> {
    lsp.pull_diagnostics(uri)
}

/// Replace an open document and wait until its deep publish has atomically
/// installed the new workspace-index facts.
fn replace_and_wait_for_index(lsp: &mut Lsp, uri: &str, version: i64, text: &str) {
    let since = lsp.notification_cursor();
    lsp.replace_document(uri, version, text);
    lsp.await_log(
        &["workspace_state.update", uri],
        Duration::from_secs(20),
        since,
    );
}

// ---------------------------------------------------------------------------
// #1331
// ---------------------------------------------------------------------------

/// **The reported bug.** Default configuration — nothing opted into — and the
/// two files joined only by living in the same workspace. The cross-file call
/// must draw no W123, and its wrong argument count must be reported.
#[test]
fn a_cross_file_call_resolves_and_reports_its_arity() {
    let mut lsp = Lsp::tcl();
    let deflib = unique_uri("tcl");
    lsp.open_ready(&deflib, DEFLIB);
    let caller = unique_uri("tcl");
    lsp.open_ready(&caller, "libtest 1 2\n");

    // The pushed set: E002 present is a deep-pass marker, so its own "no W123"
    // is not vacuous. Then the pull path, which must agree exactly.
    let pushed = settled(&lsp, &caller, |d| has(d, "E002"));
    let pulled = final_diagnostics(&mut lsp, &caller);
    for (surface, diags) in [("push", &pushed), ("pull", &pulled)] {
        assert!(
            !has(diags, "W123"),
            "[{surface}] the workspace defines `libtest`; calling it unknown \
             contradicts go-to-definition, which resolves it: {:?}",
            codes(diags),
        );
        assert_eq!(
            message_of(diags, "E002").as_deref(),
            Some("Too few arguments for 'libtest': expected at least 3, got 2"),
            "[{surface}] tclsh 9.0.4: `wrong # args: should be \"libtest a b c\"`",
        );
    }
}

/// Toggle-on regression: direct calls still have one exact workspace oracle.
/// This covers the true arity error (TP), no false W123 for `libtest` (FP), a
/// remaining W123 for an unrelated command (TN), and identical push/pull
/// reporting without a duplicated E002 (FN).
#[test]
fn an_opted_in_exact_cross_file_arity_is_reported_once_on_push_and_pull() {
    let mut lsp = Lsp::with_config(json!({
        "features": { "linkedEditingRange": true, "crossFileResolution": true }
    }));
    let deflib = unique_uri("tcl");
    lsp.open_ready(&deflib, DEFLIB);
    let caller = unique_uri("tcl");
    lsp.open_ready(&caller, "libtest 1 2\nneverDefinedAnywhereAtAll x\n");

    let pushed = settled(&lsp, &caller, |diags| {
        has(diags, "E002") && has(diags, "W123")
    });
    let pulled = final_diagnostics(&mut lsp, &caller);
    for (surface, diags) in [("push", &pushed), ("pull", &pulled)] {
        assert_eq!(
            codes(diags)
                .iter()
                .filter(|code| code.as_str() == "E002")
                .count(),
            1,
            "[{surface}] the direct cross-file arity oracle must emit E002 once: {:?}",
            codes(diags),
        );
        assert_eq!(
            message_of(diags, "E002").as_deref(),
            Some("Too few arguments for 'libtest': expected at least 3, got 2"),
            "[{surface}] must preserve the exact direct-call arity verdict",
        );
        let w123 = message_of(diags, "W123").unwrap_or_default();
        assert!(
            !w123.contains("libtest"),
            "[{surface}] the exact workspace proc must not be unknown: {w123}",
        );
        assert!(
            w123.contains("neverDefinedAnywhereAtAll"),
            "[{surface}] an unrelated unknown command must remain reported: {w123}",
        );
    }
}

/// An open definition edit changes another open document's diagnostics without
/// touching the caller. This pins add, arity-change, and removal invalidation,
/// including the push result and the revision-keyed pull cache.
#[test]
fn definition_edits_republish_open_callers_and_invalidate_their_pull_cache() {
    let mut lsp = Lsp::tcl();
    let deflib = unique_uri("tcl");
    lsp.open_ready(&deflib, "# definition arrives later\n");
    let caller = unique_uri("tcl");
    lsp.open_ready(&caller, "libtest 1 2\nneverDefinedAnywhereAtAll x\n");
    settled(&lsp, &caller, |diags| has_message(diags, "W123", "libtest"));

    // Add: the old W123 is replaced by the exact cross-file arity error.
    replace_and_wait_for_index(&mut lsp, &deflib, 2, DEFLIB);
    let pushed = settled(&lsp, &caller, |diags| {
        has(diags, "E002") && !has_message(diags, "W123", "libtest")
    });
    assert!(has(&pushed, "E002"), "the caller must be republished");
    let pulled = final_diagnostics(&mut lsp, &caller);
    assert!(has(&pulled, "E002") && !has_message(&pulled, "W123", "libtest"));

    // Arity change: two arguments now fit. The unrelated W123 is the positive
    // deep-pass marker, so the absence assertion cannot pass on the fast tier.
    replace_and_wait_for_index(
        &mut lsp,
        &deflib,
        3,
        "proc libtest {a b} { return [expr {$a + $b}] }\n",
    );
    let pushed = settled(&lsp, &caller, |diags| {
        has_message(diags, "W123", "neverDefinedAnywhereAtAll")
            && !has(diags, "E002")
            && !has(diags, "E003")
            && !has(diags, "E005")
    });
    assert!(!has(&pushed, "E002"), "the stale arity error must clear");
    let pulled = final_diagnostics(&mut lsp, &caller);
    assert!(!has(&pulled, "E002") && !has(&pulled, "E003") && !has(&pulled, "E005"));

    // Remove: the exact command no longer exists, so W123 returns.
    replace_and_wait_for_index(&mut lsp, &deflib, 4, "# libtest removed\n");
    let pushed = settled(&lsp, &caller, |diags| has_message(diags, "W123", "libtest"));
    assert!(has_message(&pushed, "W123", "libtest"));
    let pulled = final_diagnostics(&mut lsp, &caller);
    assert!(has_message(&pulled, "W123", "libtest"));
}

/// **The control the issue used.** `textDocument/definition` on the same token
/// must still resolve — if it did not, the fixture would be mis-scoped and the
/// assertion above would be measuring something else entirely.
#[test]
fn definition_still_resolves_the_same_cross_file_call() {
    let mut lsp = Lsp::tcl();
    let deflib = unique_uri("tcl");
    lsp.open_ready(&deflib, DEFLIB);
    let caller = unique_uri("tcl");
    lsp.open_ready(&caller, "libtest 1 2\n");
    settled(&lsp, &caller, |d| has(d, "E002"));

    let def = lsp.definition(&caller, 0, 2);
    let text = def.to_string();
    assert!(
        text.contains(&deflib),
        "definition must resolve to the defining document, got: {text}",
    );
}

/// **TP control.** A command nothing in the workspace defines still draws
/// W123 — the suppression is a resolution, not a blanket silencing.
#[test]
fn a_genuinely_unknown_command_still_draws_w123() {
    let mut lsp = Lsp::tcl();
    let deflib = unique_uri("tcl");
    lsp.open_ready(&deflib, DEFLIB);
    let caller = unique_uri("tcl");
    lsp.open_ready(&caller, "libtest 1 2 3\nneverDefinedAnywhereAtAll x\n");

    let diags = settled(&lsp, &caller, |d| has(d, "W123"));
    let msg = message_of(&diags, "W123").unwrap_or_default();
    assert!(
        msg.contains("neverDefinedAnywhereAtAll"),
        "the unknown command is the one that must be flagged, got: {msg}",
    );
    assert!(
        !msg.contains("libtest"),
        "`libtest` is defined in the workspace: {msg}",
    );
}

/// **FP guard — the precision that lets this be on by default.** A bare
/// `buried` call at global scope reaches `::buried`; a `proc ::deep::buried`
/// in a sibling file is a different command and must not silence it. tclsh:
/// `invalid command name "buried"`.
#[test]
fn a_namespaced_proc_does_not_silence_a_bare_global_call() {
    let mut lsp = Lsp::tcl();
    let lib = unique_uri("tcl");
    lsp.open_ready(
        &lib,
        "namespace eval ::deep { proc buried {x} { return $x } }\n",
    );
    let caller = unique_uri("tcl");
    lsp.open_ready(&caller, "buried 1\n");

    let diags = settled(&lsp, &caller, |d| has(d, "W123"));
    assert!(
        message_of(&diags, "W123")
            .unwrap_or_default()
            .contains("buried"),
        "a bare global call reaches no namespaced definition: {:?}",
        codes(&diags),
    );
}

/// **TN.** An `args`-tailed cross-file signature accepts any count above its
/// minimum, so a long call draws nothing while the call still resolves.
#[test]
fn a_cross_file_args_tailed_proc_abstains_from_arity() {
    let mut lsp = Lsp::tcl();
    let lib = unique_uri("tcl");
    lsp.open_ready(
        &lib,
        "proc argstail {a args} { return $a }\nproc sentinel {} {}\n",
    );
    let caller = unique_uri("tcl");
    lsp.open_ready(&caller, "argstail 1 2 3 4 5 6\nneverEverDefined y\n");

    // Positive marker first, so the deep pass has demonstrably run; then the
    // converged set for the absence assertion.
    settled(&lsp, &caller, |d| has(d, "W123"));
    let diags = final_diagnostics(&mut lsp, &caller);
    assert!(
        !has(&diags, "E002") && !has(&diags, "E003"),
        "a trailing `args` is unbounded; tclsh accepts every count >= 1: {:?}",
        codes(&diags),
    );
}

// ---------------------------------------------------------------------------
// #1332
// ---------------------------------------------------------------------------

/// **The reported bug, literal form.** `main.tcl` sources a file that requires
/// Tk, so `winfo` is satisfied and must draw no W120.
#[test]
fn a_literal_source_carries_package_require_tk_up() {
    let mut lsp = Lsp::tcl();
    let tk_file = unique_uri("tcl");
    lsp.open_ready(&tk_file, "package require Tk\n");
    let main = unique_uri("tcl");
    let text = format!("source {}\nwinfo exists .l\n", basename(&tk_file));
    lsp.open_ready(&main, &text);

    let diags = final_diagnostics(&mut lsp, &main);
    assert!(
        !has(&diags, "W120"),
        "Tk is loaded by the time `winfo` runs: {:?}",
        codes(&diags),
    );
}

/// A sourced child's package facts are a live dependency of the sourcing
/// document. Adding and removing the require must republish the untouched
/// parent and invalidate its pull-cache entry.
#[test]
fn sourced_child_package_edits_republish_the_open_parent() {
    let mut lsp = Lsp::tcl();
    let child = unique_uri("tcl");
    lsp.open_ready(&child, "# Tk not loaded yet\n");
    let parent = unique_uri("tcl");
    let parent_text = format!(
        "source {}\nwinfo exists .l\nneverDefinedAnywhereAtAll x\n",
        basename(&child),
    );
    lsp.open_ready(&parent, &parent_text);
    settled(&lsp, &parent, |diags| has(diags, "W120"));

    replace_and_wait_for_index(&mut lsp, &child, 2, "package require Tk\n");
    let pushed = settled(&lsp, &parent, |diags| {
        has_message(diags, "W123", "neverDefinedAnywhereAtAll") && !has(diags, "W120")
    });
    assert!(!has(&pushed, "W120"));
    assert!(!has(&final_diagnostics(&mut lsp, &parent), "W120"));

    replace_and_wait_for_index(&mut lsp, &child, 3, "# Tk removed again\n");
    let pushed = settled(&lsp, &parent, |diags| has(diags, "W120"));
    assert!(has(&pushed, "W120"));
    assert!(has(&final_diagnostics(&mut lsp, &parent), "W120"));
}

/// **The reported bug, computed form** — `[file join [file dirname [info
/// script]] …]`, the idiom real multi-file projects write. It must behave
/// exactly like the literal, and the W300 dynamic-path security warning must
/// keep firing: it is correct and unrelated.
#[test]
fn a_computed_source_path_behaves_like_the_literal_and_keeps_w300() {
    let mut lsp = Lsp::tcl();
    let tk_file = unique_uri("tcl");
    lsp.open_ready(&tk_file, "package require Tk\n");
    let main = unique_uri("tcl");
    let text = format!(
        "source [file join [file dirname [info script]] {}]\nwinfo exists .l\n",
        basename(&tk_file),
    );
    lsp.open_ready(&main, &text);

    let diags = final_diagnostics(&mut lsp, &main);
    assert!(
        !has(&diags, "W120"),
        "the computed path resolves statically, so Tk is known loaded: {:?}",
        codes(&diags),
    );
    assert!(
        has(&diags, "W300"),
        "the dynamic-path security warning is correct and must survive: {:?}",
        codes(&diags),
    );
}

/// **TN — the control that proves option (2) was not applied globally.** A
/// file that neither requires Tk nor sources anything still gets its W120.
#[test]
fn a_file_with_no_tk_and_no_source_still_gets_w120() {
    let mut lsp = Lsp::tcl();
    let solo = unique_uri("tcl");
    lsp.open_ready(&solo, "winfo exists .l\n");

    let diags = settled(&lsp, &solo, |d| has(d, "W120"));
    assert!(
        has(&diags, "W120"),
        "nothing loads Tk here; tclsh: `invalid command name \"winfo\"`: {:?}",
        codes(&diags),
    );
}

/// **TN.** `source`ing a file that does *not* require Tk contributes nothing,
/// so the W120 stands. Following `source` must inherit real facts, not act as
/// a licence to go quiet.
#[test]
fn sourcing_a_file_that_does_not_require_tk_still_gets_w120() {
    let mut lsp = Lsp::tcl();
    let plain = unique_uri("tcl");
    lsp.open_ready(&plain, "proc plainhelper {} { return 1 }\n");
    let main = unique_uri("tcl");
    let text = format!("source {}\nwinfo exists .l\n", basename(&plain));
    lsp.open_ready(&main, &text);

    let diags = settled(&lsp, &main, |d| has(d, "W120"));
    assert!(
        has(&diags, "W120"),
        "the sourced file loads no Tk: {:?}",
        codes(&diags),
    );
}

/// **TN — ordering.** A `winfo` written *above* the `source` runs before the
/// package is loaded, so it must still warn. Verified against C Tcl 9.0.4:
/// `package present` fails before the `source` statement and succeeds after.
#[test]
fn a_call_above_the_source_statement_still_gets_w120() {
    let mut lsp = Lsp::tcl();
    let tk_file = unique_uri("tcl");
    lsp.open_ready(&tk_file, "package require Tk\n");
    let main = unique_uri("tcl");
    let text = format!("winfo exists .l\nsource {}\n", basename(&tk_file));
    lsp.open_ready(&main, &text);

    let diags = settled(&lsp, &main, |d| has(d, "W120"));
    assert!(
        has(&diags, "W120"),
        "Tk is not loaded yet at this line: {:?}",
        codes(&diags),
    );
}

/// A `source` whose target the server cannot pin to an indexed document — here
/// a file that does not exist — makes the document's command and package set
/// unknowable, so W120 and W123 both abstain rather than fire wrongly.
#[test]
fn an_unfollowable_source_abstains_rather_than_misfiring() {
    let mut lsp = Lsp::tcl();
    let main = unique_uri("tcl");
    lsp.open_ready(
        &main,
        "source no_such_file_anywhere.tcl\nwinfo exists .l\nsomeUnknownThing 1\n",
    );

    let diags = final_diagnostics(&mut lsp, &main);
    assert!(
        !has(&diags, "W120") && !has(&diags, "W123"),
        "a file of unknown content is in the run; neither `winfo` nor \
         `someUnknownThing` can be called missing: {:?}",
        codes(&diags),
    );
}

/// A sourced file's **procs** are equally visible — the second symptom the
/// issue notes, closed by the same cross-file resolution as #1331.
#[test]
fn a_sourced_files_procs_resolve_in_the_sourcing_file() {
    let mut lsp = Lsp::tcl();
    let lib = unique_uri("tcl");
    lsp.open_ready(&lib, DEFLIB);
    let main = unique_uri("tcl");
    let text = format!("source {}\nlibtest 1 2\n", basename(&lib));
    lsp.open_ready(&main, &text);

    let diags = settled(&lsp, &main, |d| has(d, "E002"));
    assert!(
        !has(&diags, "W123"),
        "the sourced file defines `libtest`: {:?}",
        codes(&diags),
    );
    assert!(
        has(&diags, "E002"),
        "and its signature is known, so the bad call is reported: {:?}",
        codes(&diags),
    );
}
