"""F5 iRules dialect features, end-to-end against a dedicated server.

These run against ``lsp_server_irules`` rather than the shared ``lsp_server``:
opening an iRules document auto-switches the server's process-global command
pack into the ``f5-irules`` dialect, so dialect-sensitive cases are isolated
on their own server to keep the main Tcl server uncontaminated.

Ported from the iRules cases in ``tests/test_hover.py``.
"""

from __future__ import annotations

from ._lsp_helpers import hover_text


def _hover(lsp_server, uri, line, char):
    return hover_text(lsp_server.hover(uri, line, char))


class TestIrulesHover:
    def test_irules_subcommand_hover(self, lsp_server_irules, uri_factory):
        uri = uri_factory("irule")
        lsp_server_irules.open_ready(uri, "HTTP::header insert X-Test 1\n", language_id="tcl-irule")
        text = _hover(lsp_server_irules, uri, 0, 15)
        assert "insert" in text.lower()
        assert "header" in text.lower()

    def test_curated_irules_hover_does_not_mark_refinement_status(
        self, lsp_server_irules, uri_factory
    ):
        uri = uri_factory("irule")
        lsp_server_irules.open_ready(
            uri, 'when HTTP_REQUEST { log local0. "ok" }\n', language_id="tcl-irule"
        )
        assert "note:" not in _hover(lsp_server_irules, uri, 0, 2).lower()

    def test_namespace_only_irules_hover_shows_profile_requirement(
        self, lsp_server_irules, uri_factory
    ):
        uri = uri_factory("irule")
        lsp_server_irules.open_ready(uri, 'ACCESS::log 1 "trace"\n', language_id="tcl-irule")
        text = _hover(lsp_server_irules, uri, 0, 5)
        assert "Requires" in text
        assert "ACCESS" in text
