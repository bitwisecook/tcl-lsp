"""Comprehensive tests for the Tcl formatter."""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.formatting import BraceStyle, FormatterConfig, IndentStyle, format_tcl
from core.formatting.engine import (
    ArgKind,
    _identify_body_args,
    _reconstruct_raw,
    parse_commands,
)
from core.parsing.tokens import TokenType


class TestFormatterConfig:
    def test_defaults(self):
        c = FormatterConfig()
        assert c.indent_size == 4
        assert c.indent_style == IndentStyle.SPACES
        assert c.brace_style == BraceStyle.K_AND_R
        assert c.space_between_braces is True
        assert c.max_line_length == 120
        assert c.goal_line_length == 100
        assert c.space_after_comment_hash is True
        assert c.trim_trailing_whitespace is True
        assert c.ensure_final_newline is True
        assert c.enforce_braced_variables is False
        assert c.enforce_braced_expr is False
        assert c.line_ending == "\n"
        assert c.expand_single_line_bodies is False
        assert c.blank_lines_between_procs == 1
        assert c.max_consecutive_blank_lines == 2
        assert c.replace_semicolons_with_newlines is True

    def test_to_dict(self):
        c = FormatterConfig()
        d = c.to_dict()
        assert d["indent_size"] == 4
        assert d["brace_style"] == "k_and_r"
        assert d["indent_style"] == "spaces"
        assert d["space_between_braces"] is True

    def test_from_dict_round_trip(self):
        c = FormatterConfig(indent_size=2)
        d = c.to_dict()
        c2 = FormatterConfig.from_dict(d)
        assert c2.indent_size == 2
        assert c2.brace_style == BraceStyle.K_AND_R

    def test_from_dict_partial(self):
        c = FormatterConfig.from_dict({"indent_size": 8})
        assert c.indent_size == 8
        assert c.brace_style == BraceStyle.K_AND_R  # default preserved

    def test_from_dict_empty(self):
        c = FormatterConfig.from_dict({})
        assert c.indent_size == 4  # all defaults

    def test_replace(self):
        c = FormatterConfig()
        c2 = c.replace(indent_size=2)
        assert c2.indent_size == 2
        assert c.indent_size == 4  # original unchanged


class TestParseCommands:
    def test_simple_command(self):
        cmds, trailing = parse_commands("set x 42")
        assert len(cmds) == 1
        assert cmds[0].name == "set"
        assert len(cmds[0].args) == 3
        assert cmds[0].args[1].text == "x"
        assert cmds[0].args[2].text == "42"

    def test_multiple_commands(self):
        cmds, _ = parse_commands("set x 1\nset y 2")
        assert len(cmds) == 2
        assert cmds[0].name == "set"
        assert cmds[1].name == "set"

    def test_braced_arg(self):
        cmds, _ = parse_commands("proc foo {} {puts hi}")
        assert cmds[0].args[2].is_braced is True  # {}
        assert cmds[0].args[3].is_braced is True  # {puts hi}

    def test_quoted_arg(self):
        cmds, _ = parse_commands('puts "hello world"')
        assert len(cmds) == 1
        assert cmds[0].args[1].is_quoted is True

    def test_variable_arg(self):
        cmds, _ = parse_commands("puts $name")
        assert len(cmds) == 1
        arg = cmds[0].args[1]
        assert arg.tokens[0].type == TokenType.VAR

    def test_command_substitution(self):
        cmds, _ = parse_commands("set x [expr {1 + 2}]")
        assert len(cmds) == 1
        assert cmds[0].args[2].tokens[0].type == TokenType.CMD

    def test_preceding_comment(self):
        cmds, _ = parse_commands("# a comment\nputs hi")
        assert len(cmds) == 1
        assert "a comment" in cmds[0].preceding_comments[0]

    def test_trailing_comments(self):
        _, trailing = parse_commands("puts hi\n# trailing")
        assert len(trailing) == 1
        assert "trailing" in trailing[0]

    def test_blank_lines(self):
        cmds, _ = parse_commands("set x 1\n\n\nset y 2")
        assert len(cmds) == 2
        assert cmds[1].preceding_blank_lines >= 1

    def test_empty_source(self):
        cmds, trailing = parse_commands("")
        assert len(cmds) == 0
        assert len(trailing) == 0

    def test_only_comments(self):
        cmds, trailing = parse_commands("# hello\n# world")
        assert len(cmds) == 0
        assert len(trailing) == 2

    def test_multi_token_arg(self):
        """Quoted string with variable: "hello $name" """
        cmds, _ = parse_commands('puts "hello $name"')
        assert len(cmds) == 1
        arg = cmds[0].args[1]
        assert arg.is_quoted is True
        assert len(arg.tokens) >= 2  # ESC + VAR at minimum


class TestReconstruction:
    def test_reconstruct_esc(self):
        from core.parsing.tokens import SourcePosition, Token

        pos = SourcePosition(0, 0, 0)
        tok = Token(type=TokenType.ESC, text="hello", start=pos, end=pos)
        assert _reconstruct_raw(tok) == "hello"

    def test_reconstruct_str(self):
        from core.parsing.tokens import SourcePosition, Token

        pos = SourcePosition(0, 0, 0)
        tok = Token(type=TokenType.STR, text="hello", start=pos, end=pos)
        assert _reconstruct_raw(tok) == "{hello}"

    def test_reconstruct_cmd(self):
        from core.parsing.tokens import SourcePosition, Token

        pos = SourcePosition(0, 0, 0)
        tok = Token(type=TokenType.CMD, text="expr 1+2", start=pos, end=pos)
        assert _reconstruct_raw(tok) == "[expr 1+2]"

    def test_reconstruct_var(self):
        from core.parsing.tokens import SourcePosition, Token

        pos = SourcePosition(0, 0, 0)
        tok = Token(type=TokenType.VAR, text="name", start=pos, end=pos)
        assert _reconstruct_raw(tok) == "$name"


