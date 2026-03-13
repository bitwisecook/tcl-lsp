"""Tests for Tcl best-practice and security diagnostic checks."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.analyser import analyse
from core.analysis.semantic_model import Severity
from core.commands.registry.runtime import configure_signatures
from lsp.features.diagnostics import get_diagnostics


def _diag_with_code(source: str, code: str):
    """Return all diagnostics matching a specific code."""
    result = analyse(source)
    return [d for d in result.diagnostics if d.code == code]


# W100: Unbraced expr


class TestUnbracedExpr:
    """W100 -- expression arguments to expr/if/while/for should be braced."""

    def test_unbraced_expr_variable(self):
        diags = _diag_with_code("expr $x + 1", "W100")
        assert len(diags) == 1
        assert "not braced" in diags[0].message
        # Contains $x substitution → dangerous → ERROR severity
        assert diags[0].severity == Severity.ERROR

    def test_unbraced_expr_literal_warning(self):
        """Unbraced expr without substitution is WARNING (style only)."""
        # Quoted expression without substitution: style issue only.
        diags = _diag_with_code('if "1 > 0" {puts yes}', "W100")
        assert len(diags) == 1
        assert diags[0].severity == Severity.WARNING

    def test_braced_expr_clean(self):
        diags = _diag_with_code("expr {$x + 1}", "W100")
        assert len(diags) == 0

    def test_unbraced_if_condition(self):
        diags = _diag_with_code("if $x {puts yes}", "W100")
        assert len(diags) == 1

    def test_unbraced_if_fix_preserves_variable_marker(self):
        result = analyse("if $x {puts yes}")
        diags = [d for d in result.diagnostics if d.code == "W100"]
        assert len(diags) == 1
        assert len(diags[0].fixes) == 1
        assert diags[0].fixes[0].new_text == "{$x}"

    def test_braced_if_condition_clean(self):
        diags = _diag_with_code("if {$x > 0} {puts yes}", "W100")
        assert len(diags) == 0

    def test_unbraced_while_condition(self):
        diags = _diag_with_code("while $running {update}", "W100")
        assert len(diags) == 1

    def test_braced_while_clean(self):
        diags = _diag_with_code("while {$i < 10} {incr i}", "W100")
        assert len(diags) == 0

    def test_unbraced_for_test(self):
        diags = _diag_with_code("for {set i 0} $cond {incr i} {puts $i}", "W100")
        assert len(diags) == 1

    def test_braced_for_clean(self):
        diags = _diag_with_code("for {set i 0} {$i < 10} {incr i} {puts $i}", "W100")
        assert len(diags) == 0

    def test_numeric_literal_no_warning(self):
        """Pure numeric literals are safe unbraced."""
        diags = _diag_with_code("expr 42", "W100")
        assert len(diags) == 0

    def test_boolean_literal_no_warning(self):
        diags = _diag_with_code("if true {puts yes}", "W100")
        assert len(diags) == 0

    def test_has_fix(self):
        """W100 should have a code fix that wraps in braces."""
        diags = _diag_with_code("expr $x + 1", "W100")
        assert len(diags) == 1
        assert len(diags[0].fixes) == 1
        fix = diags[0].fixes[0]
        assert fix.new_text.startswith("{")
        assert fix.new_text.endswith("}")

    def test_expr_with_command_subst(self):
        """expr [clock seconds] should warn."""
        diags = _diag_with_code("expr [clock seconds]", "W100")
        assert len(diags) == 1

    def test_unbraced_expr_range_spans_full_expression(self):
        source = "set n [expr $n << 5]"
        diags = _diag_with_code(source, "W100")
        assert len(diags) == 1
        d = diags[0]
        highlighted = source[d.range.start.offset : d.range.end.offset + 1]
        assert highlighted == "$n << 5"

    def test_unbraced_expr_range_spans_complex_expression(self):
        source = "set n [expr $n + $b32_alphabet([string index $totp_secret $i])]"
        diags = _diag_with_code(source, "W100")
        assert len(diags) == 1
        d = diags[0]
        highlighted = source[d.range.start.offset : d.range.end.offset + 1]
        assert highlighted == "$n + $b32_alphabet([string index $totp_secret $i])"

    def test_unbraced_quoted_expr_fix_drops_outer_quotes(self):
        source = 'set ok [expr "$a == $b"]'
        result = analyse(source)
        diags = [d for d in result.diagnostics if d.code == "W100"]
        assert len(diags) == 1
        assert len(diags[0].fixes) == 1
        assert diags[0].fixes[0].new_text == "{$a == $b}"

    def test_unbraced_expr_inside_when_switch_body(self):
        """W100 should still fire for commands nested in when/switch arm bodies."""
        configure_signatures(dialect="f5-irules")
        source = """
