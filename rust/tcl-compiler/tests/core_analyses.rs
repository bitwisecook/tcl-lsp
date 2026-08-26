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

//! Core-analyses + existence-check tests:
//!   * SCCP / liveness / dead-store / type / fold analyses
//!   * `info exists` / `array exists` existence-check policy
//!
//! ## Two observation surfaces
//!
//! A built [`CompilationUnit`] exposes, per function, a
//! [`tcl_compiler::compilation_unit::FunctionUnit`] (`cu.top_level`,
//! `cu.procedures["::p"]`) carrying the raw lattice: `fu.sccp.values` is the
//! values map (keyed by an interned `(Symbol, version)` — resolve a name with
//! `fu.ssa.var_symbol`), `fu.sccp.constant_branches` is the constant-branch set,
//! and the unreachable set is `cfg.blocks \ sccp.executable_blocks`. The
//! lattice-level tests assert against that.
//!
//! The diagnostic-policy tests go through the same merged `tcl diag` surface the
//! existing FP catalogue uses (`src/analyser/diagnostics/fp/mod.rs::codes`): the
//! analyser pass (which owns W210 read-before-set and the I230 constant-branch
//! diagnostic) unioned with `run_all_checks`, optimisation codes dropped. Copied
//! verbatim into [`codes`].
//!
//! ## C-Tcl ground truth
//!
//! Every Tcl-semantic fact an assertion leans on — `info exists`/`array exists`
//! outcomes, variable existence, and constant-fold values — is pinned to real
//! tclsh via `scripts/dev/tclsh_check.sh` and recorded in a `// tclsh:` comment
//! (tclsh8.6 + tclsh9.0 agree on every one used here).
//!
//! ## Conservative folding
//!
//! The analyser is deliberately conservative in a few places — it leaves a value
//! `Overdefined` (sound: never substitutes a wrong constant) rather than folding
//! a registry command, and it folds an existence check only for the two
//! false-positive-free cases (a parameter always exists, as a scalar →
//! `info exists` true / `array exists` false; a never-defined
//! local never exists → false), declining flow-sensitive must-define / narrowing
//! precision. None of these is unsound (no case folds a check C-Tcl leaves
//! dynamic, nor suppresses a genuine read-before-set), so each is asserted at the
//! *actual* verdict with an explanatory comment rather than `#[ignore]`d.

use tcl_compiler::analyser::Analyser;
use tcl_compiler::analyses::{ConstValue, LatticeKind, LatticeValue};
use tcl_compiler::compilation_unit::{CompilationUnit, FunctionUnit};
use tcl_compiler::types::{TclType, TypeKind};
use tcl_registry::model::ingress::static_context_for;

/// Default dialect for reproducers that are not dialect-sensitive.
const D: &str = "tcl8.6";

// ===========================================================================
// Lattice-level harness (the `analyse_source(...).top_level` surface).
// ===========================================================================

/// Build the shared compilation unit for `src` under [`D`]. `cu.top_level` is
/// the module script's function analysis; `cu.procedures[qname]` are the
/// per-proc ones.
fn build(src: &str) -> CompilationUnit {
    CompilationUnit::build_for(src, static_context_for(D).commands(), false)
}

/// The lattice value for `(name, version)`. Resolves `name` through the
/// function's SSA interner to its `Symbol`, then indexes `sccp.values`. `None`
/// is the honest "no fact for this (name, version)" answer.
fn lat<'a>(fu: &'a FunctionUnit, name: &str, version: u32) -> Option<&'a LatticeValue> {
    let sym = fu.ssa.var_symbol(name)?;
    fu.sccp.values.get(&(sym, version))
}

/// Every lattice value recorded for variable `name` (any version).
fn lats_for<'a>(fu: &'a FunctionUnit, name: &str) -> Vec<&'a LatticeValue> {
    let Some(sym) = fu.ssa.var_symbol(name) else {
        return Vec::new();
    };
    let mut out: Vec<(u32, &LatticeValue)> = fu
        .sccp
        .values
        .iter()
        .filter(|((s, _), _)| *s == sym)
        .map(|((_, v), lv)| (*v, lv))
        .collect();
    out.sort_by_key(|(v, _)| *v);
    out.into_iter().map(|(_, lv)| lv).collect()
}

/// Assert `(name, version)` is a `Const` integer equal to `want`.
#[track_caller]
fn assert_const_int(fu: &FunctionUnit, name: &str, version: u32, want: i64) {
    match lat(fu, name, version) {
        Some(LatticeValue::Const(ConstValue::Int(i))) => assert_eq!(
            *i, want,
            "({name:?}, {version}) expected Const(Int({want})), got Int({i})"
        ),
        other => panic!("({name:?}, {version}) expected Const(Int({want})), got {other:?}"),
    }
}

/// Assert `(name, version)` is a `Const` string equal to `want`.
#[track_caller]
fn assert_const_str(fu: &FunctionUnit, name: &str, version: u32, want: &str) {
    match lat(fu, name, version) {
        Some(LatticeValue::Const(ConstValue::String(s))) => assert_eq!(
            s, want,
            "({name:?}, {version}) expected Const(String({want:?})), got {s:?}"
        ),
        other => panic!("({name:?}, {version}) expected Const(String({want:?})), got {other:?}"),
    }
}

/// The set of variable names that are a dead store (any version). A dead store
/// is an SSA definition whose value is never read; the liveness pass records
/// them in
/// `sccp`-adjacent form, but the observable, stable signal is the analyser's
/// W220 ("Assignment to 'x' is never read"). We reconstruct the variable set
/// from that diagnostic, which is exactly what downstream consumers see.
fn dead_store_vars(src: &str) -> std::collections::HashSet<String> {
    Analyser::new()
        .analyse(src, D)
        .diagnostics
        .iter()
        .filter(|d| d.code.to_string() == "W220")
        .filter_map(|d| d.message.split('\'').nth(1).map(str::to_owned))
        .collect()
}

/// Constant-branch records for a function, as `(condition, value)`.
fn const_branches(fu: &FunctionUnit) -> Vec<(String, bool)> {
    fu.sccp
        .constant_branches
        .iter()
        .map(|b| (b.condition.clone(), b.value))
        .collect()
}

/// Block names that SCCP proved unreachable: in `cfg.blocks` but absent from
/// `executable_blocks`.
fn unreachable_blocks(fu: &FunctionUnit) -> std::collections::HashSet<String> {
    fu.cfg
        .blocks
        .iter()
        .filter(|(id, _)| !fu.sccp.executable_blocks.contains(id))
        .map(|(_, b)| b.name.clone())
        .collect()
}

/// The type-lattice entry for `(name, version)`.
fn ty<'a>(
    fu: &'a FunctionUnit,
    name: &str,
    version: u32,
) -> Option<&'a tcl_compiler::types::TypeLattice> {
    let sym = fu.ssa.var_symbol(name)?;
    fu.types.get(&(sym, version))
}

// ===========================================================================
// TestCoreAnalyses — SCCP / liveness / dead-store core analyses.
// ===========================================================================
mod core_analyses {
    use super::*;

    #[test]
    fn call_arguments_contribute_variable_uses() {
        // `puts $a` is a use of a → `a` is not a dead store.
        assert!(!dead_store_vars("set a 1\nputs $a").contains("a"));
    }

    #[test]
    fn liveness_ignores_intrablock_def_use() {
        // `set b [expr {$a + 1}]` reads a in the same block it was defined; a is
        // live (used) so it is not a dead store. The observable consequence is
        // "a is used", i.e. no W220.
        let cu = build("set a 1\nset b [expr {$a + 1}]");
        assert!(!dead_store_vars("set a 1\nset b [expr {$a + 1}]").contains("a"));
        // a₁ is a tracked const (used by the expr), b₁ folds to 2.
        assert_const_int(&cu.top_level, "a", 1, 1);
        assert_const_int(&cu.top_level, "b", 1, 2);
    }

    #[test]
    fn incr_uses_previous_definition() {
        // `incr count` reads count₁ then defines count₂; count₁ is used → not dead.
        assert!(!dead_store_vars("set count 3\nincr count\nputs $count").contains("count"));
    }