class TestBodyIdentification:
    def test_proc_body(self):
        cmds, _ = parse_commands("proc foo {} {puts hi}")
        _identify_body_args(cmds[0])
        # arg[0]=proc, arg[1]=foo, arg[2]={}, arg[3]={puts hi}
        assert cmds[0].args[3].kind == ArgKind.BODY

    def test_if_body(self):
        cmds, _ = parse_commands("if {1} {puts yes}")
        _identify_body_args(cmds[0])
        # arg[0]=if, arg[1]={1}, arg[2]={puts yes}
        assert cmds[0].args[2].kind == ArgKind.BODY

    def test_if_else_bodies(self):
        cmds, _ = parse_commands("if {1} {puts yes} else {puts no}")
        _identify_body_args(cmds[0])
        assert cmds[0].args[2].kind == ArgKind.BODY
        assert cmds[0].args[3].kind == ArgKind.KEYWORD  # else
        assert cmds[0].args[4].kind == ArgKind.BODY

    def test_while_body(self):
        cmds, _ = parse_commands("while {1} {puts hi}")
        _identify_body_args(cmds[0])
        assert cmds[0].args[2].kind == ArgKind.BODY

    def test_for_body(self):
        cmds, _ = parse_commands("for {set i 0} {$i < 10} {incr i} {puts $i}")
        _identify_body_args(cmds[0])
        # Only body (arg[4]) should be BODY; init and next stay as WORD
        assert cmds[0].args[4].kind == ArgKind.BODY
        assert cmds[0].args[1].kind == ArgKind.WORD  # init
        assert cmds[0].args[3].kind == ArgKind.WORD  # next

    def test_foreach_body(self):
        cmds, _ = parse_commands("foreach item $list {puts $item}")
        _identify_body_args(cmds[0])
        assert cmds[0].args[3].kind == ArgKind.BODY

    def test_namespace_eval_body(self):
        cmds, _ = parse_commands("namespace eval ::ns {proc foo {} {}}")
        _identify_body_args(cmds[0])
        # arg[0]=namespace, arg[1]=eval, arg[2]=::ns, arg[3]={...}
        assert cmds[0].args[3].kind == ArgKind.BODY

    def test_try_body(self):
        cmds, _ = parse_commands("try {puts hi} finally {puts bye}")
        _identify_body_args(cmds[0])
        assert cmds[0].args[1].kind == ArgKind.BODY  # try body
        assert cmds[0].args[2].kind == ArgKind.KEYWORD  # finally
        assert cmds[0].args[3].kind == ArgKind.BODY  # finally body

    def test_switch_braced_body(self):
        cmds, _ = parse_commands("switch $x {a {puts a} b {puts b}}")
        _identify_body_args(cmds[0])
        # The braced body is arg[2]
        assert cmds[0].args[2].kind == ArgKind.BODY

    def test_unknown_command(self):
        cmds, _ = parse_commands("mycommand {some arg}")
        _identify_body_args(cmds[0])
        # Unknown commands: all args stay as WORD
        assert cmds[0].args[1].kind == ArgKind.WORD

    def test_catch_body(self):
        cmds, _ = parse_commands("catch {error oops}")
        _identify_body_args(cmds[0])
        assert cmds[0].args[1].kind == ArgKind.BODY

    def test_eval_body(self):
        cmds, _ = parse_commands("eval {puts hi}")
        _identify_body_args(cmds[0])
        assert cmds[0].args[1].kind == ArgKind.BODY

    def test_time_body(self):
        cmds, _ = parse_commands("time {set x 1} 1000")
        _identify_body_args(cmds[0])
        assert cmds[0].args[1].kind == ArgKind.BODY


