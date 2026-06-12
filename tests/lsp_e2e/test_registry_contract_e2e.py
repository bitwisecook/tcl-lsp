"""The LSP front-end must surface the same registry as the golden fixtures.

These drive the real ``workspace/executeCommand`` registry-lookup
handlers (``describeIruleEvent``, ``listIruleEvents``,
``describeIruleCommand``, ``listSubcommands``) against the committed
presence CSVs.  Because the e2e harness also drives the native server
when ``TCL_LSP_SERVER_KIND=rust`` is set, these become the cross-language
contract: a Rust server passes them unchanged.
"""

from __future__ import annotations

import csv
from pathlib import Path

_BASELINE = Path(__file__).resolve().parents[1] / "baselines" / "registry"


def _csv(name: str) -> list[dict[str, str]]:
    with (_BASELINE / name).open(encoding="utf-8", newline="") as fh:
        return list(csv.DictReader(fh))


_EVENTS = _csv("events.csv")
_KNOWN_EVENTS = [e for e in _EVENTS if e["known"] == "true"]
_COMMANDS = _csv("commands.csv")
_IRULES = sorted(r["command"] for r in _COMMANDS if r["dialect"] == "f5-irules")
_COMMAND_SAMPLE = _IRULES[::7]

# A handful of well-known core ensembles and the subcommands the LSP must
# surface for them.  ``listSubcommands`` resolves dialect-agnostically, so
# this checks named coverage rather than exact counts (the exhaustive,
# dialect-filtered subcommand sweep lives in tests/registry_contract).
_ENSEMBLE_SUBCOMMANDS = {
    "string": {"length", "range", "map", "tolower", "trim"},
    "dict": {"get", "set", "keys", "exists", "merge"},
    "info": {"body", "exists", "commands", "vars"},
    "file": {"exists", "join", "dirname", "extension"},
    "clock": {"seconds", "format", "scan"},
    "array": {"get", "set", "names", "exists"},
}


def test_list_irule_events_matches_known_events(lsp_server_irules) -> None:
    data = lsp_server_irules.execute_command("tcl-lsp.listIruleEvents", [])
    served = set(data["events"])
    expected = {e["event"] for e in _KNOWN_EVENTS}
    assert served == expected, f"missing={expected - served} extra={served - expected}"


def test_describe_every_known_event_count_matches_csv(lsp_server_irules) -> None:
    failures: list[str] = []
    for entry in _KNOWN_EVENTS:
        data = lsp_server_irules.execute_command("tcl-lsp.describeIruleEvent", [entry["event"]])
        if str(data["validCommandCount"]) != entry["valid_commands"]:
            failures.append(
                f"{entry['event']}: lsp={data['validCommandCount']} csv={entry['valid_commands']}"
            )
        if str(data["deprecated"]).lower() != entry["deprecated"]:
            failures.append(f"{entry['event']}: deprecated mismatch")
    assert not failures, "\n".join(failures[:40])


def test_describe_sampled_irule_commands_are_found(lsp_server_irules) -> None:
    failures = []
    for command in _COMMAND_SAMPLE:
        data = lsp_server_irules.execute_command("tcl-lsp.describeIruleCommand", [command])
        if data.get("found") is not True:
            failures.append(command)
    assert not failures, f"describeIruleCommand not found for: {failures[:20]}"


def test_list_subcommands_covers_known_ensembles(lsp_server) -> None:
    failures: list[str] = []
    for command, expected in _ENSEMBLE_SUBCOMMANDS.items():
        data = lsp_server.execute_command("tcl-lsp.listSubcommands", [command])
        served = {s["name"] for s in data["subcommands"]}
        missing = expected - served
        if missing:
            failures.append(f"{command}: LSP missing subcommands {sorted(missing)}")
    assert not failures, "\n".join(failures)