when ACCESS_POLICY_AGENT_EVENT {
    switch x {
        "a" {
            set n [expr $n << 5]
        }
    }
}
"""
        diags = _diag_with_code(source, "W100")
        assert len(diags) == 1


# W101: eval with string concatenation


class TestEvalInjection:
    """W101 -- eval with substituted arguments risks injection."""

    def test_eval_with_variable(self):
        diags = _diag_with_code('eval "puts $x"', "W101")
        assert len(diags) == 1
        assert "injection" in diags[0].message.lower()

    def test_eval_braced_script_clean(self):
        diags = _diag_with_code("eval {puts hello}", "W101")
        assert len(diags) == 0

    def test_eval_multiple_braced_clean(self):
        diags = _diag_with_code("eval {set x 1} {puts $x}", "W101")
        assert len(diags) == 0

    def test_eval_with_command_subst(self):
        diags = _diag_with_code("eval [build_cmd]", "W101")
        assert len(diags) == 1

    def test_eval_literal_no_substitution_clean(self):
        """eval with a literal unbraced string but no variable/cmd substitution."""
        diags = _diag_with_code("eval puts hello", "W101")
        assert len(diags) == 0


# W102: subst on variable input


class TestSubstInjection:
    """W102 -- subst with a variable argument enables code injection."""

    def test_subst_variable(self):
        diags = _diag_with_code("subst $template", "W102")
        assert len(diags) == 1
        assert "injection" in diags[0].message.lower()

    def test_subst_braced_clean(self):
        diags = _diag_with_code("subst {hello $name}", "W102")
        assert len(diags) == 0

    def test_subst_with_flags_variable(self):
        """subst -nobackslashes $template should still warn."""
        diags = _diag_with_code("subst -nobackslashes $template", "W102")
        assert len(diags) == 1

    def test_subst_literal_clean(self):
        diags = _diag_with_code('subst "hello world"', "W102")
        assert len(diags) == 0

    def test_subst_variable_with_nocommands(self):
        """subst -nocommands $template still warns (novariables missing)."""
        diags = _diag_with_code("subst -nocommands $template", "W102")
        assert len(diags) == 1
        assert "-novariables" in diags[0].message

    def test_subst_variable_with_nocommands_novariables_clean(self):
        """subst -nocommands -novariables $template is safe — only backslash left."""
        diags = _diag_with_code("subst -nocommands -novariables $template", "W102")
        assert len(diags) == 0

    def test_subst_variable_suggests_nocommands(self):
        """W102 message should suggest -nocommands when not present."""
        diags = _diag_with_code("subst $template", "W102")
        assert len(diags) == 1
        assert "-nocommands" in diags[0].message


# W103: open pipeline


class TestOpenPipeline:
    """W103 -- open with pipeline or variable argument."""

    def test_open_literal_pipeline_hint(self):
        diags = _diag_with_code('open "|ls -la"', "W103")
        assert len(diags) == 1
        assert diags[0].severity == Severity.HINT

    def test_open_pipeline_with_variable(self):
        diags = _diag_with_code('open "|$cmd"', "W103")
        assert len(diags) == 1
        assert diags[0].severity == Severity.WARNING

    def test_open_variable_arg(self):
        diags = _diag_with_code("open $filename", "W103")
        assert len(diags) == 1
        assert diags[0].severity == Severity.WARNING

    def test_open_literal_file_clean(self):
        diags = _diag_with_code('open "data.txt"', "W103")
        assert len(diags) == 0

    def test_open_braced_file_clean(self):
        diags = _diag_with_code("open {data.txt}", "W103")
        assert len(diags) == 0


# W104: String/list confusion (append with spaces)


class TestStringListConfusion:
    """W104 -- append with space-prefixed values looks like list building."""

    def test_append_with_leading_space(self):
        diags = _diag_with_code('append items " item"', "W104")
        assert len(diags) == 1
        assert "lappend" in diags[0].message.lower()

    def test_append_normal_clean(self):
        diags = _diag_with_code('append result "value"', "W104")
        assert len(diags) == 0

    def test_lappend_clean(self):
        """lappend should not trigger this check."""
        diags = _diag_with_code("lappend items item", "W104")
        assert len(diags) == 0


# W110: String comparison with == in expr


class TestStringCompareInExpr:
    """W110 -- use eq/ne instead of ==/!= for string comparison in expr."""

    def test_string_compare_with_equals(self):
        diags = _diag_with_code('if {$x == "foo"} {puts yes}', "W110")
        assert len(diags) == 1
        assert "eq" in diags[0].message

    def test_string_compare_with_not_equals(self):
        diags = _diag_with_code('if {$x != "bar"} {puts no}', "W110")
        assert len(diags) == 1
        assert "ne" in diags[0].message

    def test_numeric_compare_clean(self):
        diags = _diag_with_code("if {$x == 42} {puts yes}", "W110")
        assert len(diags) == 0

    def test_eq_operator_clean(self):
        diags = _diag_with_code('if {$x eq "foo"} {puts yes}', "W110")
        assert len(diags) == 0

    def test_string_compare_has_fix(self):
        diags = _diag_with_code('if {$x == "foo"} {puts yes}', "W110")
        assert len(diags) == 1
        assert len(diags[0].fixes) == 1
        assert "eq" in diags[0].fixes[0].new_text

    def test_variable_only_compare_clean(self):
        """Variables compared with == should not trigger W110 -- they may be ints."""
        diags = _diag_with_code("if {$a == $b} {puts yes}", "W110")
        assert len(diags) == 0

    def test_unbraced_variable_compare_clean(self):
        """Unbraced expr with variables only -- outer quotes should not trigger."""
        diags = _diag_with_code('set r [expr "$a == $b"]', "W110")
        assert len(diags) == 0

    def test_numeric_string_compare_warns(self):
        """Numeric string literals still trigger -- user wrote a string literal."""
        diags = _diag_with_code('if {$x == "42"} {puts yes}', "W110")
        assert len(diags) == 1

    def test_boolean_string_compare_warns(self):
        """Boolean-like string literals still trigger -- user wrote a string literal."""
        diags = _diag_with_code('if {$x == "true"} {puts yes}', "W110")
        assert len(diags) == 1

    def test_negated_string_compare_warns(self):
        """W110 fires through ExprUnary (negation wrapping a string ==)."""
        diags = _diag_with_code('if {!($x == "foo")} {puts no}', "W110")
        assert len(diags) == 1

    def test_mixed_ops_no_code_fix(self):
        """When some ==/ != have no string literal, skip the blanket code fix."""
        diags = _diag_with_code('if {$a == $b || $x == "foo"} {puts y}', "W110")
        assert len(diags) == 1
        assert len(diags[0].fixes) == 0


# W201: Path concatenation


class TestPathConcatenation:
    """W201 -- manual path concatenation instead of file join."""

    def test_set_with_path_and_var(self):
        diags = _diag_with_code('set path "$dir/file.txt"', "W201")
        assert len(diags) == 1
        assert "file join" in diags[0].message.lower()

    def test_set_literal_path_clean(self):
        diags = _diag_with_code('set path "/etc/config"', "W201")
        assert len(diags) == 0

    def test_set_no_path_clean(self):
        diags = _diag_with_code("set x 42", "W201")
        assert len(diags) == 0

    def test_path_concatenation_has_fix_for_simple_case(self):
        diags = _diag_with_code('set path "$dir/file.txt"', "W201")
        assert len(diags) == 1
        assert len(diags[0].fixes) == 1
        assert diags[0].fixes[0].new_text == "[file join $dir file.txt]"


# W300: source with variable path


class TestSourceVariable:
    """W300 -- source with a variable path is a code execution vector."""

    def test_source_variable(self):
        diags = _diag_with_code("source $script_path", "W300")
        assert len(diags) == 1
        assert diags[0].severity == Severity.WARNING

    def test_source_literal_clean(self):
        diags = _diag_with_code('source "lib/utils.tcl"', "W300")
        assert len(diags) == 0

    def test_source_braced_clean(self):
        diags = _diag_with_code("source {lib/utils.tcl}", "W300")
        assert len(diags) == 0

    def test_source_encoding_flag_variable(self):
        """source -encoding utf-8 $path should still warn."""
        diags = _diag_with_code("source -encoding utf-8 $path", "W300")
        assert len(diags) == 1


# W301: uplevel injection


class TestUplevelInjection:
    """W301 -- uplevel with unbraced or multi-arg scripts."""

    def test_uplevel_unbraced_with_variable(self):
        diags = _diag_with_code('uplevel 1 "set x $y"', "W301")
        assert len(diags) == 1

    def test_uplevel_braced_clean(self):
        diags = _diag_with_code("uplevel 1 {set x 1}", "W301")
        assert len(diags) == 0

    def test_uplevel_multiple_args_with_variable(self):
        diags = _diag_with_code("uplevel 1 set x $y", "W301")
        assert len(diags) == 1
        assert "concatenate" in diags[0].message.lower()

    def test_uplevel_no_level_unbraced(self):
        diags = _diag_with_code('uplevel "set x $y"', "W301")
        assert len(diags) == 1


# W302: catch without result variable


class TestCatchIgnore:
    """W302 -- catch without a result variable silently swallows errors."""

    def test_catch_no_result(self):
        diags = _diag_with_code("catch {error oops}", "W302")
        assert len(diags) == 1
        assert diags[0].severity == Severity.HINT

    def test_catch_with_result_clean(self):
        diags = _diag_with_code("catch {error oops} result", "W302")
        assert len(diags) == 0

    def test_catch_with_options_clean(self):
        diags = _diag_with_code("catch {error oops} result opts", "W302")
        assert len(diags) == 0


# W210: catch body variables visible after condition


class TestCatchBodyDefsInCondition:
    """Variables set inside a catch body in a condition should be visible."""

    def test_catch_body_set_no_w210(self):
        """``if {![catch {set x [foo]}]} { puts $x }`` — no false positive."""
        diags = _diag_with_code("if {![catch {set x 1}]} { puts $x }", "W210")
        assert len(diags) == 0

    def test_catch_body_gets_no_w210(self):
        """``gets`` inside a catch body also defines its variable."""
        diags = _diag_with_code("if {![catch {gets stdin line}]} { puts $line }", "W210")
        assert len(diags) == 0

    def test_genuine_read_before_set_still_warns(self):
        """Unrelated variables still trigger W210."""
        diags = _diag_with_code("puts $never_set", "W210")
        assert len(diags) == 1

    def test_short_circuit_and_false_lhs_skips_rhs_defs(self):
        """``0 && ![catch {set x 1}]`` — catch never executes."""
        src = "proc t {} {\n  if {0 && ![catch {set x 1}]} { puts $x }\n  puts $x\n}"
        diags = _diag_with_code(src, "W210")
        assert len(diags) >= 1

    def test_short_circuit_or_true_lhs_skips_rhs_defs(self):
        """``1 || ![catch {set x 1}]`` — catch never executes."""
        src = "proc t {} {\n  if {1 || ![catch {set x 1}]} { puts $x }\n}"
        diags = _diag_with_code(src, "W210")
        assert len(diags) >= 1

    def test_short_circuit_and_true_lhs_keeps_rhs_defs(self):
        """``1 && ![catch {set x 1}]`` — catch executes normally."""
        diags = _diag_with_code("if {1 && ![catch {set x 1}]} { puts $x }", "W210")
        assert len(diags) == 0


# W303: ReDoS


class TestReDoS:
    """W303 -- suspicious regex patterns vulnerable to catastrophic backtracking."""

    def test_nested_quantifier(self):
        diags = _diag_with_code("regexp {(a+)+} $input", "W303")
        assert len(diags) == 1
        assert "backtracking" in diags[0].message.lower()

    def test_nested_star(self):
        diags = _diag_with_code("regexp {(a*)*} $input", "W303")
        assert len(diags) == 1

    def test_overlapping_alternation(self):
        diags = _diag_with_code("regexp {(a|a)+} $input", "W303")
        assert len(diags) == 1

    def test_safe_regex_clean(self):
        diags = _diag_with_code("regexp {^\\d+$} $input", "W303")
        assert len(diags) == 0

    def test_simple_quantifier_clean(self):
        diags = _diag_with_code("regexp {a+b+} $input", "W303")
        assert len(diags) == 0

    def test_with_options(self):
        diags = _diag_with_code("regexp -nocase {(a+)+} $input", "W303")
        assert len(diags) == 1

    def test_regsub_nested_quantifier(self):
        diags = _diag_with_code("regsub {(a+)+} $input replacement result", "W303")
        assert len(diags) == 1
        assert "backtracking" in diags[0].message.lower()

    def test_regsub_safe(self):
        diags = _diag_with_code("regsub {\\d+} $input X result", "W303")
        assert len(diags) == 0

    def test_regsub_with_options(self):
        diags = _diag_with_code("regsub -all -nocase {(a*)*} $input X result", "W303")
        assert len(diags) == 1

    def test_switch_regexp_redos(self):
        diags = _diag_with_code(
            "switch -regexp $x { {(a+)+} {puts bad} {^ok} {puts ok} }",
            "W303",
        )
        assert len(diags) == 1

    def test_switch_regexp_safe(self):
        diags = _diag_with_code(
            "switch -regexp $x { {^hello} {puts hi} {^world} {puts world} }",
            "W303",
        )
        assert len(diags) == 0

    def test_switch_glob_no_redos_check(self):
        """switch -glob patterns should NOT be checked for ReDoS."""
        diags = _diag_with_code(
            "switch -glob $x { (a+)+ {puts bad} }",
            "W303",
        )
        assert len(diags) == 0


# W304: Missing option terminator (--), option injection risk


class TestMissingOptionTerminator:
    """W304 -- option-bearing commands should use '--' before positional values."""

    def test_regexp_pattern_variable_without_terminator(self):
        diags = _diag_with_code("regexp $pattern $text", "W304")
        assert len(diags) == 1
        assert "option-injection" in diags[0].message.lower()

    def test_regexp_literal_pattern_without_terminator_warns(self):
        diags = _diag_with_code("regexp {(a+)+$} $text", "W304")
        assert len(diags) == 1

    def test_regexp_pattern_variable_with_terminator_clean(self):
        diags = _diag_with_code("regexp -- $pattern $text", "W304")
        assert len(diags) == 0

    def test_regexp_with_option_value_then_variable_warns(self):
        diags = _diag_with_code("regexp -start 0 $pattern $text", "W304")
        assert len(diags) == 1

    def test_regsub_variable_without_terminator(self):
        diags = _diag_with_code("regsub $pattern $text X out", "W304")
        assert len(diags) == 1

    def test_subst_not_checked_by_w304(self):
        diags = _diag_with_code("subst $template", "W304")
        assert len(diags) == 0

    def test_exec_variable_without_terminator(self):
        diags = _diag_with_code("exec $cmd", "W304")
        assert len(diags) == 1

    def test_exec_variable_with_terminator_clean(self):
        diags = _diag_with_code("exec -- $cmd", "W304")
        assert len(diags) == 0

    def test_glob_literal_pattern_clean(self):
        diags = _diag_with_code("glob *.tcl", "W304")
        assert len(diags) == 0

    def test_string_match_variable_pattern_without_terminator(self):
        diags = _diag_with_code("string match $pattern $value", "W304")
        assert len(diags) == 1

    def test_string_match_with_terminator_clean(self):
        diags = _diag_with_code("string match -- $pattern $value", "W304")
        assert len(diags) == 0

    def test_switch_static_variable_value_is_info(self):
        source = (
            'set totp_key_storage "datagroup"\n'
            "switch $totp_key_storage {\n"
            "    default { puts ok }\n"
            "}"
        )
        diags = _diag_with_code(source, "W304")
        assert len(diags) == 2

        switch_diag = next(d for d in diags if len(d.fixes) == 1)
        origin_diag = next(d for d in diags if len(d.fixes) == 0)

        assert switch_diag.severity == Severity.INFO
        assert (
            "reported at INFO because 'totp_key_storage' currently resolves" in switch_diag.message
        )
        highlighted = source[switch_diag.range.start.offset : switch_diag.range.end.offset + 1]
        assert highlighted == "$totp_key_storage"

        assert origin_diag.severity == Severity.INFO
        assert "currently assigned static literal 'datagroup' here" in origin_diag.message
        origin_highlighted = source[
            origin_diag.range.start.offset : origin_diag.range.end.offset + 1
        ]
        assert "datagroup" in origin_highlighted

    def test_w304_has_fix(self):
        diags = _diag_with_code("regexp $pattern $text", "W304")
        assert len(diags) == 1
        assert len(diags[0].fixes) == 1
        assert diags[0].fixes[0].new_text.startswith("-- ")


# Integration: multiple checks on realistic code


class TestIntegration:
    """Test multiple checks firing on realistic code patterns."""

    def test_multiple_issues_in_proc(self):
        source = """