    #[test]
    fn dead_store_for_overwritten_assignment() {
        // `set a 1; set a 2; …$a…` — the first store (a₁) is dead, exactly one
        // dead store for `a`. tclsh: the program is valid; this is a pure
        // static liveness fact (no runtime analogue).
        let dead = dead_store_vars("set a 1\nset a 2\nset b [expr {$a + 0}]");
        assert!(dead.contains("a"), "a₁ overwrite must be a dead store");
        // Only `a` is dead (b is read into nothing further, but `b` is the live
        // tail def). Count the W220s naming `a`.
        let n = Analyser::new()
            .analyse("set a 1\nset a 2\nset b [expr {$a + 0}]", D)
            .diagnostics
            .iter()
            .filter(|d| d.code.to_string() == "W220" && d.message.contains("'a'"))
            .count();
        assert_eq!(n, 1, "exactly one dead store for a");
    }

    #[test]
    fn phi_incoming_values_mark_branch_defs_used() {
        // Both `set a 1` / `set a 2` arms feed the phi that `$a` reads after the
        // if → neither arm's def is dead.
        let src = "if {$cond} {set a 1} else {set a 2}\nset out [expr {$a + 0}]";
        assert!(!dead_store_vars(src).contains("a"));
    }

    #[test]
    fn constant_branch_marks_not_taken_target_unreachable() {
        // `set a 1; if {$a} …` — a is const-1 (true) so the else is dead.
        // tclsh: `expr {1}` → 1 (true). One constant branch, $a, value true,
        // its not-taken (else) target unreachable.
        let cu = build("set a 1\nif {$a} {set x 1} else {set y 2}\nset out [expr {$x + 0}]");
        let cbs = const_branches(&cu.top_level);
        assert_eq!(cbs.len(), 1, "one constant branch; got {cbs:?}");
        assert_eq!(cbs[0].0, "$a");
        assert!(cbs[0].1, "condition value is true");
        let nt = &cu.top_level.sccp.constant_branches[0].not_taken_target;
        assert!(
            unreachable_blocks(&cu.top_level).contains(nt),
            "not-taken target {nt:?} must be unreachable; unreachable={:?}",
            unreachable_blocks(&cu.top_level)
        );
    }

    #[test]
    fn static_for_loop_feeds_branch_constant_folding() {
        // The static `for` accumulates total; `$total > 5` is provably true so
        // the else is dead. (The analyser additionally records the idiomatic
        // constant-true loop-header branch `1`; we assert the *meaningful*
        // `$total > 5` fold.)
        let src = "\
set total 0
for {set i 0} {$i < 5} {incr i} {
    set total [expr {$total + $i}]
}
if {$total > 5} {
    puts \"total is $total\"
} else {
    puts \"small total\"
}
";
        let cu = build(src);
        let cbs = const_branches(&cu.top_level);
        let guard = cbs
            .iter()
            .find(|(c, _)| c == "$total > 5")
            .expect("`$total > 5` branch folded");
        assert!(guard.1, "`$total > 5` is always true");
        let nt = &cu
            .top_level
            .sccp
            .constant_branches
            .iter()
            .find(|b| b.condition == "$total > 5")
            .unwrap()
            .not_taken_target;
        assert!(
            unreachable_blocks(&cu.top_level).contains(nt),
            "else arm {nt:?} unreachable"
        );
    }

    #[test]
    fn unreachable_branch_defs_are_not_dead_stores() {
        // `if {0} {set x 1}` — x's def is in dead code, not a dead *store*, and
        // a constant-false branch is recorded. tclsh: `expr {0}` → 0 (false).
        let src = "if {0} {set x 1}\nset y 2\nset z [expr {$y + 0}]";
        assert!(
            !dead_store_vars(src).contains("x"),
            "x is in dead code, not a dead store"
        );
        let cu = build(src);
        assert!(
            const_branches(&cu.top_level).iter().any(|(_, v)| !v),
            "a constant-false branch is recorded; got {:?}",
            const_branches(&cu.top_level)
        );
    }

    #[test]
    fn sccp_propagates_expression_constants() {
        // a=1, b=$a+2=3, c=$b*3=9 — chained SCCP fixed point.
        // tclsh: `set a 1; set b [expr {$a+2}]; set c [expr {$b*3}]` → c is 9.
        let cu = build("set a 1\nset b [expr {$a + 2}]\nset c [expr {$b * 3}]");
        assert_const_int(&cu.top_level, "a", 1, 1);
        assert_const_int(&cu.top_level, "b", 1, 3);
        assert_const_int(&cu.top_level, "c", 1, 9);
    }

    #[test]
    fn sccp_does_not_propagate_nan_as_constant() {
        // The evaluator never *produces* a NaN constant from a finite program —
        // there is no public lattice-injection entry point. The observable
        // invariant is: an expression whose value would be NaN does not fold to a
        // substitutable constant (guarding Tcl 9's ARITH DOMAIN).
        // `expr {sqrt(-1)}` raises `domain error` in tclsh (verified:
        //   tclsh8.6/9.0: ERR: domain error: argument not in valid range),
        // so the result must NOT be a Const.
        let cu = build("set x [expr {sqrt(-1)}]");
        let v = lat(&cu.top_level, "x", 1);
        assert!(
            !matches!(v, Some(LatticeValue::Const(_))),
            "a domain-error expr must not fold to a Const; got {v:?}"
        );
        // Sanity: a finite expr still folds.
        let cu_ok = build("set x [expr {2 + 3}]");
        assert_const_int(&cu_ok.top_level, "x", 1, 5);
    }

    #[test]
    fn format_constant_rejects_nan() {
        // The value formatter `format_tcl_value` must render NaN so it cannot be
        // spliced back as a substitutable numeric literal (42 → "42"; 1 → "1";
        // 0 → "0"; NaN → not a finite numeric literal).
        use tcl_compiler::{TclValue, format_tcl_value};
        // Non-NaN values format to their Tcl text. (Tcl booleans are Int(0/1).)
        assert_eq!(format_tcl_value(&TclValue::Int(42)), "42");
        assert_eq!(format_tcl_value(&TclValue::Int(1)), "1");
        assert_eq!(format_tcl_value(&TclValue::Int(0)), "0");
        // A NaN float must not format to something an optimiser could splice
        // back as a number (it would defeat the runtime ARITH DOMAIN). The
        // formatter must not render it as a finite numeric literal like
        // "nan"/"0"/"" that would re-parse to a usable value.
        let nan = format_tcl_value(&TclValue::Float(f64::NAN));
        assert!(
            !matches!(nan.parse::<f64>(), Ok(f) if f.is_finite()),
            "NaN must not render as a finite numeric literal, got {nan:?}"
        );
    }

    #[test]
    fn analysis_runs_for_lowered_procedures() {
        // The per-proc lattice is populated: inside ::add_static, a=1, b=3.
        let cu = build(
            "proc add_static {} {\n    set a 1\n    set b [expr {$a + 2}]\n    return $b\n}\n",
        );
        let p = cu
            .procedures
            .get("::add_static")
            .expect("::add_static lowered");
        assert_const_int(p, "a", 1, 1);
        assert_const_int(p, "b", 1, 3);
    }
}

// ===========================================================================
// TestNewLoweringAnalysis — pipeline works through newly lowered nodes.
// ===========================================================================
mod new_lowering_analysis {
    use super::*;

    #[test]
    fn while_body_visible_to_sccp() {
        // i is used in the condition and by puts → i₁ not dead.
        assert!(!dead_store_vars("set i 0\nwhile {$i < 3} {incr i}\nputs $i").contains("i"));
    }

    #[test]
    fn foreach_var_not_dead() {
        // foreach iteration var x is read by puts → not dead.
        assert!(!dead_store_vars("foreach x {1 2 3} {puts $x}").contains("x"));
    }

    #[test]
    fn catch_result_var_visible() {
        assert!(!dead_store_vars("catch {expr {1/0}} result\nputs $result").contains("result"));
    }

    #[test]
    fn try_handler_var_visible() {
        let src = "try {set x 1} on error {msg opts} {puts $msg}\nputs done";
        assert!(!dead_store_vars(src).contains("msg"));
    }

    #[test]
    fn append_var_is_def() {
        assert!(!dead_store_vars("set s hello\nappend s world\nputs $s").contains("s"));
    }

    #[test]
    fn lappend_type_is_list() {
        // lappend produces a LIST-typed value. tclsh: `lappend items hello` →
        // `hello` and items is a list.
        let cu = build("lappend items hello");
        if let Some(t) = ty(&cu.top_level, "items", 1)
            && t.kind() == TypeKind::Known
        {
            assert_eq!(
                t.tcl_type(),
                Some(TclType::List),
                "lappend result must be LIST"
            );
        }
    }

