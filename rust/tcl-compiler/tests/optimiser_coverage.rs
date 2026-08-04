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

//! The comprehensive optimiser *coverage* suite (O100–O129, the shimmer
//! S100–S102 pipeline, and the GVN/CSE/LICM redundancy detector).
//!
//! ## Driving the optimiser
//!
//! Three entry points are exercised:
//!   * `apply_optimisations(src, &optimise_with_dialect(...))` for the rewritten
//!     text, and the `Vec<Optimisation>` for the `O1xx` codes.
//!   * the GVN/CSE/LICM finder: `find_redundancies_for_cu`,
//!     `find_partial_redundancies_for_cu`, and `find_loop_invariants_for_cu` over
//!     a `CompilationUnit` (full/partial → O105, loop → O106).
//!   * the shimmer/thunking detector: `find_shimmer_warnings_for_cu` (S100/S101)
//!     + `find_thunking_warnings_for_cu` (S102).
//!
//! De-Morgan / inversion logic lives inside the O110 `InstCombine` pass on the
//! expr AST (there is no standalone string→string helper). Those cases are
//! exercised as O110-driven `[expr {...}]` rewrites, the form `optimiser.rs`
//! already uses.
//!
//! ## C-Tcl proof
//!
//! Every assertion that encodes a Tcl-language *value* fact (a fold result, an
//! arithmetic outcome, an evaluation result) was checked against `tclsh9.0`
//! (and the De-Morgan / strength-reduction / self-comparison families were
//! exhaustively swept over their operands), with the verified value cited in a
//! `// tclsh:` comment. Optimiser-structural facts (which `O1xx`/`S1xx` code
//! fired, that a store was eliminated) are about the compiler IR, not Tcl
//! semantics, and are flagged as such.
//!
//! ## Notes on behaviour
//!
//! Where the optimiser is confirmed *sound* against tclsh (more aggressive, or a
//! different but equivalent O-code), the assertion is adapted to the actual
//! behaviour and commented at the site. Gaps (the optimiser soundly leaves a
//! construct unchanged) assert the conservative no-op. A genuine miscompile
//! (`$x - $x` → 0 on an untyped operand) has since been FIXED (O110 now gates the
//! self-subtraction/xor fold on a provably-numeric operand); the assertion
//! asserts the corrected behaviour. No `#[ignore]`; every test passes.

use tcl_compiler::compilation_unit::CompilationUnit;
use tcl_compiler::gvn::{
    find_loop_invariants_for_cu, find_partial_redundancies_for_cu, find_redundancies_for_cu,
};
use tcl_compiler::optimiser::manager::{
    apply_optimisations, optimise_source_multipass, optimise_with_dialect,
};
use tcl_compiler::shimmer::{find_shimmer_warnings_for_cu, find_thunking_warnings_for_cu};
use tcl_registry::registry_for_dialect;

const TCL: &str = "tcl8.6";
const IR: &str = "f5-irules";

// ---------------------------------------------------------------------------
// Helpers (mirror optimiser.rs: opt_codes / opt_fires / optimised)
// ---------------------------------------------------------------------------

/// Sorted, de-duplicated `Oxxx` codes emitted by a single optimiser pass.
fn opt_codes(src: &str, dialect: &str) -> Vec<String> {
    let registry = registry_for_dialect(dialect);
    let d = (!dialect.is_empty()).then_some(dialect);
    let mut v: Vec<String> = optimise_with_dialect(src, registry, d)
        .iter()
        .map(|o| o.code.as_str().to_owned())
        .collect();
    v.sort();
    v.dedup();
    v
}

/// True if any optimisation with `code` fires on `src` under `dialect`.
fn opt_fires(src: &str, dialect: &str, code: &str) -> bool {
    let registry = registry_for_dialect(dialect);
    let d = (!dialect.is_empty()).then_some(dialect);
    optimise_with_dialect(src, registry, d)
        .iter()
        .any(|o| o.code.as_str() == code)
}

/// True if NONE of the optimisations carry `code`.
fn opt_absent(src: &str, dialect: &str, code: &str) -> bool {
    !opt_fires(src, dialect, code)
}

/// Apply all optimisations and return the rewritten source.
fn optimised(src: &str, dialect: &str) -> String {
    let registry = registry_for_dialect(dialect);
    let d = (!dialect.is_empty()).then_some(dialect);
    apply_optimisations(src, &optimise_with_dialect(src, registry, d))
}

/// True if any emitted code is in `wanted`.
fn opt_any_of(src: &str, dialect: &str, wanted: &[&str]) -> bool {
    let registry = registry_for_dialect(dialect);
    let d = (!dialect.is_empty()).then_some(dialect);
    optimise_with_dialect(src, registry, d)
        .iter()
        .any(|o| wanted.contains(&o.code.as_str()))
}

/// GVN/CSE/LICM codes: the union of full + partial redundancies (→ O105) and
/// loop invariants (→ O106).
fn gvn_codes(src: &str) -> Vec<String> {
    let registry = registry_for_dialect(TCL);
    let cu = CompilationUnit::build_for(src, registry, false);
    let mut v: Vec<String> = Vec::new();
    for r in find_redundancies_for_cu(&cu, registry, Some(TCL)) {
        v.push(r.code.as_str().to_owned());
    }
    for r in find_partial_redundancies_for_cu(&cu, registry, Some(TCL)) {
        v.push(r.code.as_str().to_owned());
    }
    for r in find_loop_invariants_for_cu(&cu, registry, Some(TCL)) {
        v.push(r.code.as_str().to_owned());
    }
    v.sort();
    v
}

/// Shimmer-warning codes (S100/S101 + S102 thunking).
fn shimmer_codes(src: &str) -> Vec<String> {
    let registry = registry_for_dialect(TCL);
    let cu = CompilationUnit::build_for(src, registry, false);
    let mut v: Vec<String> = Vec::new();
    for w in find_shimmer_warnings_for_cu(&cu, registry) {
        v.push(w.code.as_str().to_owned());
    }
    for w in find_thunking_warnings_for_cu(&cu, registry) {
        v.push(w.code.as_str().to_owned());
    }
    v.sort();
    v
}

/// Count shimmer warnings carrying `code` (S100/S101/S102).
fn shimmer_count(src: &str, code: &str) -> usize {
    shimmer_codes(src).iter().filter(|c| *c == code).count()
}

/// Wraps `body` in a loop where `$x` is SCCP-typed INT and live (the
/// precondition the D5-O110 identity/annihilator drops require). Matches the
/// helper in `optimiser.rs`.
fn int_x(body: &str) -> String {
    format!(
        "proc f {{n}} {{\n  for {{set x 0}} {{$x < $n}} {{incr x}} {{\n    {body}\n    puts $v\n  }}\n}}\n"
    )
}

// ===========================================================================
// O100 — constant propagation into expressions
// ===========================================================================

#[test]
fn o100_constant_propagation() {
    // A propagated constant either folds the condition (O101) or eliminates the
    // `if` outright (O112 via SCCP).
    assert!(opt_any_of(
        "set a 10\nif {$a > 5} { puts yes }",
        TCL,
        &["O101", "O112"]
    ));
    assert!(opt_any_of(
        "set a true\nif {$a} { puts yes }",
        TCL,
        &["O100", "O101", "O112"]
    ));
    assert!(opt_any_of(
        "set a false\nif {$a} { puts yes }",
        TCL,
        &["O100", "O101", "O112"]
    ));

    // tclsh: `set a 1; set a 2; set b [expr {$a + 1}]` ⇒ b == 3. The reassign
    // (`set a 2`) needs the fixpoint helper to reach the fold (one pass removes
    // the dead store and forwards the literal; the multipass folds `b`), as in
    // optimiser.rs.
    let registry = registry_for_dialect(TCL);
    let (reassign, _) = optimise_source_multipass(
        "set a 1\nset a 2\nset b [expr {$a + 1}]",
        registry,
        Some(TCL),
        10,
    );
    assert!(reassign.contains("set b 3"));
    // tclsh: a=3, b=4 ⇒ a+b == 7.
    assert!(optimised("set a 3\nset b 4\nset c [expr {$a + $b}]", TCL).contains("set c 7"));
    // tclsh: a=-5 ⇒ -5 + 10 == 5.
    assert!(optimised("set a -5\nset b [expr {$a + 10}]", TCL).contains("set b 5"));
    // tclsh: a=0 ⇒ 0 + 0 == 0. In a single pass O110's `+0` identity removal
    // pre-empts the constant fold, leaving `set a 0\nset b [expr {$a}]` (still
    // b==0). With a consumer the propagation completes to the literal — assert
    // the tclsh value end-to-end.
    assert_eq!(
        optimised("set a 0\nset b [expr {$a + 0}]\nputs $b", TCL),
        "set a 0\nputs 0"
    );
}

// ===========================================================================
// O101 — fold pure constant expressions
// ===========================================================================

#[test]
fn o101_constant_folding_arithmetic() {
    // Each value tclsh9.0-verified.
    assert!(optimised("set v [expr {2 + 3}]", TCL).contains("set v 5")); // tclsh: 5
    assert!(optimised("set v [expr {10 - 7}]", TCL).contains("set v 3")); // tclsh: 3
    assert!(optimised("set v [expr {6 * 7}]", TCL).contains("set v 42")); // tclsh: 42
    assert!(optimised("set v [expr {10 / 3}]", TCL).contains("set v 3")); // tclsh: 3 (int trunc)
    assert!(optimised("set v [expr {10 % 3}]", TCL).contains("set v 1")); // tclsh: 1
    assert!(optimised("set v [expr {1 << 4}]", TCL).contains("set v 16")); // tclsh: 16
    assert!(optimised("set v [expr {16 >> 2}]", TCL).contains("set v 4")); // tclsh: 4
    assert!(optimised("set v [expr {2 ** 8}]", TCL).contains("set v 256")); // tclsh: 256
    assert!(optimised("set v [expr {(1 + 2) * (3 + 4)}]", TCL).contains("set v 21")); // tclsh: 21
}

#[test]
fn o101_constant_folding_bitwise_and_compare() {
    // Bitwise — tclsh: 0xFF&0x0F==15, 0xF0|0x0F==255, 0xFF^0xFF==0.
    assert!(optimised("set v [expr {0xFF & 0x0F}]", TCL).contains("15"));
    assert!(optimised("set v [expr {0xF0 | 0x0F}]", TCL).contains("255"));
    assert!(optimised("set v [expr {0xFF ^ 0xFF}]", TCL).contains('0'));
    // Comparisons — tclsh: 3>2==1, 1>2==0, 5==5 is 1, 5!=5 is 0.
    assert!(optimised("set v [expr {3 > 2}]", TCL).contains("set v 1"));
    assert!(optimised("set v [expr {1 > 2}]", TCL).contains("set v 0"));
    assert!(optimised("set v [expr {5 == 5}]", TCL).contains("set v 1"));
    assert!(optimised("set v [expr {5 != 5}]", TCL).contains("set v 0"));
}

#[test]
fn o101_constant_folding_ternary_unary_logical() {
    // tclsh: 1?42:99==42, 0?42:99==99.
    assert!(optimised("set v [expr {1 ? 42 : 99}]", TCL).contains("set v 42"));
    assert!(optimised("set v [expr {0 ? 42 : 99}]", TCL).contains("set v 99"));
    // tclsh: !0==1, -(-5)==5.
    assert!(optimised("set v [expr {!0}]", TCL).contains("set v 1"));
    assert!(optimised("set v [expr {-(-5)}]", TCL).contains("set v 5"));
    // tclsh: 1&&1==1, 0||1==1.
    assert!(optimised("set v [expr {1 && 1}]", TCL).contains("set v 1"));
    assert!(optimised("set v [expr {0 || 1}]", TCL).contains("set v 1"));

    // A constant branch condition triggers structure elimination (O112). tclsh:
    // 2+2==4 ⇒ 1 ⇒ the `if` body always runs.
    assert!(opt_fires("if {2 + 2 == 4} { puts yes }", TCL, "O112"));
    // No fold when the expr reads a variable.
    assert!(opt_absent("set v [expr {$x + 1}]", TCL, "O101"));
}

#[test]
fn o101_dialect_octal_arithmetic() {
    // TP: `010` is octal 8 under tcl8.x/f5-irules (this suite's default
    // dialect, TCL = "tcl8.6") — tclsh8.6: `010 + 1` = 9, `010 * 2` = 16,
    // `-010` = -8. Before the dialect-aware fix these folded to the
    // decimal reading (11, 20, -10) — a miscompile.
    assert!(optimised("set v [expr {010 + 1}]", TCL).contains("set v 9"));
    assert!(optimised("set v [expr {010 * 2}]", TCL).contains("set v 16"));
    assert!(optimised("set v [expr {-010}]", TCL).contains("set v -8"));
    assert!(optimised("set v [expr {010 + 1}]", IR).contains("set v 9"));

    // TP (control): the same source under plain tcl9.0 reads `010` as
    // decimal (TIP 472) — tclsh9.0: `010 + 1` = 11.
    assert!(optimised("set v [expr {010 + 1}]", "tcl9.0").contains("set v 11"));

    // FN guard: `08` is invalid octal under tcl8.x (Tcl raises an error —
    // tclsh8.6: `expr {08 + 1}` -> "invalid bareword \"08\"") so the
    // const-folder must decline rather than guess a decimal reading.
    assert!(opt_absent("set v [expr {08 + 1}]", TCL, "O101"));
    // Unaffected under tcl9.0, where `08` is decimal 8.
    assert!(optimised("set v [expr {08 + 1}]", "tcl9.0").contains("set v 9"));
}

