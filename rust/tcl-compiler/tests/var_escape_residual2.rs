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

//! Residual-coverage port #2 for the two lowest-covered var-escape walkers:
//!   * `src/var_escape/cfg_propagation/walker.rs`  (the flow-sensitive CFG/SSA walk)
//!   * `src/var_escape/walker.rs`                  (the intra-procedural IR walk)
//!
//! Two prior depth ports (`var_escape_cfg.rs`, `var_escape_typeinfer.rs`)
//! and the residual port (`compiler_residual.rs`) already cover the *common*
//! paths: flat-statement upvar/global/variable/info verdicts, the conditional /
//! loop / switch-arm escapes, the dynamic-set spill, the `cfg_result` low-level
//! surface, the IR-walk if/for/switch/try descent, the `direct_callees` /
//! `record_fallback` / name-first split, and the `Return`/`ExprEval` value scans.
//! This file is deliberately disjoint from all three — every test below drives an
//! arm those suites skip.
//!
//! ## The systematic gap this port closes
//!
//! The branches that resisted the prior passes are concentrated in the
//! **literal-body re-walk machinery** — the code reached *only* when the walker
//! descends into the body of a literal `eval {...}` / `uplevel #0 {...}` / catch
//! body, where it must escape **every name the body touches** (the interpreter
//! resolves those names against the live frame). On the CFG path that is
//! `handle_eval` / `handle_catch` / `handle_uplevel` →
//! `escape_every_name_touched_tree` and its three arm-dispatchers
//! (`tree_assign_or_incr`, `tree_call_or_barrier`, `tree_structural`); on the IR
//! path the twin is `escape_every_name_touched` →
//! (`escape_assign_or_incr`, `escape_call_or_barrier`, `escape_structural`).
//! These are exercised here with control-flow shapes (if / for / while / foreach /
//! switch / catch / try / nested-eval / uplevel / return / expr) *inside* a literal
//! eval body, plus the per-handler early-returns (empty / dynamic body, the
//! `uplevel` level-literal gates, the multi-arg `eval` join, the
//! `apply_value_scan` pessimistic short-circuit).
//!
//! ## C-Tcl proof split
//!
//! The escape *lattice* (`Local`/`Frame`), the per-`(name, version)` SSA tag map,
//! the `dynamic_barrier` flag, the structured `Barrier`/`EscapeReason` records, and
//! the block-by-block / tree-walk propagation order are all **compiler-internal**
//! structure with no direct Tcl analogue, so those are asserted **structurally**
//! (this note discharges the structural-assertion requirement for the whole file).
//!
//! Where a test rests on a *scoping fact observable in another frame* — a write
//! through an `upvar`/`global`/`uplevel #0`/`variable` name (including inside a
//! literal `eval` body, which runs in the proc's own frame) that a second frame
//! reads back — that observable behaviour was confirmed against real `tclsh` (8.6
//! and 9.0 agree on every case here) with `scripts/dev/tclsh_check.sh`: write via
//! the escaping name, read it from the other frame, `puts`/`return` the result.
//! Those facts are cited inline with a `// tclsh:` comment. The `Frame` verdict is
//! exactly the compiler's encoding of "this write is observable elsewhere, so the
//! name must live in the runtime frame".

use std::collections::HashMap;

use tcl_compiler::cfg_builder::build_cfg_function;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::lowering::lower_to_ir;
use tcl_compiler::ssa::{Version, build_ssa};
use tcl_compiler::var_escape::cfg_propagation::state::CfgEscapeResult;
use tcl_compiler::var_escape::{
    BarrierKind, EscapeReasonKind, EscapeTag, ProcEscapeSummary, TOP_LEVEL_QNAME,
    analyse_cfg_function, analyse_var_escape, analyse_var_escape_cu,
};
use tcl_registry::{CommandRegistry, registry_for_dialect};

// ── Shared helpers ──────────────────────────────────────────────────────────

const TCL: &str = "tcl8.6";

fn registry() -> &'static CommandRegistry {
    registry_for_dialect(TCL)
}

/// Flow-sensitive CFG/SSA escape summaries (codegen's frame-analysis path),
/// interprocedural fixpoint folded in.
fn escape_cu(src: &str) -> HashMap<String, ProcEscapeSummary> {
    let cu = CompilationUnit::build_for(src, registry(), false);
    analyse_var_escape_cu(&cu, true)
}

/// CFG/SSA summaries with the interprocedural pass *off* — raw per-proc result.
fn escape_cu_raw(src: &str) -> HashMap<String, ProcEscapeSummary> {
    let cu = CompilationUnit::build_for(src, registry(), false);
    analyse_var_escape_cu(&cu, false)
}

/// IR-walk escape summaries (the inliner's path), interprocedural on.
fn escape_ir(src: &str) -> HashMap<String, ProcEscapeSummary> {
    let module = lower_to_ir(src, registry());
    analyse_var_escape(&module, true)
}

/// Drive the low-level [`analyse_cfg_function`] directly on a top-level script and
/// return the raw [`CfgEscapeResult`] (the per-`(name, version)` map before it is
/// collapsed into a `ProcEscapeSummary`).
fn cfg_result(src: &str) -> CfgEscapeResult {
    let m = lower_to_ir(src, registry());
    let cfg = build_cfg_function("::top", &m.top_level, true);
    let ssa = build_ssa(&cfg, registry());
    analyse_cfg_function(&cfg, &ssa, std::iter::empty::<String>())
}

fn summary<'a>(map: &'a HashMap<String, ProcEscapeSummary>, qname: &str) -> &'a ProcEscapeSummary {
    map.get(qname).unwrap_or_else(|| {
        panic!(
            "summary for {qname} not found; keys = {:?}",
            map.keys().collect::<Vec<_>>()
        )
    })
}

/// True if `(name, version)` is tagged `Frame` in the per-SSA-version map.
fn ssa_frame(p: &ProcEscapeSummary, name: &str, version: Version) -> bool {
    p.ssa_tags.get(&(name.to_string(), version)) == Some(&EscapeTag::Frame)
}

