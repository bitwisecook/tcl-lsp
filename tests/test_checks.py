"""Tests for Tcl best-practice and security diagnostic checks."""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from analyser import analyse
from analyser.semantic_model import Severity
from compiler.registry.runtime import configure_signatures
from server.features.diagnostics import get_diagnostics


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

    def test_eval_list_idiom_clean(self):
        """eval [list ...] is safe -- list prevents double substitution."""
        diags = _diag_with_code("eval [list set $varname $value]", "W101")
        assert len(diags) == 0

    def test_eval_linsert_idiom_clean(self):
        """eval [linsert ...] is safe -- all list-returning cmds are canonical."""
        diags = _diag_with_code("eval [linsert $cmdlist 0 extraarg]", "W101")
        assert len(diags) == 0

    def test_eval_split_idiom_clean(self):
        """eval [split ...] is safe -- registry-derived canonical list set."""
        diags = _diag_with_code("eval [split $line :]", "W101")
        assert len(diags) == 0

    def test_eval_concat_still_warns(self):
        """concat strips quoting -- should still warn for eval."""
        diags = _diag_with_code("eval [concat $script $args]", "W101")
        assert len(diags) == 1


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

    def test_pure_separator_clean(self):
        """Appending a bare separator joins a string — not list construction."""
        assert len(_diag_with_code('append msg ", "', "W104")) == 0
        assert len(_diag_with_code('append msg ": "', "W104")) == 0
        assert len(_diag_with_code("append combined $value {, }", "W104")) == 0

    def test_newline_formatted_text_clean(self):
        """A value with a newline is formatted text (message/script), not a list."""
        assert len(_diag_with_code(r'append msg "  $dirs\n\n"', "W104")) == 0

    def test_genuine_list_element_still_flagged(self):
        diags = _diag_with_code('append items " $item"', "W104")
        assert len(diags) == 1

    def test_usage_notation_not_flagged(self):
        """Usage/template notation is display-string formatting, not a list
        element: ``?optarg?`` (optional-argument notation), ``<placeholder>``,
        and ``...`` ("and so on") only ever appear in a generated usage/help
        line (tcllib clay/cmdline ``append result " ?option value?..."`` then
        ``return $result``).  ``lappend`` is not the fix there, so suppress."""
        assert len(_diag_with_code('append result " ?option value?..."', "W104")) == 0
        assert len(_diag_with_code('append result " ?[lindex $argdef 0]?"', "W104")) == 0
        assert len(_diag_with_code('append name " <value>"', "W104")) == 0
        assert len(_diag_with_code('append desc " <$default>"', "W104")) == 0
        assert len(_diag_with_code('append usage " args..."', "W104")) == 0


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


def _taint_diag_with_code(source: str, code: str):
    """Return all taint warnings matching a specific code."""
    from compiler.taint import find_taint_warnings

    return [w for w in find_taint_warnings(source) if w.code == code]


class TestPathConcatenation:
    """W201 -- manual path concatenation instead of file join.

    W201 now runs in the taint system (compiler/taint/_path_concat.py),
    so tests use _taint_diag_with_code() instead of _diag_with_code().
    """

    # True positives

    def test_set_with_path_and_var(self):
        diags = _taint_diag_with_code('set path "$dir/file.txt"', "W201")
        assert len(diags) == 1
        assert "file join" in diags[0].message.lower()

    def test_set_literal_path_clean(self):
        diags = _taint_diag_with_code('set path "/etc/config"', "W201")
        assert len(diags) == 0

    def test_set_no_path_clean(self):
        diags = _taint_diag_with_code("set x 42", "W201")
        assert len(diags) == 0

    def test_path_concatenation_has_fix_for_simple_case(self):
        diags = _taint_diag_with_code('set path "$dir/file.txt"', "W201")
        assert len(diags) == 1
        assert len(diags[0].fixes) == 1
        assert diags[0].fixes[0].new_text == "[file join ${dir} file.txt]"

    # Issue #59 false positives (verbatim from screenshots)

    def test_issue59_join_newline_escape(self):
        """Screenshot 1: generalClasses.tcl -- [join ... \\n]."""
        diags = _taint_diag_with_code(r"set paramDefStr [join $paramDefList \n]", "W201")
        assert len(diags) == 0

    def test_issue59_expr_division(self):
        """Screenshot 2: inverter_optimization.tcl -- [= {1.0/$var}]."""
        diags = _taint_diag_with_code(r"set inpPeriod [= {1.0/$inpFreq}]", "W201")
        assert len(diags) == 0

    def test_issue59_measure_multiline(self):
        """Screenshot 3: inverter_optimization.tcl -- nested cmd subst."""
        source = (
            "set psupplyFinal [measure -xname time "
            "-data [dcreate time [dget $dataDict time] "
            "instantPower $instantPower] "
            '-rms "-vec instantPower"]'
        )
        diags = _taint_diag_with_code(source, "W201")
        assert len(diags) == 0

    # Additional false-positive coverage

    def test_join_with_slash_separator_clean(self):
        """[join ... "/"] -- slash is a join separator, not a path."""
        diags = _taint_diag_with_code('set result [join [list $a $b] "/"]', "W201")
        assert len(diags) == 0

    def test_set_with_tab_escape_clean(self):
        """\\t escape in value is not a path separator."""
        diags = _taint_diag_with_code(r'set line "$name\t$value"', "W201")
        assert len(diags) == 0

    def test_set_with_newline_escape_in_string_clean(self):
        """\\n escape in double-quoted string is not a path separator."""
        diags = _taint_diag_with_code(r'set msg "$greeting\n$farewell"', "W201")
        assert len(diags) == 0

    def test_nested_cmd_subst_clean(self):
        """Nested command substitution -- not path concatenation."""
        diags = _taint_diag_with_code(
            "set result [measure -data [dget $dataDict time] $instantPower]",
            "W201",
        )
        assert len(diags) == 0

    # file normalize / file join suppression

    def test_wrapped_in_file_normalize_clean(self):
        """Value wrapped in [file normalize] -- already normalised."""
        diags = _taint_diag_with_code(r'set path [file normalize "$dir/file.txt"]', "W201")
        assert len(diags) == 0

    def test_file_join_value_clean(self):
        """Value using [file join] -- already proper path construction."""
        diags = _taint_diag_with_code('set path [file join $dir "file.txt"]', "W201")
        assert len(diags) == 0

    def test_normalize_next_line_suppresses(self):
        """Path concatenation followed by file normalize of same var."""
        source = 'set path "$dir/file.txt"\nset path [file normalize $path]'
        diags = _taint_diag_with_code(source, "W201")
        assert len(diags) == 0

    def test_url_scheme_not_path_concat(self):
        """A URL (``scheme://...``) is not a filesystem path — [file join]
        would emit native separators, so W201 must not fire."""
        assert len(_taint_diag_with_code('set url "${proto}://$host:$port/$path"', "W201")) == 0
        assert len(_taint_diag_with_code("set u http://example.com/$page", "W201")) == 0

    def test_genuine_path_concat_still_flagged(self):
        """A real filesystem path concatenation still fires."""
        assert len(_taint_diag_with_code('set p "$dir/$file"', "W201")) == 1

    def test_html_markup_not_path_concat(self):
        """HTML/XML markup (``<tag>``) is not a filesystem path."""
        assert len(_taint_diag_with_code('set v "<link href=$x/$y>"', "W201")) == 0


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

    def test_uplevel_list_idiom_clean(self):
        """uplevel 1 [list ...] is the canonical safe quoting pattern."""
        diags = _diag_with_code("uplevel 1 [list set $varname $value]", "W301")
        assert len(diags) == 0

    def test_uplevel_list_namespace_current(self):
        """Real-world idiom: uplevel 1 [list ::namespace current]."""
        diags = _diag_with_code("set ns [uplevel 1 [list ::namespace current]]", "W301")
        assert len(diags) == 0

    def test_uplevel_lsort_idiom_clean(self):
        """Any list-returning command is safe, not just [list]."""
        diags = _diag_with_code("uplevel 1 [lsort $cmdlist]", "W301")
        assert len(diags) == 0

    def test_uplevel_concat_still_warns(self):
        """concat strips quoting -- should still warn."""
        diags = _diag_with_code("uplevel 1 [concat $script $args]", "W301")
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


class TestW302FireAndForget:
    """``catch {<cmd>}`` without a result var is the documented Tcl idiom
    for "do this if possible, ignore if not" — the standard library
    explicitly recommends it for ``after cancel``, ``file delete``,
    ``close``, ``unset``, etc.  W302 must not fire on those bodies.

    Verified against tclsh 9.0: every command listed in
    ``_FIRE_AND_FORGET_COMMANDS`` errors on missing target, so the
    ``catch {…}`` form is the only safe way to write "delete if exists"
    without first checking ``info exists`` / ``file exists``.  Capturing
    the result would just yield a variable the user immediately
    discards.
    """

    def test_after_cancel_no_w302(self):
        # tcllib ftp.tcl:210 idiom.
        assert _diag_with_code("catch {after cancel $h}", "W302") == []

    def test_file_delete_no_w302(self):
        assert _diag_with_code("catch {file delete $f}", "W302") == []

    def test_close_no_w302(self):
        assert _diag_with_code("catch {close $fh}", "W302") == []

    def test_unset_no_w302(self):
        assert _diag_with_code("catch {unset var}", "W302") == []

    def test_chan_close_no_w302(self):
        assert _diag_with_code("catch {chan close $h}", "W302") == []

    def test_interp_delete_no_w302(self):
        assert _diag_with_code("catch {interp delete slave}", "W302") == []

    def test_qualified_command_recognised(self):
        # ``catch {::close $h}`` should also match — bare-name normalised.
        assert _diag_with_code("catch {::close $h}", "W302") == []

    def test_genuine_swallowed_error_still_fires(self):
        # TP control: a user proc body genuinely shouldn't swallow errors.
        diags = _diag_with_code("catch {parse_user_input $x}", "W302")
        assert len(diags) == 1

    def test_multi_command_body_still_fires(self):
        # Multi-statement body is not the simple ignore-error idiom.
        diags = _diag_with_code("catch {parse_x; parse_y}", "W302")
        assert len(diags) == 1

    def test_package_require_still_fires(self):
        # ``package require`` is loadable — when the caller cares about
        # success they should capture.  Sole-substitution use
        # (``[catch {package require Tk}]``) doesn't fire anyway because
        # the rc IS being consumed at the call site.
        diags = _diag_with_code("catch {package require Tk}", "W302")
        assert len(diags) == 1

    # Subcommand precision: the ensemble commands ``after``, ``chan``,
    # ``array``, ``dict``, ``interp``, ``namespace``, ``file`` have BOTH
    # destructive subcommands (the fire-and-forget case) and constructive
    # subcommands (which genuinely shouldn't swallow errors).  The
    # suppression must distinguish them.

    def test_chan_configure_still_fires(self):
        # ``chan configure`` is NOT fire-and-forget.
        diags = _diag_with_code("catch {chan configure $h -buffering none}", "W302")
        assert len(diags) == 1

    def test_file_copy_still_fires(self):
        diags = _diag_with_code("catch {file copy a b}", "W302")
        assert len(diags) == 1

    def test_after_timer_still_fires(self):
        # ``after <ms> <script>`` is a scheduling op, not a cancel.
        diags = _diag_with_code("catch {after 1000 puts hi}", "W302")
        assert len(diags) == 1

    def test_interp_create_still_fires(self):
        diags = _diag_with_code("catch {interp create slave}", "W302")
        assert len(diags) == 1

    def test_namespace_eval_still_fires(self):
        diags = _diag_with_code("catch {namespace eval ::ns {}}", "W302")
        assert len(diags) == 1

    def test_dict_set_still_fires(self):
        diags = _diag_with_code("catch {dict set d k v}", "W302")
        assert len(diags) == 1

    def test_dict_unset_no_w302(self):
        # Destructive sibling: still suppressed.
        assert _diag_with_code("catch {dict unset d k}", "W302") == []

    def test_array_unset_no_w302(self):
        assert _diag_with_code("catch {array unset a *}", "W302") == []

    def test_namespace_delete_no_w302(self):
        assert _diag_with_code("catch {namespace delete ::ns}", "W302") == []

    def test_namespace_forget_no_w302(self):
        # ``namespace forget`` removes import; errors if not imported.
        assert _diag_with_code("catch {namespace forget ::pkg::*}", "W302") == []

    def test_rename_to_empty_no_w302(self):
        # ``rename foo ""`` deletes the command; errors if foo absent.
        assert _diag_with_code('catch {rename foo ""}', "W302") == []


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


class TestInfoExistsNotReadBeforeSet:
    """`info exists`/`array exists` is the canonical test-before-use idiom —
    referencing an unset variable there is legal (returns 0), so it must never
    be flagged read-before-set (W210)."""

    def test_info_exists_guard_no_w210(self):
        src = "proc f {} { if {![info exists ns]} { set ns 1 }\n return $ns }"
        assert len(_diag_with_code(src, "W210")) == 0

    def test_info_exists_assigned_no_w210(self):
        src = "proc f {} { set y [info exists x]\n return $y }"
        assert len(_diag_with_code(src, "W210")) == 0

    def test_info_exists_array_element_no_w210(self):
        src = "proc f {} { if {[info exists a(b)]} { return 1 }\n return 0 }"
        assert len(_diag_with_code(src, "W210")) == 0

    def test_info_exists_guarded_value_read_no_w210(self):
        # `[info exists a] && $a` — the value read is guarded by the test.
        src = "proc f {} { if {[info exists a] && ($a eq {})} { return 1 } }"
        assert len(_diag_with_code(src, "W210")) == 0

    def test_array_exists_no_w210(self):
        src = "proc f {} { if {[array exists arr]} { return [array size arr] }\n return 0 }"
        assert len(_diag_with_code(src, "W210")) == 0

    def test_while_info_exists_no_w210(self):
        src = "proc f {} { while {[info exists q]} { unset q } }"
        assert len(_diag_with_code(src, "W210")) == 0

    def test_genuine_read_before_set_still_warns(self):
        # A plain value read of an unset variable must still fire.
        assert len(_diag_with_code("proc f {} { set y $undefined\n return $y }", "W210")) == 1

    def test_qualified_info_exists_no_false_unused(self):
        # A fully-qualified builtin (`::info exists x`) resolves to the same
        # arg-role spec as `info exists x`, so x is recognised as referenced —
        # no false W211/W220 unused/dead-store (the bare form was already clean).
        src = "proc f {} { set x 1\n if {[::info exists x]} { return 1 }\n return 0 }"
        assert _diag_with_code(src, "W220") == []
        assert _diag_with_code(src, "W211") == []


class TestReturnReadBeforeSet:
    """A return value reads its variables — ``return $x`` / ``return [expr
    {$x+1}]`` error in tclsh when x is unset, so they must fire W210."""

    def test_return_unset_var_warns(self):
        assert len(_diag_with_code("proc f {} { return $x }", "W210")) == 1

    def test_return_expr_unset_var_warns(self):
        assert len(_diag_with_code("proc f {} { return [expr {$x + 1}] }", "W210")) == 1

    def test_return_set_var_no_w210(self):
        assert _diag_with_code("proc f {} { set x 1\n return $x }", "W210") == []

    def test_return_literal_no_w210(self):
        assert _diag_with_code("proc f {} { return 42 }", "W210") == []


