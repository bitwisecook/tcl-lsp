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

//! Flow-sensitive var-escape coverage: the CFG-propagation walker and the
//! IR walker's control-flow traversal.
//!
//! This file deepens coverage of the three lowest-covered `var_escape` modules —
//! `var_escape/cfg_propagation/walker.rs`, `var_escape/walker.rs`, and
//! `var_escape/api.rs` — by driving escape analysis over snippets whose escape
//! behaviour *depends on control flow*: a write that escapes only on one branch
//! of an `if`, an escape via a scope-crossing command inside a loop / `switch`
//! arm / `catch`, escape facts threaded through the per-block SSA-version
//! propagation, and the interprocedural fixpoint folded over the CFG path.
//!
//! It is deliberately disjoint from `var_escape_typeinfer.rs`, which
//! covers the flat-statement IR-walk verdicts, the `solve_interprocedural_escape`
//! fixpoint on synthetic summaries, the info-subcommand classifier sets, slot
//! resolution, and the `ProcEscapeSummary`/types unit surface. Nothing here
//! re-asserts those; every test below exercises a control-flow / per-SSA-version
//! / CU-interprocedural branch the other port skips.
//!
//! ## C-Tcl proof split
//!
//! The escape *lattice* (`Local`/`Frame`), the per-`(name, version)` SSA tag
//! map, the `dynamic_barrier` flag, and the block-by-block propagation are all
//! compiler-internal structure with no direct Tcl analogue, so those are
//! asserted structurally (this note discharges the structural-assertion
//! requirement for the whole file).
//!
//! Where a test rests on a *scoping fact that is observable in another frame* —
//! a write through an `upvar`/`global`/`uplevel`/`namespace`-linked name that a
//! second frame can read back — that observable behaviour is confirmed against
//! real `tclsh` (8.6 and 9.0 agree on every case here) with
//! `scripts/dev/tclsh_check.sh` and cited inline with a `// tclsh:` comment. The
//! pattern is always: write via the escaping name, read it from the other
//! frame, `puts`/`return` the result. The `Frame` verdict is exactly the
//! compiler's encoding of "this write is observable elsewhere, so the name must
//! live in the runtime frame".

use std::collections::HashMap;

use tcl_compiler::cfg_builder::build_cfg_function;
use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::lowering::lower_to_ir;
use tcl_compiler::ssa::{Version, build_ssa};
use tcl_compiler::var_escape::cfg_propagation::state::CfgEscapeResult;
use tcl_compiler::var_escape::{
    EscapeTag, ProcEscapeSummary, TOP_LEVEL_QNAME, analyse_cfg_function, analyse_var_escape,
    analyse_var_escape_cu, cfg_result_to_summary,
};
use tcl_registry::CommandRegistry;
use tcl_registry::model::ingress::static_context_for;

// Shared helpers

const TCL: &str = "tcl8.6";

fn registry() -> &'static CommandRegistry {
    static_context_for(TCL).commands()
}

/// Flow-sensitive CFG/SSA escape summaries (codegen's frame-analysis path),
/// with the interprocedural fixpoint folded in.
fn escape_cu(src: &str) -> HashMap<String, ProcEscapeSummary> {
    let cu = CompilationUnit::build_for(src, registry(), false);
    analyse_var_escape_cu(&cu, true)
}

/// CFG/SSA escape summaries with the interprocedural pass *off* — the raw
/// per-proc flow-sensitive result.
fn escape_cu_raw(src: &str) -> HashMap<String, ProcEscapeSummary> {
    let cu = CompilationUnit::build_for(src, registry(), false);
    analyse_var_escape_cu(&cu, false)
}

/// IR-walk escape summaries (the inliner's path), interprocedural on.
fn escape_ir(src: &str) -> HashMap<String, ProcEscapeSummary> {
    let module = lower_to_ir(src, registry());
    analyse_var_escape(&module, true)
}

/// Drive the low-level [`analyse_cfg_function`] entry point directly on a
/// single top-level script and return the raw [`CfgEscapeResult`] — the
/// per-`(name, version)` tag map before it is collapsed into a
/// `ProcEscapeSummary`.
fn cfg_result(src: &str) -> CfgEscapeResult {
    let m = lower_to_ir(src, registry());
    let cfg = build_cfg_function("::top", &m.top_level, true, registry(), false);
    let ssa = build_ssa(&cfg, registry());
    analyse_cfg_function(&cfg, &ssa, std::iter::empty::<String>())
}

/// Fetch the summary for a proc qname, panicking with the available keys
/// if it is missing.
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

// PART 1 — escape reached conditionally (one branch of an `if`)
//
// The CFG flattens the `if` into separate blocks; the block holding the
// scope-crossing command is still walked, so the alias escapes even though it
// is only reachable on one path. This drives the `analyse_cfg_function`
// block_order + walk_block + handle_statement Call path through a branch.

