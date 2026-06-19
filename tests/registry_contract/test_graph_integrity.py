"""Structural integrity of the event / profile / object graphs.

Sourced from the temporary registry dumper (``scripts/registry/dump.py``;
no committed JSON): stable event ordering, well-formed flow chains, and
closed reference edges.  Any front-end — Python today, Rust later — must
satisfy these.
"""

from __future__ import annotations

import pytest

from ._harness import run_dump_json


@pytest.fixture(scope="module")
def events() -> dict:
    return run_dump_json(["f5", "--section", "events"])


@pytest.fixture(scope="module")
def profiles() -> dict:
    return run_dump_json(["f5", "--section", "profiles"])


@pytest.fixture(scope="module")
def objects() -> dict:
    return run_dump_json(["f5", "--section", "objects"])


def test_master_order_is_unique_and_aligned(events: dict) -> None:
    order = [step["event"] for step in events["masterOrder"]]
    assert len(order) == len(set(order)), "duplicate events in masterOrder"
    index = {e["event"]: e["orderIndex"] for e in events["events"]}
    for position, event in enumerate(order):
        assert index.get(event) == position


def test_every_ordered_event_is_in_master_order(events: dict) -> None:
    ordered = {e["event"] for e in events["events"] if e["orderIndex"] is not None}
    in_master = {step["event"] for step in events["masterOrder"]}
    assert ordered == in_master


def test_flow_chains_reference_known_profiles(events: dict, profiles: dict) -> None:
    profile_names = set(profiles["profiles"])
    for chain_id, chain in events["flowChains"].items():
        assert chain["chain_id"] == chain_id
        assert chain["steps"]
        for profile in chain["profiles"]:
            assert profile in profile_names, f"{chain_id} -> unknown profile {profile}"


def test_protocol_namespace_profiles_are_known(profiles: dict) -> None:
    profile_names = set(profiles["profiles"])
    for prefix, spec in profiles["protocolNamespaces"].items():
        assert spec["prefix"] == prefix
        for profile in spec["profiles"]:
            assert profile in profile_names, f"{prefix} -> unknown profile {profile}"


def test_object_kinds_unique_and_header_map_closed(objects: dict) -> None:
    kinds = [k["kind"] for k in objects["objectKinds"]]
    assert len(kinds) == len(set(kinds)), "duplicate object kinds"
    kind_set = set(kinds)
    for entry in objects["headerKindMap"]:
        assert entry["kind"] in kind_set, f"headerKindMap -> unknown kind {entry['kind']}"