#[test]
fn o101_expr_redefinition_guard() {
    // FP guard: once `expr` is renamed, `expr {1 + 2}` no longer calls the
    // builtin evaluator — O101 must not fold it.
    assert!(opt_absent(
        "rename expr real_expr\nexpr {1 + 2}",
        TCL,
        "O101"
    ));
    // TN/control: an unrelated rename elsewhere in the module doesn't
    // block the fold (the gate is keyed on `expr` specifically).
    assert!(opt_fires(
        "rename puts real_puts\nexpr {1 + 2}",
        TCL,
        "O101"
    ));
}

#[test]
fn o101_mathfunc_redefinition_guard() {
    // FP guard: `proc ::tcl::mathfunc::abs` shadows the builtin math
    // function everywhere in the module — real Tcl compiles `abs(x)` to a
    // `tcl::mathfunc::abs` command invocation, so once that's
    // user-defined, `expr {abs(-5)}` no longer means "call the C abs".
    assert!(opt_absent(
        "proc ::tcl::mathfunc::abs {x} { return 999 }\nexpr {abs(-5)}",
        TCL,
        "O101"
    ));
    // TN/control: no override present, folds as usual. tclsh: abs(-5)=5.
    assert!(optimised("set v [expr {abs(-5)}]", TCL).contains("set v 5"));
}

#[test]
fn o101_global_write_across_call_guard() {
    // FP guard (repro A): `mutate` writes the global `x` via `global x; set x
    // 99`, called between the top-level `set x 5` and the `expr {$x + 1}`
    // read. tclsh: `mutate` runs before the `puts`, so `x` is 99 there —
    // `expr {$x + 1}` is 100, NOT 6. Before the CFG builder widened the
    // call's `defs` with the callee's global-write summary, SCCP treated `x`
    // as still holding the SSA value from the top-level `set x 5` across the
    // opaque `mutate` call, and O101 wrongly folded to `puts 6`.
    assert!(opt_absent(
        "proc mutate {} { global x; set x 99 }\nset x 5\nmutate\nputs [expr {$x + 1}]",
        TCL,
        "O101"
    ));

    // TN/control: an unrelated global write in a *different* proc must not
    // widen `x` — the fix is name-precise, not a coarse "any call may
    // mutate any global" widening. tclsh: `mutate_y` never touches `x`, so
    // `expr {$x + 1}` is still 6 and O101 correctly folds it.
    assert!(
        optimised(
            "proc mutate_y {} { global y; incr y }\nset x 5\nmutate_y\nputs [expr {$x + 1}]",
            TCL,
        )
        .contains("puts 6")
    );

    // TP (transitive): `outer` doesn't write `x` itself but calls `mutate`,
    // which does — the call-graph closure must still invalidate `x` at the
    // `outer` call site. tclsh: same 100 result as the direct-call repro.
    assert!(opt_absent(
        "proc mutate {} { global x; set x 99 }\nproc outer {} { mutate }\nset x 5\nouter\nputs [expr {$x + 1}]",
        TCL,
        "O101"
    ));

    // TP (namespace variable): `variable` writes are outer-scope too.
    // tclsh: same reasoning as the `global` repro — 100, not 6.
    assert!(opt_absent(
        "namespace eval ::ns { variable x 5 }\nproc ::ns::mutate {} { variable x; set x 99 }\nset ::ns::x 5\n::ns::mutate\nputs [expr {$::ns::x + 1}]",
        TCL,
        "O101"
    ));

    // TP (embedded command substitution): the same soundness gap reached via
    // `set y [mutate]` rather than a bare `mutate` statement — the
    // embedded-substitution scan must catch it too.
    assert!(opt_absent(
        "proc mutate {} { global x; set x 99; return 1 }\nset x 5\nset y [mutate]\nputs [expr {$x + 1}]",
        TCL,
        "O101"
    ));

    // TP (conditional-branch write): sound over-approximation — a global
    // write on a branch that may not execute still invalidates the name at
    // the call site (the analysis is flow-insensitive by design).
    assert!(opt_absent(
        "proc mutate {} { global x\nif {$::cond} { set x 99 } }\nset x 5\nmutate\nputs [expr {$x + 1}]",
        TCL,
        "O101"
    ));
}

#[test]
fn o101_module_wide_variable_trace_guard() {
    // FP guard (repro B): a trace on `x` is installed by `install_trace`, a
    // *different* proc than the code performing the final `set x 6` / read
    // — the flow-sensitive TRACED flag (installed and read in the same
    // function body) can't see this; it needs the module-wide fact. tclsh:
    // the write trace rewrites `x` to 99 on every write, so `set x 6` really
    // leaves `x` == 99 and `expr {$x + 1}` is 100, NOT 7.
    assert!(opt_absent(
        "proc install_trace {} { trace add variable ::x write {apply {a {set ::x 99}}} }\nset x 5\ninstall_trace\nset x 6\nputs [expr {$x + 1}]",
        TCL,
        "O101"
    ));

    // TN/control: an unrelated trace (on a different variable) must not
    // widen `x` — the fix is name-precise. tclsh: `y` is never written by
    // this trace, `x` stays a plain local, so `expr {$x + 1}` still folds.
    assert!(
        optimised(
            "trace add variable y write cb\nset x 6\nputs [expr {$x + 1}]",
            TCL,
        )
        .contains("puts 7")
    );

    // TP (trace removed later still declines to fold): the fact is
    // flow-insensitive by design — a later `trace remove` does not
    // "untrace" a name for this analysis, so this only ever prevents an
    // otherwise-unsound fold, it never wrongly blocks one tclsh agrees with.
    assert!(opt_absent(
        "trace add variable x write cb\ntrace remove variable x write cb\nset x 6\nputs [expr {$x + 1}]",
        TCL,
        "O101"
    ));
}

// ===========================================================================
// O102 — fold [expr {...}] command substitution
// ===========================================================================

#[test]
fn o102_expr_command_substitution() {
    // tclsh: 3+4 == 7. A *pure-literal* expr subst is folded by the constant
    // folder (O101); O102 is reserved for an expr that consumed a *propagated*
    // constant. Both produce the same value.
    assert!(optimised("set v [expr {3 + 4}]", TCL).contains("set v 7"));
    assert!(opt_fires("set v [expr {3 + 4}]", TCL, "O101"));

    // Propagated constant into the expr subst ⇒ O102 (+ O100 propagation). tclsh:
    // a=5 ⇒ 5*2 == 10.
    let prop = "set a 5\nset v [expr {$a * 2}]";
    assert!(optimised(prop, TCL).contains("set v 10"));
    assert!(opt_fires(prop, TCL, "O102"));
    assert!(opt_fires(prop, TCL, "O100"));

    // Dynamic expr ([clock seconds]) does not fold via O102.
    assert!(opt_absent(
        "set v [expr {[clock seconds] + 1}]",
        TCL,
        "O102"
    ));
}

// ===========================================================================
// O102 — variable-trace / aliasing / frame-crossing safety
//
// Regression coverage for a confirmed silent miscompile: `run_load_forwarding`
// (O102) and its SCCP-sourced sibling paths (O100/O101/O103, sharing the same
// `constants` map — see `propagation::run_function`) forwarded a variable's
// literal or SCCP-derived value into a later read with no notion that a
// `trace add variable` handler can run arbitrary code on read/write, that a
// write handler can rewrite the stored value, or that the value is aliased
// into another frame (upvar/global/variable) a called proc could write
// through. Each `tclsh:` value was checked against `tclsh` 8.6.14 — trace,
// `interp eval`, `uplevel`, and `unset` semantics are unchanged across
// 8.4-9.0, so the same value holds under every dialect this suite targets.
// ===========================================================================

#[test]
fn o102_read_trace_via_helper_proc_blocks_forward() {
    // FN-would-be-TP-if-safe / actual TP for the fix: a read trace
    // installed by a *called* proc — not lexically between the def and
    // the use — must still block forwarding. Before the fix this
    // rewrote to `puts 5`, silently dropping the trace's `puts "read
    // trace fired"` side effect. tclsh: prints "read trace fired" then
    // "5" — `puts $x` must survive verbatim so the trace still fires at
    // runtime.
    let src = "proc onread {name1 name2 op} {\n    puts \"read trace fired\"\n}\nproc setup {} {\n    trace add variable ::x read onread\n}\nsetup\nset x 5\nputs $x\n";
    assert!(opt_absent(src, TCL, "O102"));
    assert!(optimised(src, TCL).contains("puts $x"));
}

#[test]
fn o102_read_trace_installed_before_def_blocks_forward() {
    // The trace-install call can precede the `set` textually too — no
    // statement sits *between* the def and the use, so a same-block
    // intervening scan alone would miss this; the module-wide
    // `traced_variables` fact (position-independent, mirroring
    // `traced_commands`) is what catches it.
    let src = "proc onread {name1 name2 op} {\n    puts \"read trace fired\"\n}\nproc setup {} {\n    trace add variable ::x read onread\n}\nsetup\nset x 5\nputs $x\n";
    assert!(opt_absent(src, TCL, "O102"));
}

#[test]
fn o102_write_trace_mutating_value_blocks_forward() {
    // TP for the fix: a write-trace handler can rewrite the value being
    // stored, so the literal text at the `set` is not the runtime value.
    // tclsh: the handler increments on every write, so `x` ends up `6`,
    // not the literal `5` written in source.
    let src = "proc onwrite {name1 name2 op} {\n    upvar 1 $name1 v\n    set v [expr {$v + 1}]\n}\ntrace add variable x write onwrite\nset x 5\nputs $x\n";
    assert!(opt_absent(src, TCL, "O102"));
    assert!(optimised(src, TCL).contains("puts $x"));
}

#[test]
fn o102_legacy_trace_variable_form_blocks_forward() {
    // The deprecated `trace variable name ops command` spelling (8.4-8.6
    // only) must gate the forward exactly like the modern `trace add
    // variable` form — no per-form gap in the registry-driven fact.
    let src = "proc onread {name1 name2 op} {\n    puts \"read trace fired\"\n}\ntrace variable x r onread\nset x 5\nputs $x\n";
    assert!(opt_absent(src, TCL, "O102"));
}

#[test]
fn o102_abbreviated_trace_add_variable_blocks_forward() {
    // C Tcl's `Tcl_GetIndexFromObj` accepts a unique prefix of the trace
    // type word, so `trace add var x read h` installs the same variable
    // trace as the full `variable` spelling (tclsh 8.6.14: prints "read
    // trace fired" then "5"). The registry's `arg_role_resolver` for
    // `trace add`/`remove` previously hand-matched the literal word
    // `"variable"` only, so this abbreviated form recorded no
    // `traced_variables` fact and still forwarded to `puts 5`.
    let src = "proc onread {name1 name2 op} {\n    puts \"read trace fired\"\n}\ntrace add var x read onread\nset x 5\nputs $x\n";
    assert!(opt_absent(src, TCL, "O102"));
    assert!(optimised(src, TCL).contains("puts $x"));
}

#[test]
fn o102_transitive_trace_taint_through_expr_blocks_forward() {
    // TP for the deeper, transitive fix: `a` carries a read trace, and
    // `v`'s SCCP-constant value is *computed from* `a` inside
    // `[expr {...}]}`. Filtering only the directly-traced name (`a`)
    // is not enough — `v` must inherit the taint through the SSA def/use
    // graph, or `set v [expr {$a * 2}]` still folds to `set v 10` and
    // `puts $v` still forwards, dropping the read trace exactly as
    // before. tclsh: prints "read trace fired" then "10".
    let src = "proc onread {name1 name2 op} {\n    puts \"read trace fired\"\n}\nproc setup {} {\n    trace add variable ::a read onread\n}\nset a 5\nsetup\nset v [expr {$a * 2}]\nputs $v\n";
    assert!(opt_absent(src, TCL, "O100"));
    assert!(opt_absent(src, TCL, "O101"));
    assert!(opt_absent(src, TCL, "O102"));
    let out = optimised(src, TCL);
    assert!(out.contains("set v [expr {$a * 2}]"));
    assert!(out.contains("puts $v"));
}

#[test]
fn o102_interp_eval_self_target_barrier_blocks_forward() {
    // TP: `interp eval {}` targets *this* interpreter (the empty path is
    // the self-reference), so its body runs in the caller's own global
    // scope — unlike a named sub-interpreter, it really can rewrite `x`.
    // tclsh: prints `99`, not the literal `5` written earlier.
    let src = "set x 5\ninterp eval {} {\n    set x 99\n}\nputs $x\n";
    assert!(opt_absent(src, TCL, "O102"));
    assert!(optimised(src, TCL).contains("puts $x"));
}

