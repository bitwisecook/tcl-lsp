"""VM equivalence tests: optimised source must produce identical output.

For every test case we:
1. Execute the *original* Tcl source in the VM and capture stdout + result.
2. Apply ``optimise_source()`` to produce optimised Tcl source.
3. Execute the *optimised* source in the VM and capture stdout + result.
4. Assert that both runs produce identical observable behaviour.

This catches semantic-preservation bugs in the optimiser that static
analysis alone cannot detect — e.g. wrong constant folding, incorrect
dead-code elimination, or broken shimmer/type assumptions.

Test categories:
- Multi-pass interaction: cases that exercise several optimisation passes
  working together (constant propagation → folding → DCE → …).
- SSA stress: deeply nested control flow with phi-heavy merge points.
- GVN / CSE verification: redundant computations that should be safe
  (or unsafe) to eliminate.
- DCE / ADCE / DSE: dead code and dead stores that the optimiser should
  remove without changing observable behaviour.
- Taint tracking: data-flow through obfuscated paths.
- Shimmer / thunking: type-representation changes that affect semantics.
- Deliberately obfuscated: adversarial code designed to trip up the
  optimiser with aliasing, dynamic dispatch, and indirect references.

This file is NOT part of ``make test-py`` — run via ``make test-opt``.
"""

from __future__ import annotations

import io
import textwrap

from core.compiler.optimiser import optimise_source
from vm.interp import TclInterp
from vm.types import TclError

# helpers


def _run(source: str) -> tuple[str, str]:
    """Execute *source* in a fresh VM and return ``(stdout, result)``."""
    interp = TclInterp()
    buf = io.StringIO()
    interp.channels["stdout"] = buf
    try:
        result = interp.eval(source)
        return buf.getvalue(), result.value
    except TclError as e:
        return buf.getvalue(), f"ERROR:{e}"


def _run_both(source: str) -> tuple[str, str, str, str, str]:
    """Run original and optimised source; return outputs and optimised text.

    Returns ``(orig_stdout, orig_result, opt_stdout, opt_result, opt_source)``.
    """
    orig_stdout, orig_result = _run(source)
    opt_source, _rewrites = optimise_source(source)
    opt_stdout, opt_result = _run(opt_source)
    return orig_stdout, orig_result, opt_stdout, opt_result, opt_source


def _assert_equiv(source: str, *, strip: bool = True) -> str:
    """Assert that optimised source produces identical VM output.

    Returns the optimised source text for further inspection.
    """
    orig_out, orig_res, opt_out, opt_res, opt_src = _run_both(textwrap.dedent(source))
    if strip:
        orig_out = orig_out.strip()
        opt_out = opt_out.strip()
        orig_res = orig_res.strip()
        opt_res = opt_res.strip()
    assert orig_out == opt_out, (
        f"stdout mismatch:\n  original: {orig_out!r}\n  optimised: {opt_out!r}\n"
        f"  opt source:\n{opt_src}"
    )
    assert orig_res == opt_res, (
        f"result mismatch:\n  original: {orig_res!r}\n  optimised: {opt_res!r}\n"
        f"  opt source:\n{opt_src}"
    )
    return opt_src


# Multi-pass interaction tests


