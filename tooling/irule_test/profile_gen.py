"""Generate synthetic rule-profiler logs by running an iRule under test.

The iRule testing framework already mocks every extension command and
fires events; with the (default-off) ``::itest::profiler`` instrumentation
enabled, a test run emits a count- and sequence-accurate occurrence stream
in the F5 BIG-IP rule-profiler CSV format — *without a real BIG-IP*.

This module drives that: given an iRule and a list of stimuli (events to
fire, optionally with connection/request state and a repeat count), it runs
them through :class:`~tooling.irule_test.bridge.IruleTestSessionSync`,
collects the profiler stream, and returns it as log text or a
:class:`~compiler.pgo.profile_data.ProfileData`.

**Accuracy caveat:** timestamps are a monotonic ordinal, not wall-clock,
so this drives *frequency*-based profile-guided optimisation (which branch
is hot) — not real performance numbers.  As on a real BIG-IP, bytecode-
compiled core commands (``set``, ``if`` …) do not appear as command
occurrences.
"""

from __future__ import annotations

from dataclasses import dataclass, field
from typing import Any

from compiler.pgo.profile_data import ProfileData
from compiler.pgo.trace_parser import parse_f5_occurrences

from .bridge import IruleTestSessionSync


@dataclass(frozen=True)
class Stimulus:
    """One unit of traffic to drive the iRule with.

    ``state`` is applied (per layer, e.g. ``{"connection": {"client_addr":
    "10.0.0.2"}}``) before the event fires; ``event`` is then fired
    ``repeat`` times.  Setting state once and repeating models a burst of
    like traffic — the common case for surfacing a hot branch.
    """

    event: str = "HTTP_REQUEST"
    state: dict[str, dict[str, Any]] = field(default_factory=dict)
    repeat: int = 1


def generate_occurrence_log(
    irule_source: str,
    stimuli: list[Stimulus],
    *,
    profiles: list[str] | None = None,
    backend: str = "auto",
) -> str:
    """Run *irule_source* against *stimuli* and return rule-profiler log text.

    Returns an empty string if the test framework backend is unavailable.
    """
    try:
        session_cm = IruleTestSessionSync(
            profiles=profiles or ["TCP", "HTTP"], backend=backend
        )
    except Exception:
        return ""
    try:
        with session_cm as session:
            session.load_irule(irule_source)
            session.eval_tcl("::itest::profiler::enable")
            for stim in stimuli:
                for layer, values in stim.state.items():
                    session.set_state(layer, values)
                for _ in range(max(1, stim.repeat)):
                    session.fire_event(stim.event)
            return session.eval_tcl("::itest::profiler::dump")
    except Exception:
        return ""


def generate_profile(
    irule_source: str,
    stimuli: list[Stimulus],
    *,
    profiles: list[str] | None = None,
    backend: str = "auto",
) -> ProfileData:
    """Run *irule_source* against *stimuli* and return aggregated profile data."""
    log = generate_occurrence_log(
        irule_source, stimuli, profiles=profiles, backend=backend
    )
    return ProfileData.from_occurrences(parse_f5_occurrences(log), source="test-gen")