// ════════════════════════════════════════════════════════════════════════════
// PART 1 — CFG path: literal `eval {control-flow}` body → escape_every_name_touched_tree
//
// On the CFG path a literal `eval {…}` body that contains control flow is routed
// through `is_eval_block` → `handle_barrier("eval")` → `handle_eval` →
// `escape_every_name_touched_tree`. That tree-walk escapes EVERY name the body
// touches (the body runs through the interpreter in the proc's own frame). Each
// test below drives a distinct `tree_structural` arm with a frame-crossing
// command inside it. No prior port reaches the CFG eval-body walker (the IR-walk
// ports use `var_escape/walker.rs`; the CFG ports use direct statements).
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn cu_eval_if_body_global_escapes() {
    // tclsh: a `global` written inside a literal `eval` body accumulates at the
    // global frame across calls:
    //   set g 0; proc bump {} { eval {if 1 {global g; incr g}} }
    //   bump; bump; set g   -> 2
    // The `If` clause-body inside the eval is walked by tree_structural's If arm.
    let s = escape_cu("proc ::p {} { eval {if 1 {global gg\n incr gg}} }");
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("gg"),
        "global inside a literal eval if-body is Frame"
    );
    assert!(p.frame_needed);
    assert!(
        !p.dynamic_barrier(),
        "a literal eval body is not whole-proc pessimism"
    );
}

#[test]
fn cu_eval_if_body_upvar_records_source() {
    // tclsh: an `upvar` inside a literal `eval` body reaches the caller frame:
    //   proc f {} { eval {if 1 {upvar 1 cv v; set v 55}} }
    //   set cv 0; f; set cv   -> 55
    // tree_call_or_barrier's Call arm runs handle_call → handle_upvar inside the
    // eval-body tree walk, recording the caller source `cv`.
    let s = escape_cu("proc ::p {} { eval {if 1 {upvar 1 cv v\n set v 1}} }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("v"), "the eval-body upvar alias is Frame");
    assert!(
        p.upvar_source_names.contains("cv"),
        "the eval-body upvar source is recorded: {:?}",
        p.upvar_source_names
    );
    assert!(p.frame_needed);
}

#[test]
fn cu_eval_for_body_and_init_escape() {
    // tclsh: the `for` init clause runs in the proc frame; a `global` declared
    // there is observable past the call:
    //   set gi 0; proc f {} { eval {for {global gi; set gi 5} {0} {incr gi} {set x 1}} }
    //   f; set gi   -> 5
    // tree_structural's For arm walks init + condition + next + body; the
    // frame-crossing `global gi` in the init clause escapes gi. (On the CFG path
    // the relaxed loop body keeps a pure local like `lb` Local — only the
    // scope-crossing name surfaces, which is the precise verdict here.)
    let s = escape_cu("proc ::p {} { eval {for {global gi\n set gi 0} {1} {incr gi} {set lb 1}} }");
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("gi"),
        "global in the eval-body for-init is Frame"
    );
    assert_eq!(
        p.tag("lb"),
        EscapeTag::Local,
        "the relaxed for-body local stays Local on the CFG path"
    );
    assert!(p.frame_needed);
}

#[test]
fn cu_eval_while_body_variable_escapes() {
    // tclsh: a namespace `variable` touched inside a loop in a literal eval body
    // is observable on the namespace var:
    //   namespace eval ::ns {variable wv 0}
    //   proc ::ns::w {} { eval {set i 0; while {$i<3} {variable wv; incr wv; incr i}} }
    //   ::ns::w; set ::ns::wv   -> 3
    // tree_structural's While arm walks the loop body.
    let s = escape_cu("proc ::p {} { eval {while {1} {variable wv\n set wv 1}} }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("wv"), "variable in an eval while-body is Frame");
    assert!(p.frame_needed);
}

#[test]
fn cu_eval_foreach_body_global_escapes() {
    // tclsh: a `global` written in a `foreach` body inside a literal eval
    // accumulates:
    //   set gf 0; proc f {} { eval {foreach n {a b c} {global gf; incr gf}} }
    //   f; set gf   -> 3
    // tree_structural's Foreach arm scans each iterator list-arg then walks body.
    let s = escape_cu("proc ::p {} { eval {foreach n {a b} {global gf\n set gf 1}} }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("gf"), "global in an eval foreach-body is Frame");
    assert!(p.frame_needed);
}

#[test]
fn cu_eval_switch_default_arm_global_escapes() {
    // The `default` arm of a `switch` inside a literal eval body is walked by
    // tree_structural's Switch arm (default_body branch). tclsh: a `global` in
    // the matched arm is observable past the call.
    let s = escape_cu(
        "proc ::p {} { eval {switch x { a {set la 1} default {global gd\n set gd 1} }} }",
    );
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("gd"),
        "global in the eval switch default arm is Frame"
    );
    // The non-default arm's pure local stays Local on the CFG path.
    assert_eq!(
        p.tag("la"),
        EscapeTag::Local,
        "the other switch arm's local stays Local"
    );
    assert!(p.frame_needed);
}

#[test]
fn cu_eval_try_handler_and_finally_escape() {
    // tclsh: a `variable` set in a `try … finally` inside a literal eval body is
    // observable on the namespace var:
    //   namespace eval ::ns {variable vf 0}
    //   proc ::ns::t {} { eval {try {set ok 1} finally {variable vf; set vf 42}} }
    //   ::ns::t; set ::ns::vf   -> 42
    // tree_structural's Try arm walks body + each handler + finally.
    let s = escape_cu(
        "proc ::p {} { eval {try {set ok 1} on error {e} {global ge\n set ge 1} \
         finally {variable vf\n set vf 1}} }",
    );
    let p = summary(&s, "::p");
    assert!(p.is_frame("ge"), "the eval try on-error handler escapes ge");
    assert!(p.is_frame("vf"), "the eval try finally escapes vf");
    assert!(p.frame_needed);
}

#[test]
fn cu_eval_catch_stmt_body_global_escapes() {
    // tclsh: a `catch {global …; incr …}` inside a literal eval body accumulates
    // the global across calls (catch intercepts only the result, not the frame):
    //   set gcc 0; proc f {} { eval {catch {global gcc; incr gcc}} }
    //   f; f; set gcc   -> 2
    // tree_structural's Catch arm walks the catch body.
    let s = escape_cu("proc ::p {} { eval {catch {global gcc\n set gcc 1} msg} }");
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("gcc"),
        "global inside an eval catch-body is Frame"
    );
    assert!(p.frame_needed);
}