class TestMultiPassInteraction:
    """Cases that require several optimisation passes to cooperate."""

    def test_const_prop_into_fold_into_dce(self):
        """O100 → O101 → O107: constant propagated, folded, dead branch removed."""
        _assert_equiv("""\
            set x 10
            set y 20
            if {$x + $y > 50} {
                puts "big"
            } else {
                puts "small"
            }
        """)

    def test_const_fold_chain_through_multiple_vars(self):
        """O100 chains: a→b→c→d, each folded incrementally."""
        _assert_equiv("""\
            set a 3
            set b [expr {$a * 2}]
            set c [expr {$b + 1}]
            set d [expr {$c * $c}]
            puts $d
        """)

    def test_const_prop_through_string_build_and_fold(self):
        """O100 → O104 → O101: constant string build then numeric fold."""
        _assert_equiv("""\
            set prefix "hello"
            set suffix "world"
            append prefix " $suffix"
            puts $prefix
            set n 5
            set m [expr {$n * 3}]
            puts $m
        """)

    def test_dead_branch_with_side_effects_preserved(self):
        """Optimiser must not remove side effects even in 'dead' looking code."""
        _assert_equiv("""\
            set x 1
            if {$x} {
                puts "yes"
                set y 42
            } else {
                puts "no"
                set y 99
            }
            puts $y
        """)

    def test_nested_constant_if_chain(self):
        """Multiple nested constant-condition ifs: O112 + O101 + O107."""
        _assert_equiv("""\
            set a 1
            set b 0
            if {$a} {
                if {$b} {
                    puts "inner-true"
                } else {
                    puts "inner-false"
                }
            } else {
                puts "outer-false"
            }
        """)

    def test_strength_reduction_in_loop(self):
        """O113 strength reduction + O114 incr idiom in a counted loop."""
        _assert_equiv("""\
            set total 0
            for {set i 0} {$i < 10} {incr i} {
                set total [expr {$total + $i}]
            }
            puts $total
        """)

    def test_const_list_fold_into_foreach(self):
        """O116 constant list fold feeding into foreach iteration."""
        _assert_equiv("""\
            set items [list a b c d]
            foreach item $items {
                puts "item=$item"
            }
        """)

    def test_redundant_expr_in_conditional(self):
        """O115 redundant nested expr + O101 constant fold."""
        _assert_equiv("""\
            set x 10
            set y [expr {[expr {$x + 5}] * 2}]
            puts $y
        """)

    def test_lindex_fold_with_const_list(self):
        """O118 + O116: constant list and constant index."""
        _assert_equiv("""\
            set val [lindex {alpha beta gamma} 1]
            puts $val
        """)

    def test_string_length_zero_check_simplification(self):
        """O117: string length == 0 simplified."""
        _assert_equiv("""\
            set s ""
            if {[string length $s] == 0} {
                puts "empty"
            } else {
                puts "notempty"
            }
        """)

    def test_multi_set_packing(self):
        """O119: consecutive set literals could be packed."""
        _assert_equiv("""\
            set a 1
            set b 2
            set c 3
            puts "$a $b $c"
        """)

    def test_eq_ne_for_string_comparison(self):
        """O120: prefer eq/ne for string comparisons."""
        _assert_equiv("""\
            set s "hello"
            if {$s == "hello"} {
                puts "match"
            }
        """)

    def test_deep_fold_chain_six_levels(self):
        """Six levels of constant propagation and folding."""
        _assert_equiv("""\
            set a 2
            set b [expr {$a + 1}]
            set c [expr {$b * $b}]
            set d [expr {$c - 1}]
            set e [expr {$d / 2}]
            set f [expr {$e % 3}]
            puts $f
        """)

    def test_incr_idiom_and_dead_store(self):
        """O114 incr + O109 dead store elimination."""
        _assert_equiv("""\
            set x 0
            set x [expr {$x + 1}]
            set x [expr {$x + 1}]
            set x [expr {$x + 1}]
            puts $x
        """)

    def test_demorgan_with_constant_operands(self):
        """DeMorgan transform + constant folding."""
        _assert_equiv("""\
            set a 1
            set b 0
            if {!($a && $b)} {
                puts "demorgan-true"
            } else {
                puts "demorgan-false"
            }
        """)

    def test_braced_var_refs(self):
        """Braced variable refs (${x}) must remain semantically equivalent."""
        _assert_equiv("""\
            set x 7
            puts ${x}
            puts "${x}"
        """)

    def test_interpolation_combo_with_expr_fold_and_dead_store(self):
        """O105 interpolation + expr folding + DSE must preserve output."""
        _assert_equiv("""\
            set x 5
            set y [expr {$x + 1}]
            puts "x=$x"
            puts $y
        """)


class TestO120CornerCases:
    """Corner cases and pass interactions for O120."""

    def test_non_string_typed_var_with_boolean_literal_not_rewritten(self):
        source = textwrap.dedent("""\
            set a [expr {1 + 1}]
            if {$a == "true"} {
                puts yes
            } else {
                puts no
            }
        """)
        _assert_equiv(source)
        opt_source, rewrites = optimise_source(source)
        assert '== "true"' in opt_source
        assert not any(r.code == "O120" for r in rewrites)

    def test_known_string_var_with_boolean_literal_rewritten(self):
        source = textwrap.dedent("""\
            set a [string trim " true "]
            if {$a == "true"} {
                puts yes
            } else {
                puts no
            }
        """)
        _assert_equiv(source)
        opt_source, rewrites = optimise_source(source)
        assert '$a eq "true"' in opt_source
        assert any(r.code == "O120" for r in rewrites)

    def test_var_vs_var_known_string_types_rewritten(self):
        source = textwrap.dedent("""\
            set a [string trim "foo"]
            set b [string trim "foo"]
            if {$a == $b} {
                puts equal
            } else {
                puts notequal
            }
        """)
        _assert_equiv(source)
        opt_source, rewrites = optimise_source(source)
        assert "$a eq $b" in opt_source
        assert any(r.code == "O120" for r in rewrites)

    def test_mixed_compare_rewrites_only_string_compare(self):
        source = textwrap.dedent("""\
            set a [string trim "x"]
            set n [clock seconds]
            if {$a == "x" && $n == 1} {
                puts yes
            } else {
                puts no
            }
        """)
        _assert_equiv(source)
        opt_source, rewrites = optimise_source(source)
        assert '$a eq "x"' in opt_source
        assert "$n == 1" in opt_source
        assert any(r.code == "O120" for r in rewrites)

    def test_o120_with_o117_and_o119(self):
        source = textwrap.dedent("""\
            set p 1
            set q 2
            set r 3
            set s ""
            if {[string length $s] == 0} {
                puts empty
            }
            set a [string trim "foo"]
            if {$a == "foo"} {
                puts "$p $q $r"
            }
        """)
        _assert_equiv(source)
        _, rewrites = optimise_source(source)
        codes = {r.code for r in rewrites}
        assert {"O117", "O119", "O120"}.issubset(codes)

    def test_o120_with_o122_tail_recursion_conversion(self):
        source = textwrap.dedent("""\
            proc countdown {n} {
                if {$n <= 0} {
                    return 0
                }
                return [countdown [expr {$n - 1}]]
            }
            set a [string trim "foo"]
            if {$a == "foo"} {
                puts [countdown 3]
            }
        """)
        _assert_equiv(source)
        _, rewrites = optimise_source(source)
        codes = {r.code for r in rewrites}
        assert {"O120", "O122"}.issubset(codes)