proc dangerous {input} {
    set result [eval "process $input"]
    expr $result + 1
    catch {risky_op}
}
"""
        result = analyse(source)
        codes = {d.code for d in result.diagnostics}
        assert "W101" in codes  # eval injection
        assert "W100" in codes  # unbraced expr
        assert "W302" in codes  # catch no result

    def test_clean_proc_no_warnings(self):
        source = """
proc safe {x} {
    set result [expr {$x * 2}]
    if {$result > 10} {
        puts "big"
    }
    catch {risky_op} err
    return $result
}
"""
        result = analyse(source)
        # Filter to just W-codes (ignore E-codes from arity etc.)
        w_diags = [d for d in result.diagnostics if d.code.startswith("W")]
        assert len(w_diags) == 0

    def test_security_critical_patterns(self):
        """Multiple security issues in one script."""
        source = """
set script "doThing $userArg"
eval $script
source $untrusted_path
subst $user_template
open "|$user_cmd"
"""
        result = analyse(source)
        codes = {d.code for d in result.diagnostics}
        assert "W101" in codes  # eval
        assert "W300" in codes  # source
        assert "W102" in codes  # subst
        assert "W103" in codes  # open pipeline

    def test_no_false_positives_on_standard_patterns(self):
        """Common safe Tcl patterns should not trigger warnings."""
        source = """
set x 42
set y [expr {$x + 1}]
if {$y > 40} {
    puts "yes"
} else {
    puts "no"
}
while {$x > 0} {
    incr x -1
}
for {set i 0} {$i < 10} {incr i} {
    lappend result $i
}
foreach item {a b c} {
    puts $item
}
catch {risky} result opts
open {data.txt} r
source {lib/utils.tcl}
eval {set a 1}
"""
        result = analyse(source)
        w_diags = [d for d in result.diagnostics if d.code.startswith("W")]
        assert len(w_diags) == 0

    def test_nested_bodies_checked(self):
        """Checks work inside proc bodies and nested control flow."""
        source = """
proc test {} {
    if {1} {
        expr $x + 1
    }
}
"""
        result = analyse(source)
        assert any(d.code == "W100" for d in result.diagnostics)

    def test_switch_body_checked(self):
        """Checks work inside switch bodies."""
        source = """