class TestO107BreakReachability:
    """`break`/`continue` are modelled as jump *statements*, not CFG edges, so a
    `break` inside a constant-condition loop (`while 1 {... break ...}`) left
    the loop-exit block reachable only via the header's exit edge — which SCCP
    prunes as dead.  The exit was then wrongly 'unreachable' (false O107 + an
    unsound DCE).  SCCP now feeds the break->loop-exit edge into reachability."""

    @staticmethod
    def _o107(src: str) -> list:
        # O107 is an optimiser code surfaced via get_diagnostics (not analyse()).
        return [d for d in get_diagnostics(src) if d.code == "O107"]

    def test_while1_break_after_reachable(self):
        src = "proc f {c} { while 1 { if {$c} break }\n puts after }"
        assert self._o107(src) == []

    def test_for_true_break_after_reachable(self):
        src = "proc f {c} { for {set i 0} true {incr i} { if {$c} break }\n puts after }"
        assert self._o107(src) == []

    def test_nested_loop_break_reachable(self):
        src = "proc f {c} { while 1 { foreach x {1 2} { if {$c} break }\n if {$c} break }\n puts after }"
        assert self._o107(src) == []

    def test_genuine_infinite_loop_still_unreachable(self):
        # No break → code after an infinite loop IS unreachable (true O107).
        src = "proc f {} { while 1 { puts x }\n puts after }"
        assert len(self._o107(src)) == 1


class TestTryHandlerExceptionModelling:
    """``try`` body→handler control flow is modelled as SSA exception edges
    (analysis builds only — codegen leaves them off so default bytecode stays
    tclsh-identical).  A handler block therefore (a) is *reachable* — no false
    O107 'unreachable dead code' on the handler body — and (b) inherits the
    right SSA versions: ``on ok`` runs after the body completes normally so it
    sees body-set vars (no false W210); ``on error`` runs on an abnormal
    completion from the pre-``try`` state."""

    @staticmethod
    def _codes(src: str, code: str) -> list:
        return [d for d in get_diagnostics(src) if d.code == code]

    def test_handler_body_not_unreachable(self):
        # The handler body used to be a CFG island (no predecessor edge) →
        # false O107 on every statement in it.
        src = (
            "proc f {} {\n"
            "    try {\n"
            "        set x [doThing]\n"
            "    } on error {e opts} {\n"
            "        set y 1\n"
            "        puts $y\n"
            "    }\n"
            "}"
        )
        assert self._codes(src, "O107") == []

    def test_on_ok_reads_body_var_no_w210(self):
        # `on ok` runs only after the body completes, so `$vdata` (set in the
        # body) is defined — must not be read-before-set.
        src = (
            "proc f {} {\n"
            "    try {\n"
            "        set vdata [getData]\n"
            "    } on ok {} {\n"
            "        return $vdata\n"
            "    }\n"
            "}"
        )
        assert self._codes(src, "W210") == []

    def test_on_error_handler_var_defined(self):
        # The handler-bound var `e` is defined by the handler clause itself.
        src = (
            "proc f {} {\n    try {\n        risky\n    } on error {e} {\n        puts $e\n    }\n}"
        )
        assert self._codes(src, "W210") == []


class TestUplevelBareVarNotInjection:
    """``uplevel 1 $body`` (a single *pure* variable) is the safe idiom — the
    value is evaluated once in the target frame, no double substitution
    (verified in tclsh).  It cannot be braced (``{...}`` would eval the literal
    text, not the variable's script), so the W301 'use braces' advice is wrong.
    The genuine risk — quoted interpolation or multi-arg concat — still fires."""

    def test_bare_var_no_w301(self):
        assert _diag_with_code("proc f {body} { uplevel 1 $body }", "W301") == []

    def test_array_elem_var_no_w301(self):
        assert _diag_with_code("proc f {} { uplevel 1 $cmds(init) }", "W301") == []

    def test_no_level_bare_var_no_w301(self):
        assert _diag_with_code("proc f {body} { uplevel $body }", "W301") == []

    def test_list_idiom_no_w301(self):
        assert _diag_with_code("proc f {a} { uplevel 1 [list set y $a] }", "W301") == []

    def test_quoted_interpolation_still_w301(self):
        assert len(_diag_with_code('proc f {x} { uplevel 1 "set y $x" }', "W301")) == 1

    def test_multi_arg_concat_still_w301(self):
        assert len(_diag_with_code("proc f {a b} { uplevel 1 set $a $b }", "W301")) == 1

    def test_concatenated_vars_still_w301(self):
        # `$x$y` composes a value from two substitutions (double substitution),
        # so it is NOT the safe single-var idiom even though it starts with a
        # VAR token.  Must still warn.
        assert len(_diag_with_code("proc f {x y} { uplevel 1 $x$y }", "W301")) == 1

    def test_var_dot_literal_still_w301(self):
        # `$x.foo` concatenates a var with literal text — still double substitution.
        assert len(_diag_with_code("proc f {x} { uplevel 1 $x.foo }", "W301")) == 1

    def test_braced_var_no_w301(self):
        # `${body}` is a single pure reference — exempt.
        assert _diag_with_code("proc f {body} { uplevel 1 ${body} }", "W301") == []


class TestArrayElementDeadStoreDistinction:
    """Array elements fold to one SSA name with sequential versions, so a write
    to ``a(k)`` then ``a(j)`` made the ``a(k)`` write look dead (W220) even
    though ``$a(k)`` is read later.  The Place-overlap fallback (Phase 8E/8G)
    distinguishes elements: an element write observed by an overlapping read is
    not dead."""

    def test_distinct_element_write_not_dead(self):
        # set a(k) is read by puts $a(k); the intervening a(j) write must not
        # make it look dead (was a W220 false positive).
        src = "proc f {} { set a(k) 1\n set a(j) 2\n puts $a(k) }"
        assert _diag_with_code(src, "W220") == []

    def test_array_built_then_used_not_dead(self):
        # Building an options array element-by-element, then reading it.
        src = "proc f {} { set o(-mode) cbc\n set o(-key) k\n return [array get o] }"
        assert _diag_with_code(src, "W220") == []
        assert _diag_with_code(src, "W211") == []

    def test_genuinely_unused_element_still_fires(self):
        # No read of a anywhere → still a dead store + unused.
        src = "proc f {} { set a(k) 1\n return 0 }"
        assert len(_diag_with_code(src, "W220")) == 1
        assert len(_diag_with_code(src, "W211")) == 1

    def test_scalar_dead_store_unaffected(self):
        # Scalars keep version-precise dead-store detection.
        src = "proc f {} { set x 1\n set x 2\n return $x }"
        assert len(_diag_with_code(src, "W220")) == 1


class TestCallByNameSuppression:
    """A caller-local variable passed by *literal name* to a user proc whose
    parameter has the ``VAR_READ`` / ``VAR_WRITE`` trait (i.e. the receiver
    ``upvar``s it) is being read/written indirectly.  The per-function SSA
    can't see that, so W211 ("set but never used") and W220 ("never read")
    must not fire on those locals — the interprocedural lattice
    (``ProcDef.param_traits``) closes the gap.

    tclsh-verified pattern: the canonical tcllib ``validate_*_cmp im dm``
    idiom where the receiver does ``upvar $a aa $b bb`` then iterates the
    aliased array."""

    def test_call_by_name_silences_W211_W220(self):
        # The receiver upvar's both params; the caller's $im/$dm are read
        # indirectly through those aliases.
        src = (
            "proc cmp {ipvar ppvar} {\n"
            "    upvar $ipvar ip $ppvar pp\n"
            '    foreach k [array names ip] { puts "$k=$ip($k)" }\n'
            "    foreach k [array names pp] { if {![info exists ip($k)]} { puts miss } }\n"
            "}\n"
            "proc top {} {\n"
            "    foreach m [list a b] { set im($m) . }\n"
            "    foreach m [list c d] { set dm($m) . }\n"
            "    cmp im dm\n"
            "}\n"
        )
        assert _diag_with_code(src, "W211") == []
        assert _diag_with_code(src, "W220") == []

    def test_call_by_name_write_only_param(self):
        # VAR_WRITE alone (receiver upvar's the param and `set`s into it) is
        # an output param — also an indirect "use" of the caller's name.
        src = (
            "proc fill {outvar} {\n"
            "    upvar $outvar out\n"
            "    set out 42\n"
            "}\n"
            "proc top {} {\n"
            "    set result {}\n"
            "    fill result\n"
            "    return $result\n"
            "}\n"
        )
        assert _diag_with_code(src, "W211") == []
        assert _diag_with_code(src, "W220") == []

    def test_unrelated_dead_store_still_fires(self):
        # TP control: a dead store of a variable NOT passed to a name-receiver
        # call must still fire.  Only the by-name variable is exempt.
        src = (
            "proc cmp {ipvar} {\n"
            "    upvar $ipvar ip\n"
            "    return [array size ip]\n"
            "}\n"
            "proc top {} {\n"
            "    set im(x) 1\n"
            "    set never_used 999\n"
            "    cmp im\n"
            "}\n"
        )
        assert any("never_used" in d.message for d in _diag_with_code(src, "W211"))
        assert any("never_used" in d.message for d in _diag_with_code(src, "W220"))

    def test_substituted_arg_not_exempt(self):
        # If the arg is a substitution (``cmp $varname``) we can't tell which
        # local it names → don't suppress.  Conservative: any literal-name
        # dead store stays a TP.
        src = (
            "proc cmp {ipvar} {\n"
            "    upvar $ipvar ip\n"
            "    return [array size ip]\n"
            "}\n"
            "proc top {arg} {\n"
            "    set never_used 1\n"
            "    cmp $arg\n"
            "}\n"
        )
        assert any("never_used" in d.message for d in _diag_with_code(src, "W211"))

    def test_user_proc_without_upvar_is_not_a_name_receiver(self):
        # ``cmp`` here just takes the param by value (no upvar).  Passing
        # ``ipvar`` as the literal "im" is NOT an indirect read — it's a
        # genuine string literal.  So a preceding dead store on ``im`` would
        # still fire.  (No call-by-name trait on the param → no suppression.)
        src = "proc cmp {ipvar} { return $ipvar }\nproc top {} {\n    set im(x) 1\n    cmp im\n}\n"
        # ``im`` is an array element; the W211/W220 check still flags the
        # write because ``im`` is never read by the caller, and ``cmp`` does
        # not upvar.  (This pre-existing TP must still fire.)
        assert _diag_with_code(src, "W211") != [] or _diag_with_code(src, "W220") != []


class TestQualifiedVariableAliasNotReadBeforeSet:
    """``variable ns::tail`` links a local alias named by the *tail* (verified in
    tclsh), and a ``::``-qualified name is always a namespace reference whose
    storage lives outside the frame — neither is a local read-before-set."""

    def test_fully_qualified_variable_tail_no_w210(self):
        # `variable ::tcl::WordBreakRE` then `$WordBreakRE(after)` reads the
        # local alias `WordBreakRE` (Tcl links the tail), not an unset local.
        src = (
            "proc f {} {\n"
            "    variable ::tcl::WordBreakRE\n"
            "    regexp -indices -- $WordBreakRE(after) abc result\n"
            "    return $result\n"
            "}"
        )
        assert len(_diag_with_code(src, "W210")) == 0

    def test_dynamic_prefix_variable_tail_no_w210(self):
        # `variable ${name}::children` (struct::tree idiom) — local alias is the
        # static tail `children`; `$children($node)` is not read-before-set.
        src = (
            "proc f {name node} {\n"
            "    variable ${name}::children\n"
            "    set kids $children($node)\n"
            "    return $kids\n"
            "}"
        )
        assert len(_diag_with_code(src, "W210")) == 0

    def test_direct_qualified_namespace_read_no_w210(self):
        # `array names ${name}::parent` reads the namespace var directly; the
        # `::`-qualified name is managed in its namespace, not this frame.
        src = "proc f {name} {\n    set n [llength [array names ${name}::parent]]\n    return $n\n}"
        assert len(_diag_with_code(src, "W210")) == 0

    def test_genuine_local_read_before_set_still_warns(self):
        # An unqualified local that is genuinely read before set must still fire.
        src = "proc f {} {\n    set kids $children\n    return $kids\n}"
        assert len(_diag_with_code(src, "W210")) == 1


class TestReadModifyWriteInCmdSub:
    """A read-modify-write command (`incr`/`append`/`lappend`) reads its target's
    prior value.  When it appears inside a command substitution (`lappend out
    [incr i $j]`), that read is otherwise invisible, making the feeding `set i 0`
    look like a dead store (W220) / unused variable (W211)."""

    def test_incr_in_cmdsub_keeps_init_live(self):
        src = "proc f {} { set i 0\n foreach j {1 2 3} { lappend r [incr i $j] }\n return $r }"
        assert len(_diag_with_code(src, "W220")) == 0
        assert len(_diag_with_code(src, "W211")) == 0

    def test_append_in_cmdsub_keeps_init_live(self):
        src = "proc f {} { set s {}\n foreach x {a b} { puts [append s $x] }\n return $s }"
        assert len(_diag_with_code(src, "W220")) == 0

    def test_genuine_dead_store_still_flagged(self):
        # No read-modify-write read of i — the first assignment is truly dead.
        src = "proc f {} { set i 0\n set i 5\n return $i }"
        assert len(_diag_with_code(src, "W220")) == 1


class TestFrozenLoopBodyWrites:
    """A ``while``/``for`` with a command-substitution condition is kept as an
    opaque barrier (tclsh-parity codegen); its body never enters the CFG.  The
    body's reads are recovered name-level, so its writes must be too — otherwise
    every body-local variable looks read-before-set (W210)."""

    def test_while_cmdsub_cond_body_set_no_w210(self):
        src = (
            "proc f {} { set p {a b}\n while {[llength $p]} "
            "{ set d [lindex $p end]\n puts $d\n set p [lrange $p 0 end-1] } }"
        )
        assert len(_diag_with_code(src, "W210")) == 0

    def test_while_cmdsub_cond_foreach_var_no_w210(self):
        src = "proc f {} { while {[llength $q]} { foreach item $q { puts $item }\n set q {} } }"
        assert len(_diag_with_code(src, "W210")) == 0

    def test_while_string_length_cond_body_no_w210(self):
        src = "proc f {} { while {[string length $hexa]} { set hex 0\n puts $hex\n set hexa {} } }"
        assert len(_diag_with_code(src, "W210")) == 0

    def test_frozen_loop_genuine_unset_still_warns(self):
        # A variable never written in the body is still read-before-set.
        src = "proc f {} { while {[llength $p]} { puts $never_written\n set p {} } }"
        assert len(_diag_with_code(src, "W210")) == 1

    def test_qualified_foreach_loop_vars_no_w210(self):
        # `::foreach` (fully-qualified) stays an un-lowered IRCall, so its loop
        # variables must still be recovered as defs (not read-before-set).
        src = "proc f {} { set d {a 1}\n ::foreach {k v} $d { puts $k$v } }"
        assert len(_diag_with_code(src, "W210")) == 0

    def test_qualified_foreach_single_var_no_w210(self):
        src = "proc f {} { set lst {a b}\n ::foreach item $lst { puts $item } }"
        assert len(_diag_with_code(src, "W210")) == 0

    def test_qualified_for_body_set_no_w210(self):
        src = "proc f {} { ::for {set i 0} {$i < 3} {incr i} { set x $i\n puts $x } }"
        assert len(_diag_with_code(src, "W210")) == 0

    def test_try_handler_var_in_frozen_loop_no_w210(self):
        # `on error msg` binds msg; inside a frozen loop body it must be
        # recovered so `$msg` isn't read-before-set.
        src = (
            "proc f {} { set p {a}\n while {[llength $p]} "
            "{ try { error x } on error msg { puts $msg }\n set p {} } }"
        )
        assert len(_diag_with_code(src, "W210")) == 0


