"""Completion, end-to-end against the packaged server.

Ported from ``tests/test_completion.py``.  The text-edit assertions matter on
the live surface: an editor applies ``textEdit`` verbatim, so a wrong replace
range is exactly the class of regression the in-process test can mask.
"""

from __future__ import annotations

import textwrap

from ._lsp_helpers import completion_items, completion_labels


class TestCommandCompletion:
    def test_empty_line_returns_commands(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "")
        labels = completion_labels(lsp_server.completion(uri, 0, 0))
        assert {"set", "proc", "puts"} <= set(labels)

    def test_partial_command(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "pu")
        labels = completion_labels(lsp_server.completion(uri, 0, 2))
        assert "puts" in labels
        assert "set" not in labels

    def test_no_math_operators(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "")
        labels = completion_labels(lsp_server.completion(uri, 0, 0))
        assert "+" not in labels
        assert "-" not in labels

    def test_user_proc_in_completions(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "proc myHelper {x} { return $x }\nmy")
        labels = completion_labels(lsp_server.completion(uri, 1, 2))
        assert "myHelper" in labels


class TestVariableCompletion:
    def test_dollar_triggers_vars(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set greeting hello\nputs $")
        labels = completion_labels(lsp_server.completion(uri, 1, 6))
        assert "$greeting" in labels

    def test_partial_var_name(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set greeting hello\nset goodbye bye\nputs $gre")
        labels = completion_labels(lsp_server.completion(uri, 2, 9))
        assert "$greeting" in labels
        assert "$goodbye" not in labels

    def test_var_in_proc_scope(self, lsp_server, uri_factory):
        uri = uri_factory()
        src = textwrap.dedent("""\
            proc foo {x} {
                set local 1
                puts $
            }
        """)
        lsp_server.open_ready(uri, src)
        labels = completion_labels(lsp_server.completion(uri, 2, 10))
        assert "$x" in labels
        assert "$local" in labels

    def test_dollar_text_edit_replaces_dollar_sign(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set testvar Test\nputs $")
        items = {i["label"]: i for i in completion_items(lsp_server.completion(uri, 1, 6))}
        assert "$testvar" in items
        edit = items["$testvar"].get("textEdit")
        assert edit is not None
        assert edit["range"]["start"]["character"] == 5
        assert edit["range"]["end"]["character"] == 6
        assert edit["newText"] == "$testvar"

    def test_dollar_text_edit_brace_form(self, lsp_server, uri_factory):
        uri = uri_factory()
        lsp_server.open_ready(uri, "set greeting hello\nputs ${gre")
        items = {i["label"]: i for i in completion_items(lsp_server.completion(uri, 1, 10))}
        assert "$greeting" in items
        edit = items["$greeting"].get("textEdit")
        assert edit is not None
        assert edit["range"]["start"]["character"] == 5
        assert edit["range"]["end"]["character"] == 10
        assert edit["newText"] == "${greeting}"