switch $x {
    a {
        expr $y + 1
    }
}
"""
        result = analyse(source)
        assert any(d.code == "W100" for d in result.diagnostics)


# LSP Diagnostics integration


class TestLSPDiagnosticIntegration:
    """Test that checks produce proper LSP diagnostics."""

    def test_check_diagnostics_in_lsp(self):
        """New checks appear in get_diagnostics output."""
        from lsprotocol import types

        result = get_diagnostics("expr $x + 1")
        w100 = [d for d in result if d.code == "W100"]
        assert len(w100) == 1
        # $x substitution → dangerous → Error severity
        assert w100[0].severity == types.DiagnosticSeverity.Error

    def test_hint_severity_in_lsp(self):
        from lsprotocol import types

        result = get_diagnostics("catch {error hi}")
        w302 = [d for d in result if d.code == "W302"]
        assert len(w302) == 1
        assert w302[0].severity == types.DiagnosticSeverity.Hint

    def test_multiple_check_codes_in_lsp(self):
        source = 'eval "process $x"\nexpr $y + 1'
        result = get_diagnostics(source)
        codes = {d.code for d in result}
        assert "W100" in codes
        assert "W101" in codes

    def test_diagnostic_has_source(self):
        result = get_diagnostics("expr $x + 1")
        for d in result:
            assert d.source == "tcl-lsp"

    def test_existing_arity_checks_still_work(self):
        """Ensure the new checks don't break existing arity checking."""
        from lsprotocol import types

        result = get_diagnostics("set")
        errors = [d for d in result if d.severity == types.DiagnosticSeverity.Error]
        assert len(errors) >= 1

    def test_clean_source_still_clean(self):
        result = get_diagnostics("set x [clock seconds]\nputs $x")
        assert len(result) == 0


# Code actions / quick fixes


class TestCodeActions:
    """Test that code fixes are generated and wired correctly."""

    def test_unbraced_expr_has_fix(self):
        result = analyse("expr $x + 1")
        diags = [d for d in result.diagnostics if d.code == "W100"]
        assert len(diags) == 1
        assert len(diags[0].fixes) == 1
        fix = diags[0].fixes[0]
        assert fix.new_text.startswith("{")
        assert fix.description == "Wrap expression in braces"

    def test_unbraced_expr_fix_wraps_full_expression(self):
        source = "set n [expr $n << 5]"
        result = analyse(source)
        diags = [d for d in result.diagnostics if d.code == "W100"]
        assert len(diags) == 1
        assert len(diags[0].fixes) == 1
        fix = diags[0].fixes[0]
        assert fix.new_text == "{$n << 5}"

    def test_fix_range_matches_diagnostic(self):
        result = analyse("expr $x + 1")
        diag = [d for d in result.diagnostics if d.code == "W100"][0]
        fix = diag.fixes[0]
        # Fix range should cover the same token as the diagnostic range
        assert fix.range.start.line == diag.range.start.line
        assert fix.range.start.character == diag.range.start.character

    def test_no_fix_for_eval(self):
        """eval injection has no auto-fix (requires manual refactoring)."""
        result = analyse('eval "process $x"')
        diags = [d for d in result.diagnostics if d.code == "W101"]
        assert len(diags) == 1
        assert len(diags[0].fixes) == 0

    def test_no_fix_for_subst(self):
        result = analyse("subst $template")
        diags = [d for d in result.diagnostics if d.code == "W102"]
        assert len(diags) == 1
        assert len(diags[0].fixes) == 0


# Edge cases


class TestEdgeCases:
    """Edge cases and boundary conditions."""

    def test_empty_source(self):
        result = analyse("")
        assert len(result.diagnostics) == 0

    def test_comments_only(self):
        result = analyse("# just a comment\n# another comment")
        assert len(result.diagnostics) == 0

    def test_expr_no_args(self):
        """expr with no args should not crash the check (arity handles it)."""
        result = analyse("expr")
        # Arity error only, no W100 crash
        codes = [d.code for d in result.diagnostics]
        assert "E002" in codes
        assert "W100" not in codes

    def test_eval_no_args(self):
        result = analyse("eval")
        codes = [d.code for d in result.diagnostics]
        assert "E002" in codes
        assert "W101" not in codes

    def test_deeply_nested_check(self):
        source = """
proc a {} {
    proc b {} {
        if {1} {
            while {1} {
                expr $deep
            }
        }
    }
}
"""
        result = analyse(source)
        assert any(d.code == "W100" for d in result.diagnostics)

    def test_command_substitution_checked(self):
        """Checks run inside [command substitution]."""
        source = "set x [expr $y + 1]"
        result = analyse(source)
        assert any(d.code == "W100" for d in result.diagnostics)

    def test_semicolon_separated(self):
        """Multiple commands separated by semicolons."""
        source = "expr $a + 1; eval $script"
        result = analyse(source)
        codes = {d.code for d in result.diagnostics}
        assert "W100" in codes
        assert "W101" in codes

    def test_backslash_continuation(self):
        source = "expr \\\n$x + 1"
        result = analyse(source)
        assert any(d.code == "W100" for d in result.diagnostics)


# W200: Binary format signed/unsigned modifier requires Tcl 8.5+


class TestBinaryFormatModifiers:
    """W200 -- signed/unsigned modifiers on binary format specifiers."""

    def test_binary_format_unsigned_modifier_warns_tcl84(self):
        configure_signatures(dialect="tcl8.4")
        diags = _diag_with_code("binary format su $val", "W200")
        assert len(diags) == 1
        assert "modifier" in diags[0].message
        assert diags[0].severity == Severity.WARNING

    def test_binary_format_unsigned_modifier_clean_tcl86(self):
        configure_signatures(dialect="tcl8.6")
        diags = _diag_with_code("binary format su $val", "W200")
        assert len(diags) == 0

    def test_binary_scan_unsigned_modifier_warns_tcl84(self):
        configure_signatures(dialect="tcl8.4")
        diags = _diag_with_code("binary scan $data su x", "W200")
        assert len(diags) == 1

    def test_binary_format_no_modifier_clean_tcl84(self):
        configure_signatures(dialect="tcl8.4")
        diags = _diag_with_code("binary format s $val", "W200")
        assert len(diags) == 0

    def test_binary_format_unsigned_modifier_warns_f5_irules(self):
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code("binary format su $val", "W200")
        assert len(diags) == 1

    def test_binary_format_unsigned_modifier_warns_f5_iapps(self):
        configure_signatures(dialect="f5-iapps")
        diags = _diag_with_code("binary format su $val", "W200")
        assert len(diags) == 1


# IRULE2002: Deprecated iRules command


class TestDeprecatedIRulesCommand:
    """IRULE2002 -- deprecated iRules commands should suggest modern replacements."""

    def test_client_addr_deprecated(self):
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code("client_addr", "IRULE2002")
        assert len(diags) == 1
        assert "deprecated" in diags[0].message.lower()
        assert "IP::client_addr" in diags[0].message
        assert diags[0].severity == Severity.WARNING

    def test_redirect_deprecated(self):
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code('redirect "http://example.com"', "IRULE2002")
        assert len(diags) == 1
        assert "HTTP::redirect" in diags[0].message

    def test_http_uri_deprecated(self):
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code("set uri [http_uri]", "IRULE2002")
        assert len(diags) == 1
        assert "HTTP::uri" in diags[0].message

    def test_non_deprecated_clean(self):
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code("set x 1", "IRULE2002")
        assert len(diags) == 0

    def test_deprecated_not_flagged_in_tcl86(self):
        configure_signatures(dialect="tcl8.6")
        diags = _diag_with_code("set x 1", "IRULE2002")
        assert len(diags) == 0


# IRULE2003: Unsafe iRules command


class TestUnsafeIRulesCommand:
    """IRULE2003 -- unsafe iRules commands should be flagged as errors."""

    def test_uplevel_unsafe(self):
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code("uplevel 1 {set x 1}", "IRULE2003")
        assert len(diags) == 1
        assert "unsafe" in diags[0].message.lower()
        assert diags[0].severity == Severity.ERROR

    def test_history_unsafe(self):
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code("history", "IRULE2003")
        assert len(diags) == 1
        assert diags[0].severity == Severity.ERROR

    def test_uplevel_not_flagged_in_tcl86(self):
        configure_signatures(dialect="tcl8.6")
        diags = _diag_with_code("uplevel 1 {set x 1}", "IRULE2003")
        assert len(diags) == 0


# IRULE3102: HTTP getters should use -normalized