# SSA stress tests


class TestSSAStress:
    """Deeply nested control flow with many phi-node merge points."""

    def test_diamond_phi_simple(self):
        """Classic diamond CFG: variable defined on both branches."""
        _assert_equiv("""\
            set x 0
            if {$x == 0} {
                set y 10
            } else {
                set y 20
            }
            puts $y
        """)

    def test_diamond_phi_nested_three_levels(self):
        """Three nested diamonds — six potential phi merges."""
        _assert_equiv("""\
            set a 1
            if {$a} {
                set b 10
                if {$b > 5} {
                    set c 100
                } else {
                    set c 200
                }
            } else {
                set b 20
                if {$b > 15} {
                    set c 300
                } else {
                    set c 400
                }
            }
            puts "$b $c"
        """)

    def test_loop_carried_phi(self):
        """Loop-carried dependency: variable updated each iteration."""
        _assert_equiv("""\
            set acc 0
            for {set i 1} {$i <= 5} {incr i} {
                set acc [expr {$acc + $i}]
            }
            puts $acc
        """)

    def test_loop_with_conditional_update(self):
        """Phi at loop header merges conditional update from body."""
        _assert_equiv("""\
            set val 0
            for {set i 0} {$i < 10} {incr i} {
                if {$i % 2 == 0} {
                    set val [expr {$val + $i}]
                }
            }
            puts $val
        """)

    def test_multiple_variables_through_diamond(self):
        """Several variables all diverge and merge through a diamond."""
        _assert_equiv("""\
            set a 1
            set b 2
            set c 3
            if {$a > 0} {
                set a [expr {$a * 10}]
                set b [expr {$b * 10}]
                set c [expr {$c * 10}]
            } else {
                set a [expr {$a + 10}]
                set b [expr {$b + 10}]
                set c [expr {$c + 10}]
            }
            puts "$a $b $c"
        """)

    def test_nested_while_with_break(self):
        """While loop with early break — phi at join point after loop."""
        _assert_equiv("""\
            set result "none"
            set i 0
            while {$i < 100} {
                if {$i == 7} {
                    set result "found-$i"
                    break
                }
                incr i
            }
            puts $result
        """)

    def test_switch_many_arms(self):
        """Switch with many arms — each arm defines the same variable."""
        _assert_equiv("""\
            set code 3
            switch $code {
                1 { set msg "one" }
                2 { set msg "two" }
                3 { set msg "three" }
                4 { set msg "four" }
                5 { set msg "five" }
                default { set msg "other" }
            }
            puts $msg
        """)

    def test_deeply_nested_loops_accumulator(self):
        """Triple-nested loop — deeply carried phi dependencies."""
        _assert_equiv("""\
            set total 0
            for {set i 0} {$i < 3} {incr i} {
                for {set j 0} {$j < 3} {incr j} {
                    for {set k 0} {$k < 3} {incr k} {
                        set total [expr {$total + 1}]
                    }
                }
            }
            puts $total
        """)

    def test_variable_defined_in_all_switch_arms(self):
        """Exhaustive switch ensures variable always defined — no phi gap."""
        _assert_equiv("""\
            foreach v {a b c d} {
                switch $v {
                    a { set r 1 }
                    b { set r 2 }
                    c { set r 3 }
                    d { set r 4 }
                }
                puts "$v=$r"
            }
        """)

    def test_phi_through_catch(self):
        """Variable set inside catch — phi merges normal and error paths."""
        _assert_equiv("""\
            set x "before"
            catch {
                set x "inside"
            }
            puts $x
        """)


# GVN / CSE verification


class TestGVNCSEEquivalence:
    """Global value numbering and common subexpression elimination."""

    def test_repeated_pure_call_same_args(self):
        """Same pure computation twice — GVN should not break semantics."""
        _assert_equiv("""\
            set x 42
            set a [expr {$x * 3 + 1}]
            set b [expr {$x * 3 + 1}]
            puts "$a $b"
        """)

    def test_repeated_string_length(self):
        """string length of same variable — pure, safe to CSE."""
        _assert_equiv("""\
            set s "hello world"
            set a [string length $s]
            set b [string length $s]
            puts "$a $b"
        """)

    def test_computation_after_mutation_not_cse(self):
        """After variable mutation, recomputation is NOT redundant."""
        _assert_equiv("""\
            set x 10
            set a [expr {$x + 1}]
            set x 20
            set b [expr {$x + 1}]
            puts "$a $b"
        """)

    def test_redundant_across_diamond_dominated(self):
        """Computation in entry block dominates re-computation after if."""
        _assert_equiv("""\
            set x 5
            set a [expr {$x * $x}]
            if {$a > 10} {
                puts "big"
            } else {
                puts "small"
            }
            set b [expr {$x * $x}]
            puts "$a $b"
        """)

    def test_loop_invariant_computation(self):
        """Pure computation inside loop with invariant inputs."""
        _assert_equiv("""\
            set base 7
            set results ""
            for {set i 0} {$i < 5} {incr i} {
                set sq [expr {$base * $base}]
                append results "$sq "
            }
            puts $results
        """)

    def test_gvn_across_unrelated_mutation(self):
        """Mutation of unrelated variable should not invalidate CSE."""
        _assert_equiv("""\
            set x 10
            set a [expr {$x + 1}]
            set y 99
            set b [expr {$x + 1}]
            puts "$a $b $y"
        """)


