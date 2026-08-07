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

//! Behaviour-driven coverage for three under-covered `tcl-compiler`
//! modules: `var_escape/` (variable escape analysis), `type_infer`
//! (the SSA `TclType` lattice), and `subst_nocommands` (the compile-time
//! `subst -nocommands` evaluator).
//!
//! There is no single pytest source for these — the tests below are
//! driven from each module's public API and Tcl semantics, mirroring
//! the harness style of `tests/dataflow.rs`,
//! `tests/core_analyses.rs`, `tests/type_propagation.rs`, and
//! `tests/alias_scoping.rs`:
//!
//!   * var-escape IR-walk path → `lower_to_ir(src, registry)` then
//!     `var_escape::analyse_var_escape(&module, ipa)`.
//!   * var-escape CFG/SSA path → `CompilationUnit::build_for(src, …)`
//!     then `var_escape::analyse_var_escape_cu(&cu, ipa)`.
//!   * type-inference → `CompilationUnit::build_for(src, …)` then the
//!     per-function `FunctionUnit::types` map keyed by `(name, version)`.
//!   * subst → the public `subst_nocommands::subst_nocommands(template,
//!     const_map)` function directly.
//!
//! ## C-Tcl proof split
//!
//! Type-inference and `subst` verdicts predict a Tcl-observable value,
//! so each such expectation is pinned to real `tclsh` (8.6 and 9.0 agree
//! on every case here) via `scripts/dev/tclsh_check.sh`, using
//! `string is integer/double/boolean $v` to confirm the inferred
//! `TclType` matches Tcl's actual intrep. Those facts are cited inline
//! with a `// tclsh:` comment. Where the lattice models an abstraction
//! with no direct Tcl analogue (`Numeric` = the Int-or-Double join,
//! `Overdefined` = "unknown") that is noted instead.
//!
//! **Scope note on `subst_nocommands`.** This module is *not* the general
//! `subst` command dispatcher — it is the compile-time evaluator for the
//! `-nocommands` variant only, resolving `$var` against a supplied
//! const-map and leaving `[...]` literal (exactly what `-nocommands`
//! means). There is therefore no `-novariables` / `-nobackslashes`
//! switch surface to exercise; the `-nocommands` semantic (leave `[..]`
//! literal, substitute `$var`, process `\\` escapes) is what is pinned to
//! tclsh. The general `subst` command's other flags live in the runtime,
//! not this helper.
//!
//! Escape analysis is a compiler-internal property (does a variable need
//! to live in the runtime frame), so its verdicts are asserted
//! *structurally* against the `ProcEscapeSummary`. Where an escape mirrors
//! a real Tcl scoping fact — `upvar`/`global`/`uplevel`/`namespace upvar`
//! make a write observable in another frame — that scoping behaviour is
//! confirmed with tclsh and cited next to the structural assertion.

use std::collections::HashMap;

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::lowering::lower_to_ir;
use tcl_compiler::types::{TclType, TypeKind, TypeLattice};
use tcl_compiler::var_escape::types::{
    Barrier, BarrierKind, EscapeFlags, EscapeReason, EscapeReasonKind,
};
use tcl_compiler::var_escape::{
    self, EscapeTag, FRAME_INSPECTING_SUBCOMMANDS, INTERPRETER_GLOBAL_SUBCOMMANDS,
    LOCALS_ARRAY_CAP, ProcEscapeSummary, TOP_LEVEL_QNAME, analyse_var_escape,
    analyse_var_escape_cu, assign_local_slots, is_frame_inspecting_info_subcommand,
    is_safe_info_subcommand, solve_interprocedural_escape,
};
use tcl_registry::{CommandRegistry, registry_for_dialect};

// ---------------------------------------------------------------------------
// Shared helpers
// ---------------------------------------------------------------------------

const TCL: &str = "tcl8.6";

fn registry() -> &'static CommandRegistry {
    registry_for_dialect(TCL)
}

/// IR-walk escape summaries (the inliner's path). `ipa=true` folds the
/// interprocedural fixpoint and `pure_leaf` propagation.
fn escape_ir(src: &str) -> HashMap<String, ProcEscapeSummary> {
    let module = lower_to_ir(src, registry());
    analyse_var_escape(&module, true)
}

/// IR-walk escape summaries with the interprocedural pass *off* — the raw
/// per-proc intraprocedural result.
fn escape_ir_raw(src: &str) -> HashMap<String, ProcEscapeSummary> {
    let module = lower_to_ir(src, registry());
    analyse_var_escape(&module, false)
}