#[test]
fn o102_sub_interpreter_isolation_no_longer_over_blocked() {
    // Precision guard: a *named* sub-interpreter's variables are isolated
    // from the caller's — `interp eval slave {set x 99}` does not touch
    // the master's `x` (tclsh: prints `99` inside the slave block, then
    // `5` for the master, unaffected) — and `interp eval` with a named
    // (non-self) target is not a `Statement::Barrier`/`UpFrame`, so
    // `has_intervening_barrier` correctly does not gate on it. Was
    // previously a documented "conservative gap" only because an older
    // revision of `run_load_forwarding` blocked on *any* impure
    // intervening call, not just a real frame-crossing barrier — see
    // `o102_side_effecting_intervening_call_is_still_forwarded` for the
    // general case this generalises. The nested literal body's own `$x`
    // reference (lexically inside the slave's script, a separate
    // namespace) still only earns a hint (not a precise applicable
    // rewrite), so the source is unchanged either way — this test only
    // locks in that O102 no longer *unconditionally* declines here.
    let src = "set x 5\ninterp create slave\ninterp eval slave {\n    set x 99\n    puts $x\n}\nputs $x\n";
    assert!(opt_fires(src, TCL, "O102"));
    assert!(optimised(src, TCL).contains("puts $x"));
}

#[test]
fn o102_upvar_aliased_variable_blocks_forward() {
    // TP: `x` is written through an `upvar` alias from a called proc — a
    // memory-SSA-aliased name, so a call between a literal def and a
    // later use can rewrite it without a new SSA version appearing in
    // *this* function. (Here the literal `set x 5` is genuinely dead —
    // O109 removes it — but O102 must not offer to forward its value.)
    let src = "proc mutate {} {\n    upvar 1 x v\n    set v 99\n}\nset x 5\nmutate\nputs $x\n";
    assert!(opt_absent(src, TCL, "O102"));
}

#[test]
fn o102_pure_intervening_call_is_still_forwarded() {
    // TN / precision guard: the fix must not be so conservative that a
    // provably pure intervening call (no side effects, registry `pure:
    // true`) blocks the forward. `string length` is pure — it cannot
    // write `x` — so it must not gate the same-block scan.
    let src = "set x 5\nstring length hi\nputs $x\n";
    assert!(opt_fires(src, TCL, "O102"));
    assert!(optimised(src, TCL).contains("puts 5"));
}

#[test]
fn o102_side_effecting_intervening_call_is_still_forwarded() {
    // TN / precision guard: `puts` genuinely carries a side effect
    // (`SideEffectTarget::FileIo`), but it has no way to *write* `x` —
    // Tcl's frame-based scoping means an intervening call can only touch
    // `x` via an alias (`global`/`variable`/`upvar`), a trace, or a
    // frame-crossing `uplevel`/`interp eval` body, all of which
    // `run_load_forwarding` checks directly (see its doc comment) rather
    // than gating on every impure intervening statement. Neither applies
    // here, so O102 fires the same as the provably-pure case above.
    let src = "set x 5\nputs \"start\"\nputs $x\n";
    assert!(opt_fires(src, TCL, "O102"));
    assert!(optimised(src, TCL).contains("puts 5"));
}

#[test]
fn o102_unregistered_proc_call_between_def_and_use_still_forwards() {
    // TN / precision guard: a call to a proc the registry knows nothing
    // about cannot reach a caller's private, unaliased local (`x` is
    // never `global`/`variable`/`upvar` declared, and `helper` contains
    // no barrier), so the forward is sound — mirrors
    // `o102_still_forwards_top_level_global_no_proc_touches` in
    // propagation.rs's own unit tests, which locks in the same precision
    // at the whole-module-scan level.
    let src = "proc helper {} {\n    # opaque to the registry\n}\nset x 5\nhelper\nputs $x\n";
    assert!(opt_fires(src, TCL, "O102"));
    assert!(optimised(src, TCL).contains("puts 5"));
}

#[test]
fn o102_uplevel_hash_zero_in_called_proc_is_a_known_gap() {
    // Documented, deliberate scope limitation (see `run_load_forwarding`'s
    // doc comment): `uplevel #0` inside a *called* proc can rewrite a
    // caller-visible variable with no trace and no memory-SSA alias
    // recorded against the *caller's* function unit, so this specific
    // shape is not yet caught. tclsh: prints `99`; the optimiser still
    // (incorrectly) forwards `5`. Asserting the current, known-wrong
    // behaviour here — rather than silently leaving it uncovered — so a
    // future fix flips this assertion instead of a fix regressing
    // silently past an absent test.
    let src = "proc setter {} {\n    uplevel #0 {\n        set x 99\n    }\n}\nset x 5\nsetter\nputs $x\n";
    assert!(optimised(src, TCL).contains("puts 5"));
}

#[test]
fn o100_branch_condition_propagation_blocks_on_variable_trace() {
    // The same silent-miscompile class as O102, but through
    // `branch_folding::propagate_into_branches`'s *own* SCCP-constants
    // projection (a duplicate of `propagation::sccp_constants_for` this fix
    // also deduplicated) rather than `propagation::run_function`'s. A read
    // trace installed by a called proc must still block substituting `x`'s
    // literal into the `if` condition text. tclsh: prints "trace fired"
    // then "yes" — `$x` must survive as a real runtime read.
    let src = "proc onread {name1 name2 op} {\n    puts \"trace fired\"\n}\nproc setup {} {\n    trace add variable ::x read onread\n}\nset x 1\nsetup\nif {$x > 0} {\n    puts yes\n} else {\n    puts no\n}\n";
    assert!(opt_absent(src, TCL, "O100"));
    assert!(optimised(src, TCL).contains("if {$x > 0}"));
}

#[test]
fn o112_constant_branch_elimination_blocks_on_variable_trace() {
    // `structure_elimination`'s O112 (`if {K}` → replace with the
    // constant-true clause, dropping the `if`/condition entirely) reads
    // its own SCCP-lattice projection (`sccp_env_for`) — a second,
    // independent consumer of the same unsafe SCCP data `O100`'s branch
    // path above reads. Collapsing `if {$x} {...} else {...}` to just the
    // `puts yes` branch would delete the runtime read of `$x` outright,
    // permanently silencing the trace. tclsh: prints "trace fired" then
    // "yes" on every run — the `if` must stay a real runtime check so the
    // trace keeps firing.
    // Note: the `else` body's *contents* (`puts no`) used to be lost
    // regardless — see `o107_dead_code_elimination_of_unreachable_branch`
    // below, which now also asserts they survive (SCCP itself is
    // trace-safe, so every trace-blind consumer inherits correctness). This
    // test only asserts O112 itself does not collapse the `if` structure.
    let src = "proc onread {name1 name2 op} {\n    puts \"trace fired\"\n}\nproc setup {} {\n    trace add variable ::x read onread\n}\nset x 1\nsetup\nif {$x} {\n    puts yes\n} else {\n    puts no\n}\n";
    assert!(opt_absent(src, TCL, "O112"));
    assert!(optimised(src, TCL).contains("if {$x}"));
}

#[test]
fn o107_dead_code_elimination_of_unreachable_branch() {
    // Regression: `elimination.rs`'s O107 "unreachable code" deletion is a
    // consumer of SCCP's `executable_blocks`/`constant_branches` facts, same
    // as `propagation`/`branch_folding`/`structure_elimination` above. It
    // used to independently rediscover the "provably unreachable" `else`
    // body (once O112 stopped collapsing the `if` structure itself) and
    // delete its *contents* on a later pass, silently losing the same
    // trace-firing behaviour through a different code path. Now that SCCP's
    // own dataflow (`rust/tcl-compiler/src/sccp.rs`) treats a read of a
    // traced/aliased variable as `Overdefined` — via the whole-module
    // `Module::traced_variables` fact, which also catches a trace installed
    // by a *called* proc like this — every consumer inherits correctness
    // for free: `if {$x}` is no longer provably constant, so O107 must not
    // fire and both arms survive. tclsh: prints "trace fired" then "yes" —
    // the `else` body never runs, but it must still be present in the
    // rewritten source since the compiler cannot prove that statically.
    let src = "proc onread {name1 name2 op} {\n    puts \"trace fired\"\n}\nproc setup {} {\n    trace add variable ::x read onread\n}\nset x 1\nsetup\nif {$x} {\n    puts yes\n} else {\n    puts no\n}\n";
    assert!(opt_absent(src, TCL, "O107"));
    assert!(optimised(src, TCL).contains("puts no"));
}

#[test]
fn o107_true_positive_untraced_branch_still_eliminated() {
    // TP / precision guard: the trace-safety fix above must not make dead
    // code universally survive — the identical shape with no trace anywhere
    // in the module is still provably unreachable and the `else` content is
    // still eliminated. (The identical-shaped structure folds via O112 —
    // higher optimiser priority — which collapses the whole `if`, so O107
    // itself never gets a separate "unreachable else block" to delete here;
    // observable behaviour, not the specific code, is what this guards.)
    // tclsh: prints "yes" only; `puts no` never runs and the rewritten
    // source drops it.
    let src = "set x 1\nif {$x} {\n    puts yes\n} else {\n    puts no\n}\n";
    assert!(!optimised(src, TCL).contains("puts no"));
}

#[test]
fn o107_false_positive_guard_unrelated_variable_name_still_eliminated() {
    // FP guard: a trace on a *different*, merely similarly-named variable
    // (`x2`, sharing the `x` prefix) must not spuriously widen `x`'s own
    // branch to Overdefined — the registry-driven `traced_variables` lookup
    // is an exact-name set membership test, not a prefix/substring match.
    // (See the TP test above for why this folds via O112, not O107, once
    // the trace-guard doesn't apply.) tclsh: prints "yes" only.
    let src = "proc onread {name1 name2 op} {\n    puts \"trace fired\"\n}\nproc setup {} {\n    trace add variable ::x2 read onread\n}\nset x 1\nsetup\nif {$x} {\n    puts yes\n} else {\n    puts no\n}\n";
    assert!(!optimised(src, TCL).contains("puts no"));
}

#[test]
fn o107_dynamic_variable_trace_target_blocks_elimination() {
    // FN guard: a *non-literal* trace target (`trace add variable $name
    // ...`) exercises `Module::has_dynamic_variable_trace` rather than the
    // literal `traced_variables` set — a distinct code path in
    // `is_externally_mutable` that must widen *every* variable, not just
    // named ones, exactly like `crate::gvn::is_pure_command_with_traces`'s
    // `has_dynamic_trace` handling. tclsh: prints "trace fired" then "yes"
    // — `else { puts no }` never runs but must still survive since the
    // compiler cannot enumerate which name(s) `$name` could be.
    let src = "proc onread {name1 name2 op} {\n    puts \"trace fired\"\n}\nproc setup {name} {\n    trace add variable $name read onread\n}\nset x 1\nsetup ::x\nif {$x} {\n    puts yes\n} else {\n    puts no\n}\n";
    assert!(opt_absent(src, TCL, "O107"));
    assert!(optimised(src, TCL).contains("puts no"));
}

// ===========================================================================
// O103 — interprocedural folding
// ===========================================================================

#[test]
fn o103_interprocedural_folding() {
    // tclsh: pi == 3.
    assert!(optimised("proc pi {} { return 3 }\nset v [pi]\n", TCL).contains("set v 3"));
    // tclsh: double 21 == 42.
    assert!(
        optimised(
            "proc double {x} { return [expr {$x * 2}] }\nset v [double 21]\n",
            TCL
        )
        .contains("set v 42")
    );
    // tclsh: id 99 == 99.
    assert!(
        optimised("proc id {x} { return $x }\nset a 99\nset v [id $a]\n", TCL).contains("set v 99")
    );
    // tclsh: bump 5 (incr x; return x) == 6.
    assert!(
        optimised("proc bump {x} { incr x; return $x }\nset v [bump 5]\n", TCL).contains("set v 6")
    );
    // tclsh: util::add 10 20 == 30.
    assert!(optimised(
        "namespace eval util {\n    proc add {a b} { return [expr {$a + $b}] }\n}\nset v [util::add 10 20]\n",
        TCL
    )
    .contains("set v 30"));

    // Impure proc (puts) is not folded.
    assert!(opt_absent(
        "proc effect {} { puts hi; return 1 }\nset v [effect]\n",
        TCL,
        "O103"
    ));
    // Recursive proc is not folded (infinite-inline risk).
    assert!(opt_absent(
        "proc fact {n} {\n    if {$n <= 1} { return 1 }\n    return [expr {$n * [fact [expr {$n - 1}]]}]\n}\nset v [fact 5]\n",
        TCL,
        "O103"
    ));
}

// ===========================================================================
// O104 — fold string build chains
// ===========================================================================