class TestGlobalsWrittenByProcs:
    """Top-level reads of globals written by procs should not trigger W210.

    Pattern from tcllib's installer.tcl: a proc declares ``global X`` and
    sets X; the proc is called via ``source`` (from another file) before
    the top-level reads X.
    """

    def test_proc_sets_global_via_alias_suppresses_w210(self):
        src = (
            "proc set_name {v} {global package_name; set package_name $v}\n"
            "source foo.tcl\n"
            "set out $package_name\n"
        )
        diags = _diag_with_code(src, "W210")
        assert [d.message for d in diags if "package_name" in d.message] == []

    def test_proc_writes_fully_qualified_global_suppresses_w210(self):
        src = "proc set_name {v} {set ::package_name $v}\nset out $package_name\n"
        diags = _diag_with_code(src, "W210")
        assert [d.message for d in diags if "package_name" in d.message] == []

    def test_proc_incr_global_suppresses_w210(self):
        src = "proc bump {} {global counter; incr counter}\nputs $counter\n"
        diags = _diag_with_code(src, "W210")
        assert [d.message for d in diags if "counter" in d.message] == []

    def test_global_alias_without_write_still_warns(self):
        """``global X`` alone (no set) doesn't populate the global."""
        src = "proc reader {} {global package_name; return $package_name}\nset out $package_name\n"
        diags = _diag_with_code(src, "W210")
        names = [d.message for d in diags if "package_name" in d.message]
        assert len(names) == 1

    def test_unrelated_global_still_warns(self):
        """Procs writing other globals don't mask real read-before-set."""
        src = "proc set_name {v} {global package_name; set package_name $v}\nputs $other_var\n"
        diags = _diag_with_code(src, "W210")
        assert any("other_var" in d.message for d in diags)

    def test_unset_does_not_suppress_w210(self):
        """``unset`` destroys a global rather than initialising it."""
        src = "proc clear {} {unset ::x}\nputs $x\n"
        diags = _diag_with_code(src, "W210")
        assert any("'x'" in d.message for d in diags)

    def test_unset_via_alias_does_not_suppress_w210(self):
        """``global X; unset X`` also destroys; it shouldn't count as a write."""
        src = "proc clear {} {global x; unset x}\nputs $x\n"
        diags = _diag_with_code(src, "W210")
        assert any("'x'" in d.message for d in diags)


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

    def test_regexp_literal_safe_pattern_suppressed(self):
        # Pattern starts with '(' — structurally cannot be confused with an option.
        diags = _diag_with_code("regexp {(a+)+$} $text", "W304")
        assert len(diags) == 0

    def test_regexp_literal_dash_pattern_warns(self):
        # Pattern starts with '-' — could be confused with -nocase etc.
        diags = _diag_with_code("regexp {-[0-9]+} $text", "W304")
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

    def test_string_match_not_checked_by_w304(self):
        # string match does not support -- in real Tcl.
        diags = _diag_with_code("string match $pattern $value", "W304")
        assert len(diags) == 0

    def test_lsearch_not_checked_by_w304(self):
        # lsearch does not support -- in real Tcl.
        diags = _diag_with_code("lsearch -exact $domain c", "W304")
        assert len(diags) == 0

    def test_file_delete_variable_without_terminator(self):
        diags = _diag_with_code("file delete $path", "W304")
        assert len(diags) == 1

    def test_file_delete_with_terminator_clean(self):
        diags = _diag_with_code("file delete -- $path", "W304")
        assert len(diags) == 0

    def test_load_variable_without_terminator(self):
        diags = _diag_with_code("load $fileName", "W304")
        assert len(diags) == 1

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
        # Exclude W123 (opt-in, default=False) which fires for risky_op.
        w_diags = [d for d in result.diagnostics if d.code.startswith("W") and d.code != "W123"]
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
        # Exclude W123 (opt-in, default=False) which fires for risky.
        w_diags = [d for d in result.diagnostics if d.code.startswith("W") and d.code != "W123"]
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

    def test_eval_string_offers_list_fix(self):
        """`eval "cmd $x"` offers a quick-fix to the safe `eval [list cmd $x]`
        form. Verified vs C tclsh: the list form passes $x as a single argument
        and is not re-parsed, so it cannot inject (whereas the string form with
        x={a; exec …} executes the payload). (Updated from the old baseline,
        which assumed eval had no auto-fix.)"""
        src = 'eval "process $x"'
        diags = [d for d in analyse(src).diagnostics if d.code == "W101"]
        assert len(diags) == 1
        assert len(diags[0].fixes) == 1
        fix = diags[0].fixes[0]
        rewritten = src[: fix.range.start.offset] + fix.new_text + src[fix.range.end.offset :]
        assert rewritten == "eval [list process $x]"

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


