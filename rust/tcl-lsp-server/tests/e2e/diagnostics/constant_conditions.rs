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

//! Always-true and always-false conditions (I230) and forwarded constants (O102): what the interprocedural seed may assume about callers, dispatch and other files.

use super::*;

// `when` is an iRules-only builtin. Under plain Tcl it is an unknown
// would-be user command whose braced argument is opaque *data*, not a handler
// script — so its body must not be analysed.

#[test]
fn when_body_not_analysed_under_plain_tcl() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    // Opened with the *versioned* `tcl8.6` language id, which is a deliberate
    // editor choice and outranks detection. The bare `tcl` id every editor
    // sends for a `.tcl` buffer defers to the shared detector, and a
    // `when EVENT {` handler is one of its iRules content signatures — so
    // this source would otherwise be analysed as an iRule, which is not the
    // case under test.
    let diags = lsp.open_ready_lang(
        &uri,
        "when HTTP_REQUEST {\n    boguscmd $undefvar\n}\n",
        "tcl8.6",
    );
    // `when` itself is unknown under Tcl, but the opaque body must not be
    // recursed into: no W123 naming the body command, no W210 on its var.
    let body_w123: Vec<&Value> = diags
        .iter()
        .filter(|d| code_str(d).as_deref() == Some("W123") && message(d).contains("boguscmd"))
        .collect();
    assert!(body_w123.is_empty(), "{diags:?}");
    assert!(!has_code(&diags, "W210"), "{:?}", codes(&diags));
}

// I230 (always-true/false condition; alternate branch unreachable) folds
// `==`/`!=` on string operands, matching Tcl's polymorphic compare
// (`expr {"foo" == "foo"}` -> 1), the same as the `eq`/`ne` spelling.

#[test]
fn double_equals_string_condition_folds() {
    for op in ["==", "eq"] {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        let src = format!("set x foo\nif {{$x {op} \"foo\"}} {{ puts hi }}\n");
        assert!(has_code(&lsp.open_ready(&uri, &src), "I230"), "{op}");
    }
}

#[test]
fn bang_equals_string_condition_folds() {
    for op in ["!=", "ne"] {
        let mut lsp = Lsp::tcl();
        let uri = unique_uri("tcl");
        let src = format!("set x foo\nif {{$x {op} \"foo\"}} {{ puts hi }}\n");
        assert!(has_code(&lsp.open_ready(&uri, &src), "I230"), "{op}");
    }
}

// A method callee reached via `my` can be invisible to the caller's upvar
// context, so its `upvar 1 $refvar ref` caller-frame definition must not be
// treated as a no-op — `[info exists ref]` in the calling method must not
// fold to always-false. tclsh 9.0.4 / 8.6.14: after `my Reference? $lookup
// ref` returns true the guard is 1; on a miss, 0 — live code, so the fold
// must abstain.

#[test]
fn info_exists_after_my_dispatch_to_an_upvar_sibling_does_not_fire_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create Formatter {\n    method Reference? {lookup refvar} {\n        upvar 1 $refvar ref\n        if {$lookup eq \"x\"} { set ref 1; return 1 }\n        return 0\n    }\n    method ResolvableReference? {lookup} {\n        my Reference? $lookup ref\n        if {[info exists ref]} { puts hi }\n    }\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "I230"),
        "an upvar-defined local across a `my` dispatch must not fold: {:?}",
        codes(&diags)
    );
}

/// Rooted helper identities do not close the runtime object's method surface.
/// Tcl 9.0.4: an `oo::objdefine` replacement of helper can upvar-define zzz in
/// m, changing info exists from 0 to 1 without changing this class body.
#[test]
fn info_exists_after_runtime_selected_object_dispatch_does_not_fire_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create C {\n    method helper {} { ::return 1 }\n    method m {} {\n        [::oo::Helpers::self object] helper\n        ::if {[::info exists zzz]} { ::puts hi }\n    }\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "I230"),
        "runtime method replacement must keep the existence query dynamic: {:?}",
        codes(&diags)
    );
}

