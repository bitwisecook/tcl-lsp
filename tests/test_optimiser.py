"""Tests for static Tcl source optimiser."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.optimiser import (
    demorgan_transform,
    find_optimisations,
    invert_expression,
    optimise_source,
)


class TestOptimiser:
    def test_propagates_and_folds_expr(self):
        source = "set a 1\nset b [expr {$a + 2}]"
        optimised, rewrites = optimise_source(source)
        assert optimised == "set b 3"
        assert any(r.code == "O102" for r in rewrites)
        assert any(r.code == "O109" for r in rewrites)

    def test_division_now_folds(self):
        source = "set a 1\nset b [expr {$a / 2}]"
        optimised, rewrites = optimise_source(source)
        assert optimised == "set b 0"
        assert any(r.code == "O102" for r in rewrites)

    def test_non_static_assignment_is_not_propagated(self):
        source = "set a [clock seconds]\nset b [expr {$a + 2}]"
        optimised, rewrites = optimise_source(source)
        assert optimised == source
        assert rewrites == []

    def test_reassignment_updates_constant(self):
        source = "set a 1\nset a 5\nset b [expr {$a + 2}]"
        optimised, rewrites = optimise_source(source)
        assert optimised == "set b 7"
        assert any(r.code == "O102" for r in rewrites)

    def test_chained_constant_folding(self):
        source = "set a 1\nset b [expr {$a + 2}]\nset c [expr {$b + 5}]"
        optimised, rewrites = optimise_source(source)
        assert optimised == "set c 8"
        assert any(r.code == "O102" for r in rewrites)

    def test_unset_clears_constant(self):
        source = "set a 1\nunset a\nset b [expr {$a + 2}]"
        optimised, rewrites = optimise_source(source)
        assert optimised == source
        assert rewrites == []

    def test_proc_body_is_optimised(self):
        source = textwrap.dedent("""\
            proc add_two {} {
                set a 1
                return [expr {$a + 2}]
            }
        """)
        optimised, rewrites = optimise_source(source)
        assert "return 3" in optimised
        assert any(r.code == "O102" for r in rewrites)

    def test_direct_expr_command_substitution_folds(self):
        source = "set v [expr {3}]"
        optimised, rewrites = optimise_source(source)
        assert optimised == "set v 3"
        assert any(r.code == "O102" for r in rewrites)

    def test_escaping_command_substitution_store_not_eliminated(self):
        """Escaping substitutions (eval/uplevel/upvar-like) are non-removable."""
        source = "set n [eval $s]\nset n 1\nputs $n\n"
        optimised, rewrites = optimise_source(source)

        assert optimised.splitlines()[0] == "set n [eval $s]"
        assert any(r.code == "O109" for r in rewrites)

    def test_find_optimisations_sorted_by_source_range(self):
        source = "set a 1\nset b [expr {$a + 2}]\nset c [expr {$a + 3}]"
        rewrites = find_optimisations(source)
        offsets = [r.range.start.offset for r in rewrites]
        assert offsets == sorted(offsets)

    def test_interprocedural_static_proc_call_folds(self):
        source = textwrap.dedent("""\
            proc one {} { return 1 }
            set v [one]
        """)
        optimised, rewrites = optimise_source(source)
        assert "set v 1" in optimised
        assert any(r.code == "O103" for r in rewrites)

    def test_interprocedural_passthrough_folds_static_arg(self):
        source = textwrap.dedent("""\
            proc id {x} { return $x }
            set a 7
            set v [id $a]
        """)
        optimised, rewrites = optimise_source(source)
        assert "set v 7" in optimised
        assert any(r.code == "O103" for r in rewrites)

    def test_interprocedural_non_pure_proc_not_folded(self):
        source = textwrap.dedent("""\
            proc noisy {} { puts hi; return 1 }
            set v [noisy]
        """)
        optimised, rewrites = optimise_source(source)
        assert optimised == source
        assert not any(r.code == "O103" for r in rewrites)

    def test_interprocedural_namespace_resolution(self):
        source = textwrap.dedent("""\
            namespace eval math {
                proc one {} { return 1 }
                proc use {} { return [one] }
            }
        """)
        optimised, rewrites = optimise_source(source)
        assert "return 1" in optimised
        assert any(r.code == "O103" for r in rewrites)

    def test_does_not_propagate_outer_constants_into_loop_body(self):
        source = textwrap.dedent("""\
            set total 0
            for {set i 0} {$i < 5} {incr i} {
                set total [expr {$total + $i}]
            }
        """)
        optimised, rewrites = optimise_source(source)
        assert optimised == source
        assert rewrites == []

    def test_static_for_loop_enables_post_loop_constant_folding(self):
        source = textwrap.dedent("""\
            set total 0
            for {set i 0} {$i < 5} {incr i} {
                set total [expr {$total + $i}]
            }
            if {$total > 5} {
                puts ok
            }
        """)
        optimised, rewrites = optimise_source(source)
        assert "if {1}" in optimised
        assert any(r.code == "O101" for r in rewrites)

    def test_folds_static_append_chain_to_single_set(self):
        source = "set msg {Hello}\nappend msg { }\nappend msg World"
        optimised, rewrites = optimise_source(source)
        assert optimised == "set msg {Hello World}"
        assert any(r.code == "O104" for r in rewrites)

    def test_does_not_fold_append_chain_with_dynamic_word(self):
        source = "set msg {}\nappend msg $name\nappend msg !"
        optimised, rewrites = optimise_source(source)
        assert optimised == source
        assert not any(r.code == "O104" for r in rewrites)

    def test_folds_write_chain_across_non_reading_statement(self):
        source = "set msg {Hello}\nputs ok\nappend msg { World}"
        optimised, rewrites = optimise_source(source)
        assert "append msg" not in optimised
        assert "set msg {Hello World}" in optimised
        assert any(r.code == "O104" for r in rewrites)

    def test_does_not_fold_write_chain_when_value_is_read(self):
        source = "set msg {Hello}\nputs $msg\nappend msg { World}"
        optimised, rewrites = optimise_source(source)
        # O104 (string write chain) must not fire when there's a read between writes.
        assert not any(r.code == "O104" for r in rewrites)
        # O105 may propagate the constant into puts (correct behaviour).
        assert "append msg { World}" in optimised

    def test_interprocedural_param_dependent_fold_add(self):
        source = textwrap.dedent("""\
            proc add {a b} { return [expr {$a + $b}] }
            set v [add 3 4]
        """)
        optimised, rewrites = optimise_source(source)
        assert "set v 7" in optimised
        assert any(r.code == "O103" for r in rewrites)

    def test_interprocedural_param_dependent_fold_max(self):
        source = textwrap.dedent("""\
            proc max {a b} {
                if {$a > $b} { return $a } else { return $b }
            }
            set v [max 3 7]
        """)
        optimised, rewrites = optimise_source(source)
        assert "set v 7" in optimised
        assert any(r.code == "O103" for r in rewrites)

    def test_interprocedural_param_dependent_fold_incr(self):
        source = textwrap.dedent("""\
            proc inc3 {x} { incr x 3; return $x }
            set v [inc3 10]
        """)
        optimised, rewrites = optimise_source(source)
        assert "set v 13" in optimised
        assert any(r.code == "O103" for r in rewrites)

    def test_proc_call_inside_expr_arg_folds(self):
        source = textwrap.dedent("""\
            proc one {} { return 1 }
            if {[one] != 0} {
                puts yes
            }
        """)
        optimised, rewrites = optimise_source(source)
        assert "if {1}" in optimised
        assert any(r.code == "O101" for r in rewrites)

    def test_proc_call_with_args_inside_expr_arg_folds(self):
        source = textwrap.dedent("""\
            proc add {a b} { return [expr {$a + $b}] }
            if {[add 3 4] == 7} {
                puts yes
            }
        """)
        optimised, rewrites = optimise_source(source)
        assert "if {1}" in optimised
        assert any(r.code == "O101" for r in rewrites)

    def test_proc_call_inside_expr_with_variable_folds(self):
        source = textwrap.dedent("""\
            proc one {} { return 1 }
            set x 5
            if {[one] + $x == 6} {
                puts yes
            }
        """)
        optimised, rewrites = optimise_source(source)
        assert "if {1}" in optimised
        assert any(r.code == "O101" for r in rewrites)

    def test_impure_proc_call_inside_expr_not_folded(self):
        source = textwrap.dedent("""\
            proc noisy {} { puts hi; return 1 }
            if {[noisy] == 1} {
                puts yes
            }
        """)
        optimised, rewrites = optimise_source(source)
        assert "[noisy]" in optimised
        assert not any(r.code == "O101" for r in rewrites)

    def test_dse_eliminates_dead_store(self):
        source = "set a 1\nset a 2\nputs $a"
        optimised, rewrites = optimise_source(source)
        assert optimised == "puts 2"
        assert any(r.code == "O100" for r in rewrites)
        assert any(r.code == "O109" for r in rewrites)

    def test_adce_eliminates_transitively_dead_stores(self):
        source = "set a 1\nset a [expr {$a + 1}]\nset a 5\nputs $a"
        optimised, rewrites = optimise_source(source)
        assert optimised == "puts 5"
        assert any(r.code == "O100" for r in rewrites)
        assert any(r.code == "O108" for r in rewrites)

    def test_dce_eliminates_unreachable_block_statements(self):
        source = textwrap.dedent("""\
            if {0} {
                puts never
                set x 1
            }
            puts always
        """)
        optimised, rewrites = optimise_source(source)
        assert "puts never" not in optimised
        assert "set x 1" not in optimised
        assert "puts always" in optimised
        assert any(r.code in ("O107", "O112") for r in rewrites)

    def test_instcombine_reassociates_constant_addends(self):
        source = "set v [expr {$a + 1 + 2}]"
        optimised, rewrites = optimise_source(source)
        assert "set v [expr {$a + 3}]" in optimised
        assert any(r.code == "O110" for r in rewrites)

    # Identity / absorbing element rules

    def test_instcombine_pow_zero(self):
        source = "set v [expr {$x ** 0}]"
        optimised, _ = optimise_source(source)
        assert "set v 1" in optimised

    def test_instcombine_pow_one(self):
        source = "set v [expr {$x ** 1}]"
        optimised, rewrites = optimise_source(source)
        assert any(r.code == "O110" for r in rewrites)

    def test_instcombine_shift_zero(self):
        source = "set v [expr {$x << 0}]"
        _, rewrites = optimise_source(source)
        assert any(r.code == "O110" for r in rewrites)

    def test_instcombine_rshift_zero(self):
        source = "set v [expr {$x >> 0}]"
        _, rewrites = optimise_source(source)
        assert any(r.code == "O110" for r in rewrites)

    def test_instcombine_bitand_zero(self):
        source = "set v [expr {$x & 0}]"
        optimised, _ = optimise_source(source)
        assert "set v 0" in optimised

    def test_instcombine_bitor_zero(self):
        source = "set v [expr {$x | 0}]"
        _, rewrites = optimise_source(source)
        assert any(r.code == "O110" for r in rewrites)

    def test_instcombine_bitxor_zero(self):
        source = "set v [expr {$x ^ 0}]"
        _, rewrites = optimise_source(source)
        assert any(r.code == "O110" for r in rewrites)

    def test_instcombine_mod_one(self):
        source = "set v [expr {$x % 1}]"
        optimised, _ = optimise_source(source)
        assert "set v 0" in optimised

    # Boolean simplifications

    def test_instcombine_and_zero(self):
        source = "set v [expr {$x && 0}]"
        optimised, _ = optimise_source(source)
        assert "set v 0" in optimised

    def test_instcombine_and_one(self):
        source = "set v [expr {$x && 1}]"
        _, rewrites = optimise_source(source)
        assert any(r.code == "O110" for r in rewrites)

    def test_instcombine_or_one(self):
        source = "set v [expr {$x || 1}]"
        optimised, _ = optimise_source(source)
        assert "set v 1" in optimised

    def test_instcombine_or_zero(self):
        source = "set v [expr {$x || 0}]"
        _, rewrites = optimise_source(source)
        assert any(r.code == "O110" for r in rewrites)

    def test_instcombine_double_not_boolean_expr(self):
        """!!($a == $b) → $a == $b because == is already boolean."""
        source = "set v [expr {!!($a == $b)}]"
        optimised, _ = optimise_source(source)
        assert "set v [expr {$a == $b}]" in optimised

    def test_instcombine_not_eq(self):
        """!($a == $b) → $a != $b."""
        source = "set v [expr {!($a == $b)}]"
        optimised, _ = optimise_source(source)
        assert "set v [expr {$a != $b}]" in optimised

    def test_instcombine_not_lt(self):
        """!($a < $b) → $a >= $b."""
        source = "set v [expr {!($a < $b)}]"
        optimised, _ = optimise_source(source)
        assert "set v [expr {$a >= $b}]" in optimised

    def test_instcombine_not_in(self):
        """!($a in $b) → $a ni $b."""
        source = "set v [expr {!($a in $b)}]"
        optimised, _ = optimise_source(source)
        assert "set v [expr {$a ni $b}]" in optimised

    # De Morgan's law

    def test_instcombine_demorgan_not_and(self):
        """!($a && $b) → !$a || !$b."""
        source = "set v [expr {!($a && $b)}]"
        optimised, rewrites = optimise_source(source)
        assert "set v [expr {!$a || !$b}]" in optimised
        assert any(r.code == "O110" for r in rewrites)

    def test_instcombine_demorgan_not_or(self):
        """!($a || $b) → !$a && !$b."""
        source = "set v [expr {!($a || $b)}]"
        optimised, rewrites = optimise_source(source)
        assert "set v [expr {!$a && !$b}]" in optimised
        assert any(r.code == "O110" for r in rewrites)

    def test_instcombine_demorgan_chains_with_comparison_inversion(self):
        """!($a == $b && $c < $d) → $a != $b || $c >= $d via fixpoint."""
        source = "set v [expr {!($a == $b && $c < $d)}]"
        optimised, rewrites = optimise_source(source)
        assert "set v [expr {$a != $b || $c >= $d}]" in optimised
        assert any(r.code == "O110" for r in rewrites)

    def test_instcombine_demorgan_chains_with_or(self):
        """!($a == $b || $c < $d) → $a != $b && $c >= $d via fixpoint."""
        source = "set v [expr {!($a == $b || $c < $d)}]"
        optimised, rewrites = optimise_source(source)
        assert "set v [expr {$a != $b && $c >= $d}]" in optimised
        assert any(r.code == "O110" for r in rewrites)

    def test_instcombine_demorgan_in_if(self):
        """De Morgan's in if condition: !($x && $y) → !$x || !$y."""
        optimised, rewrites = optimise_source("if {!($x && $y)} { puts yes }")
        assert any(r.code == "O110" for r in rewrites)
        o110 = [r for r in rewrites if r.code == "O110"][0]
        assert "!$x || !$y" in o110.replacement

    def test_instcombine_double_bitnot(self):
        """~~$x → $x."""
        source = "set v [expr {~~$x}]"
        optimised, _ = optimise_source(source)
        assert "set v [expr {$x}]" in optimised

    # Self-comparison tautologies

    def test_instcombine_self_eq(self):
        source = "set v [expr {$x == $x}]"
        optimised, _ = optimise_source(source)
        assert "set v 1" in optimised

    def test_instcombine_self_ne(self):
        source = "set v [expr {$x != $x}]"
        optimised, _ = optimise_source(source)
        assert "set v 0" in optimised

    def test_instcombine_self_xor(self):
        source = "set v [expr {$x ^ $x}]"
        optimised, _ = optimise_source(source)
        assert "set v 0" in optimised

    # Ternary simplifications

    def test_instcombine_ternary_identical_branches(self):
        source = "set v [expr {$c ? $a : $a}]"
        _, rewrites = optimise_source(source)
        assert any(r.code == "O110" for r in rewrites)

    def test_instcombine_ternary_not_cond_flip(self):
        """!$c ? $a : $b → $c ? $b : $a."""
        source = "set v [expr {!$c ? $a : $b}]"
        optimised, _ = optimise_source(source)
        assert "set v [expr {$c ? $b : $a}]" in optimised

    def test_instcombine_ternary_zero_one_negation(self):
        """$x ? 0 : 1 → !$x."""
        _, rewrites = optimise_source("set v [expr {$x ? 0 : 1}]")
        assert any(r.code == "O110" for r in rewrites)
        o110 = [r for r in rewrites if r.code == "O110"][0]
        assert "!$x" in o110.replacement

    def test_instcombine_ternary_one_zero_boolean_context(self):
        """In if condition: ($a > $b) ? 1 : 0 → $a > $b."""
        optimised, rewrites = optimise_source("if {($a > $b) ? 1 : 0} { puts yes }")
        assert any(r.code == "O110" for r in rewrites)
        o110 = [r for r in rewrites if r.code == "O110"][0]
        assert "$a > $b" in o110.replacement

    def test_instcombine_ternary_constant_cond_true(self):
        _, rewrites = optimise_source("set v [expr {1 ? $a : $b}]")
        assert any(r.code == "O110" for r in rewrites)

    def test_instcombine_ternary_constant_cond_false(self):
        _, rewrites = optimise_source("set v [expr {0 ? $a : $b}]")
        assert any(r.code == "O110" for r in rewrites)

    # Boolean-context double-not

    def test_instcombine_double_not_in_if(self):
        """In if: !!$x → $x."""
        optimised, rewrites = optimise_source("if {!!$x} { puts yes }")
        assert any(r.code == "O110" for r in rewrites)
        o110 = [r for r in rewrites if r.code == "O110"][0]
        assert o110.replacement == "{$x}"


