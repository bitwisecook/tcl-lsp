"""Hover, end-to-end against the packaged server.

Full-parity port of ``tests/test_hover.py`` (the in-process provider tests)
onto the live JSON-RPC surface: open a document, wait for analysis, ask
``textDocument/hover`` at a position, assert on the rendered markdown.  The
iRules-dialect cases open the document with the ``tcl-irule`` language id (and
an ``.irule`` URI) so the server resolves the F5 command pack — the wire
equivalent of the in-process ``configure_signatures(dialect="f5-irules")``.

The white-box test that ``_infer_var_type`` / ``_infer_var_taint`` agree on the
cached vs recomputed path has no JSON-RPC surface and stays in
``tests/test_hover.py``.
"""

from __future__ import annotations

import textwrap

from ._lsp_helpers import hover_text


def _hover(lsp_server, uri, line, char):
    return hover_text(lsp_server.hover(uri, line, char))


class TestCommandHover:
    def test_builtin_command(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 42\n")
        text = _hover(lsp_server, uri, 0, 1)
        assert "set" in text
        assert "variable" in text.lower()

    def test_puts_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "puts hello\n")
        assert "puts" in _hover(lsp_server, uri, 0, 2)

    def test_unknown_command_no_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "mycommand arg\n")
        assert not lsp_server.hover(uri, 0, 4)

    def test_subcommand_parent_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "string length hello\n")
        text = _hover(lsp_server, uri, 0, 2)
        assert "subcommand" in text.lower() or "string" in text

    def test_socket_hover_uses_registry_snippet(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "socket localhost 80\n")
        text = _hover(lsp_server, uri, 0, 1)
        assert "socket ?options? host port" in text
        assert "tcp client or server socket" in text.lower()

    def test_socket_server_option_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "socket -server accept 8080\n")
        text = _hover(lsp_server, uri, 0, 8)
        assert "-server" in text
        assert "callback" in text.lower()

    def test_socket_server_option_hover_ignores_semicolon_in_quoted_arg(
        self, lsp_server, uri_factory
    ):
        uri = uri_factory()
        source = 'socket "a;b" -server accept 8080\n'
        lsp_server.open_ready(uri, source)
        option_pos = source.index("-server") + 2
        text = _hover(lsp_server, uri, 0, option_pos)
        assert "-server" in text

    def test_curated_command_hover_does_not_mark_refinement_status(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "append x y\n")
        assert "note:" not in _hover(lsp_server, uri, 0, 1).lower()


