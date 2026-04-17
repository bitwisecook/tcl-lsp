"""Tests for Phase 3 core CFG+SSA analyses."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.core_analyses import LatticeKind, analyse_source
from core.compiler.types import TclType, TypeKind


class TestCoreAnalyses:
    def test_call_arguments_contribute_variable_uses(self):
        analysis = analyse_source("set a 1\nputs $a").top_level
        assert not any(d.variable == "a" for d in analysis.dead_stores)

    def test_liveness_ignores_intrablock_def_use(self):
        analysis = analyse_source("set a 1\nset b [expr {$a + 1}]").top_level
        assert not any(("a", 1) in live for live in analysis.live_in.values())

    def test_incr_uses_previous_definition(self):
        analysis = analyse_source("set count 3\nincr count\nputs $count").top_level
        assert not any(d.variable == "count" and d.version == 1 for d in analysis.dead_stores)

    def test_dead_store_for_overwritten_assignment(self):
        analysis = analyse_source("set a 1\nset a 2\nset b [expr {$a + 0}]").top_level
        dead_a = [d for d in analysis.dead_stores if d.variable == "a"]
        assert len(dead_a) == 1
        assert dead_a[0].version == 1

    def test_phi_incoming_values_mark_branch_defs_used(self):
        source = "if {$cond} {set a 1} else {set a 2}\nset out [expr {$a + 0}]"
        analysis = analyse_source(source).top_level
        assert not any(d.variable == "a" for d in analysis.dead_stores)

    def test_constant_branch_marks_not_taken_target_unreachable(self):
        source = "set a 1\nif {$a} {set x 1} else {set y 2}\nset out [expr {$x + 0}]"
        analysis = analyse_source(source).top_level
        assert len(analysis.constant_branches) == 1
        branch = analysis.constant_branches[0]
        assert branch.condition == "$a"
        assert branch.value is True
        assert branch.not_taken_target in analysis.unreachable_blocks

    def test_static_for_loop_feeds_branch_constant_folding(self):
        source = """
set total 0
for {set i 0} {$i < 5} {incr i} {
    set total [expr {$total + $i}]
}
if {$total > 5} {
    puts "total is $total"
} else {
    puts "small total"
}
"""
        analysis = analyse_source(source).top_level
        assert len(analysis.constant_branches) == 1
        branch = analysis.constant_branches[0]
        assert branch.condition == "$total > 5"
        assert branch.value is True
        assert branch.not_taken_target in analysis.unreachable_blocks

    def test_unreachable_branch_defs_are_not_dead_stores(self):
        source = "if {0} {set x 1}\nset y 2\nset z [expr {$y + 0}]"
        analysis = analyse_source(source).top_level
        dead_vars = {d.variable for d in analysis.dead_stores}
        assert "x" not in dead_vars
        assert any(not b.value for b in analysis.constant_branches)

    def test_sccp_propagates_expression_constants(self):
        source = "set a 1\nset b [expr {$a + 2}]\nset c [expr {$b * 3}]"
        analysis = analyse_source(source).top_level
        assert analysis.values[("a", 1)].kind is LatticeKind.CONST
        assert analysis.values[("a", 1)].value == 1
        assert analysis.values[("b", 1)].kind is LatticeKind.CONST
        assert analysis.values[("b", 1)].value == 3
        assert analysis.values[("c", 1)].kind is LatticeKind.CONST
        assert analysis.values[("c", 1)].value == 9

    def test_sccp_does_not_propagate_nan_as_constant(self):
        # Verified against tclsh 9.0.3: any expression that evaluates to
        # NaN raises ARITH DOMAIN — so even if an input variable carries a
        # NaN constant (e.g. produced by a future code path, a decoded
        # literal, or an upstream analysis), the lattice evaluator must
        # widen the expression to OVERDEFINED.  Otherwise downstream passes
        # could substitute ``NaN`` into the source and defeat the runtime
        # error.  This directly exercises the guard in
        # ``_substitute_expr_with_lattice``.
        import math

        from core.compiler.core_analyses import (
            LatticeValue,
            _substitute_expr_with_lattice,
        )
        from core.compiler.expr_ast import BinOp, ExprBinary, ExprLiteral, ExprVar

        # Evaluating ``$x + 0`` with lattice x=CONST(nan) must widen.
        expr = ExprBinary(
            op=BinOp.ADD,
            left=ExprVar(text="$x", name="x", start=0, end=0),
            right=ExprLiteral(text="0", start=0, end=0),
        )
        lv = _substitute_expr_with_lattice(
            expr,
            uses={"x": 1},
            values={("x", 1): LatticeValue.const(math.nan)},
        )
        assert lv.kind is LatticeKind.OVERDEFINED

        # Sanity: non-NaN constants still propagate.
        lv_ok = _substitute_expr_with_lattice(
            expr,
            uses={"x": 1},
            values={("x", 1): LatticeValue.const(5)},
        )
        assert lv_ok.kind is LatticeKind.CONST
        assert lv_ok.value == 5

    def test_format_constant_rejects_nan(self):
        # Defensive guard in the optimiser's constant-to-source helper:
        # a NaN float must never be formatted into a substitutable string,
        # because substituting it would bypass Tcl 9.0.3's ARITH DOMAIN.
        # (Non-NaN floats — Inf, -Inf, finite values — continue to format
        # through ``format_tcl_value`` and are covered by its own tests.)
        import math

        from core.compiler.optimiser._helpers import _format_constant

        assert _format_constant(math.nan) is None
        assert _format_constant(-math.nan) is None
        # Regular floats and ints still format to a non-None string.
        assert _format_constant(42) == "42"
        assert _format_constant(True) == "1"
        assert _format_constant(False) == "0"

    def test_analysis_runs_for_lowered_procedures(self):
        source = """