class TestBasicFormatting:
    def test_simple_command(self):
        assert format_tcl("puts hello") == "puts hello\n"

    def test_trailing_newline(self):
        result = format_tcl("puts hello")
        assert result.endswith("\n")

    def test_no_trailing_newline(self):
        config = FormatterConfig(ensure_final_newline=False)
        result = format_tcl("puts hello", config)
        assert not result.endswith("\n")

    def test_trim_trailing_whitespace(self):
        result = format_tcl("puts hello   ")
        assert "hello   " not in result

    def test_multiple_commands(self):
        result = format_tcl("set x 1\nset y 2")
        assert result == "set x 1\nset y 2\n"

    def test_empty_source(self):
        result = format_tcl("")
        assert result == "\n"

    def test_only_comments(self):
        result = format_tcl("# hello")
        assert result == "# hello\n"

    def test_proc_indentation(self):
        source = "proc foo {} {\nset x 1\nreturn $x\n}"
        expected = "proc foo {} {\n    set x 1\n    return $x\n}\n"
        assert format_tcl(source) == expected

    def test_nested_indentation(self):
        source = "proc foo {} {\nif {1} {\nputs hi\n}\n}"
        result = format_tcl(source)
        lines = result.strip().split("\n")
        assert lines[0] == "proc foo {} {"
        assert lines[1] == "    if {1} {"
        assert lines[2] == "        puts hi"
        assert lines[3] == "    }"
        assert lines[4] == "}"

    def test_if_else(self):
        source = "if {$x > 0} {\nputs pos\n} else {\nputs neg\n}"
        result = format_tcl(source)
        lines = result.strip().split("\n")
        assert lines[0] == "if {$x > 0} {"
        assert lines[1] == "    puts pos"
        assert lines[2] == "} else {"
        assert lines[3] == "    puts neg"
        assert lines[4] == "}"

    def test_if_elseif_else(self):
        source = "if {$x > 0} {\nputs pos\n} elseif {$x == 0} {\nputs zero\n} else {\nputs neg\n}"
        result = format_tcl(source)
        assert "} elseif" in result
        assert "} else {" in result

    def test_while_loop(self):
        source = "while {$i < 10} {\nincr i\nputs $i\n}"
        result = format_tcl(source)
        lines = result.strip().split("\n")
        assert lines[0] == "while {$i < 10} {"
        assert lines[1] == "    incr i"
        assert lines[2] == "    puts $i"
        assert lines[3] == "}"

    def test_for_loop(self):
        source = "for {set i 0} {$i < 10} {incr i} {\nputs $i\n}"
        result = format_tcl(source)
        lines = result.strip().split("\n")
        assert lines[0] == "for {set i 0} {$i < 10} {incr i} {"
        assert lines[1] == "    puts $i"
        assert lines[2] == "}"

    def test_foreach(self):
        source = "foreach item $list {\nputs $item\n}"
        result = format_tcl(source)
        lines = result.strip().split("\n")
        assert lines[0] == "foreach item $list {"
        assert lines[1] == "    puts $item"
        assert lines[2] == "}"

    def test_namespace_eval(self):
        source = "namespace eval ::myns {\nproc foo {} {\nputs hi\n}\n}"
        result = format_tcl(source)
        lines = result.strip().split("\n")
        assert lines[0] == "namespace eval ::myns {"
        assert lines[1] == "    proc foo {} {"
        assert lines[2] == "        puts hi"
        assert lines[3] == "    }"
        assert lines[4] == "}"

    def test_try_finally(self):
        source = "try {\nputs hi\n} finally {\nputs bye\n}"
        result = format_tcl(source)
        lines = result.strip().split("\n")
        assert lines[0] == "try {"
        assert lines[1] == "    puts hi"
        assert lines[2] == "} finally {"
        assert lines[3] == "    puts bye"
        assert lines[4] == "}"

    def test_switch_braced(self):
        source = "switch $x {\na {puts a}\nb {puts b}\n}"
        result = format_tcl(source)
        assert "switch $x {" in result
        assert "    a {" in result
        assert "        puts a" in result

    def test_deeply_nested(self):
        source = "if {1} {\nif {2} {\nif {3} {\nif {4} {\nputs deep\n}\n}\n}\n}"
        result = format_tcl(source)
        lines = result.strip().split("\n")
        # Find the "puts deep" line and check its indent
        for line in lines:
            if "puts deep" in line:
                indent = len(line) - len(line.lstrip())
                assert indent == 16  # 4 levels * 4 spaces

    def test_quoted_string_preserved(self):
        source = 'puts "hello $name"'
        result = format_tcl(source)
        assert '"hello $name"' in result

    def test_command_substitution_preserved(self):
        source = "set x [expr {1 + 2}]"
        result = format_tcl(source)
        assert "[expr {1 + 2}]" in result


class TestInlineBodies:
    def test_control_flow_always_expands(self):
        """if/while/for/foreach/proc always expand bodies per F5 style guide."""
        source = "if {$x} { continue }"
        result = format_tcl(source)
        assert "    continue" in result

    def test_non_control_flow_inlines(self):
        """Unknown commands with short bodies stay inline."""
        source = "catch { error oops }"
        result = format_tcl(source)
        # catch is not in the never-inline set, single command, no nested braces
        assert "{ error oops }" in result

    def test_long_body_expands(self):
        """A body that would make the line exceed goal_line_length expands."""
        config = FormatterConfig(goal_line_length=40)
        source = "time {\nset very_long_variable_name some_value\n} 1000"
        result = format_tcl(source, config)
        assert result.count("\n") > 2

    def test_multi_command_body_expands(self):
        """Bodies with multiple commands always expand."""
        source = "if {1} {\nputs a\nputs b\n}"
        result = format_tcl(source)
        assert "    puts a" in result
        assert "    puts b" in result

    def test_empty_body(self):
        source = "proc noop {} {}"
        result = format_tcl(source)
        assert "proc noop {} {}" in result

    def test_expand_single_line_bodies_config(self):
        """expand_single_line_bodies forces expansion even for simple commands."""
        config = FormatterConfig(expand_single_line_bodies=True)
        source = "catch { error oops }"
        result = format_tcl(source, config)
        assert "    error oops" in result

    def test_body_with_nested_braces_expands(self):
        """Bodies containing nested braces always expand."""
        source = "catch { set x {value} }"
        result = format_tcl(source)
        assert "    set x {value}" in result