class TestNamespaceShadowedArity:
    """Builtin arity suppression must be namespace-aware (issue: PR #472).

    A ``proc`` whose name matches a builtin only shadows that builtin for
    calls that actually resolve to it.  A call in another namespace (e.g. a
    global ``close``) still reaches the builtin and must keep its arity
    check.
    """

    @staticmethod
    def _arity_codes(source):
        from analyser.compiler_checks import run_compiler_checks

        return [d for d in run_compiler_checks(source) if d.code in ("E002", "E003")]

    def test_global_call_to_shadowed_builtin_still_checked(self):
        # ``proc ::ns::close`` must not silence a global ``close`` that
        # resolves to the builtin (max 2 args).
        source = (
            "proc ::ns::close {a b c d} {}\n"
            "close x y z\n"  # global -> builtin close, 3 args -> E003
        )
        diags = self._arity_codes(source)
        assert len(diags) == 1
        assert diags[0].code == "E003"
        assert "at most 2" in diags[0].message

    def test_call_resolving_to_user_proc_is_suppressed(self):
        # ``close`` inside a ``::ns`` proc resolves to ``::ns::close`` (the
        # user proc, 3 args) and must NOT trip the builtin arity check.
        source = (
            "proc ::ns::close {a b c} {}\n"
            "proc ::ns::caller {} { close x y z }\n"
            "close x y z\n"  # only the global call reaches the builtin
        )
        diags = self._arity_codes(source)
        assert len(diags) == 1
        assert "at most 2" in diags[0].message  # the builtin, not ::ns::close

    def test_shadowing_proc_in_other_namespace_does_not_suppress(self):
        # A proc named ``close`` in an unrelated namespace must not silence
        # the global builtin call.
        source = "proc ::other::close {a b c} {}\nclose x y z\n"
        diags = self._arity_codes(source)
        assert len(diags) == 1
        assert diags[0].code == "E003"

    def test_top_level_call_before_definition_still_checked(self):
        # A global call resolves to the builtin when it textually precedes
        # the shadowing proc — script load runs in order, so suppression
        # must respect definition order.
        source = "close x y z\nproc close {a b c d} {}\n"
        diags = self._arity_codes(source)
        assert len(diags) == 1
        assert diags[0].code == "E003"

    def test_conditionally_defined_proc_does_not_suppress(self):
        # A proc defined only inside a conditional is not statically known
        # to exist, so the builtin arity check must still fire.
        source = "if {$x} { proc close {a b c d} {} }\nclose x y z\n"
        diags = self._arity_codes(source)
        assert len(diags) == 1
        assert diags[0].code == "E003"

    def test_top_level_call_after_definition_is_suppressed(self):
        # Once the proc is defined, a later global call resolves to it.
        source = "proc close {a b c} {}\nclose x y z\n"
        assert self._arity_codes(source) == []

    def test_websocket_close_no_false_positive(self):
        # Real-world shape: ``::websocket::close`` takes 3 args and is called
        # as a bare ``close`` from procs in the same namespace.
        source = (
            "namespace eval ::websocket {}\n"
            "proc ::websocket::close {sock {code 1000} {reason {}}} {}\n"
            "proc ::websocket::Receiver {} {\n"
            '    close $sock 1009 "too big"\n'
            "}\n"
        )
        assert self._arity_codes(source) == []


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

    def test_eval_var_quickfix_preserves_dollar(self):
        # Regression for #438 — the quick-fix used to wrap ``args[idx]``
        # (the *post-substitution* value, ``script``) and produced
        # ``{script}``, silently dropping the ``$``.  It must wrap the
        # raw source slice so ``$script`` round-trips intact.
        diags = _diag_with_code("eval $script", "W105")
        assert len(diags) == 1
        assert diags[0].fixes
        assert diags[0].fixes[0].new_text == "{$script}"

    def test_while_var_quickfix_preserves_dollar(self):
        # Same shape as above for ``while`` body argument.
        diags = _diag_with_code("while 1 $body", "W105")
        assert len(diags) == 1
        assert diags[0].fixes
        assert diags[0].fixes[0].new_text == "{$body}"


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

    def test_regexp_bare_var_pattern_clean(self):
        # A bare single $var as the entire pattern is the canonical Tcl
        # idiom for a parameterised regex.  There is no equivalent
        # ``{...}``-braced form, so W306 must not fire.  See issue #235.
        diags = _diag_with_code("regexp -- $pattern $text", "W306")
        assert len(diags) == 0

    def test_regexp_bare_braced_var_pattern_clean(self):
        # ``${name}`` form is also a bare single substitution.
        diags = _diag_with_code("regexp -- ${ns::pattern} $text", "W306")
        assert len(diags) == 0

    def test_regsub_bare_cmd_pattern_warns(self):
        # ``[cmd]`` patterns must still be flagged: a literal like ``[a-z]``
        # looks like a regex character class but is parsed by Tcl as a
        # command substitution.  Catching that confusion is exactly what
        # W306 is for.
        diags = _diag_with_code("regsub -- [build_re] $text X out", "W306")
        assert len(diags) == 1

    def test_regexp_char_class_looking_cmd_subst_warns(self):
        # Classic Tcl foot-gun: ``[a-z]`` parses as command substitution.
        diags = _diag_with_code("regexp -- [a-z] $text", "W306")
        assert len(diags) == 1

    def test_regexp_bare_var_in_dict_filter_body_clean(self):
        # Issue #235 reproducer: the bare-var suppression must apply when
        # the pattern is inside a nested braced script body (the analyser
        # recursively checks bodies of ``dict filter ... script ... body``).
        diags = _diag_with_code(
            "set out [dict filter $d script {key value} {regexp $pattern $value}]",
            "W306",
        )
        assert len(diags) == 0

    def test_regexp_bare_var_in_proc_body_clean(self):
        # Suppression must also apply deep inside proc/if/foreach bodies,
        # which use absolute base-offsets when re-lexed.
        source = (
            "proc f {} {\n"
            "    if {1} {\n"
            "        foreach x [list a b] {\n"
            "            regexp $pattern $x\n"
            "        }\n"
            "    }\n"
            "}\n"
        )
        diags = _diag_with_code(source, "W306")
        assert len(diags) == 0

    def test_regexp_concatenated_var_pattern_warns(self):
        # Concatenation like ``$a$b`` is not a single bare substitution
        # and is suspicious enough to warrant the warning.
        diags = _diag_with_code("regexp -- $pattern$suffix $text", "W306")
        assert len(diags) == 1
        assert diags[0].severity == Severity.WARNING
        assert "Literal expected" in diags[0].message

    def test_regexp_braced_pattern_clean(self):
        diags = _diag_with_code("regexp -- {^foo} $text", "W306")
        assert len(diags) == 0

    def test_regexp_quoted_pattern_with_var(self):
        diags = _diag_with_code('regexp -- "$pattern" $text', "W306")
        assert len(diags) == 1
        assert "quotes" in diags[0].message.lower()

    def test_regexp_quoted_pattern_with_dollar_anchor(self):
        # The classic foot-gun: ``$"`` is unintended substitution where
        # the user likely meant the regex end-of-line anchor.
        diags = _diag_with_code('regexp -- "^foo$" $text', "W306")
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

    def test_command_substitution_as_command(self):
        diags = _diag_with_code("[get_cmd] arg1", "W307")
        assert len(diags) == 1
        assert "Non-literal" in diags[0].message

    def test_literal_command_clean(self):
        diags = _diag_with_code("puts hello", "W307")
        assert len(diags) == 0

    def test_tcloo_var_command_no_w307(self):
        """$obj method should NOT emit W307 when obj is a TclOO instance."""
        source = """\
oo::class create MyClass {
    method greet {name} {
        puts "hello $name"
    }
}
set obj [MyClass new]
$obj greet world
"""
        diags = _diag_with_code(source, "W307")
        assert len(diags) == 0

    def test_snit_self_dispatch_no_w307(self):
        """snit's reserved object/type self-references dispatch on the object —
        ``$self foo`` / ``$type bar`` / ``$selfns`` / ``$win`` are never a stray
        non-literal command."""
        for ref in ("self", "type", "selfns", "win"):
            src = f"snit::type T {{\n method m {{}} {{ ${ref} foo }}\n}}"
            assert _diag_with_code(src, "W307") == [], ref

    def test_genuine_var_command_still_w307(self):
        # A non-self-reference variable command word is still flagged.
        assert len(_diag_with_code("proc f {} { $myparser parse }", "W307")) == 1

    def test_self_ref_outside_snit_body_still_w307(self):
        # The self-ref exemption is scoped to a snit body: a variable named
        # ``self`` (etc.) used as a command word in a vanilla proc is a genuine
        # non-literal command and must still be flagged.
        for ref in ("self", "type", "selfns", "win", "hull"):
            src = f"proc f {{}} {{ set {ref} [getThing]\n ${ref} foo }}"
            assert len(_diag_with_code(src, "W307")) == 1, ref

    def test_snit_hull_dispatch_no_w307(self):
        # ``$hull configure`` is the widgetadaptor delegation idiom.
        src = "snit::widgetadaptor W {\n method m {} { $hull configure -bg red }\n}"
        assert _diag_with_code(src, "W307") == []

    def test_factory_object_does_not_leak_across_procs(self):
        # A factory assignment in one proc must not suppress a same-named
        # variable's dispatch in another proc (where it may hold anything).
        leak = "proc a {} { set t [::struct::tree] }\nproc b {x} { set t $x\n $t foo }\n"
        assert len(_diag_with_code(leak, "W307")) == 1
        # ...but a same-proc factory dispatch is still suppressed.
        same = "proc a {} { set t [::struct::tree]\n $t foo }\n"
        assert _diag_with_code(same, "W307") == []

    def test_snit_component_dispatch_no_w307(self):
        """Inside a snit method body, dispatching on a component/instance var
        (``$myexporter export``) is object method dispatch, not a stray
        non-literal command — like an oo::class method body."""
        src = (
            "snit::type T {\n"
            "    component myexporter\n"
            "    method run {fmt} {\n"
            "        return [$myexporter export object $self $fmt]\n"
            "    }\n"
            "}"
        )
        assert _diag_with_code(src, "W307") == []

    def test_snit_constructor_dispatch_no_w307(self):
        src = (
            "snit::type T {\n"
            "    variable handler\n"
            "    constructor {args} {\n"
            "        $handler reset\n"
            "    }\n"
            "}"
        )
        assert _diag_with_code(src, "W307") == []

    def test_snit_implicit_vars_no_read_before_set(self):
        """snit's implicit ``self``/``type``/``options`` plus declared instance
        variables must not surface as read-before-set/unused."""
        src = (
            "snit::type T {\n"
            "    variable handler\n"
            "    option -mode normal\n"
            "    method m {} {\n"
            "        set x $self\n"
            "        set y $options(-mode)\n"
            "        set z $handler\n"
            "        return [list $x $y $z $type]\n"
            "    }\n"
            "}"
        )
        assert _diag_with_code(src, "W210") == []

    def test_snit_type_private_proc_still_analysed(self):
        """A ``proc`` inside a snit body is a type-private proc — its body must
        still be analysed (not silently dropped)."""
        src = (
            "snit::type T {\n    proc Helper {a} {\n        return [info exists ${a}($a)]\n    }\n}"
        )
        # The ${a}($a) scalar-vs-array smell must still fire from the proc body.
        diags = _diag_with_code(src, "W216")
        assert len(diags) == 1
        assert "${a}($a)" in diags[0].message
        assert "scalar" in diags[0].message

    def test_snit_typemethod_dispatch_no_w307(self):
        src = (
            "snit::type T {\n"
            "    typevariable registry\n"
            "    typemethod lookup {k} {\n"
            "        return [$registry get $k]\n"
            "    }\n"
            "}"
        )
        assert _diag_with_code(src, "W307") == []

    def test_snit_instance_provenance_no_w307(self):
        """A var holding a locally-defined snit instance (``[Foo create
        %AUTO%]`` / ``[Foo %AUTO%]`` / ``[Foo create name]``) is typed OBJECT,
        so dispatch on it outside a method body is not W307."""
        src = (
            "snit::type ::Counter { method bump {} { return 1 } }\n"
            "proc use {} {\n"
            "    set a [Counter create %AUTO%]\n"
            "    $a bump\n"
            "    set b [Counter create mine]\n"
            "    $b bump\n"
            "    set c [Counter %AUTO%]\n"
            "    $c bump\n"
            "}"
        )
        assert _diag_with_code(src, "W307") == []

    def test_snit_instance_no_w308(self):
        """snit instances must not get W308 method validation — delegation /
        hull / options / built-ins make it unsound."""
        src = (
            "snit::type ::Foo { method bar {} {} }\n"
            "proc u {} { set o [Foo create %AUTO%]\n $o delegated_or_builtin }"
        )
        assert _diag_with_code(src, "W308") == []

    def test_namespaced_factory_object_no_w307(self):
        """A var assigned from a *namespaced* command substitution (a tcllib
        object/ensemble factory like ``::struct::tree`` / ``pt::rde``) holds an
        object command name; dispatching on it is object dispatch, not a stray
        non-literal command (analyser-only provenance — no type-lattice change,
        so no shimmer collateral)."""
        src = (
            "proc f {} {\n"
            "    set t [::struct::tree mytree]\n"
            "    $t walk root\n"
            "    set m [struct::matrix]\n"
            "    $m add row\n"
            "}"
        )
        assert _diag_with_code(src, "W307") == []

    def test_bare_unknown_command_var_still_w307(self):
        """A var from a *non-namespaced* unknown command substitution is not
        treated as an object factory — dispatch on it still warns."""
        src = "proc f {} { set x [foo bar]\n $x run }"
        assert len(_diag_with_code(src, "W307")) == 1

    def test_snit_component_dispatch_via_typeprivate_proc_no_w307(self):
        """A snit instance variable / component is an object handle; dispatch on
        it — including from a type-private ``proc`` that ``upvar``s it (the
        tcllib pt::/grammar:: idiom) — is object dispatch, not W307."""
        src = (
            "snit::type ::P {\n"
            "    variable myparser {}\n"
            "    constructor {} { set myparser [pt::rde ${selfns}::ENGINE] }\n"
            "    method run {} { $myparser reset }\n"
            "    proc helper {} {\n"
            "        upvar 1 myparser myparser\n"
            "        $myparser si:void_state_push\n"
            "    }\n"
            "}"
        )
        assert _diag_with_code(src, "W307") == []

    def test_snit_member_name_outside_type_still_w307(self):
        """The snit-member suppression is scoped to the type body — a same-named
        var dispatched in an unrelated proc still warns."""
        src = (
            "snit::type ::P { variable myparser {} \n method m {} { $myparser go } }\n"
            "proc unrelated {} { $myparser go }"
        )
        # exactly one W307 — the unrelated proc; the in-type dispatch is clean.
        diags = _diag_with_code(src, "W307")
        assert len(diags) == 1

    def test_tcloo_unknown_method_w308(self):
        """$obj nonexistent should emit W308 when method doesn't exist."""
        source = """\
oo::class create MyClass {
    method greet {name} {
        puts "hello $name"
    }
}
set obj [MyClass new]
$obj nonexistent_method
"""
        diags = _diag_with_code(source, "W308")
        assert len(diags) == 1
        assert "Unknown method" in diags[0].message
        assert "nonexistent_method" in diags[0].message

    def test_tcloo_destroy_is_valid(self):
        """$obj destroy should not emit W308 (built-in method)."""
        source = """\
oo::class create MyClass {
    method greet {} { puts hello }
}
set obj [MyClass new]
$obj destroy
"""
        diags = _diag_with_code(source, "W308")
        assert len(diags) == 0

    def test_external_class_new_no_w307(self):
        """$obj method should NOT emit W307 when obj was set from [ExternalClass new]."""
        source = """\
set circuit [Circuit new {Monte-Carlo}]
$circuit runAndRead
"""
        diags = _diag_with_code(source, "W307")
        assert len(diags) == 0

    def test_external_class_no_w308(self):
        """Methods on external classes should not emit W308."""
        source = """\
set circuit [Circuit new {Monte-Carlo}]
$circuit runAndRead
"""
        diags = _diag_with_code(source, "W308")
        assert len(diags) == 0

    def test_foreach_known_commands_no_w307(self):
        """$cmd in foreach over known built-in commands should NOT emit W307."""
        source = """\
foreach cmd [list puts set expr] {
    $cmd hello
}
"""
        diags = _diag_with_code(source, "W307")
        assert len(diags) == 0

    def test_foreach_braced_known_commands_no_w307(self):
        """$cmd in foreach over braced list of known commands should NOT emit W307."""
        source = """\
foreach cmd {puts set expr} {
    $cmd hello
}
"""
        diags = _diag_with_code(source, "W307")
        assert len(diags) == 0

    def test_foreach_known_procs_no_w307(self):
        """$cmd iterating over locally-defined procs should NOT emit W307."""
        source = """\
proc handler_a {x} { puts $x }
proc handler_b {x} { puts $x }
foreach cmd [list handler_a handler_b] {
    $cmd value
}
"""
        diags = _diag_with_code(source, "W307")
        assert len(diags) == 0

    def test_foreach_unknown_commands_still_w307(self):
        """$cmd iterating over unknown names should still emit W307."""
        source = """\
foreach cmd [list unknown_cmd_a unknown_cmd_b] {
    $cmd value
}
"""
        diags = _diag_with_code(source, "W307")
        assert len(diags) >= 1

    def test_foreach_oo_objects_no_w307(self):
        """$elem iterating over names of known TclOO objects suppresses W307."""
        source = """\
oo::class create Widget {
    method render {} { puts rendering }
}
set a [Widget new]
set b [Widget new]
foreach obj [list a b] {
    $obj render
}
"""
        diags = _diag_with_code(source, "W307")
        assert len(diags) == 0

    def test_foreach_var_mediated_list_no_w307(self):
        """$cmd from foreach over $cmds (known const list) suppresses W307."""
        source = """\
proc a {} {puts a}
proc b {} {puts b}
set cmds {a b}
foreach cmd $cmds {
    $cmd
}
"""
        diags = _diag_with_code(source, "W307")
        assert len(diags) == 0

    def test_foreach_list_cmd_var_no_w307(self):
        """$cmd from set items [list a b]; foreach cmd $items suppresses W307."""
        source = """\
proc x {} {puts x}
proc y {} {puts y}
set items [list x y]
foreach cmd $items {
    $cmd
}
"""
        diags = _diag_with_code(source, "W307")
        assert len(diags) == 0

    def test_interpolated_cmd_name_no_w123(self):
        """cmd_$var with all resolved names known suppresses W123."""
        source = """\
proc cmd_a {} {puts a}
proc cmd_b {} {puts b}
foreach cmd {a b} {
    cmd_$cmd
}
"""
        diags = _diag_with_code(source, "W123")
        assert len(diags) == 0

    def test_proc_return_constant_no_w307(self):
        """$h from [get_handler] where proc returns constant suppresses W307."""
        source = """\
proc get_handler {} { return puts }
set h [get_handler]
$h hello
"""
        diags = _diag_with_code(source, "W307")
        assert len(diags) == 0

    def test_proc_return_user_proc_no_w307(self):
        """$h from [get_handler] returning a user-defined proc suppresses W307."""
        source = """\
proc my_handler {x} { puts $x }
proc get_handler {} { return my_handler }
set h [get_handler]
$h hello
"""
        diags = _diag_with_code(source, "W307")
        assert len(diags) == 0

    def test_oo_class_name_as_command_no_w123(self):
        """oo::class create Foo makes 'Foo' a known command — no W123."""
        source = """\
oo::class create Logger {
    method add {msg} { puts $msg }
}
set log [Logger new]
"""
        diags = _diag_with_code(source, "W123")
        assert len(diags) == 0

    def test_snit_type_body_keywords_no_w123(self):
        """snit::type body DSL keywords (method/typemethod/delegate/…) are
        definition keywords, not undefined commands."""
        source = """\
snit::type Dog {
    option -name Fido
    component mouth
    method bark {} { return Woof }
    typemethod make {} {}
    constructor {args} {}
    destructor {}
    delegate method chew to mouth
}
"""
        assert len(_diag_with_code(source, "W123")) == 0

    def test_snit_body_real_unknown_still_w123(self):
        """The snit-body keyword suppression is scoped — a genuinely undefined
        command inside the body is still flagged."""
        source = "snit::type Dog {\n    method bark {} { genuinelyUndefinedCmd }\n}\n"
        diags = _diag_with_code(source, "W123")
        assert any("genuinelyUndefinedCmd" in d.message for d in diags)

    def test_method_without_snit_still_w123(self):
        """No global pollution: bare ``method`` in a non-snit file still W123."""
        source = "proc f {} { method foo }\n"
        diags = _diag_with_code(source, "W123")
        assert any("'method'" in d.message for d in diags)

    def test_w113_core_builtin_shadow_flagged(self):
        """Redefining a core global built-in (unqualified) is a real shadow."""
        assert len(_diag_with_code("proc set {a b} {}", "W113")) == 1
        assert len(_diag_with_code("proc exit {} {}", "W113")) == 1

    def test_w113_namespaced_library_command_not_flagged(self):
        """A namespace-qualified library command (``::base64::encode``,
        ``::snit::type``) is not a *built-in* — defining it in its own
        namespace must not be flagged as shadowing a built-in."""
        assert (
            _diag_with_code("namespace eval base64 { proc encode {s} {return $s} }", "W113") == []
        )
        assert _diag_with_code("proc ::snit::type {name def} {}", "W113") == []

    def test_oo_forward_method_no_w123(self):
        """Forward methods on known classes suppress W123."""
        source = """\
oo::class create Proxy {
    forward myput puts
}
set p [Proxy new]
$p myput hello
"""
        diags = _diag_with_code(source, "W123")
        assert len(diags) == 0

    def test_oo_mixin_method_no_w123(self):
        """Methods from mixins should be resolved — no W123 on class command."""
        source = """\
oo::class create Loggable {
    method log {msg} { puts $msg }
}
oo::class create Service {
    method run {} { puts running }
}
oo::define Service mixin Loggable
set svc [Service new]
$svc log hello
"""
        diags = _diag_with_code(source, "W123")
        assert len(diags) == 0

    def test_interpolated_handler_no_w123(self):
        """handler_$action where action is CONST resolves to known proc."""
        source = """\
proc handler_start {} { puts starting }
proc handler_stop {} { puts stopping }
set action start
handler_$action
"""
        diags = _diag_with_code(source, "W123")
        assert len(diags) == 0

    def test_chained_constructor_no_w307(self):
        """[ClassName new] method should NOT emit W307."""
        source = """\
oo::class create Dog {
    method bark {} { puts woof }
}
[Dog new] bark
"""
        diags = _diag_with_code(source, "W307")
        assert len(diags) == 0

    def test_chained_constructor_validates_method(self):
        """[ClassName new] nonexistent should emit W308."""
        source = """\
oo::class create Dog {
    method bark {} { puts woof }
}
[Dog new] nonexistent
"""
        diags = _diag_with_code(source, "W308")
        assert len(diags) == 1
        assert "nonexistent" in diags[0].message

    def test_method_unknown_suppresses_w308(self):
        """Classes with 'method unknown' should NOT emit W308."""
        source = """\
oo::class create Proxy {
    method unknown {name args} { puts $name }
}
set p [Proxy new]
$p anyMethod
$p anotherOne
"""
        diags = _diag_with_code(source, "W308")
        assert len(diags) == 0

    def test_namespace_ensemble_no_w123(self):
        """namespace ensemble create makes the namespace a known command."""
        source = """\
namespace eval mylib {
    namespace export create delete
    namespace ensemble create
    proc create {name} { puts $name }
}
mylib create foo
"""
        diags = _diag_with_code(source, "W123")
        assert len(diags) == 0

    def test_objdefine_suppresses_w308(self):
        """oo::objdefine adds methods — suppress W308 for modified objects."""
        source = """\
oo::class create Base {
    method existing {} { puts hello }
}
set obj [Base new]
oo::objdefine $obj method custom {} { puts custom }
$obj custom
"""
        diags = _diag_with_code(source, "W308")
        assert len(diags) == 0


