"""Presence safety-net: the front-end registry-dump must match the golden CSVs.

The CSVs pin that every command (with its arity and counts), event,
profile, and object is present in the registry.  These tests drive the
``tcl registry-dump`` / ``f5 registry-dump`` front-ends and compare their
output to the committed CSVs, and verify the CSVs are not stale.
"""

from __future__ import annotations

import pytest

from tooling.registry_snapshot import ALL_DIALECTS, F5_DIALECTS

from ._harness import (
    BASELINE_DIR,
    load_csv,
    run_f5_json,
    run_tcl_json,
    run_tcl_subprocess,
)


def _csv_commands(dialect: str) -> dict[str, tuple[str, str, str, str]]:
    rows = load_csv("commands.csv")
    return {
        r["command"]: (r["arity_min"], r["arity_max"], r["subcommands"], r["switches"])
        for r in rows
        if r["dialect"] == dialect
    }


def _dump_commands(payload: dict) -> dict[str, tuple[str, str, str, str]]:
    out: dict[str, tuple[str, str, str, str]] = {}
    for entry in payload["commands"]:
        arity = entry["arity"] or {"min": "", "max": None}
        amin = "" if arity["min"] is None else str(arity["min"])
        amax = "" if arity["max"] is None else str(arity["max"])
        out[entry["name"]] = (
            amin,
            amax,
            str(len(entry["subcommands"])),
            str(len(entry["switches"])),
        )
    return out


@pytest.mark.parametrize("dialect", ALL_DIALECTS)
def test_command_dump_matches_csv(dialect: str) -> None:
    """Every command in the CSV is present with matching arity/counts in the front-end dump."""
    expected = _csv_commands(dialect)
    produced = _dump_commands(run_tcl_json(["registry-dump", "--dialect", dialect]))
    assert produced == expected, (
        f"{dialect}: dump/CSV mismatch — "
        f"only-in-csv={sorted(set(expected) - set(produced))[:10]}, "
        f"only-in-dump={sorted(set(produced) - set(expected))[:10]}"
    )


def test_events_dump_matches_csv() -> None:
    rows = {r["event"]: r for r in load_csv("events.csv")}
    data = run_f5_json(["registry-dump", "--section", "events"])
    served = {e["event"]: e for e in data["events"]}
    assert set(served) == set(rows)
    for name, row in rows.items():
        e = served[name]
        assert str(e["known"]).lower() == row["known"]
        assert str(e["deprecated"]).lower() == row["deprecated"]
        assert e["side"] == row["side"]
        assert e["multiplicity"] == row["multiplicity"]
        assert str(e["validCommandCount"]) == row["valid_commands"]


def test_profiles_dump_matches_csv() -> None:
    rows = {r["profile"]: r for r in load_csv("profiles.csv")}
    data = run_f5_json(["registry-dump", "--section", "profiles"])
    assert set(data["profiles"]) == set(rows)
    for name, row in rows.items():
        prof = data["profiles"][name]
        assert str(prof["layer"]) == row["layer"]
        assert str(prof["side"]) == row["side"]


def test_objects_dump_matches_csv() -> None:
    rows = {r["kind"]: r for r in load_csv("objects.csv")}
    data = run_f5_json(["registry-dump", "--section", "objects"])
    served = {k["kind"]: k for k in data["objectKinds"]}
    assert set(served) == set(rows)
    for kind, row in rows.items():
        spec = served[kind]
        assert (spec["module"] or "") == row["module"]
        assert "|".join(spec["object_types"]) == row["object_types"]


def test_registry_fixtures_are_not_stale() -> None:
    from scripts.codegen.registry_baselines import check_all

    problems = check_all(BASELINE_DIR)
    assert not problems, "stale registry fixtures (run make gen-registry-baselines):\n" + "\n".join(
        problems
    )


@pytest.mark.parametrize("dialect", F5_DIALECTS)
def test_f5_command_dump_runs(dialect: str) -> None:
    payload = run_f5_json(["registry-dump", "--section", "commands"])
    assert payload[dialect]["commandCount"] > 0


def test_dump_verb_runs_as_real_subprocess() -> None:
    """Smoke: the shipped console-script path emits the same presence as the CSV."""
    produced = _dump_commands(run_tcl_subprocess(["registry-dump", "--dialect", "tcl8.6"]))
    assert produced == _csv_commands("tcl8.6")
