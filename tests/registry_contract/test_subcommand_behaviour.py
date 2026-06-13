"""Registry-generated subcommand behaviour, asserted through ``tcl diag``.

For every ensemble command (one with subcommands) in every Tcl dialect,
the front-end must report a bare call as "requires a subcommand", a
bogus subcommand as unknown, and must not flag a real subcommand.
"""

from __future__ import annotations

import pytest

from tooling.registry_snapshot import ALL_DIALECTS

from ._generators import subcommand_cases


@pytest.mark.parametrize("dialect", ALL_DIALECTS)
def test_generated_subcommand_diagnostics(dialect: str) -> None:
    cases = list(subcommand_cases(dialect))
    assert cases, f"no subcommand cases generated for {dialect}"
    failures = [msg for case in cases if (msg := case.check()) is not None]
    assert not failures, (
        f"{dialect}: {len(failures)}/{len(cases)} subcommand cases failed:\n"
        + "\n".join(failures[:40])
    )