class TestStructureElimination:
    """O112: structural elimination of constant-condition compound statements."""

    # if

    def test_if_always_true_unwraps_body(self):
        source = "if {1} {\n    set x 1\n}"
        optimised, rewrites = optimise_source(source)
        assert "if" not in optimised
        assert "set x 1" in optimised
        assert any(r.code == "O112" for r in rewrites)

    def test_if_always_false_deletes(self):
        source = "if {0} {\n    set x 1\n}\nputs always"
        optimised, rewrites = optimise_source(source)
        assert "set x 1" not in optimised
        assert "puts always" in optimised
        assert any(r.code == "O112" for r in rewrites)

    def test_if_false_with_else_unwraps_else(self):
        source = "if {0} {\n    set x 1\n} else {\n    set y 2\n}"
        optimised, rewrites = optimise_source(source)
        assert "set x 1" not in optimised
        assert "set y 2" in optimised
        assert "if" not in optimised
        assert any(r.code == "O112" for r in rewrites)

    def test_if_elseif_chain_finds_true_clause(self):
        source = "if {0} {\n    set a 1\n} elseif {1} {\n    set b 2\n} else {\n    set c 3\n}"
        optimised, rewrites = optimise_source(source)
        assert "set a 1" not in optimised
        assert "set b 2" in optimised
        assert "set c 3" not in optimised
        assert any(r.code == "O112" for r in rewrites)

    # while

    def test_while_false_deletes(self):
        source = "while {0} {\n    puts looping\n}\nputs done"
        optimised, rewrites = optimise_source(source)
        assert "puts looping" not in optimised
        assert "puts done" in optimised
        assert any(r.code == "O112" for r in rewrites)

    # for

    def test_for_false_keeps_init(self):
        source = "for {set i 0} {0} {incr i} {\n    puts looping\n}"
        optimised, rewrites = optimise_source(source)
        assert "puts looping" not in optimised
        assert "set i 0" in optimised
        assert any(r.code == "O112" for r in rewrites)

    def test_for_false_empty_init_deletes(self):
        source = "for {} {0} {} {\n    puts looping\n}\nputs done"
        optimised, rewrites = optimise_source(source)
        assert "puts looping" not in optimised
        assert "puts done" in optimised
        assert any(r.code == "O112" for r in rewrites)

    # switch

    def test_switch_literal_match(self):
        source = "switch abc {\n    abc { set x 1 }\n    def { set y 2 }\n}"
        optimised, rewrites = optimise_source(source)
        assert "set x 1" in optimised
        assert "set y 2" not in optimised
        assert "switch" not in optimised
        assert any(r.code == "O112" for r in rewrites)

    def test_switch_no_match_with_default(self):
        source = "switch xyz {\n    abc { set a 1 }\n    default { set b 2 }\n}"
        optimised, rewrites = optimise_source(source)
        assert "set a 1" not in optimised
        assert "set b 2" in optimised
        assert any(r.code == "O112" for r in rewrites)

    def test_switch_no_match_no_default_deletes(self):
        source = "switch xyz {\n    abc { set a 1 }\n}\nputs done"
        optimised, rewrites = optimise_source(source)
        assert "set a 1" not in optimised
        assert "puts done" in optimised
        assert any(r.code == "O112" for r in rewrites)

    # non-constant conditions untouched

    def test_runtime_condition_untouched(self):
        source = "if {$x} {\n    set y 1\n}"
        _, rewrites = optimise_source(source)
        assert not any(r.code == "O112" for r in rewrites)

    # nesting

    def test_nested_constant_structures(self):
        """Nested constant if — outer unwrap in first pass, inner in second."""
        source = "if {1} {\n    if {0} {\n        set dead 1\n    }\n    set alive 2\n}"
        optimised, rewrites = optimise_source(source)
        # First pass unwraps the outer if {1} body.
        assert "set alive 2" in optimised
        o112s = [r for r in rewrites if r.code == "O112"]
        assert len(o112s) >= 1
        # A second pass should eliminate the inner if {0}.
        optimised2, rewrites2 = optimise_source(optimised)
        assert "set dead 1" not in optimised2
        assert "set alive 2" in optimised2