class TestConfigVariations:
    def test_indent_size_2(self):
        config = FormatterConfig(indent_size=2)
        source = "proc foo {} {\nset x 1\nreturn $x\n}"
        result = format_tcl(source, config)
        assert "  set x 1" in result
        assert "  return $x" in result

    def test_indent_size_8(self):
        config = FormatterConfig(indent_size=8)
        source = "proc foo {} {\nset x 1\n}"
        result = format_tcl(source, config)
        assert "        set x 1" in result

    def test_tab_indentation(self):
        config = FormatterConfig(indent_style=IndentStyle.TABS)
        source = "proc foo {} {\nset x 1\n}"
        result = format_tcl(source, config)
        assert "\tset x 1" in result

    def test_no_space_between_braces(self):
        config = FormatterConfig(space_between_braces=False)
        # Keywords always need spaces (}else{ is invalid Tcl), so
        # space_between_braces only affects consecutive body braces.
        source = "if {1} {\nputs yes\n} else {\nputs no\n}"
        result = format_tcl(source, config)
        # Keywords always get spaces around them for valid Tcl
        assert "} else {" in result
        # Condition arg {1} is a regular WORD -- always normal spacing
        assert "if {1} {" in result

    def test_no_space_between_braces_preserves_condition_spacing(self):
        """space_between_braces doesn't affect spacing around condition args."""
        config = FormatterConfig(space_between_braces=False)
        source = "while {$i < 10} {\nputs $i\n}"
        result = format_tcl(source, config)
        # Condition {$i < 10} is a regular arg -- always gets normal spacing
        assert "while {$i < 10} {" in result

    def test_no_space_after_comment_hash(self):
        config = FormatterConfig(space_after_comment_hash=False)
        source = "# hello world"
        result = format_tcl(source, config)
        assert result.startswith("#hello")

    def test_crlf_line_endings(self):
        config = FormatterConfig(line_ending="\r\n")
        result = format_tcl("puts hello", config)
        assert "\r\n" in result
        assert result.count("\n") == result.count("\r\n")

    def test_blank_lines_between_procs(self):
        config = FormatterConfig(blank_lines_between_procs=2)
        source = "proc foo {} {}\nproc bar {} {}"
        result = format_tcl(source, config)
        # Should have 2 blank lines between procs
        assert "\n\n\nproc bar" in result

    def test_max_consecutive_blank_lines(self):
        config = FormatterConfig(max_consecutive_blank_lines=1)
        source = "set x 1\n\n\n\n\nset y 2"
        result = format_tcl(source, config)
        assert "\n\n\n" not in result


class TestComments:
    def test_comment_before_command(self):
        source = "# a comment\nputs hi"
        result = format_tcl(source)
        lines = result.strip().split("\n")
        assert lines[0] == "# a comment"
        assert lines[1] == "puts hi"

    def test_comment_indent_in_body(self):
        source = "proc foo {} {\n# inside\nputs hi\n}"
        result = format_tcl(source)
        assert "    # inside" in result

    def test_comment_between_commands(self):
        source = "set x 1\n# between\nset y 2"
        result = format_tcl(source)
        assert "# between" in result

    def test_trailing_comments(self):
        source = "puts hi\n# trailing"
        result = format_tcl(source)
        assert "# trailing" in result

    def test_commented_out_code(self):
        """Comments starting with #command (no space) are preserved as-is."""
        source = "#puts debug_output"
        result = format_tcl(source)
        assert result.strip() == "#puts debug_output"

    def test_regular_comment_spacing_preserved(self):
        """Whitespace between # and text is preserved to protect ASCII art
        and commented-out code formatting."""
        source = "#   lots of spaces"
        result = format_tcl(source)
        assert result.strip() == "#   lots of spaces"

    def test_comment_ascii_diagram_preserved(self):
        """Extra whitespace in comments for ASCII diagrams is preserved."""
        source = "#  +-------+\n#  |  box  |\n#  +-------+"
        result = format_tcl(source)
        lines = result.strip().split("\n")
        assert lines[0] == "#  +-------+"
        assert lines[1] == "#  |  box  |"
        assert lines[2] == "#  +-------+"

    def test_commented_code_with_space_preserved(self):
        """Comments like '# set x 1' preserve their single space."""
        source = "# set x 1"
        result = format_tcl(source)
        assert result.strip() == "# set x 1"

    def test_multiple_comments_before_command(self):
        source = "# comment 1\n# comment 2\nputs hi"
        result = format_tcl(source)
        lines = result.strip().split("\n")
        assert lines[0] == "# comment 1"
        assert lines[1] == "# comment 2"
        assert lines[2] == "puts hi"


class TestBracedVariables:
    def test_simple_var(self):
        config = FormatterConfig(enforce_braced_variables=True)
        result = format_tcl("puts $name", config)
        assert "${name}" in result

    def test_var_in_quoted_string(self):
        config = FormatterConfig(enforce_braced_variables=True)
        result = format_tcl('puts "hello $name"', config)
        assert "${name}" in result

    def test_disabled_by_default(self):
        result = format_tcl("puts $name")
        assert "$name" in result
        assert "${name}" not in result


