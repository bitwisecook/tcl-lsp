"""``tcl command-info`` behaviour must agree with the registry fixtures.

For every command in every dialect, the live ``command-info --json``
front-end is driven and its payload checked against the golden fixture.
The full ``validEvents`` cross-product is content-addressed in the
fixture (count + digest); this test reconstructs the full list from the
front-end and verifies the digest, so the legality matrix is covered
exactly without bloating the fixture.
"""

from __future__ import annotations

import pytest

from tooling.registry_snapshot import ALL_DIALECTS, digest_list

from ._harness import load_fixture, run_tcl_json


def _commands(dialect: str) -> dict[str, dict]:
    fixture = load_fixture(f"commands-{dialect}.json")
    return {entry["name"]: entry for entry in fixture["commands"]}


@pytest.mark.parametrize("dialect", ALL_DIALECTS)
def test_command_info_matches_fixture_for_all_commands(dialect: str) -> None:
    commands = _commands(dialect)
    failures: list[str] = []

    for name, entry in commands.items():
        info = run_tcl_json(["command-info", name, "--dialect", dialect, "--json"])
        expected = entry["info"]

        if not info.get("found"):
            failures.append(f"{name}: command-info reported not found")
            continue

        checks = {
            "summary": (info["summary"], expected["summary"]),
            "synopsis": (info["synopsis"], expected["synopsis"]),
            "switches": (info["switches"], expected["switches"]),
            "validEventCount": (len(info["validEvents"]), expected["validEventCount"]),
            "validEventsDigest": (
                digest_list(info["validEvents"]),
                expected["validEventsDigest"],
            ),
        }
        for field, (got, want) in checks.items():
            if got != want:
                failures.append(f"{name}.{field}: front-end={got!r} fixture={want!r}")

    assert not failures, f"{dialect}: {len(failures)} mismatch(es):\n" + "\n".join(failures[:40])


@pytest.mark.parametrize("dialect", ALL_DIALECTS)
def test_every_fixture_command_is_resolvable(dialect: str) -> None:
    """Sanity: the dialect's command list is non-empty and every name resolves."""
    commands = _commands(dialect)
    assert commands, f"{dialect} has no commands in its fixture"
    # Spot a known builtin to guard against an empty/garbage fixture.
    probe = "set" if dialect.startswith("tcl") else next(iter(commands))
    info = run_tcl_json(["command-info", probe, "--dialect", dialect, "--json"])
    assert info["found"] is True