/// A rooted registry command with no caller-frame effects retains precision.
#[test]
fn info_exists_still_fires_i230_after_safe_rooted_command_in_method() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "oo::class create C {\n    method m {} {\n        ::string length value\n        ::if {[::info exists zzz]} { ::puts hi }\n    }\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(has_code(&diags, "I230"), "{:?}", codes(&diags));
}

// `setter`'s `uplevel #0 {set x 99}` needs no `global` declaration, so an
// interprocedural summary that misses this global-frame write would let
// O102 rewrite `puts $x` to `puts 5`, where tclsh 9.0.3/9.0.4 prints 99.
// O102 is in the `constant_folding` category, outside the default
// `readability` profile, so the test opts into the `standard` profile.

#[test]
fn o102_does_not_forward_a_global_across_a_callees_uplevel_hash_zero_write() {
    let mut lsp = Lsp::with_config(serde_json::json!({ "optimiser": { "profile": "standard" } }));
    let uri = unique_uri("tcl");
    let src = "proc setter {} {\n    uplevel #0 {\n        set x 99\n    }\n}\nset x 5\nsetter\nputs $x\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "O102"),
        "the callee's `uplevel #0` write makes the forward unsound: {:?}",
        codes(&diags)
    );
}

/// TN control for the global-frame effect summary: a callee whose
/// `uplevel #0` script writes no variable has no global effect, so the
/// caller's constant still forwards (tclsh 9.0.4: prints `hi` then `5`).
#[test]
fn o102_still_fires_when_the_callees_uplevel_hash_zero_writes_nothing() {
    let mut lsp = Lsp::with_config(serde_json::json!({ "optimiser": { "profile": "standard" } }));
    let uri = unique_uri("tcl");
    let src = "proc shout {} {\n    uplevel #0 [list puts hi]\n}\nset x 5\nshout\nputs $x\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        has_code(&diags, "O102"),
        "a non-writing global-frame script must not block forwarding: {:?}",
        codes(&diags)
    );
}

// A genuinely alternating parity check must not fold to
// "Condition '$count & 1' is always false". The interprocedural param-constant seed
// (`params_constants_from_call_sites`) must not trust a proc's parameter as
// a compile-time literal when a call site it cannot resolve exists — for
// example a namespaced proc's own bare-name recursive self-call, which a
// namespace-blind lookup would miss. Missing that call site (whose argument
// necessarily varies) would leave only the one external caller's literal
// `0` and fold `$count & 1` to a fixed `false`. See `compilation_unit.rs`'s
// `collect_call_site_constants` / `params_constants_from_call_sites`; the
// same hazard applies to a call site embedded inside a `catch` / `uplevel`
// body's `ArgRole::Body` argument.

#[test]
fn namespaced_recursive_proc_parity_check_does_not_fire_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "namespace eval ::graph {\n    proc dfs {count} {\n        if {$count & 1} {\n            set parity odd\n        } else {\n            set parity even\n        }\n        if {$count < 3} {\n            dfs [expr {$count + 1}]\n        }\n    }\n}\n::graph::dfs 0\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "I230"),
        "recursive parity check on `count` must not fold: {:?}",
        codes(&diags)
    );
}

#[test]
fn call_site_hidden_inside_catch_body_does_not_fire_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc is_even {n} {\n    if {$n % 2 == 0} { return 1 } else { return 0 }\n}\nproc main {} {\n    is_even 3\n    catch { is_even 4 }\n}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "I230"),
        "is_even is called with both 3 and 4 (the latter inside catch): {:?}",
        codes(&diags)
    );
}

/// TP control: the interprocedural seed must still fire I230 for a
/// genuinely proc-call-invariant parameter — two callers passing the
/// identical literal to a private helper. Parameters are seeded only for
/// procedures the file's own code enters, so the callers are invoked.
#[test]
fn two_callers_with_uniform_literal_still_fires_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc helper {mode} {\n    if {$mode eq \"prod\"} { set r 1 } else { set r 2 }\n}\nproc caller1 {} { helper prod }\nproc caller2 {} { helper prod }\ncaller1\ncaller2\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        has_code(&diags, "I230"),
        "two callers passing the identical literal should still fold: {:?}",
        codes(&diags)
    );
}

