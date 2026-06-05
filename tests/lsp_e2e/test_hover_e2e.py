"""Hover, end-to-end against the packaged server.

Ported from ``tests/test_hover.py`` (the in-process provider tests) onto the
live JSON-RPC surface: open a document, wait for analysis, ask
``textDocument/hover`` at a position, assert on the rendered markdown.
"""

from __future__ import annotations

import textwrap

from ._lsp_helpers import hover_text


class TestCommandHover:
    def test_builtin_command(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 42\n")
        text = hover_text(lsp_server.hover(uri, 0, 1))
        assert "set" in text
        assert "variable" in text.lower()

    def test_puts_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "puts hello\n")
        assert "puts" in hover_text(lsp_server.hover(uri, 0, 2))

    def test_unknown_command_no_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "mycommand arg\n")
        assert not lsp_server.hover(uri, 0, 4)

    def test_subcommand_parent_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "string length hello\n")
        text = hover_text(lsp_server.hover(uri, 0, 2))
        assert "subcommand" in text.lower() or "string" in text

    def test_socket_hover_uses_registry_snippet(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "socket localhost 80\n")
        text = hover_text(lsp_server.hover(uri, 0, 1))
        assert "socket ?options? host port" in text
        assert "tcp client or server socket" in text.lower()

    def test_socket_server_option_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "socket -server accept 8080\n")
        text = hover_text(lsp_server.hover(uri, 0, 8))
        assert "-server" in text
        assert "callback" in text.lower()


class TestProcHover:
    def test_proc_signature(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'proc greet {name} { puts "Hello $name" }\ngreet World\n')
        text = hover_text(lsp_server.hover(uri, 1, 2))
        assert "greet" in text
        assert "name" in text

    def test_proc_with_doc(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = "# Says hello to someone\nproc greet {name} { puts $name }\ngreet World\n"
        lsp_server.open_ready(uri, src)
        assert "Says hello" in hover_text(lsp_server.hover(uri, 2, 2))

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
        text = hover_text(lsp_server.hover(uri, 5, 2))
        assert "Calculate the sum" in text
        assert "**Parameters:**" in text
        assert "**Returns:**" in text


class TestVariableHover:
    def test_var_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 42\nputs $x\n")
        text = hover_text(lsp_server.hover(uri, 1, 7))
        assert "Variable" in text
        assert "x" in text

    def test_var_hover_shows_refs(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set x 42\nputs $x\n")
        assert "reference" in hover_text(lsp_server.hover(uri, 1, 7)).lower()


class TestFormatStringHover:
    def test_sprintf_format_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, 'format "%s %d" hello 123\n')
        text = hover_text(lsp_server.hover(uri, 0, 9))
        assert "sprintf" in text.lower() or "format string" in text.lower()
        assert "integer" in text.lower()

    def test_regexp_pattern_hover(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, r"regexp {^(\d+)\s+(\w+)$} $line" + "\n")
        text = hover_text(lsp_server.hover(uri, 0, 10))
        assert "Regex pattern" in text
        assert "Capture group" in text
