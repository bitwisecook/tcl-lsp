"""Tests for the hover provider."""

from __future__ import annotations

import sys
import textwrap
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from core.commands.registry.runtime import configure_signatures
from lsp.features.hover import get_hover


def _hover_text(result: types.Hover) -> str:
    contents = result.contents
    if isinstance(contents, types.MarkupContent):
        return contents.value
    if isinstance(contents, list):
        parts: list[str] = []
        for item in contents:
            if isinstance(item, types.MarkedStringWithLanguage):
                parts.append(item.value)
            else:
                parts.append(str(item))
        return "\n".join(parts)
    if isinstance(contents, types.MarkedStringWithLanguage):
        return contents.value
    return str(contents)


class TestCommandHover:
    def test_builtin_command(self):
        result = get_hover("set x 42", 0, 1)
        assert result is not None
        text = _hover_text(result)
        assert "set" in text
        assert "variable" in text.lower()

    def test_puts_hover(self):
        result = get_hover("puts hello", 0, 2)
        assert result is not None
        assert "puts" in _hover_text(result)

    def test_unknown_command_no_hover(self):
        result = get_hover("mycommand arg", 0, 4)
        assert result is None

    def test_subcommand_parent_hover(self):
        result = get_hover("string length hello", 0, 2)
        assert result is not None
        text = _hover_text(result)
        assert "subcommand" in text.lower() or "string" in text

    def test_socket_hover_uses_registry_snippet(self):
        result = get_hover("socket localhost 80", 0, 1)
        assert result is not None
        text = _hover_text(result)
        assert "socket ?options? host port" in text
        assert "tcp client or server socket" in text.lower()

    def test_socket_server_option_hover(self):
        result = get_hover("socket -server accept 8080", 0, 8)
        assert result is not None
        text = _hover_text(result)
        assert "-server" in text
        assert "callback" in text.lower()

    def test_socket_server_option_hover_ignores_semicolon_in_quoted_arg(self):
        source = 'socket "a;b" -server accept 8080'
        option_pos = source.index("-server") + 2
        result = get_hover(source, 0, option_pos)
        assert result is not None
        text = _hover_text(result)
        assert "-server" in text

    def test_irules_subcommand_hover(self):
        configure_signatures(dialect="f5-irules")
        result = get_hover("HTTP::header insert X-Test 1", 0, 15)
        assert result is not None
        text = _hover_text(result)
        assert "insert" in text.lower()
        assert "header" in text.lower()

    def test_generated_command_hover_marks_refinement_status(self):
        configure_signatures(dialect="f5-irules")
        result = get_hover("ACCESS::acl match all", 0, 5)
        assert result is not None
        text = _hover_text(result)
        assert "note:" in text.lower()

    def test_curated_command_hover_does_not_mark_refinement_status(self):
        result = get_hover("append x y", 0, 1)
        assert result is not None
        text = _hover_text(result)
        assert "note:" not in text.lower()

    def test_generated_irules_hover_marks_refinement_status(self):
        configure_signatures(dialect="f5-irules")
        result = get_hover("ACCESS::acl result", 0, 5)
        assert result is not None
        text = _hover_text(result)
        assert "note:" in text.lower()

    def test_curated_irules_hover_does_not_mark_refinement_status(self):
        configure_signatures(dialect="f5-irules")
        result = get_hover('when HTTP_REQUEST { log local0. "ok" }', 0, 2)
        assert result is not None
        text = _hover_text(result)
        assert "note:" not in text.lower()


class TestProcHover:
    def test_proc_signature(self):
        source = textwrap.dedent("""\
            proc greet {name} { puts "Hello $name" }
            greet World
        """)
        result = get_hover(source, 1, 2)
        assert result is not None
        text = _hover_text(result)
        assert "greet" in text
        assert "name" in text

    def test_proc_with_doc(self):
        source = textwrap.dedent("""\
            # Says hello to someone
            proc greet {name} { puts "Hello $name" }
            greet World
        """)
        result = get_hover(source, 2, 2)
        assert result is not None
        assert "Says hello" in _hover_text(result)

    def test_proc_with_defaults(self):
        source = textwrap.dedent("""\
            proc greet {{name World}} { puts "Hello $name" }
            greet
        """)
        result = get_hover(source, 1, 2)
        assert result is not None
        assert "World" in _hover_text(result)


class TestVariableHover:
    def test_var_hover(self):
        source = "set x 42\nputs $x"
        result = get_hover(source, 1, 7)
        assert result is not None
        text = _hover_text(result)
        assert "Variable" in text
        assert "x" in text

    def test_var_hover_shows_refs(self):
        source = "set x 42\nputs $x"
        result = get_hover(source, 1, 7)
        assert result is not None
        assert "reference" in _hover_text(result).lower()

    def test_namespace_var_hover(self):
        source = textwrap.dedent("""\
            namespace eval myns {
                variable nsVar 1
                puts $nsVar
            }
        """)
        result = get_hover(source, 2, 10)
        assert result is not None
        text = _hover_text(result)
        assert "Variable" in text
        assert "nsVar" in text


