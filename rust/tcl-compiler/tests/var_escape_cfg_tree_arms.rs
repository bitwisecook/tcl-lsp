//! CFG-walker `tree_*` sub-arm coverage: `return` / `expr` statements nested
//! inside an opaque `catch` body's control flow.
//!
//! Companion to `var_escape_cfg_catch.rs`. That file reaches the
//! `tree_assign_or_incr` (set/incr/upvar) and `tree_structural`
//! (if/for/while/foreach/switch) arms of `cfg_propagation/walker.rs` through a
//! `catch {…}` opaque body. This file pins the remaining `tree_call_or_barrier`
//! arms — `Statement::Return` and `Statement::ExprEval` — which only run when
//! such a statement appears *inside* the walked body.
//!
//! Precondition (see `handle_catch`): the body must contain no `$`/`[`
//! substitution, or the walker bails on a dynamic token before recursing. Every
//! body below is therefore substitution-free (literal conditions, literal
//! return values, constant `expr` operands). The asserted facts are
//! compiler-internal structure (the walk recurses without a spurious whole-proc
//! barrier, and reaches the inner write); `catch` runs its body in the
//! enclosing frame (verified on tclsh 8.6 + 9.0 in the companion file), so a
//! name the body writes is observable there.

use tcl_compiler::cfg_builder::build_cfg_function;
use tcl_compiler::lowering::lower_to_ir;
use tcl_compiler::ssa::build_ssa;
use tcl_compiler::var_escape::cfg_propagation::state::CfgEscapeResult;
use tcl_compiler::var_escape::{EscapeTag, analyse_cfg_function};
use tcl_registry::{CommandRegistry, registry_for_dialect};

fn registry() -> &'static CommandRegistry {
    registry_for_dialect("tcl8.6")
}

fn cfg_result(src: &str) -> CfgEscapeResult {
    let m = lower_to_ir(src, registry());
    let cfg = build_cfg_function("::top", &m.top_level, true);
    let ssa = build_ssa(&cfg, registry());
    analyse_cfg_function(&cfg, &ssa, std::iter::empty::<String>())
}

fn is_frame(r: &CfgEscapeResult, name: &str) -> bool {
    r.name_tags.get(name) == Some(&EscapeTag::Frame)
}

#[test]
fn catch_literal_return_walks_return_arm() {
    // `tree_call_or_barrier`'s `Return` arm: a literal-value `return` scans its
    // value (no escape, no barrier) and recurses cleanly.
    let r = cfg_result("catch {return 5}");
    assert!(
        !r.dynamic_barrier(),
        "literal catch'd return is not pessimistic"
    );
}

#[test]
fn catch_while_return_walks_structural_then_return() {
    // `tree_structural` While arm -> `tree_call_or_barrier` Return arm.
    let r = cfg_result("catch {while {1} {return 7}}");
    assert!(
        !r.dynamic_barrier(),
        "catch'd while+return is not pessimistic"
    );
}

#[test]
fn catch_foreach_return_walks_structural_then_return() {
    // `tree_structural` Foreach arm -> Return arm, with a literal list.
    let r = cfg_result("catch {foreach e {a b} {return 3}}");
    assert!(
        !r.dynamic_barrier(),
        "catch'd foreach+return is not pessimistic"
    );
}

#[test]
fn catch_if_expr_eval_walks_expr_arm() {
    // `tree_structural` If arm -> `tree_call_or_barrier` ExprEval arm (a bare
    // `expr` statement with constant operands).
    let r = cfg_result("catch {if {1} {expr {2 + 2}}}");
    assert!(!r.dynamic_barrier(), "catch'd if+expr is not pessimistic");
}

#[test]
fn catch_switch_expr_eval_walks_expr_arm() {
    // `tree_structural` Switch arm -> ExprEval arm, with a literal switch value.
    let r = cfg_result("catch {switch x {x {expr {9 - 1}}}}");
    assert!(
        !r.dynamic_barrier(),
        "catch'd switch+expr is not pessimistic"
    );
}

#[test]
fn catch_for_body_return_walks_for_arm() {
    // `tree_structural` For arm (init/cond/next/body) -> Return arm. The `for`
    // init `set i 0` is a write the body escapes; the trailing `return` exercises
    // the Return arm in the same walk.
    let r = cfg_result("catch {for {set i 0} {1} {incr i} {return 4}}");
    assert!(
        is_frame(&r, "i"),
        "for-init write escapes: {:?}",
        r.name_tags
    );
    assert!(
        !r.dynamic_barrier(),
        "catch'd for+return is not pessimistic"
    );
}

#[test]
fn catch_nested_if_global_then_return() {
    // Nested `if` inside `if` inside the catch body: both `tree_structural` If
    // recursions run, the inner `global` escapes, and the `return` arm runs.
    let r = cfg_result("catch {if {1} {if {1} {global deep}} else {return 0}}");
    assert!(
        is_frame(&r, "deep"),
        "deeply nested global escapes: {:?}",
        r.name_tags
    );
}