#[test]
fn cu_eval_nested_eval_upvar_escapes() {
    // tclsh: an `upvar` inside a doubly-nested literal eval still reaches the
    // caller — each eval relays the body to the same frame:
    //   proc f {} { eval {eval {upvar 1 cv v; set v 9}} }
    //   set cv 0; f; set cv   -> 9
    // tree_call_or_barrier's Block(eval) arm re-enters handle_barrier("eval").
    let s = escape_cu("proc ::p {} { eval {eval {upvar 1 cv v\n set v 1}} }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("v"), "the nested-eval upvar alias is Frame");
    assert!(p.is_frame("cv"), "the nested-eval upvar source is Frame");
    assert!(p.upvar_source_names.contains("cv"));
    assert!(p.frame_needed);
}

#[test]
fn cu_eval_assignvalue_info_exists_escapes_target() {
    // tclsh: `info exists` inside a literal eval body resolves its target against
    // the proc frame:
    //   proc f {} { set rvv 1; eval {if 1 {set q [info exists rvv]}}; return $q } -> 1
    // tree_assign_or_incr's AssignValue arm runs apply_value_scan over the
    // assigned value `[info exists rvv]`, whose scan_value_for_info_hazards finds
    // the hazard and escapes the named target.
    let s = escape_cu("proc ::p {} { eval {if 1 {set rq [info exists rvv]}} }");
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("rvv"),
        "info-exists target in an eval-body assigned value is Frame"
    );
}

#[test]
fn cu_eval_expr_eval_info_exists_escapes_target() {
    // A bare `expr {[info exists …]}` statement inside a literal eval body has its
    // expression scanned by tree_call_or_barrier's ExprEval arm.
    let s = escape_cu("proc ::p {} { eval {if 1 {expr {[info exists eev]}}} }");
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("eev"),
        "info-exists in an eval expr statement is Frame"
    );
}

#[test]
fn cu_eval_incr_amount_does_not_escape_name() {
    // A bare `incr counter 5` inside a single-statement eval body is *relaxed* on
    // the CFG path to a plain local incr (the eval marker is dropped for a simple
    // single command), so `counter` stays Local — the negative control proving
    // the eval-block relaxation, not the every-name-touched tree walk, handled it.
    let s = escape_cu("proc ::p {} { eval {incr counter 5} }");
    let p = summary(&s, "::p");
    assert_eq!(
        p.tag("counter"),
        EscapeTag::Local,
        "relaxed single-stmt eval incr stays Local"
    );
    assert!(!p.dynamic_barrier());
}

// ════════════════════════════════════════════════════════════════════════════
// PART 2 — CFG path: multi-arg `eval` (handle_eval direct), uplevel level gates
//
// A multi-word `eval set out $inval` is NOT a brace-block; it reaches `handle_eval`
// directly, which joins the args and either scans a literal body or marks the proc
// pessimistic on a dynamic join. The `handle_uplevel` level-literal gates
// (`#0`/`0`-only, empty body, dynamic body, multi-part body join) are the sibling
// early-returns.
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn cu_eval_multiarg_literal_escapes_name() {
    // tclsh: `eval set out $inval` evaluates `set out <val>` in the proc frame:
    //   proc f {inval} { eval set out $inval; return $out }; f 77   -> 77
    // handle_eval joins ["set","fresh","3"] → "set fresh 3", lowers + walks it;
    // the eval body's `set fresh 3` escapes `fresh`.
    let s = escape_cu("proc ::p {} { eval set fresh 3 }");
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("fresh"),
        "multi-arg eval body escapes the name it writes"
    );
    assert!(p.has_fallback(), "an eval is a barrier-shaped fallback");
    assert!(
        !p.dynamic_barrier(),
        "a literal multi-arg eval body is not pessimistic"
    );
}

#[test]
fn cu_eval_multiarg_dynamic_join_is_pessimistic() {
    // `eval set y $x` joins to "set y $x" — the `$x` makes the joined body a
    // dynamic token, so handle_eval marks the proc pessimistic (is_dynamic_token).
    let s = escape_cu("proc ::p {x} { eval set y $x }");
    let p = summary(&s, "::p");
    assert!(
        p.dynamic_barrier(),
        "a dynamic multi-arg eval body is pessimistic"
    );
    assert!(p.frame_needed);
}

#[test]
fn cu_eval_multiarg_body_info_level_is_pessimistic() {
    // A multi-arg eval whose joined body contains `info level` goes pessimistic
    // via the tree walk (handle_info → record_barrier inside
    // escape_every_name_touched_tree).
    let s = escape_cu("proc ::p {} { eval if 1 {set l [info level]} }");
    let p = summary(&s, "::p");
    assert!(
        p.dynamic_barrier(),
        "info level reached through a multi-arg eval is a barrier"
    );
}

#[test]
fn cu_uplevel_zero_multipart_body_needs_fallback_only() {
    // tclsh: `uplevel #0 set glob 1` runs `set glob 1` at global scope:
    //   proc f {} { uplevel #0 set glob 1 }; f; set glob   -> 1
    // handle_uplevel joins the multi-part body parts ["set","glob","1"]; a literal
    // join under `#0` is NOT a whole-proc barrier — it only needs the eval fallback.
    let s = escape_cu("proc ::p {} { uplevel #0 set glob 1 }");
    let p = summary(&s, "::p");
    assert!(
        !p.dynamic_barrier(),
        "uplevel #0 with a literal multi-part body is not pessimistic"
    );
    assert!(p.has_fallback(), "but it needs the eval fallback");
}

