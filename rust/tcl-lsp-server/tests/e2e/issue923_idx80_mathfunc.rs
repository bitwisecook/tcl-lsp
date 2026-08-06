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

//! Issue #923 differential audit, finding idx 80 — a `tcl::mathfunc` override
//! injected in one file and applied by an `expr` in a sibling file.
//!
//! Two surfaces had to be brought into agreement, and only the real protocol
//! shows it: the existing in-process unit test hand-populated the workspace
//! index and called `full_diagnostics_for` directly, so it saw neither the
//! `workspace/configuration` handshake (which decides `crossFileResolution`)
//! nor `textDocument/codeAction` at all — and `codeAction` was still offering
//! `Replace with 'ni'` over working code even when the diagnostic had already
//! been suppressed.
//!
//! Oracle — tclsh 8.6.16 and 9.0.4, sourcing `helper.tcl` then `vector.tcl`:
//!
//! ```text
//! AngleBetween(-1.0) => 3.141592653589793
//! NegateX(7) => -7.0
//! ```
//!
//! `Pi()` / `Inv()` really do dispatch cross-file to `::tcl::mathfunc::Pi` /
//! `::tcl::mathfunc::Inv`.  Accepting the quick-fix breaks that outright — both
//! releases reject the rewritten form:
//!
//! ```text
//! $ tclsh9.0 -c 'puts [expr {ni() / acos(-1.0)}]'
//! missing operand at _@_
//! in expression "_@_ni() / acos(-1.0)"
//! ```
//!
//! and a name nothing defines is a genuine error on both, which is what the
//! true-positive controls below pin:
//!
//! ```text
//! invalid command name "tcl::mathfunc::NeverDefinedAnywhere"
//! ```

use crate::common::{Lsp, unique_uri};
use serde_json::{Value, json};
use std::time::Duration;

/// The `::tcl::mathfunc` injection every test here resolves against.
const HELPER: &str = "namespace eval ::tcl::mathfunc {\n\
                      \x20   proc Pi {} { return 3.141592653589793 }\n\
                      \x20   proc Inv {x} { return [expr {-1.0 * $x}] }\n\
                      }\n";

/// The consumer: `Pi()` on line 2, `Inv($x)` on line 5, and a math function
/// nothing anywhere defines on line 8 (the true-positive control).
const VECTOR: &str = "namespace eval tomato::vector {}\n\
                      proc tomato::vector::AngleBetween {c} {\n\
                      \x20   return [expr {acos($c) * (Pi() / acos(-1.0))}]\n\
                      }\n\
                      proc tomato::vector::NegateX {x} {\n\
                      \x20   return [expr {Inv($x)}]\n\
                      }\n\
                      proc tomato::vector::Broken {} {\n\
                      \x20   return [expr {NeverDefinedAnywhere()}]\n\
                      }\n";

/// The names W123 reports, in source order.
fn w123_names(diags: &[Value]) -> Vec<String> {
    diags
        .iter()
        .filter(|d| d.get("code").and_then(Value::as_str) == Some("W123"))
        .filter_map(|d| {
            d.get("message")
                .and_then(Value::as_str)
                .and_then(|m| m.split('\'').nth(1))
                .map(ToOwned::to_owned)
        })
        .collect()
}