class TestDemorganTransform:
    """Unit tests for the public demorgan_transform() function."""

    def test_forward_not_and(self):
        assert demorgan_transform("!($a && $b)") == "!$a || !$b"

    def test_forward_not_or(self):
        assert demorgan_transform("!($a || $b)") == "!$a && !$b"

    def test_reverse_not_or(self):
        assert demorgan_transform("!$a || !$b") == "!($a && $b)"

    def test_reverse_not_and(self):
        assert demorgan_transform("!$a && !$b") == "!($a || $b)"

    def test_no_match_plain_and(self):
        assert demorgan_transform("$a && $b") is None

    def test_no_match_comparison(self):
        assert demorgan_transform("$a == $b") is None

    def test_no_match_single_not(self):
        assert demorgan_transform("!$a") is None

    def test_no_match_empty(self):
        assert demorgan_transform("") is None

    def test_no_match_unparseable(self):
        assert demorgan_transform("&&& bad") is None


class TestInvertExpression:
    """Unit tests for the public invert_expression() function."""

    def test_invert_eq(self):
        assert invert_expression("$a == $b") == "$a != $b"

    def test_invert_lt(self):
        assert invert_expression("$a < $b") == "$a >= $b"

    def test_invert_ge(self):
        assert invert_expression("$a >= $b") == "$a < $b"

    def test_invert_and(self):
        assert invert_expression("$a && $b") == "!$a || !$b"

    def test_invert_or(self):
        assert invert_expression("$a || $b") == "!$a && !$b"

    def test_invert_not(self):
        """!$x → $x (double negation removed in bool context)."""
        assert invert_expression("!$x") == "$x"

    def test_invert_complex(self):
        """$a == $b && $c < $d → $a != $b || $c >= $d."""
        assert invert_expression("$a == $b && $c < $d") == "$a != $b || $c >= $d"

    def test_invert_variable(self):
        assert invert_expression("$x") == "!$x"

    def test_invert_empty(self):
        assert invert_expression("") is None

    def test_invert_unparseable(self):
        assert invert_expression("&&& bad") is None


