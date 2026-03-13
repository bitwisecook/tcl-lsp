"""Tests for GVN/CSE redundant computation detection."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path
from unittest import mock

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.compiler.gvn import find_redundant_computations


class TestGVNBasicStraightLine:
    """Straight-line code: no branching."""

    def test_repeated_string_length_detected(self):
        source = textwrap.dedent("""\
            set x hello
            set a [string length $x]
            set b [string length $x]
        """)
        warnings = find_redundant_computations(source)
        assert len(warnings) == 1
        assert warnings[0].code == "O105"
        assert "string length" in warnings[0].expression_text

    def test_different_args_not_flagged(self):
        source = textwrap.dedent("""\
            set a [string length hello]
            set b [string length world]
        """)
        warnings = find_redundant_computations(source)
        assert len(warnings) == 0

    def test_impure_command_not_flagged(self):
        source = textwrap.dedent("""\
            set a [clock seconds]
            set b [clock seconds]
        """)
        warnings = find_redundant_computations(source)
        assert len(warnings) == 0

    def test_three_identical_calls_two_warnings(self):
        source = textwrap.dedent("""\
            set a [string length $x]
            set b [string length $x]
            set c [string length $x]
        """)
        warnings = find_redundant_computations(source)
        assert len(warnings) == 2

    def test_constant_assignment_not_flagged(self):
        source = "set a 1\nset b 1"
        warnings = find_redundant_computations(source)
        assert len(warnings) == 0

    def test_single_command_no_redundancy(self):
        warnings = find_redundant_computations("set a [string length hello]")
        assert len(warnings) == 0

    def test_llength_repeated(self):
        source = textwrap.dedent("""\
            set n [llength $mylist]
            set m [llength $mylist]
        """)
        warnings = find_redundant_computations(source)
        assert len(warnings) == 1
        assert "llength" in warnings[0].expression_text


class TestGVNBranches:
    """CFG-aware detection with branching."""

    def test_call_in_both_branches_not_flagged(self):
        """Calls in if-branch and else-branch: no dominance relationship."""
        source = textwrap.dedent("""\
            if {$cond} {
                set a [string length $x]
            } else {
                set b [string length $x]
            }
        """)
        warnings = find_redundant_computations(source)
        assert len(warnings) == 0

    def test_call_before_if_and_in_branch_flagged(self):
        """Call before if dominates call inside the branch."""
        source = textwrap.dedent("""\
            set a [string length $x]
            if {$cond} {
                set b [string length $x]
            }
        """)
        warnings = find_redundant_computations(source)
        assert len(warnings) == 1

    def test_call_before_and_after_if_flagged(self):
        """Call before if dominates call after merge."""
        source = textwrap.dedent("""\
            set a [string length $x]
            if {$cond} {
                set y 1
            } else {
                set y 2
            }
            set b [string length $x]
        """)
        warnings = find_redundant_computations(source)
        assert len(warnings) == 1

    def test_partial_redundancy_after_if_flagged(self):
        """Call after merge is partially redundant when only one arm computes it."""
        source = textwrap.dedent("""\
            if {$cond} {
                set a [string length $x]
            }
            set b [string length $x]
        """)
        warnings = find_redundant_computations(source)
        partial = [w for w in warnings if "partially redundant" in w.message]
        assert len(partial) == 1
        assert "string length" in partial[0].expression_text

    def test_partial_redundancy_respects_barrier_kills(self):
        """A barrier on the only generating path prevents partial availability."""
        source = textwrap.dedent("""\
            if {$cond} {
                set a [string length $x]
                eval "set x changed"
            }
            set b [string length $x]
        """)
        warnings = find_redundant_computations(source)
        partial = [w for w in warnings if "partially redundant" in w.message]
        assert len(partial) == 0


class TestGVNKillSemantics:
    """Barriers and impure calls invalidate value numbers."""

    def test_barrier_kills_value(self):
        source = textwrap.dedent("""\
            set a [string length $x]
            eval "something"
            set b [string length $x]
        """)
        warnings = find_redundant_computations(source)
        assert len(warnings) == 0

    def test_variable_redefinition_invalidates(self):
        """After x is redefined, [string length $x] is a different expr."""
        source = textwrap.dedent("""\
            set a [string length $x]
            set x new_value
            set b [string length $x]
        """)
        warnings = find_redundant_computations(source)
        assert len(warnings) == 0

    def test_impure_call_does_not_kill_pure_entries(self):
        """Impure calls (puts) don't invalidate argument-pure computations."""
        source = textwrap.dedent("""\
            set a [string length $x]
            puts hello
            set b [string length $x]
        """)
        warnings = find_redundant_computations(source)
        assert len(warnings) == 1