/// CFG/SSA escape summaries (codegen's frame-analysis path).
fn escape_cu(src: &str) -> HashMap<String, ProcEscapeSummary> {
    let cu = CompilationUnit::build_for(src, registry(), false);
    analyse_var_escape_cu(&cu, true)
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

/// The inferred `TclType` of `var` at its highest SSA version in `qname`
/// (`"::top"` for the top-level script), or `None` if never typed.
fn infer_type(src: &str, qname: &str, var: &str) -> Option<TclType> {
    let cu = CompilationUnit::build_for(src, registry(), false);
    let fu = if qname == "::top" {
        &cu.top_level
    } else {
        cu.procedures
            .get(qname)
            .unwrap_or_else(|| panic!("procedure {qname} not found"))
    };
    fu.types
        .iter()
        .filter(|((sym, _), _)| fu.ssa.var_name(*sym) == var)
        .max_by_key(|((_, ver), _)| *ver)
        .and_then(|(_, t)| t.tcl_type())
}

/// The inferred `TypeLattice` (kind + `tcl_type`) of `var`'s highest version
/// in the top-level script.
fn infer_lattice(src: &str, var: &str) -> Option<TypeLattice> {
    let cu = CompilationUnit::build_for(src, registry(), false);
    let fu = &cu.top_level;
    fu.types
        .iter()
        .filter(|((sym, _), _)| fu.ssa.var_name(*sym) == var)
        .max_by_key(|((_, ver), _)| *ver)
        .map(|(_, t)| t.clone())
}

/// Convenience: the top-level inferred type of `var` (panics if untyped).
fn ty(src: &str, var: &str) -> TclType {
    infer_type(src, "::top", var)
        .unwrap_or_else(|| panic!("no type inferred for `{var}` in {src:?}"))
}

/// A const-map for the `subst_nocommands` evaluator.
fn cmap(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

fn subst(template: &str, pairs: &[(&str, &str)]) -> Option<String> {
    var_escape_subst(template, &cmap(pairs))
}

fn var_escape_subst(template: &str, m: &HashMap<String, String>) -> Option<String> {
    tcl_compiler::subst_nocommands::subst_nocommands(template, m)
}

// ===========================================================================
// PART 1 — var_escape: escape via scope-crossing constructs
// ===========================================================================

#[test]
fn upvar_escapes_local_alias_and_records_source() {
    // tclsh: `proc setit {name val} { upvar 1 $name v; set v $val }
    //         set x 0; setit x 42; set x` -> 42 (the write reaches the
    // caller's frame), so the alias `v` and the named source must live in
    // the runtime frame.
    let s = escape_ir("proc ::setit {name val} { upvar 1 caller_v v\n set v $val }");
    let p = summary(&s, "::setit");
    assert_eq!(p.tag("v"), EscapeTag::Frame, "upvar alias must be Frame");
    assert!(p.is_frame("caller_v"), "named upvar source must be Frame");
    assert!(
        p.upvar_source_names.contains("caller_v"),
        "caller-frame source name recorded: {:?}",
        p.upvar_source_names
    );
    assert!(p.frame_needed);
    assert!(!p.pure_leaf, "a proc that upvars cannot be pure-leaf");
}

#[test]
fn upvar_escape_reason_is_upvar_source() {
    // The per-name escape reason carries the structured UpvarSource kind so
    // the LSP / explorer can answer "why is v Frame?".
    let s = escape_ir("proc ::setit {name val} { upvar 1 caller_v v\n set v $val }");
    let p = summary(&s, "::setit");
    let reasons = p.reasons_for("v");
    assert!(
        reasons
            .iter()
            .any(|r| r.kind == EscapeReasonKind::UpvarSource),
        "expected an UpvarSource reason, got {reasons:?}"
    );
}

#[test]
fn global_declared_var_escapes_to_frame() {
    // tclsh: `set g 1; proc bump {} { global g; incr g }; bump; bump; set g`
    // -> 3, so the global-linked `g` is observed across frames and must be
    // Frame.
    let s = escape_ir("proc ::bump {} { global g\n incr g }");
    let p = summary(&s, "::bump");
    assert!(p.is_frame("g"), "global-linked var must be Frame");
    assert!(p.frame_needed);
}

#[test]
fn variable_declared_var_escapes_to_frame() {
    // tclsh: `namespace eval ns {variable v 7; proc get {} {variable v;
    //          return $v}}; ns::get` -> 7 (the namespace var is read by
    // name), so `v` must live in the frame.
    let s = escape_ir("proc ::get {} { variable counter\n return $counter }");
    let p = summary(&s, "::get");
    assert!(p.is_frame("counter"), "variable-linked var must be Frame");
}

#[test]
fn namespace_upvar_escapes_local_alias() {
    // tclsh: `namespace eval ns {variable counter 5}; proc readit {} {
    //          namespace upvar ns counter c; return $c}; readit` -> 5
    // (the alias `c` reads the namespace var), so `c` must be Frame.
    let s = escape_ir("proc ::readit {} { namespace upvar ns counter c\n return $c }");
    let p = summary(&s, "::readit");
    assert!(p.is_frame("c"), "namespace-upvar alias must be Frame");
}

#[test]
fn uplevel_non_zero_body_is_a_dynamic_barrier() {
    // tclsh: `proc setc {} { uplevel 1 {set y 99} };
    //         proc outer {} { setc; return $y }; outer` -> 99 — the body
    // runs in the caller's frame and writes the caller's `y`. The analysis
    // can't bound which caller names the body touches, so the whole proc
    // goes to the pessimistic dynamic-barrier path.
    let s = escape_ir("proc ::setc {} { uplevel 1 {set y 99} }");
    let p = summary(&s, "::setc");
    assert!(p.dynamic_barrier(), "uplevel 1 must mark a dynamic barrier");
    assert!(p.frame_needed);
    // A barrier forces *every* name to Frame.
    assert!(p.is_frame("any_name_at_all"));
}

#[test]
fn info_exists_literal_escapes_just_that_name() {
    // `info exists myvar` resolves the name by string lookup against the
    // runtime frame, so `myvar` must be Frame — but only that name.
    let s = escape_ir("proc ::ie {} { set myvar 1\n if {[info exists myvar]} { return 1 } }");
    let p = summary(&s, "::ie");
    assert!(p.is_frame("myvar"), "info-exists target must be Frame");
    assert!(
        !p.dynamic_barrier(),
        "a literal info-exists is not a whole-proc barrier"
    );
    // Only the named target escapes — a sibling local stays Local.
    let s2 = escape_ir("proc ::ie3 {} { set myvar 1\n set other 2\n return [info exists myvar] }");
    let p2 = summary(&s2, "::ie3");
    assert!(p2.is_frame("myvar"), "info-exists target Frame");
    assert_eq!(
        p2.tag("other"),
        EscapeTag::Local,
        "a name not named by info-exists stays Local"
    );
    // (The IR-walk summary records the Frame verdict; the structured
    // per-name InfoExists reason is surfaced on the analysis-internal path
    // and is exercised by the synthetic ProcEscapeSummary reason tests.)
}

#[test]
fn info_level_forces_pessimistic_frame() {
    // `info level` reads the caller frame; the analysis can't bound the
    // effect, so it goes pessimistic.
    let s = escape_ir("proc ::il {} { set x 1\n return [info level] }");
    let p = summary(&s, "::il");
    assert!(p.dynamic_barrier(), "info level must be a barrier");
    // A barrier reason should be surfaceable for any var.
    let reasons = p.reasons_for("x");
    assert!(
        reasons.iter().any(|r| r.kind == EscapeReasonKind::Barrier),
        "expected a Barrier reason, got {reasons:?}"
    );
}

#[test]
fn eval_dynamic_body_is_a_barrier() {
    // `eval $body` runs arbitrary code in the current frame — the analysis
    // can't scan it, so the proc is pessimistic.
    let s = escape_ir("proc ::ev {body} { set x 1\n eval $body }");
    let p = summary(&s, "::ev");
    assert!(p.dynamic_barrier());
    assert!(p.has_fallback(), "dynamic eval needs the eval fallback");
}

#[test]
fn dynamic_set_name_spills_all_known_locals() {
    // `set $n 99` with an unresolved `$n` could write *any* local, so the
    // fallback spills every known proc-local to the frame.
    let s = escape_ir("proc ::dn {n} { set a 1\n set b 2\n set $n 99 }");
    let p = summary(&s, "::dn");
    assert!(p.is_frame("a"), "a spilled by dynamic set-name");
    assert!(p.is_frame("b"), "b spilled by dynamic set-name");
    assert!(p.dynamic_barrier());
}

// --- NEGATIVE cases: a pure local does NOT escape ---

#[test]
fn pure_local_does_not_escape() {
    // tclsh: `proc pure {a b} { set s [expr {$a+$b}]; return $s }; pure 2 3`
    // -> 5, computed entirely within the proc frame — nothing is observable
    // outside it, so no var escapes and the proc is pure-leaf.
    let s = escape_ir("proc ::pure {a b} { set s [expr {$a+$b}]\n return $s }");
    let p = summary(&s, "::pure");
    assert_eq!(p.tag("s"), EscapeTag::Local, "pure local stays Local");
    assert_eq!(p.tag("a"), EscapeTag::Local);
    assert!(!p.frame_needed, "pure proc needs no runtime frame");
    assert!(!p.dynamic_barrier());
    assert!(p.pure_leaf, "pure leaf proc should be flagged");
    assert!(p.safe_to_inline());
    assert!(p.safe_to_dce());
    assert!(p.safe_for_frame_elision());
    // reasons_for a Local var is always empty — it is not in the frame.
    assert!(p.reasons_for("s").is_empty());
}

#[test]
fn frameless_builtin_call_keeps_local() {
    // Calling only frameless runtime builtins (`puts`, `list`) keeps the
    // proc frame-elidable.
    let s = escape_ir("proc ::log {msg} { set out [list $msg]\n puts $out }");
    let p = summary(&s, "::log");
    assert_eq!(p.tag("out"), EscapeTag::Local);
    assert!(!p.frame_needed);
    assert!(p.pure_leaf, "wrapper of frameless builtins is pure-leaf");
}

#[test]
fn plain_set_at_top_level_no_escape() {
    // A top-level pure assignment escapes nothing.
    let s = escape_ir("set x 1\nset y 2\nexpr {$x + $y}");
    let top = summary(&s, TOP_LEVEL_QNAME);
    assert_eq!(top.tag("x"), EscapeTag::Local);
    assert_eq!(top.tag("y"), EscapeTag::Local);
    assert!(!top.dynamic_barrier());
}

// ===========================================================================
// PART 2 — var_escape: interprocedural propagation
// ===========================================================================

#[test]
fn interprocedural_caller_inherits_upvar_pessimism() {
    // `::setit` upvars a *dynamic* source (`$name`), which can't be
    // enumerated → unbounded upvar source. `::wrap` calls it, so the
    // pessimism flows up the call edge.
    let s = escape_ir(
        "proc ::setit {name val} { upvar 1 $name v\n set v $val }\n\
         proc ::wrap {} { set x 5\n setit x 1 }",
    );
    let setit = summary(&s, "::setit");
    assert!(
        setit.unbounded_upvar_source(),
        "dynamic upvar src is unbounded"
    );
    assert!(setit.dynamic_barrier());
    let wrap = summary(&s, "::wrap");
    assert!(
        wrap.dynamic_barrier(),
        "caller of an unbounded-upvar proc is downgraded to pessimistic"
    );
    assert!(!wrap.pure_leaf);
}

#[test]
fn interprocedural_named_source_flows_to_caller() {
    // `::leaf` upvars a *literal* caller-frame name `x`. The caller `::host`
    // has a local `x`, which must therefore become Frame so the alias
    // resolves.
    let s = escape_ir(
        "proc ::leaf {} { upvar 1 x y\n set y 9 }\n\
         proc ::host {} { set x 1\n leaf }",
    );
    let leaf = summary(&s, "::leaf");
    assert!(leaf.upvar_source_names.contains("x"));
    let host = summary(&s, "::host");
    assert!(
        host.upvar_source_names.contains("x"),
        "transitive upvar source reaches the caller: {:?}",
        host.upvar_source_names
    );
    assert!(host.is_frame("x"), "caller's matching local must be Frame");
}

#[test]
fn interprocedural_pure_leaf_downgrade_through_callees() {
    // `::pureleaf` is a genuine pure leaf. `::wrap` calls `::setit` (upvar),
    // so it is downgraded. A wrapper of a *frameless runtime builtin*
    // (`puts`) stays pure-leaf, but a wrapper of any *user proc* (even a
    // pure-leaf one) does not: an intraprocedural call to a compiled proc
    // sets HAS_CALL_FALLBACK, and the pure-leaf clause forbids a call
    // fallback.
    let s = escape_ir(
        "proc ::setit {name val} { upvar 1 $name v\n set v $val }\n\
         proc ::wrap {} { setit a 1 }\n\
         proc ::pureleaf {x} { return [expr {$x * 2}] }\n\
         proc ::wrapuser {} { pureleaf 3 }\n\
         proc ::wrapbuiltin {msg} { puts $msg }",
    );
    assert!(
        !summary(&s, "::setit").pure_leaf,
        "upvar proc is not pure-leaf"
    );
    assert!(
        !summary(&s, "::wrap").pure_leaf,
        "caller of impure proc downgraded"
    );
    assert!(
        summary(&s, "::pureleaf").pure_leaf,
        "leaf doing pure compute is pure-leaf"
    );
    assert!(
        !summary(&s, "::wrapuser").pure_leaf,
        "a caller of a compiled proc has a call fallback, so it is not pure-leaf"
    );
    assert!(summary(&s, "::wrapuser").has_call_fallback());
    assert!(
        summary(&s, "::wrapbuiltin").pure_leaf,
        "a wrapper of a frameless builtin stays pure-leaf"
    );
}

#[test]
fn interprocedural_off_keeps_raw_intraprocedural_result() {
    // With IPA off, the intraprocedural pass still runs: a genuine pure leaf
    // is flagged, and the top-level key is present, but a caller is *not*
    // downgraded by its callee (no fixpoint).
    let s = escape_ir_raw(
        "proc ::setit {name val} { upvar 1 $name v\n set v $val }\n\
         proc ::wrap {} { set x 5\n setit x 1 }",
    );
    assert!(s.contains_key(TOP_LEVEL_QNAME));
    // Intraprocedurally `::wrap` only sees a call to setit (a HAS_CALL_FALLBACK
    // marker), not the callee's pessimism — so it is NOT yet a dynamic barrier.
    assert!(
        !summary(&s, "::wrap").dynamic_barrier(),
        "without IPA, the callee's pessimism has not propagated yet"
    );
    assert!(summary(&s, "::wrap").has_call_fallback());
}

#[test]
fn solve_interprocedural_escape_on_synthetic_summaries() {
    // Exercise the fixpoint directly on hand-built summaries (no lowering),
    // covering the public `solve_interprocedural_escape` entry point.
    let mk = |callees: &[&str], sources: &[&str]| ProcEscapeSummary {
        direct_callees: callees.iter().map(|s| (*s).to_string()).collect(),
        upvar_source_names: sources.iter().map(|s| (*s).to_string()).collect(),
        ..Default::default()
    };
    let mut input: HashMap<String, ProcEscapeSummary> = HashMap::new();
    input.insert("::a".into(), mk(&["::b"], &[]));
    input.insert("::b".into(), mk(&["::c"], &[]));
    input.insert("::c".into(), mk(&[], &["shared"]));
    let out = solve_interprocedural_escape(&input);
    // The transitive source flows up the whole a → b → c chain.
    assert!(out["::a"].upvar_source_names.contains("shared"));
    assert!(out["::b"].upvar_source_names.contains("shared"));
    assert!(out["::c"].upvar_source_names.contains("shared"));
    // Empty input round-trips to empty.
    assert!(solve_interprocedural_escape(&HashMap::<String, ProcEscapeSummary>::new()).is_empty());
}

// ===========================================================================
// PART 3 — var_escape: CFG/SSA path (analyse_var_escape_cu)
// ===========================================================================

#[test]
fn cu_path_upvar_escapes_alias() {
    // The flow-sensitive CFG/SSA path agrees with the IR walk on the
    // structural escape verdict for `upvar`.
    let s = escape_cu("proc ::setit {name val} { upvar 1 caller_v v\n set v $val }");
    let p = summary(&s, "::setit");
    assert_eq!(p.tag("v"), EscapeTag::Frame);
    assert!(p.is_frame("caller_v"));
    assert!(p.upvar_source_names.contains("caller_v"));
    assert!(p.frame_needed);
}

#[test]
fn cu_path_pure_local_needs_no_frame() {
    // On the CU path `pure_leaf` is intentionally left at its default
    // (the inlining predicate is only computed on the IR walk — see
    // `analyse_var_escape_cu`'s docs), so we assert the frame verdict
    // rather than pure_leaf here.
    let s = escape_cu("proc ::pure {a b} { set t [expr {$a + $b}]\n return $t }");
    let p = summary(&s, "::pure");
    assert_eq!(p.tag("t"), EscapeTag::Local);
    assert!(
        !p.frame_needed,
        "pure proc frame-elidable on the CU path too"
    );
    assert!(!p.dynamic_barrier());
}

#[test]
fn cu_path_includes_top_level_key() {
    let s = escape_cu("set x 1\nproc ::p {} { return 1 }");
    assert!(s.contains_key(TOP_LEVEL_QNAME));
    assert!(s.contains_key("::p"));
}

// ===========================================================================
// PART 4 — var_escape: slot resolution
// ===========================================================================

#[test]
fn slot_resolution_assigns_params_then_locals() {
    // A pure, slot-eligible proc gets compile-time local slots: params
    // first (slots 0, 1), then body locals in source order.
    let s = escape_ir("proc ::p {a b} { set x 1\n set y 2\n return [expr {$x+$y}] }");
    let p = summary(&s, "::p");
    assert_eq!(p.local_slots.get("a"), Some(&0));
    assert_eq!(p.local_slots.get("b"), Some(&1));
    assert_eq!(p.local_slots.get("x"), Some(&2));
    assert_eq!(p.local_slots.get("y"), Some(&3));
}

#[test]
fn slot_resolution_empty_for_pessimistic_proc() {
    // A proc that goes pessimistic (dynamic eval) is not slot-eligible, so
    // the slot map is empty.
    let s = escape_ir("proc ::p {body} { set x 1\n eval $body }");
    let p = summary(&s, "::p");
    assert!(
        p.local_slots.is_empty(),
        "pessimistic proc has no slots: {:?}",
        p.local_slots
    );
}

#[test]
fn assign_local_slots_direct_api() {
    // Drive `assign_local_slots` directly from the lowered proc body +
    // its summary — the public slot-resolution entry point.
    let module = lower_to_ir("proc ::q {p1} { set m 1\n set n 2 }", registry());
    let summaries = analyse_var_escape(&module, true);
    let proc = module.procedures.get("::q").expect("proc ::q lowered");
    let summ = summary(&summaries, "::q");
    let slots = assign_local_slots(&proc.body, summ, &proc.params);
    assert_eq!(slots.get("p1"), Some(&0), "param claims slot 0");
    assert_eq!(slots.get("m"), Some(&1));
    assert_eq!(slots.get("n"), Some(&2));
    assert!((slots.len() as usize) <= LOCALS_ARRAY_CAP);
}

// ===========================================================================
// PART 5 — var_escape: info-subcommand classification
// ===========================================================================

#[test]
fn info_subcommand_classification_sets() {
    // Frame-inspecting subcommands force pessimism.
    for sub in FRAME_INSPECTING_SUBCOMMANDS {
        assert!(
            is_frame_inspecting_info_subcommand(sub),
            "{sub} frame-inspecting"
        );
        assert!(!is_safe_info_subcommand(sub), "{sub} must not also be safe");
    }
    // Interpreter-global subcommands are safe.
    for sub in INTERPRETER_GLOBAL_SUBCOMMANDS {
        assert!(is_safe_info_subcommand(sub), "{sub} interpreter-global");
    }
    // Specific spot checks.
    assert!(is_frame_inspecting_info_subcommand("level"));
    assert!(is_frame_inspecting_info_subcommand("vars"));
    assert!(is_frame_inspecting_info_subcommand("locals"));
    assert!(is_frame_inspecting_info_subcommand("frame"));
    assert!(is_frame_inspecting_info_subcommand("coroutine"));
    assert!(is_frame_inspecting_info_subcommand("errorstack"));
    assert!(is_safe_info_subcommand("body"));
    assert!(is_safe_info_subcommand("commands"));
    assert!(is_safe_info_subcommand("patchlevel"));
    assert!(is_safe_info_subcommand("globals"));
    // An unknown subcommand is neither.
    assert!(!is_safe_info_subcommand("totally_made_up"));
    assert!(!is_frame_inspecting_info_subcommand("totally_made_up"));
}

#[test]
fn info_vars_in_proc_forces_pessimism() {
    // `info vars` enumerates frame locals → barrier.
    let s = escape_ir("proc ::iv {} { set x 1\n return [info vars] }");
    assert!(summary(&s, "::iv").dynamic_barrier());
}

#[test]
fn info_body_in_proc_is_safe() {
    // `info body` reads the global proc table, not the current frame — so
    // it is *not* a barrier and the local stays Local.
    let s = escape_ir("proc ::ib {} { set x [info body ::ib]\n return $x }");
    let p = summary(&s, "::ib");
    assert!(
        !p.dynamic_barrier(),
        "info body is interpreter-global, not a barrier"
    );
}

// ===========================================================================
// PART 6 — var_escape: ProcEscapeSummary / types unit surface
// ===========================================================================

#[test]
fn summary_join_and_default_tag() {
    assert_eq!(
        var_escape::join(EscapeTag::Local, EscapeTag::Local),
        EscapeTag::Local
    );
    assert_eq!(
        var_escape::join(EscapeTag::Local, EscapeTag::Frame),
        EscapeTag::Frame
    );
    assert_eq!(
        var_escape::join(EscapeTag::Frame, EscapeTag::Local),
        EscapeTag::Frame
    );
    let d = ProcEscapeSummary::default();
    assert_eq!(d.tag("anything"), EscapeTag::Local);
    assert!(!d.wants_frame());
}

#[test]
fn summary_with_escapes_and_barrier_helpers() {
    let base = ProcEscapeSummary::default();
    let spilled = base.with_escapes(["p".into(), "q".into()], false);
    assert_eq!(spilled.tag("p"), EscapeTag::Frame);
    assert_eq!(spilled.tag("q"), EscapeTag::Frame);
    assert!(spilled.frame_needed);

    // A pessimistic spill sets the dynamic barrier and forces every name.
    let pess = base.with_escapes(std::iter::empty(), true);
    assert!(pess.dynamic_barrier());
    assert!(pess.is_frame("never_named"));
    assert!(pess.wants_frame());

    // A populated `barriers` vec implies dynamic_barrier even without the
    // flag being set explicitly.
    let with_barrier = ProcEscapeSummary {
        barriers: vec![Barrier::with_detail(BarrierKind::Eval, "eval $body")],
        ..Default::default()
    };
    assert!(with_barrier.dynamic_barrier());
    let reasons = with_barrier.reasons_for("x");
    assert_eq!(reasons.len(), 1);
    assert_eq!(reasons[0].kind, EscapeReasonKind::Barrier);
    assert_eq!(reasons[0].detail, "eval $body");
}

#[test]
fn summary_explicit_reason_passthrough_and_flags() {
    let mut tag_reasons: HashMap<String, Vec<EscapeReason>> = HashMap::new();
    tag_reasons.insert(
        "v".into(),
        vec![EscapeReason::with_detail(
            EscapeReasonKind::CalleeUpvar,
            "callee upvars v",
        )],
    );
    let s = ProcEscapeSummary {
        tags: HashMap::from([("v".to_string(), EscapeTag::Frame)]),
        tag_reasons,
        flags: EscapeFlags::HAS_FALLBACK,
        ..Default::default()
    };
    let r = s.reasons_for("v");
    assert_eq!(r.len(), 1);
    assert_eq!(r[0].kind, EscapeReasonKind::CalleeUpvar);
    assert!(s.has_fallback());
    assert!(s.wants_frame(), "has_fallback alone wants a frame");
    // A var not tagged Frame has no reasons.
    assert!(s.reasons_for("unknown").is_empty());
}

// ===========================================================================
// PART 7 — type_infer: literal classification (set-statement context)
// ===========================================================================

#[test]
fn literal_scalar_types() {
    // tclsh: `string is integer 42` = 1 => Int.
    assert_eq!(ty("set x 42", "x"), TclType::Int);
    // tclsh: `string is double 3.14` = 1, `string is integer 3.14` = 0 => Double.
    assert_eq!(ty("set x 3.14", "x"), TclType::Double);
    // tclsh: `string is boolean true` = 1 => Boolean.
    assert_eq!(ty("set x true", "x"), TclType::Boolean);
    assert_eq!(ty("set x false", "x"), TclType::Boolean);
    // A plain word with no numeric/boolean intrep is a String.
    assert_eq!(ty("set x hello", "x"), TclType::String);
}

#[test]
fn hex_and_binary_literals_are_int_octal_is_string() {
    // tclsh: `string is integer 0x1f` = 1 => Int (hex stores an INT intrep).
    assert_eq!(ty("set x 0x1f", "x"), TclType::Int);
    assert_eq!(ty("set x 0X80", "x"), TclType::Int);
    assert_eq!(ty("set x 0b1010", "x"), TclType::Int);
    assert_eq!(ty("set x -0xFF", "x"), TclType::Int);
    // Set-statement classifier keeps `0o…` octal as String (its canonical
    // stringified intrep differs from the source text).
    assert_eq!(ty("set x 0o17", "x"), TclType::String);
}

#[test]
fn interpolated_value_is_string() {
    // tclsh: a `$`-interpolated word builds a plain string value; the
    // set-statement classifier types it String.
    assert_eq!(ty("set y 1\nset x foo$y", "x"), TclType::String);
    assert_eq!(ty("set x a[string length z]b", "x"), TclType::String);
}

#[test]
fn incr_result_is_int() {
    // tclsh: `set c 1; incr c` -> 2, `string is integer 2` = 1 => Int.
    assert_eq!(ty("incr c", "c"), TclType::Int);
    assert_eq!(ty("set c 5\nincr c 3", "c"), TclType::Int);
}

// ===========================================================================
// PART 8 — type_infer: command return types (via `set v [cmd …]`)
// ===========================================================================

#[test]
fn command_return_type_llength_is_int() {
    // tclsh: `string is integer [llength {a b}]` = 1 => Int.
    assert_eq!(ty("set n [llength {a b c}]", "n"), TclType::Int);
    assert_eq!(ty("set lst {a b}\nset n [llength $lst]", "n"), TclType::Int);
}

#[test]
fn command_return_type_string_length_is_int() {
    // tclsh: `string is integer [string length hello]` = 1 => Int.
    assert_eq!(ty("set n [string length hello]", "n"), TclType::Int);
}

#[test]
fn command_return_type_string_index_is_string() {
    // tclsh: `string index abc 0` -> "a" (a string); the registry types
    // `string index` as String.
    assert_eq!(ty("set i [string index abc 0]", "i"), TclType::String);
}

#[test]
fn command_return_type_string_is_is_boolean() {
    // tclsh: `string is integer 5` -> 1, a boolean 0/1; registry => Boolean.
    assert_eq!(ty("set b [string is integer 5]", "b"), TclType::Boolean);
}

#[test]
fn command_return_type_string_toupper_is_string() {
    // tclsh: `string toupper hi` -> "HI" (a string).
    assert_eq!(ty("set u [string toupper hi]", "u"), TclType::String);
}

#[test]
fn command_return_type_format_is_string() {
    // tclsh: `format %d 5` -> "5" — a string value (the registry types
    // `format` as String regardless of the conversion).
    assert_eq!(ty("set s [format %d 5]", "s"), TclType::String);
}

#[test]
fn command_return_type_clock_seconds_is_int() {
    // tclsh: `string is integer [clock seconds]` = 1 => Int.
    assert_eq!(ty("set t [clock seconds]", "t"), TclType::Int);
}

#[test]
fn command_return_type_clock_format_is_string() {
    // tclsh: `clock format 0` -> a date string; registry => String. The
    // subcommand dispatch (`clock format` vs `clock seconds`) is exercised:
    // the two siblings infer different types.
    assert_eq!(ty("set t [clock format 0]", "t"), TclType::String);
}

#[test]
fn unknown_command_result_is_unknown() {
    // A command absent from the registry has no resolvable return type, so
    // its def widens to Overdefined — i.e. no concrete `tcl_type`.
    assert!(
        infer_type("set x [no_such_command_xyz]", "::top", "x").is_none(),
        "unknown command result should not get a concrete type"
    );
}

// ===========================================================================
// PART 9 — type_infer: expr results (the `expr {…}` lowered form)
// ===========================================================================

#[test]
fn expr_integer_arithmetic_is_int() {
    // tclsh: `string is integer [expr {3 + 4}]` = 1 => Int.
    assert_eq!(ty("set i [expr {3 + 4}]", "i"), TclType::Int);
    // tclsh: `expr {7 % 3}` -> 1 (Int); `expr {2 ** 10}` -> 1024 (Int).
    assert_eq!(ty("set m [expr {7 % 3}]", "m"), TclType::Int);
    assert_eq!(ty("set p [expr {2 ** 10}]", "p"), TclType::Int);
}

#[test]
fn expr_double_arithmetic_is_double() {
    // tclsh: `set v [expr {1.0 + 2}]; string is double $v` = 1 and
    // `string is integer $v` = 0 => Double (a double operand promotes).
    assert_eq!(ty("set d [expr {1.0 + 2}]", "d"), TclType::Double);
    // tclsh: `expr {1.0/2}` -> 0.5 (Double).
    assert_eq!(ty("set d [expr {1.0 / 2}]", "d"), TclType::Double);
}

#[test]
fn expr_integer_division_stays_int() {
    // tclsh: `expr {1/2}` -> 0 — integer division yields an Int.
    assert_eq!(ty("set q [expr {1 / 2}]", "q"), TclType::Int);
}

#[test]
fn expr_comparison_and_logical_are_boolean() {
    // tclsh: `expr {1 < 2}` -> 1, a boolean 0/1 result.
    assert_eq!(ty("set q [expr {1 < 2}]", "q"), TclType::Boolean);
    assert_eq!(ty("set q [expr {3 == 3}]", "q"), TclType::Boolean);
    assert_eq!(ty("set q [expr {1 && 0}]", "q"), TclType::Boolean);
    assert_eq!(ty("set q [expr {!1}]", "q"), TclType::Boolean);
}

#[test]
fn expr_bitwise_and_shift_are_int() {
    // tclsh: `expr {5 & 3}` -> 1 (Int). Bitwise/shift always coerce to Int.
    assert_eq!(ty("set r [expr {5 & 3}]", "r"), TclType::Int);
    assert_eq!(ty("set r [expr {5 | 2}]", "r"), TclType::Int);
    assert_eq!(ty("set r [expr {6 ^ 3}]", "r"), TclType::Int);
    assert_eq!(ty("set r [expr {1 << 4}]", "r"), TclType::Int);
    assert_eq!(ty("set r [expr {64 >> 2}]", "r"), TclType::Int);
    // tclsh: `string is integer [expr {~3}]` = 1 — bitwise NOT coerces to Int.
    assert_eq!(ty("set r [expr {~3}]", "r"), TclType::Int);
}

#[test]
fn expr_math_functions_infer_return_type() {
    // tclsh: `set v [expr {sqrt(2.0)}]; string is double $v` = 1,
    // `string is integer $v` = 0 => Double.
    assert_eq!(ty("set v [expr {sqrt(2.0)}]", "v"), TclType::Double);
    // tclsh: `string is integer [expr {int(3.9)}]` = 1 => Int.
    assert_eq!(ty("set v [expr {int(3.9)}]", "v"), TclType::Int);
    // tclsh: `string is integer [expr {abs(-5)}]` = 1 => Int (abs is identity).
    assert_eq!(ty("set v [expr {abs(-5)}]", "v"), TclType::Int);
}

#[test]
fn expr_ceil_is_double() {
    // tclsh: `set v [expr {ceil(3.1)}]; string is integer $v` = 0, `$v` ==
    // 4.0 => Double. The pass infers Double, matching tclsh. (An earlier
    // revision mis-inferred Int here; the current `expr_call_type` lists
    // ceil/floor under double-returning math, so the two agree.)
    assert_eq!(ty("set v [expr {ceil(3.1)}]", "v"), TclType::Double);
    assert_eq!(ty("set v [expr {floor(9.9)}]", "v"), TclType::Double);
}

#[test]
fn expr_unary_minus_preserves_operand_int() {
    // tclsh: `string is integer [expr {-5}]` = 1 — unary sign is identity.
    assert_eq!(ty("set v [expr {-5}]", "v"), TclType::Int);
    // tclsh: `set v [expr {-2.5}]; string is double $v` = 1 => Double.
    assert_eq!(ty("set v [expr {-2.5}]", "v"), TclType::Double);
}

#[test]
fn expr_ternary_mixed_branches_join_to_numeric() {
    // The pass does not model branch selection, so a ternary's type is the
    // *join* of both arms. `type_join(Double, Int)` is the abstract
    // `Numeric` lattice point (the Int-or-Double join), not a concrete
    // Double. tclsh proves the *runtime* value of `expr {1 ? 2.0 : 3}` is a
    // Double (`string is double` = 1, `string is integer` = 0), but the
    // static lattice over-approximates the unselected branch to Numeric.
    let lat = infer_lattice("set v [expr {1 ? 2.0 : 3}]", "v").expect("v typed");
    assert_eq!(lat.tcl_type(), Some(TclType::Numeric));
    // Two same-typed arms collapse to that concrete type.
    assert_eq!(ty("set v [expr {1 ? 5 : 9}]", "v"), TclType::Int);
}

#[test]
fn expr_unknown_function_is_numeric() {
    // An unknown expr function over an untyped operand can't be resolved to
    // a concrete Int/Double, but an `expr` always yields a number, so the
    // pass conservatively widens to the abstract `Numeric` join (no direct
    // single-value tclsh analogue — it is the Int-or-Double lattice point).
    let lat = infer_lattice("set v [expr {not_a_real_fn($v)}]", "v").expect("v typed");
    assert_eq!(lat.tcl_type(), Some(TclType::Numeric));
}

#[test]
fn expr_arithmetic_over_untyped_var_is_numeric() {
    // tclsh: `expr {$x + 1}` is a number, but with `$x` of unknown type the
    // pass can't decide Int vs Double, so it lands on the `Numeric` join.
    // (Top-level `$x` comes from a barrier-typed source, so it is untyped.)
    let lat = infer_lattice("set v [expr {$unknownvar + 1}]", "v").expect("v typed");
    assert_eq!(lat.tcl_type(), Some(TclType::Numeric));
}

// ===========================================================================
// PART 10 — type_infer: propagation through pure copies and scope aliases
// ===========================================================================

#[test]
fn pure_var_copy_inherits_source_type() {
    // tclsh: `set x 42; set y $x` makes `y` carry x's Int intrep.
    assert_eq!(ty("set x 42\nset y $x", "y"), TclType::Int);
    assert_eq!(ty("set x 3.5\nset y $x", "y"), TclType::Double);
}

#[test]
fn scope_alias_def_widens_to_overdefined() {
    // `variable counter` imports an externally-determined var; its def must
    // be Overdefined (the imported intrep is unknown), not `variable`'s
    // nominal String return type. tclsh: the value of a `variable`-linked
    // name is whatever the namespace holds — the compiler can't know it.
    let cu = CompilationUnit::build_for(
        "proc ::f {} { variable counter\n return $counter }",
        registry(),
        false,
    );
    let fu = cu.procedures.get("::f").expect("proc ::f");
    let widened = fu
        .types
        .iter()
        .any(|((n, _), t)| fu.ssa.var_name(*n) == "counter" && t.kind() == TypeKind::Overdefined);
    assert!(
        widened,
        "scope-aliased counter should be Overdefined: {:?}",
        fu.types
    );
}

#[test]
fn global_alias_def_widens_to_overdefined() {
    // `global g` likewise imports an external intrep → Overdefined.
    let cu = CompilationUnit::build_for("proc ::f {} { global g\n return $g }", registry(), false);
    let fu = cu.procedures.get("::f").expect("proc ::f");
    let widened = fu
        .types
        .iter()
        .any(|((n, _), t)| fu.ssa.var_name(*n) == "g" && t.kind() == TypeKind::Overdefined);
    assert!(
        widened,
        "global-aliased g should be Overdefined: {:?}",
        fu.types
    );
}

// ===========================================================================
// PART 11 — subst_nocommands: the compile-time `-nocommands` evaluator
// ===========================================================================

#[test]
fn subst_nocommands_leaves_brackets_literal_and_substitutes_var() {
    // tclsh: `set x Q; subst -nocommands {a[b]c $x}` -> "a[b]c Q". The
    // `[b]` command substitution is suppressed (that *is* -nocommands) while
    // `$x` is substituted. The compile-time helper matches, resolving `x`
    // from the const-map.
    assert_eq!(subst("a[b]c $x", &[("x", "Q")]).as_deref(), Some("a[b]c Q"));
}

#[test]
fn subst_nocommands_nested_brackets_kept_whole() {
    // tclsh: `subst -nocommands {x[a [b] c]y}` -> "x[a [b] c]y" — the whole
    // balanced bracket span is left literal.
    assert_eq!(subst("x[a [b] c]y", &[]).as_deref(), Some("x[a [b] c]y"));
}

#[test]
fn subst_nocommands_braced_and_bare_var_resolve_same_entry() {
    // tclsh: with `set name world`, both `subst -nocommands {$name}` and
    // `subst -nocommands {${name}}` -> "world".
    let m = cmap(&[("name", "world")]);
    assert_eq!(
        var_escape_subst("hi $name", &m).as_deref(),
        Some("hi world")
    );
    assert_eq!(
        var_escape_subst("hi ${name}", &m).as_deref(),
        Some("hi world")
    );
}

#[test]
fn subst_nocommands_backslash_escapes_processed() {
    // tclsh: `subst -nocommands {x\ny}` -> "x\ny" (a real newline) — the
    // -nocommands flag does NOT suppress backslash processing.
    assert_eq!(subst(r"x\ny", &[]).as_deref(), Some("x\ny"));
    // tclsh: `subst -nocommands {\t}` -> a tab.
    assert_eq!(subst(r"\t", &[]).as_deref(), Some("\t"));
    // tclsh: `subst -nocommands {\x41}` -> "A" (hex escape).
    assert_eq!(subst(r"\x41", &[]).as_deref(), Some("A"));
    // tclsh: `subst -nocommands {é}` -> "é" (unicode escape).
    assert_eq!(subst(r"é", &[]).as_deref(), Some("\u{00e9}"));
}

#[test]
fn subst_nocommands_dollar_dollar_kept_literal() {
    // tclsh: `subst -nocommands {$$}` -> "$$" — neither `$` starts a
    // resolvable name.
    assert_eq!(subst("$$", &[]).as_deref(), Some("$$"));
    // A `$` followed by punctuation stays literal.
    assert_eq!(
        subst("cost is $ now", &[]).as_deref(),
        Some("cost is $ now")
    );
}

#[test]
fn subst_nocommands_unicode_passthrough() {
    // Non-ASCII text passes through unchanged (the ASCII fast-path vs
    // char-decode branch). tclsh: `subst -nocommands {héllo}` -> "héllo".
    assert_eq!(subst("h\u{00e9}llo", &[]).as_deref(), Some("h\u{00e9}llo"));
    assert_eq!(subst("", &[]).as_deref(), Some(""));
}

#[test]
fn subst_nocommands_stray_close_bracket_is_literal() {
    // A `]` with no matching `[` is output literally (the stray-`]` arm).
    assert_eq!(subst("a]b", &[]).as_deref(), Some("a]b"));
}

// --- Refusal (None) cases: the helper conservatively bails out ---

#[test]
fn subst_nocommands_missing_var_refuses() {
    // A `$var` with no const-map entry refuses the whole evaluation — the
    // caller keeps the dynamic dispatch path.
    assert!(subst("$absent", &[]).is_none());
    assert!(subst("${absent}", &[]).is_none());
}

#[test]
fn subst_nocommands_unbalanced_bracket_is_literal() {
    // `-nocommands` disables *command* substitution, so `[` is
    // an ordinary character and an unbalanced `[` is NOT an error — it is
    // copied literally (tclsh: `subst -nocommands {[unbalanced}` -> `[unbalanced`).
    assert_eq!(subst("[unbalanced", &[]).as_deref(), Some("[unbalanced"));
    assert_eq!(
        subst("ok then [oops", &[("x", "1")]).as_deref(),
        Some("ok then [oops")
    );
}

#[test]
fn subst_nocommands_array_ref_refuses() {
    // `$a(idx)` array references are refused.
    assert!(subst("$a(idx)", &[("a", "x")]).is_none());
}

#[test]
fn subst_nocommands_namespace_qualified_refuses() {
    // `$a::b` and `$::name` namespace-qualified refs are refused.
    assert!(subst("$a::b", &[("a", "x")]).is_none());
    assert!(subst("$::name", &[("name", "x")]).is_none());
}

#[test]
fn subst_nocommands_complex_braced_name_refuses() {
    // An empty `${}` or a braced name with `::` / array / interpolation is
    // a complex name → refuse.
    assert!(subst("${}", &[]).is_none());
    assert!(subst("${a::b}", &[("a::b", "x")]).is_none());
    assert!(subst("${a(b)}", &[]).is_none());
}

#[test]
fn subst_nocommands_combination_of_features() {
    // A template mixing all the accepted shapes: bare var, braced var,
    // literal bracket span, backslash escape, and stray `$`.
    // tclsh: `set a A; set b B; subst -nocommands {$a-${b}-[x y]-\t-$}`
    //   -> "A-B-[x y]-<tab>-$".
    let m = cmap(&[("a", "A"), ("b", "B")]);
    assert_eq!(
        var_escape_subst("$a-${b}-[x y]-\\t-$", &m).as_deref(),
        Some("A-B-[x y]-\t-$")
    );
}