proc add_static {} {
    set a 1
    set b [expr {$a + 2}]
    return $b
}
"""
        mod = analyse_source(source)
        assert "::add_static" in mod.procedures
        proc = mod.procedures["::add_static"]
        assert proc.values[("a", 1)].kind is LatticeKind.CONST
        assert proc.values[("a", 1)].value == 1
        assert proc.values[("b", 1)].kind is LatticeKind.CONST
        assert proc.values[("b", 1)].value == 3


class TestNewLoweringAnalysis:
    """Tests verifying the full pipeline works through newly lowered nodes."""

    def test_while_body_visible_to_sccp(self):
        source = "set i 0\nwhile {$i < 3} {incr i}\nputs $i"
        analysis = analyse_source(source).top_level
        # i should not be a dead store since it's used in the condition and by puts.
        assert not any(d.variable == "i" and d.version == 1 for d in analysis.dead_stores)

    def test_foreach_var_not_dead(self):
        source = "foreach x {1 2 3} {puts $x}"
        analysis = analyse_source(source).top_level
        # The foreach iteration variable x is used by puts, so not dead.
        assert not any(d.variable == "x" for d in analysis.dead_stores)

    def test_catch_result_var_visible(self):
        source = "catch {expr {1/0}} result\nputs $result"
        analysis = analyse_source(source).top_level
        # result is used by puts, not dead.
        assert not any(d.variable == "result" for d in analysis.dead_stores)

    def test_try_handler_var_visible(self):
        source = "try {set x 1} on error {msg opts} {puts $msg}\nputs done"
        analysis = analyse_source(source).top_level
        # msg is used inside the handler.
        assert not any(d.variable == "msg" for d in analysis.dead_stores)

    def test_append_var_is_def(self):
        source = "set s hello\nappend s world\nputs $s"
        analysis = analyse_source(source).top_level
        assert not any(d.variable == "s" for d in analysis.dead_stores)

    def test_lappend_type_is_list(self):
        source = "lappend items hello"
        analysis = analyse_source(source).top_level
        # lappend should produce LIST type.
        items_types = {
            k: v for k, v in analysis.types.items() if k[0] == "items" and v.kind is TypeKind.KNOWN
        }
        if items_types:
            for _key, type_val in items_types.items():
                assert type_val.tcl_type is TclType.LIST

    def test_dict_set_var_is_def(self):
        source = "dict set d key val\nputs $d"
        analysis = analyse_source(source).top_level
        assert not any(d.variable == "d" for d in analysis.dead_stores)


class TestInterpolationFolding:
    """Constant-folding through string interpolation."""

    def test_interpolation_folds_to_string_const(self):
        """``set c "${c}0"`` with c=1 folds to CONST("10")."""
        source = 'set c [expr {0+1}]\nset c "${c}0"'
        analysis = analyse_source(source).top_level
        assert analysis.values[("c", 1)].kind is LatticeKind.CONST
        assert analysis.values[("c", 1)].value == 1
        assert analysis.values[("c", 2)].kind is LatticeKind.CONST
        assert analysis.values[("c", 2)].value == "10"  # string, not int

    def test_interpolation_result_typed_as_string(self):
        """Interpolated value should be typed STRING regardless of content."""
        source = 'set c [expr {0+1}]\nset c "${c}0"'
        analysis = analyse_source(source).top_level
        c2_type = analysis.types.get(("c", 2))
        assert c2_type is not None
        assert c2_type.kind is TypeKind.KNOWN
        assert c2_type.tcl_type is TclType.STRING

    def test_interpolation_multiple_vars(self):
        """Multiple variable substitutions fold correctly."""
        source = 'set a 1\nset b 2\nset c "${a}_${b}"'
        analysis = analyse_source(source).top_level
        assert analysis.values[("c", 1)].kind is LatticeKind.CONST
        assert analysis.values[("c", 1)].value == "1_2"

    def test_interpolation_with_command_sub_is_overdefined(self):
        """Command substitution mixed with variable interpolation → OVERDEFINED."""
        source = 'set a 1\nset c "${a}[clock seconds]"'
        analysis = analyse_source(source).top_level
        c_vals = {k: v for k, v in analysis.values.items() if k[0] == "c"}
        for _k, v in c_vals.items():
            assert v.kind is not LatticeKind.CONST

    def test_incr_with_string_const_increment(self):
        """incr folds through a string const parseable as int."""
        source = 'set c [expr {0+1}]\nset c "${c}0"\nset b 1\nincr b $c'
        analysis = analyse_source(source).top_level
        assert analysis.values[("b", 2)].kind is LatticeKind.CONST
        assert analysis.values[("b", 2)].value == 11

    def test_interpolation_folds_in_proc(self):
        """Interpolation folding works inside a proc body."""
        source = """\