# DCE / ADCE / DSE verification


class TestDCEADCEDSE:
    """Dead code, aggressive dead code, and dead store elimination."""

    def test_simple_dead_store(self):
        """Variable set but never read — DSE should not change output."""
        _assert_equiv("""\
            set x 42
            set unused 999
            puts $x
        """)

    def test_dead_branch_constant_false(self):
        """Branch on false constant — dead branch removed by DCE."""
        _assert_equiv("""\
            set flag 0
            if {$flag} {
                puts "dead"
            }
            puts "alive"
        """)

    def test_dead_code_after_return(self):
        """Code after return in a proc — unreachable."""
        _assert_equiv("""\
            proc f {} {
                return 42
                puts "unreachable"
            }
            puts [f]
        """)

    def test_transitive_dead_store_chain(self):
        """ADCE: a→b→c where c is dead, making b and a transitively dead."""
        _assert_equiv("""\
            set a 1
            set b [expr {$a + 1}]
            set c [expr {$b + 1}]
            puts "done"
        """)

    def test_dead_store_with_live_sibling(self):
        """One branch of a diamond is dead, other is live."""
        _assert_equiv("""\
            set x 1
            if {$x} {
                set live "yes"
                set dead_var "dead"
            } else {
                set live "no"
                set dead_var "also-dead"
            }
            puts $live
        """)

    def test_dead_loop_with_no_side_effects(self):
        """Loop whose only effect is writing a variable that is overwritten."""
        _assert_equiv("""\
            set x 0
            for {set i 0} {$i < 10} {incr i} {
                set x [expr {$x + $i}]
            }
            set x 999
            puts $x
        """)

    def test_dce_preserves_error_side_effects(self):
        """Even if result is unused, error-producing code must not be removed."""
        _assert_equiv("""\
            set x 1
            catch { expr {1/0} } err
            puts "caught: $err"
        """)

    def test_multiple_dead_stores_same_var(self):
        """Several writes to same variable — only last read matters."""
        _assert_equiv("""\
            set v 1
            set v 2
            set v 3
            set v 4
            set v 5
            puts $v
        """)

    def test_adce_through_proc_return(self):
        """Dead computation in proc whose return value is used."""
        _assert_equiv("""\
            proc compute {} {
                set dead 999
                set useful [expr {2 + 3}]
                return $useful
            }
            puts [compute]
        """)


# Shimmer / thunking stress tests


class TestShimmerThunking:
    """Type representation changes that could affect optimiser correctness."""

    def test_integer_to_string_shimmer(self):
        """Use integer as string then back as integer."""
        _assert_equiv("""\
            set x 42
            set s [string length $x]
            set y [expr {$x + 1}]
            puts "$s $y"
        """)

    def test_list_to_string_shimmer(self):
        """Use list as string (shimmer from list rep to string rep)."""
        _assert_equiv("""\
            set lst {a b c}
            set len [llength $lst]
            set s [string length $lst]
            puts "$len $s"
        """)

    def test_string_to_integer_in_expr(self):
        """String "42" used in arithmetic — shimmer to integer."""
        _assert_equiv("""\
            set s "42"
            set r [expr {$s + 8}]
            puts $r
        """)

    def test_repeated_shimmer_in_loop(self):
        """Shimmer on every iteration — thunking scenario."""
        _assert_equiv("""\
            set v {1 2 3}
            set total 0
            foreach item $v {
                set total [expr {$total + [string length $item]}]
                set total [expr {$total + $item}]
            }
            puts $total
        """)

    def test_boolean_shimmer_in_condition(self):
        """Boolean/integer type shimmer through conditional usage."""
        _assert_equiv("""\
            set flag "1"
            if {$flag} {
                set flag "true"
            }
            if {$flag eq "true"} {
                puts "is-true"
            } else {
                puts "is-false"
            }
        """)

    def test_numeric_string_equality(self):
        """Numeric strings: == does numeric comparison, eq does string."""
        _assert_equiv("""\
            set a "010"
            set b "10"
            set numeric_eq [expr {$a == $b}]
            set string_eq [expr {$a eq $b}]
            puts "numeric=$numeric_eq string=$string_eq"
        """)

    def test_double_to_int_precision(self):
        """Double precision vs integer — optimiser must not assume type."""
        _assert_equiv("""\
            set x 3
            set y [expr {$x / 2}]
            set z [expr {$x / 2.0}]
            puts "int=$y double=$z"
        """)

    def test_octal_leading_zero(self):
        """Leading zeros in Tcl 8.x are octal — must not change semantics."""
        _assert_equiv("""\
            set a 010
            set b [expr {$a + 0}]
            puts $b
        """)


# Taint tracking through obfuscated paths