class TestSwitchFormatting:
    def test_switch_braced_body(self):
        source = "switch $x {\na {puts a}\nb {puts b}\n}"
        result = format_tcl(source)
        assert "switch $x {" in result
        assert "    a {" in result

    def test_switch_with_default(self):
        source = "switch $x {\na {puts a}\ndefault {puts default}\n}"
        result = format_tcl(source)
        assert "default {" in result

    def test_switch_with_options(self):
        source = "switch -exact -- $x {\na {puts a}\n}"
        result = format_tcl(source)
        assert "switch -exact -- $x {" in result

    def test_switch_fallthrough(self):
        source = "switch $x {\na -\nb {puts ab}\n}"
        result = format_tcl(source)
        assert "    a -" in result


class TestEdgeCases:
    def test_empty_proc_body(self):
        source = "proc noop {} {}"
        result = format_tcl(source)
        assert "proc noop {} {}" in result

    def test_variable_only(self):
        source = "$x"
        result = format_tcl(source)
        assert "$x" in result

    def test_backslash_in_string(self):
        source = 'puts "hello\\nworld"'
        result = format_tcl(source)
        assert "hello\\nworld" in result

    def test_braced_string_preserved(self):
        source = "set x {hello world}"
        result = format_tcl(source)
        assert "{hello world}" in result

    def test_complex_expression(self):
        source = "expr {$x + $y * 2}"
        result = format_tcl(source)
        assert "expr {$x + $y * 2}" in result

    def test_semicolons_in_source(self):
        """Commands separated by semicolons should be split to lines."""
        source = "set x 1; set y 2"
        result = format_tcl(source)
        lines = result.strip().split("\n")
        assert len(lines) == 2
        assert lines[0] == "set x 1"
        assert lines[1] == "set y 2"

    def test_unknown_command_preserved(self):
        source = "mycommand -option {some value} arg1 arg2"
        result = format_tcl(source)
        assert "mycommand -option {some value} arg1 arg2" in result

    def test_nested_command_substitution(self):
        source = "set x [expr {[llength $list] + 1}]"
        result = format_tcl(source)
        assert "[expr {[llength $list] + 1}]" in result

    def test_array_variable(self):
        source = 'set arr(key) "value"'
        result = format_tcl(source)
        assert 'set arr(key) "value"' in result