class TestIRulesNormalizedHttpGetters:
    """IRULE3102 -- HTTP::path/uri/query getter usage should be normalized."""

    def test_http_path_getter_warns(self):
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code("set p [HTTP::path]", "IRULE3102")
        assert len(diags) == 1
        assert "HTTP::path -normalized" in diags[0].message
        assert diags[0].severity == Severity.WARNING

    def test_http_path_normalized_clean(self):
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code("set p [HTTP::path -normalized]", "IRULE3102")
        assert len(diags) == 0

    def test_http_uri_getter_warns(self):
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code("set u [HTTP::uri]", "IRULE3102")
        assert len(diags) == 1
        assert "HTTP::uri -normalized" in diags[0].message

    def test_http_uri_normalized_clean(self):
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code("set u [HTTP::uri -normalized]", "IRULE3102")
        assert len(diags) == 0

    def test_http_query_getter_warns(self):
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code("set q [HTTP::query]", "IRULE3102")
        assert len(diags) == 1
        assert "HTTP::query -normalized" in diags[0].message

    def test_setter_form_is_not_flagged(self):
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code("HTTP::path /lowercase/path", "IRULE3102")
        assert len(diags) == 0

    def test_not_checked_in_tcl86(self):
        configure_signatures(dialect="tcl8.6")
        diags = _diag_with_code("set p [HTTP::path]", "IRULE3102")
        assert len(diags) == 0


# W105: Unbraced code block


class TestUnbracedBody:
    """W105 -- code block arguments should be braced."""

    def test_if_unbraced_body_with_var(self):
        diags = _diag_with_code('if {1} "puts $x"', "W105")
        assert len(diags) == 1
        # Quoted body — lexer resolves $x so text-level substitution check
        # sees 'puts x' (no '$'), producing WARNING rather than ERROR.
        assert diags[0].severity == Severity.WARNING

    def test_if_braced_body_clean(self):
        diags = _diag_with_code("if {1} {puts hello}", "W105")
        assert len(diags) == 0

    def test_foreach_unbraced_body_with_cmd_subst(self):
        diags = _diag_with_code('foreach x {a b} "puts [list $x]"', "W105")
        assert len(diags) == 1
        assert diags[0].severity == Severity.ERROR


# W106: Dangerous unbraced switch body


class TestUnbracedSwitchBody:
    """W106 -- switch bodies should be braced."""

    def test_switch_alternating_unbraced_body_with_var(self):
        diags = _diag_with_code('switch -- $x a "puts $y" b {puts ok}', "W106")
        assert len(diags) == 1
        # Quoted body — lexer resolves $y so text-level substitution check
        # sees 'puts y' (no '$'), producing WARNING rather than ERROR.
        assert diags[0].severity == Severity.WARNING

    def test_switch_braced_body_clean(self):
        diags = _diag_with_code("switch -- $x { a {puts hi} b {puts ok} }", "W106")
        assert len(diags) == 0

    def test_switch_regexp_unbraced_body(self):
        diags = _diag_with_code('switch -regexp -- $x {^a} "puts matched"', "W106")
        assert len(diags) == 1
        assert "regexp" in diags[0].message.lower()
        assert diags[0].severity == Severity.ERROR


# IRULE1002: Unknown iRules event


class TestUnknownIRulesEvent:
    """IRULE1002 -- when with unknown event names."""

    def test_unknown_event(self):
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code("when FAKE_EVENT {puts hi}", "IRULE1002")
        assert len(diags) == 1
        assert "Unknown iRules event" in diags[0].message
        assert diags[0].severity == Severity.WARNING

    def test_known_event_clean(self):
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code("when HTTP_REQUEST {puts hi}", "IRULE1002")
        assert len(diags) == 0

    def test_not_checked_in_tcl86(self):
        configure_signatures(dialect="tcl8.6")
        diags = _diag_with_code("when FAKE_EVENT {puts hi}", "IRULE1002")
        assert len(diags) == 0


# W306: Literal-expected position contains substitution


class TestLiteralExpected:
    """W306 -- literal-expected positions should not contain substitution."""

    def test_regexp_pattern_with_var(self):
        diags = _diag_with_code("regexp -- $pattern $text", "W306")
        assert len(diags) == 1
        assert diags[0].severity == Severity.ERROR
        assert "Literal expected" in diags[0].message

    def test_regexp_braced_pattern_clean(self):
        diags = _diag_with_code("regexp -- {^foo} $text", "W306")
        assert len(diags) == 0

    def test_regexp_quoted_pattern_with_var(self):
        diags = _diag_with_code('regexp -- "$pattern" $text', "W306")
        assert len(diags) == 1
        assert "quotes" in diags[0].message.lower()

    def test_regsub_pattern_with_cmd_subst(self):
        diags = _diag_with_code("regsub -- [build_re] $text X out", "W306")
        assert len(diags) == 1

    def test_class_match_literal_name_clean(self):
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code("class match -- [HTTP::uri] ends_with {my_class}", "W306")
        assert len(diags) == 0


# W307: Non-literal command name


class TestNonLiteralCommand:
    """W307 -- command name is a variable or substitution."""

    def test_variable_command_name(self):
        diags = _diag_with_code("$cmd arg1 arg2", "W307")
        assert len(diags) == 1
        assert "Non-literal" in diags[0].message
        assert diags[0].severity == Severity.WARNING

    def test_literal_command_clean(self):
        diags = _diag_with_code("puts hello", "W307")
        assert len(diags) == 0


# W108: Non-ASCII token content


class TestNonAscii:
    """W108 -- tokens with non-standard ASCII characters."""

    def test_non_ascii_character(self):
        diags = _diag_with_code("set x \u00a9value", "W108")
        assert len(diags) == 1
        assert "ASCII" in diags[0].message
        assert diags[0].severity == Severity.WARNING

    def test_pure_ascii_clean(self):
        diags = _diag_with_code("set x hello", "W108")
        assert len(diags) == 0

    def test_tab_and_newline_allowed(self):
        diags = _diag_with_code("set x {hello\tworld}", "W108")
        assert len(diags) == 0


# W304: Missing option terminator for new commands (class, table, unset)


class TestOptionTerminatorNewCommands:
    """W304 -- option terminator for class, table, and unset."""

    def test_unset_variable_without_terminator(self):
        diags = _diag_with_code("unset $varname", "W304")
        assert len(diags) == 1

    def test_unset_with_terminator_clean(self):
        diags = _diag_with_code("unset -- $varname", "W304")
        assert len(diags) == 0

    def test_table_set_variable_without_terminator(self):
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code("table set $key $value", "W304")
        assert len(diags) == 1

    def test_class_match_variable_without_terminator(self):
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code("class match $item starts_with my_class", "W304")
        assert len(diags) == 1


# W308: subst without -nocommands


class TestSubstNocommands:
    """W308 -- subst without -nocommands when template has [cmd]."""

    def test_subst_braced_with_bracket(self):
        """subst {hello [clock seconds]} without -nocommands → HINT."""
        diags = _diag_with_code("subst {hello [clock seconds]}", "W308")
        assert len(diags) == 1
        assert diags[0].severity == Severity.HINT
        assert "-nocommands" in diags[0].message

    def test_subst_with_nocommands_clean(self):
        """subst -nocommands {hello [clock seconds]} → no W308."""
        diags = _diag_with_code("subst -nocommands {hello [clock seconds]}", "W308")
        assert len(diags) == 0

    def test_subst_no_brackets_clean(self):
        """subst {hello $name} without brackets → no W308."""
        diags = _diag_with_code("subst {hello $name}", "W308")
        assert len(diags) == 0

    def test_subst_variable_defers_to_w102(self):
        """subst $template → W102 fires instead, W308 should not."""
        diags = _diag_with_code("subst $template", "W308")
        assert len(diags) == 0


# W309: eval/uplevel with subst — double substitution


class TestEvalSubstDoubleSubstitution:
    """W309 -- eval [subst ...] creates double substitution risk."""

    def test_eval_subst_variable(self):
        """eval [subst $template] → ERROR."""
        diags = _diag_with_code("eval [subst $template]", "W309")
        assert len(diags) == 1
        assert diags[0].severity == Severity.ERROR
        assert "double substitution" in diags[0].message.lower()

    def test_eval_subst_braced(self):
        """eval [subst {set x [expr {$a + $b}]}] → ERROR."""
        diags = _diag_with_code("eval [subst {set x [expr {$a + $b}]}]", "W309")
        assert len(diags) == 1
        assert diags[0].severity == Severity.ERROR

    def test_uplevel_subst(self):
        """uplevel 1 [subst $template] → ERROR."""
        diags = _diag_with_code("uplevel 1 [subst $template]", "W309")
        assert len(diags) == 1
        assert diags[0].severity == Severity.ERROR

    def test_eval_no_subst_clean(self):
        """eval {puts hello} → no W309."""
        diags = _diag_with_code("eval {puts hello}", "W309")
        assert len(diags) == 0

    def test_eval_command_subst_not_subst(self):
        """eval [build_cmd] → no W309 (not subst)."""
        diags = _diag_with_code("eval [build_cmd]", "W309")
        assert len(diags) == 0