// The interprocedural seed above must account for a call dispatched
// through a variable (`set cmd helper; $cmd dev`), not just literal
// command words — otherwise such a call counts neither for nor against any
// proc's parameters, even when it demonstrably reaches one the scan
// already seeded from its literal call sites. The scan resolves a dispatch
// by value (`unit_scope.rs`): an enumerable one becomes an ordinary call
// site for each name it can hold, and an unenumerable one withdraws every
// seed in the module.

#[test]
fn dynamic_dispatch_with_a_differing_literal_does_not_fire_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc helper {mode} {\n    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }\n}\nhelper prod\nhelper prod\nset cmd helper\n$cmd dev\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "I230"),
        "`$cmd dev` also reaches helper, with a differing literal: {:?}",
        codes(&diags)
    );
}

/// TP control: the same dispatch passing the *same* literal every other
/// caller does must still fold — the fix resolves the dispatch rather than
/// blanket-disqualifying the module.
#[test]
fn dynamic_dispatch_agreeing_on_the_literal_still_fires_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc helper {mode} {\n    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }\n}\nhelper prod\nhelper prod\nset cmd helper\n$cmd prod\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        has_code(&diags, "I230"),
        "every caller — dispatched or not — passes \"prod\": {:?}",
        codes(&diags)
    );
}

/// TP control: a dispatch that provably names a *different* proc must leave
/// an unrelated proc's fold alone.
#[test]
fn dynamic_dispatch_to_a_different_proc_still_fires_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc helper {mode} {\n    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }\n}\nproc other {a} { return $a }\nhelper prod\nhelper prod\nset cmd other\n$cmd dev\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        has_code(&diags, "I230"),
        "the dispatch provably names `other`, never `helper`: {:?}",
        codes(&diags)
    );
}

#[test]
fn unenumerable_dynamic_dispatch_does_not_fire_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc helper {mode} {\n    if {$mode eq \"prod\"} { set x 1 } else { set x 2 }\n}\nhelper prod\nhelper prod\nset cmd [gets stdin]\n$cmd dev\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "I230"),
        "a dispatch whose target cannot be enumerated may reach helper: {:?}",
        codes(&diags)
    );
}

#[test]
fn command_prefix_callback_target_does_not_fire_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc cmp {a b} {\n    if {$a eq \"x\"} { return -1 } else { return 1 }\n}\ncmp x x\ncmp x x\nlsort -command cmp {p q}\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "I230"),
        "`lsort -command cmp` invokes cmp with runtime-supplied arguments: {:?}",
        codes(&diags)
    );
}

// Tcl dispatches every unresolved command word to the module's own
// `unknown` handler, passing the word and that call's arguments — so the
// handler's *direct* callers are never its complete caller set, and a
// coincidentally-uniform set of them must not fold a condition that varies
// at runtime. tclsh 8.6/9.0 confirm: with `proc unknown {cmd args}` in
// scope, `bogus beta` runs the handler with `cmd` = `bogus`. The registry
// marks the handler with `Traits::UNRESOLVED_COMMAND_HANDLER`, so no
// compiler code spells the name.

#[test]
fn unresolved_word_reaching_the_unknown_handler_does_not_fire_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc unknown {cmd args} {\n    if {$cmd eq \"alpha\"} { return 1 } else { return 2 }\n}\nunknown alpha\nunknown alpha\nbogus beta\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "I230"),
        "`bogus beta` also reaches unknown, with cmd = \"bogus\": {:?}",
        codes(&diags)
    );
}

