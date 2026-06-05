"""F5 iRules dialect features, end-to-end against a dedicated server.

These run against ``lsp_server_irules`` rather than the shared ``lsp_server``:
opening an iRules document auto-switches the server's process-global command
pack into the ``f5-irules`` dialect, so dialect-sensitive cases are isolated
on their own server to keep the main Tcl server uncontaminated.

Ported from the iRules cases in ``tests/test_hover.py``.
"""

from __future__ import annotations

from ._lsp_helpers import completion_items, completion_labels, hover_text


def _hover(lsp_server, uri, line, char):
    return hover_text(lsp_server.hover(uri, line, char))


def _open(lsp_server_irules, uri_factory, source):
    uri = uri_factory("irule")
    lsp_server_irules.open_ready(uri, source, language_id="tcl-irule")
    return uri


def _labels(lsp_server_irules, uri, line, char):
    return completion_labels(lsp_server_irules.completion(uri, line, char))


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


class TestIrulesCompletion:
    def test_when_event_name_completion(self, lsp_server_irules, uri_factory):
        uri = _open(lsp_server_irules, uri_factory, "when ")
        labels = _labels(lsp_server_irules, uri, 0, 5)
        assert "HTTP_REQUEST" in labels
        assert "CLIENT_ACCEPTED" in labels

    def test_when_priority_and_timing_keywords_after_event(self, lsp_server_irules, uri_factory):
        src = "when HTTP_REQUEST "
        uri = _open(lsp_server_irules, uri_factory, src)
        labels = _labels(lsp_server_irules, uri, 0, len(src))
        assert "priority" in labels
        assert "timing" in labels

    def test_when_priority_and_timing_partial_keyword(self, lsp_server_irules, uri_factory):
        src = "when HTTP_REQUEST pr"
        uri = _open(lsp_server_irules, uri_factory, src)
        labels = _labels(lsp_server_irules, uri, 0, len(src))
        assert "priority" in labels
        assert "timing" not in labels

    def test_when_timing_value_keywords_after_timing(self, lsp_server_irules, uri_factory):
        src = "when HTTP_REQUEST timing "
        uri = _open(lsp_server_irules, uri_factory, src)
        labels = _labels(lsp_server_irules, uri, 0, len(src))
        assert "enable" in labels
        assert "disable" in labels

    def test_when_timing_values_not_suggested_after_priority(self, lsp_server_irules, uri_factory):
        src = "when HTTP_REQUEST priority "
        uri = _open(lsp_server_irules, uri_factory, src)
        labels = _labels(lsp_server_irules, uri, 0, len(src))
        assert "enable" not in labels
        assert "disable" not in labels

    def test_http_header_subcommand_keywords(self, lsp_server_irules, uri_factory):
        uri = _open(lsp_server_irules, uri_factory, "HTTP::header ")
        labels = _labels(lsp_server_irules, uri, 0, 13)
        assert {"insert", "replace", "value"} <= set(labels)

    def test_http_header_partial_keyword(self, lsp_server_irules, uri_factory):
        uri = _open(lsp_server_irules, uri_factory, "HTTP::header re")
        labels = _labels(lsp_server_irules, uri, 0, 15)
        assert "remove" in labels
        assert "replace" in labels
        assert "insert" not in labels

    def test_http_respond_options_after_status_code(self, lsp_server_irules, uri_factory):
        uri = _open(lsp_server_irules, uri_factory, "HTTP::respond 302 ")
        labels = _labels(lsp_server_irules, uri, 0, 18)
        assert {"content", "noserver", "version"} <= set(labels)

    def test_irules_event_valid_command_ranked_before_invalid(self, lsp_server_irules, uri_factory):
        uri = _open(lsp_server_irules, uri_factory, "when HTTP_REQUEST {\n    \n}\n")
        by = {i["label"]: i for i in completion_items(lsp_server_irules.completion(uri, 1, 4))}
        assert by["HTTP::header"]["sortText"] < by["TCP::collect"]["sortText"]

    def test_when_priority_and_timing_after_priority_value(self, lsp_server_irules, uri_factory):
        src = "when HTTP_REQUEST priority 500 "
        uri = _open(lsp_server_irules, uri_factory, src)
        labels = _labels(lsp_server_irules, uri, 0, len(src))
        assert "priority" in labels
        assert "timing" in labels

    def test_argument_value_has_documentation(self, lsp_server_irules, uri_factory):
        uri = _open(lsp_server_irules, uri_factory, "when ")
        items = completion_items(lsp_server_irules.completion(uri, 0, 5))
        assert items
        assert any(
            i.get("documentation") is not None for i in items if i["label"] == "HTTP_REQUEST"
        )
