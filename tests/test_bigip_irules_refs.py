"""Tests for parser-backed iRules object reference extraction."""

from __future__ import annotations

from core.bigip.irules_refs import extract_irules_object_references


def test_extract_irules_refs_for_pool_snatpool_and_datagroup() -> None:
    source = """
when HTTP_REQUEST {
    if {[class match -- [HTTP::host] equals /Common/host_dg]} {
        snatpool /Common/sp1
        pool /Common/web_pool
    }
}
"""
    refs = extract_irules_object_references(source)
    by_name = {(ref.command, ref.name): ref.kinds for ref in refs}

    assert ("class", "/Common/host_dg") in by_name
    assert ("snatpool", "/Common/sp1") in by_name
    assert ("pool", "/Common/web_pool") in by_name


def test_extract_irules_refs_nested_in_body_and_command_substitution() -> None:
    source = """
when HTTP_REQUEST {
    set count [active_members /Common/app_pool]
    LB::reselect pool /Common/fallback_pool
}
"""
    refs = extract_irules_object_references(source)
    names = [ref.name for ref in refs]
    assert "/Common/app_pool" in names
    assert "/Common/fallback_pool" in names