proc add {a b} {
    set c [expr {0+1}]
    set c "${c}0"
    incr b $c
    return $c
}
"""
        mod = analyse_source(source)
        proc = mod.procedures["::add"]
        # c₁ folds to CONST(1), c₂ folds to CONST("10") via interpolation
        assert proc.values[("c", 1)].kind is LatticeKind.CONST
        assert proc.values[("c", 1)].value == 1
        assert proc.values[("c", 2)].kind is LatticeKind.CONST
        assert proc.values[("c", 2)].value == "10"
        # b is a parameter → UNKNOWN, so incr b $c can't fold b₂
        # but c₂ = "10" is correctly tracked as a string const

    def test_incr_non_numeric_string_const_is_overdefined(self):
        """incr with a string const that is not an integer → OVERDEFINED."""
        source = "set s hello\nset x 0\nincr x $s"
        analysis = analyse_source(source).top_level
        x_val = analysis.values.get(("x", 2))
        assert x_val is None or x_val.kind is LatticeKind.OVERDEFINED

    def test_shimmer_preserved_after_folding(self):
        """Shimmer warning on incr increment arg is still emitted."""
        from core.compiler.shimmer import ShimmerWarning, find_shimmer_warnings

        source = 'set c [expr {0+1}]\nset c "${c}0"\nset b 1\nincr b $c'
        warnings = find_shimmer_warnings(source)
        shimmer = [
            w
            for w in warnings
            if isinstance(w, ShimmerWarning) and w.variable == "c" and w.command == "incr"
        ]
        assert shimmer, f"Shimmer on c should still be detected, got {warnings}"
        assert shimmer[0].from_type is TclType.STRING
        assert shimmer[0].to_type is TclType.INT


class TestVariableShapeCoreAnalyses:
    """Variable-shape handling in CFG/SSA/type consumers."""

    def test_namespaced_scalar_flows_with_qualified_name(self):
        analysis = analyse_source('set ::ns::x "alpha beta"\nset n [llength $::ns::x]').top_level
        assert ("::ns::x", 1) in analysis.types

    def test_namespaced_array_element_flows_as_base_array_name(self):
        analysis = analyse_source(
            'set ::ns::arr(k) "alpha beta"\nset n [llength $::ns::arr(k)]'
        ).top_level
        assert ("::ns::arr", 1) in analysis.types

    def test_braced_scalar_like_array_name_flows_as_scalar_symbol(self):
        analysis = analyse_source('set {a(1)} "alpha beta"\nset n [llength ${a(1)}]').top_level
        assert ("a", 1) in analysis.types

    def test_unbraced_array_ref_flows_as_base_array_symbol(self):
        analysis = analyse_source('set a(1) "alpha beta"\nset n [llength $a(1)]').top_level
        assert ("a", 1) in analysis.types

    def test_namespace_var_commands_with_qualified_names_do_not_break_analysis(self):
        source = """\