class TestCrossEventDSE:
    """Optimiser must not eliminate stores consumed by a later event."""

    def test_cross_event_store_not_eliminated(self):
        """set in HTTP_REQUEST used in HTTP_RESPONSE → NOT eliminated."""
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                set uri [HTTP::uri]
            }
            when HTTP_RESPONSE {
                log local0. "uri=$uri"
            }
        """)
        optimised, rewrites = optimise_source(source)
        assert "set uri" in optimised
        # Should not have an O109 for the 'set uri' — it's used cross-event
        assert not any(r.code == "O109" for r in rewrites if "uri" in getattr(r, "message", ""))

    def test_cross_event_info_exists_not_eliminated(self):
        """set in DNS_REQUEST checked via [info exists] in DNS_RESPONSE → NOT eliminated."""
        source = textwrap.dedent("""\
            when DNS_REQUEST {
                if { [DNS::header opcode] ne "QUERY" } {
                    set ans_cleared 1
                    DNS::return
                    return
                }
            }
            when DNS_RESPONSE {
                if { [info exists ans_cleared] } {
                    unset -nocomplain -- ans_cleared
                    return
                }
            }
        """)
        optimised, rewrites = optimise_source(source)
        assert "set ans_cleared" in optimised
        assert not any(r.code == "O109" for r in rewrites)

    def test_cross_event_info_exists_allowlist_not_eliminated(self):
        """set in DNS_REQUEST checked via [info exists] in DNS_RESPONSE — allowlist pattern."""
        source = textwrap.dedent("""\
            when DNS_REQUEST {
                set allowlist 1
                return
            }
            when DNS_RESPONSE {
                if { [info exists allowlist] } {
                    unset -nocomplain -- allowlist
                    return
                }
            }
        """)
        optimised, rewrites = optimise_source(source)
        assert "set allowlist" in optimised
        assert not any(r.code == "O109" for r in rewrites)


class TestConstantVarRefPropagation:
    """O100: propagate scalar integer constants into command variable references."""

    def test_propagates_constant_into_command_arg(self):
        source = "set x 42\nputs $x"
        optimised, rewrites = optimise_source(source)
        assert optimised == "puts 42"
        assert any(r.code == "O100" for r in rewrites)
        assert any(r.code == "O109" for r in rewrites)

    def test_propagates_through_expr_and_command(self):
        """set a 1; set b [expr {$a + 1}]; puts $b → puts 2."""
        source = "set a 1\nset b [expr {$a + 1}]\nputs $b"
        optimised, rewrites = optimise_source(source)
        assert optimised == "puts 2"
        assert any(r.code == "O100" for r in rewrites)

    def test_chained_propagation_through_args(self):
        source = "set a 1\nset b [expr {$a + 2}]\nset c [expr {$b + 5}]\nputs $c"
        optimised, rewrites = optimise_source(source)
        assert optimised == "puts 8"

    def test_all_uses_propagated_allows_full_dse(self):
        source = "set x 5\nputs $x\nset y [expr {$x + 1}]"
        optimised, rewrites = optimise_source(source)
        # Both uses of x are propagated (O100 into puts, O100+O102 into expr)
        assert optimised == "puts 5\nset y 6"

    def test_var_in_string_replaced(self):
        """$x inside double-quoted string is now propagated via string interpolation."""
        source = 'set x 5\nputs "val=$x"'
        optimised, rewrites = optimise_source(source)
        assert any(r.code == "O105" for r in rewrites)
        assert 'puts "val=5"' in optimised

    def test_braced_var_replaced_without_trailing_brace(self):
        source = "set x 7\nputs ${x}"
        optimised, rewrites = optimise_source(source)
        # ${x} as a standalone word is handled by O100, not O105
        assert any(r.code in ("O100", "O105") for r in rewrites)
        assert "puts 7}" not in optimised
        assert "puts 7" in optimised

    def test_braced_var_in_string_replaced_without_trailing_brace(self):
        source = 'set x 7\nputs "${x}"'
        optimised, rewrites = optimise_source(source)
        assert any(r.code == "O105" for r in rewrites)
        assert 'puts "7}"' not in optimised
        assert 'puts "7"' in optimised

    def test_var_in_string_not_replaced_across_call_barrier(self):
        source = 'set x 5\nstring length abc\nputs "val=$x"'
        optimised, rewrites = optimise_source(source)
        assert optimised == source
        assert not any(r.code == "O105" for r in rewrites)

    def test_var_in_string_combines_with_expr_fold_and_dse(self):
        source = 'set x 5\nset y [expr {$x + 1}]\nputs "x=$x"\nputs $y'
        optimised, rewrites = optimise_source(source)
        assert optimised == 'puts "x=5"\nputs 6'
        assert any(r.code == "O105" for r in rewrites)
        assert any(r.code == "O109" for r in rewrites)


class TestPatternMatchSimplification:
    """O110: simplify matches_regex / matches_glob to simpler string ops."""

    def _setup_irules(self):
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")

    # -- matches_regex --

    def test_regex_anchored_both_to_equals(self):
        self._setup_irules()
        source = "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {^/api$} } { pool p1 } }"
        optimised, rewrites = optimise_source(source)
        assert "equals" in optimised
        assert "matches_regex" not in optimised
        assert any(r.code == "O110" for r in rewrites)

    def test_regex_anchored_start_to_starts_with(self):
        self._setup_irules()
        source = "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {^/api/} } { pool p1 } }"
        optimised, rewrites = optimise_source(source)
        assert "starts_with" in optimised
        assert "matches_regex" not in optimised
        assert any(r.code == "O110" for r in rewrites)

    def test_regex_anchored_end_to_ends_with(self):
        self._setup_irules()
        source = "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {/health$} } { pool p1 } }"
        optimised, rewrites = optimise_source(source)
        assert "ends_with" in optimised
        assert "matches_regex" not in optimised
        assert any(r.code == "O110" for r in rewrites)

    def test_regex_no_anchors_to_contains(self):
        self._setup_irules()
        source = "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {api} } { pool p1 } }"
        optimised, rewrites = optimise_source(source)
        assert "contains" in optimised
        assert "matches_regex" not in optimised
        assert any(r.code == "O110" for r in rewrites)

    def test_regex_dot_metachar_not_simplified(self):
        self._setup_irules()
        source = "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {.html$} } { pool p1 } }"
        optimised, _ = optimise_source(source)
        assert "matches_regex" in optimised

    def test_regex_dot_star_not_simplified(self):
        self._setup_irules()
        source = "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {^/api/.*} } { pool p1 } }"
        optimised, _ = optimise_source(source)
        assert "matches_regex" in optimised

    def test_regex_char_class_not_simplified(self):
        self._setup_irules()
        source = "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {^/[a-z]+$} } { pool p1 } }"
        optimised, _ = optimise_source(source)
        assert "matches_regex" in optimised

    def test_regex_backslash_not_simplified(self):
        self._setup_irules()
        source = (
            r"when HTTP_REQUEST { if { [HTTP::uri] matches_regex {^/api\.html$} } { pool p1 } }"
        )
        optimised, _ = optimise_source(source)
        assert "matches_regex" in optimised

    # -- matches_glob --

    def test_glob_leading_star_to_ends_with(self):
        self._setup_irules()
        source = "when HTTP_REQUEST { if { $host matches_glob {*.example.com} } { pool p1 } }"
        optimised, rewrites = optimise_source(source)
        assert "ends_with" in optimised
        assert "matches_glob" not in optimised
        assert any(r.code == "O110" for r in rewrites)

    def test_glob_trailing_star_to_starts_with(self):
        self._setup_irules()
        source = "when HTTP_REQUEST { if { $host matches_glob {api.example.*} } { pool p1 } }"
        optimised, rewrites = optimise_source(source)
        assert "starts_with" in optimised
        assert "matches_glob" not in optimised
        assert any(r.code == "O110" for r in rewrites)

    def test_glob_both_stars_to_contains(self):
        self._setup_irules()
        source = "when HTTP_REQUEST { if { $host matches_glob {*example*} } { pool p1 } }"
        optimised, rewrites = optimise_source(source)
        assert "contains" in optimised
        assert "matches_glob" not in optimised
        assert any(r.code == "O110" for r in rewrites)

    def test_glob_no_wildcards_to_equals(self):
        self._setup_irules()
        source = "when HTTP_REQUEST { if { $host matches_glob {www.example.com} } { pool p1 } }"
        optimised, rewrites = optimise_source(source)
        assert "equals" in optimised
        assert "matches_glob" not in optimised
        assert any(r.code == "O110" for r in rewrites)

    def test_glob_question_mark_not_simplified(self):
        self._setup_irules()
        source = "when HTTP_REQUEST { if { $host matches_glob {*.example.co?} } { pool p1 } }"
        optimised, _ = optimise_source(source)
        assert "matches_glob" in optimised

    def test_glob_middle_star_not_simplified(self):
        self._setup_irules()
        source = "when HTTP_REQUEST { if { $host matches_glob {api.*.com} } { pool p1 } }"
        optimised, _ = optimise_source(source)
        assert "matches_glob" in optimised

    # -- Edge cases --

    def test_empty_regex_pattern_not_simplified(self):
        self._setup_irules()
        source = "when HTTP_REQUEST { if { [HTTP::uri] matches_regex {} } { pool p1 } }"
        optimised, _ = optimise_source(source)
        assert "matches_regex" in optimised

    def test_variable_rhs_not_simplified(self):
        self._setup_irules()
        source = "when HTTP_REQUEST { if { [HTTP::uri] matches_regex $pattern } { pool p1 } }"
        optimised, _ = optimise_source(source)
        assert "matches_regex" in optimised

    def test_glob_single_star_not_simplified(self):
        self._setup_irules()
        source = "when HTTP_REQUEST { if { $host matches_glob {*} } { pool p1 } }"
        optimised, _ = optimise_source(source)
        assert "matches_glob" in optimised


class TestStrengthReduction:
    """O113 — strength reduction."""

    def test_pow_two(self):
        source = "if {$x ** 2} {}"
        optimised, rewrites = optimise_source(source)
        assert "**" not in optimised
        assert "*" in optimised
        assert any(r.code == "O113" for r in rewrites)

    def test_mod_power_of_two(self):
        source = "if {$x % 8} {}"
        optimised, rewrites = optimise_source(source)
        assert "&" in optimised
        assert "%" not in optimised
        assert any(r.code == "O113" for r in rewrites)


class TestIncrIdiom:
    """O114 — incr idiom recognition."""

    def test_incr_add_one(self):
        source = textwrap.dedent("""\
            proc foo {x} {
                set x [expr {$x + 1}]
            }
        """).rstrip()
        optimised, rewrites = optimise_source(source)
        assert "incr x" in optimised
        assert any(r.code == "O114" for r in rewrites)

    def test_incr_add_n(self):
        source = textwrap.dedent("""\
            proc foo {x} {
                set x [expr {$x + 5}]
            }
        """).rstrip()
        optimised, rewrites = optimise_source(source)
        assert "incr x 5" in optimised
        assert any(r.code == "O114" for r in rewrites)

    def test_incr_sub_n(self):
        source = textwrap.dedent("""\
            proc foo {x} {
                set x [expr {$x - 3}]
            }
        """).rstrip()
        optimised, rewrites = optimise_source(source)
        assert "incr x -3" in optimised
        assert any(r.code == "O114" for r in rewrites)


class TestNestedExprUnwrap:
    """O115 — redundant nested expr elimination."""

    def test_unwrap_nested_expr_in_if(self):
        source = "if {[expr {$x + 1}]} {}"
        optimised, rewrites = optimise_source(source)
        assert "[expr" not in optimised
        assert any(r.code == "O115" for r in rewrites)


class TestListFolding:
    """O116 — constant list command folding."""

    def test_fold_list_literals(self):
        source = "set x [list a b c]\nputs $x"
        optimised, rewrites = optimise_source(source)
        assert "[list" not in optimised
        assert any(r.code == "O116" for r in rewrites)


class TestStrlenZeroCheck:
    """O117 — string length zero-check simplification."""

    def test_strlen_eq_zero(self):
        source = "if {[string length $s] == 0} {}"
        optimised, rewrites = optimise_source(source)
        assert 'eq ""' in optimised or "string length" not in optimised

    def test_strlen_ne_zero(self):
        source = "if {[string length $s] != 0} {}"
        optimised, rewrites = optimise_source(source)
        assert 'ne ""' in optimised or "string length" not in optimised

    def test_strlen_gt_zero(self):
        source = "if {[string length $s] > 0} {}"
        optimised, rewrites = optimise_source(source)
        assert 'ne ""' in optimised or "string length" not in optimised


class TestStringCompareEqNe:
    """O120 — prefer eq/ne for string comparisons."""

    def test_rewrites_eq_with_string_literal(self):
        source = 'if {$a == "hello"} {}'
        optimised, rewrites = optimise_source(source)
        assert '$a eq "hello"' in optimised
        assert any(r.code == "O120" for r in rewrites)

    def test_rewrites_ne_in_expr_command_substitution(self):
        source = 'set ok [expr {$a != "hello"}]'
        optimised, rewrites = optimise_source(source)
        assert '$a ne "hello"' in optimised
        assert any(r.code == "O120" for r in rewrites)

    def test_numeric_like_literal_not_rewritten_for_known_non_string_var(self):
        source = 'set a [clock seconds]\nif {$a == "1"} {}'
        optimised, rewrites = optimise_source(source)
        assert '$a == "1"' in optimised
        assert not any(r.code == "O120" for r in rewrites)

    def test_numeric_like_literal_rewritten_for_known_string_var(self):
        source = 'set a [string trim $raw]\nif {$a == "1"} {}'
        optimised, rewrites = optimise_source(source)
        assert '$a eq "1"' in optimised
        assert any(r.code == "O120" for r in rewrites)

    def test_var_vs_var_both_string_typed(self):
        source = "set a foo\nset b bar\nif {$a == $b} {}"
        optimised, rewrites = optimise_source(source)
        # O105 may propagate constants, giving "foo" eq "bar"
        assert "eq" in optimised
        assert "==" not in optimised
        assert any(r.code == "O120" for r in rewrites)

    def test_var_vs_var_one_string_typed_not_rewritten(self):
        """Only one side is known-string; the other is unknown — don't rewrite."""
        source = "set a foo\nif {$a == $b} {}"
        _, rewrites = optimise_source(source)
        assert not any(r.code == "O120" for r in rewrites)

    def test_var_vs_var_int_typed_not_rewritten(self):
        source = "set a [expr {1 + 2}]\nset b [expr {3 + 4}]\nif {$a == $b} {}"
        _, rewrites = optimise_source(source)
        assert not any(r.code == "O120" for r in rewrites)

    def test_var_vs_var_unknown_not_rewritten(self):
        source = "if {$a == $b} {}"
        _, rewrites = optimise_source(source)
        assert not any(r.code == "O120" for r in rewrites)

    def test_boolean_like_literal_not_rewritten_for_unknown_var(self):
        source = 'if {$a == "true"} {}'
        _, rewrites = optimise_source(source)
        assert not any(r.code == "O120" for r in rewrites)

    def test_float_like_literal_not_rewritten_for_unknown_var(self):
        source = 'if {$a == "1.25"} {}'
        _, rewrites = optimise_source(source)
        assert not any(r.code == "O120" for r in rewrites)

    def test_boolean_like_literal_rewritten_for_known_string_var(self):
        source = 'set a [string trim $raw]\nif {$a == "true"} {}'
        optimised, rewrites = optimise_source(source)
        assert '$a eq "true"' in optimised
        assert any(r.code == "O120" for r in rewrites)

    def test_var_vs_var_from_string_producers_rewritten(self):
        source = "set a [string trim $x]\nset b [string trim $y]\nif {$a == $b} {}"
        optimised, rewrites = optimise_source(source)
        assert "$a eq $b" in optimised
        assert any(r.code == "O120" for r in rewrites)

    def test_mixed_expr_only_rewrites_string_compare(self):
        source = 'set a [string trim $raw]\nif {$a == "x" && $n == 1} {}'
        optimised, rewrites = optimise_source(source)
        assert '$a eq "x"' in optimised
        assert "$n == 1" in optimised
        assert any(r.code == "O120" for r in rewrites)


