"""Tests for the var-escape analysis."""

from __future__ import annotations

import textwrap

from core.compiler.var_escape import (
    TOP_LEVEL_QNAME,
    EscapeTag,
    ProcEscapeSummary,
    analyse_var_escape,
)


def _summary(source: str, qname: str = "::foo") -> ProcEscapeSummary:
    """Run the analysis and return the summary for ``qname``."""
    result = analyse_var_escape(source=textwrap.dedent(source))
    assert qname in result, f"qname {qname!r} not in {sorted(result)!r}"
    return result[qname]


# Lattice basics


class TestLattice:
    def test_default_tag_is_local(self):
        summary = ProcEscapeSummary()
        assert summary.tag("anything") is EscapeTag.LOCAL
        assert not summary.is_frame("anything")

    def test_frame_overrides_default(self):
        summary = ProcEscapeSummary(tags={"x": EscapeTag.FRAME})
        assert summary.is_frame("x")
        assert not summary.is_frame("y")

    def test_dynamic_barrier_forces_frame(self):
        summary = ProcEscapeSummary(dynamic_barrier=True)
        assert summary.is_frame("any_var")


# Structural pass


class TestAnalysisEntryPoint:
    def test_empty_proc_has_no_frame_need(self):
        summary = _summary(
            """
            proc foo {} {}
            """
        )
        assert not summary.frame_needed
        assert not summary.dynamic_barrier
        assert summary.tags == {}

    def test_top_level_is_keyed(self):
        result = analyse_var_escape(source="set x 1")
        assert TOP_LEVEL_QNAME in result


# Rule 1 — upvar / global / variable bindings


class TestUpvarGlobalVariable:
    def test_upvar_default_level_escapes_named_var(self):
        # ``upvar src x`` defaults to level 1 — x is still name-bounded.
        summary = _summary(
            """
            proc foo {} {
                upvar src x
                set x 1
            }
            """
        )
        assert summary.frame_needed
        assert summary.is_frame("x")
        assert not summary.dynamic_barrier

    def test_upvar_level_1_escapes_named_var(self):
        summary = _summary(
            """
            proc foo {} {
                upvar 1 src x
                set x 1
            }
            """
        )
        assert summary.is_frame("x")

    def test_upvar_level_hash_0_escapes_named_var(self):
        summary = _summary(
            """
            proc foo {} {
                upvar #0 src x
                set x 1
            }
            """
        )
        assert summary.is_frame("x")
        assert not summary.dynamic_barrier

    def test_global_escapes_listed_vars(self):
        summary = _summary(
            """
            proc foo {} {
                global g1 g2
                set local 1
                set g1 2
            }
            """
        )
        assert summary.is_frame("g1")
        assert summary.is_frame("g2")
        assert not summary.is_frame("local")

    def test_variable_escapes_declared_names(self):
        summary = _summary(
            """
            proc foo {} {
                variable v1 default v2
                set v1 1
                set local 2
            }
            """
        )
        assert summary.is_frame("v1")
        assert summary.is_frame("v2")
        assert not summary.is_frame("local")


# Rule 2 — dynamic var refs


class TestDynamicVarRefs:
    def test_set_dynamic_name_escapes_all_known(self):
        summary = _summary(
            """
            proc foo {} {
                set a 1
                set b 2
                set $name value
            }
            """
        )
        # ``$name`` is the variable-name argument — we can't bound it, so
        # every known name spills.
        assert summary.is_frame("a")
        assert summary.is_frame("b")

    def test_incr_dynamic_name_escapes_all(self):
        summary = _summary(
            """
            proc foo {} {
                set counter 0
                incr $name
            }
            """
        )
        assert summary.is_frame("counter")

    def test_unset_dynamic_name_escapes_all(self):
        summary = _summary(
            """
            proc foo {} {
                set x 1
                unset $n
            }
            """
        )
        assert summary.is_frame("x")

    def test_set_literal_name_does_not_escape(self):
        summary = _summary(
            """
            proc foo {} {
                set x 1
                set y 2
            }
            """
        )
        assert not summary.frame_needed


# Rule 3 — eval body