#[test]
fn o104_string_build_chain() {
    // tclsh: {Hello}+{ }+World ⇒ "Hello World".
    assert_eq!(
        optimised("set msg {Hello}\nappend msg { }\nappend msg World", TCL),
        "set msg {Hello World}"
    );

    // A multi-append chain folds (O104).
    assert!(opt_fires(
        "set msg {a}\nappend msg {b}\nappend msg {c}\nappend msg {d}",
        TCL,
        "O104"
    ));
    // set + single append is a valid 2-write chain.
    assert!(opt_fires(
        "set msg {Hello}\nappend msg { World}",
        TCL,
        "O104"
    ));
    // Empty initial value still chains.
    assert!(opt_fires(
        "set msg {}\nappend msg hello\nappend msg { world}",
        TCL,
        "O104"
    ));

    // Dynamic word blocks the fold.
    assert!(opt_absent("set msg {Hello}\nappend msg $name", TCL, "O104"));
    // A read between writes blocks the chain.
    assert!(opt_absent(
        "set msg {Hello}\nputs $msg\nappend msg { World}",
        TCL,
        "O104"
    ));
    // An eval/uplevel barrier breaks the chain.
    assert!(opt_absent(
        "set msg {Hello}\neval {puts hi}\nappend msg { World}",
        TCL,
        "O104"
    ));

    // Gap (sound): the optimiser does not fold a chain that straddles an
    // intervening statement (`puts ok`) into `set msg {Hello World}`; it leaves
    // `append` intact. tclsh: both forms bind msg to "Hello World", so leaving it
    // is sound.
    assert!(
        optimised("set msg {Hello}\nputs ok\nappend msg { World}", TCL)
            .contains("append msg { World}")
    );
    assert!(opt_absent(
        "set msg {Hello}\nputs ok\nappend msg { World}",
        TCL,
        "O104"
    ));
}

// ===========================================================================
// O100 / O105 — propagate constants into command refs and strings
// ===========================================================================

#[test]
fn o100_o105_const_ref_propagation() {
    // tclsh: x=42 ⇒ `puts 42`.
    assert_eq!(optimised("set x 42\nputs $x", TCL), "puts 42");
    // Two reads both fold. tclsh: x=5 ⇒ two `puts 5`.
    assert_eq!(
        optimised("set x 5\nputs $x\nputs $x", TCL),
        "puts 5\nputs 5"
    );
    // Through a chain. tclsh: a=1 ⇒ b==2 ⇒ `puts 2`.
    assert_eq!(
        optimised("set a 1\nset b [expr {$a + 1}]\nputs $b", TCL),
        "puts 2"
    );

    // String interpolation: $x inside "...". tclsh: x=5 ⇒ "val=5".
    // The string-interp fold is labelled inline as O100 (+ O102/O109 DSE) rather
    // than O105. Same observable output.
    let s = "set x 5\nputs \"val=$x\"";
    assert!(optimised(s, TCL).contains("puts \"val=5\""));
    assert!(opt_fires(s, TCL, "O100"));

    // Multiple vars in a string. tclsh: a=10,b=20 ⇒ "10 20".
    assert!(optimised("set a 10\nset b 20\nputs \"$a $b\"", TCL).contains("puts \"10 20\""));

    // ${x} standalone word — no trailing brace. tclsh: x=7 ⇒ `puts 7`.
    let braced = optimised("set x 7\nputs ${x}", TCL);
    assert!(!braced.contains("puts 7}"));
    assert!(braced.contains("puts 7"));

    // ${x} inside a string. tclsh: `puts "7"`.
    let bstr = optimised("set x 7\nputs \"${x}\"", TCL);
    assert!(!bstr.contains("\"7}\""));
    assert!(bstr.contains('7'));

    // A *safe* string constant folds into a string. tclsh: x=hello ⇒ "val=hello".
    assert!(optimised("set x hello\nputs \"val=$x\"", TCL).contains("puts \"val=hello\""));
    // A value with a space folds into the string. tclsh: "a b" ⇒ "val=a b".
    assert!(optimised("set x \"a b\"\nputs \"val=$x\"", TCL).contains("puts \"val=a b\""));

    // An *unsafe* value (contains `$` / `[`) must NOT fold into a string — it
    // would inject a fresh substitution. (Structural: no O100/O105 fires.)
    assert!(opt_absent("set x {$y}\nputs \"val=$x\"", TCL, "O100"));
    assert!(opt_absent("set x {$y}\nputs \"val=$x\"", TCL, "O105"));
    assert!(opt_absent("set x {[exit]}\nputs \"val=$x\"", TCL, "O100"));

    // Combined string-interp + expr fold + DSE. tclsh: x=5 ⇒ "x=5" and y==6.
    assert_eq!(
        optimised(
            "set x 5\nset y [expr {$x + 1}]\nputs \"x=$x\"\nputs $y",
            TCL
        ),
        "puts \"x=5\"\nputs 6"
    );

    // Dynamic value is not propagated.
    assert!(opt_absent("set x [clock seconds]\nputs $x", TCL, "O100"));
    // Array reference is not propagated (shape guard).
    let arr = "set x $arr(key)\nputs $x";
    assert_eq!(optimised(arr, TCL), arr);
    assert!(opt_absent(arr, TCL, "O100"));
}

// ===========================================================================
// O107 / O108 / O109 — dead code & dead store elimination
// ===========================================================================

#[test]
fn o107_dead_code_in_false_branch() {
    // `if {0}` body is unreachable; trailing statement survives.
    let out = optimised("if {0} {\n    puts dead\n}\nputs alive\n", TCL);
    assert!(!out.contains("puts dead"));
    assert!(out.contains("puts alive"));

    // Code after `return` in a proc: must not crash (Rust may or may not emit
    // O107). Assert only that optimisation completes (proc preserved).
    let after_ret = "proc foo {} {\n    return 1\n    puts never\n}\n";
    let _ = opt_codes(after_ret, TCL); // no panic
}

#[test]
fn o108_aggressive_dce() {
    // tclsh: a=1, b reassigned to 10 ⇒ b==10. The dead chain is removed via
    // O108/O109 (either code may carry it).
    let s = "set a 1\nset b [expr {$a + 1}]\nset b 10\nputs $b";
    assert!(optimised(s, TCL).contains("puts 10"));
    assert!(opt_any_of(s, TCL, &["O108", "O109"]));

    // A consumed value is not eliminated. tclsh: a=1 ⇒ b==2 ⇒ `puts 2`.
    assert!(optimised("set a 1\nset b [expr {$a + 1}]\nputs $b", TCL).contains("puts 2"));

    // Multi-step transitively-dead chain. tclsh: final c==99 ⇒ `puts 99`.
    let multi = "set a 1\nset b [expr {$a + 1}]\nset c [expr {$b + 1}]\nset c 99\nputs $c\n";
    assert!(optimised(multi, TCL).contains("puts 99"));
    assert!(opt_any_of(multi, TCL, &["O108", "O109"]));
}

#[test]
fn o109_dead_store_elimination() {
    // First store dead (O109). tclsh: a==2.
    let s = "set a 1\nset a 2\nputs $a";
    assert!(opt_fires(s, TCL, "O109"));
    assert!(optimised(s, TCL).contains("puts 2"));

    // tclsh: x final 200.
    assert!(optimised("set x 100\nset x 200\nputs $x", TCL).contains("puts 200"));

    // eval reads all vars — the store feeding `[eval $s]` survives (not DSE'd).
    let esc = "set n [eval $s]\nset n 1\nputs $n";
    assert_eq!(
        optimised(esc, TCL).lines().next().unwrap(),
        "set n [eval $s]"
    );

    // A dead store inside a proc fires O109.
    assert!(opt_fires(
        "proc foo {} {\n    set x 1\n    set x 2\n    return $x\n}\n",
        TCL,
        "O109"
    ));

    // Call-by-name suppression: a local passed BY NAME to a proc with upvar
    // VAR_READ/VAR_WRITE traits is indirectly read — its store must survive
    // (no O109/O126). Deleting it would feed an uninitialised name to the callee.
    let byname = "proc reader {name} { upvar 1 $name local; puts $local }\nproc f {} {\n    set x \"hello\"\n    reader x\n}\n";
    assert!(opt_absent(byname, TCL, "O109"));
    assert!(opt_absent(byname, TCL, "O126"));

    // TP control: a dead store on a DIFFERENT (not passed-by-name) var still
    // fires O109.
    let byname_tp = "proc reader {name} { upvar 1 $name local; puts $local }\nproc f {} {\n    set unrelated \"discarded\"\n    set unrelated \"actual\"\n    set y \"passed\"\n    reader y\n    return $unrelated\n}\n";
    assert!(opt_fires(byname_tp, TCL, "O109"));
}

// ===========================================================================
// O110 — InstCombine: reassociation / identity / annihilator
// ===========================================================================

#[test]
fn o110_reassociation() {
    // tclsh sweep: $a + 1 + 2 == $a + 3 (all $a).
    assert!(optimised("set v [expr {$a + 1 + 2}]", TCL).contains("$a + 3"));
    // tclsh sweep: $a * 2 * 3 == $a * 6.
    assert!(optimised("set v [expr {$a * 2 * 3}]", TCL).contains("$a * 6"));
    // tclsh sweep: $a + 3 - 1 == $a + 2.
    assert!(optimised("set v [expr {$a + 3 - 1}]", TCL).contains("$a + 2"));
    // Real fold through reassoc at top level: ($a + 1) + 2 flattens to $a + 3.
    // tclsh sweep: ($a + 1) + 2 == $a + 3.
    assert!(optimised("set v [expr {($a + 1) + 2}]", TCL).contains("$a + 3"));
    assert!(opt_fires("set v [expr {($a + 1) + 2}]", TCL, "O110"));
    // NOTE (gap, sound): the same reassoc inside a `proc ... return [expr {...}]`
    // value position is NOT folded (no code fires); leaving it is sound. Given
    // `proc f {a} { return [expr {($a + 1) + 2}] }` no optimisation fires.
    assert!(opt_codes("proc f {a} { return [expr {($a + 1) + 2}] }", TCL).is_empty());
}

#[test]
fn o110_identity_and_annihilator() {
    // Identity/annihilator drops need a provably-INT operand (D5-O110) — wrap in
    // `int_x`. tclsh sweep (x≥0): x**0==1, x**1==x, x<<0==x, x>>0==x, x&0==0,
    // x|0==x, x^0==x, x%1==0, x*0==0, x*1==x, x+0==x, x-0==x, x/1==x.
    assert!(optimised(&int_x("set v [expr {$x ** 0}]"), TCL).contains("set v 1")); // tclsh: 1
    assert!(opt_fires(&int_x("set v [expr {$x ** 1}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x << 0}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x >> 0}]"), TCL, "O110"));
    assert!(optimised(&int_x("set v [expr {$x & 0}]"), TCL).contains("set v 0")); // tclsh: 0
    assert!(opt_fires(&int_x("set v [expr {$x | 0}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x ^ 0}]"), TCL, "O110"));
    assert!(optimised(&int_x("set v [expr {$x % 1}]"), TCL).contains("set v 0")); // tclsh: 0
    assert!(optimised(&int_x("set v [expr {$x * 0}]"), TCL).contains("set v 0")); // tclsh: 0
    assert!(opt_fires(&int_x("set v [expr {$x * 1}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x + 0}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x - 0}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {$x / 1}]"), TCL, "O110"));
    assert!(optimised(&int_x("set v [expr {$x % 1}]"), TCL).contains("set v 0"));

    // Left-side identity (commutativity). tclsh sweep: 0|x==x, 0^x==x.
    assert!(opt_fires(&int_x("set v [expr {0 | $x}]"), TCL, "O110"));
    assert!(opt_fires(&int_x("set v [expr {0 ^ $x}]"), TCL, "O110"));
}

#[test]
fn o110_logical_identities() {
    // Logical annihilators/identities do not require an INT proof (boolean
    // context). tclsh sweep: x&&0==0, x||1==1, 0&&x==0, 1||x==1.
    assert!(optimised("set v [expr {$x && 0}]", TCL).contains("set v 0")); // tclsh: 0
    assert!(optimised("set v [expr {$x || 1}]", TCL).contains("set v 1")); // tclsh: 1
    assert!(optimised("set v [expr {0 && $x}]", TCL).contains("set v 0")); // tclsh: 0
    assert!(optimised("set v [expr {1 || $x}]", TCL).contains("set v 1")); // tclsh: 1
    // x&&1, x||0, 1&&x, 0||x simplify (O110) without becoming a constant.
    assert!(opt_fires("set v [expr {$x && 1}]", TCL, "O110"));
    assert!(opt_fires("set v [expr {$x || 0}]", TCL, "O110"));
    assert!(opt_fires("set v [expr {1 && $x}]", TCL, "O110"));
    assert!(opt_fires("set v [expr {0 || $x}]", TCL, "O110"));
}

#[test]
fn o110_whitespace_noise_suppressed() {
    // Whitespace-only differences must NOT fire O110 (the user's spacing is not a
    // defect). All confirmed no-O110 in Rust.
    for src in [
        "set v [expr {$x-1}]",
        "set v [expr {$x*$y}]",
        "set v [expr {$a*$b/$c}]",
        "set v [expr {$secure ? 6697: 6667}]",
        "set v [expr {$tagnumber >>7}]",
        "proc f {x} { if {$x<0} { return -1 }; return 1 }",
        "proc f {x} { while {$x>0} { incr x -1 } }",
        // Pure commutative reorder — no win.
        "proc f {x} { return [expr {1 + $x}] }",
        "proc f {x} { return [expr {2 * $x}] }",
        // Mixed bitwise/shift parens are kept for readability (CERT EXP00-C).
        "set v [expr {($x >> 16) & 0xFF}]",
        // Gaps (sound): `$x - -1` ⇒ `$x + 1` and the same-op bitwise chain
        // `(a & b) & c` could fire O110 but NEITHER does (no rewrite) — leaving
        // both unchanged is sound.
        "proc f {x} { return [expr {$x - -1}] }",
        "set v [expr {($a & $b) & $c}]",
    ] {
        assert!(opt_absent(src, TCL, "O110"), "spurious O110: {src}");
    }

    // Genuine structural changes that DO still fire O110.
    // Additive identity `$x + 0` ⇒ `$x` (INT-proven).
    assert!(opt_fires(&int_x("set v [expr {$x + 0}]"), TCL, "O110"));
    // Boolean simplify `($action && 1) ? ...` ⇒ `$action ? ...`.
    assert!(opt_fires(
        "set v [expr {($action && 1) ? \"on\" : \"off\"}]",
        TCL,
        "O110"
    ));
}