class TestW307ProcParamDispatcher:
    """W307 suppression for proc parameters used as object dispatchers.

    When a proc explicitly dispatches on a parameter (``proc walk
    {tree} {[\\$tree leaves]; \\$tree visit \\$n}``), the user has
    documented the proc's API contract: it expects ``\\$tree`` to be
    an object handle.  Flagging W307 on those dispatches is noise.

    Detection: pre-compute per enclosing proc the set of var names
    used as dispatch heads in the body; suppress only when the var
    is BOTH a parameter of the enclosing proc AND in its dispatcher
    set.  Sound — the evidence (a dispatch in the body) lives in the
    proc itself.
    """

    def test_param_used_as_dispatcher_no_w307(self):
        # Sole-dispatch (one site) on a param: clear intent.
        src = "proc once {tree} { $tree visit }"
        assert _diag_with_code(src, "W307") == []

    def test_param_used_as_dispatcher_multi_no_w307(self):
        # tcllib struct::tree walker idiom.
        src = (
            "proc walk {tree} {\n    foreach n [$tree leaves] {\n        $tree visit $n\n    }\n}\n"
        )
        assert _diag_with_code(src, "W307") == []

    def test_local_var_dispatcher_not_param_still_w307(self):
        # TP control: a local that's a dispatcher but NOT a param of the
        # enclosing proc still fires.
        src = "proc f {} {\n    set foo [some_factory]\n    $foo method\n}"
        assert len(_diag_with_code(src, "W307")) == 1

    def test_qualified_proc_command_recognised(self):
        # ``::proc`` and ``proc`` are equivalent (both call ``::proc``).
        # tcllib's clay module qualifies it explicitly to bypass
        # overrides.  The proc must still register so its body's
        # multi-dispatch suppression sees the proc context.
        src = (
            "::proc ::clay::Ensemble {rawmethod args} {\n"
            "    set class [current_class]\n"
            "    $class clay set foo $a\n"
            "    $class clay set bar $b\n"
            "}\n"
        )
        assert _diag_with_code(src, "W307") == []

    def test_top_level_multi_dispatch_no_w307(self):
        # tcllib examples/irc/mainloop.tcl idiom: a top-level (no
        # enclosing proc) script registers an object handle and
        # dispatches on it many times.  Top-level scripts are real
        # code; the multi-dispatch evidence is just as strong as
        # inside a proc body — suppress when count ≥ 2.
        src = (
            "set cn [irc::connection]\n"
            "$cn connect server 6667\n"
            "$cn user nick localhost domain\n"
            "$cn nick foo\n"
        )
        assert _diag_with_code(src, "W307") == []

    def test_top_level_single_dispatch_still_w307(self):
        # TP control: a single top-level dispatch on a local doesn't
        # meet the multi-dispatch threshold.
        src = "set foo [factory]\n$foo method\n"
        assert len(_diag_with_code(src, "W307")) == 1

    def test_cross_proc_object_factory_no_w307(self):
        # tcllib struct/graphops idiom: ``createTGraph`` wraps the
        # namespaced ``struct::graph`` factory and returns the handle.
        # Callers do ``set TGraph [createTGraph ...]`` then dispatch on
        # ``\$TGraph`` — single dispatch, but the value came from a
        # known object-returning user proc.  The transitive factory
        # inference must propagate the OBJECT trait through the user
        # proc so a downstream SINGLE dispatch is silent.
        src = (
            "proc createTGraph {G T D} {\n"
            "    set TGraph [struct::graph]\n"
            "    return $TGraph\n"
            "}\n"
            "proc analyze {G T} {\n"
            "    set TG [createTGraph $G $T 0]\n"
            "    $TG dispose\n"
            "}\n"
        )
        assert _diag_with_code(src, "W307") == []

    def test_transitive_object_factory_chain_no_w307(self):
        # Chain: ``h`` calls ``g`` which calls ``f`` (the namespaced
        # factory).  The fixpoint must propagate the OBJECT trait up
        # the chain so callers of ``h`` are also recognised.
        src = (
            "proc f {} { return [struct::graph] }\n"
            "proc g {} { set x [f]; return $x }\n"
            "proc h {} { set y [g]; return $y }\n"
            "proc consumer {} { set z [h]; $z dispose }\n"
        )
        assert _diag_with_code(src, "W307") == []

    def test_non_factory_user_proc_still_w307(self):
        # TP control: a user proc returning a string (not an object)
        # — single-dispatch on the result still fires W307.
        src = "proc make_name {} { return foo }\nproc f {} { set x [make_name]; $x method }\n"
        assert len(_diag_with_code(src, "W307")) == 1

    def test_param_array_element_dispatch_no_w307(self):
        # ``proc f {Verify} { $Verify(default) ... }`` — Verify is a
        # parameter and the user dispatches on an element of that
        # array.  The param itself documents the callback-table
        # contract; the array-element form is just the indexed
        # callback registry.  tcllib tcltest's L602 ``$Verify($option)
        # [lindex $args 1]`` pattern uses this shape (a dispatch
        # table passed by the caller).
        src = (
            "proc f {Verify} {\n"
            "    if {[catch {$Verify(default) bar} value]} { return 0 }\n"
            "    return 1\n"
            "}\n"
        )
        assert _diag_with_code(src, "W307") == []

    def test_non_param_array_element_still_w307(self):
        # TP control: array-elem dispatch where the BASE is not a
        # parameter (here ``Verify`` is a namespace variable, not a
        # param) still fires.  The param-contract signal isn't
        # present.
        src = "proc f {} {\n    variable Verify\n    $Verify(default) arg\n}\n"
        assert len(_diag_with_code(src, "W307")) == 1

    def test_cmdsub_namespaced_ensemble_no_w307(self):
        # ``[[namespace parent]::outputChannel]`` — the same
        # namespaced-ensemble idiom as ``${log}::method`` but the
        # namespace prefix comes from a COMMAND substitution rather
        # than a variable.  ``[namespace parent]`` evaluates to the
        # parent namespace path, ``::outputChannel`` is the literal
        # method suffix.  Used in tcl9 tcltest.tcl L1663/1666 for
        # channel-test forwarding.
        src = (
            "proc f {channel} {\n"
            "    if {$channel in [list [[namespace parent]::outputChannel] stdout]} {\n"
            "        return ok\n"
            "    }\n"
            "}\n"
        )
        assert _diag_with_code(src, "W307") == []

    def test_eval_substituted_no_w307_dup(self):
        # ``eval \$cmd`` fires W101 (eval-injection) which is the
        # canonical, specific warning for double substitution.  The
        # generic W307 on the same site is redundant noise.  Dedup
        # via start-offset match (W101 and W307 ranges differ by 1
        # char at the end; start is identical).
        src = "proc f {} {\n    set cmd {puts hello}\n    eval $cmd\n}\n"
        codes = sorted(
            {d.code for d in _diag_with_code(src, "W101") + _diag_with_code(src, "W307")}
        )
        # W101 should be present; W307 should be deduped out.
        assert codes == ["W101"]

    def test_w307_still_fires_without_w101(self):
        # TP control: a non-eval dispatch (no W101 path) still fires
        # W307 — dedup only applies when both codes coincide.
        src = "proc f {} {\n    set obj [factory]\n    $obj method\n}\n"
        assert len(_diag_with_code(src, "W307")) == 1

    def test_param_used_only_as_data_param_dispatcher_unaffected(self):
        # TP control: a SEPARATE local dispatcher (dispatched ONCE) in the
        # same proc still fires; param-dispatcher suppression is scoped to
        # the param, and the multi-dispatch heuristic needs ≥2 sites.
        src = (
            "proc f {x} {\n"
            "    puts $x\n"  # x used only as data, never dispatched
            "    set y [some_factory]\n"
            "    $y method\n"  # y is dispatcher (single) but NOT a param
            "}\n"
        )
        assert len(_diag_with_code(src, "W307")) == 1

    def test_local_dispatched_multiple_times_no_w307(self):
        # tcllib struct/graphops idiom: ``set TGraph [createTGraph $G]``
        # then multiple dispatches on $TGraph in the same proc body.  Two+
        # uses of the same local as a dispatcher demonstrate firm intent
        # — the user designed it as an object handle.  Single dispatch
        # could be a typo; multi is unambiguous.  Suppress W307.
        src = (
            "proc analyze {G} {\n"
            "    set TGraph [createTGraph $G]\n"
            "    set first [$TGraph node first]\n"
            "    $TGraph node visit $first\n"
            "    $TGraph dispose\n"
            "}\n"
        )
        assert _diag_with_code(src, "W307") == []

    def test_local_dispatched_once_still_w307(self):
        # TP control: a local dispatched ONCE doesn't meet the multi-
        # dispatch threshold — still fires.  Distinguishes firm intent
        # from a possible typo / one-off.
        src = "proc f {} {\n    set foo [some_factory]\n    $foo method\n}\n"
        assert len(_diag_with_code(src, "W307")) == 1

    def test_tainted_multi_dispatch_still_w307(self):
        # SECURITY: a TAINTED var (eg from gets/exec/env/read) must NEVER
        # be suppressed even when dispatched multiple times — that IS
        # the command-injection vector the warning exists to flag.
        # Verified vs tclsh: ``\$usercmd`` IS executed as a command word,
        # so user-controlled command names are real injection risks.
        src = (
            "proc f {} {\n"
            "    set usercmd [gets stdin]\n"
            "    $usercmd action1\n"
            "    $usercmd action2\n"
            "}\n"
        )
        # Both dispatch sites should fire (taint reaches the dispatch).
        assert len(_diag_with_code(src, "W307")) == 2

    def test_tainted_exec_result_multi_dispatch_still_w307(self):
        src = (
            "proc f {arg} {\n"
            "    set cmd [exec which $arg]\n"
            "    $cmd --version\n"
            "    $cmd --help\n"
            "}\n"
        )
        assert len(_diag_with_code(src, "W307")) == 2

    def test_switch_style_array_callback_no_w307(self):
        # ``\$state(-command) \$token`` is the documented Tcl/Tk idiom for
        # configurable callbacks (widget ``-command``, http
        # ``-proxyfilter``, etc.).  The dash-prefixed array key marks the
        # element as an option the user explicitly registered as a
        # callback; flagging W307 is noise.  Verified against tclsh:
        # ``set state(-command) myProc; \$state(-command) arg`` is the
        # canonical option-as-callback pattern.
        src = "proc test {token} {\n    variable state\n    $state(-command) $token\n}\n"
        assert _diag_with_code(src, "W307") == []

    def test_array_element_without_switch_key_still_w307(self):
        # TP control: an array element whose key is NOT a switch-style
        # option and doesn't match the callback-suffix family
        # (cmd/command/callback/handler/hook/proc) is not the callback
        # idiom — still fires.  Keys like ``data`` or ``value`` clearly
        # don't name a registered command.
        src = "proc f {arg} {\n    variable arr\n    $arr(data) $arg\n}\n"
        assert len(_diag_with_code(src, "W307")) == 1

    def test_array_element_with_callback_suffix_no_w307(self):
        # Callback-suffix array elements (suffix ``cmd``/``command``/
        # ``callback``/``handler``/``hook``/``proc``) are documented
        # Tcl/Tk idioms for user-registered command callbacks
        # (``state(openCmd)``, ``state(doneCallback)``, ``state(myHandler)``).
        # Same principle as the dash-prefixed switch-style key — the
        # user explicitly assigned a command to this slot.  tcllib
        # http.tcl ``\$state(openCmd)``, tcltest ``\$CustomMatch($mode)``
        # use this pattern (where ``$mode``-keyed access still maps to
        # a callback registry).
        for key in ("openCmd", "doneCallback", "myHandler", "hookHook", "renderProc"):
            src = f"proc f {{arg}} {{\n    variable state\n    $state({key}) $arg\n}}\n"
            assert _diag_with_code(src, "W307") == [], (key, _diag_with_code(src, "W307"))

    def test_namespaced_ensemble_dispatch_no_w307(self):
        # ``\${log}::debug "msg"`` — tcllib logger / namespaced ensemble
        # idiom: the variable holds a namespace prefix, ``::method``
        # appended makes a qualified command path.  tcllib dns/spf/irc/
        # multiplexer/logger all use this shape.  The ``::`` suffix
        # after the variable substitution is a strong signal of
        # intentional namespaced command construction.
        src = (
            "namespace eval ::dns {\n"
            "    variable log\n"
            "    proc resolve {} {\n"
            "        variable log\n"
            '        ${log}::debug "msg"\n'
            '        ${log}::error "err"\n'
            "    }\n"
            "}\n"
        )
        assert _diag_with_code(src, "W307") == []

    def test_var_without_namespace_suffix_still_w307(self):
        # TP control: ``${cmd} arg`` (no ``::tail``) is not the
        # namespaced-ensemble idiom — still fires.
        src = "proc f {} {\n    set cmd [factory]\n    ${cmd} arg\n}\n"
        assert len(_diag_with_code(src, "W307")) == 1


class TestOOClassNameComposition:
    """FQ-name composition for OO/snit class definitions (tclsh 9.0.3 oracle):
    an absolute ``::Foo`` lives at the global root regardless of the enclosing
    namespace; a relative name resolves against the current namespace.  The old
    string concatenation doubled the ``::`` for absolute names (``::::Foo``) so
    the class was never matched on lookup (no W308), and used only the immediate
    scope name for relative ones (wrong for nested ``namespace eval``)."""

    def _classes(self, src: str):
        return sorted(analyse(src).all_classes.keys())

    def test_oo_class_absolute_name_no_double_colon(self):
        # `oo::class create ::Foo` records ::Foo (not ::::Foo), so the unknown
        # method on the instance is validated.  Was silently dropped before.
        assert self._classes("oo::class create ::Foo { method m {} {} }\n") == ["::Foo"]
        src = "oo::class create ::Foo { method m {} {} }\nproc f {} { set o [::Foo new]\n $o nosuch }\n"
        diags = _diag_with_code(src, "W308")
        assert len(diags) == 1
        assert "nosuch" in diags[0].message

    def test_oo_class_absolute_name_inside_namespace_eval(self):
        # An absolute name ignores the caller scope: recorded ::Top, not
        # ::ns::::Top, so the dispatch on the instance still validates.
        src = "namespace eval ns { oo::class create ::Top {} }\n"
        assert self._classes(src) == ["::Top"]
        disp = "namespace eval ns { oo::class create ::Top {} }\nproc f {} { set o [::Top new]\n $o m }\n"
        assert len(_diag_with_code(disp, "W308")) == 1

    def test_oo_class_relative_name_top_level_is_global(self):
        # Regression guard: a top-level relative name must stay ::Baz, not ::::Baz.
        assert self._classes("oo::class create Baz { method m {} {} }\n") == ["::Baz"]

    def test_oo_class_relative_name_resolves_enclosing_namespace(self):
        # `namespace eval ns { oo::class create Foo }` records ::ns::Foo, not the
        # former ::Foo.
        src = "namespace eval ns { oo::class create Foo { method m {} {} } }\n"
        assert self._classes(src) == ["::ns::Foo"]

    def test_oo_class_relative_dispatch_in_namespace_gets_w308(self):
        # A relative `[Foo new]` inside the namespace resolves to ::ns::Foo, so
        # the unknown method is validated (W308) and the dispatch is not flagged
        # as a non-literal command (no W307).  Requires namespace-aware object
        # typing in _return_type_for_command + _extract_class_names.
        src = (
            "namespace eval ns {\n"
            "  oo::class create Foo { method m {} {} }\n"
            "  proc use {} { set o [Foo new]\n $o nosuch }\n"
            "}\n"
        )
        assert len(_diag_with_code(src, "W308")) == 1
        assert _diag_with_code(src, "W307") == []

    def test_oo_class_relative_dispatch_valid_method_silent(self):
        # The opposing case: a real method on the relatively-named class must not
        # warn.
        src = (
            "namespace eval ns {\n"
            "  oo::class create Foo { method m {} {} }\n"
            "  proc use {} { set o [Foo new]\n $o m }\n"
            "}\n"
        )
        assert _diag_with_code(src, "W308") == []

    def test_oo_class_relative_name_nested_namespace(self):
        # Nested `namespace eval` must compose the full path, not just the
        # innermost scope name (the former bug recorded ::b::C).
        src = "namespace eval a { namespace eval b { oo::class create C {} } }\n"
        assert self._classes(src) == ["::a::b::C"]

    def test_oo_define_uses_same_qualified_name(self):
        # `oo::define` must resolve to the same FQ key as the `oo::class create`
        # so it augments the existing class rather than creating a phantom one.
        src = "namespace eval ns { oo::class create Foo {}\n oo::define Foo { method m {} {} } }\n"
        assert self._classes(src) == ["::ns::Foo"]

    def test_snit_type_qualified_name(self):
        # snit::type follows the same rules; absolute names are not doubled and
        # relative names resolve against the enclosing namespace.
        assert self._classes("namespace eval ns { snit::type Foo {} }\n") == ["::ns::Foo"]
        assert self._classes("namespace eval ns { snit::type ::Foo {} }\n") == ["::Foo"]


# W108: Non-ASCII token content


