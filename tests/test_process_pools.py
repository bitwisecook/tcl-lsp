"""The cold-build and deep-diagnostics process pools must stay separate.

A single shared 2-worker pool let a burst of multi-second cold builds block
every open document's deep-diagnostics pass (head-of-line blocking).  Cold and
deep now use distinct lazily-created pools.
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import server.state as st


def test_cold_and_deep_pools_are_distinct_and_sized(monkeypatch):
    created: list[dict] = []

    class _FakePool:
        def __init__(self, **kwargs):
            created.append(kwargs)

        def shutdown(self, **_kwargs):
            pass

    monkeypatch.setattr(st, "ProcessPoolExecutor", _FakePool)
    monkeypatch.setattr(st, "_process_pool", None)
    monkeypatch.setattr(st, "_deep_pool", None)

    cold = st._get_process_pool()
    deep = st._get_deep_pool()

    # Distinct pools — a cold-build storm cannot starve deep diagnostics.
    assert cold is not deep
    # Lazy singletons within each role.
    assert st._get_process_pool() is cold
    assert st._get_deep_pool() is deep
    # Cold scales with the machine (>= deep's fixed size); both have >= 2.
    assert created[0]["max_workers"] == st._COLD_POOL_WORKERS >= 2
    assert created[1]["max_workers"] == st._DEEP_POOL_WORKERS >= 2
    assert st._COLD_POOL_WORKERS >= st._DEEP_POOL_WORKERS


def test_reset_process_pool_poisons_and_recreates(monkeypatch):
    """A wedged/broken cold pool is torn down (shutdown) and the next get builds
    a fresh one — the poison-recreate the ceiling and BrokenProcessPool use."""
    shutdowns: list[dict] = []
    created: list[dict] = []

    class _FakePool:
        def __init__(self, **kwargs):
            created.append(kwargs)

        def shutdown(self, **kwargs):
            shutdowns.append(kwargs)

    monkeypatch.setattr(st, "ProcessPoolExecutor", _FakePool)
    monkeypatch.setattr(st, "_process_pool", None)

    first = st._get_process_pool()
    st._reset_process_pool()

    # Old pool was shut down without waiting, cancelling queued futures.
    assert shutdowns == [{"wait": False, "cancel_futures": True}]
    # Reference cleared, so the next get builds a brand-new pool.
    assert st._process_pool is None
    second = st._get_process_pool()
    assert second is not first
    assert len(created) == 2


def test_reset_process_pool_is_noop_when_unset(monkeypatch):
    monkeypatch.setattr(st, "_process_pool", None)
    st._reset_process_pool()  # must not raise
    assert st._process_pool is None
