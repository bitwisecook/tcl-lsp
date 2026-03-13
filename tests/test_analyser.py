"""Tests for the Tcl semantic analyser."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.analyser import analyse
from core.analysis.semantic_model import Severity


class TestProcAnalysis:
    def test_simple_proc(self):
        result = analyse("proc greet {name} { puts $name }")
        assert "greet" in result.global_scope.procs
        proc = result.global_scope.procs["greet"]
        assert proc.name == "greet"
        assert proc.qualified_name == "::greet"
        assert len(proc.params) == 1
        assert proc.params[0].name == "name"

    def test_proc_multiple_params(self):
        result = analyse("proc add {a b} { return [+ $a $b] }")
        proc = result.global_scope.procs["add"]
        assert len(proc.params) == 2
        assert proc.params[0].name == "a"
        assert proc.params[1].name == "b"

    def test_proc_default_param(self):
        result = analyse("proc greet {{name World}} { puts $name }")
        proc = result.global_scope.procs["greet"]
        assert proc.params[0].name == "name"
        assert proc.params[0].has_default is True
        assert proc.params[0].default_value == "World"

    def test_proc_no_params(self):
        result = analyse("proc noop {} { }")
        proc = result.global_scope.procs["noop"]
        assert len(proc.params) == 0

    def test_proc_in_all_procs(self):
        result = analyse("proc foo {} {}")
        assert "::foo" in result.all_procs

    def test_proc_doc_from_comment(self):
        source = textwrap.dedent("""\
            # Adds two numbers
            proc add {a b} { return [+ $a $b] }
        """)
        result = analyse(source)
        proc = result.global_scope.procs["add"]
        assert "Adds two numbers" in proc.doc

    def test_multiple_procs(self):
        source = textwrap.dedent("""\
            proc foo {} {}
            proc bar {x} { return $x }
        """)
        result = analyse(source)
        assert "foo" in result.global_scope.procs
        assert "bar" in result.global_scope.procs

    def test_proc_creates_child_scope(self):
        result = analyse("proc foo {x} { set y 1 }")
        assert len(result.global_scope.children) == 1
        proc_scope = result.global_scope.children[0]
        assert proc_scope.kind == "proc"
        assert proc_scope.name == "foo"
        # 'x' is a parameter, 'y' is set inside
        assert "x" in proc_scope.variables
        assert "y" in proc_scope.variables


class TestVariableAnalysis:
    def test_set_defines_var(self):
        result = analyse("set x 42")
        assert "x" in result.global_scope.variables

    def test_multiple_sets(self):
        source = "set x 1\nset y 2\nset z 3"
        result = analyse(source)
        assert "x" in result.global_scope.variables
        assert "y" in result.global_scope.variables
        assert "z" in result.global_scope.variables

    def test_incr_defines_var(self):
        result = analyse("incr counter")
        assert "counter" in result.global_scope.variables

    def test_array_var(self):
        result = analyse("set arr(key) value")
        # Should define 'arr' (base name)
        assert "arr" in result.global_scope.variables

    def test_var_in_proc_scope(self):
        result = analyse("proc foo {} { set local 1 }")
        # 'local' should not be in global scope
        assert "local" not in result.global_scope.variables
        proc_scope = result.global_scope.children[0]
        assert "local" in proc_scope.variables

    def test_set_one_arg_is_read_only(self):
        result = analyse("set x")
        assert "x" not in result.global_scope.variables

    def test_variable_command_skips_value_words(self):
        result = analyse("variable port 8080")
        assert "port" in result.global_scope.variables
        assert "8080" not in result.global_scope.variables


class TestNamespaceAnalysis:
    def test_namespace_eval(self):
        source = textwrap.dedent("""\
            namespace eval myns {
                proc helper {} { return 1 }
            }
        """)
        result = analyse(source)
        assert len(result.global_scope.children) == 1
        ns = result.global_scope.children[0]
        assert ns.kind == "namespace"
        assert ns.name == "myns"
        assert "helper" in ns.procs

    def test_namespace_qualified_proc(self):
        source = textwrap.dedent("""\
            namespace eval math {
                proc add {a b} { return [+ $a $b] }
            }
        """)
        result = analyse(source)
        assert "::math::add" in result.all_procs


# Arity checking / diagnostics
class TestDiagnostics:
    def test_too_few_args_set(self):
        result = analyse("set")
        errors = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(errors) >= 1
        assert "Too few" in errors[0].message

    def test_too_many_args_break(self):
        result = analyse("break extra")
        errors = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(errors) >= 1
        assert "Too many" in errors[0].message

    def test_correct_arity_no_error(self):
        result = analyse("set x 42")
        errors = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(errors) == 0

    def test_puts_too_many_args(self):
        result = analyse("puts a b c")
        errors = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(errors) >= 1

    def test_while_too_few_args(self):
        result = analyse("while {1}")
        errors = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(errors) >= 1

    def test_unknown_subcommand_warning(self):
        result = analyse("string bogus hello")
        warnings = [d for d in result.diagnostics if d.severity == Severity.WARNING]
        assert len(warnings) >= 1
        assert "Unknown subcommand" in warnings[0].message

    def test_missing_subcommand(self):
        result = analyse("namespace")
        errors = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(errors) >= 1
        assert "subcommand" in errors[0].message.lower()

    def test_proc_arity_not_checked_for_unknown(self):
        # Unknown commands should not produce arity errors
        result = analyse("mycommand a b c d e")
        errors = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(errors) == 0

    def test_error_has_range(self):
        result = analyse("set")
        assert len(result.diagnostics) >= 1
        d = result.diagnostics[0]
        assert d.range.start.line == 0
        assert d.range.start.offset == 0

    def test_for_too_few_args(self):
        result = analyse("for {set i 0} {$i < 10}")
        errors = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(errors) >= 1

    def test_proc_call_too_few_args(self):
        source = textwrap.dedent("""\
            proc add {a b} { return [expr {$a + $b}] }
            add 1
        """)
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.code == "E002"]
        assert len(errors) >= 1
        assert "::add" in errors[0].message

    def test_proc_call_too_many_args(self):
        source = textwrap.dedent("""\
            proc add {a b} { return [expr {$a + $b}] }
            add 1 2 3
        """)
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.code == "E003"]
        assert len(errors) >= 1
        assert "::add" in errors[0].message

    def test_proc_call_with_default_arg(self):
        source = textwrap.dedent("""\
            proc greet {name {title Mr}} { return "$title $name" }
            greet Bob
        """)
        result = analyse(source)
        proc_arity = [
            d for d in result.diagnostics if d.code in ("E002", "E003") and "::greet" in d.message
        ]
        assert len(proc_arity) == 0

    def test_proc_call_variadic_args(self):
        source = textwrap.dedent("""\
            proc log {level args} { return $level }
            log info a b c
        """)
        result = analyse(source)
        proc_arity = [
            d for d in result.diagnostics if d.code in ("E002", "E003") and "::log" in d.message
        ]
        assert len(proc_arity) == 0

    def test_namespace_proc_call_arity(self):
        source = textwrap.dedent("""\
            namespace eval math {
                proc add {a b} { return [expr {$a + $b}] }
                add 1
            }
        """)
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.code == "E002" and "::math::add" in d.message]
        assert len(errors) >= 1

    def test_multiline_diagnostics(self):
        source = "set x 1\nbreak extra\nset y 2"
        result = analyse(source)
        errors = [d for d in result.diagnostics if d.severity == Severity.ERROR]
        assert len(errors) >= 1
        # The error should be on line 1 (0-indexed)
        assert errors[0].range.start.line == 1

    def test_read_before_set_warning(self):
        result = analyse("puts $x")
        warnings = [d for d in result.diagnostics if d.code == "W210"]
        assert len(warnings) == 1
        assert warnings[0].severity == Severity.WARNING

    def test_read_before_set_warning_in_expr(self):
        result = analyse("if {$x > 0} { puts yes }")
        warnings = [d for d in result.diagnostics if d.code == "W210"]
        assert len(warnings) == 1

    def test_read_before_set_clears_after_assignment(self):
        result = analyse("set x 1\nputs $x")
        warnings = [d for d in result.diagnostics if d.code == "W210"]
        assert len(warnings) == 0

    def test_array_read_uses_base_variable(self):
        result = analyse("set arr(key) 1\nputs $arr(key)")
        warnings = [d for d in result.diagnostics if d.code == "W210"]
        assert len(warnings) == 0

    def test_unused_assigned_variable_hint(self):
        result = analyse("proc foo {} { set x 1 }")
        hints = [d for d in result.diagnostics if d.code == "W211"]
        assert len(hints) == 1
        assert hints[0].severity == Severity.HINT

    def test_used_variable_no_unused_hint(self):
        result = analyse("proc foo {} { set x 1; puts $x }")
        hints = [d for d in result.diagnostics if d.code == "W211"]
        assert len(hints) == 0

    def test_dead_assignment_detected_with_ssa(self):
        result = analyse("proc foo {} { set x 1; set x 2; puts $x }")
        dead = [d for d in result.diagnostics if d.code == "W220"]
        assert len(dead) == 1
        assert "x" in dead[0].message

    def test_possible_paste_error_for_duplicate_static_assignment(self):
        result = analyse("proc foo {} { set x 0; set x 0; puts $x }")
        possible = [d for d in result.diagnostics if d.code == "H300"]
        assert len(possible) == 1
        assert possible[0].severity == Severity.HINT
        assert "x" in possible[0].message

    def test_possible_paste_error_not_emitted_when_value_changes(self):
        result = analyse("proc foo {} { set x 0; set x 1; puts $x }")
        possible = [d for d in result.diagnostics if d.code == "H300"]
        assert len(possible) == 0

    def test_possible_paste_error_not_emitted_for_dynamic_value(self):
        result = analyse("proc foo {} { set x $y; set x $y; puts $x }")
        possible = [d for d in result.diagnostics if d.code == "H300"]
        assert len(possible) == 0

    def test_constant_if_unreachable_branch(self):
        result = analyse("if {1} { set x 1 } else { set y 2 }")
        unreachable = [d for d in result.diagnostics if d.code == "I230"]
        assert len(unreachable) >= 1

    def test_constant_switch_unreachable_arm(self):
        source = "switch 1 {1 {set x 1} 2 {set y 2} default {set z 3}}"
        result = analyse(source)
        unreachable = [d for d in result.diagnostics if d.code == "I231"]
        assert len(unreachable) >= 1


class TestControlFlow:
    def test_if_body_analysed(self):
        source = "if {1} { set x 42 }"
        result = analyse(source)
        assert "x" in result.global_scope.variables

    def test_while_body_analysed(self):
        source = "while {1} { set x 42; break }"
        result = analyse(source)
        assert "x" in result.global_scope.variables

    def test_for_body_analysed(self):
        source = "for {set i 0} {$i < 10} {incr i} { set x $i }"
        result = analyse(source)
        assert "i" in result.global_scope.variables
        assert "x" in result.global_scope.variables

    def test_if_expr_records_var_reference(self):
        source = "set x 1\nif {$x > 0} { set y 1 }"
        result = analyse(source)
        assert "x" in result.global_scope.variables
        assert len(result.global_scope.variables["x"].references) >= 1

    def test_foreach_defines_var(self):
        source = "foreach item {a b c} { puts $item }"
        result = analyse(source)
        assert "item" in result.global_scope.variables

    def test_foreach_multi_var_individual_ranges(self):
        source = "foreach {a b c} {1 2 3} { puts $a }"
        result = analyse(source)
        assert "a" in result.global_scope.variables
        assert "b" in result.global_scope.variables
        assert "c" in result.global_scope.variables
        # Each variable should have its own distinct range, not all sharing
        # the range of the entire varList token.
        ra = result.global_scope.variables["a"].definition_range
        rb = result.global_scope.variables["b"].definition_range
        rc = result.global_scope.variables["c"].definition_range
        assert ra.start.character != rb.start.character
        assert rb.start.character != rc.start.character
        # Verify each range covers just the variable name (1 char each)
        assert ra.start.character == ra.end.character
        assert rb.start.character == rb.end.character
        assert rc.start.character == rc.end.character

    def test_nested_proc_and_if(self):
        source = textwrap.dedent("""\
            proc check {x} {
                if {== $x 0} {
                    set result zero
                } else {
                    set result nonzero
                }
                return $result
            }
        """)
        result = analyse(source)
        assert "check" in result.global_scope.procs

    def test_if_elseif_else_all_bodies_analysed(self):
        source = "if {$x} { set a 1 } elseif {$y} { set b 2 } else { set c 3 }"
        result = analyse(source)
        assert "a" in result.global_scope.variables
        assert "b" in result.global_scope.variables
        assert "c" in result.global_scope.variables

    def test_dict_for_body_analysed(self):
        source = "dict for {k v} $d { set seen 1 }"
        result = analyse(source)
        assert "seen" in result.global_scope.variables

    def test_command_subst_inside_expr_analysed(self):
        source = "set n [expr {[set y 1] + 2}]"
        result = analyse(source)
        assert "y" in result.global_scope.variables

    def test_tcloo_method_body_analysed(self):
        source = textwrap.dedent("""\
            oo::class create Dog {
                method bark {} {
                    set message woof
                }
            }
        """)
        result = analyse(source)
        assert "message" in result.global_scope.variables


class TestFixtures:
    @pytest.fixture
    def fixtures_dir(self):
        return Path(__file__).parent / "fixtures"

    def test_simple_tcl(self, fixtures_dir):
        source = (fixtures_dir / "simple.tcl").read_text()
        result = analyse(source)
        assert "greeting" in result.global_scope.variables

    def test_procs_tcl(self, fixtures_dir):
        source = (fixtures_dir / "procs.tcl").read_text()
        result = analyse(source)
        assert "fib" in result.global_scope.procs
        assert "factorial" in result.global_scope.procs

    def test_namespaces_tcl(self, fixtures_dir):
        source = (fixtures_dir / "namespaces.tcl").read_text()
        result = analyse(source)
        # Should find the math namespace
        ns_children = [c for c in result.global_scope.children if c.kind == "namespace"]
        assert any("math" in c.name for c in ns_children)


class TestRegexPatterns:
    """Tests for regex_patterns on AnalysisResult."""

    def test_regexp_simple(self):
        result = analyse("regexp {^[a-z]+$} $str")
        assert len(result.regex_patterns) == 1
        rp = result.regex_patterns[0]
        assert rp.pattern == "^[a-z]+$"
        assert rp.command == "regexp"

    def test_regexp_with_options(self):
        result = analyse("regexp -nocase -expanded {\\d+} $str")
        assert len(result.regex_patterns) == 1
        rp = result.regex_patterns[0]
        assert rp.pattern == "\\d+"
        assert rp.command == "regexp"

    def test_regexp_with_option_terminator(self):
        result = analyse("regexp -nocase -- {^test} $str")
        assert len(result.regex_patterns) == 1
        assert result.regex_patterns[0].pattern == "^test"

    def test_regexp_with_start_option(self):
        result = analyse("regexp -start 5 {pattern} $str")
        assert len(result.regex_patterns) == 1
        assert result.regex_patterns[0].pattern == "pattern"

    def test_regsub_simple(self):
        result = analyse("regsub {\\d+} $str replacement result")
        assert len(result.regex_patterns) == 1
        rp = result.regex_patterns[0]
        assert rp.pattern == "\\d+"
        assert rp.command == "regsub"

    def test_regsub_with_options(self):
        result = analyse("regsub -all -nocase {foo} $str bar result")
        assert len(result.regex_patterns) == 1
        assert result.regex_patterns[0].pattern == "foo"

    def test_switch_regexp_braced(self):
        source = "switch -regexp $x { {^a} {puts a} {^b} {puts b} }"
        result = analyse(source)
        assert len(result.regex_patterns) == 2
        patterns = [rp.pattern for rp in result.regex_patterns]
        assert "^a" in patterns
        assert "^b" in patterns
        assert all(rp.command == "switch" for rp in result.regex_patterns)

    def test_switch_regexp_inline(self):
        source = "switch -regexp $x {^a} {puts a} {^b} {puts b}"
        result = analyse(source)
        assert len(result.regex_patterns) == 2
        patterns = [rp.pattern for rp in result.regex_patterns]
        assert "^a" in patterns
        assert "^b" in patterns

    def test_switch_regexp_default_excluded(self):
        source = "switch -regexp $x { {^a} {puts a} default {puts other} }"
        result = analyse(source)
        assert len(result.regex_patterns) == 1
        assert result.regex_patterns[0].pattern == "^a"

    def test_switch_glob_no_regex(self):
        """switch -glob should NOT record regex patterns."""
        result = analyse("switch -glob $x { a* {puts a} b* {puts b} }")
        assert len(result.regex_patterns) == 0

    def test_switch_exact_no_regex(self):
        """switch -exact should NOT record regex patterns."""
        result = analyse("switch -exact $x { hello {puts hi} }")
        assert len(result.regex_patterns) == 0

    def test_switch_default_no_regex(self):
        """Plain switch (default glob mode) should NOT record regex patterns."""
        result = analyse("switch $x { hello {puts hi} }")
        assert len(result.regex_patterns) == 0

    def test_no_regex_in_other_commands(self):
        result = analyse("set x 1\nputs hello\nif {$x > 0} { puts yes }")
        assert len(result.regex_patterns) == 0

    def test_multiple_regexp_in_same_file(self):
        source = "regexp {^a} $x\nregsub {^b} $y c result"
        result = analyse(source)
        assert len(result.regex_patterns) == 2
        commands = {rp.command for rp in result.regex_patterns}
        assert commands == {"regexp", "regsub"}

    def test_regexp_inside_proc(self):
        source = "proc check {s} { regexp {^\\d+$} $s }"
        result = analyse(source)
        assert len(result.regex_patterns) == 1
        assert result.regex_patterns[0].command == "regexp"

    def test_range_points_at_pattern(self):
        source = "regexp {^hello} $str"
        result = analyse(source)
        rp = result.regex_patterns[0]
        # Pattern starts after "regexp " (col 7)
        assert rp.range.start.character == 7


class TestRegexVariablePropagation:
    """Tests for tracking regex patterns through variable assignments."""

    def test_set_then_regexp(self):
        """set pat {regex}; regexp $pat $str — should find 2 regex_patterns."""
        source = "set pat {^\\d+$}\nregexp $pat $str"
        result = analyse(source)
        # One for the $pat usage in regexp, one for the set definition site
        assert len(result.regex_patterns) == 2
        patterns = [rp.pattern for rp in result.regex_patterns]
        assert all(p == "^\\d+$" for p in patterns)

    def test_set_then_regsub(self):
        source = "set pat {foo}\nregsub $pat $str bar result"
        result = analyse(source)
        assert len(result.regex_patterns) == 2
        assert all(rp.pattern == "foo" for rp in result.regex_patterns)

    def test_set_then_switch_regexp(self):
        source = "set pat {^hello}\nswitch -regexp $x $pat {puts matched}"
        result = analyse(source)
        assert any(rp.pattern == "^hello" for rp in result.regex_patterns)

    def test_variable_not_constant_no_regex(self):
        """When the variable's value is dynamic, don't record regex."""
        source = "set pat $dynamic_value\nregexp $pat $str"
        result = analyse(source)
        # Only direct patterns are recorded, not dynamic ones
        assert len(result.regex_patterns) == 0

    def test_variable_with_interpolation_no_regex(self):
        """set pat "^$prefix"; regexp $pat $str — not constant."""
        source = 'set pat "^$prefix"\nregexp $pat $str'
        result = analyse(source)
        assert len(result.regex_patterns) == 0

    def test_variable_reassigned_dynamic(self):
        """Variable reassigned to non-constant loses const tracking."""
        source = "set pat {^\\d+}\nset pat $other\nregexp $pat $str"
        result = analyse(source)
        # pat was reassigned dynamically, so $pat in regexp is not a regex
        assert len(result.regex_patterns) == 0

    def test_variable_in_proc_scope(self):
        source = "proc check {s} { set pat {^\\w+$}; regexp $pat $s }"
        result = analyse(source)
        assert len(result.regex_patterns) == 2
        assert all(rp.pattern == "^\\w+$" for rp in result.regex_patterns)

    def test_literal_and_variable_patterns_mixed(self):
        """File with both literal and variable patterns."""
        source = "set pat {^a}\nregexp $pat $str\nregexp {^b} $str2"
        result = analyse(source)
        patterns = sorted(rp.pattern for rp in result.regex_patterns)
        assert "^a" in patterns
        assert "^b" in patterns

    def test_definition_site_marked_as_regex(self):
        """The 'set pat {regex}' definition site should itself be a regex_pattern."""
        source = "set pat {^test}\nregexp $pat $str"
        result = analyse(source)
        # Find the regex pattern that points at the definition site (col 0 for "set pat")
        # The definition range should point at where the var was defined (the var name token)
        def_patterns = [rp for rp in result.regex_patterns if rp.range.start.line == 0]
        assert len(def_patterns) == 1
        assert def_patterns[0].pattern == "^test"

    def test_use_site_marked_as_regex(self):
        """The '$pat' usage site should be a regex_pattern."""
        source = "set pat {^test}\nregexp $pat $str"
        result = analyse(source)
        use_patterns = [rp for rp in result.regex_patterns if rp.range.start.line == 1]
        assert len(use_patterns) == 1
        assert use_patterns[0].pattern == "^test"
        assert use_patterns[0].command == "regexp"


class TestPackageRequire:
    def test_simple_require(self):
        result = analyse("package require http")
        assert len(result.package_requires) == 1
        assert result.package_requires[0].name == "http"
        assert result.package_requires[0].version is None

    def test_require_with_version(self):
        result = analyse("package require http 2.9")
        assert len(result.package_requires) == 1
        assert result.package_requires[0].name == "http"
        assert result.package_requires[0].version == "2.9"

    def test_require_exact(self):
        result = analyse("package require -exact http 2.9")
        assert len(result.package_requires) == 1
        assert result.package_requires[0].name == "http"
        assert result.package_requires[0].version == "2.9"

    def test_multiple_requires(self):
        source = "package require http\npackage require tls"
        result = analyse(source)
        assert len(result.package_requires) == 2
        names = {pr.name for pr in result.package_requires}
        assert names == {"http", "tls"}

    def test_package_provide_not_recorded(self):
        result = analyse("package provide mylib 1.0")
        assert len(result.package_requires) == 0

    def test_no_package_commands(self):
        result = analyse("set x 42")
        assert len(result.package_requires) == 0


# W214
class TestUnusedProcParameters:
    def test_unused_param_detected(self):
        result = analyse("proc foo {x y} { puts $x }")
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 1
        assert "y" in w214[0].message
        assert w214[0].severity == Severity.HINT

    def test_all_params_used_no_warning(self):
        result = analyse("proc foo {a b} { puts $a; puts $b }")
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_args_param_not_flagged(self):
        result = analyse("proc foo {args} { puts hello }")
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_underscore_prefix_not_flagged(self):
        result = analyse("proc foo {_unused x} { puts $x }")
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_multiple_unused_params(self):
        result = analyse("proc foo {a b c} { puts hello }")
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 3

    def test_param_used_in_return(self):
        result = analyse("proc foo {x} { return $x }")
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0

    def test_param_used_in_branch_condition(self):
        result = analyse("proc foo {x} { if {$x > 0} { puts yes } }")
        w214 = [d for d in result.diagnostics if d.code == "W214"]
        assert len(w214) == 0