/// End-to-end pair for the unit-level
/// `a_handler_never_seeds_even_when_every_visible_caller_agrees_1044`:
/// defining the handler is itself the unenumerable caller, so agreement
/// among the callers the scan can see proves nothing and no fold happens.
///
/// Tcl routes to the handler every word that resolves to nothing at the
/// instant of the call, and most of those (an autoloaded name, a name
/// another sourced file introduces, a name built by string arithmetic)
/// appear nowhere in the source for any scan to find, so "no unresolved
/// word in the file" does not mean "the direct callers are all of them".
///
/// tclsh8.6 confirms the seeded words are wrong in both directions: `Dog
/// new` after `oo::class create Dog` and `worker` after `coroutine worker
/// body` are recorded as dispatches yet never reach the handler, while a
/// `bogus beta` written before `proc unknown` is handled by the builtin
/// `::unknown` and errors.
#[test]
fn unknown_handler_never_fires_i230_even_with_agreeing_callers() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc unknown {cmd args} {\n    if {$cmd eq \"alpha\"} { return 1 } else { return 2 }\n}\nunknown alpha\nunknown alpha\nputs hi\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        !has_code(&diags, "I230"),
        "the handler's caller set is unenumerable, so it never seeds: {:?}",
        codes(&diags)
    );
}

/// TN control and the regression risk: a module with **no** handler is the
/// common case and must be entirely unaffected by the unresolved-word path.
#[test]
fn unresolved_word_without_an_unknown_handler_still_fires_i230() {
    let mut lsp = Lsp::tcl();
    let uri = unique_uri("tcl");
    let src = "proc helper {mode} {\n    if {$mode eq \"prod\"} { set r 1 } else { set r 2 }\n}\nproc caller1 {} { helper prod }\nproc caller2 {} { helper prod }\nbogus beta\n";
    let diags = lsp.open_ready(&uri, src);
    assert!(
        has_code(&diags, "I230"),
        "with no handler defined, an unresolved word reaches nothing: {:?}",
        codes(&diags)
    );
}

// A plain library file with no `package provide`, `source`d by another file
// that calls its procs with a different literal, must not fold. `lib.tcl`
// analysed alone sees only its own two `helper prod` callers and would seed
// `mode` as `"prod"`. The project's call sites are threaded into the
// compilation unit (`tcl_lsp_db::project_call_site_evidence` →
// `SourceFile::external_call_sites` → `tcl_compiler::unit_scope`), so
// `main.tcl`'s `helper dev` retracts the fold.

/// The library file from the issue: no `package provide`, two agreeing
/// in-file callers.
/// The file name a sibling document writes in its `source` statement: the
/// harness gives every document a unique name, so a literal `lib.tcl` links
/// to nothing.
fn sibling_name(uri: &str) -> &str {
    uri.rsplit('/').next().unwrap_or(uri)
}

/// A line that always draws W123, so a wait can name a positive signal: a
/// retracted I230 leaves an empty list that looks like "nothing published yet".
const SETTLED_MARKER: &str = "neverDefinedAnywhereAtAll x\n";

const CROSS_FILE_LIB: &str = "proc helper {mode} {\n    if {$mode eq \"prod\"} { set r 1 } else { set r 2 }\n}\nhelper prod\nhelper prod\n";

/// Also the standing proof that the project's call-site evidence is **not**
/// an opt-in refinement: this session leaves `tclLsp.features.
/// crossFileResolution` at its default (off), so gating the server's
/// evidence sync on that toggle would reinstate the unsound fold and fail
/// here.
#[test]
fn caller_in_a_sourcing_file_with_a_differing_literal_clears_i230() {
    let mut lsp = Lsp::tcl();
    let main_uri = unique_uri("tcl");
    let lib_uri = unique_uri("tcl");
    // Open the caller first so it is already in the project when the library
    // is analysed — the same ordering a workspace scan produces.
    lsp.open_ready(
        &main_uri,
        &format!("source {}\nhelper dev\n", sibling_name(&lib_uri)),
    );
    lsp.open_document(&lib_uri, &format!("{CROSS_FILE_LIB}{SETTLED_MARKER}"));
    // The cross-file retraction lands on a *later* publish than the library's
    // own first result: the project's call-site evidence is refreshed after
    // publishing, not in front of it (putting it in front delayed the
    // semantic-token enrichment tier on a large document). Wait for the
    // converged state rather than the first publish.
    let diags = lsp.await_diagnostics_settled(&lib_uri, Duration::from_secs(20), |d| {
        has_code(d, "W123") && !has_code(d, "I230")
    });
    assert!(
        !has_code(&diags, "I230"),
        "helper is called with \"dev\" from the sourcing file; must not fold: {:?}",
        codes(&diags)
    );
}