class TestEval:
    def test_eval_literal_body_escapes_named_refs(self):
        summary = _summary(
            """
            proc foo {} {
                set x 1
                set y 2
                eval {set x 99}
            }
            """
        )
        assert summary.is_frame("x")

    def test_eval_dynamic_body_is_pessimistic(self):
        summary = _summary(
            """
            proc foo {script} {
                set x 1
                eval $script
            }
            """
        )
        assert summary.dynamic_barrier
        assert summary.frame_needed

    def test_eval_does_not_escape_untouched_vars(self):
        # After a literal-body eval, vars it didn't reference stay LOCAL.
        summary = _summary(
            """
            proc foo {} {
                set touched 1
                set pristine 2
                eval {set touched 99}
            }
            """
        )
        assert summary.is_frame("touched")
        assert not summary.is_frame("pristine")


# Rule 4 — uplevel level


class TestUplevel:
    def test_uplevel_hash_0_literal_is_not_pessimistic(self):
        summary = _summary(
            """
            proc foo {} {
                set x 1
                uplevel #0 {set ::g 1}
            }
            """
        )
        # #0 runs in global scope; our locals aren't visible.
        assert not summary.dynamic_barrier

    def test_uplevel_level_1_is_pessimistic(self):
        summary = _summary(
            """
            proc foo {} {
                set x 1
                uplevel 1 {set x 99}
            }
            """
        )
        assert summary.dynamic_barrier

    def test_uplevel_dynamic_level_is_pessimistic(self):
        summary = _summary(
            """
            proc foo {lvl} {
                set x 1
                uplevel $lvl {set x 99}
            }
            """
        )
        assert summary.dynamic_barrier


# Rule 5 — info subcommands


class TestInfoSubcommand:
    def test_info_level_is_pessimistic(self):
        summary = _summary(
            """
            proc foo {} {
                set x 1
                set lvl [info level]
            }
            """
        )
        assert summary.dynamic_barrier

    def test_info_frame_is_pessimistic(self):
        summary = _summary(
            """
            proc foo {} {
                set x 1
                set f [info frame]
            }
            """
        )
        assert summary.dynamic_barrier

    def test_info_vars_is_pessimistic(self):
        summary = _summary(
            """
            proc foo {} {
                set x 1
                set v [info vars]
            }
            """
        )
        assert summary.dynamic_barrier

    def test_info_locals_is_pessimistic(self):
        summary = _summary(
            """
            proc foo {} {
                set x 1
                set l [info locals]
            }
            """
        )
        assert summary.dynamic_barrier

    def test_info_exists_literal_escapes_named_var(self):
        summary = _summary(
            """
            proc foo {} {
                set x 1
                set y 2
                if {[info exists x]} {
                    set y 3
                }
            }
            """
        )
        assert summary.is_frame("x")
        assert not summary.dynamic_barrier

    def test_info_exists_dynamic_is_pessimistic(self):
        summary = _summary(
            """
            proc foo {n} {
                set x 1
                if {[info exists $n]} {}
            }
            """
        )
        assert summary.dynamic_barrier

    def test_info_body_is_safe(self):
        summary = _summary(
            """
            proc foo {} {
                set x 1
                set b [info body foo]
            }
            """
        )
        assert not summary.dynamic_barrier

    def test_info_commands_is_safe(self):
        summary = _summary(
            """
            proc foo {} {
                set x 1
                set c [info commands]
            }
            """
        )
        assert not summary.dynamic_barrier


# Rule flow — structural recursion


class TestStructuredControlFlow:
    def test_escape_inside_if_body_propagates(self):
        summary = _summary(
            """
            proc foo {cond} {
                if {$cond} {
                    upvar 1 src x
                    set x 1
                }
            }
            """
        )
        assert summary.is_frame("x")

    def test_escape_inside_foreach_propagates(self):
        summary = _summary(
            """
            proc foo {items} {
                foreach item $items {
                    upvar 1 src alias
                    set alias $item
                }
            }
            """
        )
        assert summary.is_frame("alias")

    def test_nested_if_in_proc(self):
        summary = _summary(
            """
            proc foo {a b} {
                if {$a} {
                    if {$b} {
                        upvar 1 outer x
                    }
                }
                set local 1
            }
            """
        )
        assert summary.is_frame("x")
        assert not summary.is_frame("local")


# Integration — pristine proc gets zero escape