class TestProcHover:
    def test_proc_signature(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'proc greet {name} { puts "Hello $name" }\ngreet World\n')
        text = _hover(lsp_server, uri, 1, 2)
        assert "greet" in text
        assert "name" in text

    def test_proc_with_doc(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = '# Says hello to someone\nproc greet {name} { puts "Hello $name" }\ngreet World\n'
        lsp_server.open_ready(uri, src)
        assert "Says hello" in _hover(lsp_server, uri, 2, 2)

    def test_proc_multiline_doc(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            # Greet a user.
            # This proc says hello.
            proc greet {name} { puts "Hello $name" }
            greet World
        """)
        lsp_server.open_ready(uri, src)
        text = _hover(lsp_server, uri, 3, 2)
        assert "Greet a user." in text
        assert "This proc says hello." in text

    def test_proc_body_docstring(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc update_pkg {filename} {
                # ....................................................................
                # Update build date and time in pkg
                #
                # @param filename - Filename to pkg file
                # ....................................................................
                puts $filename
            }
            update_pkg test.pkg
        """)
        lsp_server.open_ready(uri, src)
        text = _hover(lsp_server, uri, 8, 2)
        assert "Update build date" in text
        assert "**filename**" in text

    def test_proc_no_comment_bleed(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc first {} {
                # internal comment
                puts hello
            }
            proc second {} { puts world }
            second
        """)
        lsp_server.open_ready(uri, src)
        assert "internal comment" not in _hover(lsp_server, uri, 5, 2)

    def test_proc_doxygen_tags(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            # @brief Calculate the sum
            # @param a - First number
            # @param b - Second number
            # @return The sum of a and b
            proc add {a b} { expr {$a + $b} }
            add 1 2
        """)
        lsp_server.open_ready(uri, src)
        text = _hover(lsp_server, uri, 5, 2)
        assert "Calculate the sum" in text
        assert "**Parameters:**" in text
        assert "**a**" in text
        assert "**b**" in text
        assert "**Returns:**" in text

    def test_proc_with_defaults(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'proc greet {{name World}} { puts "Hello $name" }\ngreet\n')
        assert "World" in _hover(lsp_server, uri, 1, 2)


class TestVariableHover:
    def test_var_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 42\nputs $x\n")
        text = _hover(lsp_server, uri, 1, 7)
        assert "Variable" in text
        assert "x" in text

    def test_var_hover_shows_refs(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 42\nputs $x\n")
        assert "reference" in _hover(lsp_server, uri, 1, 7).lower()

    def test_namespace_var_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            namespace eval myns {
                variable nsVar 1
                puts $nsVar
            }
        """)
        lsp_server.open_ready(uri, src)
        text = _hover(lsp_server, uri, 2, 10)
        assert "Variable" in text
        assert "nsVar" in text


class TestLeanHover:
    """Hover shows synopsis + summary but omits snippet/examples."""

    def test_set_hover_omits_snippet(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 42\n")
        text = _hover(lsp_server, uri, 0, 1)
        assert "set varName" in text
        assert "Read or write" in text
        assert "With one argument" not in text

    def test_socket_hover_omits_snippet(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "socket localhost 80\n")
        text = _hover(lsp_server, uri, 0, 1)
        assert "socket ?options? host port" in text
        assert "listening socket" not in text

    def test_option_hover_omits_snippet(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "socket -myaddr 0.0.0.0 localhost 80\n")
        text = _hover(lsp_server, uri, 0, 8)
        assert "-myaddr" in text
        assert "multi-homed" not in text


class TestFormatStringHover:
    def test_sprintf_format_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'format "%s %d" hello 123\n')
        text = _hover(lsp_server, uri, 0, 9)
        assert "sprintf" in text.lower() or "format string" in text.lower()
        assert "String" in text
        assert "integer" in text.lower()

    def test_sprintf_format_hover_flags(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'format "%-10.5f" 3.14\n')
        text = _hover(lsp_server, uri, 0, 9)
        assert "float" in text.lower() or "Floating" in text

    def test_clock_format_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'clock format $t -format "%Y-%m-%d"\n')
        text = _hover(lsp_server, uri, 0, 26)
        assert "year" in text.lower()
        assert "month" in text.lower()

    def test_binary_format_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "binary format c2s $x $y\n")
        text = _hover(lsp_server, uri, 0, 14)
        assert "int8" in text
        assert "int16" in text
        assert "binary format" in text
        assert "4 bytes" in text

    def test_regsub_subspec_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, r"regsub {pat} $str {\1} result" + "\n")
        text = _hover(lsp_server, uri, 0, 20)
        assert "capture" in text.lower()

    def test_glob_pattern_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "string match {*.tcl} $f\n")
        text = _hover(lsp_server, uri, 0, 15)
        assert "sequence" in text.lower() or "Matches" in text

    def test_non_format_arg_no_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'puts "hello"\n')
        result = lsp_server.hover(uri, 0, 7)
        assert not result or "format" not in hover_text(result).lower()

    def test_binary_scan_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "binary scan $data cu2 x y\n")
        text = _hover(lsp_server, uri, 0, 18)
        assert "uint8" in text
        assert "binary scan" in text

    def test_binary_format_hover_with_absolute_seek(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "binary format c3S4i@20f 1 2 3 4 5 6 7 8 9\n")
        text = _hover(lsp_server, uri, 0, 15)
        assert "binary format" in text
        assert "24 bytes" in text

    def test_regexp_pattern_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, r"regexp {^(\d+)\s+(\w+)$} $line" + "\n")
        text = _hover(lsp_server, uri, 0, 10)
        assert "Regex pattern" in text
        assert "anchor" in text.lower()
        assert "Capture group" in text
        assert "Digit" in text

    def test_regexp_pattern_hover_char_class(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, r"regexp {[a-z]+} $str" + "\n")
        assert "Character class" in _hover(lsp_server, uri, 0, 10)

    def test_regexp_with_options_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, r"regexp -nocase -- {^hello} $str" + "\n")
        text = _hover(lsp_server, uri, 0, 22)
        assert "Regex pattern" in text
        assert "anchor" in text.lower()

    def test_regsub_pattern_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, r"regsub {(\w+)} $str {\1} result" + "\n")
        text = _hover(lsp_server, uri, 0, 10)
        assert "Regex pattern" in text
        assert "Word character" in text

    def test_regexp_literal_no_metachar(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, r"regexp {hello} $str" + "\n")
        text = _hover(lsp_server, uri, 0, 10)
        assert "Literal" in text or "no metacharacters" in text.lower()


class TestAliasHover:
    def test_alias_hover_shows_target(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "interp alias {} = {} expr\n= {1 + 2}\n")
        text = _hover(lsp_server, uri, 1, 0)
        assert "Alias" in text
        assert "expr" in text

    def test_alias_hover_with_prepended_args(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "interp alias {} myput {} puts stdout\nmyput hello\n")
        text = _hover(lsp_server, uri, 1, 0)
        assert "Alias" in text
        assert "puts" in text
        assert "stdout" in text