class TestLindexFolding:
    """O118 — constant lindex command folding."""

    def test_fold_lindex_literal(self):
        source = "set x [lindex {a b c} 1]\nputs $x"
        optimised, rewrites = optimise_source(source)
        assert "[lindex" not in optimised
        assert any(r.code == "O118" for r in rewrites)

    def test_fold_lindex_end(self):
        source = "set x [lindex {x y z} end]\nputs $x"
        optimised, rewrites = optimise_source(source)
        assert "[lindex" not in optimised
        assert any(r.code == "O118" for r in rewrites)


class TestMultiSetPacking:
    """O119 — multi-set packing."""

    def test_consecutive_sets_packed(self):
        # Use eval barrier so O105 can't propagate constants and eliminate stores.
        source = textwrap.dedent("""\
            set a 1
            set b 2
            set c 3
            eval {$a $b $c}
        """).rstrip()
        optimised, rewrites = optimise_source(source)
        assert any(r.code == "O119" for r in rewrites)
        assert "lassign" in optimised or "foreach" in optimised

    def test_interspersed_sets_packed(self):
        source = textwrap.dedent("""\
            set a 1
            puts hello
            set b 2
            puts world
            set c 3
            eval {$a $b $c}
        """).rstrip()
        optimised, rewrites = optimise_source(source)
        # The sets should be packed since puts doesn't read a/b/c before the last set.
        assert any(r.code == "O119" for r in rewrites)

    def test_read_breaks_group(self):
        source = textwrap.dedent("""\
            set a 1
            puts $a
            set b 2
            set c 3
            puts "$b $c"
        """).rstrip()
        _optimised, rewrites = optimise_source(source)
        # $a is read between set a and set b, so set a cannot be grouped with b,c.
        o119_rewrites = [r for r in rewrites if r.code == "O119"]
        # Either no O119 (group of b,c is only 2) or a is not in any lassign.
        if o119_rewrites:
            for r in o119_rewrites:
                if r.replacement and "lassign" in r.replacement:
                    assert "a" not in r.replacement.split()

    def test_too_few_not_packed(self):
        source = textwrap.dedent("""\
            set a 1
            set b 2
            puts "$a $b"
        """).rstrip()
        _optimised, rewrites = optimise_source(source)
        assert not any(r.code == "O119" for r in rewrites)

    def test_tcl9_skips_packing(self):
        """In Tcl 9.0 individual set is faster — O119 must not fire."""
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="tcl9.0")
        try:
            source = textwrap.dedent("""\
                set a 1
                set b 2
                set c 3
                puts "$a $b $c"
            """).rstrip()
            _optimised, rewrites = optimise_source(source)
            assert not any(r.code == "O119" for r in rewrites)
        finally:
            configure_signatures(dialect="tcl8.6")


class TestVariableShapeOptimisationGuardrails:
    """Variable-shape forms should not be conflated by optimiser rewrites."""

    def test_braced_scalar_like_array_name_not_rewritten_as_array_ref(self):
        source = "set x ${a(1)}\nputs $x"
        optimised, rewrites = optimise_source(source)
        assert optimised == source
        assert rewrites == []

    def test_unbraced_array_ref_not_rewritten_as_braced_scalar_name(self):
        source = "set x $a(1)\nputs $x"
        optimised, rewrites = optimise_source(source)
        assert optimised == source
        assert rewrites == []

    def test_namespaced_array_ref_shape_preserved(self):
        source = "set x $::ns::arr(k)\nputs $x"
        optimised, rewrites = optimise_source(source)
        assert optimised == source
        assert rewrites == []


