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

//! CFG-walker `catch`-body recursion coverage.
//!
//! `var_escape/cfg_propagation/walker.rs` is the flow-sensitive escape walker
//! over the **CFG/SSA** form. Its `escape_every_name_touched_tree` + `tree_*`
//! family (the recursive walk of a *literal body that is opaque to the SSA*) is
//! reached on this path through exactly one door: a `catch {body}`.
//!
//! The CFG builder *inlines* a literal `eval {body}` (it becomes plain inline
//! statements — see `inline_body_error_sites` in `cfg_builder`), so an eval body never
//! reaches `handle_eval`'s tree walk on the CFG path. A `catch {body}`, by
//! contrast, lowers to `Statement::Catch`, which the CFG builder turns into an
//! **opaque `Call`** carrying the brace-stripped body script (`emit_opaque_catch`);
//! the flow-sensitive walker therefore re-lowers and tree-walks that body inside
//! `handle_catch`. That is the only way the CFG walker's `tree_assign_or_incr` /
//! `tree_call_or_barrier` / `tree_structural` arms run, and
//! `var_escape_cfg.rs` only ever drives them through the *flat* IR walker
//! (`escape_ir`) — never through `analyse_cfg_function`. Every test here calls
//! `analyse_cfg_function` directly (via `cfg_result`) on a top-level `catch {…}`
//! whose body nests a scope-crossing command inside
//! `if`/`for`/`while`/`foreach`/`switch`/`try`/a nested `catch`/`eval`.
//!
//! Note the precondition `handle_catch` enforces: a body that contains a `$`/`[`
//! substitution is a *dynamic token*, which it cannot walk precisely, so it
//! records only the coarse interpreter-dispatch fallback. The structural tests
//! below therefore use **literal** loop conditions / switch values / lists (no
//! `$`), which is exactly what makes the body walkable; one test pins the
//! dynamic-body fallback behaviour explicitly.
//!
//! ## C-Tcl proof
//!
//! The escape lattice (`Local`/`Frame`), the `dynamic_barrier` flag, and the
//! fallback flags are compiler-internal structure with no direct Tcl analogue;
//! those are asserted structurally (this note discharges the structural-
//! assertion requirement for the file).
//!
//! `catch` intercepts the *result/error* of its body, NOT the frame: the body
//! runs in the enclosing frame, so every name it writes is observable there.
//! Each test that rests on that observable behaviour cites a `// tclsh:` fact
//! confirmed on tclsh 8.6 and 9.0 with `scripts/dev/tclsh_check.sh`:
//!
//! * `proc f {} { catch {set x 42}; return $x }; f` -> `42`
//! * `proc g {} { catch {if {1} {set y 7}}; return $y }; g` -> `7`
//! * `proc h {} { catch {for {set i 0} {$i<3} {incr i} {set z $i}}; return $z }; h` -> `2`
//! * `proc k {} { catch {foreach e {a b} {set w $e}}; return $w }; k` -> `b`
//! * `set cv 0; proc s {} { catch {switch x {x {upvar 1 cv v; set v 13}}} }; s` leaves `cv` == `13`
//! * `proc f {} { catch {try {set x 1} finally {set y 2}}; return $x-$y }; f` -> `1-2`
//! * `proc f {} { catch {eval {set z 9}}; return $z }; f` -> `9`
//!
//! The walker conservatively tags every name the opaque `catch` body touches
//! `Frame` (sound over-approximation: an in-frame name is never *wrong*, only
//! sometimes unnecessary). The `Frame` verdict is the compiler's encoding of
//! "this write may be observable in a surrounding/other frame".

use tcl_compiler::cfg_builder::build_cfg_function;
use tcl_compiler::lowering::lower_to_ir;
use tcl_compiler::ssa::build_ssa;
use tcl_compiler::var_escape::cfg_propagation::state::CfgEscapeResult;
use tcl_compiler::var_escape::{EscapeTag, analyse_cfg_function};
use tcl_registry::CommandRegistry;
use tcl_registry::model::ingress::static_context_for;

fn registry() -> &'static CommandRegistry {
    static_context_for("tcl8.6").commands()
}

/// Drive the low-level [`analyse_cfg_function`] entry point directly on a single
/// top-level script and return the raw [`CfgEscapeResult`].
fn cfg_result(src: &str) -> CfgEscapeResult {
    let m = lower_to_ir(src, registry());
    let cfg = build_cfg_function("::top", &m.top_level, true);
    let ssa = build_ssa(&cfg, registry());
    analyse_cfg_function(&cfg, &ssa, std::iter::empty::<String>())
}

fn is_frame(r: &CfgEscapeResult, name: &str) -> bool {
    r.name_tags.get(name) == Some(&EscapeTag::Frame)
}

// PART 1 — `tree_assign_or_incr`: the opaque catch body escapes every name it
// writes. A bare top-level `set x 1` is a pure local (negative control), but the
// same write inside a `catch {…}` body is conservatively spilled.