#[test]
fn o110_self_comparison_tautologies() {
    // tclsh sweep (any defined $x, string or numeric — `==`/`<` use the string
    // fallback): x==x ⇒ 1, x!=x ⇒ 0, x<=x ⇒ 1, x>=x ⇒ 1, x<x ⇒ 0, x>x ⇒ 0.
    assert!(optimised("set v [expr {$x == $x}]", TCL).contains("set v 1"));
    assert!(optimised("set v [expr {$x != $x}]", TCL).contains("set v 0"));
    assert!(optimised("set v [expr {$x <= $x}]", TCL).contains("set v 1"));
    assert!(optimised("set v [expr {$x >= $x}]", TCL).contains("set v 1"));
    assert!(optimised("set v [expr {$x < $x}]", TCL).contains("set v 0"));
    assert!(optimised("set v [expr {$x > $x}]", TCL).contains("set v 0"));
    // String self-comparison. tclsh: x eq x ⇒ 1, x ne x ⇒ 0 (any $x).
    assert!(optimised("set v [expr {$x eq $x}]", TCL).contains("set v 1"));
    assert!(optimised("set v [expr {$x ne $x}]", TCL).contains("set v 0"));
    // x ^ x ⇒ 0 — needs an INT proof (xor errors on non-numeric strings); the
    // `int_x` wrapper supplies it. tclsh sweep (x≥0): x^x == 0.
    assert!(optimised(&int_x("set v [expr {$x ^ $x}]"), TCL).contains("set v 0"));

    // FIXED (was a miscompile): bare `set v [expr {$x - $x}]` on an UNTYPED $x
    // must NOT fold to `set v 0`. tclsh proves the original RAISES an error when
    // $x is a non-numeric string:
    //   set x hello; expr {$x - $x}  ⇒  "can't use non-numeric string as operand
    //   of \"-\"" (8.6) / "cannot use non-numeric string ... as left operand"
    //   (9.0)  — catch ⇒ 1
    // so folding to 0 would silently swallow that error. Subtraction is
    // arithmetic-only, unlike `==`/`<` (string fallback), so this is not folded.
    assert!(
        !optimised("set v [expr {$x - $x}]", TCL).contains("set v 0"),
        "untyped $x - $x must not fold to 0 (arithmetic op errors on non-numeric)"
    );
    // …but with a provably-numeric operand the fold IS sound. tclsh sweep (x≥0):
    // x-x == 0. The `int_x` wrapper makes $x provably numeric.
    assert!(optimised(&int_x("set v [expr {$x - $x}]"), TCL).contains("set v 0"));
}

#[test]
fn o110_unary_and_not_inversions() {
    // tclsh: !!(bool) collapses (==/!=/< are already boolean).
    assert!(optimised("set v [expr {!!($a == $b)}]", TCL).contains("$a == $b"));
    // tclsh sweep: !(a==b) ⇒ a!=b, !(a<b) ⇒ a>=b, !(a>=b) ⇒ a<b, !(a>b) ⇒ a<=b,
    // !(a<=b) ⇒ a>b, !(a!=b) ⇒ a==b.
    assert!(optimised("set v [expr {!($a == $b)}]", TCL).contains("$a != $b"));
    assert!(optimised("set v [expr {!($a < $b)}]", TCL).contains("$a >= $b"));
    assert!(optimised("set v [expr {!($a >= $b)}]", TCL).contains("$a < $b"));
    assert!(optimised("set v [expr {!($a > $b)}]", TCL).contains("$a <= $b"));
    assert!(optimised("set v [expr {!($a <= $b)}]", TCL).contains("$a > $b"));
    assert!(optimised("set v [expr {!($a != $b)}]", TCL).contains("$a == $b"));
    // String comparison inversions. tclsh: !(a eq b) ⇒ a ne b, !(a ne b) ⇒ a eq b.
    assert!(optimised("set v [expr {!($a eq $b)}]", TCL).contains("$a ne $b"));
    assert!(optimised("set v [expr {!($a ne $b)}]", TCL).contains("$a eq $b"));

    // Double-not in an `if` condition simplifies (O110). tclsh: !!x ⇒ x (bool ctx).
    assert!(opt_fires("if {!!$x} { puts yes }", TCL, "O110"));
    assert!(optimised("if {!!$x} { puts yes }", TCL).contains("if {$x}"));

    // INT-proven unary folds: ~~x ⇒ x, -(-x) ⇒ x, +x ⇒ x. tclsh sweep (x≥0).
    assert!(optimised(&int_x("set v [expr {~~$x}]"), TCL).contains("set v [expr {$x}]"));
    assert!(optimised(&int_x("set v [expr {-(-$x)}]"), TCL).contains("$x"));
    assert!(opt_fires(&int_x("set v [expr {+$x}]"), TCL, "O110"));

    // NOTE on gaps (sound to leave unchanged):
    //  - `!($a in $b)` ⇒ `$a ni $b` and `!($a ni $b)` ⇒ `$a in $b` — in/ni are
    //    not inverted.
    //  - `-(5)` ⇒ `-5` literal negation — not folded (stays `-(5)`).
}

#[test]
fn o110_de_morgan() {
    // tclsh sweep (a,b ∈ {0,1,5}): !($a && $b) == !$a || !$b; !($a || $b) ==
    // !$a && !$b; the 4-var chain matches its inversion (tclsh-proven).
    assert!(optimised("set v [expr {!($a && $b)}]", TCL).contains("!$a || !$b"));
    assert!(optimised("set v [expr {!($a || $b)}]", TCL).contains("!$a && !$b"));
    assert!(
        optimised("set v [expr {!($a == $b && $c < $d)}]", TCL).contains("$a != $b || $c >= $d")
    );

    // De Morgan inside an `if` condition is carried under O110 or O113 (the
    // if-condition rewrite path is owned by the strength-reduce pass). The
    // rewrite text matches tclsh either way.
    let in_if = "if {!($x && $y)} { puts yes }";
    assert!(optimised(in_if, TCL).contains("!$x || !$y"));
    assert!(opt_any_of(in_if, TCL, &["O110", "O113"]));
}

#[test]
fn o110_ternary_and_double_not() {
    // Constant-condition ternary simplifies (O110). tclsh: 1?a:b==a, 0?a:b==b.
    assert!(opt_fires("set v [expr {1 ? $a : $b}]", TCL, "O110"));
    assert!(opt_fires("set v [expr {0 ? $a : $b}]", TCL, "O110"));
    // Double-not in an `if`. (Structural: O110 fires.)
    assert!(opt_fires("if {!!$x} { puts yes }", TCL, "O110"));
    // De Morgan inside an `if` is reported under O110/O113.
    assert!(opt_any_of(
        "if {!($x && $y)} { puts yes }",
        TCL,
        &["O110", "O113"]
    ));
    // Boolean-context ternary `($a > $b) ? 1 : 0` simplifies (O110).
    assert!(opt_fires(
        "if {($a > $b) ? 1 : 0} { puts yes }",
        TCL,
        "O110"
    ));

    // NOTE on gaps (sound): `$x ? 0 : 1` ⇒ `!$x`, `$c ? $a : $a` (identical
    // branches), and `!$c ? $a : $b` ⇒ `$c ? $b : $a` (cond flip) are not folded.
    assert!(opt_absent("set v [expr {$x ? 0 : 1}]", TCL, "O110"));
}

#[test]
fn o110_no_instcombine_with_command_subst() {
    // x+0=x is unsafe when x has side effects ([clock seconds]). No O110.
    assert!(opt_absent(
        "set v [expr {[clock seconds] + 0}]",
        TCL,
        "O110"
    ));
}

// ===========================================================================
// O112 — structure elimination
// ===========================================================================

#[test]
fn o112_if_elimination() {
    // tclsh: `if {1} {set x 1}` runs the body ⇒ unwrap.
    let it = optimised("if {1} {\n    set x 1\n}", TCL);
    assert!(!it.contains("if"));
    assert!(it.contains("set x 1"));

    // `if {0}` deletes the body; trailing statement survives.
    let iff = optimised("if {0} {\n    set x 1\n}\nputs alive", TCL);
    assert!(!iff.contains("set x 1"));
    assert!(iff.contains("puts alive"));

    // `if {0} ... else ...` keeps the else body. tclsh: else runs.
    let ie = optimised("if {0} {\n    set x 1\n} else {\n    set y 2\n}", TCL);
    assert!(!ie.contains("set x 1"));
    assert!(ie.contains("set y 2"));

    // elseif chain selects the first true clause. tclsh: `set b 2` runs.
    let el = optimised(
        "if {0} {\n    set a 1\n} elseif {1} {\n    set b 2\n} else {\n    set c 3\n}",
        TCL,
    );
    assert!(el.contains("set b 2"));
    assert!(!el.contains("set a 1"));
    assert!(!el.contains("set c 3"));

    // All conditions false, no else ⇒ whole thing gone, trailing survives (O112).
    let all_false = "if {0} { set a 1 } elseif {0} { set b 2 }\nputs done";
    assert!(optimised(all_false, TCL).contains("puts done"));
    assert!(opt_fires(all_false, TCL, "O112"));

    // Runtime condition `if {$x}` is untouched.
    assert!(opt_absent("if {$x} {\n    set y 1\n}", TCL, "O112"));
}

#[test]
fn o112_while_for_elimination() {
    // `while {0}` deletes the loop.
    let wf = "while {0} {\n    puts looping\n}\nputs done";
    assert!(!optimised(wf, TCL).contains("puts looping"));
    assert!(optimised(wf, TCL).contains("puts done"));
    assert!(opt_fires(wf, TCL, "O112"));

    // `while {1}` is an infinite loop — NOT eliminated by O112. (The constant `1`
    // condition folds as O101 but the loop is never deleted; O112 does not fire.)
    assert!(opt_absent("while {1} {\n    break\n}", TCL, "O112"));

    // `for {init} {0} ...` keeps only the init.
    let ff = "for {set i 0} {0} {incr i} {\n    puts looping\n}";
    assert!(!optimised(ff, TCL).contains("puts looping"));
    assert!(optimised(ff, TCL).contains("set i 0"));
    assert!(opt_fires(ff, TCL, "O112"));

    // `for {} {0} {} ...` deletes entirely.
    let ffe = "for {} {0} {} {\n    puts looping\n}\nputs done";
    assert!(!optimised(ffe, TCL).contains("puts looping"));
    assert!(optimised(ffe, TCL).contains("puts done"));
    assert!(opt_fires(ffe, TCL, "O112"));
}

#[test]
fn o112_switch_elimination() {
    // tclsh: switch abc {abc {1} def {2}} ⇒ arm 1.
    let lit = optimised(
        "switch abc {\n    abc { set x 1 }\n    def { set y 2 }\n}",
        TCL,
    );
    assert!(lit.contains("set x 1"));
    assert!(!lit.contains("set y 2"));

    // No literal match ⇒ default. tclsh: switch xyz {...default {2}} ⇒ 2.
    let def = optimised(
        "switch xyz {\n    abc { set a 1 }\n    default { set b 2 }\n}",
        TCL,
    );
    assert!(!def.contains("set a 1"));
    assert!(def.contains("set b 2"));

    // No match, no default ⇒ whole switch deleted.
    let nm = optimised("switch xyz {\n    abc { set a 1 }\n}\nputs done", TCL);
    assert!(!nm.contains("set a 1"));
    assert!(nm.contains("puts done"));

    // Dynamic subject is untouched.
    assert!(opt_absent(
        "switch $x {\n    abc { set a 1 }\n}",
        TCL,
        "O112"
    ));
}

#[test]
fn o112_nested_via_multipass() {
    // First pass unwraps `if {1}`; the fixpoint helper eliminates the inner
    // `if {0}` dead block.
    let nested = "if {1} {\n    if {0} {\n        set dead 1\n    }\n    set alive 2\n}";
    assert!(optimised(nested, TCL).contains("set alive 2"));
    let registry = registry_for_dialect(TCL);
    let (fixed, _) = optimise_source_multipass(nested, registry, Some(TCL), 10);
    assert!(!fixed.contains("set dead 1"));
    assert!(fixed.contains("set alive 2"));
}

// ===========================================================================
// O113 — strength reduction
// ===========================================================================