class TestTailCallOptimisation:
    """O121 — tail-call detection, O122 — loop conversion, O123 — accumulator hint."""

    # -- O121: tailcall detection (pre-overlap, via find_optimisations) --

    def test_tail_call_detected_for_return_self_call(self):
        """Tail-position self-call (return [self ...]) triggers O121 or O122."""
        source = textwrap.dedent("""\
            proc factorial {n acc} {
                if {$n <= 1} {
                    return $acc
                }
                return [factorial [expr {$n - 1}] [expr {$n * $acc}]]
            }
        """)
        rewrites = find_optimisations(source)
        assert any(r.code in ("O121", "O122") for r in rewrites)

    def test_tail_call_detected_for_bare_self_call(self):
        """Tail-position bare self-call triggers O121 or O122."""
        source = textwrap.dedent("""\
            proc loop {items} {
                if {[llength $items] == 0} {
                    return
                }
                puts [lindex $items 0]
                loop [lrange $items 1 end]
            }
        """)
        rewrites = find_optimisations(source)
        assert any(r.code in ("O121", "O122") for r in rewrites)

    def test_non_tail_self_call_not_detected(self):
        """Self-call NOT in tail position produces no O121/O122."""
        source = textwrap.dedent("""\
            proc fib {n} {
                if {$n <= 1} {
                    return $n
                }
                expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}
            }
        """)
        rewrites = find_optimisations(source)
        assert not any(r.code in ("O121", "O122") for r in rewrites)

    def test_mutual_recursion_not_detected(self):
        """Calls to a different proc produce no O121/O122."""
        source = textwrap.dedent("""\
            proc even {n} {
                if {$n == 0} { return 1 }
                return [odd [expr {$n - 1}]]
            }
            proc odd {n} {
                if {$n == 0} { return 0 }
                return [even [expr {$n - 1}]]
            }
        """)
        rewrites = find_optimisations(source)
        assert not any(r.code in ("O121", "O122") for r in rewrites)

    def test_tail_call_in_if_else_branch(self):
        """Self-call in if/else branch triggers O121 or O122."""
        source = textwrap.dedent("""\
            proc gcd {a b} {
                if {$b == 0} {
                    return $a
                } else {
                    return [gcd $b [expr {$a % $b}]]
                }
            }
        """)
        rewrites = find_optimisations(source)
        assert any(r.code in ("O121", "O122") for r in rewrites)

    def test_tail_call_in_switch_arm(self):
        """Self-call in switch arm triggers O121 or O122."""
        source = textwrap.dedent("""\
            proc walk {tree} {
                switch [lindex $tree 0] {
                    leaf {
                        return [lindex $tree 1]
                    }
                    node {
                        walk [lindex $tree 2]
                    }
                }
            }
        """)
        rewrites = find_optimisations(source)
        assert any(r.code in ("O121", "O122") for r in rewrites)

    # -- O122: loop conversion (applied via optimise_source) --

    def test_o122_converts_return_self_call_to_loop(self):
        """O122 rewrites return [self ...] proc as while loop."""
        source = textwrap.dedent("""\
            proc factorial {n acc} {
                if {$n <= 1} {
                    return $acc
                }
                return [factorial [expr {$n - 1}] [expr {$n * $acc}]]
            }
        """)
        optimised, rewrites = optimise_source(source)
        assert "while {1}" in optimised
        assert "lassign" in optimised
        assert "factorial" not in optimised.split("proc ", 1)[-1].split("{", 2)[-1]
        assert any(r.code == "O122" for r in rewrites)

    def test_o122_converts_bare_self_call_to_loop(self):
        """O122 rewrites bare self-call proc as while loop with set."""
        source = textwrap.dedent("""\
            proc loop {items} {
                if {[llength $items] == 0} {
                    return
                }
                puts [lindex $items 0]
                loop [lrange $items 1 end]
            }
        """)
        optimised, rewrites = optimise_source(source)
        assert "while {1}" in optimised
        assert "set items" in optimised
        assert any(r.code == "O122" for r in rewrites)

    def test_o122_converts_if_else_branch(self):
        """O122 rewrites gcd-style if/else proc as while loop."""
        source = textwrap.dedent("""\
            proc gcd {a b} {
                if {$b == 0} {
                    return $a
                } else {
                    return [gcd $b [expr {$a % $b}]]
                }
            }
        """)
        optimised, rewrites = optimise_source(source)
        assert "while {1}" in optimised
        assert "lassign" in optimised
        assert any(r.code == "O122" for r in rewrites)

    def test_o122_skipped_for_non_tail_recursion(self):
        """O122 does not fire when self-calls are not all in tail position."""
        source = textwrap.dedent("""\
            proc fib {n} {
                if {$n <= 1} {
                    return $n
                }
                expr {[fib [expr {$n - 1}]] + [fib [expr {$n - 2}]]}
            }
        """)
        _, rewrites = optimise_source(source)
        assert not any(r.code == "O122" for r in rewrites)

    def test_o122_skipped_for_mixed_tail_and_non_tail(self):
        """O122 does not fire when proc has both tail and non-tail self-calls."""
        source = textwrap.dedent("""\
            proc bad {n acc} {
                if {$n <= 1} {
                    return $acc
                }
                set partial [bad [expr {$n - 2}] $acc]
                return [bad [expr {$n - 1}] $partial]
            }
        """)
        rewrites = find_optimisations(source)
        assert not any(r.code == "O122" for r in rewrites)

    # -- O123: accumulator-eligible non-tail recursion --

    def test_o123_detects_accumulator_pattern(self):
        """O123 fires for return [expr {$n * [self ...]}]."""
        source = textwrap.dedent("""\
            proc factorial {n} {
                if {$n <= 1} {
                    return 1
                }
                return [expr {$n * [factorial [expr {$n - 1}]]}]
            }
        """)
        rewrites = find_optimisations(source)
        assert any(r.code == "O123" for r in rewrites)

    def test_o123_not_for_tail_recursive_proc(self):
        """O123 does not fire when the proc is already tail-recursive."""
        source = textwrap.dedent("""\
            proc factorial {n acc} {
                if {$n <= 1} {
                    return $acc
                }
                return [factorial [expr {$n - 1}] [expr {$n * $acc}]]
            }
        """)
        rewrites = find_optimisations(source)
        assert not any(r.code == "O123" for r in rewrites)

    def test_o123_not_for_non_recursive_proc(self):
        """O123 does not fire when the proc has no self-calls."""
        source = textwrap.dedent("""\
            proc add {a b} {
                return [expr {$a + $b}]
            }
        """)
        rewrites = find_optimisations(source)
        assert not any(r.code == "O123" for r in rewrites)

    def test_o123_not_for_doubly_recursive_proc(self):
        """O123 does not fire for doubly-recursive patterns like Fibonacci."""
        source = textwrap.dedent("""\
            proc fib {n} {
                if {$n <= 1} { return $n }
                return [expr {[fib [expr {$n-1}]] + [fib [expr {$n-2}]]}]
            }
        """)
        rewrites = find_optimisations(source)
        assert not any(r.code == "O123" for r in rewrites)

    def test_o123_not_for_non_accumulator_pattern(self):
        """O123 does not fire for embedded self-call without associative op."""
        source = textwrap.dedent("""\
            proc transform {x} {
                if {$x eq ""} { return "" }
                return [format "%s" [transform [string range $x 1 end]]]
            }
        """)
        rewrites = find_optimisations(source)
        assert not any(r.code == "O123" for r in rewrites)

    def test_o123_hint_only_flag(self):
        """O123 findings have hint_only=True."""
        source = textwrap.dedent("""\
            proc factorial {n} {
                if {$n <= 1} { return 1 }
                return [expr {$n * [factorial [expr {$n - 1}]]}]
            }
        """)
        rewrites = find_optimisations(source)
        o123 = [r for r in rewrites if r.code == "O123"]
        assert len(o123) == 1
        assert o123[0].hint_only is True

    def test_o122_output_is_valid_tcl(self):
        """O122 rewrite should not produce trailing syntax errors."""
        from core.analysis.analyser import analyse

        source = textwrap.dedent("""\
            proc gcd {a b} {
                if {$b == 0} {
                    return $a
                } else {
                    return [gcd $b [expr {$a % $b}]]
                }
            }
        """)
        result, _opts = optimise_source(source)
        errors = [d for d in analyse(result).diagnostics if (d.code or "").startswith("E")]
        assert errors == []
        assert "while {1}" in result

    def test_o122_not_for_self_call_in_condition(self):
        """O122 should not fire when self-call appears in an if condition."""
        source = textwrap.dedent("""\
            proc f {n} {
                if {[f $n]} {
                    return [f [expr {$n - 1}]]
                } else {
                    return $n
                }
            }
        """)
        rewrites = find_optimisations(source)
        # Should not produce O122 (loop conversion) because the condition
        # contains a non-tail self-call.
        assert not any(r.code == "O122" for r in rewrites)

    def test_o123_fires_with_mixed_tail_and_non_tail(self):
        """O123 fires for non-tail self-call even when tail sites exist."""
        source = textwrap.dedent("""\
            proc calc {n} {
                if {$n <= 0} { return 0 }
                if {$n == 1} {
                    return [expr {$n * [calc [expr {$n - 1}]]}]
                }
                return [calc [expr {$n - 2}]]
            }
        """)
        rewrites = find_optimisations(source)
        # The non-tail branch should still get an O123 hint.
        assert any(r.code == "O123" for r in rewrites)

    def test_o121_o122_not_for_braced_return_literal(self):
        """Braced literal '[self ...]' is not an executable tail call."""
        source = textwrap.dedent("""\
            proc f {n} {
                return {[f $n]}
            }
        """)
        rewrites = find_optimisations(source)
        assert not any(r.code in ("O121", "O122") for r in rewrites)

    def test_o123_not_for_braced_expr_literal(self):
        """Braced literal expr text must not produce an O123 hint."""
        source = textwrap.dedent("""\
            proc f {n} {
                return {[expr {$n * [f [expr {$n - 1}]]}]}
            }
        """)
        rewrites = find_optimisations(source)
        assert not any(r.code == "O123" for r in rewrites)

    def test_o122_not_blocked_by_braced_literal_set(self):
        """Literal bracket text in a braced set value must not count as recursion."""
        source = textwrap.dedent("""\
            proc fact {n acc} {
                set marker {[fact $n]}
                if {$n <= 1} {
                    return $acc
                }
                return [fact [expr {$n - 1}] [expr {$n * $acc}]]
            }
        """)
        rewrites = find_optimisations(source)
        assert any(r.code == "O122" for r in rewrites)

    def test_o122_not_for_self_call_in_switch_subject(self):
        """O122 should not fire when recursion appears in switch subject."""
        source = textwrap.dedent("""\
            proc f {n} {
                switch [f [expr {$n - 1}]] {
                    0 { return 0 }
                    default { return [f [expr {$n - 2}]] }
                }
            }
        """)
        rewrites = find_optimisations(source)
        assert not any(r.code == "O122" for r in rewrites)

    def test_o122_not_for_self_call_in_while_condition(self):
        """O122 should not fire when recursion appears in while condition."""
        source = textwrap.dedent("""\
            proc f {n} {
                while {[f $n]} {
                    return [f [expr {$n - 1}]]
                }
                return $n
            }
        """)
        rewrites = find_optimisations(source)
        assert not any(r.code == "O122" for r in rewrites)

    def test_o122_not_for_self_call_in_for_condition(self):
        """O122 should not fire when recursion appears in for condition."""
        source = textwrap.dedent("""\
            proc f {n} {
                for {set i 0} {[f $n]} {incr i} {
                    return [f [expr {$n - 1}]]
                }
                return $n
            }
        """)
        rewrites = find_optimisations(source)
        assert not any(r.code == "O122" for r in rewrites)

    def test_o122_skips_when_tail_call_arity_mismatch(self):
        """O122 requires each tail-call site to pass all proc parameters."""
        source = textwrap.dedent("""\
            proc f {a b} {
                return [f $a]
            }
        """)
        rewrites = find_optimisations(source)
        assert any(r.code == "O121" for r in rewrites)
        assert not any(r.code == "O122" for r in rewrites)

    def test_o122_selected_rewrite_suppresses_o121_overlap(self):
        """In selected rewrites, O122 should subsume per-site O121 edits."""
        source = textwrap.dedent("""\
            proc f {n acc} {
                if {$n <= 1} { return $acc }
                return [f [expr {$n - 1}] [expr {$n * $acc}]]
            }
        """)
        rewrites = find_optimisations(source)
        assert any(r.code == "O122" for r in rewrites)
        assert not any(r.code == "O121" for r in rewrites)

    def test_o123_coexists_with_other_rewrite_passes(self):
        """Hint-only O123 should coexist with independent rewrite passes."""
        source = textwrap.dedent("""\
            proc f {n} {
                set a 1
                if {$n <= 1} { return [expr {$a + 0}] }
                return [expr {$n * [f [expr {$n - 1}]]}]
            }
        """)
        rewrites = find_optimisations(source)
        assert any(r.code == "O123" for r in rewrites)
        assert any(r.code == "O102" for r in rewrites)
        optimised, _ = optimise_source(source)
        assert "return 1" in optimised
        assert "return [expr {$n * [f [expr {$n - 1}]]}]" in optimised

    def test_o121_detects_qualified_self_call(self):
        """Tail recursion using fully-qualified self name should be detected."""
        source = textwrap.dedent("""\
            namespace eval ns {
                proc f {n} {
                    return [::ns::f [expr {$n - 1}]]
                }
            }
        """)
        rewrites = find_optimisations(source)
        assert any(r.code in ("O121", "O122") for r in rewrites)