# W212: Variable substitution where a variable *name* is expected


class TestNameVsValue:
    """W212 — commands that take a variable name should not get $substitution."""

    # -- set --

    def test_set_dollar_var(self):
        diags = _diag_with_code("set $x 1", "W212")
        assert len(diags) == 1
        assert "$x" in diags[0].message

    def test_set_plain_clean(self):
        diags = _diag_with_code("set x 1", "W212")
        assert len(diags) == 0

    # -- incr --

    def test_incr_dollar_var(self):
        diags = _diag_with_code("incr $count", "W212")
        assert len(diags) == 1

    def test_incr_plain_clean(self):
        diags = _diag_with_code("incr count", "W212")
        assert len(diags) == 0

    # -- append --

    def test_append_dollar_var(self):
        diags = _diag_with_code('append $buf "text"', "W212")
        assert len(diags) == 1

    def test_append_plain_clean(self):
        diags = _diag_with_code('append buf "text"', "W212")
        assert len(diags) == 0

    # -- lappend --

    def test_lappend_dollar_var(self):
        diags = _diag_with_code("lappend $mylist item", "W212")
        assert len(diags) == 1

    def test_lappend_plain_clean(self):
        diags = _diag_with_code("lappend mylist item", "W212")
        assert len(diags) == 0

    # -- unset --

    def test_unset_dollar_var(self):
        diags = _diag_with_code("unset $x", "W212")
        assert len(diags) == 1

    def test_unset_nocomplain_dollar_var(self):
        diags = _diag_with_code("unset -nocomplain $x", "W212")
        assert len(diags) == 1

    def test_unset_terminator_dollar_var(self):
        diags = _diag_with_code("unset -nocomplain -- $x", "W212")
        assert len(diags) == 1

    def test_unset_multiple_dollar_vars(self):
        diags = _diag_with_code("unset $a $b", "W212")
        assert len(diags) == 2

    def test_unset_plain_clean(self):
        diags = _diag_with_code("unset x", "W212")
        assert len(diags) == 0

    # -- info exists --

    def test_info_exists_dollar_var(self):
        diags = _diag_with_code("info exists $x", "W212")
        assert len(diags) == 1
        assert "info exists" in diags[0].message

    def test_info_exists_plain_clean(self):
        diags = _diag_with_code("info exists x", "W212")
        assert len(diags) == 0

    def test_info_commands_no_warning(self):
        """Only 'exists' subcommand triggers — 'commands' should not."""
        diags = _diag_with_code("info commands $pattern", "W212")
        assert len(diags) == 0

    # -- upvar --

    def test_upvar_local_dollar_var(self):
        diags = _diag_with_code("upvar 1 other $local", "W212")
        assert len(diags) == 1

    def test_upvar_no_level_local_dollar_var(self):
        diags = _diag_with_code("upvar other $local", "W212")
        assert len(diags) == 1

    def test_upvar_remote_dollar_is_ok(self):
        """The remote var (otherVar) *can* legitimately be $var."""
        diags = _diag_with_code("upvar 1 $remote local", "W212")
        assert len(diags) == 0

    def test_upvar_multiple_pairs(self):
        diags = _diag_with_code("upvar 1 a $b c $d", "W212")
        assert len(diags) == 2

    def test_upvar_plain_clean(self):
        diags = _diag_with_code("upvar 1 other local", "W212")
        assert len(diags) == 0


# E100: Unmatched close bracket


class TestUnmatchedCloseBracket:
    """E100 -- unmatched ']' with optional CodeFix."""

    def test_bare_close_bracket_detected(self):
        diags = _diag_with_code("set x foo]", "E100")
        assert len(diags) == 1
        assert "']'" in diags[0].message

    def test_no_false_positive_in_cmd_substitution(self):
        """Properly matched [cmd] should not trigger E100."""
        diags = _diag_with_code("set x [string length foo]", "E100")
        assert len(diags) == 0

    def test_no_false_positive_in_braces(self):
        """Braced string containing ] should not trigger E100."""
        diags = _diag_with_code("set x {contains ]}", "E100")
        assert len(diags) == 0

    def test_codefix_inserts_bracket(self):
        """When a known command precedes ], the fix inserts [."""
        configure_signatures(dialect="f5-irules")
        # ACCESS::policy is a known iRules command
        diags = _diag_with_code(
            "switch -- ACCESS::policy result]",
            "E100",
        )
        assert len(diags) >= 1
        fixes = diags[0].fixes
        assert len(fixes) == 1
        assert fixes[0].new_text == "["

    def test_no_codefix_without_known_command(self):
        """No fix when no known command is found before ] and arity isn't exceeded."""
        diags = _diag_with_code("set x blah]", "E100")
        # Should still have the diagnostic but no fix
        assert len(diags) == 1
        assert len(diags[0].fixes) == 0

    def test_arity_based_codefix(self):
        """When arity overflows, insert [ before last expected arg position."""
        # set has arity (1, 2).  5 args exceed max 2 → [ before arg[1].
        diags = _diag_with_code(
            "set username unknown_proc arg1 arg2]",
            "E100",
        )
        assert len(diags) >= 1
        fixes = diags[0].fixes
        assert len(fixes) == 1
        assert fixes[0].new_text == "["
        # [ should be inserted at the start of "unknown_proc" (col 13).
        assert fixes[0].range.start.character == 13

    def test_arity_based_diagnostic_range(self):
        """Arity-based heuristic highlights the whole expression, not just ]."""
        diags = _diag_with_code(
            "set username unknown_proc arg1 arg2]",
            "E100",
        )
        assert len(diags) >= 1
        diag = diags[0]
        # Range should span from "unknown_proc" to "]"
        assert diag.range.start.character == 13  # start of unknown_proc
        assert diag.range.end.character == 35  # position of ]

    def test_known_command_diagnostic_range(self):
        """Known-command heuristic highlights from the insertion point to ]."""
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code(
            "switch -- ACCESS::policy result]",
            "E100",
        )
        assert len(diags) >= 1
        diag = diags[0]
        # Range should span from ACCESS::policy to ]
        assert diag.range.start.character == 10  # start of ACCESS::policy
        assert diag.range.end.character == 31  # position of ]

    def test_fallback_highlights_from_cmd_start(self):
        """When no heuristic matches, highlight from command start to ]."""
        diags = _diag_with_code("set x blah]", "E100")
        assert len(diags) == 1
        diag = diags[0]
        # Range from "set" (col 0) to "]" (col 10)
        assert diag.range.start.character == 0
        assert diag.range.end.character == 10


# W213: unset without -nocomplain on variable that may not exist


class TestUnsetNocomplain:
    """W213: unset without -nocomplain on potentially undefined variable."""

    def test_unset_undefined_var(self):
        """unset on a never-set variable → W213."""
        diags = _diag_with_code("unset x", "W213")
        assert len(diags) == 1
        assert "x" in diags[0].message
        assert "nocomplain" in diags[0].message

    def test_unset_nocomplain_undefined_var(self):
        """unset -nocomplain on undefined variable → no W213."""
        diags = _diag_with_code("unset -nocomplain x", "W213")
        assert len(diags) == 0

    def test_unset_defined_var(self):
        """unset on a previously set variable → no W213."""
        diags = _diag_with_code("set x 1\nunset x", "W213")
        assert len(diags) == 0

    def test_unset_nocomplain_terminator(self):
        """unset -nocomplain -- on undefined → no W213."""
        diags = _diag_with_code("unset -nocomplain -- x", "W213")
        assert len(diags) == 0


# Cross-event variable scoping (connection scope)


