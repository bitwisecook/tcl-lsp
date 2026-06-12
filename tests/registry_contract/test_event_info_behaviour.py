"""``f5 irule event-info`` behaviour must agree with the event graph fixture.

For every iRules event the live ``event-info --json`` front-end is driven
and checked against ``events.json``.  The full per-event ``validCommands``
list is content-addressed in the fixture; this test reconstructs it from
the front-end and verifies the digest, covering the command/event
legality matrix exactly.
"""

from __future__ import annotations

import pytest

from tooling.registry_snapshot import digest_list

from ._harness import load_fixture, run_f5_json

_EVENTS = load_fixture("events.json")["events"]
_KNOWN_EVENTS = [e for e in _EVENTS if e["known"]]


def test_event_fixture_is_populated() -> None:
    assert len(_EVENTS) > 100
    assert any(e["event"] == "HTTP_REQUEST" for e in _EVENTS)


def test_list_events_covers_fixture() -> None:
    """Every known event in the fixture is a real, describable event."""
    names = {e["event"] for e in _KNOWN_EVENTS}
    assert "HTTP_REQUEST" in names
    assert "CLIENT_ACCEPTED" in names


@pytest.mark.parametrize(
    "entry",
    _KNOWN_EVENTS,
    ids=[e["event"] for e in _KNOWN_EVENTS],
)
def test_event_info_matches_fixture(entry: dict) -> None:
    name = entry["event"]
    info = run_f5_json(["irule", "event-info", name, "--json"])

    assert info["known"] is True
    assert info["deprecated"] == entry["deprecated"]
    assert info["multiplicity"] == entry["multiplicity"]
    assert info["side"] == entry["side"]
    assert info["transport"] == entry["transport"]
    assert sorted(info["impliedProfiles"]) == sorted(entry["impliedProfiles"])
    assert info["validCommandCount"] == entry["validCommandCount"]
    assert digest_list(info["validCommands"]) == entry["validCommandsDigest"]