class TestPristineProc:
    def test_arithmetic_proc_needs_no_frame(self):
        summary = _summary(
            """
            proc add {a b} {
                set sum [expr {$a + $b}]
                return $sum
            }
            """,
            qname="::add",
        )
        assert not summary.frame_needed
        assert not summary.dynamic_barrier
        assert not summary.tags

    def test_list_manipulation_proc_needs_no_frame(self):
        summary = _summary(
            """
            proc reverse_list {xs} {
                set out [list]
                foreach x $xs {
                    set out [linsert $out 0 $x]
                }
                return $out
            }
            """,
            qname="::reverse_list",
        )
        assert not summary.frame_needed

    def test_mixed_proc_spills_only_escaped_vars(self):
        summary = _summary(
            """
            proc foo {} {
                set local_a 1
                set local_b 2
                upvar 1 src alias
                set alias $local_a
            }
            """
        )
        assert summary.is_frame("alias")
        assert not summary.is_frame("local_a")
        assert not summary.is_frame("local_b")


# Interprocedural propagation


class TestInterproceduralUpvar:
    def test_callee_upvar_source_escapes_caller_local(self):
        result = analyse_var_escape(
            source=textwrap.dedent(
                """
                proc bar {} {
                    upvar 1 shared local
                    set local 1
                }
                proc foo {} {
                    set shared 1
                    bar
                }
                """
            )
        )
        # ``foo``'s ``shared`` must be FRAME because ``bar`` aliases it
        # by name from the caller's frame.
        assert result["::foo"].is_frame("shared")
        # ``bar``'s ``local`` (the alias target) is still FRAME.
        assert result["::bar"].is_frame("local")

    def test_callee_upvar_propagates_transitively(self):
        result = analyse_var_escape(
            source=textwrap.dedent(
                """
                proc leaf {} {
                    upvar 1 shared local
                    set local 1
                }
                proc middle {} {
                    leaf
                }
                proc top {} {
                    set shared 42
                    middle
                }
                """
            )
        )
        # Even though ``top`` only calls ``middle`` directly, the
        # transitive closure should spill ``shared`` because ``leaf``
        # names it as an upvar source.
        assert result["::top"].is_frame("shared")

    def test_unbounded_callee_upvar_marks_caller_pessimistic(self):
        result = analyse_var_escape(
            source=textwrap.dedent(
                """
                proc bar {name} {
                    upvar 1 $name local
                    set local 1
                }
                proc foo {} {
                    set x 1
                    bar x
                }
                """
            )
        )
        # ``bar``'s dynamic-source upvar makes it pessimistic; ``foo``
        # inherits the unbounded flag and must spill every local.
        assert result["::foo"].dynamic_barrier

    def test_unrelated_callee_does_not_escape_caller_local(self):
        result = analyse_var_escape(
            source=textwrap.dedent(
                """
                proc helper {} {
                    return 42
                }
                proc foo {} {
                    set x 1
                    helper
                }
                """
            )
        )
        # ``helper`` has no upvars; ``foo``'s ``x`` stays LOCAL.
        assert not result["::foo"].is_frame("x")
        assert not result["::foo"].frame_needed

    def test_source_set_accumulates_from_multiple_callees(self):
        result = analyse_var_escape(
            source=textwrap.dedent(
                """
                proc reader {} {
                    upvar 1 a alias_a
                    return $alias_a
                }
                proc writer {} {
                    upvar 1 b alias_b
                    set alias_b 99
                }
                proc top {} {
                    set a 1
                    set b 2
                    set c 3
                    reader
                    writer
                }
                """
            )
        )
        assert result["::top"].is_frame("a")
        assert result["::top"].is_frame("b")
        # ``c`` is not named by any callee — stays LOCAL.
        assert not result["::top"].is_frame("c")

    def test_interprocedural_disabled_returns_per_proc(self):
        result = analyse_var_escape(
            source=textwrap.dedent(
                """
                proc bar {} {
                    upvar 1 shared local
                }
                proc foo {} {
                    set shared 1
                    bar
                }
                """
            ),
            interprocedural=False,
        )
        # Without the fixpoint, ``foo``'s ``shared`` is not escaped.
        assert not result["::foo"].is_frame("shared")
        # ``bar`` still has the raw source-name set recorded though.
        assert "shared" in result["::bar"].upvar_source_names