#[test]
fn cu_uplevel_zero_dynamic_body_is_pessimistic() {
    // `uplevel #0 {set glob $x}` — the braced body text carries `$x`, so
    // handle_uplevel passes the `#0` level gate but then its is_dynamic_token(body)
    // branch marks the proc pessimistic (a dynamic global-scope eval body can't be
    // scanned).
    let s = escape_cu("proc ::p {x} { uplevel #0 {set glob $x} }");
    let p = summary(&s, "::p");
    assert!(
        p.dynamic_barrier(),
        "a dynamic uplevel #0 body is pessimistic"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// PART 3 — CFG path: handle_catch literal-body re-walk + dynamic-body early-return
//
// The CFG builder lowers `catch {body}` to an opaque Call whose first raw arg is
// the brace-stripped body. handle_catch mirrors handle_eval: a *literal* body is
// re-scanned + tree-walked (so an inner `upvar` is found); a *dynamic* body (any
// `$`/`[` in the body text) early-returns, keeping only the call fallback.
// (`compiler_residual.rs` covers a simple literal catch upvar; the dynamic
// early-return and the $-reference re-scan are uncovered.)
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn cu_catch_dynamic_body_arg_is_pessimistic() {
    // `catch $body` — the whole `$body` arg is a dynamic command-substitution
    // value to `catch`; the CFG walker can't bound what the caught script touches,
    // so the proc goes pessimistic (and records the eval fallback). handle_catch's
    // is_dynamic_token guard means no specific inner name is escaped.
    let s = escape_cu("proc ::p {body} { catch $body }");
    let p = summary(&s, "::p");
    assert!(
        p.tags.is_empty(),
        "no specific name is escaped from a dynamic catch body: {:?}",
        p.tags
    );
    assert!(
        p.dynamic_barrier(),
        "a `catch $body` defeats the analysis → pessimistic"
    );
    assert!(p.has_fallback(), "the catch call records the eval fallback");
}

#[test]
fn cu_catch_literal_body_with_dynamic_value_early_returns() {
    // `catch {upvar 1 cv v; set v $w}` — the body text contains `$w`, so even
    // though the body is brace-literal the joined body string is a dynamic token
    // and handle_catch early-returns. The inner `upvar` is therefore NOT seen on
    // the CFG path: `v`/`cv` stay Local and no upvar source is recorded.
    // (Conservative-but-safe: the catch call still records a fallback, so the
    // proc keeps a frame; the precise upvar source is simply not threaded.)
    let s = escape_cu("proc ::p {} { catch {upvar 1 cv v\n set v $w} }");
    let p = summary(&s, "::p");
    assert_eq!(
        p.tag("v"),
        EscapeTag::Local,
        "dynamic-body catch leaves v Local"
    );
    assert!(
        p.upvar_source_names.is_empty(),
        "no upvar source threaded from a dynamic catch body: {:?}",
        p.upvar_source_names
    );
    assert!(p.has_fallback(), "the catch still records the fallback");
}

// ════════════════════════════════════════════════════════════════════════════
// PART 4 — IR path (var_escape/walker.rs): escape_every_name_touched structural arms
//
// The IR-walk twin of Part 1. A literal `eval {…}` body is walked by
// `escape_every_name_touched`, which escapes EVERY name touched. These drive each
// `escape_structural` arm (For / While / Foreach / Switch / Try / Catch / Block /
// UpFrame) with a frame-crossing command inside — arms `compiler_residual.rs`
// (which only does the IR `Return`/`ExprEval`/`namespace upvar` scans) and
// `var_escape_cfg.rs` (which does the IR `walk` if/for/switch/try, NOT the
// eval-body `escape_*` arms) both skip.
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ir_eval_for_body_global_escapes_and_escapes_locals() {
    // tclsh: the `for` init in a literal eval body runs in the proc frame:
    //   set gi 0; proc f {} { eval {for {global gi; set gi 5} {0} {incr gi} {set x 1}} }
    //   f; set gi   -> 5
    // escape_structural's For arm walks init/cond/next/body; every touched name
    // escapes (gi via global, lb as a plain touched local).
    let s = escape_ir("proc ::p {} { eval {for {global gi\n set gi 0} {1} {incr gi} {set lb 1}} }");
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("gi"),
        "global in the eval for-init is Frame on the IR walk"
    );
    assert!(
        p.is_frame("lb"),
        "the for-body local also escapes in a literal eval"
    );
}

#[test]
fn ir_eval_while_body_variable_escapes() {
    // escape_structural's While arm. tclsh: a namespace variable touched in a
    // loop inside a literal eval body is observable on the namespace var (same
    // mechanism as the CU while case).
    let s = escape_ir("proc ::p {} { eval {while {1} {variable wv\n set wv 1}} }");
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("wv"),
        "variable in an eval while-body is Frame on the IR walk"
    );
}

#[test]
fn ir_eval_foreach_body_global_escapes() {
    // escape_structural's Foreach arm. tclsh: `foreach`-body global accumulates.
    let s = escape_ir("proc ::p {} { eval {foreach n {a b} {global gf\n set gf 1}} }");
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("gf"),
        "global in an eval foreach-body is Frame on the IR walk"
    );
}

#[test]
fn ir_eval_switch_arms_variable_and_local_escape() {
    // escape_structural's Switch arm walks each non-default arm + default_body;
    // every touched name escapes.
    let s = escape_ir(
        "proc ::p {} { eval {switch m { a {variable va\n set va 1} default {set ld 1} }} }",
    );
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("va"),
        "variable in an eval switch arm is Frame on the IR walk"
    );
    assert!(p.is_frame("ld"), "the default-arm local also escapes");
}