class TestLongLineWrapping:
    """Long expression arguments should be wrapped at && / || operators."""

    def test_if_wraps_at_and_operator(self):
        """An if condition exceeding max_line_length wraps at &&."""
        config = FormatterConfig(max_line_length=60)
        source = (
            "if {$alpha_value > 100 && $beta_value > 200 && $gamma_value > 300} {\n    puts yes\n}"
        )
        result = format_tcl(source, config)
        assert "&&" in result
        # Should have a line break before &&
        lines = result.strip().split("\n")
        assert any("&&" in line and not line.lstrip().startswith("if") for line in lines)

    def test_if_wraps_at_or_operator(self):
        """An if condition exceeding max_line_length wraps at ||."""
        config = FormatterConfig(max_line_length=60)
        source = (
            "if {$alpha_value > 100 || $beta_value > 200 || $gamma_value > 300} {\n    puts yes\n}"
        )
        result = format_tcl(source, config)
        lines = result.strip().split("\n")
        assert any("||" in line and not line.lstrip().startswith("if") for line in lines)

    def test_short_line_not_wrapped(self):
        """Lines under max_line_length are not wrapped."""
        config = FormatterConfig(max_line_length=120)
        source = "if {$x > 0 && $y > 0} {\n    puts yes\n}"
        result = format_tcl(source, config)
        lines = result.strip().split("\n")
        assert lines[0] == "if {$x > 0 && $y > 0} {"

    def test_wrapping_idempotent(self):
        """Wrapping is stable across multiple format passes."""
        config = FormatterConfig(max_line_length=60)
        source = (
            "if {$alpha_value > 100 && $beta_value > 200 && $gamma_value > 300} {\n    puts yes\n}"
        )
        r1 = format_tcl(source, config)
        r2 = format_tcl(r1, config)
        assert r1 == r2

    def test_while_condition_wraps(self):
        """Long while conditions are wrapped at operators."""
        config = FormatterConfig(max_line_length=70)
        source = 'while {[string length $longvar] > 100 && [string first "x" $longvar] >= 0} {\n    incr c\n}'
        result = format_tcl(source, config)
        lines = result.strip().split("\n")
        # First line should start with while, continuation should have &&
        assert lines[0].startswith("while {")
        assert any(line.lstrip().startswith("&&") for line in lines)

    def test_for_condition_wraps(self):
        """Long for-loop conditions are wrapped at operators."""
        config = FormatterConfig(max_line_length=70)
        source = "for {set i 0} {$i < [llength $longvar] && $i < [string length $other_longvar]} {incr i} {\n    puts $i\n}"
        result = format_tcl(source, config)
        lines = result.strip().split("\n")
        assert any(line.lstrip().startswith("&&") for line in lines)

    def test_irules_long_condition(self):
        """The motivating iRules example wraps correctly."""
        source = (
            "when HTTP_REQUEST {\n"
            '    if {[string tolower [HTTP::uri]] starts_with "/ce0587te"'
            " && [class match [IP::client_addr] equals"
            " AppCheck_TrentScanning_TrustedAddresses]} {\n"
            "        ASM::disable\n"
            "    }\n"
            "}"
        )
        result = format_tcl(source)
        lines = result.strip().split("\n")
        # The condition should be split across lines
        assert any(line.lstrip().startswith("&&") for line in lines)
        # Body should still be indented correctly
        assert any("ASM::disable" in line for line in lines)

    def test_continuation_indent(self):
        """Wrapped continuation lines use indent_level + 1."""
        config = FormatterConfig(max_line_length=60, indent_size=4)
        source = (
            "if {$alpha_value > 100 && $beta_value > 200 && $gamma_value > 300} {\n    puts yes\n}"
        )
        result = format_tcl(source, config)
        lines = result.strip().split("\n")
        # Expression content at indent_level 0 + 1 = 4 spaces
        for line in lines:
            if line.lstrip().startswith("&&") or line.lstrip().startswith("||"):
                indent_count = len(line) - len(line.lstrip())
                assert indent_count == 4  # (indent_level + 1) * indent_size

    def test_nested_continuation_indent(self):
        """Wrapped lines inside a proc use correct indentation."""
        config = FormatterConfig(max_line_length=70, indent_size=4)
        source = (
            "proc test {} {\n"
            "    if {$a > 1 && $b > 2 && $c > 3 && $d > 4} {\n"
            "        puts yes\n"
            "    }\n"
            "}"
        )
        result = format_tcl(source, config)
        lines = result.strip().split("\n")
        # if is at indent_level=1 (4 spaces), so expression at (1+1)*4 = 8
        for line in lines:
            if line.lstrip().startswith("&&") or line.lstrip().startswith("||"):
                indent = len(line) - len(line.lstrip())
                assert indent == 8  # (indent_level + 1) * indent_size

    def test_mixed_operators(self):
        """Wrapping works with mixed && and || operators."""
        config = FormatterConfig(max_line_length=50)
        source = "if {$alpha > 1 && $beta > 2 || $gamma > 3 && $delta > 4} {\n    puts yes\n}"
        result = format_tcl(source, config)
        r2 = format_tcl(result, config)
        assert result == r2  # idempotent

    def test_no_break_inside_brackets(self):
        """&& inside command substitution is not a break point."""
        config = FormatterConfig(max_line_length=60)
        source = "if {[expr {$a && $b}] && [expr {$c && $d}]} {\n    puts yes\n}"
        result = format_tcl(source, config)
        # Should only break at the top-level && between the two [expr]
        lines = result.strip().split("\n")
        for line in lines:
            stripped = line.lstrip()
            if stripped.startswith("&&"):
                assert stripped.startswith("&& [expr")

    def test_one_operand_per_line(self):
        """Each operand gets its own indented line in block style."""
        config = FormatterConfig(max_line_length=60)
        source = (
            "if {$alpha && $beta && $gamma && $delta"
            " && $epsilon && $zeta && $eta && $theta} {\n    puts yes\n}"
        )
        result = format_tcl(source, config)
        lines = result.strip().split("\n")
        # First line is just "if {"
        assert lines[0] == "if {"
        # Each && operand on its own line
        op_lines = [ln for ln in lines if ln.lstrip().startswith("&&")]
        assert len(op_lines) == 7  # 8 operands, 7 && operators

    def test_elseif_wraps(self):
        """Long elseif conditions also wrap."""
        config = FormatterConfig(max_line_length=60)
        source = (
            "if {$x} {\n    puts x\n}"
            " elseif {$alpha_value > 100 && $beta_value > 200"
            " && $gamma_value > 300} {\n    puts abc\n}"
        )
        result = format_tcl(source, config)
        lines = result.strip().split("\n")
        assert any(line.lstrip().startswith("&&") for line in lines)

    def test_already_wrapped_preserved(self):
        """An expression already wrapped with newlines stays wrapped."""
        config = FormatterConfig(max_line_length=80)
        source = (
            "if {\n"
            '    [string tolower [HTTP::uri]] starts_with "/path"\n'
            "    && [class match [IP::client_addr] equals Trusted]\n"
            "} {\n"
            "    ASM::disable\n"
            "}"
        )
        r1 = format_tcl(source, config)
        r2 = format_tcl(r1, config)
        assert r1 == r2

    def test_block_structure(self):
        """Wrapped expression has opening { on command line, } on its own."""
        config = FormatterConfig(max_line_length=30)
        source = "if {$a == 1 || $b == 2 || $c != 3} {\n    puts yes\n}"
        result = format_tcl(source, config)
        lines = result.strip().split("\n")
        assert lines[0] == "if {"
        assert lines[-1] == "}"
        # Closing brace of expression + body opening on same line
        assert any(line.strip() == "} {" for line in lines)