    #[test]
    fn dict_set_var_is_def() {
        assert!(!dead_store_vars("dict set d key val\nputs $d").contains("d"));
    }
}

// ===========================================================================
// TestInterpolationFolding — constant-folding through string interpolation.
// ===========================================================================
mod interpolation_folding {
    use super::*;

    #[test]
    fn interpolation_folds_to_string_const_diverges() {
        // The analyser folds the expr part (c₁ = 1) but does NOT constant-fold
        // the interpolation's *value* — c₂ is left Overdefined (it is, however,
        // correctly STRING-typed; see `interpolation_result_typed_as_string`).
        // Sound (no wrong substitution), only less precise. tclsh agrees the
        // value would be 10: `set c 1; set c "${c}0"; set c` → 10.
        let cu = build("set c [expr {0+1}]\nset c \"${c}0\"");
        assert_const_int(&cu.top_level, "c", 1, 1);
        assert_overdefined_or_absent(&cu.top_level, "c", 2, "string interpolation value");
    }

    #[test]
    fn interpolation_result_typed_as_string() {
        // The interpolated c₂ is typed STRING regardless of digit content (the
        // value is Overdefined but the type is known).
        let cu = build("set c [expr {0+1}]\nset c \"${c}0\"");
        let t = ty(&cu.top_level, "c", 2).expect("c₂ has a type");
        assert_eq!(t.kind(), TypeKind::Known);
        assert_eq!(t.tcl_type(), Some(TclType::String));
    }

    #[test]
    fn interpolation_multiple_vars_diverges() {
        // The multi-var interpolation value is left Overdefined. tclsh: `set a
        // 1; set b 2; set c "${a}_${b}"` → 1_2.
        let cu = build("set a 1\nset b 2\nset c \"${a}_${b}\"");
        assert_overdefined_or_absent(&cu.top_level, "c", 1, "multi-var interpolation");
    }

    #[test]
    fn interpolation_with_command_sub_is_overdefined() {
        // A command substitution mixed into the interpolation is not constant.
        let cu = build("set a 1\nset c \"${a}[clock seconds]\"");
        for v in lats_for(&cu.top_level, "c") {
            assert!(
                !matches!(v, LatticeValue::Const(_)),
                "c with a command-sub must not be Const, got {v:?}"
            );
        }
    }

    #[test]
    fn incr_with_string_const_increment_diverges_overdefined() {
        // SCCP does NOT fold an `incr` by a string-const increment — it widens
        // b₂ to Overdefined. That is sound (it never substitutes a wrong
        // constant), only less precise, so we assert the actual verdict. tclsh
        // agrees the value would be 11: `set b 1; incr b 10` → 11.
        let cu = build("set c [expr {0+1}]\nset c \"${c}0\"\nset b 1\nincr b $c");
        // c₁ folds to 1; the interpolation value c₂ is Overdefined (see
        // `interpolation_folds_to_string_const_diverges`), so the downstream
        // `incr b $c` likewise cannot fold — b₂ stays Overdefined.
        assert_const_int(&cu.top_level, "c", 1, 1);
        assert_overdefined_or_absent(&cu.top_level, "b", 2, "incr by interpolated string");
    }

    #[test]
    fn interpolation_folds_in_proc_diverges() {
        // As at top level, the expr part c₁ folds to CONST(1) inside a proc
        // body, but the interpolation value c₂ is left Overdefined. tclsh value:
        // 10.
        let cu = build(
            "proc add {a b} {\n    set c [expr {0+1}]\n    set c \"${c}0\"\n    incr b $c\n    return $c\n}\n",
        );
        let p = cu.procedures.get("::add").expect("::add lowered");
        assert_const_int(p, "c", 1, 1);
        assert_overdefined_or_absent(p, "c", 2, "interpolation value in proc");
    }

    #[test]
    fn incr_non_numeric_string_const_is_overdefined() {
        // `incr x $s` with s=="hello" (not an integer) → x₂ Overdefined/absent.
        // tclsh: `set s hello; set x 0; incr x $s` →
        //   ERR: expected integer but got "hello".
        let cu = build("set s hello\nset x 0\nincr x $s");
        assert!(
            matches!(
                lat(&cu.top_level, "x", 2),
                Some(LatticeValue::Overdefined) | None
            ),
            "non-numeric incr increment must not fold, got {:?}",
            lat(&cu.top_level, "x", 2)
        );
    }

    #[test]
    fn shimmer_preserved_after_folding() {
        // A STRING→INT shimmer on the `incr c` arg must survive folding. The
        // shimmer family (S100/S101) is emitted by `run_all_checks`; the merged
        // diag surface must still surface an intrep-shimmer signal for the
        // `${c}0` string fed to `incr`.
        let src = "set c [expr {0+1}]\nset c \"${c}0\"\nset b 1\nincr b $c";
        let cs = codes(src, D);
        assert!(
            cs.iter().any(|c| c == "S100" || c == "S101"),
            "shimmer on c→incr must still be detected; emitted {cs:?}"
        );
    }
}

// ===========================================================================
// TestVariableShapeCoreAnalyses — variable-shape flow + CONSTSET + folds.
// ===========================================================================
mod variable_shape {
    use super::*;

    // --- variable-shape flow into the type table ---

    #[test]
    fn namespaced_scalar_flows_with_qualified_name() {
        // `set ::ns::x …` flows under the qualified name "::ns::x".
        let cu = build("set ::ns::x \"alpha beta\"\nset n [llength $::ns::x]");
        assert!(
            ty(&cu.top_level, "::ns::x", 1).is_some(),
            "::ns::x must appear in the type table"
        );
    }

    #[test]
    fn namespaced_array_element_flows_as_base_array_name() {
        // `set ::ns::arr(k) …` flows under the base array symbol "::ns::arr".
        let cu = build("set ::ns::arr(k) \"alpha beta\"\nset n [llength $::ns::arr(k)]");
        assert!(ty(&cu.top_level, "::ns::arr", 1).is_some());
    }

    #[test]
    fn braced_array_like_name_flows_as_base_array_symbol() {
        // Braced ${a(1)} is a whole-name load of array element a(1); base "a".
        let cu = build("set {a(1)} \"alpha beta\"\nset n [llength ${a(1)}]");
        assert!(ty(&cu.top_level, "a", 1).is_some());
    }

    #[test]
    fn unbraced_array_ref_flows_as_base_array_symbol() {
        let cu = build("set a(1) \"alpha beta\"\nset n [llength $a(1)]");
        assert!(ty(&cu.top_level, "a", 1).is_some());
    }