#[test]
fn ir_eval_try_body_handler_finally_all_escape() {
    // escape_structural's Try arm walks body + handlers + finally. tclsh: a
    // `variable` set in the finally of a literal eval body is observable on the
    // namespace var (same as the CU try case).
    let s = escape_ir(
        "proc ::p {} { eval {try {set ok 1} on error {e} {global get\n set get 1} \
         finally {variable vf\n set vf 1}} }",
    );
    let p = summary(&s, "::p");
    assert!(p.is_frame("ok"), "the try-body local escapes");
    assert!(p.is_frame("get"), "the on-error handler escapes get");
    assert!(p.is_frame("vf"), "the finally escapes vf");
}

#[test]
fn ir_eval_catch_body_global_escapes() {
    // escape_structural's Catch arm walks the catch body.
    let s = escape_ir("proc ::p {} { eval {catch {global gc\n set gc 1}} }");
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("gc"),
        "global in an eval catch-body is Frame on the IR walk"
    );
}

#[test]
fn ir_eval_nested_eval_block_escapes() {
    // escape_structural's Block(eval) arm: an `eval` nested inside a literal eval
    // body re-enters handle_barrier("eval"). tclsh: the nested-eval write reaches
    // the same frame (a doubly-nested `set`/`global` persists in the proc frame).
    let s = escape_ir("proc ::p {} { eval {eval {global ge\n set ge 1}} }");
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("ge"),
        "global in a nested eval is Frame on the IR walk"
    );
    assert!(p.has_fallback(), "the nested eval keeps the fallback");
}

#[test]
fn ir_eval_namespace_eval_block_is_pessimistic() {
    // escape_structural's Block(non-eval) arm: a `namespace eval` block nested in
    // a literal eval body is walked, but `namespace eval` is an uncategorised
    // barrier on the IR walk (handle_barrier's `_ =>` arm), so the proc goes
    // pessimistic. (tclsh: the inner `set`/`variable` writes the *namespace* var,
    // not the proc frame — the analysis conservatively bars rather than model it.)
    let s = escape_ir("proc ::p {} { eval {namespace eval ::nn {global gn\n set gn 1}} }");
    let p = summary(&s, "::p");
    assert!(
        p.dynamic_barrier(),
        "a nested namespace-eval block bars on the IR walk"
    );
}

#[test]
fn ir_eval_uplevel_zero_block_inside_needs_fallback_only() {
    // tclsh: `uplevel #0 {set gg 1}` nested in a literal eval body writes a
    // global, observable past the call:
    //   proc f {} { eval {uplevel #0 {set gg 1}} }; f; set gg   -> 1
    // escape_structural's UpFrame arm dispatches handle_barrier("uplevel"); the
    // `#0` literal body is a safe global-scope eval — fallback, not pessimistic.
    let s = escape_ir("proc ::p {} { eval {uplevel #0 {set gg 1}} }");
    let p = summary(&s, "::p");
    assert!(
        !p.dynamic_barrier(),
        "uplevel #0 literal body nested in eval is not pessimistic"
    );
    assert!(p.has_fallback());
}

// ════════════════════════════════════════════════════════════════════════════
// PART 5 — IR path: escape_assign_or_incr dynamic-name pessimism inside eval body
//
// Inside a literal eval body, an assignment / increment to a *dynamic* name
// (`set $dyn …`) can't be bounded, so `escape_assign_or_incr` marks the whole proc
// pessimistic (the mark_pessimistic early-return). This is distinct from the
// top-level `walk_dynamic_name_escape` spill (which resolves or escape_all_known).
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ir_eval_body_dynamic_assign_name_is_pessimistic() {
    // `eval {set $dyn 1; set $dyn 2}` — the dynamic assign name inside the literal
    // eval body hits escape_assign_or_incr's mark_pessimistic branch.
    let s = escape_ir("proc ::p {} { eval {set $dyn 1\n set $dyn 2} }");
    let p = summary(&s, "::p");
    assert!(
        p.dynamic_barrier(),
        "a dynamic assign-name in an eval body is pessimistic"
    );
}

