"""The LSP front-end must surface the same registry as the golden fixtures.

These drive the real ``workspace/executeCommand`` registry-lookup
handlers (``describeIruleEvent``, ``describeIruleCommand``,
``listIruleEvents``, ``listSubcommands``) against the committed registry
fixtures.  Because the e2e harness also drives the native server when
``TCL_LSP_SERVER_KIND=rust`` is set, these become the cross-language
contract: a Rust server passes them unchanged.

The exhaustive per-command sweep lives in the fast in-process
``tests/registry_contract`` suite; here we prove the *LSP surface* agrees
with that contract over the full event list, every subcommand-bearing
command, and a broad command sample.
"""

from __future__ import annotations

import hashlib
import json
from pathlib import Path

import pytest

_BASELINE = Path(__file__).resolve().parents[1] / "baselines" / "registry"


def _load(name: str) -> dict:
    return json.loads((_BASELINE / name).read_text(encoding="utf-8"))


def _digest(items: list[str]) -> str:
    return "sha256:" + hashlib.sha256("\n".join(sorted(items)).encode("utf-8")).hexdigest()


_EVENTS = _load("events.json")["events"]
_KNOWN_EVENTS = [e for e in _EVENTS if e["known"]]
_IRULES = {c["name"]: c for c in _load("commands-f5-irules.json")["commands"]}
# A broad but bounded command sweep for the per-command LSP lookups.
_COMMAND_SAMPLE = sorted(_IRULES)[::7]

_ALL_DIALECTS = ("tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "f5-irules", "f5-iapps")


def _subcommand_bounds() -> dict[str, tuple[frozenset[str], frozenset[str], bool]]:
    """Map command -> (intersection, union, strict) of subcommand names.

    The LSP ``listSubcommands`` handler resolves a command dialect-agnostically
    (``get_any``), so a command overloaded across dialects (e.g. ``event`` —
    a subcommand ensemble in core Tcl but a bare command in f5-iRules) yields
    exactly one dialect's subcommand set, which may be empty.

    ``strict`` is ``True`` only when the command declares subcommands in
    *every* dialect it exists in; then the served set must be non-empty and
    within ``[intersection, union]``.  Otherwise the served set need only be a
    subset of the union (an empty result is a legitimate bare-form overload).
    """
    sub_sets: dict[str, list[frozenset[str]]] = {}
    exists_without_subs: set[str] = set()
    for dialect in _ALL_DIALECTS:
        for c in _load(f"commands-{dialect}.json")["commands"]:
            if c["subcommands"]:
                names = frozenset(s["name"] for s in c["subcommands"])
                sub_sets.setdefault(c["name"], []).append(names)
            else:
                exists_without_subs.add(c["name"])
    bounds: dict[str, tuple[frozenset[str], frozenset[str], bool]] = {}
    for command, sets in sub_sets.items():
        strict = command not in exists_without_subs
        bounds[command] = (frozenset.intersection(*sets), frozenset.union(*sets), strict)
    return bounds


_SUBCMD_BOUNDS = _subcommand_bounds()


def test_list_irule_events_matches_known_events(lsp_server_irules) -> None:
    data = lsp_server_irules.execute_command("tcl-lsp.listIruleEvents", [])
    served = set(data["events"])
    expected = {e["event"] for e in _KNOWN_EVENTS}
    assert served == expected, f"missing={expected - served} extra={served - expected}"


def test_describe_every_known_event_count_matches_fixture(lsp_server_irules) -> None:
    failures: list[str] = []
    for entry in _KNOWN_EVENTS:
        data = lsp_server_irules.execute_command("tcl-lsp.describeIruleEvent", [entry["event"]])
        if data["validCommandCount"] != entry["validCommandCount"]:
            failures.append(
                f"{entry['event']}: lsp={data['validCommandCount']} "
                f"fixture={entry['validCommandCount']}"
            )
        if data["deprecated"] != entry["deprecated"]:
            failures.append(f"{entry['event']}: deprecated mismatch")
    assert not failures, "\n".join(failures[:40])


@pytest.mark.parametrize("command", _COMMAND_SAMPLE)
def test_describe_irule_command_matches_fixture(lsp_server_irules, command: str) -> None:
    data = lsp_server_irules.execute_command("tcl-lsp.describeIruleCommand", [command])
    assert data["found"] is True
    expected = _IRULES[command]
    assert data["switches"] == expected["switches"]
    if "validEvents" in data:
        assert _digest(data["validEvents"]) == expected["info"]["validEventsDigest"]


def test_list_subcommands_covers_registry_subcommands(lsp_server) -> None:
    failures: list[str] = []
    for command, (low, high, strict) in _SUBCMD_BOUNDS.items():
        data = lsp_server.execute_command("tcl-lsp.listSubcommands", [command])
        served = frozenset(s["name"] for s in data["subcommands"])
        if not served <= high:
            failures.append(f"{command}: served {sorted(served - high)} not in any dialect")
        elif strict and not (served and low <= served):
            failures.append(f"{command}: served {sorted(served)} below required {sorted(low)}")
    assert not failures, "\n".join(failures[:40])