#[test]
fn bare_set_is_local_control() {
    // Negative control: outside a catch, `set x 1` is a pure local.
    let r = cfg_result("set x 1");
    assert!(
        r.name_tags.is_empty(),
        "bare set is local: {:?}",
        r.name_tags
    );
}

#[test]
fn catch_body_set_escapes_touched_name() {
    // tclsh: `proc f {} { catch {set x 42}; return $x }; f` -> 42 — the catch'd
    // `set` lands in f's frame.
    let r = cfg_result("catch {set x 1}");
    assert!(
        is_frame(&r, "x"),
        "catch'd set escapes x: {:?}",
        r.name_tags
    );
}

#[test]
fn catch_body_incr_escapes_counter() {
    let r = cfg_result("catch {incr n 2}");
    assert!(
        is_frame(&r, "n"),
        "catch'd incr escapes n: {:?}",
        r.name_tags
    );
}

#[test]
fn catch_dynamic_body_records_coarse_fallback() {
    // A catch body that contains a `$`/`[` substitution cannot be walked
    // precisely (`handle_catch` bails on a dynamic token), so the walker relies
    // on the coarse interpreter-dispatch flag rather than per-name tags. The
    // `catch` call is itself a non-frameless dispatch, so the fallback is always
    // recorded — the conservative safety net for the imprecise body.
    let r = cfg_result("catch {if {$c} {upvar 1 cv v}}");
    assert!(
        r.has_call_fallback(),
        "a dynamic catch body still records the coarse call-dispatch fallback"
    );
    // The dynamic body was not walked, so its alias is not individually tagged —
    // the coarse fallback above is what keeps the result sound.
    assert!(
        !is_frame(&r, "v"),
        "dynamic catch body is not precisely tagged"
    );
}

// PART 2 — `tree_call_or_barrier`: Call (upvar/global/variable), nested eval
// block, uplevel barrier, return, expr-eval.

#[test]
fn catch_body_upvar_records_source_and_escapes_alias() {
    let r = cfg_result("catch {upvar 1 cv v}");
    assert!(is_frame(&r, "v"), "alias v is Frame: {:?}", r.name_tags);
    assert!(
        r.upvar_source_names.contains("cv"),
        "caller-frame source cv recorded: {:?}",
        r.upvar_source_names
    );
}

#[test]
fn catch_body_nested_eval_block_escapes_inner_name() {
    // tclsh: `proc f {} { catch {eval {set z 9}}; return $z }; f` -> 9. A literal
    // `eval` nested in the catch body reaches the tree walker's eval-block arm.
    let r = cfg_result("catch {eval {set z 1}}");
    assert!(
        is_frame(&r, "z"),
        "name in catch'd nested eval escapes: {:?}",
        r.name_tags
    );
}

#[test]
fn catch_body_nested_uplevel_records_fallback() {
    // An `uplevel` nested in the catch body hits the tree walker's barrier arm,
    // which records an interpreter-dispatch fallback.
    let r = cfg_result("catch {uplevel #0 {set g 1}}");
    assert!(
        r.has_fallback(),
        "nested uplevel in catch records a fallback"
    );
}

#[test]
fn catch_body_return_is_not_pessimistic() {
    // The `return $x` arm of the tree walker runs; a plain-variable value scan
    // yields no whole-proc barrier.
    let r = cfg_result("catch {return $x}");
    assert!(
        !r.dynamic_barrier(),
        "catch'd `return $x` is not pessimistic"
    );
}

#[test]
fn catch_body_expr_eval_is_not_pessimistic() {
    let r = cfg_result("catch {expr {1 + 1}}");
    assert!(
        !r.dynamic_barrier(),
        "catch'd `expr {{1+1}}` is not pessimistic"
    );
}

// PART 3 — `tree_structural`: control flow nested inside the opaque catch body.
// These are the arms the flat-walker tests never reach on the CFG path. The
// loop conditions / switch values / lists are deliberately LITERAL (no `$`) so
// `handle_catch` walks the body rather than bailing on a dynamic token.

#[test]
fn catch_if_branch_upvar_escapes() {
    // tclsh: `proc g {} { catch {if {1} {set y 7}}; return $y }; g` -> 7.
    let r = cfg_result("catch {if {1} {upvar 1 cv v}}");
    assert!(
        is_frame(&r, "v"),
        "upvar in catch'd if escapes: {:?}",
        r.name_tags
    );
    assert!(r.upvar_source_names.contains("cv"));
}

#[test]
fn catch_if_else_global_and_variable_escape() {
    // Both clauses of the catch'd `if` are walked.
    let r = cfg_result("catch {if {1} {global g} else {variable w}}");
    assert!(
        is_frame(&r, "g"),
        "then-clause global escapes: {:?}",
        r.name_tags
    );
    assert!(
        is_frame(&r, "w"),
        "else-clause variable escapes: {:?}",
        r.name_tags
    );
}