#[test]
fn cu_upvar_on_one_if_branch_still_escapes() {
    // tclsh: a conditional `upvar` write is observable in the caller's frame:
    //   proc setit {flag} { if {$flag} { upvar 1 caller_v v; set v 42 } }
    //   set caller_v 0; setit 1; set caller_v   -> 42
    // So the alias `v` and the named source `caller_v` must be Frame even
    // though the write only happens on the true branch.
    let s = escape_cu(
        "proc ::p {flag} { if {$flag} { upvar 1 caller_v v\n set v 1 } else { set w 2 } }",
    );
    let p = summary(&s, "::p");
    assert_eq!(
        p.tag("v"),
        EscapeTag::Frame,
        "conditional upvar alias is Frame"
    );
    assert!(p.is_frame("caller_v"), "named upvar source is Frame");
    assert!(
        p.upvar_source_names.contains("caller_v"),
        "caller source recorded across the branch: {:?}",
        p.upvar_source_names
    );
    // The else-branch local never crosses a frame — it stays Local.
    assert_eq!(
        p.tag("w"),
        EscapeTag::Local,
        "else-branch local stays Local"
    );
    assert!(p.frame_needed);
    assert!(
        !p.dynamic_barrier(),
        "a conditional upvar is not a whole-proc barrier"
    );
    // The flow-sensitive path tags a concrete SSA version of the alias.
    assert!(
        ssa_frame(p, "v", 1),
        "v#1 tagged Frame on the CFG path: {:?}",
        p.ssa_tags
    );
}

#[test]
fn cu_namespace_upvar_on_one_if_branch_escapes_alias() {
    // tclsh: `namespace upvar` makes the alias read/write the namespace var:
    //   namespace eval ::ns {variable counter 5}
    //   proc readit {} { namespace upvar ::ns counter c; return $c } -> 5
    // Reached conditionally, `c` must still be Frame.
    let s =
        escape_cu("proc ::p {flag} { if {$flag} { namespace upvar ::ns counter c\n set c 1 } }");
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("c"),
        "conditional namespace-upvar alias is Frame"
    );
    assert!(p.frame_needed);
    assert!(!p.dynamic_barrier());
}

#[test]
fn cu_pure_conditional_does_not_escape() {
    // A conditional with only pure local assignments escapes nothing — the
    // negative control for the branch-walk machinery.
    let s = escape_cu("proc ::p {flag} { if {$flag} { set a 1 } else { set b 2 }\n set c 3 }");
    let p = summary(&s, "::p");
    assert_eq!(p.tag("a"), EscapeTag::Local);
    assert_eq!(p.tag("b"), EscapeTag::Local);
    assert_eq!(p.tag("c"), EscapeTag::Local);
    assert!(!p.frame_needed, "pure conditional proc is frame-elidable");
    assert!(!p.dynamic_barrier());
}

// PART 2 — escape via a command inside a loop body