class TestTaintObfuscated:
    """Data flow through indirect and obfuscated paths."""

    def test_taint_through_string_operations(self):
        """Data flows through string map/range — taint must track."""
        _assert_equiv("""\
            set raw "hello<script>world"
            set clean [string map {< &lt; > &gt;} $raw]
            puts $clean
        """)

    def test_taint_through_list_operations(self):
        """Data flows through lappend/lindex — taint must track."""
        _assert_equiv("""\
            set data {}
            lappend data "first"
            lappend data "second"
            lappend data "third"
            set item [lindex $data 1]
            puts $item
        """)

    def test_taint_through_proc_call(self):
        """Tainted data passed to proc and returned — interprocedural."""
        _assert_equiv("""\
            proc identity {x} { return $x }
            set val "secret"
            set out [identity $val]
            puts $out
        """)

    def test_taint_through_upvar(self):
        """Taint propagation through upvar aliasing."""
        _assert_equiv("""\
            proc setter {name val} {
                upvar 1 $name var
                set var $val
            }
            setter result "tainted-data"
            puts $result
        """)

    def test_taint_through_nested_procs(self):
        """Three-level proc call chain carrying data."""
        _assert_equiv("""\
            proc inner {x} { return "[$x]" }
            proc middle {x} { return [string toupper $x] }
            proc outer {x} { return [middle $x] }
            puts [outer "hello"]
        """)


# Deliberately obfuscated adversarial tests


class TestObfuscatedAdversarial:
    """Adversarial code designed to confuse the optimiser."""

    def test_variable_name_shadows_command(self):
        """Variable named 'set' — must not confuse optimiser."""
        _assert_equiv("""\
            set set 42
            puts $set
        """)

    def test_self_referential_expr(self):
        """Variable used in its own definition expression."""
        _assert_equiv("""\
            set x 1
            set x [expr {$x + $x}]
            set x [expr {$x + $x}]
            puts $x
        """)

    def test_expr_with_string_comparison_and_arithmetic(self):
        """Mixed string/arithmetic in same expression — tricky for type inference."""
        _assert_equiv("""\
            set a "hello"
            set b 42
            set r [expr {$b + [string length $a]}]
            puts $r
        """)

    def test_deeply_nested_command_substitution(self):
        """Five levels of [command] nesting."""
        _assert_equiv("""\
            puts [string length [string toupper [string trim [string map {a b} "  aaa  "]]]]
        """)

    def test_variable_reassignment_between_uses(self):
        """Optimiser must not propagate stale constant after reassignment."""
        _assert_equiv("""\
            set x 1
            puts $x
            set x 2
            puts $x
            set x 3
            puts $x
        """)

    def test_fibonacci_loop(self):
        """Fibonacci computation — stress test for loop-carried phi + fold."""
        _assert_equiv("""\
            set a 0
            set b 1
            for {set i 0} {$i < 10} {incr i} {
                set temp $b
                set b [expr {$a + $b}]
                set a $temp
            }
            puts $a
        """)

    def test_collatz_sequence(self):
        """Collatz computation — conditional update in loop."""
        _assert_equiv("""\
            set n 27
            set steps 0
            while {$n != 1} {
                if {$n % 2 == 0} {
                    set n [expr {$n / 2}]
                } else {
                    set n [expr {3 * $n + 1}]
                }
                incr steps
            }
            puts "$n $steps"
        """)

    def test_obfuscated_constant_through_format(self):
        """Constant value smuggled through format command."""
        _assert_equiv("""\
            set x [format "%d" 42]
            set y [expr {$x + 1}]
            puts $y
        """)

    def test_variable_in_braces_not_substituted(self):
        """Braces suppress substitution — $var should be literal."""
        _assert_equiv("""\
            set x 42
            set a {$x}
            puts $a
        """)

    def test_backslash_continuation_in_expr(self):
        """Backslash-newline continuation inside expression."""
        _assert_equiv("""\
            set x 10
            set y [expr {$x + \\
                5}]
            puts $y
        """)

    def test_semicolon_separated_commands(self):
        """Commands separated by semicolons — all must execute."""
        _assert_equiv("""\
            set a 1; set b 2; set c [expr {$a + $b}]; puts $c
        """)

    def test_empty_string_is_false(self):
        """Empty string is falsy in Tcl boolean context."""
        _assert_equiv("""\
            set x ""
            if {$x eq ""} {
                puts "empty"
            }
        """)

    def test_string_is_integer_ambiguity(self):
        """String "0" is both falsy-integer and truthy-nonempty-string."""
        _assert_equiv("""\
            set v "0"
            if {$v} {
                puts "truthy"
            } else {
                puts "falsy"
            }
            if {$v eq ""} {
                puts "empty"
            } else {
                puts "notempty"
            }
        """)

    def test_proc_default_arg_with_constant(self):
        """Default argument value — constant propagation must not override."""
        _assert_equiv("""\
            proc greet {{name "World"}} {
                return "Hello, $name!"
            }
            puts [greet]
            puts [greet "Tcl"]
        """)

    def test_namespace_variable_scoping(self):
        """Namespace scoping — optimiser must respect scope boundaries."""
        _assert_equiv("""\
            namespace eval myns {
                variable counter 0
                proc increment {} {
                    variable counter
                    incr counter
                    return $counter
                }
            }
            puts [myns::increment]
            puts [myns::increment]
            puts [myns::increment]
        """)

    def test_catch_error_changes_variable(self):
        """catch changes variable on error — must not be optimised away."""
        _assert_equiv("""\
            set result "ok"
            set code [catch {expr {1/0}} result]
            puts "code=$code"
        """)

    def test_double_negation_fold(self):
        """!!x should fold to boolean value of x, not the literal."""
        _assert_equiv("""\
            set x 42
            set y [expr {!!$x}]
            puts $y
        """)

    def test_expr_short_circuit_evaluation(self):
        """Short-circuit: second operand should not evaluate if first decides."""
        _assert_equiv("""\
            set x 0
            set y [expr {$x && [expr {1/0}]}]
            puts $y
        """)

    def test_aliased_variable_through_upvar(self):
        """Two names for the same variable — mutation through one visible in other."""
        _assert_equiv("""\
            proc modify_via_alias {varname} {
                upvar 1 $varname alias
                set alias "modified"
            }
            set original "initial"
            modify_via_alias original
            puts $original
        """)

    def test_computed_variable_name(self):
        """Variable name constructed at runtime — defeats static analysis."""
        _assert_equiv("""\
            set prefix "var"
            set suffix "1"
            set name "${prefix}_${suffix}"
            set var_1 "hello"
            puts [set $name]
        """)

    def test_interleaved_dead_and_live_stores(self):
        """Dead stores interleaved with live ones — precise DSE required."""
        _assert_equiv("""\
            set x 1
            set y 10
            set x 2
            puts $y
            set y 20
            set x 3
            puts $x
            puts $y
        """)