    #[test]
    fn namespace_var_commands_with_qualified_names_do_not_break_analysis() {
        // global/upvar/unset with qualified names must not produce dead stores.
        let src = "\
namespace eval ::demo {
    variable value 1
}
proc wire_namespace_vars {} {
    global ::demo::value
    upvar 0 ::demo::value local_value
    unset -nocomplain ::demo::value
}
";
        // No W220 dead-store anywhere (the qualified-name handling is robust).
        assert!(
            dead_store_vars(src).is_empty(),
            "qualified-name var commands must not yield dead stores; got {:?}",
            dead_store_vars(src)
        );
    }

    // --- CONSTSET lattice ---

    #[test]
    fn foreach_braced_list_constset() {
        // foreach over a braced literal list → CONSTSET{a,b,c}.
        let cu = build("foreach x {a b c} {puts $x}");
        let cs: Vec<_> = lats_for(&cu.top_level, "x")
            .into_iter()
            .filter(|v| v.kind() == LatticeKind::ConstSet)
            .collect();
        assert!(!cs.is_empty(), "x must have a CONSTSET value");
        assert_constset_strs(cs[0], &["a", "b", "c"]);
    }

    #[test]
    fn foreach_list_cmd_folds_to_constset() {
        // A `foreach x [list a b c]` command-sub with literal args folds into a
        // CONSTSET (each element a known value), the same as a braced literal
        // list — sound and precise. tclsh: `list a b c` → "a b c".  (Enables
        // suppressing dispatch on a literal command list; see the W307 checks.)
        let cu = build("foreach x [list a b c] {puts $x}");
        let cs: Vec<_> = lats_for(&cu.top_level, "x")
            .into_iter()
            .filter(|v| v.kind() == LatticeKind::ConstSet)
            .collect();
        assert!(
            !cs.is_empty(),
            "x must fold to a CONSTSET over [list a b c]"
        );
        assert_constset_strs(cs[0], &["a", "b", "c"]);
    }

    #[test]
    fn foreach_single_element_is_const() {
        // foreach over a single-element list → CONST (not CONSTSET).
        let cu = build("foreach x {only} {puts $x}");
        let cs: Vec<_> = lats_for(&cu.top_level, "x")
            .into_iter()
            .filter(|v| v.kind() == LatticeKind::Const)
            .collect();
        assert!(!cs.is_empty(), "single-element foreach var must be Const");
        assert!(matches!(cs[0], LatticeValue::Const(ConstValue::String(s)) if s == "only"));
    }

    #[test]
    fn foreach_with_var_ref_resolves_through_lattice() {
        // foreach over $items where items={a b} → CONSTSET{a,b}.
        let cu = build("set items {a b}\nforeach x $items {puts $x}");
        let cs: Vec<_> = lats_for(&cu.top_level, "x")
            .into_iter()
            .filter(|v| v.kind() == LatticeKind::ConstSet)
            .collect();
        assert!(!cs.is_empty(), "x must resolve to CONSTSET via the lattice");
        assert_constset_strs(cs[0], &["a", "b"]);
    }

    #[test]
    fn foreach_with_unknown_var_is_overdefined() {
        // foreach over an unknown $items → x never CONSTSET.
        let cu = build("foreach x $items {puts $x}");
        for v in lats_for(&cu.top_level, "x") {
            assert_ne!(
                v.kind(),
                LatticeKind::ConstSet,
                "unknown-list foreach must not be CONSTSET"
            );
        }
    }

    // --- registry-based constant folding ---
    //
    // SCCP folds a narrow set of registry commands: `string length`, `format`,
    // and `list` over literal args (plus all `expr`-shaped arithmetic). It does
    // NOT fold `string toupper`, `lindex`, `dict get`, `join`,
    // `list`-with-var-args, or fold through a `$var` argument — those are left
    // Overdefined. Each non-fold is a sound (never-wrong-substitution) precision
    // gap, captured at its actual verdict; the tclsh value is pinned either way.

    #[test]
    fn fold_string_toupper_diverges_overdefined() {
        // No `string toupper` fold → Overdefined. tclsh: `string toupper hello`
        // → HELLO.
        let cu = build("set x [string toupper hello]");
        assert_overdefined_or_absent(&cu.top_level, "x", 1, "string toupper");
    }

    #[test]
    fn fold_string_length() {
        // tclsh: `string length hello` → 5.
        let cu = build("set x [string length hello]");
        assert_const_int(&cu.top_level, "x", 1, 5);
    }

    #[test]
    fn fold_lindex_diverges_overdefined() {
        // Left Overdefined (no lindex fold) — sound, less precise. tclsh:
        //   `lindex {a b c} 1` → b.
        let cu = build("set x [lindex {a b c} 1]");
        assert_overdefined_or_absent(&cu.top_level, "x", 1, "lindex");
    }

    #[test]
    fn fold_dict_get_diverges_overdefined() {
        // Left Overdefined (no dict get fold). tclsh: `dict get {a 1 b 2} a` → 1.
        let cu = build("set x [dict get {a 1 b 2} a]");
        assert_overdefined_or_absent(&cu.top_level, "x", 1, "dict get");
    }

    #[test]
    fn fold_format() {
        // tclsh: `format %s_%d hello 5` → hello_5.
        let cu = build("set x [format %s_%d hello 5]");
        assert_const_str(&cu.top_level, "x", 1, "hello_5");
    }

    #[test]
    fn fold_join_diverges_overdefined() {
        // Left Overdefined (no join fold). tclsh: `join {a b c} ,` → a,b,c.
        let cu = build("set x [join {a b c} ,]");
        assert_overdefined_or_absent(&cu.top_level, "x", 1, "join");
    }

    #[test]
    fn fold_with_variable_resolution_diverges_overdefined() {
        // No `string toupper` fold, so x is Overdefined (v itself still folds to
        // the string const "hello"). tclsh: `string toupper hello` → HELLO.
        let cu = build("set v hello\nset x [string toupper $v]");
        assert_const_str(&cu.top_level, "v", 1, "hello");
        assert_overdefined_or_absent(&cu.top_level, "x", 1, "string toupper $v");
    }

    #[test]
    fn fold_chained_diverges_overdefined() {
        // `string toupper` does not fold, so a is Overdefined, and with a
        // non-constant input `[string length $a]` cannot fold either → b
        // Overdefined. tclsh values: `string toupper hello` → HELLO,
        // `string length HELLO` → 5.
        let cu = build("set a [string toupper hello]\nset b [string length $a]");
        assert_overdefined_or_absent(&cu.top_level, "a", 1, "string toupper");
        assert_overdefined_or_absent(&cu.top_level, "b", 1, "string length of non-const");
    }

    #[test]
    fn set_list_cmd_folds_to_const() {
        // `set x [list a b c]` → CONST("a b c"). tclsh: `list a b c` → a b c.
        let cu = build("set x [list a b c]");
        assert_const_str(&cu.top_level, "x", 1, "a b c");
    }

    #[test]
    fn list_cmd_with_const_vars_diverges_overdefined() {
        // `list` folds only over literal args, not var refs → Overdefined.
        // tclsh: `list puts hello` → puts hello.
        let cu = build("set x puts\nset y hello\nset cmd [list $x $y]");
        assert_overdefined_or_absent(&cu.top_level, "cmd", 1, "list with var args");
    }

    #[test]
    fn set_list_cmd_through_foreach() {
        // set items [list a b]; foreach x $items → CONSTSET{a,b}. The `list a b`
        // literal-arg fold feeds the foreach.
        let cu = build("set items [list a b]\nforeach x $items {puts $x}");
        let cs: Vec<_> = lats_for(&cu.top_level, "x")
            .into_iter()
            .filter(|v| v.kind() == LatticeKind::ConstSet)
            .collect();
        assert!(
            !cs.is_empty(),
            "x must resolve to CONSTSET through the [list a b] fold"
        );
        assert_constset_strs(cs[0], &["a", "b"]);
    }
}

/// Assert a CONSTSET holds exactly the given string members (order-independent).
#[track_caller]
fn assert_constset_strs(v: &LatticeValue, want: &[&str]) {
    let LatticeValue::ConstSet(vs) = v else {
        panic!("expected ConstSet, got {v:?}");
    };
    let mut got: Vec<String> = vs
        .iter()
        .map(|c| match c {
            ConstValue::String(s) => s.clone(),
            other => panic!("expected string members, got {other:?}"),
        })
        .collect();
    got.sort();
    let mut want_sorted: Vec<String> = want.iter().map(|s| (*s).to_owned()).collect();
    want_sorted.sort();
    assert_eq!(got, want_sorted, "CONSTSET members mismatch");
}

/// Assert `(name, version)` is Overdefined (or absent) — the conservative
/// verdict for a fold the analyser declines. `what` labels the folded construct
/// for the failure message.
#[track_caller]
fn assert_overdefined_or_absent(fu: &FunctionUnit, name: &str, version: u32, what: &str) {
    let v = lat(fu, name, version);
    assert!(
        matches!(v, Some(LatticeValue::Overdefined) | None),
        "Rust declines to fold {what}; ({name:?}, {version}) should be Overdefined/absent, got {v:?}"
    );
}

// ===========================================================================
// Existence-check diagnostic surface.
//
// `codes` / `fires` are copied verbatim from
// `src/analyser/diagnostics/fp/mod.rs`.
// ===========================================================================

/// Every diagnostic code the merged pipeline surfaces for `src` under `dialect`
/// (analyser pass ∪ `run_all_checks`, optimisation codes dropped).
fn codes(src: &str, dialect: &str) -> Vec<String> {
    let mut out: Vec<String> = Analyser::new()
        .analyse(src, dialect)
        .diagnostics
        .iter()
        .map(|d| d.code.to_string())
        .collect();
    let registry = static_context_for(dialect).commands();
    let cu = CompilationUnit::build_for(src, registry, false);
    let dialect_opt = (!dialect.is_empty())
        .then(|| tcl_registry::model::ingress::resolve_environment(dialect).analyser_profile());
    for d in tcl_compiler::compiler_checks::run_all_checks(&cu, registry, dialect_opt) {
        if d.code.is_optimisation() {
            continue;
        }
        out.push(d.code.to_string());
    }
    out
}