#[test]
fn cu_global_inside_while_loop_escapes() {
    // tclsh: a `global` write inside a loop accumulates across calls and is
    // observable at the global frame:
    //   set g 0
    //   proc bump {} { set i 0; while {$i<3} { global g; incr g; incr i } }
    //   bump; set g   -> 3
    let s = escape_cu("proc ::p {} { set i 0\n while {$i < 10} { global g\n incr g\n incr i } }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("g"), "global-linked var inside a loop is Frame");
    assert_eq!(p.tag("i"), EscapeTag::Local, "the loop counter stays Local");
    assert!(p.frame_needed);
    assert!(
        ssa_frame(p, "g", 1),
        "g#1 tagged on the CFG path: {:?}",
        p.ssa_tags
    );
}

#[test]
fn cu_upvar_inside_foreach_escapes() {
    // tclsh: an `upvar` write inside `foreach` reaches the caller:
    //   proc f {} { foreach n {a b} { upvar 1 caller_x x; set x $n } }
    //   set caller_x z; f; set caller_x   -> b
    let s = escape_cu("proc ::p {} { foreach n {a b} { upvar 1 caller_x x\n set x 1 } }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("x"), "upvar alias inside foreach is Frame");
    assert!(p.upvar_source_names.contains("caller_x"));
    assert!(p.frame_needed);
}

#[test]
fn cu_global_inside_for_loop_body_escapes() {
    // A `for` loop whose body declares a global — the body block is walked.
    // tclsh: the accumulated global persists past the call.
    let s = escape_cu("proc ::p {} { for {set i 0} {$i < 3} {incr i} { global acc\n incr acc } }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("acc"), "global inside a for-body is Frame");
    assert_eq!(p.tag("i"), EscapeTag::Local);
    assert!(p.frame_needed);
}

// PART 3 — escape via a command inside a `switch` arm / `catch` body
//
// `switch` arms are flattened into CFG blocks and walked. `catch` bodies are
// NOT walked on the CU path (the CFG builder collapses the body to a string
// arg) — that gap is a documented BUG below; the working IR-path catch test
// lives next to the BUG note.

#[test]
fn cu_variable_inside_switch_arm_escapes() {
    // tclsh: a `variable` write inside one switch arm is observable on the
    // namespace var:
    //   namespace eval ::ns {variable counter 0}
    //   proc ::ns::touch {mode} { switch $mode { a {variable counter; set counter 99} b {} } }
    //   ::ns::touch a; set ::ns::counter   -> 99
    let s = escape_cu(
        "proc ::p {mode} { switch $mode { a { variable counter\n set counter 1 } \
         b { set local 2 } } }",
    );
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("counter"),
        "variable inside a switch arm is Frame"
    );
    assert_eq!(
        p.tag("local"),
        EscapeTag::Local,
        "the other arm's local stays Local"
    );
    assert!(p.frame_needed);
    assert!(!p.dynamic_barrier());
}

#[test]
fn cu_global_in_one_switch_arm_only() {
    // Three-arm switch where exactly one arm crosses a frame (global). The two
    // pure arms contribute Local names; only the global escapes.
    let s = escape_cu(
        "proc ::p {m} { switch -- $m { a { set la 1 } b { global gb\n set gb 2 } \
         default { set ld 3 } } }",
    );
    let p = summary(&s, "::p");
    assert!(p.is_frame("gb"), "the global-declaring arm escapes gb");
    assert_eq!(p.tag("la"), EscapeTag::Local);
    assert_eq!(p.tag("ld"), EscapeTag::Local);
}

// FIXED (was a bug): the CFG/CU path did not descend into a `catch {body}`
// body, so a scope-crossing `upvar` inside `catch` was invisible to the
// flow-sensitive walker. The CFG builder lowers `catch {upvar 1 cv v; set v 1}` to a plain
// `Statement::Call { command: "catch", args: [" upvar 1 cv v ... " ] }` — the
// body collapses to a brace-literal string argument, so `handle_call` only
// records a (call-)fallback and never sees the inner `upvar`. The IR walk, by
// contrast, lowers it to `Statement::Catch { body }` and walks it correctly
// (see `ir_upvar_inside_catch_body_escapes`, which passes).
//
// Why this is misleading, not merely conservative: the dropped fact is the
// *upvar source name*, which the interprocedural pass needs to spill the
// caller's matching local. With a caller present, the CU path marks the
// caller's local Local with `wants_frame() == false`:
//
//   proc ::leaf {} { catch { upvar 1 cv y; set y 9 } }
//   proc ::host {} { set cv 1; leaf }
//   -> CU: host.is_frame("cv") == false, host.wants_frame() == false,
//          host.upvar_source_names == {}        (WRONG)
//   -> IR: host.is_frame("cv") == true,  host.upvar_source_names == {"cv"}
//
// tclsh proves the write IS observable in the caller's frame (8.6 + 9.0):
//   proc leaf {} { catch { upvar 1 cv y; set y 9 } }
//   proc host {} { set cv 1; leaf; return $cv }; host   -> 9
// So treating host's `cv` as a private frame-elided slot would drop a write
// that real Tcl makes visible. FIXED: the CU walker's `handle_call` now
// descends into a literal `catch` body (mirroring its `eval` handling), so the
// inner `upvar` is found and the CU path agrees with the IR path below.
#[test]
fn cu_upvar_inside_catch_body_escapes() {
    let s = escape_cu("proc ::p {} { catch { upvar 1 cv v\n set v 1 } }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("v"), "upvar alias inside catch is Frame");
    assert!(p.upvar_source_names.contains("cv"));
    assert!(p.frame_needed);
}

#[test]
fn ir_upvar_inside_catch_body_escapes() {
    // tclsh: an `upvar` write inside a `catch` body still reaches the caller
    // (catch only intercepts the result/error, not the frame aliasing):
    //   proc f {} { catch { upvar 1 cv v; set v 7 } }
    //   set cv 0; f; set cv   -> 7
    // The IR walk's `Statement::Catch` arm descends the body and finds the
    // upvar. (The CU/CFG path does NOT — see the BUG note above.)
    let s = escape_ir("proc ::p {} { catch { upvar 1 cv v\n set v 1 } }");
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("v"),
        "upvar alias inside catch is Frame on the IR walk"
    );
    assert!(p.upvar_source_names.contains("cv"));
    assert!(p.frame_needed);
}

// PART 4 — escape through the terminator branch *condition*
//
// `walk_block` evaluates the terminator's branch condition so that an
// `[info exists ...]` inside an `if` condition is not missed. This is the
// `apply_expr_scan(Some(cond), …)` path in `walk_block`.

#[test]
fn cu_info_exists_in_if_condition_escapes_target() {
    // tclsh: `info exists` resolves its target against the runtime frame:
    //   proc f {} { set myvar 1; if {[info exists myvar]} {return yes} else {return no} }
    //   f   -> yes     (and -> no for an unset local)
    // So `myvar`, named by `info exists` in the branch condition, must be
    // Frame — the verdict comes from the terminator-condition scan, not a
    // statement body.
    let s = escape_cu("proc ::p {} { set myvar 1\n if {[info exists myvar]} { return 1 } }");
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("myvar"),
        "info-exists target in the branch condition is Frame"
    );
    assert!(
        !p.dynamic_barrier(),
        "a literal info-exists in a condition is not a whole-proc barrier"
    );
    assert!(
        ssa_frame(p, "myvar", 1),
        "myvar#1 tagged from the terminator condition: {:?}",
        p.ssa_tags
    );
}

// PART 5 — per-SSA-version flow sensitivity
//
// The CFG path is per-`(name, version)`: it tags exactly the SSA version live
// where the scope-crossing command runs. These tests pin which version is
// `Frame`, which is the whole point of the flow-sensitive walker over the
// flat IR walk.

#[test]
fn cu_upvar_first_tags_initial_version() {
    // tclsh: writes *through* an alias all reach the caller:
    //   proc f {} { upvar 1 cy y; set y 1; set y 2; set y 3 }
    //   set cy init; f; set cy   -> 3
    // The upvar binds `y` before any write, so the analysis tags y's *first*
    // SSA version (1); the later store-versions are the same Frame name.
    let s = escape_cu("proc ::p {} { upvar 1 cy y\n set y 1\n set y 2\n set y 3 }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("y"));
    assert!(
        ssa_frame(p, "y", 1),
        "the upvar tags y's initial version: {:?}",
        p.ssa_tags
    );
    assert!(p.upvar_source_names.contains("cy"));
}

#[test]
fn cu_upvar_after_writes_tags_latest_version() {
    // The mirror of the above: three writes then the upvar. The upvar's def is
    // the highest SSA version, so *that* version is tagged Frame. (This exact
    // script errors at runtime — `upvar` rejects an already-existing local —
    // so the assertion is purely structural: it pins which version the
    // flow-sensitive walker tags, not a runtime value.)
    let s = escape_cu("proc ::p {} { set y 1\n set y 2\n set y 3\n upvar 1 cy y }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("y"));
    assert!(
        ssa_frame(p, "y", 4),
        "the trailing upvar tags the latest version (4): {:?}",
        p.ssa_tags
    );
    // Earlier pure-store versions were not individually tagged Frame.
    assert!(!ssa_frame(p, "y", 1), "store version 1 was not tagged");
}

#[test]
fn cu_conditional_global_tags_def_version() {
    // tclsh: a conditional `incr` on a global persists:
    //   set g 10; proc f {flag} { global g; if {$flag} { incr g } }
    //   f 1; set g   -> 11
    // `global g` introduces version 1 of g; it is the tagged version.
    let s = escape_cu("proc ::p {flag} { global g\n if {$flag} { incr g } }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("g"));
    assert!(ssa_frame(p, "g", 1), "g#1 tagged: {:?}", p.ssa_tags);
}

// PART 6 — dynamic-name spill on the CFG path
//
// `set $n value` with an unresolved `$n` could write any local, so the CFG
// walker's `dynamic_name_escape` → `escape_all_known` spills every known
// proc-local at its current version, and marks the whole proc pessimistic.

#[test]
fn cu_dynamic_set_name_spills_all_known_locals() {
    // tclsh: `set $n 99` writes whichever local `$n` names — so an optimiser
    // must treat *every* local as frame-resident:
    //   proc f {n} { set a 1; set b 2; set $n 99; return "$a $b" }
    //   f a -> "99 2" ;  f b -> "1 99"
    let s = escape_cu("proc ::p {n} { set a 1\n set b 2\n set $n 99 }");
    let p = summary(&s, "::p");
    assert!(
        p.dynamic_barrier(),
        "an unresolved dynamic set-name is pessimistic"
    );
    assert!(p.is_frame("a"), "a spilled by the dynamic set-name");
    assert!(p.is_frame("b"), "b spilled by the dynamic set-name");
    assert!(
        ssa_frame(p, "a", 1),
        "a spilled at its current version: {:?}",
        p.ssa_tags
    );
    assert!(ssa_frame(p, "b", 1));
}

#[test]
fn cu_dynamic_set_name_resolved_to_literal_records_that_name_under_the_guard() {
    // tclsh: when the name var holds a known literal, only that target is
    // written:
    //   proc f {} { set target real; set $target 77; return $real }  -> 77
    // The CFG walker's single-writer literal tracking still resolves
    // `$target` to `real`. The registry-owned source projection cannot use
    // that later flow fact, so it also retains the conservative dynamic-name
    // guard needed by consumers which do not run this lattice.
    let s = escape_cu("proc ::p {} { set target real\n set $target 99 }");
    let p = summary(&s, "::p");
    assert!(p.dynamic_barrier(), "the source name remains opaque: {p:?}");
    assert!(p.is_frame("real"), "the resolved literal target escapes");
    assert!(ssa_frame(p, "real", 0), "real#0 escaped: {:?}", p.ssa_tags);
}

// PART 7 — whole-proc pessimism through nested control flow (CFG path)

#[test]
fn cu_info_level_inside_loop_is_a_barrier() {
    // `info level` reads the caller frame; reached inside a loop it still
    // forces the pessimistic whole-proc path.
    let s = escape_cu("proc ::p {} { set i 0\n while {$i < 3} { set l [info level]\n incr i } }");
    let p = summary(&s, "::p");
    assert!(p.dynamic_barrier(), "info level in a loop is a barrier");
    // A barrier forces every name to Frame.
    assert!(p.is_frame("i"));
    assert!(p.is_frame("any_name"));
}

#[test]
fn cu_uplevel_one_inside_conditional_is_a_barrier() {
    // tclsh: `uplevel 1` runs its body in the caller's frame and writes the
    // caller's vars:
    //   proc setc {} { uplevel 1 {set y 99} }
    //   proc outer {} { setc; return $y }; outer   -> 99
    // The analysis can't bound which caller names the body touches, so even a
    // conditional `uplevel 1` makes the whole proc pessimistic.
    let s = escape_cu("proc ::p {flag} { if {$flag} { uplevel 1 {set y 99} } }");
    let p = summary(&s, "::p");
    assert!(p.dynamic_barrier(), "uplevel 1 marks a dynamic barrier");
    assert!(p.frame_needed);
}

#[test]
fn cu_uplevel_global_zero_literal_body_is_not_a_barrier() {
    // tclsh: `uplevel #0` runs at global scope; our proc-locals are not
    // visible there, so a literal-body `uplevel #0` is NOT a whole-proc
    // barrier (it only needs the eval fallback):
    //   proc f {} { uplevel #0 {set glob 88} }; f; set glob   -> 88
    let s = escape_cu("proc ::p {} { uplevel #0 {set glob 1} }");
    let p = summary(&s, "::p");
    assert!(
        !p.dynamic_barrier(),
        "uplevel #0 with a literal body is not a whole-proc barrier"
    );
    assert!(p.has_fallback(), "but it does need the eval fallback");
}

#[test]
fn cu_expand_word_in_unknown_call_is_a_barrier() {
    // `{*}$args` in an unknown command defeats argument-index analysis (we
    // can't tell where a name argument landed), so the proc goes pessimistic.
    let s = escape_cu("proc ::p {args} { set x 1\n some_cmd {*}$args }");
    let p = summary(&s, "::p");
    assert!(
        p.dynamic_barrier(),
        "{{*}}-expansion in an unknown call is pessimistic"
    );
    assert!(p.is_frame("x"), "the barrier forces every local to Frame");
}

// PART 8 — interprocedural propagation folded over the CFG path
//
// `analyse_var_escape_cu(cu, true)` runs `solve_interprocedural_escape` after
// the per-function CFG walk. These tests verify that callee-induced pessimism
// and named upvar sources flow up call edges on the CU path, and that turning
// the fixpoint off leaves the raw per-proc result.

#[test]
fn cu_caller_inherits_unbounded_upvar_pessimism() {
    // `::setit` upvars a *dynamic* source (`$n`) — an unbounded upvar source —
    // so it is pessimistic. `::wrap` calls it; on the CU+IPA path the
    // pessimism propagates up the call edge.
    let s = escape_cu(
        "proc ::setit {n v} { upvar 1 $n a\n set a $v }\n\
         proc ::wrap {} { set x 5\n setit x 1 }",
    );
    let setit = summary(&s, "::setit");
    assert!(
        setit.unbounded_upvar_source(),
        "dynamic upvar source is unbounded"
    );
    assert!(setit.dynamic_barrier());
    let wrap = summary(&s, "::wrap");
    assert!(
        wrap.dynamic_barrier(),
        "the caller of an unbounded-upvar proc is downgraded on the CU path"
    );
}

#[test]
fn cu_named_upvar_source_flows_to_caller() {
    // tclsh: `::leaf` upvars the literal caller-frame name `x`; the caller
    // `::host` has its own local `x`, which is the one written:
    //   proc leaf {} { upvar 1 x y; set y 9 }
    //   proc host {} { set x 1; leaf; return $x }; host   -> 9
    // On the CU+IPA path the named source `x` must reach `::host` and make its
    // matching local Frame.
    let s = escape_cu(
        "proc ::leaf {} { upvar 1 x y\n set y 9 }\n\
         proc ::host {} { set x 1\n leaf }",
    );
    let leaf = summary(&s, "::leaf");
    assert!(leaf.upvar_source_names.contains("x"));
    let host = summary(&s, "::host");
    assert!(
        host.upvar_source_names.contains("x"),
        "the named upvar source reaches the caller on the CU path: {:?}",
        host.upvar_source_names
    );
    assert!(
        host.is_frame("x"),
        "the caller's matching local becomes Frame"
    );
}

#[test]
fn cu_interprocedural_off_keeps_raw_per_proc_result() {
    // With the CU fixpoint off, the per-function flow-sensitive pass still
    // runs, but a caller is NOT downgraded by its callee. `::wrap` records the
    // call (HAS_CALL_FALLBACK) without inheriting `::setit`'s pessimism.
    let s = escape_cu_raw(
        "proc ::setit {n v} { upvar 1 $n a\n set a $v }\n\
         proc ::wrap {} { set x 5\n setit x 1 }",
    );
    assert!(s.contains_key(TOP_LEVEL_QNAME));
    let wrap = summary(&s, "::wrap");
    assert!(
        !wrap.dynamic_barrier(),
        "without the CU fixpoint the callee's pessimism has not propagated"
    );
    assert!(wrap.has_call_fallback(), "but the call itself was recorded");
    // `::setit` itself still computes its own intraprocedural pessimism.
    assert!(summary(&s, "::setit").unbounded_upvar_source());
}

// PART 9 — nested proc / namespace-eval bodies become their own CU functions
//
// A `proc` nested in a proc, and a `proc` inside `namespace eval`, are lifted
// to separate qualified summaries. Their escape verdicts are independent of
// the enclosing scope, which the CU path keys by qname.

#[test]
fn cu_nested_proc_gets_its_own_escape_summary() {
    // tclsh: the inner proc's upvar reaches *its* caller's frame independently
    // of the outer proc.
    let s = escape_cu("proc ::outer {} { proc ::inner {} { upvar 1 cv v\n set v 1 }\n set x 1 }");
    // The inner proc is a separate summary and is the one that escapes.
    let inner = summary(&s, "::inner");
    assert!(
        inner.is_frame("v"),
        "the nested proc's upvar alias is Frame"
    );
    assert!(inner.upvar_source_names.contains("cv"));
    assert!(inner.frame_needed);
    // The outer proc's own local `x` does not escape.
    let outer = summary(&s, "::outer");
    assert_eq!(
        outer.tag("x"),
        EscapeTag::Local,
        "the outer proc's local stays Local"
    );
    assert!(!outer.frame_needed);
}

#[test]
fn cu_namespace_eval_proc_is_lifted_and_escapes_variable() {
    // tclsh: a namespace `variable` shared between calls is observable on the
    // namespace var:
    //   namespace eval ::ns {variable shared 0; proc bump {} {variable shared; incr shared}}
    //   ::ns::bump; ::ns::bump; set ::ns::shared   -> 2
    // The proc is lifted to `::ns::bump`; its `variable`-linked `shared` is
    // Frame.
    let s = escape_cu(
        "namespace eval ::ns { variable shared 0\n \
         proc bump {} { variable shared\n incr shared } }",
    );
    let bump = summary(&s, "::ns::bump");
    assert!(
        bump.is_frame("shared"),
        "the lifted proc's namespace var is Frame"
    );
    assert!(bump.frame_needed);
    assert!(
        ssa_frame(bump, "shared", 1),
        "shared#1 tagged: {:?}",
        bump.ssa_tags
    );
}

#[test]
fn cu_apply_body_with_upvar_is_pessimistic() {
    // An `apply {{x} {...}}` literal lambda body is dispatched through the
    // interpreter from the enclosing proc's perspective — the CFG walker
    // can't statically thread its frame effects, so the enclosing proc goes
    // pessimistic (frame is needed).
    let s = escape_cu("proc ::p {} { apply {{x} { upvar 1 cv v\n set v $x }} 5 }");
    let p = summary(&s, "::p");
    assert!(
        p.frame_needed,
        "an apply-with-upvar enclosing proc needs a frame"
    );
    assert!(p.dynamic_barrier());
}

// PART 10 — IR walker control-flow: literal `eval` body traversal
//
// `var_escape/walker.rs`'s `handle_eval` + `escape_every_name_touched` walk a
// literal `eval {...}` body and escape every name it writes/reads/declares,
// recursing through nested control flow. This is the IR-walk path (the CU path
// flattens eval differently), so these drive `analyse_var_escape` /
// `analyse_script`.

#[test]
fn ir_eval_literal_body_escapes_names_it_writes() {
    // tclsh: a `set` inside a literal `eval` body runs in the proc frame and
    // persists there:
    //   proc f {} { eval {set fresh 33}; return $fresh }; f   -> 33
    // The walker escapes every name the literal body touches.
    let s = escape_ir("proc ::p {} { set x 1\n eval {set x 2\n set fresh 3} }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("x"), "a name the eval body writes is Frame");
    assert!(
        p.is_frame("fresh"),
        "a fresh name introduced in the eval body is Frame"
    );
    assert!(p.has_fallback(), "an eval is a barrier-shaped fallback");
    assert!(
        !p.dynamic_barrier(),
        "a literal eval body is not pessimistic"
    );
}

#[test]
fn ir_eval_literal_body_with_upvar_records_source() {
    // tclsh: an `upvar` inside a literal `eval` body still reaches the caller:
    //   proc f {} { eval {upvar 1 cv v; set v 55} }
    //   set cv 0; f; set cv   -> 55
    let s = escape_ir("proc ::p {} { eval {upvar 1 cv v\n set v 1} }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("v"), "the eval-body upvar alias is Frame");
    assert!(p.is_frame("cv"), "the eval-body upvar source is Frame");
    assert!(
        p.upvar_source_names.contains("cv"),
        "the eval-body upvar source is recorded: {:?}",
        p.upvar_source_names
    );
}

#[test]
fn ir_eval_literal_body_recurses_into_nested_if() {
    // The eval-body walker descends nested control flow: both branches of an
    // `if` inside the literal body have their names escaped.
    let s = escape_ir("proc ::p {} { eval {if {1} {set a 1} else {set b 2}} }");
    let p = summary(&s, "::p");
    assert!(
        p.is_frame("a"),
        "the true-branch name inside the eval body is Frame"
    );
    assert!(
        p.is_frame("b"),
        "the else-branch name inside the eval body is Frame"
    );
}

#[test]
fn ir_eval_dynamic_body_is_pessimistic() {
    // A `$var` reference makes the whole eval body dynamic — the walker can't
    // scan it, so it records an Eval barrier and goes pessimistic.
    use tcl_compiler::var_escape::BarrierKind;
    let s = escape_ir("proc ::p {} { set q 1\n eval {puts $q} }");
    let p = summary(&s, "::p");
    assert!(
        p.dynamic_barrier(),
        "a $-referencing eval body is pessimistic"
    );
    assert!(
        p.barriers.iter().any(|b| b.kind == BarrierKind::Eval),
        "an Eval barrier is recorded: {:?}",
        p.barriers
    );
}

#[test]
fn ir_eval_multiword_body_escapes_through_the_joined_script() {
    // Pin (#1051): `handle_eval` joins a multi-word `eval` with single spaces
    // before scanning, exactly as `Tcl_ConcatObj` does at run time, so a `$x`
    // buried in a trailing word still escapes.
    //
    // tclsh8.6.14 / tclsh9.0.4: `set x l2; eval set $x hello; puts $l2`
    // → `hello` — the substituted word really is part of the script.
    use tcl_compiler::var_escape::BarrierKind;
    let s = escape_ir("proc ::p {} { set x 1\n eval set l2 $x }");
    let p = summary(&s, "::p");
    assert!(
        p.dynamic_barrier(),
        "a $-referencing word anywhere in the joined body is pessimistic"
    );
    assert!(
        p.barriers.iter().any(|b| b.kind == BarrierKind::Eval),
        "an Eval barrier is recorded for the joined body: {:?}",
        p.barriers
    );
    assert!(p.is_frame("x"), "the referenced name escapes: {p:?}");
}

#[test]
fn ir_uplevel_global_zero_literal_body_escapes_touched_names() {
    // tclsh: `uplevel #0 {set glob 1}` writes a global, observable past the
    // call. On the IR walk the literal `uplevel #0` body is treated like a
    // safe global-scope eval — it needs the fallback but is not pessimistic.
    let s = escape_ir("proc ::p {} { uplevel #0 {set glob 1} }");
    let p = summary(&s, "::p");
    assert!(
        !p.dynamic_barrier(),
        "uplevel #0 literal body is not pessimistic"
    );
    assert!(p.has_fallback());
}

// PART 11 — IR-walk control-flow descent (the `walk` structural arms)
//
// `var_escape/walker.rs`'s `walk` recurses through every structured statement.
// These confirm a scope-crossing command buried in each control-flow shape is
// found on the IR-walk path too (the inliner's path), complementing the CU
// flow-sensitive tests above.

#[test]
fn ir_upvar_in_if_branch_escapes() {
    // tclsh proof: see `cu_upvar_on_one_if_branch_still_escapes` (same script
    // semantics). Here the IR walk's `If` arm is the one under test.
    let s = escape_ir("proc ::p {flag} { if {$flag} { upvar 1 caller_v v\n set v 1 } }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("v"));
    assert!(p.is_frame("caller_v"));
    assert!(p.upvar_source_names.contains("caller_v"));
}

#[test]
fn ir_variable_in_for_body_escapes() {
    // A `variable` declaration inside a `for` body is found by the IR walk's
    // `For` arm. tclsh: a namespace variable touched in a loop is observable
    // on the namespace var (same mechanism as the switch-arm case).
    let s =
        escape_ir("proc ::p {} { for {set i 0} {$i < 2} {incr i} { variable nsv\n set nsv 1 } }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("nsv"), "variable inside a for-body is Frame");
    assert_eq!(p.tag("i"), EscapeTag::Local);
}

#[test]
fn ir_global_in_switch_arm_escapes() {
    // The IR walk's `Switch` arm finds a `global` in one arm.
    let s = escape_ir("proc ::p {m} { switch $m { x { global gx\n set gx 1 } y { set ly 2 } } }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("gx"));
    assert_eq!(p.tag("ly"), EscapeTag::Local);
}

#[test]
fn ir_upvar_in_try_handler_escapes() {
    // The IR walk descends a `try ... on error {...}` handler body.
    let s = escape_ir("proc ::p {} { try { set ok 1 } on error {e} { upvar 1 cv v\n set v 1 } }");
    let p = summary(&s, "::p");
    assert!(p.is_frame("v"), "upvar inside a try handler is Frame");
    assert!(p.upvar_source_names.contains("cv"));
}

// PART 12 — low-level CfgEscapeResult + cfg_result_to_summary surface
//
// Drive `analyse_cfg_function` directly and inspect the raw
// `CfgEscapeResult` (per-version `ssa_tags`, the `name_tags` collapse), then
// exercise `cfg_result_to_summary` — the `api.rs` adapter that turns it into a
// `ProcEscapeSummary`.

#[test]
fn cfg_result_collapses_versions_to_name_tags() {
    // `upvar 1 cy y; set y 1; set y 2` — y has several SSA versions but the
    // raw result must (a) tag at least one version Frame and (b) collapse to a
    // single `name_tags[y] = Frame`.
    let r = cfg_result("upvar 1 cy y\nset y 1\nset y 2");
    assert_eq!(
        r.name_tags.get("y"),
        Some(&EscapeTag::Frame),
        "name collapse"
    );
    assert!(r.upvar_source_names.contains("cy"));
    assert!(
        r.ssa_tags
            .iter()
            .any(|((n, _), t)| n == "y" && *t == EscapeTag::Frame),
        "at least one y version is tagged Frame: {:?}",
        r.ssa_tags
    );
    assert!(!r.dynamic_barrier());
}

#[test]
fn cfg_result_branch_condition_info_exists_tags_name() {
    // The low-level entry point also runs the terminator-condition scan:
    // `[info exists flag]` in an `if` condition tags `flag`.
    let r = cfg_result("set flag 1\nif {[info exists flag]} { set r 1 }");
    assert_eq!(r.name_tags.get("flag"), Some(&EscapeTag::Frame));
    assert!(!r.dynamic_barrier());
}

#[test]
fn cfg_result_to_summary_preserves_tags_and_clears_pure_leaf() {
    // `cfg_result_to_summary` carries the name tags, flags, upvar sources, and
    // ssa_tags across, sets `frame_needed`, and (by contract) leaves
    // `pure_leaf` false — the inlining predicate is only computed on the IR
    // walk.
    let r = cfg_result("global g");
    assert_eq!(r.name_tags.get("g"), Some(&EscapeTag::Frame));
    let summ = cfg_result_to_summary(&r);
    assert!(summ.is_frame("g"), "tag carried into the summary");
    assert!(summ.frame_needed, "frame_needed derived from a Frame tag");
    assert!(
        !summ.pure_leaf,
        "cfg_result_to_summary leaves pure_leaf at its default"
    );
    // The ssa_tags map is carried verbatim.
    assert_eq!(summ.ssa_tags, r.ssa_tags);
}

#[test]
fn cfg_result_to_summary_propagates_barrier() {
    // A pessimistic result (`info level`) round-trips to a summary that is
    // `dynamic_barrier` and forces every name to Frame.
    let r = cfg_result("info level");
    assert!(r.dynamic_barrier());
    let summ = cfg_result_to_summary(&r);
    assert!(summ.dynamic_barrier());
    assert!(summ.is_frame("whatever"));
    assert!(summ.frame_needed);
}

#[test]
fn cfg_result_pure_script_is_empty() {
    // A pure top-level script taints nothing — the negative control for the
    // whole low-level surface.
    let r = cfg_result("set a 1\nset b 2\nexpr {$a + $b}");
    assert!(r.name_tags.is_empty(), "no name escapes: {:?}", r.name_tags);
    assert!(r.ssa_tags.is_empty());
    assert!(!r.dynamic_barrier());
    let summ = cfg_result_to_summary(&r);
    assert!(!summ.frame_needed);
}

// PART 13 — the top-level key is present on the CU path

#[test]
fn cu_includes_top_level_and_proc_keys() {
    let s = escape_cu("set x 1\nproc ::p {} { upvar 1 cv v\n set v 1 }");
    assert!(s.contains_key(TOP_LEVEL_QNAME), "top-level key present");
    assert!(s.contains_key("::p"), "proc key present");
    // The top-level pure assignment escapes nothing.
    assert_eq!(summary(&s, TOP_LEVEL_QNAME).tag("x"), EscapeTag::Local);
    // The proc escapes its alias.
    assert!(summary(&s, "::p").is_frame("v"));
}