#[test]
fn o113_strength_reduction() {
    // tclsh sweep: x ** 2 == x * x.
    let p = optimised("if {$x ** 2} {}", TCL);
    assert!(!p.contains("**"));
    assert!(p.contains('*'));
    assert!(opt_fires("if {$x ** 2} {}", TCL, "O113"));

    // tclsh sweep (x≥0): x % 8 == x & 7, x % 16 == x & 15, x % 2 == x & 1.
    let m8 = optimised("if {$x % 8} {}", TCL);
    assert!(m8.contains('&'));
    assert!(!m8.contains('%'));
    assert!(optimised("if {$x % 16} {}", TCL).contains("& 15")); // tclsh: x%16 == x&15
    assert!(optimised("if {$x % 2} {}", TCL).contains("& 1")); // tclsh: x%2 == x&1

    // Non-power-of-two modulus is not reduced.
    assert!(opt_absent("if {$x % 7} {}", TCL, "O113"));
    // x ** 3 is not reduced (only x ** 2).
    assert!(opt_absent("if {$x ** 3} {}", TCL, "O113"));
    // Command subst present ⇒ no reduction (side effects).
    assert!(opt_absent("if {[clock seconds] ** 2} {}", TCL, "O113"));
}

// ===========================================================================
// O114 — incr idiom recognition (needs SSA-known INT type, kept live)
// ===========================================================================

#[test]
fn o114_incr_idiom() {
    // D5-O114: the loop-counter pattern makes `x` INT-typed AND live (read via
    // `puts $x`), so the rewrite is sound. tclsh: `set x N; set x [expr {$x+1}]`
    // is the same as `incr x` for INT x.
    let add1 = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {$x + 1}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(optimised(add1, TCL).contains("incr x"));
    assert!(opt_fires(add1, TCL, "O114"));

    let add5 = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {$x + 5}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(optimised(add5, TCL).contains("incr x 5")); // tclsh: x+5 == incr x 5

    let sub3 = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {$x - 3}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(optimised(sub3, TCL).contains("incr x -3")); // tclsh: x-3 == incr x -3

    // Commutative `N + $x` also matches.
    let comm = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {1 + $x}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(optimised(comm, TCL).contains("incr x"));

    // Multiplication is not an incr idiom.
    let mul = "proc foo {n} {\n  for {set x 0} {$x < $n} {incr x} {\n    set x [expr {$x * 2}]\n    puts $x\n  }\n}\nfoo 3\n";
    assert!(opt_absent(mul, TCL, "O114"));

    // Unknown-type param must NOT be incr-rewritten (could be a float at runtime;
    // tclsh: `set x [expr {$x + 1}]` on 1.5 ⇒ 2.5, but `incr x` errors).
    assert!(opt_absent(
        "proc foo {x} {\n    set x [expr {$x + 1}]\n}\n",
        TCL,
        "O114"
    ));
    // Different target var ⇒ no incr.
    assert!(opt_absent(
        "proc foo {x} {\n    set y [expr {$x + 1}]\n}\n",
        TCL,
        "O114"
    ));
}

// ===========================================================================
// O115 — redundant nested expr elimination
// ===========================================================================

#[test]
fn o115_nested_expr_unwrap() {
    // `if {[expr {...}]}` — the nested expr is redundant in a condition. tclsh:
    // `if {[expr {$x + 1}]}` == `if {$x + 1}`.
    let in_if = "if {[expr {$x + 1}]} {}";
    assert!(!optimised(in_if, TCL).contains("[expr"));
    assert!(opt_fires(in_if, TCL, "O115"));

    // `[expr {[expr {...}]}]` in a return value unwraps the inner. tclsh:
    // `[expr {[expr {$x * 2}]}]` == `[expr {$x * 2}]`.
    let in_ret = "proc double_expr {x} {\n    return [expr {[expr {$x * 2}]}]\n}";
    assert!(optimised(in_ret, TCL).contains("return [expr {$x * 2}]"));
    assert!(opt_fires(in_ret, TCL, "O115"));

    // `[string length $s]` is not an expr — not unwrapped.
    assert!(opt_absent("if {[string length $s]} {}", TCL, "O115"));

    // NOTE on gaps (sound): `while {[expr {...}]}` and
    // `set y [expr {[expr {...}]}]` are not unwrapped (only if/return paths fire).
}

// ===========================================================================
// O116 — constant list folding
// ===========================================================================

#[test]
fn o116_list_folding() {
    // [list a] folds away the `[list` call (via O116 or O100 propagation).
    // tclsh: [list a] == "a".
    let single = "set x [list a]\nputs $x";
    assert!(!optimised(single, TCL).contains("[list"));
    assert!(opt_any_of(single, TCL, &["O116", "O100"]));

    // Empty list folds (O116, or SCCP O100/O109).
    assert!(opt_any_of(
        "set x [list]\nputs $x",
        TCL,
        &["O116", "O100", "O109"]
    ));

    // Multi-element [list a b c] must NOT fold via O116 (intrep shimmer); it may
    // be propagated as a braced word via O100, which is semantically identical
    // (tclsh: [list a b c] == {a b c}). Key invariant: no O116.
    assert!(opt_absent("set x [list a b c]\nputs $x", TCL, "O116"));
    // List with a variable element ⇒ not folded.
    assert!(opt_absent("set x [list $a b c]\nputs $x", TCL, "O116"));
    // Element with a space ⇒ not O116.
    assert!(opt_absent("set x [list \"a b\" c]\nputs $x", TCL, "O116"));
}

// ===========================================================================
// O117 — string length zero-check simplification
// ===========================================================================

#[test]
fn o117_strlen_zero_check() {
    // tclsh sweep: ([string length $s] == 0) <=> ($s eq ""); != 0 <=> ne "".
    assert!(opt_fires("if {[string length $s] == 0} {}", TCL, "O117"));
    assert!(opt_fires("if {[string length $s] != 0} {}", TCL, "O117"));
    // Swapped `0 == [string length $s]` also simplifies.
    assert!(opt_fires("if {0 == [string length $s]} {}", TCL, "O117"));

    // [string length $s] == 5 is NOT simplified.
    assert!(opt_absent("if {[string length $s] == 5} {}", TCL, "O117"));

    // Gaps (sound): the `> 0`, `<= 0`, and swapped `0 < [string length $s]` forms
    // are equivalent to `ne ""` / `eq ""` but only the `== 0` / `!= 0` / `0 ==`
    // forms are simplified. Confirm the conservative no-op for `> 0` (left
    // unchanged is sound).
    assert!(opt_absent("if {[string length $s] > 0} {}", TCL, "O117"));
    assert!(opt_absent("if {[string length $s] <= 0} {}", TCL, "O117"));
    assert!(opt_absent("if {0 < [string length $s]} {}", TCL, "O117"));
}

// ===========================================================================
// O118 — constant lindex folding
// ===========================================================================

#[test]
fn o118_lindex_folding() {
    // tclsh-verified each fold result.  Since issue #1134 the registry
    // fold also enters the SCCP lattice, so the `$x` read downstream
    // propagates too (`puts $x` → `puts a`) — semantically identical
    // (tclsh: both print `a`), one hop further folded.
    assert_eq!(
        optimised("set x [lindex {a b c} 0]\nputs $x", TCL),
        "set x a\nputs a"
    ); // tclsh: a
    assert_eq!(
        optimised("set x [lindex {a b c} 1]\nputs $x", TCL),
        "set x b\nputs b"
    ); // tclsh: b
    assert_eq!(
        optimised("set x [lindex {x y z} end]\nputs $x", TCL),
        "set x z\nputs z"
    ); // tclsh: z
    assert_eq!(
        optimised("set x [lindex {a b c d} end-1]\nputs $x", TCL),
        "set x c\nputs c"
    ); // tclsh: c

    // Out-of-range index folds (to empty); Rust emits O118 (or SCCP O100/O109).
    assert!(opt_any_of(
        "set x [lindex {a b} 5]\nputs $x",
        TCL,
        &["O118", "O100", "O109"]
    ));

    // Variable index ⇒ not folded.
    assert!(opt_absent(
        "set x [lindex {a b c} $i]\nputs $x",
        TCL,
        "O118"
    ));

    // MORE precise (tclsh-proven sound): one might expect `[lindex {{a b} c} 0]`
    // to NOT fold. tclsh disproves the caution: `[lindex {{a b} c} 0]` == "a b",
    // and it folds to `set x {a b}` — `set x {a b}; puts $x` prints "a b",
    // byte-identical to the original. Assert the sound rewrite rather than an
    // over-cautious no-op.
    assert_eq!(
        optimised("set x [lindex {{a b} c} 0]\nputs $x", TCL),
        // tclsh: [lindex {{a b} c} 0] == "a b"; the lattice fold (#1134)
        // then propagates the read as the brace-quoted single word, which
        // prints identically (`puts {a b}` == `puts $x` == "a b").
        "set x {a b}\nputs {a b}"
    );
}

// ===========================================================================
// O119 — multi-set packing
// ===========================================================================

#[test]
fn o119_multi_set_packing() {
    // OMISSION (as in optimiser.rs): with an `eval {$a $b $c}` barrier the
    // constants are forwarded THROUGH the braced `eval {...}` literal (O102/O109)
    // — `eval {1 2 3}` — so no surviving stores remain and O119 never fires.
    // tclsh: `set a 1; set b 2; set c 3; eval {$a $b $c}` and `eval {1 2 3}` are
    // identical, so the fold is sound. Assert the packing-disabled invariants.

    // Tcl 9.0: individual `set` is faster ⇒ O119 must not fire.
    assert!(opt_absent(
        "set a 1\nset b 2\nset c 3\nputs \"$a $b $c\"",
        "tcl9.0",
        "O119"
    ));
    // Too few consecutive sets ⇒ no packing.
    assert!(opt_absent("set a 1\nset b 2\neval {$a $b}", TCL, "O119"));
    // The eval-barrier form is folded rather than packed. tclsh: identical value.
    assert_eq!(
        optimised("set a 1\nset b 2\nset c 3\neval {$a $b $c}", TCL),
        "eval {1 2 3}"
    );
}

// ===========================================================================
// O120 — prefer eq/ne for string comparisons
// ===========================================================================

#[test]
fn o120_string_compare_eq_ne() {
    // tclsh sweep: $a == "hello" <=> $a eq "hello" ("hello" is non-numeric).
    assert!(optimised("if {$a == \"hello\"} {}", TCL).contains("$a eq \"hello\""));
    assert!(opt_fires("if {$a == \"hello\"} {}", TCL, "O120"));

    // != inside an expr substitution ⇒ ne. Rust routes this through O110 (expr
    // instcombine) rather than O120; the rewrite text is identical. tclsh sweep:
    // $a != "hello" <=> $a ne "hello".
    let ne = "set ok [expr {$a != \"hello\"}]";
    assert!(optimised(ne, TCL).contains("$a ne \"hello\""));
    assert!(opt_any_of(ne, TCL, &["O120", "O110"]));

    // String literal on the left also rewrites. tclsh: "hello" == $a <=> eq.
    assert!(optimised("if {\"hello\" == $a} {}", TCL).contains("eq"));
    // != with a string on the right ⇒ ne.
    assert!(optimised("if {$x != \"foo\"} {}", TCL).contains("ne"));

    // Conservative non-rewrites (D5-O120: needs ≥1 provably-non-numeric operand).
    for src in [
        "set a [clock seconds]\nif {$a == \"1\"} {}", // INT-typed var, numeric literal
        "set a [string trim $raw]\nif {$a == \"1\"} {}", // STRING type ≠ non-numeric value
        "set a [string trim $raw]\nif {$a == \"true\"} {}", // boolean-like literal
        "set a [string trim $raw]\nif {$a == \"1.25\"} {}", // float-like literal
        "set a [string trim $x]\nset b [string trim $y]\nif {$a == $b} {}", // two STRING vars
        "if {$a == $b} {}",                           // both unknown
        "if {$a == \"true\"} {}",                     // boolean-like literal, unknown var
        "if {$a == \"1.25\"} {}",                     // float-like literal, unknown var
    ] {
        assert!(opt_absent(src, TCL, "O120"), "O120 must not fire: {src}");
    }
}

#[test]
fn o120_with_sccp_const_and_mixed() {
    // A var known non-numeric via SCCP CONST ("foo") proves the string path.
    // tclsh: `set a foo; expr {$a == $b}` == `expr {$a eq $b}` for every $b.
    let vc = "set a foo\nif {$a == $b} {}";
    assert!(opt_fires(vc, TCL, "O120"));
    assert!(optimised(vc, TCL).contains("eq $b"));

    // Mixed expr: only the string compare is rewritten; numeric `$n == 1` stays.
    // Rust carries this via O110 canonicalisation; the eq-half is present.
    let mixed = "set a [string trim $raw]\nif {$a == \"x\" && $n == 1} {}";
    let mx = optimised(mixed, TCL);
    assert!(mx.contains("$a eq \"x\""));
    assert!(mx.contains("$n == 1"));
    assert!(opt_any_of(mixed, TCL, &["O120", "O110"]));

    // Two SCCP-constant operands: the compare folds outright. tclsh: "foo" !=
    // "bar" ⇒ condition is constant-false ⇒ `if {0}` deleted (O112).
    let vv = "set a foo\nset b bar\nif {$a == $b} {}";
    let v = optimised(vv, TCL);
    assert!(!v.contains("=="));
    assert!(!v.contains(" eq "));
    assert!(opt_fires(vv, TCL, "O112"));
}