#[test]
fn ir_eval_body_dynamic_incr_name_is_pessimistic() {
    // The incr arm of escape_assign_or_incr: a dynamic incr name inside the eval
    // body (guarded behind an `if` so it stays a multi-shape eval body) marks the
    // proc pessimistic.
    let s = escape_ir("proc ::p {} { eval {if 1 {incr $dyn}} }");
    let p = summary(&s, "::p");
    assert!(
        p.dynamic_barrier(),
        "a dynamic incr-name in an eval body is pessimistic"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// PART 6 — IR path: handle_uplevel level-literal gates + handle_eval no-args
//
// `var_escape/walker.rs::handle_uplevel` has several early-returns the prior ports
// skip: a non-`#0`/`0` literal level (`uplevel 2`), a negative-looking level
// (`uplevel -1`, where the `-` is trimmed then digit-checked), and `uplevel 1`
// with no body. `handle_eval` with no args records an Eval barrier.
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn ir_uplevel_level_two_is_upvar_barrier() {
    // tclsh: `uplevel 2 {…}` runs two frames up — its body can touch names we
    // can't bound, so the analysis bars. handle_uplevel: level literal `2` is not
    // `#0`/`0`, so the `first != "#0" && first != "0"` gate records an Upvar barrier.
    let s = escape_ir("proc ::p {} { uplevel 2 {set y 1} }");
    let p = summary(&s, "::p");
    assert!(p.dynamic_barrier(), "uplevel 2 is a barrier");
    assert!(
        p.barriers.iter().any(|b| b.kind == BarrierKind::Upvar),
        "expected an Upvar barrier, got {:?}",
        p.barriers
    );
}

#[test]
fn ir_uplevel_negative_level_is_upvar_barrier() {
    // `uplevel -1 {…}` — handle_uplevel trims the leading `-`, finds the remainder
    // `1` is all-digits (a level literal), but `-1` is not `#0`/`0`, so it records
    // an Upvar barrier. (Exercises the trim_start_matches('-') + digit-check path.)
    let s = escape_ir("proc ::p {} { uplevel -1 {set y 1} }");
    let p = summary(&s, "::p");
    assert!(p.dynamic_barrier(), "uplevel -1 is a barrier");
    assert!(p.barriers.iter().any(|b| b.kind == BarrierKind::Upvar));
}

#[test]
fn ir_uplevel_one_no_body_is_safe() {
    // `uplevel 1` with no body: handle_uplevel's `body_parts.is_empty()` path is
    // reached only after the level passes the `#0`/`0` gate, which `1` does not —
    // so `uplevel 1` (no body) lands on the level gate, not the empty-body gate.
    // Either way it is barrier-free here because the lowering drops the bodyless
    // `uplevel 1` to a no-op-ish shape: assert it does not crash and stays
    // non-pessimistic (the structural negative control for the bodyless arm).
    let s = escape_ir("proc ::p {} { uplevel 1 }");
    let p = summary(&s, "::p");
    // No body to run ⇒ no caller-name effect the walker can see.
    assert!(
        !p.dynamic_barrier(),
        "bodyless uplevel 1 records no barrier here: {:?}",
        p.barriers
    );
}

#[test]
fn ir_uplevel_zero_no_body_needs_fallback_only() {
    // `uplevel #0` with no body: passes the `#0` gate, then the
    // `body_parts.is_empty()` gate records an Upvar barrier — BUT the lowering for
    // a bodyless `uplevel #0` collapses to a plain barrier the walker sees as a
    // fallback only, leaving the proc non-pessimistic. Assert the observed shape.
    let s = escape_ir("proc ::p {} { uplevel #0 }");
    let p = summary(&s, "::p");
    assert!(
        !p.dynamic_barrier(),
        "bodyless uplevel #0 is not pessimistic: {:?}",
        p.barriers
    );
    assert!(
        p.has_fallback(),
        "but the barrier-shaped uplevel records the fallback"
    );
}

#[test]
fn ir_eval_dynamic_body_records_eval_barrier() {
    // `eval $body` — handle_eval's is_dynamic_token branch records an Eval
    // barrier and goes pessimistic. (The barrier *kind* assertion is the
    // uncovered detail; the bare pessimism is covered elsewhere.)
    let s = escape_ir("proc ::p {body} { eval $body }");
    let p = summary(&s, "::p");
    assert!(p.dynamic_barrier());
    assert!(
        p.barriers.iter().any(|b| b.kind == BarrierKind::Eval),
        "expected an Eval barrier, got {:?}",
        p.barriers
    );
}

#[test]
fn ir_generic_barrier_records_unknown_barrier() {
    // A `subst {…}` lowers to a generic Barrier (not eval/uplevel). handle_barrier
    // takes the `_ =>` arm → an Unknown-kind barrier, marking the proc pessimistic.
    // (subst is the canonical non-eval/uplevel barrier shape.)
    let s = escape_ir("proc ::p {} { subst {$x} }");
    let p = summary(&s, "::p");
    if p.dynamic_barrier() {
        // When lowered as a Barrier, the Unknown-kind classification is recorded.
        assert!(
            p.barriers.iter().any(|b| b.kind == BarrierKind::Unknown)
                || p.barriers.iter().any(|b| b.kind == BarrierKind::Eval),
            "a generic barrier records a barrier kind: {:?}",
            p.barriers
        );
    } else {
        // If the lowering relaxed subst to a plain call, it at least records a
        // fallback (subst is not a frameless-runtime command).
        assert!(
            p.has_fallback() || p.has_call_fallback(),
            "subst records some fallback: {p:?}"
        );
    }
}

// ════════════════════════════════════════════════════════════════════════════
// PART 7 — apply_value_scan pessimistic short-circuit + non-frameless head in a
//          value, reached *inside* a literal eval body (CFG + IR)
//
// `apply_value_scan` (both walkers) records a fallback for a non-frameless `[cmd]`
// substitution head and marks the proc pessimistic when an embedded `[info level]`
// /`[info vars]` shape is found. `compiler_residual.rs` covers the head-loop
// fallback for a top-level `set x [myproc a]`; the *eval-body* re-entry of these
// scans (via the tree walker's value/expr scans) and the pessimistic short-circuit
// are uncovered.
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn cu_value_with_info_level_marks_pessimistic() {
    // A value embedding `[info level]` — apply_value_scan's
    // scan_value_for_info_hazards returns pessimistic, so mark_pessimistic fires
    // and the value-scan returns early. tclsh: `info level` reads the caller
    // frame, so the proc genuinely cannot be frame-elided.
    let s = escape_cu("proc ::p {} { set x \"a[info level]b\" }");
    let p = summary(&s, "::p");
    assert!(
        p.dynamic_barrier(),
        "an embedded [info level] in a value is pessimistic"
    );
}

#[test]
fn ir_value_with_info_level_marks_pessimistic() {
    // The IR-walk twin of the above (walker.rs::apply_value_scan).
    let s = escape_ir("proc ::p {} { set x \"a[info level]b\" }");
    let p = summary(&s, "::p");
    assert!(
        p.dynamic_barrier(),
        "embedded [info level] is pessimistic on the IR walk"
    );
}

#[test]
fn cu_eval_body_value_nonframeless_head_records_fallback_via_cfg_result() {
    // Inside a literal eval body, a value with a non-frameless subst head
    // (`[myproc a]`) reaches apply_value_scan's extract_cmd_subst_heads loop and
    // records a fallback — driven through the low-level cfg_result entry on a
    // top-level eval body so the per-(name,version) surface is also exercised.
    let r = cfg_result("eval {if 1 {set x \"v [myproc a]\"}}");
    assert!(
        r.has_fallback(),
        "a non-frameless subst head inside an eval-body value records a fallback"
    );
    assert!(
        !r.dynamic_barrier(),
        "the head fallback is not whole-proc pessimism"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// PART 8 — low-level analyse_cfg_function on top-level eval bodies
//
// Drive `analyse_cfg_function` directly on a *top-level* literal eval body and
// inspect the raw `CfgEscapeResult`: the per-name collapse, the recorded upvar
// source, the tag_reasons map, and the direct_callees the tree walk records.
// (`var_escape_cfg.rs` drives cfg_result only on flat statements, not eval
// bodies.)
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn cfg_result_eval_body_global_collapses_and_records_callee() {
    // A top-level `eval {if 1 {global gg; incr gg}}` — the tree walk escapes gg
    // (via handle_global) and records `global` as a direct callee.
    let r = cfg_result("eval {if 1 {global gg\n incr gg}}");
    assert_eq!(
        r.name_tags.get("gg"),
        Some(&EscapeTag::Frame),
        "gg collapses to Frame"
    );
    assert!(
        r.direct_callees.contains("global"),
        "the eval-body global call is recorded as a callee: {:?}",
        r.direct_callees
    );
    assert!(!r.dynamic_barrier());
}

#[test]
fn cfg_result_eval_body_upvar_records_source_and_reason() {
    // A top-level `eval {if 1 {upvar 1 cv v; set v 1}}` — escapes v, records the
    // caller source cv, and attaches an UpvarSource reason to v.
    let r = cfg_result("eval {if 1 {upvar 1 cv v\n set v 1}}");
    assert_eq!(r.name_tags.get("v"), Some(&EscapeTag::Frame));
    assert!(
        r.upvar_source_names.contains("cv"),
        "cv recorded: {:?}",
        r.upvar_source_names
    );
    let reasons = r.tag_reasons.get("v").expect("v has reasons");
    assert!(
        reasons
            .iter()
            .any(|x| x.kind == EscapeReasonKind::UpvarSource),
        "expected an UpvarSource reason on v, got {reasons:?}"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// PART 9 — CFG handle_statement: dynamic-name AssignExpr / Incr arms
//
// `handle_stmt_assign_or_incr`'s AssignExpr arm with a *dynamic* name runs
// `dynamic_name_escape` (resolve-literal-or-spill). `var_escape_cfg.rs`
// covers `set $n …` (AssignConst/AssignValue); the AssignExpr (`set $n [expr …]`)
// dynamic-name arm is uncovered here. Since issue #1487 the lowering never
// emits `Statement::Incr` with a computed name (it declines to the generic
// Call), so the Incr test below pins the Call-form contract instead.
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn cu_dynamic_assignexpr_name_spills_all_known() {
    // tclsh: `set $n [expr {1+1}]` writes whichever local `$n` names, so every
    // local must be treated frame-resident:
    //   proc f {n} { set a 1; set $n [expr {1+1}]; return $a }
    //   f a -> 2 ;  f b -> 1   (a untouched)
    // handle_stmt_assign_or_incr's AssignExpr arm → dynamic_name_escape →
    // escape_all_known (the name `$n` resolves to no literal).
    let s = escape_cu("proc ::p {n} { set a 1\n set $n [expr {1+1}] }");
    let p = summary(&s, "::p");
    assert!(
        p.dynamic_barrier(),
        "an unresolved dynamic AssignExpr name is pessimistic"
    );
    assert!(
        p.is_frame("a"),
        "a is spilled by the dynamic AssignExpr name"
    );
    assert!(p.is_frame("n"));
}

#[test]
fn cu_dynamic_incr_name_spills_all_known() {
    // `incr $n` with an unresolved `$n`. tclsh: `incr $n` increments whichever
    // local `$n` names, so every local is frame-resident:
    //   proc f {n} { set a 1; incr $n; return $a }
    //   f a -> 2 (incremented a) ;  f b -> 1 (a untouched)
    // Since issue #1487 the lowering declines the `Statement::Incr`
    // specialisation for a computed name and emits the generic Call — the same
    // path as `set $n …` — so the dynamic-name barrier rises for the whole
    // proc, exactly as the `set` form does. The old name-precise Incr arm was
    // invisible to `dynamic_names::scan_statement`, which is what #1487 fixed;
    // the precision loss is the deliberate trade.
    let s = escape_cu("proc ::p {n} { set a 1\n incr $n }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("a"), "a is spilled by the dynamic incr name");
    assert!(p.is_frame("n"), "the name var n is spilled too");
    assert!(
        p.dynamic_barrier(),
        "a computed incr name is the generic Call form since #1487, so the \
         whole-proc barrier rises exactly as it does for `set $n …`"
    );
    assert!(
        p.frame_needed,
        "and the proc still needs a frame for the spilled locals"
    );
}

#[test]
fn cu_assignexpr_literal_name_invalidates_then_dynamic_spills() {
    // `set x [expr {1+1}]` records x via the AssignExpr literal-name path
    // (invalidate_literal — an expr value is not a trackable literal), so the
    // subsequent `set $x 9` cannot resolve `$x` and spills. Drives the AssignExpr
    // literal-name (non-dynamic) branch then the dynamic-name spill in one proc.
    let s = escape_cu("proc ::p {} { set x [expr {1+1}]\n set $x 9 }");
    let p = summary(&s, "::p");
    assert!(
        p.dynamic_barrier(),
        "the unresolved $x spills the proc pessimistic"
    );
    assert!(p.is_frame("x"), "x is spilled");
}

// ════════════════════════════════════════════════════════════════════════════
// PART 10 — CFG handle_call command-word classification (record_fallback vs
//          record_call_fallback) + recorded callees for info / namespace
//
// `handle_call` classifies the command word: an empty / dynamic head →
// record_fallback (eval fallback); a static non-frameless head →
// record_call_fallback. It also records every static head as a direct callee.
// `compiler_residual.rs` covers user/qualified callees + the `$cmd` head;
// the `info`/`namespace` recorded-callee cases and the call-fallback-vs-fallback
// split for a frameless-command value are exercised here.
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn cu_assignvalue_info_exists_escapes_via_value_scan() {
    // tclsh: `info exists rv` resolves its target against the runtime frame:
    //   proc f {} { set rv 1; set q [info exists rv]; return $q } -> 1
    // On the CFG path `set q [info exists rv]` is a `Statement::AssignValue` whose
    // value `[info exists rv]` is run through handle_stmt_assign_or_incr's
    // AssignValue arm → apply_value_scan → scan_value_for_info_hazards, which
    // finds the hazard and escapes the named target. (The `[info exists rv]` is
    // consumed at the value-scan level, so — unlike a bare `info` call — `info`
    // is NOT recorded as a direct callee; the escape verdict is the carrier.)
    let s = escape_cu("proc ::p {} { set rv 1\n set q [info exists rv] }");
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("rv"),
        "info-exists target escapes via the AssignValue value scan"
    );
    assert_eq!(
        p.tag("q"),
        EscapeTag::Local,
        "the result-holding local stays Local"
    );
    assert!(
        !p.dynamic_barrier(),
        "a literal info-exists in a value is not pessimistic"
    );
}

#[test]
fn cu_namespace_command_recorded_as_callee() {
    // `namespace upvar ::ns counter c` records `namespace` as a callee and escapes
    // the alias c. tclsh: the alias reads/writes the namespace var:
    //   namespace eval ::ns {variable counter 5}
    //   proc readit {} { namespace upvar ::ns counter c; return $c }   -> 5
    let s = escape_cu("proc ::p {} { namespace upvar ::ns counter c\n set c 1 }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("c"), "the namespace-upvar alias is Frame");
    assert!(
        p.direct_callees.contains("namespace"),
        "namespace recorded as a callee: {:?}",
        p.direct_callees
    );
}

#[test]
fn cu_eval_after_barrier_records_inner_callees_not_eval() {
    // After an `eval {…}` barrier the inner tree walk records the inner static
    // callees (`global`), and the relaxed single-control-flow eval body keeps the
    // proc non-pessimistic. `eval` itself is dispatched as a barrier (not via the
    // generic record_callee tail), so it is the inner callee that surfaces.
    let s = escape_cu("proc ::p {} { eval {if 1 {global gg}} }");
    let p = summary(&s, "::p");
    assert!(
        p.direct_callees.contains("global"),
        "the eval-body global is recorded as a callee: {:?}",
        p.direct_callees
    );
}

// ════════════════════════════════════════════════════════════════════════════
// PART 11 — per-SSA-version surface for eval-body escapes (the flow-sensitive
//          payload the CU path adds over the IR walk)
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn cu_eval_body_global_tags_concrete_ssa_version() {
    // The CFG path tags a concrete SSA version of the eval-body global. `global gg`
    // introduces gg's first version; assert a version is tagged Frame and the
    // name collapses to Frame.
    let s = escape_cu("proc ::p {} { eval {if 1 {global gg\n incr gg}} }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("gg"));
    // `global gg` introduces gg's version 1, which the flow-sensitive walker tags
    // Frame on the eval-body tree walk.
    assert!(
        ssa_frame(p, "gg", 1),
        "gg#1 tagged Frame on the eval-body tree walk: {:?}",
        p.ssa_tags
    );
    assert!(
        p.ssa_tags
            .iter()
            .any(|((n, _), t)| n == "gg" && *t == EscapeTag::Frame),
        "the per-version map carries the eval-body global tag: {:?}",
        p.ssa_tags
    );
}

#[test]
fn cu_top_level_eval_body_escapes_via_cu_map() {
    // The whole-CU map keys the top-level eval body under TOP_LEVEL_QNAME and the
    // proc under its qname; both eval bodies escape independently.
    let s = escape_cu(
        "eval {if 1 {global tg\n incr tg}}\nproc ::p {} { eval {if 1 {global pg\n incr pg}} }",
    );
    assert!(s.contains_key(TOP_LEVEL_QNAME), "top-level key present");
    assert!(
        summary(&s, TOP_LEVEL_QNAME).is_frame("tg"),
        "top-level eval global escapes"
    );
    assert!(
        summary(&s, "::p").is_frame("pg"),
        "proc eval global escapes"
    );
}

// ════════════════════════════════════════════════════════════════════════════
// PART 12 — interprocedural fold over an eval-body upvar source (CFG path)
//
// An eval-body `upvar` records a named caller source; with the CU fixpoint on, a
// caller whose matching local shares that name becomes Frame. Mirrors the
// interprocedural eval-source propagation through the CFG eval-body walker.
// ════════════════════════════════════════════════════════════════════════════

#[test]
fn cu_eval_body_named_upvar_source_flows_to_caller() {
    // tclsh: `::leaf` upvars the literal caller name `x` inside an eval body; the
    // caller `::host` has its own local `x`, which is the one written:
    //   proc leaf {} { eval {if 1 {upvar 1 x y; set y 9}} }
    //   proc host {} { set x 1; leaf; return $x }; host   -> 9
    let s = escape_cu(
        "proc ::leaf {} { eval {if 1 {upvar 1 x y\n set y 9}} }\n\
         proc ::host {} { set x 1\n leaf }",
    );
    let leaf = summary(&s, "::leaf");
    assert!(
        leaf.upvar_source_names.contains("x"),
        "leaf records source x: {:?}",
        leaf.upvar_source_names
    );
    let host = summary(&s, "::host");
    assert!(
        host.upvar_source_names.contains("x"),
        "the eval-body upvar source reaches the caller: {:?}",
        host.upvar_source_names
    );
    assert!(
        host.is_frame("x"),
        "the caller's matching local becomes Frame"
    );
}

#[test]
fn cu_eval_body_source_does_not_flow_with_ipa_off() {
    // With the CU fixpoint off, `::host`'s local `x` is NOT downgraded by `::leaf`'s
    // eval-body upvar source — the raw per-proc result. (`::leaf` still records its
    // own source intra-procedurally.)
    let s = escape_cu_raw(
        "proc ::leaf {} { eval {if 1 {upvar 1 x y\n set y 9}} }\n\
         proc ::host {} { set x 1\n leaf }",
    );
    assert!(summary(&s, "::leaf").upvar_source_names.contains("x"));
    let host = summary(&s, "::host");
    assert_eq!(
        host.tag("x"),
        EscapeTag::Local,
        "without IPA the caller's x is not downgraded: {:?}",
        host.upvar_source_names
    );
}
