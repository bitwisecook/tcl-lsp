"""Event-graph behaviour: generated multi-event iRules, ordered by the front-end.

An iRule containing many ``when EVENT { }`` blocks in scrambled order is
fed to ``f5 irule event-order``; the front-end must return them in the
registry's canonical firing order.  This exercises the event graph
through a real artifact rather than a data dump.
"""

from __future__ import annotations

import random

from compiler.registry.namespace_registry import NAMESPACE_REGISTRY

from ._harness import f5_event_order

# The canonical firing order, restricted to events that have an order index.
_ORDERED = [
    name
    for name in sorted(
        NAMESPACE_REGISTRY.all_event_names(),
        key=lambda n: NAMESPACE_REGISTRY.event_index(n) or 0,
    )
    if NAMESPACE_REGISTRY.event_index(name) is not None
]


def _expected_order(events: set[str]) -> list[str]:
    return [e for e in _ORDERED if e in events]


def test_scrambled_events_are_returned_in_registry_order() -> None:
    rng = random.Random(1234)
    # A spread of events across the connection lifecycle.
    pick = rng.sample(_ORDERED, k=min(12, len(_ORDERED)))
    scrambled = pick[:]
    rng.shuffle(scrambled)

    source = "".join(f"when {event} {{ }}\n" for event in scrambled)
    served = f5_event_order(source)

    assert served == _expected_order(set(pick)), (
        f"front-end order {served} != registry order {_expected_order(set(pick))}"
    )


def test_full_lifecycle_orders_deterministically() -> None:
    # Every ordered event in one iRule, scrambled, must come back canonical.
    rng = random.Random(99)
    scrambled = _ORDERED[:]
    rng.shuffle(scrambled)
    source = "".join(f"when {event} {{ }}\n" for event in scrambled)
    assert f5_event_order(source) == _ORDERED