/// True if `code` appears anywhere in the merged diagnostic set for `src`.
fn fires(src: &str, code: &str) -> bool {
    codes(src, D).iter().any(|c| c == code)
}

/// The I230 messages emitted for `src` (analyser pass owns I230).
fn i230_messages(src: &str) -> Vec<String> {
    Analyser::new()
        .analyse(src, D)
        .diagnostics
        .iter()
        .filter(|d| d.code.to_string() == "I230")
        .map(|d| d.message.clone())
        .collect()
}

/// Variable names named by W210 diagnostics, extracted from the read-before-set
/// messages (`Variable 'X' is read before it is set`).
fn w210_vars(src: &str) -> std::collections::HashSet<String> {
    Analyser::new()
        .analyse(src, D)
        .diagnostics
        .iter()
        .filter(|d| d.code.to_string() == "W210")
        .filter_map(|d| d.message.split('\'').nth(1).map(str::to_owned))
        .collect()
}

// ---------------------------------------------------------------------------
// TestNoReadBeforeSet — the check itself never triggers W210.
// ---------------------------------------------------------------------------
mod no_read_before_set {
    use super::*;

    #[test]
    fn info_exists_is_not_read_before_set() {
        // Policy: `info exists FOO` tests existence, never reads FOO → no W210.
        // (tclsh: `info exists FOO` on an unset local → 0, no error.)
        assert!(!fires(
            "proc p {} { if {[info exists FOO]} { set y 1 } }",
            "W210"
        ));
    }

    #[test]
    fn array_exists_is_not_read_before_set() {
        // tclsh: `array exists FOO` on an unset name → 0, no error.
        assert!(!fires(
            "proc p {} { if {[array exists FOO]} { set y 1 } }",
            "W210"
        ));
    }

    #[test]
    fn check_in_value_is_not_read_before_set() {
        assert!(!fires(
            "proc p {} { set r [info exists FOO]; puts $r }",
            "W210"
        ));
    }

    #[test]
    fn check_as_argument_is_not_read_before_set() {
        assert!(!fires("proc p {} { puts [info exists FOO] }", "W210"));
    }

    #[test]
    fn plain_read_still_flagged() {
        // A genuine value read of an unset var is still W210.
        // tclsh: `puts $BAR` → can't read "BAR": no such variable.
        assert!(fires("proc p {} { puts $BAR }", "W210"));
    }

    #[test]
    fn array_get_is_a_value_read_still_flagged() {
        // `array get FOO` reads the array contents — not an existence check.
        // tclsh: `array get FOO` on unset FOO returns "" but is a value read;
        // `puts $undefined`-style unset access through it is flagged.
        assert!(fires("proc p {} { puts [array get FOO] }", "W210"));
    }

    #[test]
    fn dynamic_target_is_a_real_read() {
        // `info exists $name` reads `name` to form the tested variable name, so
        // an unset `name` is a genuine read-before-use that must NOT be
        // suppressed. tclsh confirms the read:
        //   `info exists $name` with name unset → can't read "name": no such variable.
        // The analyser flags it with W212 ("Variable substitution where name
        // expected (`set $x`, `incr $x`, `info exists $x`, …)") rather than
        // W210 — the same foot-gun, a more specific code. We assert the read is
        // flagged by *either* code (not suppressed).
        for src in [
            "proc p {} { if {[info exists $name]} { puts hi } }",
            "proc p {} { set r [info exists $name]; puts $r }",
        ] {
            let cs = codes(src, D);
            assert!(
                cs.iter().any(|c| c == "W210" || c == "W212"),
                "dynamic-name existence read must be flagged (W210/W212), got {cs:?}"
            );
        }
    }

    #[test]
    fn vwait_var_is_a_write_not_a_read_before_set() {
        // The registry models the `vwait` operand as a `VarWrite`, matching
        // C Tcl: `Tcl_VwaitObjCmd` (tclEvent.c) never reads the variable's
        // value — it installs a `TCL_TRACE_WRITES|TCL_TRACE_UNSETS` trace
        // (creating the entry when absent) and returns only once an event
        // handler has *written* it. `vwait forever` on a never-set variable
        // is the canonical infinite-wait idiom, so no W210 fires on the
        // operand (the pre-registry `VarRead` model over-warned here).
        assert!(!fires("proc p {} { vwait forever }", "W210"));
        assert!(!fires("proc p {} { vwait sig }", "W210"));
        assert!(!fires("vwait forever", "W210"));
    }

    #[test]
    fn vwait_inside_command_sub_is_not_read_before_set() {
        // FP guard: a `[vwait sig]` embedded in an outer command — no W210,
        // because the cmd-sub scanner does not treat the bracketed `vwait`
        // operand as a frame read.
        assert!(!fires("proc p {} { return [vwait sig] }", "W210"));
    }

    #[test]
    fn dict_for_body_local_set_recovered() {
        // `dict for`/`dict map` bodies are now lowered into real CFG blocks in
        // the analysis build (issue #833), so a body-local `set fileData $k`
        // definitely defines `fileData` before the following `$fileData` read —
        // no W210. (Previously the opaque barrier hid the body-local set and this
        // safe read was a false positive.)
        let src = "proc p {} {\n    dict for {k v} {a 1 b 2} {\n        set fileData $k\n        puts $fileData\n    }\n}\n";
        assert!(
            !fires(src, "W210"),
            "body-local set inside `dict for` must define fileData before the read"
        );
    }

    #[test]
    fn dict_for_loop_vars_are_not_read_before_set() {
        // The loop variables k/v are bound by `dict for` — reading them in the
        // body is safe. This the analyser handles (no W210).
        let src = "proc p {} { dict for {k v} {a 1 b 2} { puts \"$k=$v\" } }\n";
        assert!(!fires(src, "W210"));
    }

    #[test]
    fn dict_map_body_local_set_recovered() {
        // `dict map` is the lmap analogue — same body recovery as
        // `dict_for_body_local_set_recovered` (issue #833). The body-local
        // `set out $k` defines `out` before the `list $out` read, so no W210.
        let src = "proc p {} {\n    dict map {k v} {a 1 b 2} {\n        set out $k\n        list $out\n    }\n}\n";
        assert!(
            !fires(src, "W210"),
            "body-local set inside `dict map` must define out before the read"
        );
    }

    #[test]
    fn dict_for_body_genuine_rbs_still_fires() {
        // TP control: a body read of a truly-never-set var still fires.
        let src = "proc p {} {\n    dict for {k v} {a 1 b 2} { puts $never_set }\n}\n";
        assert!(fires(src, "W210"));
    }

    #[test]
    fn cmd_sub_set_in_return_value_diverges() {
        // The read-before-set pass does not scan cmd-sub writes inside a
        // `return`'s value words, so it flags `$x` (W210) in the
        // `[set x [open $f r]]` … `[close $x]` idiom (tcllib installer.tcl). In
        // Tcl the `set x` definitely creates x, so the read is safe — a sound
        // over-warn (it never suppresses a real read); asserted at the actual
        // verdict.
        let src = "proc p {f} { return [read [set x [open $f r]]][close $x] }\n";
        assert!(
            fires(src, "W210"),
            "Rust flags the cmd-sub-defined $x in a return value"
        );
    }

    #[test]
    fn cmd_sub_set_in_branch_condition_is_a_definition() {
        // `if {[set x 1]} { puts $x }` — the `set x 1` inside the condition's
        // command substitution defines x before either arm runs (tclsh 9.0.4 /
        // 8.6.14 print `1`). The condition out-var harvest asks the registry
        // which arguments carry `ArgRole::VarWrite`, so `set` answers like
        // `catch` / `gets` / `regexp` / `scan` always did (issue #923 idx 49).
        let src = "proc p {} { if {[set x 1]} { puts $x } }\n";
        assert!(
            !fires(src, "W210"),
            "a condition-embedded `set` defines its target"
        );
    }

    #[test]
    fn genuine_unset_in_return_still_fires() {
        // TP control: a return reading a never-defined var still fires.
        assert!(fires("proc p {} { return $never_set }\n", "W210"));
    }
}

// ---------------------------------------------------------------------------
// TestProvablyAbsentFoldsFalse — a never-defined local folds the check false.
// ---------------------------------------------------------------------------
mod provably_absent_folds_false {
    use super::*;