namespace eval ::demo {
    variable value 1
}
proc wire_namespace_vars {} {
    global ::demo::value
    upvar 0 ::demo::value local_value
    unset -nocomplain ::demo::value
}
"""
        analysis = analyse_source(source).top_level
        assert not analysis.dead_stores

    # --- CONSTSET lattice tests ---

    def test_foreach_braced_list_constset(self):
        """foreach over a braced literal list produces CONSTSET."""
        source = "foreach x {a b c} {puts $x}"
        analysis = analyse_source(source).top_level
        # x should be CONSTSET with values {a, b, c}
        x_vals = [
            v
            for (name, _ver), v in analysis.values.items()
            if name == "x" and v.kind is LatticeKind.CONSTSET
        ]
        assert len(x_vals) >= 1
        assert x_vals[0].values == frozenset({"a", "b", "c"})

    def test_foreach_list_cmd_constset(self):
        """foreach over [list ...] with literal args produces CONSTSET."""
        source = "foreach x [list a b c] {puts $x}"
        analysis = analyse_source(source).top_level
        x_vals = [
            v
            for (name, _ver), v in analysis.values.items()
            if name == "x" and v.kind is LatticeKind.CONSTSET
        ]
        assert len(x_vals) >= 1
        assert x_vals[0].values == frozenset({"a", "b", "c"})

    def test_foreach_single_element_is_const(self):
        """foreach over a single-element list produces CONST (not CONSTSET)."""
        source = "foreach x {only} {puts $x}"
        analysis = analyse_source(source).top_level
        x_vals = [
            v
            for (name, _ver), v in analysis.values.items()
            if name == "x" and v.kind is LatticeKind.CONST
        ]
        assert len(x_vals) >= 1
        assert x_vals[0].value == "only"

    def test_foreach_with_var_ref_resolves_through_lattice(self):
        """foreach over $items where items is a known constant resolves to CONSTSET."""
        source = "set items {a b}\nforeach x $items {puts $x}"
        analysis = analyse_source(source).top_level
        x_vals = [
            v
            for (name, _ver), v in analysis.values.items()
            if name == "x" and v.kind is LatticeKind.CONSTSET
        ]
        assert len(x_vals) >= 1
        assert x_vals[0].values == frozenset({"a", "b"})

    def test_foreach_with_unknown_var_is_overdefined(self):
        """foreach over $items where items is unknown remains OVERDEFINED."""
        source = "foreach x $items {puts $x}"
        analysis = analyse_source(source).top_level
        x_vals = [v for (name, _ver), v in analysis.values.items() if name == "x"]
        for v in x_vals:
            assert v.kind is not LatticeKind.CONSTSET

    # --- Registry-based constant folding tests ---

    def test_fold_string_toupper(self):
        source = "set x [string toupper hello]"
        analysis = analyse_source(source).top_level
        assert analysis.values[("x", 1)].kind is LatticeKind.CONST
        assert analysis.values[("x", 1)].value == "HELLO"

    def test_fold_string_length(self):
        source = "set x [string length hello]"
        analysis = analyse_source(source).top_level
        assert analysis.values[("x", 1)].kind is LatticeKind.CONST
        assert analysis.values[("x", 1)].value == 5

    def test_fold_lindex(self):
        source = "set x [lindex {a b c} 1]"
        analysis = analyse_source(source).top_level
        assert analysis.values[("x", 1)].kind is LatticeKind.CONST
        assert analysis.values[("x", 1)].value == "b"

    def test_fold_llength(self):
        source = "set x [llength {a b c}]"
        analysis = analyse_source(source).top_level
        assert analysis.values[("x", 1)].kind is LatticeKind.CONST
        assert analysis.values[("x", 1)].value == 3

    def test_fold_dict_get(self):
        source = "set x [dict get {a 1 b 2} a]"
        analysis = analyse_source(source).top_level
        assert analysis.values[("x", 1)].kind is LatticeKind.CONST
        assert analysis.values[("x", 1)].value == 1

    def test_fold_format(self):
        source = "set x [format %s_%d hello 5]"
        analysis = analyse_source(source).top_level
        assert analysis.values[("x", 1)].kind is LatticeKind.CONST
        assert analysis.values[("x", 1)].value == "hello_5"

    def test_fold_join(self):
        source = "set x [join {a b c} ,]"
        analysis = analyse_source(source).top_level
        assert analysis.values[("x", 1)].kind is LatticeKind.CONST
        assert analysis.values[("x", 1)].value == "a,b,c"

    def test_fold_with_variable_resolution(self):
        """Fold through variable references: [string toupper $v]."""
        source = "set v hello\nset x [string toupper $v]"
        analysis = analyse_source(source).top_level
        assert analysis.values[("x", 1)].kind is LatticeKind.CONST
        assert analysis.values[("x", 1)].value == "HELLO"

    def test_fold_chained(self):
        """Fold chained operations via SCCP fixed-point."""
        source = "set a [string toupper hello]\nset b [string length $a]"
        analysis = analyse_source(source).top_level
        assert analysis.values[("a", 1)].value == "HELLO"
        assert analysis.values[("b", 1)].value == 5

    def test_set_list_cmd_folds_to_const(self):
        """set x [list a b c] should fold to CONST."""
        source = "set x [list a b c]"
        analysis = analyse_source(source).top_level
        x_vals = [
            v
            for (name, _ver), v in analysis.values.items()
            if name == "x" and v.kind is LatticeKind.CONST
        ]
        assert len(x_vals) >= 1
        assert x_vals[0].value == "a b c"

    def test_list_cmd_with_const_vars_folds(self):
        """[list $x $y] with known-const vars should fold to CONST."""
        source = "set x puts\nset y hello\nset cmd [list $x $y]"
        analysis = analyse_source(source).top_level
        cmd_vals = [
            v
            for (name, _ver), v in analysis.values.items()
            if name == "cmd" and v.kind is LatticeKind.CONST
        ]
        assert len(cmd_vals) >= 1
        assert cmd_vals[0].value == "puts hello"

    def test_set_list_cmd_through_foreach(self):
        """set items [list a b]; foreach x $items should resolve CONSTSET."""
        source = "set items [list a b]\nforeach x $items {puts $x}"
        analysis = analyse_source(source).top_level
        x_vals = [
            v
            for (name, _ver), v in analysis.values.items()
            if name == "x" and v.kind is LatticeKind.CONSTSET
        ]
        assert len(x_vals) >= 1
        assert x_vals[0].values == frozenset({"a", "b"})