/// Every code-action title the server offers over `range`.
fn action_titles(actions: &Value) -> Vec<String> {
    actions
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|a| a.get("title").and_then(Value::as_str))
                .map(ToOwned::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

/// A one-character range on `line` at `ch` — a caret, the way an editor asks.
fn caret(line: u32, ch: u32) -> Value {
    json!({
        "start": { "line": line, "character": ch },
        "end": { "line": line, "character": ch + 1 },
    })
}

/// Titles offered at the `Pi()` call site (line 2) of [`VECTOR`].
fn titles_at_pi(lsp: &mut Lsp, uri: &str) -> Vec<String> {
    action_titles(&lsp.code_actions(uri, caret(2, 30), json!([])))
}

/// Titles offered at the genuinely-undefined call site (line 8) of [`VECTOR`].
fn titles_at_undefined(lsp: &mut Lsp, uri: &str) -> Vec<String> {
    action_titles(&lsp.code_actions(uri, caret(8, 20), json!([])))
}

/// Whether any offered title is a "did you mean…?" command rewrite.
fn has_replace_fix(titles: &[String]) -> bool {
    titles.iter().any(|t| t.starts_with("Replace with"))
}

/// **The reported bug, out of the box.** Default configuration — nothing
/// opted into, `crossFileResolution` left at its documented default of off —
/// and the two files joined only by living in the same workspace, exactly as
/// a `pkgIndex.tcl`-only project is. `Pi()` and `Inv()` must draw no W123,
/// and the name nothing defines must still draw one.
#[test]
fn default_config_clears_w123_for_a_cross_file_mathfunc() {
    let mut lsp = Lsp::tcl();
    let helper = unique_uri("tcl");
    lsp.open_ready(&helper, HELPER);
    let vector = unique_uri("tcl");
    lsp.open_ready(&vector, VECTOR);

    // The fast tier publishes no W120/W123 at all, so a batch carrying the
    // control's W123 is necessarily a deep-pass batch with the workspace
    // refinements applied — the only state worth asserting on.
    let diags = lsp.await_diagnostics_settled(&vector, Duration::from_secs(20), |d| {
        w123_names(d).iter().any(|n| n == "NeverDefinedAnywhere")
    });
    let names = w123_names(&diags);
    assert!(
        !names.iter().any(|n| n == "Pi" || n == "Inv"),
        "a `::tcl::mathfunc` override defined in a sibling document is the one \
         `Pi`/`Inv` every `expr` in the workspace can call; W123 must not fire \
         on it under the default configuration, got: {names:?}",
    );
    assert!(
        names.iter().any(|n| n == "NeverDefinedAnywhere"),
        "TP control: a math function nothing defines is a real error \
         (tclsh: `invalid command name \"tcl::mathfunc::NeverDefinedAnywhere\"`) \
         and must still draw W123, got: {names:?}",
    );
}

/// **The harmful half.** `textDocument/codeAction` must reach the same verdict
/// as the diagnostics it lifts from: no unresolved-command rewrite over the
/// working `Pi()` call, and — the control that proves the mechanism is still
/// alive, not merely switched off — the offer still stands where the name
/// really is unknown.
#[test]
fn default_config_offers_no_command_rewrite_over_a_cross_file_mathfunc() {
    let mut lsp = Lsp::tcl();
    let helper = unique_uri("tcl");
    lsp.open_ready(&helper, HELPER);
    let vector = unique_uri("tcl");
    lsp.open_ready(&vector, VECTOR);
    lsp.await_diagnostics_settled(&vector, Duration::from_secs(20), |d| {
        w123_names(d).iter().any(|n| n == "NeverDefinedAnywhere")
    });

    let at_pi = titles_at_pi(&mut lsp, &vector);
    assert!(
        !has_replace_fix(&at_pi),
        "accepting a `Replace with …` over `Pi()` rewrites working code into a \
         parse error (tclsh 8.6.16 / 9.0.4: `missing operand`); offered: {at_pi:?}",
    );
    let at_undefined = titles_at_undefined(&mut lsp, &vector);
    assert!(
        has_replace_fix(&at_undefined),
        "TP control: the unresolved-command quick-fix must still be offered \
         where the name really is unknown, offered: {at_undefined:?}",
    );
}

/// The same shape driven through a **real workspace scan**: the two files sit
/// on disk, joined only by a `pkgIndex.tcl`, and the server discovers the
/// sibling itself rather than being handed it by a `didOpen`.
#[test]
fn workspace_scan_clears_w123_for_a_cross_file_mathfunc() {
    let root = std::env::temp_dir().join(format!(
        "tcl-lsp-e2e-idx80-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos(),
    ));
    std::fs::create_dir_all(&root).expect("create workspace root");
    std::fs::write(root.join("helper.tcl"), HELPER).expect("write helper.tcl");
    std::fs::write(
        root.join("pkgIndex.tcl"),
        "package ifneeded tomato 1.0 [list source [file join $dir helper.tcl]]\n",
    )
    .expect("write pkgIndex.tcl");
    let vector_path = root.join("vector.tcl");
    std::fs::write(&vector_path, VECTOR).expect("write vector.tcl");

    let mut lsp = Lsp::at_workspace_root(&root);
    let vector = format!("file://{}", vector_path.to_string_lossy());
    lsp.open_ready(&vector, VECTOR);
    let diags = lsp.await_diagnostics_settled(&vector, Duration::from_secs(30), |d| {
        w123_names(d).iter().any(|n| n == "NeverDefinedAnywhere")
    });
    let names = w123_names(&diags);
    assert!(
        !names.iter().any(|n| n == "Pi" || n == "Inv"),
        "the scan indexed helper.tcl; W123 must not fire on the cross-file \
         math functions, got: {names:?}",
    );
    assert!(
        names.iter().any(|n| n == "NeverDefinedAnywhere"),
        "TP control through the scan path, got: {names:?}",
    );
    let titles = titles_at_pi(&mut lsp, &vector);
    assert!(
        !has_replace_fix(&titles),
        "code actions must agree with the scan-informed diagnostics: {titles:?}",
    );

    let _ = std::fs::remove_dir_all(&root);
}

/// Opting into `crossFileResolution` changes nothing about the math-function
/// verdict — it was never the toggle's to decide — and code actions still
/// agree with diagnostics in that state too.
#[test]
fn cross_file_resolution_on_keeps_the_mathfunc_verdict_and_the_action_agrees() {
    let mut lsp = Lsp::with_config(json!({
        "features": { "linkedEditingRange": true, "crossFileResolution": true }
    }));
    let helper = unique_uri("tcl");
    lsp.open_ready(&helper, HELPER);
    let vector = unique_uri("tcl");
    lsp.open_ready(&vector, VECTOR);
    // The fast tier publishes no W120/W123 at all, so a batch carrying the
    // control's W123 is necessarily a deep-pass batch with the workspace
    // refinements applied — the only state worth asserting on.
    let diags = lsp.await_diagnostics_settled(&vector, Duration::from_secs(20), |d| {
        w123_names(d).iter().any(|n| n == "NeverDefinedAnywhere")
    });
    let names = w123_names(&diags);
    assert!(
        !names.iter().any(|n| n == "Pi" || n == "Inv"),
        "got: {names:?}",
    );
    assert!(
        names.iter().any(|n| n == "NeverDefinedAnywhere"),
        "TP control with the toggle on, got: {names:?}",
    );
    let at_pi = titles_at_pi(&mut lsp, &vector);
    assert!(!has_replace_fix(&at_pi), "offered: {at_pi:?}");
    let at_undefined = titles_at_undefined(&mut lsp, &vector);
    assert!(
        has_replace_fix(&at_undefined),
        "TP control: offered: {at_undefined:?}",
    );
}

/// The **project tier**, which stays opt-in: an ordinary bareword call to a
/// proc in a sibling document. It draws W123 by default (a workspace is not
/// automatically one program — a bare `helper` in an unrelated script is a
/// real error), and the quick-fix is offered because the diagnostic is.
#[test]
fn a_plain_cross_file_proc_call_still_draws_w123_by_default() {
    let mut lsp = Lsp::tcl();
    let lib = unique_uri("tcl");
    lsp.open_ready(&lib, "proc renderReport {a b} { return [list $a $b] }\n");
    let caller = unique_uri("tcl");
    let diags = lsp.open_ready(&caller, "renderReport 1 2\n");
    assert!(
        w123_names(&diags).iter().any(|n| n == "renderReport"),
        "the project tier is opt-in; without it a cross-file bareword call is \
         still unknown, got: {:?}",
        w123_names(&diags),
    );
}

/// …and with `crossFileResolution` on, **both** surfaces move together: the
/// W123 clears and so does its quick-fix. This is the pairing the old code
/// could not express — diagnostics consulted the workspace, code actions did
/// not.
#[test]
fn cross_file_resolution_on_clears_a_plain_proc_w123_and_its_quick_fix() {
    let mut lsp = Lsp::with_config(json!({
        "features": { "linkedEditingRange": true, "crossFileResolution": true }
    }));
    let lib = unique_uri("tcl");
    lsp.open_ready(&lib, "proc renderReport {a b} { return [list $a $b] }\n");
    let caller = unique_uri("tcl");
    let src = "renderReport 1 2\nnoSuchCommandAnywhere 1\n";
    lsp.open_ready(&caller, src);
    let diags = lsp.await_diagnostics_settled(&caller, Duration::from_secs(20), |d| {
        w123_names(d).iter().any(|n| n == "noSuchCommandAnywhere")
    });
    let names = w123_names(&diags);
    assert!(
        !names.iter().any(|n| n == "renderReport"),
        "the sibling document defines it, got: {names:?}",
    );
    assert!(
        names.iter().any(|n| n == "noSuchCommandAnywhere"),
        "TP control: got: {names:?}",
    );

    let at_call = action_titles(&lsp.code_actions(&caller, caret(0, 4), json!([])));
    assert!(
        !has_replace_fix(&at_call),
        "the diagnostic is suppressed, so its quick-fix must be too: {at_call:?}",
    );
    let at_unknown = action_titles(&lsp.code_actions(&caller, caret(1, 4), json!([])));
    assert!(has_replace_fix(&at_unknown), "TP control: {at_unknown:?}",);
}