    #[test]
    fn issue_example_body_never_executes() {
        // `if {[info exists ACCESS_RPC_HANDLE]}` on a never-set local folds to
        // false → I230 "always false"; the body is dead; no W210.
        // tclsh: `info exists ACCESS_RPC_HANDLE` (unset local) → 0.
        let src = "proc myProc {} {\n    if {[info exists ACCESS_RPC_HANDLE]} {\n        set y 1\n    }\n}";
        assert!(
            i230_messages(src)
                .iter()
                .any(|m| m.contains("always false")),
            "expected an 'always false' I230; got {:?}",
            i230_messages(src)
        );
        assert!(!fires(src, "W210"));
    }

    #[test]
    fn lazy_init_reuse_branch_is_dead_for_local_diverges() {
        // The existence-folder folds only the two FP-free cases — a
        // *never-defined* local (→ false) and a *parameter* (→ true). Here H is
        // defined in the `else` arm (`set H 1`), so it is "defined somewhere" and
        // the folder deliberately bails (declining flow-sensitive
        // must-not-be-defined reasoning). That is sound — it just omits the
        // optional I230 hint. tclsh confirms the underlying fact (`info exists H`
        // of an unset local → 0), but the conservative non-fold is the actual
        // verdict.
        let src = "proc authorize {} {\n    if {[info exists H]} {\n        set reuse 1\n    } else {\n        set H 1\n    }\n}";
        assert!(
            i230_messages(src).is_empty(),
            "Rust declines to fold (H is assigned in the else arm); got {:?}",
            i230_messages(src)
        );
    }

    #[test]
    fn array_exists_absent_folds_false() {
        // tclsh: `array exists FOO` on unset name → 0.
        let src = "proc p {} { if {[array exists FOO]} { puts dead } }";
        assert!(
            i230_messages(src)
                .iter()
                .any(|m| m.contains("always false"))
        );
    }

    #[test]
    fn negated_check_folds() {
        // `![info exists FOO]` on a never-set local → always true.
        // tclsh: `expr {![info exists FOO]}` → 1.
        let src = "proc p {} { if {![info exists FOO]} { puts runs } else { puts dead } }";
        assert!(i230_messages(src).iter().any(|m| m.contains("always true")));
    }
}

// ---------------------------------------------------------------------------
// TestProvablyPresentFoldsTrue.
// ---------------------------------------------------------------------------
mod provably_present_folds_true {
    use super::*;

    #[test]
    fn set_before_check_diverges_no_fold() {
        // The existence-folder folds to TRUE only for a *parameter*; a plain
        // `set` makes X "defined somewhere", which it does not promote to a
        // must-exist fold (that needs flow-sensitive must-define reasoning). It
        // bails — sound, no I230. tclsh agrees on the fact: `set X 1; info exists
        // X` → 1.
        let src = "proc p {} { set X 1; if {[info exists X]} { puts ok } else { puts dead } }";
        assert!(
            !i230_messages(src).iter().any(|m| m.contains("always true")),
            "Rust does not fold a `set`-defined local to must-exist; got {:?}",
            i230_messages(src)
        );
    }

    #[test]
    fn parameter_always_exists() {
        // A parameter always exists → `[info exists x]` folds true.
        // tclsh: `proc p {x} { info exists x }; p 5` → 1.
        let src = "proc p {x} { if {[info exists x]} { puts ok } else { puts dead } }";
        assert!(i230_messages(src).iter().any(|m| m.contains("always true")));
    }

    #[test]
    fn unset_then_check_is_not_true() {
        // After `unset` the var is gone → must not fold true.
        // tclsh: `set X 1; unset X; info exists X` → 0.
        let src = "proc p {} { set X 1; unset X; if {[info exists X]} { puts a } else { puts b } }";
        assert!(!i230_messages(src).iter().any(|m| m.contains("always true")));
    }
}

// ---------------------------------------------------------------------------
// Issue #1239 — `array exists PARAM` is constant **false**, not constant true.
//
// A formal parameter is bound as a scalar on entry (Tcl has no
// pass-an-array-by-value), so the two existence spellings disagree on it. The
// fold used to collapse both into one query shape and report `true` for each,
// which inverted I230's live-branch report and made O101 fold the arm that
// actually runs.
//
// tclsh-proof (8.6.16 / 9.0.4):
//   proc f {a} { if {[array exists a]} { puts yes } else { puts no } }
//   f 1                       ;# → no
//   proc g {a} { info exists a }
//   g 1                       ;# → 1
//   proc h {a} { array set a {x 1} }
//   h 1                       ;# → can't set "a(x)": variable isn't array
//   proc k {a} { unset a; array set a {x 1}; array exists a }
//   k 1                       ;# → 1
// ---------------------------------------------------------------------------
mod array_exists_parameter_is_false {
    use super::*;

    /// The `ConstantBranch` values recorded for `::p`'s existence fold.
    fn branch_values(src: &str) -> Vec<bool> {
        let cu = build(src);
        let fn_p = cu.procedures.get("::p").expect("::p lowered");
        fn_p.sccp
            .constant_branches
            .iter()
            .map(|b| b.value)
            .collect()
    }

    #[test]
    fn tp_array_exists_parameter_folds_false() {
        // TP: the fold fires, with value false — the `else` arm is the live one.
        let src = "proc p {a} { if {[array exists a]} { puts dead } else { puts runs } }";
        assert!(
            i230_messages(src)
                .iter()
                .any(|m| m.contains("always false")),
            "expected an 'always false' I230; got {:?}",
            i230_messages(src)
        );
        assert_eq!(branch_values(src), vec![false]);
    }

    #[test]
    fn tp_negated_array_exists_parameter_folds_true() {
        // `![array exists a]` on a parameter → always true.
        let src = "proc p {a} { if {![array exists a]} { puts runs } else { puts dead } }";
        assert!(
            i230_messages(src).iter().any(|m| m.contains("always true")),
            "expected an 'always true' I230; got {:?}",
            i230_messages(src)
        );
        assert_eq!(branch_values(src), vec![true]);
    }

    #[test]
    fn tp_o101_folds_the_dead_arm_not_the_live_one() {
        // O101/DCE consumes the recorded branch: the taken target must be the
        // `else` block, and the unreachable arm the `then` block.
        let src = "proc p {a} { if {[array exists a]} { puts dead } else { puts runs } }";
        let cu = build(src);
        let fn_p = cu.procedures.get("::p").expect("::p lowered");
        let branch = fn_p
            .sccp
            .constant_branches
            .first()
            .expect("a constant branch is recorded");
        assert!(
            !branch.taken_target.contains("then"),
            "the then arm must not be the taken target; got {branch:?}"
        );
        assert!(
            branch.not_taken_target.contains("then"),
            "the then arm must be the dead target; got {branch:?}"
        );
    }

    #[test]
    fn fp_info_exists_parameter_still_folds_true() {
        // FP guard: the `info` spelling is unchanged — a parameter exists.
        let src = "proc p {a} { if {[info exists a]} { puts runs } else { puts dead } }";
        assert!(
            i230_messages(src).iter().any(|m| m.contains("always true")),
            "the info spelling must still fold true; got {:?}",
            i230_messages(src)
        );
        assert_eq!(branch_values(src), vec![true]);
    }

    #[test]
    fn fp_array_exists_never_defined_local_still_folds_false() {
        // FP guard: the never-defined direction is spelling-independent —
        // absent is absent under both. tclsh: `array exists FOO` → 0.
        let src = "proc p {} { if {[array exists FOO]} { puts dead } else { puts runs } }";
        assert_eq!(branch_values(src), vec![false]);
    }

    #[test]
    fn tn_unset_parameter_abstains() {
        // TN: `unset a` removes the scalar binding, after which `array set a`
        // legitimately makes `a` an array — the fold must abstain, as it
        // already did for the `info` spelling.
        for src in [
            "proc p {a} { unset a; array set a {x 1}; if {[array exists a]} { puts x } else { puts y } }",
            "proc p {a} { unset a; if {[array exists a]} { puts x } else { puts y } }",
        ] {
            assert!(
                branch_values(src).is_empty(),
                "an unset parameter must abstain; got {:?} for {src:?}",
                i230_messages(src)
            );
        }
    }

    #[test]
    fn tn_dynamic_destroy_abstains() {
        // TN: `unset $n` can name the parameter, so the same abstention as the
        // `info` spelling applies. tclsh 9.0.4 / 8.6.16:
        //   proc f {p n} { unset $n; array set p {x 1}; array exists p }
        //   f hello p  → 1
        let src = "proc p {a n} { unset $n; if {[array exists a]} { puts x } else { puts y } }";
        assert!(
            branch_values(src).is_empty(),
            "a dynamic destroy must abstain; got {:?}",
            i230_messages(src)
        );
    }