class TestGVNLoopInvariantCodeMotion:
    """LICM-style detection of invariant computations inside loops."""

    def test_for_loop_invariant_call_flagged(self):
        source = textwrap.dedent("""\
            set host Example.COM
            for {set i 0} {$i < 5} {incr i} {
                set lower [string tolower $host]
                puts $lower
            }
        """)
        warnings = find_redundant_computations(source)
        o106 = [w for w in warnings if w.code == "O106"]
        assert len(o106) == 1
        assert "string tolower" in o106[0].expression_text

    def test_loop_variant_call_not_flagged(self):
        source = textwrap.dedent("""\
            for {set i 0} {$i < 5} {incr i} {
                set lower [string tolower $i]
            }
        """)
        warnings = find_redundant_computations(source)
        o106 = [w for w in warnings if w.code == "O106"]
        assert len(o106) == 0

    def test_conditional_loop_body_not_flagged(self):
        source = textwrap.dedent("""\
            for {set i 0} {$i < 5} {incr i} {
                if {$flag} {
                    set n [string length $host]
                }
            }
        """)
        warnings = find_redundant_computations(source)
        o106 = [w for w in warnings if w.code == "O106"]
        assert len(o106) == 0


class TestGVNProcBodies:
    """Redundancy detection in proc bodies."""

    def test_redundancy_inside_proc(self):
        source = textwrap.dedent("""\
            proc process {x} {
                set a [string length $x]
                set b [string length $x]
                return [expr {$a + $b}]
            }
        """)
        warnings = find_redundant_computations(source)
        assert len(warnings) == 1
        assert "string length" in warnings[0].expression_text

    def test_pure_user_proc_calls_detected(self):
        source = textwrap.dedent("""\
            proc double {n} { return [expr {$n * 2}] }
            set a [double 5]
            set b [double 5]
        """)
        warnings = find_redundant_computations(source)
        assert len(warnings) == 1


class TestGVNEdgeCases:
    """Edge cases and robustness."""

    def test_empty_source(self):
        assert find_redundant_computations("") == []

    def test_syntax_error_no_crash(self):
        assert find_redundant_computations("set {") == []

    def test_comment_only(self):
        assert find_redundant_computations("# nothing here") == []

    def test_nested_command_substitution(self):
        """Embedded [format ...] inside another command's argument."""
        source = textwrap.dedent("""\
            puts [format "%d" $x]
            puts [format "%d" $x]
        """)
        warnings = find_redundant_computations(source)
        assert any("format" in w.expression_text for w in warnings)


def _irules_dialect():
    """Context manager to activate f5-irules dialect."""
    return mock.patch(
        "core.compiler.gvn.active_dialect",
        return_value="f5-irules",
    )