class TestNonAscii:
    """W108 -- tokens with non-standard ASCII characters."""

    def test_confusable_character_flagged(self):
        """Default 'confusables' mode flags smart quotes (copy-paste artifact)."""
        diags = _diag_with_code("set x \u201chello\u201d", "W108")
        assert len(diags) == 2  # left and right smart quotes

    def test_unicode_confusable_flagged(self):
        """Default 'confusables' mode flags Unicode homoglyphs (Cyrillic A)."""
        diags = _diag_with_code("set x \u0410bc", "W108")  # Cyrillic А looks like Latin A
        assert len(diags) == 1
        assert diags[0].fixes  # should have auto-fix to ASCII

    def test_benign_unicode_skipped_in_default_mode(self):
        """Default 'confusables' mode allows copyright symbol (not confusable)."""
        diags = _diag_with_code("set x \u00a9value", "W108")
        assert len(diags) == 0

    def test_degree_symbol_allowed_in_default_mode(self):
        """Default 'confusables' mode allows degree symbol (not confusable)."""
        diags = _diag_with_code("set x {90\u00b0}", "W108")
        assert len(diags) == 0

    def test_strict_mode_flags_all_non_ascii(self):
        from analyser.checks._style import _non_ascii_mode_var, set_non_ascii_mode

        prev = _non_ascii_mode_var.get()
        set_non_ascii_mode("strict")
        try:
            diags = _diag_with_code("set x \u00a9value", "W108")
            assert len(diags) == 1
            assert "ASCII" in diags[0].message
            assert diags[0].severity == Severity.WARNING
        finally:
            set_non_ascii_mode(prev)

    def test_common_mode_allows_symbols(self):
        from analyser.checks._style import _non_ascii_mode_var, set_non_ascii_mode

        prev = _non_ascii_mode_var.get()
        set_non_ascii_mode("common")
        try:
            # Degree symbol (scientific) should be allowed
            diags = _diag_with_code("set x {90\u00b0}", "W108")
            assert len(diags) == 0
            # But smart quotes (confusables) should still be flagged
            diags = _diag_with_code("set x \u201chello\u201d", "W108")
            assert len(diags) == 2
        finally:
            set_non_ascii_mode(prev)

    def test_off_mode_disables_w108(self):
        from analyser.checks._style import _non_ascii_mode_var, set_non_ascii_mode

        prev = _non_ascii_mode_var.get()
        set_non_ascii_mode("off")
        try:
            diags = _diag_with_code("set x \u201chello\u201d", "W108")
            assert len(diags) == 0
        finally:
            set_non_ascii_mode(prev)

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
        from compiler.compilation_unit import compile_source

        cu = compile_source(source)
        configure_signatures(dialect="f5-irules")
        result = analyse(source, cu=cu)
        return [d.code for d in result.diagnostics]

    @staticmethod
    def _diags_with_cu(source: str, code: str):
        from compiler.compilation_unit import compile_source

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

    def test_static_var_cross_event_no_false_w211(self):
        """static:: set in RULE_INIT and used in another event → no W211."""
        src = (
            "when RULE_INIT priority 500 {\n"
            "    set static::hdr_len 8\n"
            "}\n"
            "when SERVER_CONNECTED priority 500 {\n"
            "    TCP::collect $static::hdr_len\n"
            "}"
        )
        diags = self._diags_with_cu(src, "W211")
        hdr_diags = [d for d in diags if "hdr_len" in d.message]
        assert len(hdr_diags) == 0

    def test_static_var_unused_w211(self):
        """static:: set in RULE_INIT but never used anywhere → W211."""
        src = (
            "when RULE_INIT priority 500 {\n"
            "    set static::hdr_len 8\n"
            "}\n"
            "when SERVER_CONNECTED priority 500 {\n"
            "    TCP::collect 16\n"
            "}"
        )
        diags = self._diags_with_cu(src, "W211")
        hdr_diags = [d for d in diags if "hdr_len" in d.message]
        assert len(hdr_diags) == 1

    def test_static_var_set_outside_rule_init_no_w211(self):
        """static:: set in HTTP_REQUEST and read in HTTP_RESPONSE → no W211.

        The variable IS used cross-event; it's just racy (IRULE4005).
        """
        src = (
            "when HTTP_REQUEST priority 500 {\n"
            "    set static::token [HTTP::header Authorization]\n"
            "}\n"
            "when HTTP_RESPONSE priority 500 {\n"
            "    log local0. $static::token\n"
            "}"
        )
        diags = self._diags_with_cu(src, "W211")
        token_diags = [d for d in diags if "token" in d.message]
        assert len(token_diags) == 0

    def test_static_var_set_outside_rule_init_fires_irule4005(self):
        """static:: set in HTTP_REQUEST and read in HTTP_RESPONSE → IRULE4005."""
        src = (
            "when HTTP_REQUEST priority 500 {\n"
            "    set static::token [HTTP::header Authorization]\n"
            "}\n"
            "when HTTP_RESPONSE priority 500 {\n"
            "    log local0. $static::token\n"
            "}"
        )
        diags = self._diags_with_cu(src, "IRULE4005")
        token_diags = [d for d in diags if "token" in d.message]
        assert len(token_diags) == 1
        assert "race" in token_diags[0].message.lower()

    def test_static_var_in_rule_init_no_irule4005(self):
        """static:: set in RULE_INIT and used cross-event → no IRULE4005."""
        src = (
            "when RULE_INIT priority 500 {\n"
            "    set static::hdr_len 8\n"
            "}\n"
            "when SERVER_CONNECTED priority 500 {\n"
            "    TCP::collect $static::hdr_len\n"
            "}"
        )
        diags = self._diags_with_cu(src, "IRULE4005")
        assert len(diags) == 0

    def test_unset_static_var_not_treated_as_def(self):
        """unset static::x in RULE_INIT must not count as a cross-event definition.

        The unset should not suppress W211 for a real set in another event,
        and should not itself appear as an unused-set W211.
        """
        src = (
            "when RULE_INIT priority 500 {\n"
            "    unset -nocomplain -- static::old_cache\n"
            "}\n"
            "when HTTP_REQUEST priority 500 {\n"
            "    set static::old_cache [HTTP::uri]\n"
            "}"
        )
        # The set in HTTP_REQUEST is unused — unset in RULE_INIT must not
        # suppress W211 (unset is not a real definition for cross-event flow)
        w211_diags = self._diags_with_cu(src, "W211")
        cache_w211 = [d for d in w211_diags if "old_cache" in d.message]
        assert len(cache_w211) == 1

    def test_append_static_var_racy_irule4005(self):
        """append to static:: outside RULE_INIT and read cross-event → IRULE4005."""
        src = (
            "when HTTP_REQUEST priority 500 {\n"
            "    append static::log_buf [HTTP::uri]\n"
            "}\n"
            "when HTTP_RESPONSE priority 500 {\n"
            "    log local0. $static::log_buf\n"
            "}"
        )
        diags = self._diags_with_cu(src, "IRULE4005")
        buf_diags = [d for d in diags if "log_buf" in d.message]
        assert len(buf_diags) == 1

    def test_lappend_static_var_racy_irule4005(self):
        """lappend to static:: outside RULE_INIT and read cross-event → IRULE4005."""
        src = (
            "when HTTP_REQUEST priority 500 {\n"
            "    lappend static::uri_list [HTTP::uri]\n"
            "}\n"
            "when HTTP_RESPONSE priority 500 {\n"
            "    log local0. $static::uri_list\n"
            "}"
        )
        diags = self._diags_with_cu(src, "IRULE4005")
        list_diags = [d for d in diags if "uri_list" in d.message]
        assert len(list_diags) == 1

    def test_incr_static_var_racy_irule4005(self):
        """incr of static:: outside RULE_INIT and read cross-event → IRULE4005."""
        src = (
            "when HTTP_REQUEST priority 500 {\n"
            "    incr static::req_count\n"
            "}\n"
            "when HTTP_RESPONSE priority 500 {\n"
            "    log local0. $static::req_count\n"
            "}"
        )
        diags = self._diags_with_cu(src, "IRULE4005")
        count_diags = [d for d in diags if "req_count" in d.message]
        assert len(count_diags) == 1

    def test_array_set_static_var_racy_irule4005(self):
        """array set of static:: outside RULE_INIT and read cross-event → IRULE4005."""
        src = (
            "when HTTP_REQUEST priority 500 {\n"
            "    array set static::config {timeout 30}\n"
            "}\n"
            "when HTTP_RESPONSE priority 500 {\n"
            "    log local0. $static::config(timeout)\n"
            "}"
        )
        diags = self._diags_with_cu(src, "IRULE4005")
        config_diags = [d for d in diags if "config" in d.message]
        assert len(config_diags) == 1

    def test_multiple_racy_writers_across_events(self):
        """static::x written in both CLIENT_ACCEPTED and HTTP_REQUEST → IRULE4005 on both.

        CLIENT_ACCEPTED fires before HTTP_REQUEST/HTTP_RESPONSE, so
        cross-event flow from CLIENT_ACCEPTED to those events is valid.
        Both writers should get IRULE4005.
        """
        src = (
            "when CLIENT_ACCEPTED priority 500 {\n"
            "    set static::counter 1\n"
            "}\n"
            "when HTTP_REQUEST priority 500 {\n"
            "    set static::counter 2\n"
            "}\n"
            "when HTTP_RESPONSE priority 500 {\n"
            "    log local0. $static::counter\n"
            "}"
        )
        diags = self._diags_with_cu(src, "IRULE4005")
        counter_diags = [d for d in diags if "counter" in d.message]
        assert len(counter_diags) == 2

    def test_conditional_write_static_var_racy_irule4005(self):
        """static:: set inside if branch in non-RULE_INIT → still IRULE4005."""
        src = (
            "when HTTP_REQUEST priority 500 {\n"
            '    if {[HTTP::uri] eq "/debug"} {\n'
            "        set static::debug_mode 1\n"
            "    }\n"
            "}\n"
            "when HTTP_RESPONSE priority 500 {\n"
            "    log local0. $static::debug_mode\n"
            "}"
        )
        diags = self._diags_with_cu(src, "IRULE4005")
        debug_diags = [d for d in diags if "debug_mode" in d.message]
        assert len(debug_diags) == 1


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

    def test_interp_eval_list_idiom_clean(self):
        """interp eval $child [list ...] is the canonical safe pattern."""
        diags = _diag_with_code("interp eval $child [list set $varname $value]", "W312")
        assert len(diags) == 0


# W313: Destructive file operations with variable path (taint-aware)


class TestDestructiveFileOps:
    """W313 -- file delete/rename/mkdir with variable path.

    W313 now runs in the taint pipeline (compiler/taint/_sinks.py),
    so tests use _taint_diag_with_code() instead of _diag_with_code().
    """

    def test_file_delete_variable(self):
        """file delete with variable path → W313."""
        diags = _taint_diag_with_code("set path /tmp/x\nfile delete $path", "W313")
        assert len(diags) == 1
        assert "path-traversal" in diags[0].message.lower()

    def test_file_delete_literal_clean(self):
        """file delete with literal path → no W313."""
        diags = _taint_diag_with_code("file delete /tmp/myfile.txt", "W313")
        assert len(diags) == 0

    def test_file_rename_variable(self):
        """file rename with variable source → W313."""
        diags = _taint_diag_with_code("set src /tmp/x\nfile rename $src /tmp/dest", "W313")
        assert len(diags) == 1

    def test_file_mkdir_variable(self):
        """file mkdir with variable path → W313."""
        diags = _taint_diag_with_code("set dir /tmp/x\nfile mkdir $dir", "W313")
        assert len(diags) == 1

    def test_file_delete_force_variable(self):
        """file delete -force $path → W313 (skips -force flag)."""
        diags = _taint_diag_with_code("set path /tmp/x\nfile delete -force $path", "W313")
        assert len(diags) == 1

    def test_file_exists_clean(self):
        """file exists with variable → no W313 (not destructive)."""
        diags = _taint_diag_with_code("set path /tmp/x\nfile exists $path", "W313")
        assert len(diags) == 0

    def test_file_join_clean(self):
        """file join with variable → no W313 (not destructive)."""
        diags = _taint_diag_with_code("set base /tmp\nfile join $base subdir", "W313")
        assert len(diags) == 0

    def test_normalised_only_still_warns(self):
        """file delete with [file normalize] but no bounds check → still W313."""
        src = "set norm [file normalize $path]\nfile delete $norm"
        diags = _taint_diag_with_code(src, "W313")
        assert len(diags) == 1
        assert "normalised" in diags[0].message.lower() or "verify" in diags[0].message.lower()

    def test_normalised_and_bounded_suppressed(self):
        """file delete with normalise + bounds check → no W313."""
        src = (
            "set norm [file normalize $path]\n"
            'if {![string match "$base/*" $norm]} { error outside }\n'
            "file delete $norm"
        )
        diags = _taint_diag_with_code(src, "W313")
        assert len(diags) == 0

    def test_bounded_inside_true_branch(self):
        """file delete inside guarded true branch → no W313."""
        src = (
            "set norm [file normalize $path]\n"
            'if {[string match "/safe/*" $norm]} {\n'
            "    file delete $norm\n"
            "}"
        )
        diags = _taint_diag_with_code(src, "W313")
        assert len(diags) == 0

    def test_bounded_does_not_leak_to_false_branch(self):
        """file delete in false branch of guard → still W313."""
        src = (
            "set norm [file normalize $path]\n"
            'if {[string match "/safe/*" $norm]} {\n'
            "    puts safe\n"
            "} else {\n"
            "    file delete $norm\n"
            "}"
        )
        diags = _taint_diag_with_code(src, "W313")
        assert len(diags) == 1

    def test_unnormalised_path_warns(self):
        """file delete without normalisation → W313."""
        src = "set joined [file join $base $name]\nfile delete $joined"
        diags = _taint_diag_with_code(src, "W313")
        assert len(diags) == 1


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


def _ip_diags(source: str) -> list:
    """Return all W122 or W124 diagnostics (IP validation from either pass)."""
    result = analyse(source)
    return [d for d in result.diagnostics if d.code in ("W122", "W124")]


class TestMistypedIPv4:
    """W122/W124 -- dotted-quad with out-of-range octets or leading zeros."""

    def test_octet_over_255(self):
        """192.168.1.256 has octet > 255 → W122 or W124."""
        diags = _ip_diags("set addr 192.168.1.256")
        assert len(diags) == 1
        assert "exceeds 255" in diags[0].message
        assert diags[0].severity == Severity.ERROR

    def test_leading_zero(self):
        """192.168.01.1 has leading zero → W122 or W124."""
        diags = _ip_diags("set addr 192.168.01.1")
        assert len(diags) == 1
        assert "leading zero" in diags[0].message
        assert diags[0].severity == Severity.WARNING

    def test_valid_ip_clean(self):
        """192.168.1.1 is valid → no W122/W124."""
        diags = _ip_diags("set addr 192.168.1.1")
        assert len(diags) == 0

    def test_valid_ip_10_0_0_1_clean(self):
        """10.0.0.1 is valid → no W122/W124."""
        diags = _ip_diags("set addr 10.0.0.1")
        assert len(diags) == 0

    def test_octet_999(self):
        """10.0.0.999 has octet > 255 → W122 or W124."""
        diags = _ip_diags("set addr 10.0.0.999")
        assert len(diags) == 1

    def test_ldap_oid_no_w122(self):
        """LDAP OID ``1.3.6.1.4.1.4203.1.11.3`` is not an IPv4 — must
        not fire W122/W124.  The validator was matching the substring
        ``1.4.1.4203`` and complaining about octet 4203.  Detect that
        the matched quad is part of a longer dotted chain (preceded
        OR followed by ``.<digit>``) and skip.  Verified against the
        tcllib ldap.tcl ``Whoami`` proc which uses this exact OID.
        """
        diags = _ip_diags("set oid 1.3.6.1.4.1.4203.1.11.3")
        assert diags == [], diags

    def test_snmp_oid_no_w122(self):
        """SNMP sysName.0 OID ``1.3.6.1.2.1.1.5.0`` — same pattern,
        all octets are small but the regex still matches embedded
        quads.  Must not fire."""
        diags = _ip_diags("set oid 1.3.6.1.2.1.1.5.0")
        assert diags == [], diags

    def test_all_zeros_clean(self):
        """0.0.0.0 is valid → no W122/W124."""
        diags = _ip_diags("set addr 0.0.0.0")
        assert len(diags) == 0

    def test_leading_zero_non_octal_digit(self):
        """192.168.09.1 has leading zero but '9' is not octal → no W122/W124."""
        diags = _ip_diags("set addr 192.168.09.1")
        assert len(diags) == 0

    def test_leading_zero_08_not_octal(self):
        """10.0.08.1 — '08' contains '8' which is not octal → no W122/W124."""
        diags = _ip_diags("set addr 10.0.08.1")
        assert len(diags) == 0

    def test_leading_zero_099_not_octal(self):
        """192.168.099.1 — '099' contains '9' → no W122/W124."""
        diags = _ip_diags("set addr 192.168.099.1")
        assert len(diags) == 0

    def test_leading_zero_octal_ambiguous(self):
        """192.168.077.1 — '077' is valid octal → W122 or W124."""
        diags = _ip_diags("set addr 192.168.077.1")
        assert len(diags) == 1
        assert "leading zero" in diags[0].message

    def test_leading_zero_00_octal_ambiguous(self):
        """10.00.0.1 — '00' is valid octal → W122 or W124."""
        diags = _ip_diags("set addr 10.00.0.1")
        assert len(diags) == 1
        assert "leading zero" in diags[0].message

    def test_valid_mask_clean(self):
        """255.255.255.0 is valid → no W122/W124."""
        diags = _ip_diags("set mask 255.255.255.0")
        assert len(diags) == 0

    def test_version_number_after_slash_not_flagged(self):
        """Chrome/28.0.1550.0 is a version, not an IP → no W122/W124."""
        diags = _ip_diags('set ua "Mozilla/5.0 Chrome/28.0.1550.0 Safari/537.36"')
        assert len(diags) == 0

    def test_range_covers_only_quad(self):
        """IP diagnostic range should be reasonable, not the whole file."""
        diags = _ip_diags('set x "prefix 192.168.1.999 suffix"')
        assert len(diags) == 1

    def test_version_number_in_user_agent_string(self):
        """Realistic User-Agent string with version numbers → no W122/W124."""
        source = (
            'HSL::send $hsl "GET / HTTP/1.0\\r\\n'
            "User-Agent: Iron/28.0.1550.0 Chrome/28.0.1550.0\\r\\n\\r\\n"
            '"'
        )
        diags = _ip_diags(source)
        assert len(diags) == 0