// ===========================================================================
// De Morgan / inversion — exercised via O110 rewrites
// ===========================================================================

#[test]
fn demorgan_logic_via_o110() {
    // The De-Morgan logic is the O110 expr rewrite; assert the transforms on
    // `[expr {...}]`. All tclsh-swept above.
    assert!(optimised("set v [expr {!($a && $b)}]", TCL).contains("!$a || !$b"));
    assert!(optimised("set v [expr {!($a || $b)}]", TCL).contains("!$a && !$b"));
    // Plain `&&` / `==` / single `!` are NOT De-Morgan candidates (no rewrite to
    // the inverse form fires).
    assert!(optimised("set v [expr {$a && $b}]", TCL).contains("$a && $b"));
    assert!(!optimised("set v [expr {$a == $b}]", TCL).contains("||"));
}

#[test]
fn invert_logic_via_o110() {
    // The inversion logic surfaces as the `!(...)` O110 rewrite. tclsh-swept.
    assert!(optimised("set v [expr {!($a == $b)}]", TCL).contains("$a != $b"));
    assert!(optimised("set v [expr {!($a < $b)}]", TCL).contains("$a >= $b"));
    assert!(optimised("set v [expr {!($a >= $b)}]", TCL).contains("$a < $b"));
    assert!(optimised("set v [expr {!($a && $b)}]", TCL).contains("!$a || !$b"));
    assert!(optimised("set v [expr {!($a || $b)}]", TCL).contains("!$a && !$b"));
    // Complex chain. tclsh sweep: !(a==b && c<d) == (a!=b || c>=d).
    assert!(
        optimised("set v [expr {!($a == $b && $c < $d)}]", TCL).contains("$a != $b || $c >= $d")
    );
    // Double-not removal in a boolean context. tclsh: !!x in `if` == x.
    assert!(optimised("if {!!$x} { puts yes }", TCL).contains("if {$x}"));
}

/// The `!(x <cmp> y)` inversion used to be a local 8-arm match covering only
/// `==`/`!=`/`<`/`>=`/`>`/`<=`/`eq`/`ne`, missing the TIP 461 string-ordering
/// four (`lt`/`le`/`gt`/`ge`) and list membership (`in`/`ni`) entirely — a
/// missed simplification, not a correctness bug, since the negation identity
/// holds for both (a total order for `lt`/etc., a direct definitional
/// negation for `in`/`ni`) exactly like it does for the forms already
/// covered. Now derived from `BinOp::inverse()`
/// (`tcl_syntax::expr::operators`, issue #983's unification), which covers
/// all fourteen comparison operators.
#[test]
fn invert_logic_covers_tip461_and_membership_operators() {
    const TCL9: &str = "tcl9.0";
    assert!(optimised("set v [expr {!($a lt $b)}]", TCL9).contains("$a ge $b"));
    assert!(optimised("set v [expr {!($a le $b)}]", TCL9).contains("$a gt $b"));
    assert!(optimised("set v [expr {!($a gt $b)}]", TCL9).contains("$a le $b"));
    assert!(optimised("set v [expr {!($a ge $b)}]", TCL9).contains("$a lt $b"));
    assert!(optimised("set v [expr {!($a eq $b)}]", TCL9).contains("$a ne $b"));
    assert!(optimised("set v [expr {!($a in $b)}]", TCL9).contains("$a ni $b"));
    assert!(optimised("set v [expr {!($a ni $b)}]", TCL9).contains("$a in $b"));
}

// ===========================================================================
// apply_optimisations / find_optimisations — edge cases
// ===========================================================================

#[test]
fn apply_optimisations_edge_cases() {
    let registry = registry_for_dialect(TCL);
    // Empty optimisation set ⇒ source unchanged.
    assert_eq!(apply_optimisations("set x 1", &[]), "set x 1");

    // Multiple non-overlapping rewrites. tclsh: a=1, b=2 ⇒ a+b == 3.
    assert!(optimised("set a 1\nset b 2\nset c [expr {$a + $b}]", TCL).contains("set c 3"));

    // Reverse-offset application stays stable. tclsh: a=1, b=2 ⇒ `puts 1`, `puts 2`.
    let rev = optimised("set a 1\nset b 2\nputs $a\nputs $b", TCL);
    assert!(rev.contains("puts 1"));
    assert!(rev.contains("puts 2"));

    // Sorting: the optimise_with_dialect set is span-sorted (the manager's
    // determinism chokepoint). Co-located entries (e.g. O100 + O102 on the same
    // expr span) are permitted before application; `apply_optimisations` resolves
    // them, which the value assertions above already exercise.
    let src = "set a 1\nset b [expr {$a + 2}]\nset c [expr {$a + 3}]";
    let opts = optimise_with_dialect(src, registry, Some(TCL));
    let starts: Vec<u32> = opts.iter().map(|o| o.span.start()).collect();
    let mut sorted = starts.clone();
    sorted.sort_unstable();
    assert_eq!(
        starts, sorted,
        "optimisations must be span-start-sorted: {starts:?}"
    );
}

// ===========================================================================
// Variable-shape guardrails — array/namespaced refs not rewritten
// ===========================================================================

#[test]
fn variable_shape_guardrails() {
    for src in [
        "set x ${a(1)}\nputs $x",
        "set x $a(1)\nputs $x",
        "set x $::ns::arr(k)\nputs $x",
    ] {
        assert_eq!(optimised(src, TCL), src);
        assert!(
            opt_codes(src, TCL).is_empty(),
            "no rewrite for shape: {src}"
        );
    }
}

// ===========================================================================
// Cross-event DSE guardrails (f5-irules) — stores read in a later event survive
// ===========================================================================

#[test]
fn cross_event_dse_guardrails() {
    // `$uri` set in HTTP_REQUEST, read in HTTP_RESPONSE — the store must survive
    // (connection-scoped variables persist across `when` handlers). (Structural:
    // deleting `set uri` would leave $uri undefined in the response handler.)
    let uri = "when HTTP_REQUEST {\n    set uri [HTTP::uri]\n}\nwhen HTTP_RESPONSE {\n    log local0. \"uri=$uri\"\n}\n";
    assert!(optimised(uri, IR).contains("set uri"));

    // `[info exists ans_cleared]` in DNS_RESPONSE observes a flag set in
    // DNS_REQUEST — neither the store nor the existence check may be folded.
    let flag = "when DNS_REQUEST {\n    if { [DNS::header opcode] ne \"QUERY\" } {\n        set ans_cleared 1\n        DNS::return\n        return\n    }\n}\nwhen DNS_RESPONSE {\n    if { [info exists ans_cleared] } {\n        unset -nocomplain -- ans_cleared\n        return\n    }\n}\n";
    assert!(optimised(flag, IR).contains("set ans_cleared"));
}

// ===========================================================================
// Shimmer detection — S100 / S101 / S102
// ===========================================================================

#[test]
fn shimmer_no_false_positives() {
    // Clean integer code: no shimmer.
    assert_eq!(shimmer_codes("set x 0\nincr x"), Vec::<String>::new());
    // List operations stay list-typed: no shimmer.
    assert_eq!(
        shimmer_codes("set x [list a b c]\nset n [llength $x]"),
        Vec::<String>::new()
    );
    // Integer arithmetic: no shimmer.
    assert_eq!(
        shimmer_codes("set x 42\nset y [expr {$x + 1}]"),
        Vec::<String>::new()
    );
    // `string length` accepts any type — int→strlen does not shimmer.
    assert_eq!(
        shimmer_count("set x 42\nset y [string length $x]", "S100"),
        0
    );
    // boolean → int is a numeric sub-type, not a shimmer.
    assert_eq!(
        shimmer_count("set x true\nset y [expr {$x + 1}]", "S100"),
        0
    );
}

#[test]
fn shimmer_string_in_numeric_context_s100() {
    // A STRING-typed var used in a numeric expr shimmers (S100 outside a loop).
    assert!(
        shimmer_count(
            "set x [string trim \" hello \"]\nset y [expr {$x + 1}]",
            "S100"
        ) >= 1
    );
    // incr on a STRING var shimmers.
    assert!(shimmer_count("set x [string trim \" hello \"]\nincr x", "S100") >= 1);
}

#[test]
fn shimmer_loop_paths_no_crash() {
    // Shimmer inside a loop (S101 territory) — type-analysis-dependent; assert no
    // panic and a well-formed result.
    let _ = shimmer_codes(
        "set x [list a b c]\nfor {set i 0} {$i < 3} {incr i} {\n    set n [llength $x]\n    set y [expr {$x + 1}]\n}\n",
    );
    // Consistent integer loop: no S102 thunking.
    assert_eq!(
        shimmer_count(
            "set x 0\nfor {set i 0} {$i < 10} {incr i} {\n    incr x\n}\n",
            "S102"
        ),
        0
    );
    // Oscillating-type loop body — may or may not detect S102 depending on
    // inference depth; assert no panic.
    let _ = shimmer_codes(
        "set x 0\nfor {set i 0} {$i < 10} {incr i} {\n    set x [string length [string repeat a $x]]\n}\n",
    );
}

// ===========================================================================
// GVN / CSE / LICM — find_redundant_computations (O105 / O106)
// ===========================================================================

#[test]
fn gvn_cse_detection() {
    // A repeated pure call ([string length $x]) is detected (O105). (Structural:
    // the second computation is provably redundant.)
    let dup = "set x hello\nset a [string length $x]\nset b [string length $x]";
    assert!(gvn_codes(dup).iter().any(|c| c == "O105"));
    assert!(!gvn_codes(dup).is_empty());

    // Different arguments ⇒ not flagged.
    assert_eq!(
        gvn_codes("set a [string length hello]\nset b [string length world]"),
        Vec::<String>::new()
    );

    // Three identical ⇒ two redundancy warnings.
    let three = "set a [string length $x]\nset b [string length $x]\nset c [string length $x]";
    assert!(gvn_codes(three).len() >= 2);

    // A mutation between the calls invalidates the redundancy.
    assert_eq!(
        gvn_codes("set a [string length $x]\nset x different\nset b [string length $x]"),
        Vec::<String>::new()
    );

    // Precision note (NOT a miscompile): the `find_redundancies_for_cu` GVN finder
    // DOES flag the repeated `[clock seconds]` with O105 (it treats the call key
    // as redundant) even though `[clock seconds]` is impure. This is a
    // detection-precision detail: it is a HINT, not an applied rewrite — the full
    // `optimise_with_dialect` pipeline does NOT CSE `clock seconds` (no O105
    // survives), so no miscompile is produced. Assert the sound end-to-end
    // invariant: the pipeline leaves the two calls intact.
    let clk = "set a [clock seconds]\nset b [clock seconds]";
    assert!(opt_absent(clk, TCL, "O105"));
    assert!(optimised(clk, TCL).contains("set a [clock seconds]"));
    assert!(optimised(clk, TCL).contains("set b [clock seconds]"));
}

// ===========================================================================
// Edge cases and robustness — must not crash, well-formed results
// ===========================================================================

#[test]
fn edge_cases_no_crash() {
    // Empty / comment / whitespace sources ⇒ no optimisations.
    assert!(opt_codes("", TCL).is_empty());
    assert!(opt_codes("# just a comment", TCL).is_empty());
    assert!(opt_codes("   \n\n  ", TCL).is_empty());
    assert_eq!(optimised("", TCL), "");

    // Nested proc definitions ⇒ no crash.
    let _ = opt_codes(
        "proc outer {} {\n    proc inner {} { return 1 }\n    return [inner]\n}\n",
        TCL,
    );

    // Deep expression nesting ⇒ no stack overflow.
    let mut expr = String::from("$x");
    for _ in 0..20 {
        expr = format!("({expr} + 1)");
    }
    let _ = opt_codes(&format!("set v [expr {{{expr}}}]"), TCL);

    // Many sets ⇒ no performance pathology (just completes).
    let mut lines: Vec<String> = (0..50).map(|i| format!("set v{i} {i}")).collect();
    lines.push("puts $v49".into());
    let _ = opt_codes(&lines.join("\n"), TCL);

    // unset clears the constant ⇒ no fold across it (source unchanged).
    let unset_src = "set a 1\nunset a\nset b [expr {$a + 2}]";
    assert_eq!(optimised(unset_src, TCL), unset_src);

    // catch / try / foreach bodies ⇒ no crash.
    let _ = opt_codes("catch {\n    set x [expr {1 + 2}]\n}\n", TCL);
    let _ = opt_codes(
        "try {\n    set x [expr {1 + 2}]\n} on error {e} {\n    puts $e\n}\n",
        TCL,
    );
    let _ = opt_codes("foreach item {a b c} {\n    puts $item\n}\n", TCL);
    let _ = opt_codes("switch abc {\n    abc -\n    def { set x 1 }\n}\n", TCL);
}