class TestUnusedIruleProcs:
    """O124: comment out unused procs in iRules."""

    def _setup_irules(self):
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="f5-irules")

    def _teardown_irules(self):
        from core.commands.registry.runtime import configure_signatures

        configure_signatures(dialect="tcl8.6")

    def test_unused_proc_is_commented_out(self):
        self._setup_irules()
        try:
            source = textwrap.dedent("""\
                proc helper {x} {
                    return $x
                }

                when HTTP_REQUEST {
                    pool my_pool
                }""")
            optimised, rewrites = optimise_source(source)
            assert any(r.code == "O124" for r in rewrites)
            assert "# proc helper" in optimised
            assert "proc helper" not in optimised.replace("# proc helper", "")
        finally:
            self._teardown_irules()

    def test_used_proc_not_commented_out(self):
        self._setup_irules()
        try:
            source = textwrap.dedent("""\
                proc helper {} {
                    return 1
                }

                when HTTP_REQUEST {
                    set val [call helper]
                }""")
            _optimised, rewrites = optimise_source(source)
            assert not any(r.code == "O124" for r in rewrites)
        finally:
            self._teardown_irules()

    def test_transitively_used_proc_not_commented_out(self):
        self._setup_irules()
        try:
            source = textwrap.dedent("""\
                proc inner {} {
                    return 42
                }

                proc outer {} {
                    return [call inner]
                }

                when HTTP_REQUEST {
                    set val [call outer]
                }""")
            _optimised, rewrites = optimise_source(source)
            assert not any(r.code == "O124" for r in rewrites)
        finally:
            self._teardown_irules()

    def test_library_irule_not_affected(self):
        """iRules with only procs and RULE_INIT are libraries — skip."""
        self._setup_irules()
        try:
            source = textwrap.dedent("""\
                proc helper {} {
                    return 1
                }

                when RULE_INIT {
                    set ::debug 0
                }""")
            _optimised, rewrites = optimise_source(source)
            assert not any(r.code == "O124" for r in rewrites)
        finally:
            self._teardown_irules()

    def test_procs_only_no_events_not_affected(self):
        """iRules with only procs (no events) are libraries — skip."""
        self._setup_irules()
        try:
            source = textwrap.dedent("""\
                proc helper_a {} {
                    return 1
                }

                proc helper_b {} {
                    return [call helper_a]
                }""")
            _optimised, rewrites = optimise_source(source)
            assert not any(r.code == "O124" for r in rewrites)
        finally:
            self._teardown_irules()

    def test_plain_tcl_not_affected(self):
        """O124 only applies to f5-irules dialect."""
        source = textwrap.dedent("""\
            proc unused {} {
                return 1
            }
            puts hello""")
        _optimised, rewrites = optimise_source(source)
        assert not any(r.code == "O124" for r in rewrites)

    def test_multiple_unused_procs(self):
        self._setup_irules()
        try:
            source = textwrap.dedent("""\
                proc used {} {
                    return 1
                }

                proc unused_a {} {
                    return 2
                }

                proc unused_b {} {
                    return 3
                }

                when HTTP_REQUEST {
                    set val [call used]
                }""")
            _optimised, rewrites = optimise_source(source)
            o124s = [r for r in rewrites if r.code == "O124"]
            assert len(o124s) == 2
            names = {r.message for r in o124s}
            assert any("unused_a" in n for n in names)
            assert any("unused_b" in n for n in names)
        finally:
            self._teardown_irules()

    def test_unused_proc_called_from_rule_init_only(self):
        """Proc called only from RULE_INIT but not from other events is still used."""
        self._setup_irules()
        try:
            source = textwrap.dedent("""\
                proc init_helper {} {
                    return 1
                }

                when RULE_INIT {
                    set ::val [call init_helper]
                }

                when HTTP_REQUEST {
                    pool my_pool
                }""")
            _optimised, rewrites = optimise_source(source)
            assert not any(r.code == "O124" for r in rewrites)
        finally:
            self._teardown_irules()

    def test_direct_proc_invocation_counts_as_used(self):
        """Direct invocation `[helper]` should keep helper reachable."""
        self._setup_irules()
        try:
            source = textwrap.dedent("""\
                proc helper {} {
                    return 1
                }

                when HTTP_REQUEST {
                    set val [helper]
                }""")
            _optimised, rewrites = optimise_source(source)
            assert not any(r.code == "O124" for r in rewrites)
        finally:
            self._teardown_irules()

    def test_qualified_proc_call_counts_as_used(self):
        """Qualified call targets should be resolved in reachability graph."""
        self._setup_irules()
        try:
            source = textwrap.dedent("""\
                namespace eval ns {
                    proc helper {} {
                        return 1
                    }
                }

                when HTTP_REQUEST {
                    set val [call ::ns::helper]
                }""")
            _optimised, rewrites = optimise_source(source)
            assert not any(r.code == "O124" for r in rewrites)
        finally:
            self._teardown_irules()

    def test_mutually_recursive_but_unreachable_procs_are_flagged(self):
        """Mutually-recursive helpers unused by events should both get O124."""
        self._setup_irules()
        try:
            source = textwrap.dedent("""\
                proc ping {n} {
                    if {$n <= 0} {
                        return 0
                    }
                    return [call pong [expr {$n - 1}]]
                }

                proc pong {n} {
                    if {$n <= 0} {
                        return 0
                    }
                    return [call ping [expr {$n - 1}]]
                }

                when HTTP_REQUEST {
                    pool my_pool
                }""")
            _optimised, rewrites = optimise_source(source)
            o124 = [r for r in rewrites if r.code == "O124"]
            assert len(o124) == 2
            assert any("ping" in r.message for r in o124)
            assert any("pong" in r.message for r in o124)
            assert not any(r.code in ("O121", "O122") for r in rewrites)
        finally:
            self._teardown_irules()

    def test_o124_supersedes_tail_call_rewrites_for_unused_proc(self):
        """Unused tail-recursive proc should prefer whole-proc O124 rewrite."""
        self._setup_irules()
        try:
            source = textwrap.dedent("""\
                proc fact {n acc} {
                    if {$n <= 1} {
                        return $acc
                    }
                    return [fact [expr {$n - 1}] [expr {$n * $acc}]]
                }

                when HTTP_REQUEST {
                    pool my_pool
                }""")
            _optimised, rewrites = optimise_source(source)
            assert any(r.code == "O124" for r in rewrites)
            assert not any(r.code in ("O121", "O122") for r in rewrites)
        finally:
            self._teardown_irules()

    def test_o124_does_not_block_other_optimisations_for_used_proc(self):
        """Used procs should still participate in independent optimisation passes."""
        self._setup_irules()
        try:
            source = textwrap.dedent("""\
                proc helper {x} {
                    if {$x == "foo"} {
                        return 1
                    }
                    return 0
                }

                when HTTP_REQUEST {
                    set val [call helper bar]
                }""")
            _optimised, rewrites = optimise_source(source)
            assert any(r.code == "O120" for r in rewrites)
            assert not any(r.code == "O124" for r in rewrites)
        finally:
            self._teardown_irules()

    def test_o124_coexists_with_non_overlapping_event_rewrite(self):
        """O124 should coexist with event-level rewrites in disjoint ranges."""
        self._setup_irules()
        try:
            source = textwrap.dedent("""\
                proc helper {} {
                    return 1
                }

                when HTTP_REQUEST {
                    set x [expr {1 + 2}]
                    pool my_pool
                }""")
            _optimised, rewrites = optimise_source(source)
            assert any(r.code == "O124" for r in rewrites)
            # O126 may subsume the expr fold when x is unused, so
            # accept either the fold codes or O126.
            assert any(r.code in ("O101", "O102", "O126") for r in rewrites)
        finally:
            self._teardown_irules()

    def test_o124_suppressed_when_event_has_eval(self):
        """Dynamic dispatch via eval in event handler suppresses O124."""
        self._setup_irules()
        try:
            source = textwrap.dedent("""\
                proc helper {} {
                    return 1
                }

                when HTTP_REQUEST {
                    eval {set x 1}
                    pool my_pool
                }""")
            _optimised, rewrites = optimise_source(source)
            assert not any(r.code == "O124" for r in rewrites)
        finally:
            self._teardown_irules()

    def test_o124_suppressed_when_reachable_proc_has_eval(self):
        """Dynamic dispatch via eval in a transitively reachable proc suppresses O124."""
        self._setup_irules()
        try:
            source = textwrap.dedent("""\
                proc unused {} {
                    return 1
                }

                proc dispatcher {} {
                    eval {set x 1}
                }

                when HTTP_REQUEST {
                    call dispatcher
                }""")
            _optimised, rewrites = optimise_source(source)
            assert not any(r.code == "O124" for r in rewrites)
        finally:
            self._teardown_irules()

    def test_o124_not_suppressed_when_unreachable_proc_has_eval(self):
        """Dynamic dispatch in an unreachable proc does not suppress O124."""
        self._setup_irules()
        try:
            source = textwrap.dedent("""\
                proc dynamic_helper {} {
                    eval {set x 1}
                }

                when HTTP_REQUEST {
                    pool my_pool
                }""")
            _optimised, rewrites = optimise_source(source)
            assert any(r.code == "O124" for r in rewrites)
        finally:
            self._teardown_irules()