# W124: SSA-traced IP address validation


class TestSSATracedIPValidation:
    """W124 -- CFG-SSA pass that validates IP addresses via SCCP constants."""

    def test_ipv4_bad_octet_in_proc(self):
        """IPv4 address with octet > 255 inside a proc → W124."""
        diags = _diag_with_code("proc f {} { set a 192.168.1.256 }", "W124")
        assert len(diags) == 1
        assert "exceeds 255" in diags[0].message
        assert diags[0].severity == Severity.ERROR

    def test_ipv4_leading_zero_in_proc(self):
        """IPv4 address with leading zero inside a proc → W124."""
        diags = _diag_with_code("proc f {} { set a 192.168.01.1 }", "W124")
        assert len(diags) == 1
        assert "leading zero" in diags[0].message
        assert diags[0].severity == Severity.WARNING

    def test_valid_ipv4_clean(self):
        """Valid IPv4 inside a proc → no W124."""
        diags = _diag_with_code("proc f {} { set a 10.0.0.1 }", "W124")
        assert len(diags) == 0

    def test_version_number_not_flagged(self):
        """Version number in User-Agent string → no W124."""
        diags = _diag_with_code('proc f {} { set ua "Chrome/28.0.1550.0" }', "W124")
        assert len(diags) == 0

    def test_variable_trace_with_use(self):
        """Bad IP traced to use site → W124 with related_ranges."""
        source = "proc f {} {\n    set a 10.0.0.999\n    puts $a\n}"
        diags = _diag_with_code(source, "W124")
        assert len(diags) == 1
        assert diags[0].related_ranges  # at least one use site

    def test_w122_suppressed_when_w124_fires(self):
        """W122 should be suppressed on the same line where W124 fires."""
        source = "proc f {} { set a 192.168.1.256 }"
        result = analyse(source)
        w122 = [d for d in result.diagnostics if d.code == "W122"]
        w124 = [d for d in result.diagnostics if d.code == "W124"]
        assert len(w124) == 1
        # W122 should be suppressed (deduped) on the same line
        for d122 in w122:
            assert d122.range.start.line != w124[0].range.start.line

    def test_top_level_gets_w124(self):
        """Top-level bad IP gets W124 (SSA analysis covers top-level too)."""
        diags = _diag_with_code("set a 192.168.1.256", "W124")
        assert len(diags) == 1


# W126: Channel type validation


class TestChannelValidation:
    """W126 -- non-channel value passed to channel argument position."""

    def test_valid_open_channel(self):
        """Variable from open is CHANNEL type → no W126."""
        diags = _diag_with_code(
            "proc f {} { set fd [open /tmp/t r]; gets $fd line; close $fd }",
            "W126",
        )
        assert len(diags) == 0

    def test_valid_socket_channel(self):
        """Variable from socket is CHANNEL type → no W126."""
        diags = _diag_with_code(
            "proc f {} { set s [socket localhost 80]; close $s }",
            "W126",
        )
        assert len(diags) == 0

    def test_valid_stdout(self):
        """stdout is a standard channel → no W126."""
        diags = _diag_with_code("proc f {} { puts stdout hello }", "W126")
        assert len(diags) == 0

    def test_valid_stderr(self):
        """stderr is a standard channel → no W126."""
        diags = _diag_with_code("proc f {} { puts stderr hello }", "W126")
        assert len(diags) == 0

    def test_int_var_as_channel(self):
        """INT variable used as channel → W126."""
        diags = _diag_with_code("proc f {} { set x 42; close $x }", "W126")
        assert len(diags) == 1
        assert "INT" in diags[0].message

    def test_literal_non_channel(self):
        """String literal that isn't stdout/stderr/stdin → W126."""
        diags = _diag_with_code("proc f {} { gets notachannel line }", "W126")
        assert len(diags) == 1
        assert "notachannel" in diags[0].message

    def test_overdefined_no_warn(self):
        """OVERDEFINED variable (e.g. from proc param) → no W126 (conservative)."""
        diags = _diag_with_code("proc f {fd} { close $fd }", "W126")
        assert len(diags) == 0

    def test_string_typed_channel_no_warn(self):
        """A channel handle is an opaque STRING; a channel round-tripped through
        a var/array element types as STRING and must NOT be flagged (the value
        can be a real channel — e.g. http's `set sock $state(sock)`)."""
        src = "proc f {token} { upvar 0 $token state; set sock $state(sock); close $sock }"
        assert _diag_with_code(src, "W126") == []

    def test_list_var_as_channel_still_warns(self):
        """A LIST cannot be a channel handle → still flagged."""
        diags = _diag_with_code("proc f {} { set l [list a b]; close $l }", "W126")
        assert len(diags) == 1
        assert "LIST" in diags[0].message


# W125: Orphaned control-flow keyword


class TestOrphanedKeyword:
    """W125 — control-flow keywords used as standalone commands."""

    def test_orphaned_else(self):
        """else on separate line from closing } → W125."""
        source = "if {1} {\n    puts yes\n}\nelse {\n    puts no\n}\n"
        diags = _diag_with_code(source, "W125")
        assert len(diags) == 1
        assert '"else"' in diags[0].message
        assert '"if"' in diags[0].message

    def test_orphaned_elseif(self):
        """elseif on separate line → W125."""
        source = "if {1} {\n    puts a\n}\nelseif {0} {\n    puts b\n}\n"
        diags = _diag_with_code(source, "W125")
        assert len(diags) == 1
        assert '"elseif"' in diags[0].message

    def test_orphaned_then(self):
        """then on separate line → W125."""
        source = "if {1}\nthen {\n    puts yes\n}\n"
        diags = _diag_with_code(source, "W125")
        assert len(diags) == 1
        assert '"then"' in diags[0].message

    def test_orphaned_finally(self):
        """finally on separate line → W125."""
        source = "try {\n    error oops\n}\nfinally {\n    puts cleanup\n}\n"
        diags = _diag_with_code(source, "W125")
        assert len(diags) == 1
        assert '"finally"' in diags[0].message

    def test_orphaned_on(self):
        """on error handler on separate line → W125."""
        source = "try {\n    error oops\n}\non error {msg opts} {\n    puts $msg\n}\n"
        diags = _diag_with_code(source, "W125")
        assert len(diags) == 1
        assert '"on"' in diags[0].message

    def test_orphaned_trap(self):
        """trap handler on separate line → W125."""
        source = "try {\n    error oops\n}\ntrap {} e {\n    puts caught\n}\n"
        diags = _diag_with_code(source, "W125")
        assert len(diags) == 1
        assert '"trap"' in diags[0].message

    def test_correct_else_no_warning(self):
        """} else { on same line → no W125."""
        source = "if {1} {\n    puts yes\n} else {\n    puts no\n}\n"
        diags = _diag_with_code(source, "W125")
        assert len(diags) == 0

    def test_correct_elseif_no_warning(self):
        """} elseif { on same line → no W125."""
        source = "if {1} {\n    puts a\n} elseif {0} {\n    puts b\n}\n"
        diags = _diag_with_code(source, "W125")
        assert len(diags) == 0

    def test_correct_finally_no_warning(self):
        """} finally { on same line → no W125."""
        source = "try {\n    error oops\n} finally {\n    puts cleanup\n}\n"
        diags = _diag_with_code(source, "W125")
        assert len(diags) == 0

    def test_proc_else_suppresses(self):
        """User-defined proc named else → no W125."""
        source = "proc else {x} { puts $x }\nelse hello\n"
        diags = _diag_with_code(source, "W125")
        assert len(diags) == 0

    def test_proc_finally_suppresses(self):
        """User-defined proc named finally → no W125."""
        source = "proc finally {body} { uplevel 1 $body }\nfinally { puts done }\n"
        diags = _diag_with_code(source, "W125")
        assert len(diags) == 0

    def test_multiple_orphaned(self):
        """Multiple orphaned keywords in one file → multiple W125."""
        source = "if {1} {\n    puts a\n}\nelseif {0} {\n    puts b\n}\nelse {\n    puts c\n}\n"
        diags = _diag_with_code(source, "W125")
        assert len(diags) == 2
        codes = {d.message.split('"')[1] for d in diags}
        assert codes == {"elseif", "else"}


class TestStructuralBodyDoesNotLeakIntoOuterScope:
    """Issue #250 — OO / snit / uri::register bodies are STRUCTURAL.

    A structural body must not contribute reads/writes to the enclosing
    proc's data flow.  These tests use a deliberately ambiguous proc
    that, if the body bled through, would silence W210 (read before
    set), W211 (set but never used), and W214 (unused parameter) — so
    the presence of all three confirms the body was excluded from the
    outer scope's SSA scan.
    """

    @staticmethod
    def _outer_with_structural_body(structural_call: str) -> str:
        return f"proc outer {{p}} {{\n    set x 1\n    puts $y\n    {structural_call}\n}}\n"

    def _assert_outer_diagnostics_fire(self, structural_call: str) -> None:
        source = self._outer_with_structural_body(structural_call)
        result = analyse(source)
        codes = {d.code for d in result.diagnostics}
        assert "W210" in codes, f"missing W210 for outer $y; got {sorted(codes)}"
        assert "W211" in codes, f"missing W211 for outer set x; got {sorted(codes)}"
        assert "W214" in codes, f"missing W214 for outer param p; got {sorted(codes)}"

    def test_oo_class_body_does_not_leak(self):
        self._assert_outer_diagnostics_fire(
            "oo::class create C { method m {} { puts $x; set y 1; set p 2 } }"
        )

    def test_oo_define_body_does_not_leak(self):
        self._assert_outer_diagnostics_fire(
            "oo::define C { method m {} { puts $x; set y 1; set p 2 } }"
        )

    def test_oo_objdefine_method_does_not_leak(self):
        self._assert_outer_diagnostics_fire(
            "oo::objdefine C method m {} { puts $x; set y 1; set p 2 }"
        )

    def test_snit_type_body_does_not_leak(self):
        self._assert_outer_diagnostics_fire(
            "snit::type T { method m {} { puts $x; set y 1; set p 2 } }"
        )

    def test_snit_method_body_does_not_leak(self):
        self._assert_outer_diagnostics_fire("snit::method T m {} { puts $x; set y 1; set p 2 }")

    def test_uri_register_body_does_not_leak(self):
        self._assert_outer_diagnostics_fire("uri::register demo { puts $x; set y 1; set p 2 }")


class TestSwitchSubjectCountsAsParamUse:
    """Issue #471 — a parameter read only as a ``switch`` subject must not
    be reported as an unused parameter (W214).

    An exact ``switch -- $col {...}`` lowers to a chain of CFG branch
    conditions whose subject is preserved as an ``ExprRaw`` node; the read
    of ``col`` must still be recognised as a use.
    """

    def test_switch_subject_param_not_unused(self):
        source = (
            "proc editStartCmd {tbl row col text} {\n"
            "    switch -- $col {\n"
            "        1 { set combList columns }\n"
            "        default { return $text }\n"
            "    }\n"
            "    return $combList\n"
            "}\n"
        )
        result = analyse(source)
        unused = {d.message.split("'")[1] for d in result.diagnostics if d.code == "W214"}
        assert "col" not in unused, f"col wrongly flagged unused; got {sorted(unused)}"
        # ``row`` is genuinely unused — the check still fires for it.
        assert "row" in unused, f"expected row to be unused; got {sorted(unused)}"

    @staticmethod
    def _w214(source: str) -> set[str]:
        return {d.message.split("'")[1] for d in analyse(source).diagnostics if d.code == "W214"}

    def test_glob_switch_subject_and_body_reads(self):
        # -glob and fallthrough switches are kept as an IRSwitch statement
        # (never lowered into CFG branches), so the subject and arm-body
        # reads must be recovered from the structured form.  (-regexp is a
        # barrier handled separately — see PR #474.)
        source = (
            "proc p {col text} {\n"
            "    switch -glob -- $col {\n"
            "        a* { return $text }\n"
            "        default { return 0 }\n"
            "    }\n"
            "}\n"
        )
        assert "col" not in self._w214(source)
        assert "text" not in self._w214(source)

    def test_fallthrough_switch_body_reads(self):
        source = (
            "proc p {col text} {\n"
            "    switch -- $col {\n"
            "        a -\n"
            "        b { return $text }\n"
            "        default { return 0 }\n"
            "    }\n"
            "}\n"
        )
        assert "col" not in self._w214(source)
        assert "text" not in self._w214(source)

    def test_nested_control_flow_in_switch_arm_read(self):
        # Reads buried in nested if/loop bodies inside an un-lowered switch
        # arm must still be discovered.
        source = (
            "proc p {col text} {\n"
            "    switch -glob -- $col {\n"
            "        a* { if {1} { foreach x {1 2} { puts $text } } }\n"
            "        default { return 0 }\n"
            "    }\n"
            "}\n"
        )
        assert "text" not in self._w214(source)

    def test_arm_local_set_then_read_no_read_before_set(self):
        # A temporary set then read *inside* an arm is arm-local; collecting
        # its read onto the outer IRSwitch must not surface as W210
        # read-before-set (the outer statement has no view of the arm's def).
        source = (
            "proc p {x} {\n"
            "    switch -glob -- $x {\n"
            "        a* { set tmp 1; puts $tmp }\n"
            "        default { return 0 }\n"
            "    }\n"
            "}\n"
        )
        codes = {d.code for d in analyse(source).diagnostics}
        assert "W210" not in codes, f"unexpected read-before-set; got {sorted(codes)}"

    def test_arm_for_init_var_no_read_before_set(self):
        # A `for` loop variable set in the init clause inside a switch arm must
        # not surface as read-before-set (init defs were previously missed).
        source = (
            "proc p {id} {\n"
            "    switch -glob -- $id {\n"
            "        a* { for {set j 0} {$j < 3} {incr j} { puts $j } }\n"
            "        default { return 0 }\n"
            "    }\n"
            "}\n"
        )
        codes = {d.code for d in analyse(source).diagnostics}
        assert "W210" not in codes, f"unexpected read-before-set; got {sorted(codes)}"

    def test_arm_regexp_output_var_no_read_before_set(self):
        # regexp output vars bound in a condition command-sub inside an arm.
        source = (
            "proc p {id} {\n"
            "    switch -glob -- $id {\n"
            "        a* { if {[regexp {(.)-(.*)} $id -> v rest]} { puts $v$rest } }\n"
            "        default { return 0 }\n"
            "    }\n"
            "}\n"
        )
        codes = {d.code for d in analyse(source).diagnostics}
        assert "W210" not in codes, f"unexpected read-before-set; got {sorted(codes)}"