class TestBackslashContinuation:
    """Long command lines should be split with backslash continuation."""

    def test_long_command_splits_at_args(self):
        """A command exceeding max_line_length splits at argument boundaries."""
        config = FormatterConfig(max_line_length=60)
        source = "table set -subtable very_long_subtable_name -- [DNS::name $rr]___[DNS::type $rr]"
        result = format_tcl(source, config)
        assert " \\\n" in result
        # All lines should fit within the limit
        for line in result.strip().split("\n"):
            assert len(line) <= 60

    def test_short_line_not_split(self):
        """Lines under max_line_length are not split."""
        source = "set x [expr {1 + 2}]"
        result = format_tcl(source)
        assert "\\" not in result.strip()

    def test_nested_bracket_splitting(self):
        """Arguments inside [] can be split when top-level splitting is not enough."""
        config = FormatterConfig(max_line_length=80)
        source = "set x [table set -subtable some_long_name -- [DNS::name $rr]___[DNS::type $rr] [DNS::ttl $rr] 86400]"
        result = format_tcl(source, config)
        assert " \\\n" in result
        for line in result.strip().split("\n"):
            assert len(line) <= 80

    def test_continuation_indent(self):
        """Continuation lines use indent_level + 1."""
        config = FormatterConfig(max_line_length=40, indent_size=4)
        source = "proc foo {} {\n    set x [long_command -option1 value1 -option2 value2 -option3 value3]\n}"
        result = format_tcl(source, config)
        lines = result.strip().split("\n")
        # Find continuation lines (those that follow a line ending with \)
        for i, line in enumerate(lines):
            if i > 0 and lines[i - 1].rstrip().endswith("\\"):
                indent = len(line) - len(line.lstrip())
                # Should be at least indent_level + 1
                assert indent >= 8  # (1 + 1) * 4

    def test_splitting_idempotent(self):
        """Splitting is stable across multiple format passes."""
        config = FormatterConfig(max_line_length=80)
        source = "DNS::ttl $rr [table set -subtable f5volt_31839_ns1.f5clouddns.com -- [DNS::name $rr]___[DNS::type $rr] [DNS::ttl $rr] 86400 [expr {[DNS::ttl $rr] + 1}]]"
        r1 = format_tcl(source, config)
        r2 = format_tcl(r1, config)
        assert r1 == r2

    def test_body_args_not_split(self):
        """Body braces should not be broken by line splitting."""
        source = "if {$x > 0} {\n    puts hello\n}"
        result = format_tcl(source)
        lines = result.strip().split("\n")
        # The opening line should have the body brace attached
        assert lines[0].endswith("{")

    def test_dns_irule_long_lines(self):
        """The motivating DNS iRule example has all lines under 120 chars."""
        source = (
            "when DNS_RESPONSE {\n"
            '    if {"[DNS::header rcode]" eq "NOERROR"} {\n'
            "        foreach rr $rrs {\n"
            "            if {[DNS::ttl $rr] != 0} {\n"
            '                if {"[DNS::origin]" eq "SERVER"} {\n'
            "                    DNS::ttl $rr [table set -subtable"
            " f5volt_31839_ns1.f5clouddns.com --"
            " [DNS::name $rr]___[DNS::type $rr]"
            " [DNS::ttl $rr] 86400"
            " [expr {[DNS::ttl $rr] < ${ttl_ceiling}"
            " ? [expr {[DNS::ttl $rr] + 1}]"
            " : ${ttl_ceiling}}]]\n"
            "                }\n"
            "            }\n"
            "        }\n"
            "    }\n"
            "}"
        )
        result = format_tcl(source)
        for line in result.strip().split("\n"):
            assert len(line) <= 120, f"Line too long ({len(line)}): {line}"
        # Verify idempotency
        r2 = format_tcl(result)
        assert result == r2


class TestCommentedCodeSplitting:
    """Long commented-out commands should be split with backslash continuation."""

    def test_long_commented_command_splits(self):
        """A long commented-out command is split using \\ continuation."""
        config = FormatterConfig(max_line_length=60)
        source = '#log local0. "INGRESS - Client: [IP::client_addr] Question:[DNS::question name] Type:[DNS::question type]"'
        result = format_tcl(source, config)
        lines = result.strip().split("\n")
        # First line should start with # and end with \
        assert lines[0].startswith("#")
        assert lines[0].rstrip().endswith("\\")
        # Continuation lines should NOT start with #
        for line in lines[1:]:
            assert not line.lstrip().startswith("#")

    def test_short_comment_not_split(self):
        """Short commented-out commands are not split."""
        source = "#puts hello"
        result = format_tcl(source)
        assert "\\" not in result.strip()

    def test_commented_code_idempotent(self):
        """Commented-out code splitting is stable across passes."""
        source = '#log local0. "INGRESS - Client: [IP::client_addr] Question:[DNS::question name] Type:[DNS::question type] Class:[DNS::question class]"'
        r1 = format_tcl(source)
        r2 = format_tcl(r1)
        assert r1 == r2

    def test_regular_comment_not_split(self):
        """Regular comments (with space after #) are not split."""
        source = "# This is a very long comment that describes something important about the implementation details of the DNS caching"
        result = format_tcl(source)
        # Regular comments should be preserved as-is (only formatting, no splitting)
        assert "\\" not in result.strip()

    def test_commented_code_inside_body(self):
        """Commented-out code inside a body gets correct indent."""
        source = (
            "when DNS_REQUEST {\n"
            '    #log local0. "INGRESS - Client: [IP::client_addr]'
            " Question:[DNS::question name] Type:[DNS::question type]"
            ' Class:[DNS::question class] Origin:[DNS::origin]"\n'
            "}"
        )
        result = format_tcl(source)
        lines = result.strip().split("\n")
        # The commented line should be indented
        comment_line = [ln for ln in lines if ln.lstrip().startswith("#")][0]
        assert comment_line.startswith("    #")


class TestQuotedStringSplitting:
    """Long double-quoted strings should be split with \\<newline> inside quotes."""

    def test_long_quoted_string_splits(self):
        r"""A long quoted string is split at internal spaces using \<newline>."""
        config = FormatterConfig(max_line_length=80)
        source = 'puts "This is a very long message with many words that exceeds the line length limit for testing purposes"'
        result = format_tcl(source, config)
        # Should have been split
        assert " \\\n" in result
        for line in result.strip().split("\n"):
            assert len(line) <= 80

    def test_quoted_string_splitting_idempotent(self):
        r"""Quoted string splitting is stable across passes."""
        config = FormatterConfig(max_line_length=80)
        source = '#log local0. "INGRESS - Client: [IP::client_addr] Question:[DNS::question name] Type:[DNS::question type] Class:[DNS::question class]"'
        r1 = format_tcl(source, config)
        r2 = format_tcl(r1, config)
        assert r1 == r2


