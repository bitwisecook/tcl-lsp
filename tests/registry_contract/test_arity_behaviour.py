"""Registry-generated arity behaviour, asserted through ``tcl diag``.

For every plain-positional command in every Tcl dialect, a call with too
few / too many / exactly-min arguments is synthesised from the registry
arity and fed to the front-end; the front-end must raise (or withhold)
the matching arity diagnostic.
"""

from __future__ import annotations

import pytest

from tooling.registry_snapshot import ALL_DIALECTS

from ._generators import arity_cases


@pytest.mark.parametrize("dialect", ALL_DIALECTS)
def test_generated_arity_diagnostics(dialect: str) -> None:
    cases = list(arity_cases(dialect))
    assert cases, f"no arity cases generated for {dialect}"
    failures = [msg for case in cases if (msg := case.check()) is not None]
    assert not failures, (
        f"{dialect}: {len(failures)}/{len(cases)} arity cases failed:\n" + "\n".join(failures[:40])
    )