class TestCrossEventScope:
    """Cross-event variable scope: suppress false diagnostics for
    variables that flow across when-event boundaries."""

    @staticmethod
    def _diag_codes_with_cu(source: str) -> list[str]:
        from core.compiler.compilation_unit import compile_source

        cu = compile_source(source)
        configure_signatures(dialect="f5-irules")
        result = analyse(source, cu=cu)
        return [d.code for d in result.diagnostics]

    @staticmethod
    def _diags_with_cu(source: str, code: str):
        from core.compiler.compilation_unit import compile_source

        cu = compile_source(source)
        configure_signatures(dialect="f5-irules")
        result = analyse(source, cu=cu)
        return [d for d in result.diagnostics if d.code == code]

    def test_set_in_request_read_in_response_no_w210(self):
        """Variable set in HTTP_REQUEST, read in HTTP_RESPONSE → no W210."""
        src = (
            "when HTTP_REQUEST {\n"
            "    set uri [HTTP::uri]\n"
            "}\n"
            "when HTTP_RESPONSE {\n"
            '    log local0. "uri=$uri"\n'
            "}"
        )
        diags = self._diags_with_cu(src, "W210")
        uri_diags = [d for d in diags if "uri" in d.message]
        assert len(uri_diags) == 0

    def test_set_in_response_read_in_request_still_warns(self):
        """Variable set in HTTP_RESPONSE, read in HTTP_REQUEST (wrong order) → W210."""
        src = (
            "when HTTP_REQUEST {\n"
            '    log local0. "uri=$uri"\n'
            "}\n"
            "when HTTP_RESPONSE {\n"
            "    set uri [HTTP::uri]\n"
            "}"
        )
        diags = self._diags_with_cu(src, "W210")
        uri_diags = [d for d in diags if "uri" in d.message]
        assert len(uri_diags) == 1

    def test_set_in_request_no_false_w220(self):
        """Dead store in one event shouldn't fire if consumed by another event."""
        src = (
            "when HTTP_REQUEST {\n"
            "    set uri [HTTP::uri]\n"
            "}\n"
            "when HTTP_RESPONSE {\n"
            '    log local0. "uri=$uri"\n'
            "}"
        )
        diags = self._diags_with_cu(src, "W220")
        uri_diags = [d for d in diags if "uri" in d.message]
        assert len(uri_diags) == 0

    def test_set_in_request_no_false_w211(self):
        """Unused variable in one event shouldn't fire if consumed by another."""
        src = (
            "when HTTP_REQUEST {\n"
            "    set uri [HTTP::uri]\n"
            "}\n"
            "when HTTP_RESPONSE {\n"
            '    log local0. "uri=$uri"\n'
            "}"
        )
        diags = self._diags_with_cu(src, "W211")
        uri_diags = [d for d in diags if "uri" in d.message]
        assert len(uri_diags) == 0

    def test_info_exists_cross_event_no_false_w220(self):
        """set in DNS_REQUEST checked via [info exists] in DNS_RESPONSE → no W220."""
        src = (
            "when DNS_REQUEST {\n"
            '    if { [DNS::header opcode] ne "QUERY" } {\n'
            "        set ans_cleared 1\n"
            "        DNS::return\n"
            "        return\n"
            "    }\n"
            "}\n"
            "when DNS_RESPONSE {\n"
            "    if { [info exists ans_cleared] } {\n"
            "        unset -nocomplain -- ans_cleared\n"
            "        return\n"
            "    }\n"
            "}"
        )
        diags = self._diags_with_cu(src, "W220")
        cleared_diags = [d for d in diags if "ans_cleared" in d.message]
        assert len(cleared_diags) == 0

    def test_info_exists_cross_event_no_false_w211(self):
        """set in DNS_REQUEST checked via [info exists] in DNS_RESPONSE → no W211."""
        src = (
            "when DNS_REQUEST {\n"
            "    set ans_cleared 1\n"
            "    return\n"
            "}\n"
            "when DNS_RESPONSE {\n"
            "    if { [info exists ans_cleared] } {\n"
            "        unset -nocomplain -- ans_cleared\n"
            "        return\n"
            "    }\n"
            "}"
        )
        diags = self._diags_with_cu(src, "W211")
        cleared_diags = [d for d in diags if "ans_cleared" in d.message]
        assert len(cleared_diags) == 0


# W310: Hardcoded credentials


class TestHardcodedCredentials:
    """W310 -- literal secrets in password/auth arguments."""

    def test_password_option_literal(self):
        """Literal value after -password flag → W310."""
        diags = _diag_with_code('http::geturl $url -password "s3cret"', "W310")
        assert len(diags) == 1
        assert diags[0].severity == Severity.WARNING
        assert "credential" in diags[0].message.lower()

    def test_password_option_variable_clean(self):
        """Variable after -password flag → no W310 (value not hardcoded)."""
        diags = _diag_with_code("http::geturl $url -password $pw", "W310")
        assert len(diags) == 0

    def test_token_option(self):
        """-token with literal → W310."""
        diags = _diag_with_code('some_command -token "abc123def456"', "W310")
        assert len(diags) == 1

    def test_sensitive_header_insert(self):
        """HTTP::header insert Authorization with literal value → W310."""
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code(
            'HTTP::header insert Authorization "Bearer hardcoded"',
            "W310",
        )
        assert len(diags) == 1
        assert "authorization" in diags[0].message.lower()

    def test_sensitive_header_variable_clean(self):
        """HTTP::header insert Authorization with variable → no W310."""
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code(
            "HTTP::header insert Authorization $token",
            "W310",
        )
        assert len(diags) == 0

    def test_non_sensitive_header_clean(self):
        """HTTP::header insert Content-Type with literal → no W310."""
        configure_signatures(dialect="f5-irules")
        diags = _diag_with_code(
            'HTTP::header insert Content-Type "text/html"',
            "W310",
        )
        assert len(diags) == 0


# W311: Unsafe channel encoding mismatch


class TestEncodingMismatch:
    """W311 -- -encoding binary with conflicting -translation."""

    def test_binary_encoding_with_text_translation(self):
        """fconfigure with -encoding binary and -translation auto → W311."""
        diags = _diag_with_code(
            "fconfigure $fd -encoding binary -translation auto",
            "W311",
        )
        assert len(diags) == 1
        assert diags[0].severity == Severity.WARNING
        assert "binary" in diags[0].message.lower()

    def test_binary_encoding_binary_translation_clean(self):
        """fconfigure with -encoding binary -translation binary → no W311."""
        diags = _diag_with_code(
            "fconfigure $fd -encoding binary -translation binary",
            "W311",
        )
        assert len(diags) == 0

    def test_text_encoding_no_conflict_clean(self):
        """fconfigure with -encoding utf-8 -translation auto → no W311."""
        diags = _diag_with_code(
            "fconfigure $fd -encoding utf-8 -translation auto",
            "W311",
        )
        assert len(diags) == 0

    def test_chan_configure_mismatch(self):
        """chan configure with -encoding binary -translation crlf → W311."""
        diags = _diag_with_code(
            "chan configure $fd -encoding binary -translation crlf",
            "W311",
        )
        assert len(diags) == 1

    def test_encoding_only_clean(self):
        """fconfigure with only -encoding binary → no W311."""
        diags = _diag_with_code(
            "fconfigure $fd -encoding binary",
            "W311",
        )
        assert len(diags) == 0


# W312: interp eval / interp invokehidden injection


class TestInterpEvalInjection:
    """W312 -- interp eval with dynamic script arguments."""

    def test_interp_eval_unbraced_with_variable(self):
        """interp eval with unbraced script containing variable → W312."""
        diags = _diag_with_code('interp eval $child "set x $y"', "W312")
        assert len(diags) == 1
        assert diags[0].severity == Severity.WARNING

    def test_interp_eval_braced_clean(self):
        """interp eval with braced script → no W312."""
        diags = _diag_with_code("interp eval $child {set x 1}", "W312")
        assert len(diags) == 0

    def test_interp_eval_multiple_args(self):
        """interp eval with multiple args (concat behaviour) → W312."""
        diags = _diag_with_code("interp eval $child set x $y", "W312")
        assert len(diags) == 1
        assert "concatenate" in diags[0].message.lower()

    def test_interp_invokehidden_unbraced(self):
        """interp invokehidden with unbraced argument → W312."""
        diags = _diag_with_code('interp invokehidden $child "source $path"', "W312")
        assert len(diags) == 1

    def test_interp_aliases_clean(self):
        """interp aliases → no W312 (not eval/invokehidden)."""
        diags = _diag_with_code("interp aliases $child", "W312")
        assert len(diags) == 0