#[test]
fn edge_cases_loops_and_namespaces() {
    // Constants before a loop that may modify them are NOT propagated in.
    let loop_src =
        "set total 0\nfor {set i 0} {$i < 5} {incr i} {\n    set total [expr {$total + $i}]\n}\n";
    assert_eq!(optimised(loop_src, TCL), loop_src);
    assert!(opt_codes(loop_src, TCL).is_empty());

    // Deeply nested namespace proc folds at the call site. tclsh: a::b::c::f == 1.
    let deep = "namespace eval a {\n    namespace eval b {\n        namespace eval c {\n            proc f {} { return 1 }\n        }\n    }\n}\nset v [a::b::c::f]\n";
    assert!(optimised(deep, TCL).contains("set v 1"));
}

#[test]
fn robustness_malformed_inputs() {
    // Malformed / unusual inputs ⇒ no panic, well-formed (possibly empty) result.
    for src in [
        "set v [expr {",
        "if {$x",
        "puts",
        "set x 1",
        "set x \"héllo\"\nputs $x",
        "set x \"hello\\nworld\"\nputs $x",
        "set x {$not_a_var}\nputs $x",
        "set a 1; set b [expr {$a + 1}]",
    ] {
        let _ = opt_codes(src, TCL); // no panic; result is a Vec
    }
    // `{$not_a_var}` is a brace literal, not a variable ref — propagation is
    // suppressed (no fresh substitution injected). Output is sound either way.
    let bracelit = optimised("set x {$not_a_var}\nputs $x", TCL);
    assert!(bracelit.contains("$not_a_var")); // tclsh: prints literal "$not_a_var"
}

// ===========================================================================
// O124 — comment out unused procs (f5-irules only)
// ===========================================================================

#[test]
fn o124_unused_irule_procs() {
    // Unused proc commented out.
    let unused = "proc helper {} { return 1 }\nwhen HTTP_REQUEST { pool p1 }";
    assert!(opt_fires(unused, IR, "O124"));
    let uo = optimised(unused, IR);
    assert!(uo.contains("# proc helper"));
    assert!(uo.contains("# [O124]"));

    // Used / transitively-used / direct-call / qualified procs are NOT commented.
    let kept = [
        "proc helper {} { return 1 }\nwhen HTTP_REQUEST { set v [call helper] }",
        "proc helper {} { return 1 }\nwhen HTTP_REQUEST { set v [helper] }",
        "proc inner {} { return 1 }\nproc outer {} { return [call inner] }\nwhen HTTP_REQUEST { set v [call outer] }",
        // library iRule (only procs + RULE_INIT) — skip.
        "proc helper {} { return 1 }\nwhen RULE_INIT { set ::x 1 }",
    ];
    for src in kept {
        assert!(opt_absent(src, IR, "O124"), "O124 must not fire: {src}");
    }

    // Plain Tcl is never touched by O124.
    assert!(opt_absent(
        "proc helper {} { return 1 }\nputs hello",
        TCL,
        "O124"
    ));

    // O124 supersedes tail-call rewrites for an unused tail-recursive proc.
    let supersede = "proc fact {n acc} {\n    if {$n <= 1} { return $acc }\n    return [fact [expr {$n - 1}] [expr {$n * $acc}]]\n}\nwhen HTTP_REQUEST { pool p1 }";
    let codes = opt_codes(supersede, IR);
    assert!(codes.iter().any(|c| c == "O124"));
    assert!(!codes.iter().any(|c| c == "O121"));
    assert!(!codes.iter().any(|c| c == "O122"));

    // A used proc allows other rewrites (O120 inside it); not commented out.
    let used = "proc helper {x} {\n    if {$x == \"foo\"} { return 1 }\n    return 0\n}\nwhen HTTP_REQUEST { set v [call helper bar] }";
    assert!(opt_fires(used, IR, "O120"));
    assert!(opt_absent(used, IR, "O124"));

    // O124 coexists with an in-event rewrite.
    let coexist = "proc helper {} { return 1 }\nwhen HTTP_REQUEST {\n    set x [expr {1 + 2}]\n    pool p1\n}";
    let cc = opt_codes(coexist, IR);
    assert!(cc.iter().any(|c| c == "O124"));
    assert!(cc.iter().any(|c| c == "O101" || c == "O102" || c == "O126"));

    // eval only in an UNREACHABLE proc does not suppress O124.
    let unreach = "proc dynamic_helper {} {\n    set cmd foo\n    eval $cmd\n}\nwhen HTTP_REQUEST { pool p1 }";
    assert!(opt_fires(unreach, IR, "O124"));
}

// ===========================================================================
// O121 / O122 / O123 — tail-call / loop / accumulator
// ===========================================================================

#[test]
fn tail_call_o121_o122_o123() {
    // Tail-position self-call ⇒ O121 (tailcall) or O122 (loop). tclsh: the
    // tailcall/loop forms compute the same result (factorial 5 1 == 120).
    let fac = "proc factorial {n acc} {\n    if {$n <= 1} { return $acc }\n    return [factorial [expr {$n - 1}] [expr {$n * $acc}]]\n}\n";
    assert!(opt_any_of(fac, TCL, &["O121", "O122"]));

    // Arity mismatch ⇒ O121 may fire but O122 (full loop conversion) must not.
    let arity = "proc f {a b} {\n    return [f $a]\n}\n";
    assert!(opt_fires(arity, TCL, "O121"));
    assert!(opt_absent(arity, TCL, "O122"));

    // Non-tail accumulator-eligible recursion ⇒ O123 hint.
    let nontail = "proc factorial {n} {\n    if {$n <= 1} { return 1 }\n    return [expr {$n * [factorial [expr {$n - 1}]]}]\n}\n";
    assert!(opt_fires(nontail, TCL, "O123"));
    // An already-tail-recursive proc does NOT get the O123 hint.
    assert!(opt_absent(fac, TCL, "O123"));
    // A non-recursive proc gets neither tail-call code nor O123.
    let plain = "proc add {a b} {\n    return [expr {$a + $b}]\n}\n";
    assert!(opt_absent(plain, TCL, "O121"));
    assert!(opt_absent(plain, TCL, "O123"));
    // Doubly-recursive (fib) ⇒ no O122 (not a tail call).
    let fib = "proc fib {n} {\n    if {$n <= 1} { return $n }\n    return [expr {[fib [expr {$n-1}]] + [fib [expr {$n-2}]]}]\n}\n";
    assert!(opt_absent(fib, TCL, "O122"));
}

// ===========================================================================
// O125 — code sinking
// ===========================================================================

#[test]
fn o125_code_sinking() {
    // Basic sink into an `if` whose condition does not read the var. Either O125
    // or O100 is accepted; here O125 fires. tclsh: `set b foo; if {$a} {puts $b}`
    // and the sunk form both print foo iff $a is true.
    let basic = "set b foo\nif {$a} {\n    puts $b\n}";
    assert!(opt_any_of(basic, TCL, &["O125", "O100"]));

    // O125 must NOT fire when the var is read in the condition or its RHS is a
    // command substitution.
    assert!(opt_absent(
        "set b $x\nif {$b} {\n    puts hello\n}",
        TCL,
        "O125"
    )); // var in condition
    assert!(opt_absent(
        "set b [clock seconds]\nif {$a} {\n    puts $b\n}",
        TCL,
        "O125"
    )); // cmd-sub RHS

    // Coexists with O120 (string-compare rewrite in the condition).
    let with120 = "set b foo\nif {$kind == \"x\"} {\n    puts $b\n}";
    assert!(opt_fires(with120, TCL, "O120"));
    assert!(optimised(with120, TCL).contains("$kind eq \"x\""));
}

// ===========================================================================
// O127 — single-use store-to-load forwarding (proc-local)
// ===========================================================================

#[test]
fn o127_load_forwarding() {
    // Single-use proc-local with a command substitution is inlined. tclsh: the
    // inlined `[set x [clock seconds]]` assigns x and yields its value, identical
    // to `set x [clock seconds]; puts $x`.
    let cmdsub = "proc f {} { set x [clock seconds]\n puts $x }";
    assert!(opt_fires(cmdsub, TCL, "O127"));
    assert!(optimised(cmdsub, TCL).contains("puts [set x [clock seconds]]"));

    // A constant is handled by propagation/DSE, NOT O127. tclsh: x=42 ⇒ puts 42.
    let constv = "proc f {} { set x 42\n puts $x }";
    assert!(opt_absent(constv, TCL, "O127"));
    assert!(opt_fires(constv, TCL, "O102"));
    assert!(optimised(constv, TCL).contains("puts 42"));

    // More-than-once use ⇒ not inlined.
    assert!(opt_absent(
        "proc f {} { set x [clock seconds]\n puts $x\n puts $x }",
        TCL,
        "O127"
    ));
    // Top-level variables ⇒ not inlined.
    assert!(opt_absent("set x [clock seconds]\nputs $x", TCL, "O127"));
    // A barrier (info exists between) blocks forwarding.
    assert!(opt_absent(
        "proc f {} { set x [clock seconds]\n info exists x\n puts $x }",
        TCL,
        "O127"
    ));
    // upvar/global aliasing blocks forwarding.
    assert!(opt_absent(
        "proc f {} { upvar 1 ext x\n set x [clock seconds]\n puts $x }",
        TCL,
        "O127"
    ));
    assert!(opt_absent(
        "proc f {} { global x\n set x [clock seconds]\n puts $x }",
        TCL,
        "O127"
    ));
    // A phi merge (two defining branches) blocks forwarding.
    assert!(opt_absent(
        "proc f {c} { if {$c} { set x [clock seconds] } else { set x [clock clicks] }\n puts $x }",
        TCL,
        "O127"
    ));

    // NOTE on gaps (sound): the var-copy form `set x $arg; puts $x` is left
    // unchanged rather than inlined (O127); an intervening empty `eval {}` barrier
    // is not honoured.
}

// ===========================================================================
// Pass interactions — composed rewrites
// ===========================================================================

#[test]
fn pass_interactions() {
    // Constant fold resolves the condition, then O112/O101 eliminates the if.
    assert!(opt_any_of(
        "set x 1\nif {$x > 0} {\n    puts yes\n}\n",
        TCL,
        &["O101", "O112"]
    ));
    // Propagation enables DSE: after folding x into puts, `set x` is dead.
    let prop = "set x 42\nputs $x";
    assert!(optimised(prop, TCL).contains("puts 42"));
    assert!(opt_fires(prop, TCL, "O109"));

    // Chained propagation across multiple variables. tclsh: a=1 ⇒ d==4.
    assert!(optimised(
        "set a 1\nset b [expr {$a + 1}]\nset c [expr {$b + 1}]\nset d [expr {$c + 1}]\nputs $d\n",
        TCL
    )
    .contains("puts 4"));

    // All uses propagated then DSE. tclsh: x=5 ⇒ `puts 5` and y==6.
    let dse = "set x 5\nputs $x\nset y [expr {$x + 1}]";
    assert!(optimised(dse, TCL).contains("puts 5"));
    assert!(optimised(dse, TCL).contains("set y 6"));

    // Overlapping nested-expr rewrites resolve without corruption (no panic).
    let _ = opt_codes("set a 1\nset b [expr {$a + [expr {2 + 3}]}]", TCL);

    // InstCombine in a branch then fold. tclsh: a=5 ⇒ 5+0==5 ⇒ condition true.
    assert!(opt_any_of(
        "set a 5\nif {$a + 0 == 5} { puts yes }",
        TCL,
        &["O101", "O112"]
    ));

    // Strength reduction inside a command substitution expr.
    assert!(opt_fires("if {$x ** 2} { puts yes }", TCL, "O113"));
    // O117 fires in branch conditions.
    assert!(opt_fires(
        "if {[string length $s] == 0} { puts empty }",
        TCL,
        "O117"
    ));
    // String-compare ⇒ eq rewrite inside [expr {...}]. Rust routes the expr-subst
    // form through O110 (instcombine) rather than O120; the rewrite is identical.
    // tclsh: $a == "hello" <=> $a eq "hello".
    let expr_eq = "set ok [expr {$a == \"hello\"}]";
    assert!(optimised(expr_eq, TCL).contains("$a eq \"hello\""));
    assert!(opt_any_of(expr_eq, TCL, &["O120", "O110"]));
}

// ===========================================================================
// Profile directive survival + multipass string-build collapse
// ===========================================================================

#[test]
fn profile_directive_and_multipass() {
    // The `# profiles: HTTP2` comment survives optimisation; the `if {1}` inside
    // the event unwraps (O112) and keeps the HTTP2 call.
    let src =
        "# profiles: HTTP2\nwhen HTTP_REQUEST {\n    if {1} {\n        HTTP2::active\n    }\n}\n";
    let out = optimised(src, IR);
    assert!(out.contains("# profiles: HTTP2"));
    assert!(out.contains("HTTP2::active"));
    assert!(opt_fires(src, IR, "O112"));

    // Multipass: a write-only string build in a proc collapses to a single
    // `return {Hello World}`; the intermediate local is removed. tclsh: the proc
    // returns "Hello World" either way.
    let registry = registry_for_dialect(TCL);
    let build = "proc build_banner {} {\n    set msg {Hello}\n    append msg { }\n    append msg World\n    return $msg\n}\n";
    let (mp, _) = optimise_source_multipass(build, registry, Some(TCL), 10);
    assert!(mp.contains("return {Hello World}"));
    assert!(!mp.contains("append"));
    assert!(!mp.contains("set msg"));
}