# Complex multi-optimisation scenarios


class TestComplexScenarios:
    """Real-world-ish scenarios combining many features."""

    def test_accumulator_with_conditional_ops(self):
        """Accumulator pattern with mixed arithmetic ops."""
        _assert_equiv("""\
            set acc 0
            foreach op {add sub mul add sub} {
                switch $op {
                    add { set acc [expr {$acc + 10}] }
                    sub { set acc [expr {$acc - 3}] }
                    mul { set acc [expr {$acc * 2}] }
                }
            }
            puts $acc
        """)

    def test_matrix_flatten(self):
        """Nested list flattening — string/list shimmer stress."""
        _assert_equiv("""\
            set matrix {{1 2 3} {4 5 6} {7 8 9}}
            set flat {}
            foreach row $matrix {
                foreach elem $row {
                    lappend flat $elem
                }
            }
            puts $flat
        """)

    def test_string_builder_with_conditional_parts(self):
        """Build a string conditionally — O104 + diamond merge."""
        _assert_equiv("""\
            set msg ""
            set level 2
            append msg "Status: "
            if {$level > 1} {
                append msg "WARNING "
            } else {
                append msg "OK "
            }
            append msg "(level=$level)"
            puts $msg
        """)

    def test_recursive_factorial(self):
        """Recursive proc — interprocedural analysis boundary."""
        _assert_equiv("""\
            proc fact {n} {
                if {$n <= 1} {
                    return 1
                }
                return [expr {$n * [fact [expr {$n - 1}]]}]
            }
            puts [fact 10]
        """)

    def test_mutual_recursion(self):
        """Two mutually recursive procs — tests interprocedural limits."""
        _assert_equiv("""\
            proc is_even {n} {
                if {$n == 0} { return 1 }
                return [is_odd [expr {$n - 1}]]
            }
            proc is_odd {n} {
                if {$n == 0} { return 0 }
                return [is_even [expr {$n - 1}]]
            }
            puts [is_even 10]
            puts [is_odd 7]
        """)

    def test_sieve_of_eratosthenes(self):
        """Sieve algorithm — arrays, loops, conditionals combined."""
        _assert_equiv("""\
            set limit 30
            for {set i 2} {$i <= $limit} {incr i} {
                set is_prime($i) 1
            }
            for {set i 2} {$i * $i <= $limit} {incr i} {
                if {$is_prime($i)} {
                    for {set j [expr {$i * $i}]} {$j <= $limit} {incr j $i} {
                        set is_prime($j) 0
                    }
                }
            }
            set primes {}
            for {set i 2} {$i <= $limit} {incr i} {
                if {$is_prime($i)} {
                    lappend primes $i
                }
            }
            puts $primes
        """)

    def test_string_encoding_pipeline(self):
        """Chain of string transformations — each pure and foldable."""
        _assert_equiv("""\
            set raw "  Hello, WORLD!  "
            set trimmed [string trim $raw]
            set lowered [string tolower $trimmed]
            set mapped [string map {world tcl} $lowered]
            puts $mapped
        """)

    def test_proc_with_variable_args(self):
        """Proc with args — optimiser cannot assume fixed arity."""
        _assert_equiv("""\
            proc sum {args} {
                set total 0
                foreach n $args {
                    set total [expr {$total + $n}]
                }
                return $total
            }
            puts [sum 1 2 3 4 5]
        """)

    def test_complex_loop_with_early_termination(self):
        """Loop with break, continue, and conditional accumulation."""
        _assert_equiv("""\
            set results {}
            for {set i 0} {$i < 20} {incr i} {
                if {$i == 15} { break }
                if {$i % 3 == 0} { continue }
                lappend results $i
            }
            puts $results
        """)

    def test_nested_catch_with_variable_propagation(self):
        """Nested catch blocks — variable state must propagate correctly."""
        _assert_equiv("""\
            set x "initial"
            catch {
                set x "first"
                catch {
                    set x "second"
                }
                set x "$x-after"
            }
            puts $x
        """)

    def test_dynamic_proc_dispatch(self):
        """Dynamic dispatch via variable — defeats interprocedural folding."""
        _assert_equiv("""\
            proc add {a b} { return [expr {$a + $b}] }
            proc sub {a b} { return [expr {$a - $b}] }
            set op "add"
            puts [$op 10 3]
            set op "sub"
            puts [$op 10 3]
        """)

    def test_gcd_euclidean(self):
        """GCD by Euclidean algorithm — classic loop test."""
        _assert_equiv("""\
            proc gcd {a b} {
                while {$b != 0} {
                    set temp $b
                    set b [expr {$a % $b}]
                    set a $temp
                }
                return $a
            }
            puts [gcd 48 18]
            puts [gcd 100 75]
        """)

    def test_insertion_sort(self):
        """Insertion sort — nested loop with shifting."""
        _assert_equiv("""\
            set lst {5 3 8 1 9 2 7 4 6}
            set n [llength $lst]
            for {set i 1} {$i < $n} {incr i} {
                set key [lindex $lst $i]
                set j [expr {$i - 1}]
                while {$j >= 0 && [lindex $lst $j] > $key} {
                    lset lst [expr {$j + 1}] [lindex $lst $j]
                    incr j -1
                }
                lset lst [expr {$j + 1}] $key
            }
            puts $lst
        """)

    def test_closure_over_namespace_variable(self):
        """Proc closing over namespace variable — aliased state."""
        _assert_equiv("""\
            namespace eval counter {
                variable n 0
                proc next {} {
                    variable n
                    incr n
                    return $n
                }
                proc current {} {
                    variable n
                    return $n
                }
            }
            counter::next
            counter::next
            counter::next
            puts [counter::current]
        """)


