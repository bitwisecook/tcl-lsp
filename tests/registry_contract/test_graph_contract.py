"""Structural invariants of the event / profile / object graphs.

These assert graph integrity that any front-end (Python today, Rust
later) must satisfy, independent of the exact contents: stable event
ordering, well-formed flow chains, and closed reference edges in the
profile and object graphs.
"""

from __future__ import annotations

from ._harness import load_fixture

_EVENTS = load_fixture("events.json")
_PROFILES = load_fixture("profiles.json")
_OBJECTS = load_fixture("objects.json")


def test_master_order_is_a_stable_unique_sequence() -> None:
    order = [step["event"] for step in _EVENTS["masterOrder"]]
    assert len(order) == len(set(order)), "duplicate events in masterOrder"

    order_index = {e["event"]: e["orderIndex"] for e in _EVENTS["events"]}
    for position, event in enumerate(order):
        assert order_index.get(event) == position, (
            f"{event}: orderIndex={order_index.get(event)} but masterOrder position={position}"
        )


def test_every_ordered_event_is_in_master_order() -> None:
    ordered = {e["event"] for e in _EVENTS["events"] if e["orderIndex"] is not None}
    in_master = {step["event"] for step in _EVENTS["masterOrder"]}
    assert ordered == in_master


def test_flow_chains_are_well_formed() -> None:
    profile_names = set(_PROFILES["profiles"])
    for chain_id, chain in _EVENTS["flowChains"].items():
        assert chain["chain_id"] == chain_id
        assert isinstance(chain["steps"], list) and chain["steps"]
        for profile in chain["profiles"]:
            assert profile in profile_names, f"{chain_id} references unknown profile {profile}"


def test_event_validity_digests_are_present() -> None:
    for event in _EVENTS["events"]:
        if event["known"]:
            assert event["validCommandsDigest"].startswith("sha256:")
            assert event["validCommandCount"] >= 0


def test_profile_entries_are_self_consistent() -> None:
    for name, profile in _PROFILES["profiles"].items():
        assert profile["name"] == name


def test_protocol_namespace_profiles_are_known() -> None:
    profile_names = set(_PROFILES["profiles"])
    for prefix, spec in _PROFILES["protocolNamespaces"].items():
        assert spec["prefix"] == prefix
        for profile in spec["profiles"]:
            assert profile in profile_names, f"{prefix} -> unknown profile {profile}"


def test_object_kinds_are_unique() -> None:
    kinds = [k["kind"] for k in _OBJECTS["objectKinds"]]
    assert len(kinds) == len(set(kinds)), "duplicate object kind slugs"


def test_header_kind_map_targets_known_kinds() -> None:
    kinds = {k["kind"] for k in _OBJECTS["objectKinds"]}
    for entry in _OBJECTS["headerKindMap"]:
        assert entry["kind"] in kinds, f"headerKindMap -> unknown kind {entry['kind']}"


def test_property_reference_edges_are_strings() -> None:
    for prop in _OBJECTS["propertyReferences"]:
        assert isinstance(prop["property"], str) and prop["property"]
        for ref in prop["references"]:
            assert isinstance(ref, str) and ref, "empty object reference edge"