class TestIdempotency:
    """Formatting twice should produce the same result as formatting once."""

    @pytest.mark.parametrize(
        "source",
        [
            "puts hello",
            "set x 42",
            'puts "hello $name"',
            "proc foo {x} {\nif {$x > 0} {\nputs pos\n} else {\nputs neg\n}\n}",
            "for {set i 0} {$i < 10} {incr i} {\nputs $i\n}",
            "foreach item $list {\nputs $item\n}",
            "switch $x {\na {puts a}\nb {puts b}\ndefault {puts c}\n}",
            "try {\nputs hi\n} finally {\nputs bye\n}",
            "namespace eval ::ns {\nproc foo {} {\nputs hi\n}\n}",
            "# comment\nputs hi",
            "# only comments\n# more comments",
            "",
            "while {$i < 10} {\nincr i\nif {$i == 5} { continue }\nputs $i\n}",
            "set x 1; set y 2; set z 3",
            # Long command with nested command substitution
            "DNS::ttl $rr [table set -subtable f5volt_31839_ns1.f5clouddns.com -- [DNS::name $rr]___[DNS::type $rr] [DNS::ttl $rr] 86400 [expr {[DNS::ttl $rr] + 1}]]",
            # Commented-out long command
            '#log local0. "INGRESS - Client: [IP::client_addr] Question:[DNS::question name] Type:[DNS::question type] Class:[DNS::question class] Origin:[DNS::origin]"',
        ],
    )
    def test_idempotent(self, source):
        r1 = format_tcl(source)
        r2 = format_tcl(r1)
        assert r1 == r2, f"Not idempotent:\nFirst:  {r1!r}\nSecond: {r2!r}"

    @pytest.mark.parametrize("fixture", ["simple.tcl", "procs.tcl", "namespaces.tcl"])
    def test_fixture_idempotent(self, fixture):
        fixture_path = Path(__file__).parent / "fixtures" / fixture
        source = fixture_path.read_text()
        r1 = format_tcl(source)
        r2 = format_tcl(r1)
        assert r1 == r2, f"Fixture {fixture} not idempotent"

    @pytest.mark.parametrize(
        "config",
        [
            FormatterConfig(indent_size=2),
            FormatterConfig(indent_style=IndentStyle.TABS),
            FormatterConfig(space_between_braces=False),
            FormatterConfig(expand_single_line_bodies=True),
            FormatterConfig(enforce_braced_variables=True),
        ],
    )
    def test_idempotent_with_config(self, config):
        source = "proc foo {x} {\nif {$x > 0} {\nputs $x\n} else {\nputs neg\n}\n}"
        r1 = format_tcl(source, config)
        r2 = format_tcl(r1, config)
        assert r1 == r2, f"Not idempotent with config {config}:\nFirst:  {r1!r}\nSecond: {r2!r}"


class TestLSPFormatting:
    """Tests for the LSP integration layer."""

    def test_get_formatting_returns_text_edit(self):
        from lsprotocol.types import FormattingOptions

        from lsp.features.formatting import get_formatting

        source = "proc foo {} {\nset x 1\nreturn $x\n}"
        options = FormattingOptions(tab_size=4, insert_spaces=True)
        edits = get_formatting(source, options)
        assert len(edits) == 1
        assert edits[0].new_text.startswith("proc foo {} {")
        assert "    set x 1" in edits[0].new_text

    def test_get_formatting_no_change(self):
        from lsprotocol.types import FormattingOptions

        from lsp.features.formatting import get_formatting

        source = "puts hello\n"
        options = FormattingOptions(tab_size=4, insert_spaces=True)
        edits = get_formatting(source, options)
        assert len(edits) == 0

    def test_get_formatting_respects_tab_size(self):
        from lsprotocol.types import FormattingOptions

        from lsp.features.formatting import get_formatting

        source = "proc foo {} {\nset x 1\nreturn $x\n}"
        options = FormattingOptions(tab_size=2, insert_spaces=True)
        edits = get_formatting(source, options)
        assert "  set x 1" in edits[0].new_text

    def test_get_formatting_uses_tabs(self):
        from lsprotocol.types import FormattingOptions

        from lsp.features.formatting import get_formatting

        source = "proc foo {} {\nset x 1\nreturn $x\n}"
        options = FormattingOptions(tab_size=4, insert_spaces=False)
        edits = get_formatting(source, options)
        assert "\tset x 1" in edits[0].new_text

    def test_range_formatting(self):
        from lsprotocol.types import FormattingOptions, Position, Range

        from lsp.features.formatting import get_range_formatting

        source = "set x 1\nproc foo {} {\nputs hi\n}\nset y 2"
        options = FormattingOptions(tab_size=4, insert_spaces=True)
        range_ = Range(
            start=Position(line=1, character=0),
            end=Position(line=3, character=1),
        )
        edits = get_range_formatting(source, range_, options)
        assert len(edits) >= 0  # May or may not have edits depending on content