# SCCP (Sparse Conditional Constant Propagation) edge cases


class TestSCCPEdgeCases:
    """Edge cases for the SCCP lattice and branch resolution."""

    def test_unreachable_branch_with_error(self):
        """Unreachable branch contains an error — must not execute."""
        _assert_equiv("""\
            set x 1
            if {$x} {
                puts "reachable"
            } else {
                error "should never run"
            }
        """)

    def test_chain_of_constant_branches(self):
        """Multiple constant branches in sequence."""
        _assert_equiv("""\
            set a 1
            set b 2
            set c 3
            if {$a == 1} { puts "a-yes" }
            if {$b == 2} { puts "b-yes" }
            if {$c == 3} { puts "c-yes" }
            if {$a == 2} { puts "a-no" }
        """)

    def test_lattice_meet_at_phi(self):
        """Two constant values meeting at phi → overdefined."""
        _assert_equiv("""\
            set x 5
            if {[llength {a b}] > 1} {
                set y [expr {$x + 1}]
            } else {
                set y [expr {$x + 2}]
            }
            puts $y
        """)

    def test_constant_propagation_through_multiple_blocks(self):
        """Constant flows through many basic blocks."""
        _assert_equiv("""\
            set n 100
            set a [expr {$n / 2}]
            set b [expr {$a / 2}]
            set c [expr {$b / 2}]
            set d [expr {$c / 2}]
            puts $d
        """)

    def test_dead_switch_arms(self):
        """Switch on known constant — only matching arm should execute."""
        _assert_equiv("""\
            set x "b"
            set result "none"
            switch $x {
                a { set result "alpha" }
                b { set result "beta" }
                c { set result "gamma" }
            }
            puts $result
        """)


# Expression canonicalisation and reassociation (O110)


class TestExprCanonicalisation:
    """O110: InstCombine-style expression canonicalisation."""

    def test_commutative_reorder(self):
        """a + b same as b + a — canonicalisation must not change value."""
        _assert_equiv("""\
            set a 3
            set b 7
            set r1 [expr {$a + $b}]
            set r2 [expr {$b + $a}]
            puts "$r1 $r2"
        """)

    def test_reassociation_chain(self):
        """((a + b) + c) == (a + (b + c)) for integers."""
        _assert_equiv("""\
            set a 100
            set b 200
            set c 300
            set r [expr {($a + $b) + $c}]
            puts $r
        """)

    def test_distributive_safe(self):
        """Verify that distributive transforms preserve exact value."""
        _assert_equiv("""\
            set x 7
            set r [expr {$x * 3 + $x * 4}]
            puts $r
        """)

    def test_strength_reduction_power_of_two(self):
        """x**2 → x*x — same result."""
        _assert_equiv("""\
            set x 13
            set r [expr {$x ** 2}]
            puts $r
        """)

    def test_strength_reduction_modulo_power_of_two(self):
        """x % 8 → x & 7 for non-negative x."""
        _assert_equiv("""\
            set x 123
            set r [expr {$x % 8}]
            puts $r
        """)


# Adversarial obfuscation: aliasing, dynamic dispatch, encoding tricks