class TestLeanHover:
    """Verify hover shows synopsis + summary but omits snippet/examples."""

    def test_set_hover_omits_snippet(self):
        result = get_hover("set x 42", 0, 1)
        assert result is not None
        text = _hover_text(result)
        # Synopsis and summary present
        assert "set varName" in text
        assert "Read or write" in text
        # Snippet detail NOT present (moved to signature help)
        assert "With one argument" not in text

    def test_socket_hover_omits_snippet(self):
        result = get_hover("socket localhost 80", 0, 1)
        assert result is not None
        text = _hover_text(result)
        # Synopsis still present
        assert "socket ?options? host port" in text
        # Snippet NOT present
        assert "listening socket" not in text

    def test_option_hover_omits_snippet(self):
        result = get_hover("socket -myaddr 0.0.0.0 localhost 80", 0, 8)
        assert result is not None
        text = _hover_text(result)
        assert "-myaddr" in text
        # Snippet NOT present
        assert "multi-homed" not in text


class TestFormatStringHover:
    """Tests for format-string decode hover tooltips."""

    def test_sprintf_format_hover(self):
        result = get_hover('format "%s %d" hello 123', 0, 9)
        assert result is not None
        text = _hover_text(result)
        assert "sprintf" in text.lower() or "format string" in text.lower()
        assert "String" in text
        assert "integer" in text.lower()

    def test_sprintf_format_hover_flags(self):
        result = get_hover('format "%-10.5f" 3.14', 0, 9)
        assert result is not None
        text = _hover_text(result)
        assert "float" in text.lower() or "Floating" in text

    def test_clock_format_hover(self):
        result = get_hover('clock format $t -format "%Y-%m-%d"', 0, 26)
        assert result is not None
        text = _hover_text(result)
        assert "year" in text.lower()
        assert "month" in text.lower()

    def test_binary_format_hover(self):
        result = get_hover("binary format c2s $x $y", 0, 14)
        assert result is not None
        text = _hover_text(result)
        assert "int8" in text
        assert "int16" in text
        # Hybrid layout: byte-ruler diagram + detail table
        assert "binary format" in text
        assert "4 bytes" in text

    def test_regsub_subspec_hover(self):
        result = get_hover(r"regsub {pat} $str {\1} result", 0, 20)
        assert result is not None
        text = _hover_text(result)
        assert "capture" in text.lower()

    def test_glob_pattern_hover(self):
        result = get_hover("string match {*.tcl} $f", 0, 15)
        assert result is not None
        text = _hover_text(result)
        assert "sequence" in text.lower() or "Matches" in text

    def test_non_format_arg_no_hover(self):
        # cursor on "hello" which is not a format string arg
        result = get_hover('puts "hello"', 0, 7)
        assert result is None or "format" not in _hover_text(result).lower()

    def test_binary_scan_hover(self):
        result = get_hover("binary scan $data cu2 x y", 0, 18)
        assert result is not None
        text = _hover_text(result)
        assert "uint8" in text
        assert "binary scan" in text

    def test_binary_format_hover_with_absolute_seek(self):
        result = get_hover("binary format c3S4i@20f 1 2 3 4 5 6 7 8 9", 0, 15)
        assert result is not None
        text = _hover_text(result)
        assert "binary format" in text
        assert "24 bytes" in text

    def test_regexp_pattern_hover(self):
        result = get_hover(r"regexp {^(\d+)\s+(\w+)$} $line", 0, 10)
        assert result is not None
        text = _hover_text(result)
        assert "Regex pattern" in text
        assert "anchor" in text.lower()
        assert "Capture group" in text
        assert "Digit" in text

    def test_regexp_pattern_hover_char_class(self):
        result = get_hover(r"regexp {[a-z]+} $str", 0, 10)
        assert result is not None
        text = _hover_text(result)
        assert "Character class" in text

    def test_regexp_with_options_hover(self):
        result = get_hover(r"regexp -nocase -- {^hello} $str", 0, 22)
        assert result is not None
        text = _hover_text(result)
        assert "Regex pattern" in text
        assert "anchor" in text.lower()

    def test_regsub_pattern_hover(self):
        result = get_hover(r"regsub {(\w+)} $str {\1} result", 0, 10)
        assert result is not None
        text = _hover_text(result)
        assert "Regex pattern" in text
        assert "Word character" in text

    def test_regexp_literal_no_metachar(self):
        result = get_hover(r"regexp {hello} $str", 0, 10)
        assert result is not None
        text = _hover_text(result)
        assert "Literal" in text or "no metacharacters" in text.lower()
