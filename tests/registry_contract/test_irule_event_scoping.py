"""Registry-generated iRules event-scoping behaviour (IRULE1001 / IRULE1002).

For every iRules command the front-end is fed a ``when EVENT { command }``
snippet: an any-event command must never warn, a command used outside its
valid events must raise IRULE1001, and an unknown event must raise
IRULE1002.  The valid-event set comes from the ``command-info`` front-end,
so this links two front-end surfaces through the registry.
"""

from __future__ import annotations

from ._generators import event_scoping_cases, unknown_event_case


def test_generated_event_scoping_diagnostics() -> None:
    cases = list(event_scoping_cases("f5-irules"))
    assert len(cases) > 200, f"expected a broad event-scoping sweep, got {len(cases)}"
    failures = [msg for case in cases if (msg := case.check()) is not None]
    assert not failures, f"{len(failures)}/{len(cases)} event-scoping cases failed:\n" + "\n".join(
        failures[:40]
    )


def test_unknown_event_is_reported() -> None:
    failure = unknown_event_case("f5-irules").check()
    assert failure is None, failure