class TestAdversarialObfuscation:
    """Maximally confusing code intended to break incorrect optimisations."""

    def test_rename_builtin_command(self):
        """Rename a builtin — must not assume old name still works."""
        _assert_equiv("""\
            rename puts myputs
            myputs "via renamed puts"
            rename myputs puts
        """)

    def test_eval_computed_script(self):
        """eval of dynamically constructed script — no static analysis possible."""
        _assert_equiv("""\
            set cmd "puts"
            set arg "dynamic"
            eval "$cmd $arg"
        """)

    def test_upvar_aliasing_diamond(self):
        """upvar creates alias; mutation via alias visible in both names."""
        _assert_equiv("""\
            proc swap_via_upvar {a_name b_name} {
                upvar 1 $a_name a $b_name b
                set temp $a
                set a $b
                set b $temp
            }
            set x "hello"
            set y "world"
            swap_via_upvar x y
            puts "$x $y"
        """)

    def test_trace_modifies_variable(self):
        """Variable trace fires on read — changes value mid-read."""
        # NOTE: trace support may be limited; test observable behaviour
        _assert_equiv("""\
            set counter 0
            proc on_read {name1 name2 op} {
                upvar 1 $name1 v
                set v [expr {$v + 1}]
            }
            trace add variable counter read on_read
            set a $counter
            set b $counter
            puts "$a $b"
        """)

    def test_multiline_command_with_comments(self):
        """Multi-line command with inline comments — parse stress."""
        _assert_equiv("""\
            set result [expr {
                1 + 2 +
                3 + 4 +
                5
            }]
            puts $result
        """)

    def test_empty_proc_body(self):
        """Proc with empty body — returns empty string."""
        _assert_equiv("""\
            proc noop {} {}
            set r [noop]
            puts "result='$r'"
        """)

    def test_proc_with_all_defaults(self):
        """Proc where every parameter has a default."""
        _assert_equiv("""\
            proc defaults {{a 1} {b 2} {c 3}} {
                return [expr {$a + $b + $c}]
            }
            puts [defaults]
            puts [defaults 10]
            puts [defaults 10 20]
            puts [defaults 10 20 30]
        """)

    def test_string_with_embedded_newlines(self):
        """String containing literal newlines — must preserve exactly."""
        _assert_equiv("""\
            set s "line1\nline2\nline3"
            puts $s
        """)

    def test_list_string_duality(self):
        """Value is both valid list and meaningful string."""
        _assert_equiv("""\
            set v "a b c"
            set as_list_len [llength $v]
            set as_str_len [string length $v]
            puts "$as_list_len $as_str_len"
        """)

    def test_expr_with_wide_integer(self):
        """Wide integers (> 32-bit) — must not overflow during folding."""
        _assert_equiv("""\
            set big [expr {2**40}]
            set bigger [expr {$big * 2}]
            puts $bigger
        """)

    def test_negative_zero(self):
        """Negative zero in floating-point — edge case."""
        _assert_equiv("""\
            set a [expr {-0.0}]
            set b [expr {0.0}]
            puts [expr {$a == $b}]
        """)

    def test_cascading_proc_calls_with_side_effects(self):
        """Proc call order matters when they have side effects."""
        _assert_equiv("""\
            set log ""
            proc step {n} {
                upvar 1 log log
                append log "$n "
                return $n
            }
            set a [step 1]
            set b [step 2]
            set c [step 3]
            puts $log
            puts "$a $b $c"
        """)

    def test_exception_in_middle_of_computation(self):
        """Exception mid-computation — partial state must be correct."""
        _assert_equiv("""\
            set a 10
            set b 20
            catch {
                set a [expr {$a + 1}]
                set b [expr {1 / 0}]
                set a [expr {$a + 1}]
            }
            puts "$a $b"
        """)

    def test_redefine_proc_mid_execution(self):
        """Proc redefined between calls — second call uses new definition."""
        _assert_equiv("""\
            proc f {} { return "first" }
            puts [f]
            proc f {} { return "second" }
            puts [f]
        """)

    def test_global_variable_from_proc(self):
        """Global variable access from proc — must not confuse scoping."""
        _assert_equiv("""\
            set ::g 42
            proc read_global {} {
                global g
                return $g
            }
            proc write_global {v} {
                global g
                set g $v
            }
            puts [read_global]
            write_global 99
            puts [read_global]
        """)

    def test_nested_loops_with_shared_counter(self):
        """Outer and inner loops sharing variable name (shadowing)."""
        _assert_equiv("""\
            set result ""
            for {set i 0} {$i < 3} {incr i} {
                for {set j 0} {$j < 3} {incr j} {
                    append result "$i$j "
                }
            }
            puts $result
        """)

    def test_string_repeat_and_index(self):
        """String repeat then index — pure but complex."""
        _assert_equiv("""\
            set s [string repeat "abc" 10]
            set c [string index $s 15]
            puts "len=[string length $s] char=$c"
        """)

    def test_lsort_stability(self):
        """lsort with custom comparison — side-effect ordering matters."""
        _assert_equiv("""\
            set data {5 3 1 4 2}
            set sorted [lsort -integer $data]
            puts $sorted
        """)

    def test_info_exists_governs_read(self):
        """info exists before read — optimiser must not move read before check."""
        _assert_equiv("""\
            if {[info exists ::maybe]} {
                puts "exists: $::maybe"
            } else {
                puts "does not exist"
            }
        """)
