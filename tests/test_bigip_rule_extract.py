"""Regression tests for BIG-IP embedded iRule extraction ranges."""

from __future__ import annotations

from core.bigip.rule_extract import find_embedded_rules, find_rule_at_offset


def test_find_embedded_rules_range_maps_to_braces() -> None:
    source = (
        "ltm rule /Common/my_irule {\n"
        "    when HTTP_REQUEST {\n"
        "        pool /Common/web_pool\n"
        "    }\n"
        "}\n"
    )
    rules = find_embedded_rules(source)
    assert len(rules) == 1
    rule = rules[0]

    assert rule.range.start.line == 0
    assert rule.range.start.character == 0
    assert source[rule.range.start.offset : rule.range.start.offset + 8] == "ltm rule"

    assert source[rule.range.end.offset] == "}"
    assert rule.range.end.line == 4
    assert rule.range.end.character == 0


def test_find_rule_at_offset_inside_block() -> None:
    source = 'ltm rule /Common/a {\n    when HTTP_REQUEST { log local0. "a" }\n}\n'
    offset = source.index("HTTP_REQUEST")
    rule = find_rule_at_offset(source, offset)
    assert rule is not None
    assert rule.full_path == "/Common/a"