class TestCallByNameViaCommandSubstitution:
    """The dominant call-by-name shape in tcllib is a literal-name arg
    inside a ``[..]`` substitution: ``set len [asnPeekTag data tag type
    dummy]`` (asn module).  ``tag``/``type`` are caller-locals consumed
    indirectly via ``upvar`` inside ``asnPeekTag`` — must not trigger
    W211/W220 on their initialisers.  The earlier call-by-name fix only
    scanned top-level ``IRCall`` statements and missed this shape.
    """

    @staticmethod
    def _diags(src, code):
        from server.features.diagnostics import get_diagnostics

        return [d for d in get_diagnostics(src) if d.code == code]

    def test_call_by_name_inside_command_subst_silences_w211(self):
        src = (
            "namespace eval ::asn {\n"
            "    proc asnPeekTag {data_var tag_var tagtype_var constr_var} {\n"
            "        upvar 1 $data_var data $tag_var tag "
            "$tagtype_var tagtype $constr_var constr\n"
            "        set tag 1; set tagtype 0x02\n"
            "    }\n"
            "    proc asnRetag {data_var newTag} {\n"
            "        upvar 1 $data_var data\n"
            '        set tag ""\n'
            '        set type ""\n'
            "        set len [asnPeekTag data tag type dummy]\n"
            "        puts $len\n"
            "    }\n"
            "}\n"
        )
        w211 = [d.message for d in self._diags(src, "W211")]
        w220 = [d.message for d in self._diags(src, "W220")]
        assert not any("'tag' is set" in m for m in w211), w211
        assert not any("'type' is set" in m for m in w211), w211
        assert not any("'tag' is never read" in m for m in w220), w220
        assert not any("'type' is never read" in m for m in w220), w220

    def test_genuine_unused_outside_call_by_name_still_fires(self):
        # TP control: a vestigial init NOT passed by name still fires.
        src = (
            "proc reader {nameVar} { upvar 1 $nameVar local; return $local }\n"
            "proc f {} {\n"
            '    set used "hello"\n'
            '    set unused "discarded"\n'
            "    return [reader used]\n"
            "}\n"
        )
        w211 = [d.message for d in self._diags(src, "W211")]
        assert any("'unused'" in m for m in w211)


class TestW214EmptyBodyStubs:
    """``proc foo {a b} {}`` with an empty body is a SIGNATURE STUB
    declaring an API — the implementation lives elsewhere (eg
    ``grammar_fa/faop.tcl`` declares the full FA algebra as 14 empty
    procs).  Every parameter is necessarily "unused" because there is
    no body to use it; flagging them is pure noise.

    tclsh ground truth: an empty-body proc is a valid placeholder
    callable that simply returns the empty string.  Production
    codebases use this pattern to seed the proc table that overlay
    files later populate.
    """

    @staticmethod
    def _w214(src):
        import re as _re

        from server.features.diagnostics import get_diagnostics

        return {
            m.group(1)
            for d in get_diagnostics(src)
            if d.code == "W214" and (m := _re.search(r"'(\w+)'", d.message)) is not None
        }

    def test_empty_body_silences_w214(self):
        # tcllib faop.tcl idiom.
        src = "proc reverse {fa} {}\n"
        assert self._w214(src) == set()

    def test_empty_body_with_defaults_silences_w214(self):
        src = "proc complete {fa {sink {}}} {}\n"
        assert self._w214(src) == set()

    def test_real_body_still_fires_w214(self):
        src = "proc foo {x y} { puts hello }\n"
        assert self._w214(src) == {"x", "y"}


class TestW214QuotedKeywordMarker:
    """snit-style positional keyword markers like ``{"as" ""}`` declare
    a param whose NAME is the quoted literal — the caller must write
    ``expose mycomp as foo`` and the keyword ``as`` is captured by the
    placeholder param.  The body cannot meaningfully USE such a
    parameter (``$\"as\"`` is not even valid syntax), so flagging is
    pure noise.  tcllib snit::Comp.statement.expose is the canonical
    example.
    """

    @staticmethod
    def _w214_names(src):
        import re as _re

        from server.features.diagnostics import get_diagnostics

        return {
            m.group(1)
            for d in get_diagnostics(src)
            if d.code == "W214"
            and (m := _re.search(r"Parameter '(.*?)' of", d.message)) is not None
        }

    def test_quoted_keyword_marker_silenced(self):
        src = (
            'proc ::snit::expose {component {"as" ""} {methodname ""}} {\n'
            "    return $component$methodname\n"
            "}\n"
        )
        # ``"as"`` is the keyword marker — must not fire.
        assert '"as"' not in self._w214_names(src)

    def test_normal_unused_param_alongside_marker_still_fires(self):
        # TP control: ordinary unused params alongside a marker still flag.
        src = 'proc ::ns::foo {component {"as" ""} unused} {\n    return $component\n}\n'
        names = self._w214_names(src)
        assert "unused" in names
        assert '"as"' not in names


class TestDispatchProtocolSuppression:
    """W214 dispatch-protocol suppression: when ≥3 peer procs in the same
    namespace share an identical leading-param signature, those params are
    a dispatcher contract (parser-rule visitor, snit method protocol,
    tcllib pt::peg::from::peg::GEN::* rules with ``{s e}``).  Firing W214
    on every rule that doesn't read its protocol params is noise.

    Sound under-approximation: a protocol is only inferred when ≥3 peers
    carry it.  A unique signature in the namespace still fires (it's a
    real proc with genuinely unused args).
    """

    @staticmethod
    def _w214(source: str) -> set[str]:
        from analyser import analyse

        return {d.message.split("'")[1] for d in analyse(source).diagnostics if d.code == "W214"}

    def test_three_peer_protocol_suppresses_W214(self):
        # Mirrors the tcllib pt::peg::from::peg::GEN::* parser-rule visitor.
        # ALNUM/ALPHA/DIGIT all accept ``{s e}``; ≥3 peers → protocol.
        src = (
            "namespace eval ::g {\n"
            "    proc ALNUM {s e} { return alnum }\n"
            "    proc ALPHA {s e} { return alpha }\n"
            "    proc DIGIT {s e} { return digit }\n"
            "}\n"
        )
        assert self._w214(src) == set()

    def test_two_peer_signature_does_not_form_protocol(self):
        # Only 2 peers — not enough to infer a dispatcher contract; each
        # rule's unused args still fire as genuine W214.
        src = (
            "namespace eval ::g {\n"
            "    proc rule_a {s e} { return alnum }\n"
            "    proc rule_b {s e} { return alpha }\n"
            "}\n"
        )
        # Both ``s`` and ``e`` unused in both procs → 4 fires.
        assert self._w214(src) == {"s", "e"}

    def test_extra_param_beyond_protocol_still_fires(self):
        # The protocol part is suppressed but extra unused params past the
        # protocol still fire — the contract only covers the shared prefix.
        src = (
            "namespace eval ::g {\n"
            "    proc ALNUM {s e} { return alnum }\n"
            "    proc ALPHA {s e} { return alpha }\n"
            "    proc DIGIT {s e} { return digit }\n"
            "    proc SPECIAL {s e weird} { return $s$e }\n"
            "}\n"
        )
        assert self._w214(src) == {"weird"}

    def test_single_proc_still_fires(self):
        # A unique signature in the namespace is not part of any protocol.
        src = "proc foo {x y unused} { puts $x; puts $y }\n"
        assert self._w214(src) == {"unused"}

    def test_args_variadic_excluded_from_protocol_shape(self):
        # ``args`` is Tcl's variadic catch-all — never part of the protocol
        # shape, so peers that all share ``{s e}`` form a protocol even when
        # SOME of them additionally have ``args``.
        src = (
            "namespace eval ::g {\n"
            "    proc ALNUM {s e} { return alnum }\n"
            "    proc ALPHA {s e} { return alpha }\n"
            "    proc DIGIT {s e args} { return [lindex $args 0] }\n"
            "}\n"
        )
        # All three procs share the leading ``{s e}``; protocol applies.
        # No unused (each body either ignores or uses the args).
        assert self._w214(src) == set()


class TestExprSubstitutionReadTracking:
    """Reads inside command-substituted exprs (``incr i [expr {$w}]``) must be
    recognised so the assignment feeding them is not falsely flagged dead /
    unused (the vars live inside expr ``{...}``, opaque to the word scanner)."""

    def test_incr_amount_expr_read_not_dead_store(self):
        src = "proc f {} { set w 5; set i 0; incr i [expr {$w}]; return $i }"
        assert _diag_with_code(src, "W220") == []  # 'w' is read
        assert _diag_with_code(src, "W211") == []

    def test_var_read_only_in_cmdsub_expr_not_unused(self):
        src = "proc f {n} { set scale 3; return [expr {$n * $scale}] }"
        assert _diag_with_code(src, "W211") == []
        assert _diag_with_code(src, "W220") == []

    def test_genuine_dead_store_still_flagged(self):
        # No expr-substitution read of y: the first assignment is truly dead and
        # must still be reported (the fix only suppresses; must not over-suppress).
        src = "proc f {} { set y 1; set y 2; return $y }"
        assert len(_diag_with_code(src, "W220")) >= 1

    def test_cmdsub_expr_read_does_not_add_read_before_set(self):
        # The set feeding the read sits in a sibling command substitution;
        # must not be reported as read-before-set.
        src = 'proc f {eps} { return "[set ee [expr {1.0 + $eps}]] / [expr {$ee - 1.0}]" }'
        assert _diag_with_code(src, "W210") == []


class TestEvalBodyReadTracking:
    """`eval {literal}` runs in the current scope; reads inside its braced body
    must be recognised so the assignment feeding them is not falsely flagged
    dead/unused (the body is opaque to the CFG)."""

    def test_eval_braced_body_read_not_unused(self):
        src = "proc f {} { set x 1; eval {puts $x} }"
        assert _diag_with_code(src, "W220") == []
        assert _diag_with_code(src, "W211") == []

    def test_eval_braced_body_expr_read(self):
        src = "proc f {} { set x 1; eval {set y [expr {$x + 1}]} }"
        assert _diag_with_code(src, "W220") == []
        assert _diag_with_code(src, "W211") == []

    def test_eval_body_without_read_still_flags(self):
        # x genuinely unused: the eval body does not reference it.
        src = "proc f {} { set x 1; eval {puts hi} }"
        assert len(_diag_with_code(src, "W211")) == 1


class TestNamespaceEvalBodyScope:
    """A ``namespace eval ns {…}`` body runs in ``ns``, NOT the caller frame, so
    an unqualified ``$x`` inside it resolves in ``ns`` (Tcl errors
    ``can't read "x"`` when absent) — it is *not* a read of a caller local.
    Recovering it would wrongly suppress a genuine unused-parameter (W214) /
    dead-store (W220) finding.  Plain ``eval {…}`` keeps its caller-scope
    recovery (it really does run in the current frame)."""

    @staticmethod
    def _w214(src: str) -> set[str]:
        return {d.message.split("'")[1] for d in analyse(src).diagnostics if d.code == "W214"}

    def test_param_used_only_in_ns_eval_body_is_unused(self):
        # `x` is referenced only inside `namespace eval ::ns {…}`, which runs in
        # ::ns — so the parameter `x` is genuinely unused in the caller.
        src = 'proc g {x} { namespace eval ::ns { puts "hello $x" } }'
        assert "x" in self._w214(src)

    def test_param_used_in_plain_eval_body_is_used(self):
        # Plain `eval {…}` runs in the current scope, so `$x` IS a use of the
        # caller's parameter — must NOT be flagged unused (contrast above).
        src = 'proc g {x} { eval { puts "hello $x" } }'
        assert "x" not in self._w214(src)

    def test_param_used_only_in_dynamic_ns_eval_body_is_unused(self):
        # A dynamically-named `namespace eval $ns {…}` body still runs in some
        # child namespace, never the caller frame.
        src = 'proc g {x ns} { namespace eval $ns { puts "hello $x" } }'
        assert "x" in self._w214(src)

    def test_ns_eval_dead_store_still_fires(self):
        # `set y 1` is dead in the caller: the namespace eval body's `$y` runs
        # in ::ns, so it does not keep the caller's `y` live.  Assert the exact
        # variable (and that only it) is flagged, not merely that something fired.
        src = 'proc g {} { set y 1; namespace eval ::ns { puts "$y" } }'
        flagged = [d.message.split("'")[1] for d in _diag_with_code(src, "W220")]
        assert flagged == ["y"], f"expected W220 on 'y' only; got {flagged}"

    def test_ns_eval_name_arg_read_still_recovered(self):
        # The namespace *name* expression is evaluated in the caller scope, so
        # `name` IS read there (regression guard: the body-scope fix must not
        # suppress the name-arg read).
        src = "proc f {conn} { set name [format ns%s $conn]\n namespace eval $name { variable s } }"
        assert _diag_with_code(src, "W220") == []
        assert _diag_with_code(src, "W211") == []

    def test_ns_eval_caller_scope_survives_uplevel_inline_rebuild(self):
        # An uplevel-passthrough candidate (`reset`) called inside the ns-eval
        # body makes inline_uplevel rebuild the IRBlock.  That rebuild must keep
        # caller_scope=False: `$x` runs in ::ns (tclsh: "can't read x"), so the
        # parameter `x` is genuinely unused in `g` — W214 must still fire.  A
        # rebuild that dropped caller_scope would recover `$x` as a caller read
        # and falsely suppress it.
        src = (
            "proc reset {} { uplevel 1 {set counter 0} }\n"
            "proc g {x} {\n"
            "    namespace eval ::ns {\n"
            "        reset\n"
            '        puts "hello $x"\n'
            "    }\n"
            "}\n"
        )
        assert "x" in self._w214(src)


class TestLoopStubStillTracksVars:
    """A `-loop` stub (declared with stubs-begin/end markers) must lower as a
    real loop so the loop variable is a def and body reads are tracked — no
    false read-before-set/unused on the loop var or accumulator."""

    def test_foreach_in_collection_loop_var_is_def(self):
        src = (
            "# tcl-lsp: stubs-begin\n"
            "# tcl-lsp: stub foreach_in_collection {varName:var collection body:body} -loop\n"
            "# tcl-lsp: stubs-end\n"
            "proc f {coll} { set t 0; foreach_in_collection x $coll { set t $x }; return $t }"
        )
        for code in ("W210", "W211", "W220"):
            assert _diag_with_code(src, code) == [], f"unexpected {code}"


class TestEvalListIsSafe:
    """`eval [list ...]` / command-substitution bodies are the recommended safe
    form (verified vs C tclsh: no double substitution) and cannot be braced, so
    W105 must not flag them — while genuine unbraced/quoted bodies still do."""

    def test_eval_list_not_flagged(self):
        assert _diag_with_code("proc f {a} { eval [list puts $a] }", "W105") == []

    def test_eval_linsert_not_flagged(self):
        assert _diag_with_code("proc f {a} { eval [linsert $a 0 puts] }", "W105") == []

    def test_eval_quoted_substitution_still_flagged(self):
        assert len(_diag_with_code('proc f {a} { eval "puts $a" }', "W105")) == 1