/// TP control: with every project caller agreeing on the literal the seed is
/// sound and I230 still fires — the fix must not disable the mechanism.
#[test]
fn caller_in_another_file_agreeing_on_the_literal_still_fires_i230() {
    let mut lsp = Lsp::tcl();
    let main_uri = unique_uri("tcl");
    let lib_uri = unique_uri("tcl");
    lsp.open_ready(
        &main_uri,
        &format!("source {}\nhelper prod\n", sibling_name(&lib_uri)),
    );
    let diags = lsp.open_ready(&lib_uri, CROSS_FILE_LIB);
    assert!(
        has_code(&diags, "I230"),
        "every caller in the project passes \"prod\": {:?}",
        codes(&diags)
    );
}

/// TN control: a project file that never calls into the library contributes
/// no evidence and must not disturb a sound fold.
#[test]
fn unrelated_project_file_does_not_clear_i230() {
    let mut lsp = Lsp::tcl();
    let other_uri = unique_uri("tcl");
    let lib_uri = unique_uri("tcl");
    lsp.open_ready(&other_uri, "proc other {x} { return $x }\nother 1\n");
    let diags = lsp.open_ready(&lib_uri, CROSS_FILE_LIB);
    assert!(
        has_code(&diags, "I230"),
        "an unrelated file must not retract a sound fold: {:?}",
        codes(&diags)
    );
}

/// The other direction: the *sourcing* file's own proc is called back from
/// the file it sources. `main.tcl`'s two visible callers agree on `"prod"`,
/// but `lib.tcl` — whose script runs in the same interpreter — calls
/// `local dev`, so the fold must be retracted there too.
#[test]
fn callback_from_a_sourced_file_clears_i230_in_the_sourcing_file() {
    let mut lsp = Lsp::tcl();
    let lib_uri = unique_uri("tcl");
    let main_uri = unique_uri("tcl");
    lsp.open_ready(&lib_uri, "local dev\n");
    let src = format!(
        "source {}\nproc local {{mode}} {{\n    if {{$mode eq \"prod\"}} {{ set r 1 }} else {{ set r 2 }}\n}}\nlocal prod\nlocal prod\n{SETTLED_MARKER}",
        sibling_name(&lib_uri),
    );
    lsp.open_document(&main_uri, &src);
    let diags = lsp.await_diagnostics_settled(&main_uri, Duration::from_secs(20), |d| {
        has_code(d, "W123") && !has_code(d, "I230")
    });
    assert!(
        !has_code(&diags, "I230"),
        "the sourced script calls ::local with \"dev\": {:?}",
        codes(&diags)
    );
}

/// A workspace pools every file into one project, so an unrelated file that
/// happens to reuse a common proc name must not drag its call sites into
/// this one's evidence. The VS Code suite shares one workspace folder
/// across roughly 200 fixtures, so a second global `helper` (zero arity)
/// defined in an unrelated fixture must not suppress this control.
#[test]
fn an_unrelated_file_reusing_a_proc_name_does_not_clear_i230() {
    let mut lsp = Lsp::tcl();
    let unrelated_uri = unique_uri("tcl");
    let ctl_uri = unique_uri("tcl");
    lsp.open_ready(
        &unrelated_uri,
        "proc helper {} {}\nproc caller {} { helper }\n",
    );
    let diags = lsp.open_ready(&ctl_uri, CROSS_FILE_LIB);
    assert!(
        has_code(&diags, "I230"),
        "the unrelated file's own `helper` binds to its own definition: {:?}",
        codes(&diags)
    );
}