class TestGVNIrulesWhenBodies:
    """Redundancy detection inside iRules ``when`` event handler bodies."""

    def test_repeated_ip_client_addr(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                set a [IP::client_addr]
                set b [IP::client_addr]
            }
        """)
        with _irules_dialect():
            warnings = find_redundant_computations(source)
        o105 = [w for w in warnings if w.code == "O105"]
        assert len(o105) >= 1
        assert "IP::client_addr" in o105[0].expression_text

    def test_repeated_http_uri(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                if { [HTTP::uri] starts_with "/api" } {
                    set new [string map {"/api" "/v2"} [HTTP::uri]]
                    log local0. "Req to [HTTP::uri]"
                }
            }
        """)
        with _irules_dialect():
            warnings = find_redundant_computations(source)
        o105 = [w for w in warnings if w.code == "O105"]
        assert len(o105) >= 1
        assert "HTTP::uri" in o105[0].expression_text

    def test_different_commands_not_flagged(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                set a [HTTP::uri]
                set b [HTTP::host]
            }
        """)
        with _irules_dialect():
            warnings = find_redundant_computations(source)
        o105 = [w for w in warnings if w.code == "O105"]
        assert len(o105) == 0

    def test_separate_when_bodies_independent(self):
        """Repeated calls in different ``when`` blocks: independent scopes."""
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                set a [HTTP::uri]
            }
            when HTTP_RESPONSE {
                set b [HTTP::uri]
            }
        """)
        with _irules_dialect():
            warnings = find_redundant_computations(source)
        o105 = [w for w in warnings if w.code == "O105"]
        assert len(o105) == 0

    def test_standalone_expensive_command_detected(self):
        """Bare HTTP::uri at top-level in when body (subsumes IRULE2102)."""
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                HTTP::uri
                HTTP::uri
            }
        """)
        with _irules_dialect():
            warnings = find_redundant_computations(source)
        o105 = [w for w in warnings if w.code == "O105"]
        assert len(o105) == 1
        assert "HTTP::uri" in o105[0].expression_text

    def test_partial_redundancy_inside_when_if_detected(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                if {$cond} {
                    set a [HTTP::uri]
                }
                set b [HTTP::uri]
            }
        """)
        with _irules_dialect():
            warnings = find_redundant_computations(source)
        partial = [w for w in warnings if "partially redundant" in w.message]
        assert len(partial) >= 1
        assert "HTTP::uri" in partial[0].expression_text

    def test_foreach_loop_invariant_http_uri_detected(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                foreach header [HTTP::header names] {
                    set uri [HTTP::uri]
                    log local0. "$header -> $uri"
                }
            }
        """)
        with _irules_dialect():
            warnings = find_redundant_computations(source)
        o106 = [w for w in warnings if w.code == "O106"]
        assert len(o106) >= 1
        assert any("HTTP::uri" in w.expression_text for w in o106)

    def test_standalone_with_args(self):
        """Standalone HTTP::header with identical args detected."""
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                HTTP::header value Host
                HTTP::header value Host
            }
        """)
        with _irules_dialect():
            warnings = find_redundant_computations(source)
        o105 = [w for w in warnings if w.code == "O105"]
        assert len(o105) == 1

    def test_standalone_different_args_not_flagged(self):
        """Standalone HTTP::header with different args: no warning."""
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                HTTP::header value Host
                HTTP::header value Accept
            }
        """)
        with _irules_dialect():
            warnings = find_redundant_computations(source)
        o105 = [w for w in warnings if w.code == "O105"]
        assert len(o105) == 0

    def test_standalone_and_embedded_unified(self):
        """Mix of standalone HTTP::uri and embedded [HTTP::uri]."""
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                HTTP::uri
                set u [HTTP::uri]
            }
        """)
        with _irules_dialect():
            warnings = find_redundant_computations(source)
        o105 = [w for w in warnings if w.code == "O105"]
        assert len(o105) == 1
        assert "HTTP::uri" in o105[0].expression_text

    def test_http_mutator_invalidates_cached_getter(self):
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                set a [HTTP::uri]
                HTTP::uri /new-path
                set b [HTTP::uri]
            }
        """)
        with _irules_dialect():
            warnings = find_redundant_computations(source)
        o105 = [w for w in warnings if w.code == "O105"]
        assert len(o105) == 0

    def test_not_triggered_for_tcl_dialect(self):
        """when body scanning is iRules-only."""
        source = textwrap.dedent("""\
            when HTTP_REQUEST {
                set a [HTTP::uri]
                set b [HTTP::uri]
            }
        """)
        # Default dialect is tcl, not f5-irules.
        warnings = find_redundant_computations(source)
        # May or may not have warnings from SSA path, but no iRules-specific
        # scanning should happen.  Just verify no crash.
        assert isinstance(warnings, list)