# W313: Destructive file operations with variable path


class TestDestructiveFileOps:
    """W313 -- file delete/rename/mkdir with variable path."""

    def test_file_delete_variable(self):
        """file delete with variable path → W313."""
        diags = _diag_with_code("file delete $path", "W313")
        assert len(diags) == 1
        assert diags[0].severity == Severity.WARNING
        assert "path-traversal" in diags[0].message.lower()

    def test_file_delete_literal_clean(self):
        """file delete with literal path → no W313."""
        diags = _diag_with_code("file delete /tmp/myfile.txt", "W313")
        assert len(diags) == 0

    def test_file_rename_variable(self):
        """file rename with variable source → W313."""
        diags = _diag_with_code("file rename $src /tmp/dest", "W313")
        assert len(diags) == 1

    def test_file_mkdir_variable(self):
        """file mkdir with variable path → W313."""
        diags = _diag_with_code("file mkdir $dir", "W313")
        assert len(diags) == 1

    def test_file_delete_force_variable(self):
        """file delete -force $path → W313 (skips -force flag)."""
        diags = _diag_with_code("file delete -force $path", "W313")
        assert len(diags) == 1

    def test_file_exists_clean(self):
        """file exists with variable → no W313 (not destructive)."""
        diags = _diag_with_code("file exists $path", "W313")
        assert len(diags) == 0

    def test_file_join_clean(self):
        """file join with variable → no W313 (not destructive)."""
        diags = _diag_with_code("file join $base subdir", "W313")
        assert len(diags) == 0


# W121: Invalid subnet mask


class TestInvalidSubnetMask:
    """W121 -- dotted-quad that looks like a subnet mask but has non-contiguous bits."""

    def test_invalid_mask_255_255_255_1(self):
        """255.255.255.1 has non-contiguous bits → W121."""
        diags = _diag_with_code("set mask 255.255.255.1", "W121")
        assert len(diags) == 1
        assert "non-contiguous" in diags[0].message

    def test_invalid_mask_255_0_255_0(self):
        """255.0.255.0 has non-contiguous bits → W121."""
        diags = _diag_with_code("set mask 255.0.255.0", "W121")
        assert len(diags) == 1

    def test_invalid_mask_255_255_0_128(self):
        """255.255.0.128 has non-contiguous bits → W121."""
        diags = _diag_with_code("set mask 255.255.0.128", "W121")
        assert len(diags) == 1

    def test_invalid_mask_255_255_253_0(self):
        """255.255.253.0 — 253 is not a valid mask octet but first octet is 255 → W121."""
        diags = _diag_with_code("set mask 255.255.253.0", "W121")
        assert len(diags) == 1
        assert "Did you mean" in diags[0].message

    def test_valid_mask_255_255_255_0_clean(self):
        """255.255.255.0 is a valid /24 mask → no W121."""
        diags = _diag_with_code("set mask 255.255.255.0", "W121")
        assert len(diags) == 0

    def test_valid_mask_255_255_254_0_clean(self):
        """255.255.254.0 is a valid /23 mask → no W121."""
        diags = _diag_with_code("set mask 255.255.254.0", "W121")
        assert len(diags) == 0

    def test_valid_mask_255_255_255_192_clean(self):
        """255.255.255.192 is a valid /26 mask → no W121."""
        diags = _diag_with_code("set mask 255.255.255.192", "W121")
        assert len(diags) == 0

    def test_valid_mask_255_255_255_255_clean(self):
        """255.255.255.255 is a valid /32 mask → no W121."""
        diags = _diag_with_code("set mask 255.255.255.255", "W121")
        assert len(diags) == 0

    def test_valid_mask_255_0_0_0_clean(self):
        """255.0.0.0 is a valid /8 mask → no W121."""
        diags = _diag_with_code("set mask 255.0.0.0", "W121")
        assert len(diags) == 0

    def test_regular_ip_no_false_positive(self):
        """10.0.0.1 is a regular IP, not a mask → no W121."""
        diags = _diag_with_code("set addr 10.0.0.1", "W121")
        assert len(diags) == 0

    def test_regular_ip_192_168_1_1_no_false_positive(self):
        """192.168.1.1 does not look like a mask → no W121."""
        diags = _diag_with_code("set addr 192.168.1.1", "W121")
        assert len(diags) == 0

    def test_invalid_mask_in_braced_string(self):
        """Invalid mask inside braces still triggers → W121."""
        diags = _diag_with_code('puts "mask is 255.255.255.1"', "W121")
        assert len(diags) == 1

    def test_suggestion_fix_attached(self):
        """W121 attaches a CodeFix suggesting the nearest valid mask."""
        diags = _diag_with_code("set mask 255.255.255.1", "W121")
        assert len(diags) == 1
        assert diags[0].fixes
        assert "255.255.255.0" in diags[0].fixes[0].new_text

    def test_zero_mask_clean(self):
        """0.0.0.0 is a valid /0 mask → no W121."""
        diags = _diag_with_code("set mask 0.0.0.0", "W121")
        assert len(diags) == 0

    def test_valid_mask_128_0_0_0_clean(self):
        """128.0.0.0 is a valid /1 mask → no W121."""
        diags = _diag_with_code("set mask 128.0.0.0", "W121")
        assert len(diags) == 0


# W122: Mistyped IPv4 address


class TestMistypedIPv4:
    """W122 -- dotted-quad with out-of-range octets or leading zeros."""

    def test_octet_over_255(self):
        """192.168.1.256 has octet > 255 → W122."""
        diags = _diag_with_code("set addr 192.168.1.256", "W122")
        assert len(diags) == 1
        assert "exceeds 255" in diags[0].message
        assert diags[0].severity == Severity.ERROR

    def test_leading_zero(self):
        """192.168.01.1 has leading zero → W122."""
        diags = _diag_with_code("set addr 192.168.01.1", "W122")
        assert len(diags) == 1
        assert "leading zero" in diags[0].message
        assert diags[0].severity == Severity.WARNING

    def test_valid_ip_clean(self):
        """192.168.1.1 is valid → no W122."""
        diags = _diag_with_code("set addr 192.168.1.1", "W122")
        assert len(diags) == 0

    def test_valid_ip_10_0_0_1_clean(self):
        """10.0.0.1 is valid → no W122."""
        diags = _diag_with_code("set addr 10.0.0.1", "W122")
        assert len(diags) == 0

    def test_octet_999(self):
        """10.0.0.999 has octet > 255 → W122."""
        diags = _diag_with_code("set addr 10.0.0.999", "W122")
        assert len(diags) == 1

    def test_all_zeros_clean(self):
        """0.0.0.0 is valid → no W122."""
        diags = _diag_with_code("set addr 0.0.0.0", "W122")
        assert len(diags) == 0

    def test_leading_zero_non_octal_digit(self):
        """192.168.09.1 has leading zero but '9' is not octal → no W122."""
        diags = _diag_with_code("set addr 192.168.09.1", "W122")
        assert len(diags) == 0

    def test_leading_zero_08_not_octal(self):
        """10.0.08.1 — '08' contains '8' which is not octal → no W122."""
        diags = _diag_with_code("set addr 10.0.08.1", "W122")
        assert len(diags) == 0

    def test_leading_zero_099_not_octal(self):
        """192.168.099.1 — '099' contains '9' → no W122."""
        diags = _diag_with_code("set addr 192.168.099.1", "W122")
        assert len(diags) == 0

    def test_leading_zero_octal_ambiguous(self):
        """192.168.077.1 — '077' is valid octal → W122."""
        diags = _diag_with_code("set addr 192.168.077.1", "W122")
        assert len(diags) == 1
        assert "leading zero" in diags[0].message

    def test_leading_zero_00_octal_ambiguous(self):
        """10.00.0.1 — '00' is valid octal → W122."""
        diags = _diag_with_code("set addr 10.00.0.1", "W122")
        assert len(diags) == 1
        assert "leading zero" in diags[0].message

    def test_valid_mask_clean(self):
        """255.255.255.0 is valid → no W122."""
        diags = _diag_with_code("set mask 255.255.255.0", "W122")
        assert len(diags) == 0
