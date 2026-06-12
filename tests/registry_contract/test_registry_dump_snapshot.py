"""The ``registry-dump`` front-end output must match the golden fixtures.

This is the bulk shape contract: every command's arity, switches, forms,
subcommands, and boolean traits, plus the event / profile / object
graphs.  When the registry changes intentionally, regenerate with
``make gen-registry-baselines`` and commit the diff.
"""

from __future__ import annotations

import json

import pytest

from tooling.registry_snapshot import ALL_DIALECTS, F5_DIALECTS, TCL_DIALECTS

from ._harness import (
    BASELINE_DIR,
    load_fixture,
    run_f5_json,
    run_tcl_json,
    run_tcl_subprocess,
)


@pytest.mark.parametrize("dialect", ALL_DIALECTS)
def test_command_dump_matches_fixture(dialect: str) -> None:
    """``tcl registry-dump --dialect D`` reproduces the committed fixture."""
    produced = run_tcl_json(["registry-dump", "--dialect", dialect])
    expected = load_fixture(f"commands-{dialect}.json")
    assert produced == expected


def test_registry_fixtures_are_not_stale() -> None:
    """The committed fixtures match what the builders produce right now.

    Mirrors ``python scripts/codegen/registry_baselines.py --check`` so a
    plain ``pytest`` run flags registry drift even without ``make``.
    """
    from scripts.codegen.registry_baselines import check_all

    problems = check_all(BASELINE_DIR)
    assert not problems, "stale registry fixtures (run make gen-registry-baselines):\n" + "\n".join(
        problems
    )


@pytest.mark.parametrize("dialect", TCL_DIALECTS)
def test_all_dialects_dump_contains_each_dialect(dialect: str) -> None:
    """``--all-dialects`` bundles every Tcl dialect, each matching its fixture."""
    bundle = run_tcl_json(["registry-dump", "--all-dialects"])
    assert bundle["schema"] == "tcl-lsp/registry/commands/v1"
    assert bundle["dialects"][dialect] == load_fixture(f"commands-{dialect}.json")


@pytest.mark.parametrize("section", ["events", "profiles", "objects"])
def test_f5_graph_dump_matches_fixture(section: str) -> None:
    """``f5 registry-dump --section X`` reproduces the graph fixtures."""
    produced = run_f5_json(["registry-dump", "--section", section])
    assert produced == load_fixture(f"{section}.json")


@pytest.mark.parametrize("dialect", F5_DIALECTS)
def test_f5_command_dump_matches_fixture(dialect: str) -> None:
    produced = run_f5_json(["registry-dump", "--section", "commands"])
    assert produced[dialect] == load_fixture(f"commands-{dialect}.json")


def test_dump_verb_runs_as_real_subprocess() -> None:
    """Smoke: the shipped console-script path emits the same JSON.

    Proves the contract holds over a real process boundary (the mode a
    Rust front-end test harness uses), not just in-process.
    """
    produced = run_tcl_subprocess(["registry-dump", "--dialect", "tcl8.6"])
    assert produced == load_fixture("commands-tcl8.6.json")


def test_fixtures_are_canonical_json() -> None:
    """Every fixture is sorted-key, 2-space-indented JSON (stable diffs)."""
    for path in sorted(BASELINE_DIR.glob("*.json")):
        text = path.read_text(encoding="utf-8")
        reserialised = json.dumps(json.loads(text), indent=2, sort_keys=True) + "\n"
        assert text == reserialised, f"{path.name} is not canonical JSON"