class TestCodeSinking:
    """O125: Sink definitions into decision blocks."""

    def test_basic_sink_into_if(self):
        source = textwrap.dedent("""\
            set b foo
            if {$a} {
                puts $b
            }""")
        optimised, rewrites = optimise_source(source)
        codes = {r.code for r in rewrites}
        # O100+O109 (propagation) may subsume O125 (sinking) when SCCP
        # propagates string constants.
        assert codes & {"O125", "O100"}, f"expected O125 or O100, got {codes}"

    def test_no_sink_when_var_in_condition(self):
        source = textwrap.dedent("""\
            set b $x
            if {$b} {
                puts hello
            }""")
        optimised, rewrites = optimise_source(source)
        assert not any(r.code == "O125" for r in rewrites)

    def test_no_sink_when_var_used_after(self):
        source = textwrap.dedent("""\
            set b foo
            if {$a} {
                puts $b
            }
            puts $b""")
        optimised, rewrites = optimise_source(source)
        assert not any(r.code == "O125" for r in rewrites)

    def test_sink_into_both_branches(self):
        source = textwrap.dedent("""\
            set b foo
            if {$a} {
                puts $b
            } else {
                puts $b
            }""")
        optimised, rewrites = optimise_source(source)
        codes = {r.code for r in rewrites}
        # O100+O109 may subsume O125 for simple string constants.
        assert codes & {"O125", "O100"}, f"expected O125 or O100, got {codes}"

    def test_deep_sink_into_nested_if(self):
        source = textwrap.dedent("""\
            set b foo
            if {$a} {
                if {$c} {
                    puts $b
                }
            }""")
        optimised, rewrites = optimise_source(source)
        codes = {r.code for r in rewrites}
        # O100+O109 may subsume O125 for simple string constants.
        assert codes & {"O125", "O100"}, f"expected O125 or O100, got {codes}"

    def test_no_sink_when_var_not_used_in_branches(self):
        source = textwrap.dedent("""\
            set b foo
            if {$a} {
                puts hello
            }""")
        optimised, rewrites = optimise_source(source)
        assert not any(r.code == "O125" for r in rewrites)

    def test_sink_only_into_using_branch(self):
        source = textwrap.dedent("""\
            set b foo
            if {$a} {
                puts $b
            } else {
                puts hello
            }""")
        optimised, rewrites = optimise_source(source)
        codes = {r.code for r in rewrites}
        # O100+O109 may subsume O125 for simple string constants.
        assert codes & {"O125", "O100"}, f"expected O125 or O100, got {codes}"

    def test_no_sink_for_command_substitution_value(self):
        source = textwrap.dedent("""\
            set b [clock seconds]
            if {$a} {
                puts $b
            }""")
        optimised, rewrites = optimise_source(source)
        assert not any(r.code == "O125" for r in rewrites)

    def test_numeric_constant_handled_by_propagation_not_sinking(self):
        """Numeric constants are better handled by O100/O109 than O125."""
        source = textwrap.dedent("""\
            set b 42
            if {$a} {
                puts $b
            }""")
        _optimised, rewrites = optimise_source(source)
        # O100 propagates, O109 eliminates — O125 should not fire.
        assert not any(r.code == "O125" for r in rewrites)

    def test_cross_event_var_not_sunk(self):
        """Variables shared across iRule events must not be sunk."""
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                set b foo
                if {[HTTP::uri] eq "/"} {
                    puts $b
                }
            }
            when HTTP_RESPONSE {
                puts $b
            }""")
        _optimised, rewrites = optimise_source(source)
        # Cross-event vars should be excluded from sinking.
        assert not any(r.code == "O125" for r in rewrites)

    def test_sink_preserves_indentation(self):
        source = textwrap.dedent("""\
            set b foo
            if {$a} {
                puts $b
            }""")
        optimised, _rewrites = optimise_source(source)
        lines = optimised.split("\n")
        # The sunk statement should match the body indentation.
        for line in lines:
            if "set b foo" in line and "# [O125]" not in line:
                # This is the sunk statement inside the body.
                leading = len(line) - len(line.lstrip())
                assert leading == 4  # matches puts indentation

    def test_no_sink_when_rhs_dep_can_change_in_condition(self):
        source = textwrap.dedent("""\
            set b $x
            if {[incr x] > 0} {
                puts $b
            }""")
        _optimised, rewrites = optimise_source(source)
        assert not any(r.code == "O125" for r in rewrites)

    def test_no_sink_when_var_used_after_via_bare_name(self):
        source = textwrap.dedent("""\
            set b foo
            if {$a} {
                puts $b
            }
            incr b""")
        _optimised, rewrites = optimise_source(source)
        assert not any(r.code == "O125" for r in rewrites)

    def test_no_sink_when_var_read_after_via_set_read_form(self):
        source = textwrap.dedent("""\
            set b foo
            if {$a} {
                puts $b
            }
            set b""")
        _optimised, rewrites = optimise_source(source)
        assert not any(r.code == "O125" for r in rewrites)

    def test_no_deep_sink_past_prior_nested_redefinition(self):
        source = textwrap.dedent("""\
            set b foo
            if {$a} {
                if {$c} {
                    set b bar
                }
                if {$d} {
                    puts $b
                }
            }""")
        optimised, rewrites = optimise_source(source)
        assert any(r.code == "O125" for r in rewrites)
        # Sink at the outer level; do not sink past a prior nested redefine.
        assert "if {$a} {\n    set b foo" in optimised
        assert "if {$d} {\n        set b foo" not in optimised

    def test_o125_can_coexist_with_o120(self):
        source = textwrap.dedent("""\
            set b foo
            if {$kind == "x"} {
                puts $b
            }""")
        optimised, rewrites = optimise_source(source)
        codes = {r.code for r in rewrites}
        # O100+O109 may subsume O125 for simple string constants.
        assert codes & {"O125", "O100"}, f"expected O125 or O100, got {codes}"
        assert any(r.code == "O120" for r in rewrites)
        assert '$kind eq "x"' in optimised

    def test_o125_o112_overlap_drops_all_o125(self):
        """When O112 eliminates the decision block, all O125 parts are dropped."""
        source = textwrap.dedent("""\
            set b foo
            if {0} {
                puts $b
            }""")
        _optimised, rewrites = optimise_source(source)
        assert any(r.code == "O112" for r in rewrites)
        # Both insertions and orphaned comments must be dropped.
        assert not any(r.code == "O125" for r in rewrites)