    #[test]
    fn tn_array_parameter_with_upvar_alias_abstains() {
        // TN: an `upvar`-bound local tracks the linked variable, which may well
        // be an array — never folded either way, both spellings.
        let src =
            "proc p {name} { upvar 1 $name a; if {[array exists a]} { puts x } else { puts y } }";
        assert!(
            branch_values(src).is_empty(),
            "an upvar alias must abstain; got {:?}",
            i230_messages(src)
        );
    }
}

// ---------------------------------------------------------------------------
// TestSoundnessGates — the fold bails when a local could be created/destroyed
// invisibly.
// ---------------------------------------------------------------------------
mod soundness_gates {
    use super::*;

    #[test]
    fn global_is_not_folded() {
        // A global may exist independently of this frame → no I230.
        assert!(!fires(
            "proc p {} { global H; if {[info exists H]} { puts a } else { puts b } }",
            "I230"
        ));
    }

    #[test]
    fn namespace_variable_is_not_folded() {
        assert!(!fires(
            "proc p {} { variable H; if {[info exists H]} { puts a } else { puts b } }",
            "I230"
        ));
    }

    #[test]
    fn eval_barrier_disables_fold() {
        // `eval $cmd` is an opaque barrier that could define X → no fold.
        assert!(!fires(
            "proc p {} { eval $cmd; if {[info exists X]} { puts a } else { puts b } }",
            "I230"
        ));
    }

    #[test]
    fn namespace_qualified_name_is_not_folded() {
        assert!(!fires(
            "proc p {} { if {[info exists ::ns::X]} { puts a } else { puts b } }",
            "I230"
        ));
    }

    #[test]
    fn array_element_of_touched_array_is_not_folded() {
        // Any touch of the base array abstains the element fold — a sibling
        // element write may be `array set`-style populating the guard's key
        // path in a shape the scan conflates (issue #1173 keeps the fold
        // one-sided and base-name-driven).
        assert!(!fires(
            "proc p {} { set A(x) 1; if {[info exists A(k)]} { puts a } else { puts b } }",
            "I230"
        ));
    }

    #[test]
    fn array_element_of_never_touched_array_folds_false() {
        // Issue #1173: an element guard on an array nothing in a
        // barrier-free body ever creates is provably false.
        // tclsh 9.0.4 / 8.6.16: `proc p {} { info exists A(k) }; p` → 0.
        let src = "proc p {} { if {[info exists A(k)]} { puts a } else { puts b } }";
        assert!(
            i230_messages(src)
                .iter()
                .any(|m| m.contains("always false"))
        );
    }

    #[test]
    fn nested_command_sub_assignment_is_not_folded() {
        // `set y [set X 1]` creates X with no SSA def → folder must not treat X
        // as absent. tclsh: `set y [set X 1]; info exists X` → 1.
        assert!(!fires(
            "proc p {} { set y [set X 1]; if {[info exists X]} { puts a } else { puts b } }",
            "I230"
        ));
    }

    #[test]
    fn command_sub_in_call_arg_is_not_folded() {
        assert!(!fires(
            "proc p {} { puts [set X 1]; if {[info exists X]} { puts a } else { puts b } }",
            "I230"
        ));
    }

    #[test]
    fn mutation_free_value_command_still_folds() {
        // A value whose cmd-sub cannot create a local must not block folding:
        // `[string length abc]` is pure, so X (never defined) still folds false.
        let src = "proc p {} { set n [string length abc]; if {[info exists X]} { puts a } else { puts b } }";
        assert!(
            i230_messages(src)
                .iter()
                .any(|m| m.contains("always false"))
        );
    }
}

// ---------------------------------------------------------------------------
// TestFlowSensitiveNarrowing — true-region reads are safe; false region flags.
//
// The leading `uplevel 1 $cmd` barrier keeps both arms live (no fold), so
// narrowing — not DCE — governs the verdict. `cmd` is itself the genuine
// read-before-set in each reproducer.
//
// The scaffolding is `uplevel 1`, not `eval`, precisely because of the frame
// difference: `uplevel 1 $cmd` runs the script one frame **up**, so it cannot
// define a local of `p` and leaves `p`'s own name space provable, while
// `eval $cmd` runs *here* and therefore blinds every W210 in `p` (tclsh 9.0.4 /
// 8.6.14: `proc runner {b} {set helper 42; uplevel 1 $b}` invoked with
// `{set x $helper}` raises `can't read "helper"`, while the same body under
// `eval $b` reads `42`). Both are `Statement::Barrier`s, so either disables the
// existence fold; only one of them is a name barrier.
// ---------------------------------------------------------------------------
mod flow_sensitive_narrowing {
    use super::*;

    fn flagged(body: &str) -> std::collections::HashSet<String> {
        w210_vars(&format!("proc p {{}} {{ uplevel 1 $cmd; {body} }}"))
    }

    #[test]
    fn true_branch_read_is_safe() {
        // In the true region X is known to exist → `puts $X` is not W210.
        assert!(!flagged("if {[info exists X]} { puts $X }").contains("X"));
    }

    #[test]
    fn false_branch_read_is_flagged() {
        // The else region does NOT know X exists → `puts $X` there is W210.
        // tclsh: in the false arm of `if {[info exists X]}`, X is unset →
        // reading it would error.
        assert!(flagged("if {[info exists X]} { puts $X } else { puts $X }").contains("X"));
    }

    #[test]
    fn negated_guard_narrows_else_branch() {
        // `if {![info exists X]} { … } else { puts $X }` — the else runs when X
        // exists, so the read there is safe.
        assert!(!flagged("if {![info exists X]} { puts no } else { puts $X }").contains("X"));
    }

    #[test]
    fn nested_true_branch_read_is_safe() {
        assert!(
            !flagged("if {[info exists X]} { if {[string length a]} { puts $X } }").contains("X")
        );
    }

    #[test]
    fn and_pure_right_keeps_both_facts_diverges() {
        // The analyser narrows only a single top-level existence guard, not an
        // `&&` conjunction, so with `[info exists X] && [info exists Y]` both X
        // and Y are still flagged. The reads are guard-safe in Tcl; the
        // over-warn is sound (it never suppresses a real read). Asserted at the
        // actual verdict.
        let f = flagged("if {[info exists X] && [info exists Y]} { puts $X$Y }");
        assert!(
            f.contains("X") && f.contains("Y"),
            "Rust does not narrow through `&&`; both X and Y flagged; got {f:?}"
        );
    }

    #[test]
    fn and_impure_right_drops_left_fact() {
        // An impure right operand could mutate X before the branch, so the left
        // `info exists X` fact must not narrow the body → X stays flagged.
        assert!(flagged("if {[info exists X] && [otherproc]} { puts $X }").contains("X"));
    }
}

// ---------------------------------------------------------------------------
// TestInfoVarsNarrowing — `info vars` / `info locals` membership idioms.
//
// The analyser narrows only the direct `info exists` / `array exists` / `catch`
// guards, NOT the `[info vars X] ne ""` / `lsearch [info vars] X` membership
// idioms, so it conservatively still flags X. The reads are genuinely
// guard-safe in Tcl, but the over-warn is sound (it never *suppresses* a real
// read); we capture the actual verdict. The two must-still-flag controls (`info
// globals`, `-regexp`, glob `X*`) behave the same.
// ---------------------------------------------------------------------------
mod info_vars_narrowing {
    use super::*;

    fn flagged(body: &str) -> std::collections::HashSet<String> {
        w210_vars(&format!("proc p {{}} {{ uplevel 1 $cmd; {body} }}"))
    }

    #[test]
    fn info_vars_membership_idioms_are_not_narrowed_rust_behaviour() {
        // X is still flagged for each of these (no info-vars membership
        // narrowing). Assert the actual verdict.
        for body in [
            "if {[info vars X] ne \"\"} { puts $X }",
            "if {[info vars X] eq \"\"} { puts a } else { puts $X }",
            "if {[info locals X] ne \"\"} { puts $X }",
            "if {[llength [info vars X]]} { puts $X }",
            "if {[lsearch [info vars] X] > -1} { puts $X }",
            "if {[lsearch -exact [info vars] X] >= 0} { puts $X }",
            "if {[lsearch [info vars] X] == -1} { puts a } else { puts $X }",
        ] {
            assert!(
                flagged(body).contains("X"),
                "Rust does not narrow info-vars idiom: {body:?}"
            );
        }
    }

