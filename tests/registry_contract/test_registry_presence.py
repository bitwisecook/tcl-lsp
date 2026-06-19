"""Presence safety-net: the committed CSVs must match the live registry.

The CSVs under ``tests/baselines/registry/`` pin that every command (with
its arity and subcommand/switch counts), event, profile, and object is
present in the registry.  ``check_all`` regenerates the rows straight from
the registry and compares them to the committed CSVs, so this single test
guards both presence and staleness.
"""

from __future__ import annotations

from ._harness import BASELINE_DIR


def test_registry_fixtures_are_not_stale() -> None:
    from scripts.codegen.registry_baselines import check_all

    problems = check_all(BASELINE_DIR)
    assert not problems, "stale registry fixtures (run make gen-registry-baselines):\n" + "\n".join(
        problems
    )