#[test]
fn catch_for_body_variable_escapes() {
    // tclsh: `proc h {} { catch {for {set i 0} {$i<3} {incr i} {set z $i}}; return $z }; h` -> 2.
    let r = cfg_result("catch {for {set i 0} {1} {incr i} {variable counter}}");
    assert!(
        is_frame(&r, "counter"),
        "variable in catch'd for escapes: {:?}",
        r.name_tags
    );
}

#[test]
fn catch_while_body_global_escapes() {
    let r = cfg_result("catch {while {1} {global g}}");
    assert!(
        is_frame(&r, "g"),
        "global in catch'd while escapes: {:?}",
        r.name_tags
    );
}

#[test]
fn catch_foreach_body_upvar_escapes() {
    // tclsh: `proc k {} { catch {foreach e {a b} {set w $e}}; return $w }; k` -> b.
    let r = cfg_result("catch {foreach k {a b} {upvar 1 cv v}}");
    assert!(
        is_frame(&r, "v"),
        "upvar in catch'd foreach escapes: {:?}",
        r.name_tags
    );
    assert!(r.upvar_source_names.contains("cv"));
}

#[test]
fn catch_switch_arm_upvar_escapes() {
    // tclsh: `set cv 0; proc s {} { catch {switch x {x {upvar 1 cv v; set v 13}}} }; s`
    // leaves cv == 13.
    let r = cfg_result("catch {switch x {x {upvar 1 cv v}}}");
    assert!(
        is_frame(&r, "v"),
        "upvar in catch'd switch arm escapes: {:?}",
        r.name_tags
    );
    assert!(r.upvar_source_names.contains("cv"));
}

#[test]
fn catch_nested_catch_upvar_escapes() {
    // A `catch` nested inside the catch body is itself walked.
    let r = cfg_result("catch {catch {upvar 1 cv v}}");
    assert!(
        is_frame(&r, "v"),
        "upvar in nested catch escapes: {:?}",
        r.name_tags
    );
    assert!(r.upvar_source_names.contains("cv"));
}

#[test]
fn catch_try_body_and_finally_escape() {
    // tclsh: `proc f {} { catch {try {set x 1} finally {set y 2}}; return $x-$y }; f` -> 1-2.
    let r = cfg_result("catch {try {upvar 1 cv v} finally {global g}}");
    assert!(
        is_frame(&r, "v"),
        "upvar in catch'd try body escapes: {:?}",
        r.name_tags
    );
    assert!(
        is_frame(&r, "g"),
        "global in catch'd try finally escapes: {:?}",
        r.name_tags
    );
}

// PART 4 — top-level barrier / call arms reached *without* a catch wrapper.

#[test]
fn eval_dynamic_body_is_pessimistic() {
    // A non-literal eval body (`eval $cmd`) survives CFG inlining as an opaque
    // barrier; `handle_eval` cannot walk it, so the script is pessimistic.
    let r = cfg_result("eval $cmd");
    assert!(r.dynamic_barrier(), "dynamic eval body is pessimistic");
}

#[test]
fn uplevel_one_literal_body_is_pessimistic() {
    // tclsh: `uplevel 1 {set y 99}` writes the caller's frame (99 read back
    // there). The CFG walker is conservatively pessimistic about every
    // non-`#0`/`0` literal level.
    let r = cfg_result("uplevel 1 {set y 1}");
    assert!(r.dynamic_barrier(), "uplevel 1 is a dynamic barrier");
}

#[test]
fn uplevel_global_zero_literal_body_is_not_pessimistic() {
    // tclsh: `uplevel #0 {set glob 1}` runs at global scope; proc-locals are not
    // visible there, so it is NOT a whole-proc barrier — but it still records an
    // interpreter-dispatch fallback.
    let r = cfg_result("uplevel #0 {set g 1}");
    assert!(
        !r.dynamic_barrier(),
        "uplevel #0 literal body is not pessimistic"
    );
    assert!(
        r.has_fallback(),
        "uplevel still records an interpreter-dispatch fallback"
    );
}

#[test]
fn expand_word_unknown_call_is_pessimistic() {
    // `{*}$args` on a non-list/concat call defeats argument reasoning.
    let r = cfg_result("someproc {*}$args");
    assert!(
        r.dynamic_barrier(),
        "expand-word on unknown call is pessimistic"
    );
}

#[test]
fn expand_word_list_is_not_pessimistic() {
    // `list`/`concat` are carved out of the expand-word barrier.
    let r = cfg_result("list {*}$args");
    assert!(
        !r.dynamic_barrier(),
        "expand-word on `list` is not pessimistic"
    );
}

#[test]
fn value_scan_unknown_cmd_subst_records_fallback() {
    // An embedded `[unknownproc …]` substitution head is not frameless.
    let r = cfg_result("set r [unknownproc x]");
    assert!(
        r.has_fallback(),
        "non-frameless cmd-subst head records a fallback"
    );
}

#[test]
fn value_scan_frameless_cmd_subst_no_fallback() {
    // `[string length …]` is a frameless runtime command — no fallback.
    let r = cfg_result("set r [string length $x]");
    assert!(
        !r.has_fallback(),
        "frameless cmd-subst head records no fallback"
    );
}