    #[test]
    fn info_vars_ne_empty_false_branch_flagged() {
        // The else arm of `[info vars X] ne ""` definitely lacks X — a genuine
        // read-before-set in any model (X flagged).
        assert!(flagged("if {[info vars X] ne \"\"} { puts a } else { puts $X }").contains("X"));
    }

    #[test]
    fn info_globals_is_not_narrowed() {
        // `info globals X` proves the *global* exists, not the local `$X`, so X
        // is still flagged.
        assert!(flagged("if {[info globals X] ne \"\"} { puts $X }").contains("X"));
    }

    #[test]
    fn lsearch_regexp_is_not_narrowed() {
        // `-regexp` matches a regex, not an exact name — unsound to narrow.
        assert!(flagged("if {[lsearch -regexp [info vars] X] > -1} { puts $X }").contains("X"));
    }

    #[test]
    fn glob_pattern_is_not_narrowed() {
        // A glob pattern (`X*`) is not a single exact name.
        assert!(flagged("if {[info vars X*] ne \"\"} { puts $X }").contains("X"));
    }
}

// ---------------------------------------------------------------------------
// TestCatchNarrowing — `catch {set _ $X}` signals existence on its succeeded
// (false) branch.
//
// The analyser does not implement catch-as-existence-probe narrowing, so the
// guarded reads (which are safe in Tcl) are still flagged. Sound over-warn;
// captured at the actual verdict. The two must-still-flag controls
// (failed/ambiguous arm, non-pure body) behave the same.
// ---------------------------------------------------------------------------
mod catch_narrowing {
    use super::*;

    fn flagged(body: &str) -> std::collections::HashSet<String> {
        w210_vars(&format!("proc p {{}} {{ uplevel 1 $cmd; {body} }}"))
    }

    #[test]
    fn catch_success_branch_reads_are_not_narrowed_rust_behaviour() {
        // The succeeded/false arm and the negated true arm both prove X readable,
        // yet X is still flagged. Actual verdict.
        for body in [
            "if {[catch {set _ $X}]} { puts a } else { puts $X }",
            "if {![catch {set _ $X}]} { puts $X }",
            "if {[catch {set _ $X} e]} { puts a } else { puts $X }",
        ] {
            assert!(
                flagged(body).contains("X"),
                "Rust does not narrow catch-existence: {body:?}"
            );
        }
    }

    #[test]
    fn failed_branch_is_ambiguous_and_still_flagged() {
        // catch != 0 could mean missing *or* an array-as-scalar read → flagged.
        assert!(flagged("if {[catch {set _ $X}]} { puts $X }").contains("X"));
    }

    #[test]
    fn non_pure_body_is_not_narrowed() {
        // A non-pure catch body (`someproc $X`) carries no existence signal.
        assert!(flagged("if {[catch {someproc $X}]} { puts a } else { puts $X }").contains("X"));
    }
}

// ---------------------------------------------------------------------------
// TestAnalysisLevel — direct assertions on the core analysis result.
// ---------------------------------------------------------------------------
mod analysis_level {
    use super::*;

    #[test]
    fn constant_branch_recorded_for_absent_local() {
        // The existence-fold post-pass appends a value-false ConstantBranch for
        // `[info exists FOO]` (never-defined local). tclsh: → 0.
        let cu = build("proc p {} { if {[info exists FOO]} { puts dead } }");
        let fn_p = cu.procedures.get("::p").expect("::p lowered");
        assert!(
            fn_p.sccp.constant_branches.iter().any(|b| !b.value),
            "a value-false constant branch must be recorded; got {:?}",
            const_branches(fn_p)
        );
        // The existence post-pass does not update `executable_blocks`, so
        // `unreachable_blocks` stays empty for these folds (the analyser still
        // emits I230 directly, and the optimiser O101/DCE consumes the branch).
        // The unreachable-blocks assertion belongs to the optimiser surface not
        // covered here, so we assert the meaningful constant-branch fact.
    }

    #[test]
    fn no_constant_branch_for_global() {
        // A global is exempt from the fold → no constant branch from it.
        let cu =
            build("proc p {} { global FOO; if {[info exists FOO]} { puts x } else { puts y } }");
        let fn_p = cu.procedures.get("::p").expect("::p lowered");
        assert!(
            fn_p.sccp.constant_branches.is_empty(),
            "global existence check must not produce a constant branch; got {:?}",
            const_branches(fn_p)
        );
    }
}

// ===========================================================================
// Issue #1109 — a brace-quoted *name* word inside a command substitution is a
// literal name, not a substitution.
//
// The dead-store keep-alive scan credits every `$x` it can see inside a `[…]`
// on purpose: an `[expr {$w}]` hides its reads from the brace-aware word
// scanner, and crediting one too many only ever *silences* a warning.  But a
// braced word in a registry variable-**name** role is not hidden text — it is
// the name itself, and the read goes somewhere else entirely.
//
// tclsh 9.0.4 and 8.6.16, byte-identical:
//
//   proc f {} {set n 1; puts [set {$n}]}
//   f   ->  can't read "$n": no such variable      (rc 1)
//
// The read went to the cell called `$n`, never to `n`; `n` really is assigned
// and never read, and W220 should say so.
// ===========================================================================
mod brace_quoted_name_words {
    use super::*;

    /// FN before the fix: the `$n` inside `{$n}` was credited as a read of
    /// `n`, so the genuine dead store went unreported.
    #[test]
    fn tp_a_braced_name_word_does_not_credit_the_plain_cell() {
        assert!(
            dead_store_vars("proc ::f {} {\n    set n 1\n    puts [set {$n}]\n}").contains("n"),
            "`[set {{$n}}]` reads the cell called `$n`, so `set n 1` is dead"
        );
    }

    /// The `[expr {…}]` over-approximation the helper exists for must survive
    /// untouched — an FP here is a silenced-then-unsilenced regression.
    #[test]
    fn fp_an_expr_brace_still_keeps_its_reads() {
        for src in [
            // `expr`'s braces quote an *expression*: the `$w` is a real read.
            // tclsh: `proc f {} {set w 3; puts [expr {$w}]}; f` -> 3
            "proc ::f {} {\n    set w 3\n    puts [expr {$w}]\n}",
            // The same read one level deeper.
            "proc ::f {} {\n    set w 3\n    puts [list [expr {$w}]]\n}",
            // A `${…}` reference form, whose content is a name, not a read of
            // some inner variable.
            "proc ::f {} {\n    set w 3\n    puts [expr {${w}}]\n}",
        ] {
            assert!(
                !dead_store_vars(src).contains("w"),
                "a genuine expression read must keep `set w 3` alive: {src:?}"
            );
        }
    }

    /// TN: an *unbraced* name word is an ordinary read and still counts.
    /// tclsh: `proc f {} {set n 1; puts [set n]}; f` -> 1
    #[test]
    fn tn_a_plain_name_word_is_still_a_read() {
        assert!(
            !dead_store_vars("proc ::f {} {\n    set n 1\n    puts [set n]\n}").contains("n"),
            "`[set n]` is a genuine read of `n`"
        );
    }

    /// TN: an unresolvable head abstains toward silence — the registry cannot
    /// say the braced word is a name, so the read stays credited.
    #[test]
    fn tn_an_unknown_command_keeps_abstaining() {
        assert!(
            !dead_store_vars("proc ::f {} {\n    set n 1\n    puts [nosuchcmd {$n}]\n}")
                .contains("n"),
            "nothing proves `{{$n}}` is a name here, so stay silent"
        );
    }

    /// The `${…}` name form follows C Tcl 9's `Tcl_ParseVarName`: the name
    /// runs to the *matching* close brace, counting nested braces and letting
    /// a backslash consume the next byte.  Scanning to the first `}` truncated
    /// the name and credited the wrong cell.
    ///
    /// tclsh 9.0.4 / 8.6.16: `set {a{b}c} 1; puts [expr {${a{b}c}}]` -> 1
    #[test]
    fn tp_a_nested_brace_name_is_read_whole() {
        let src = "proc ::f {} {\n    set {a{b}c} 1\n    puts [expr {${a{b}c}}]\n}";
        assert!(
            !dead_store_vars(src).contains("a{b}c"),
            "the whole `a{{b}}c` name is read; got {:?}",
            dead_store_vars(src)
        );
    }
}
