"""Tests for shared iRules event-context detection helper."""

from __future__ import annotations

from lsp.features.irules_context import find_enclosing_when_event


def test_find_enclosing_when_event_simple_block() -> None:
    source = "when CLIENT_DATA {\n    set payload [TCP::payload]\n}\n"
    event, anchor = find_enclosing_when_event(source, 1)
    assert event == "CLIENT_DATA"
    assert anchor == 0


def test_find_enclosing_when_event_prefers_innermost_nested_when() -> None:
    source = (
        'when CLIENT_ACCEPTED {\n    when HTTP_REQUEST {\n        log local0. "nested"\n    }\n}\n'
    )
    event, anchor = find_enclosing_when_event(source, 2)
    assert event == "HTTP_REQUEST"
    assert anchor == 1


def test_find_enclosing_when_event_returns_none_outside_events() -> None:
    source = "set x 1\nputs $x\n"
    event, anchor = find_enclosing_when_event(source, 1)
    assert event is None
    assert anchor == 1
