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


class TestDeepDiagnosticsWorkerContext:
    """The deep-diagnostics subprocess (``_run_deep_diagnostics``) does not share
    the parent's ContextVars, so it must re-establish the per-folder command
    overlay (``extra_commands``, issue #407) and the workspace ``.tcl.stubs`` —
    exactly as ``_analyse_document_fresh`` does for the cold build.  Otherwise
    the deep pass re-lexes against a bare registry and mis-resolves folder-scoped
    / stub-declared commands.  These tests call the worker in-process (the pool
    re-applies state per call, so the effect is observable on the registry)."""

    def _snapshot(self):
        from compiler.registry import stub_comments
        from compiler.registry.runtime import _dialect_var, _extra_commands_var

        return (
            _dialect_var.get(),
            _extra_commands_var.get(),
            stub_comments._ambient_cmd_stubs_var.get(),
        )

    def _restore(self, snap):
        from compiler.registry import stub_comments
        from compiler.registry.runtime import _dialect_var, _extra_commands_var

        _dialect_var.set(snap[0])
        _extra_commands_var.set(snap[1])
        stub_comments._ambient_cmd_stubs_var.set(snap[2])

    def test_worker_reestablishes_extra_commands(self):
        from compiler.registry.runtime import _extra_commands_var
        from server.features.diagnostics import _run_deep_diagnostics

        snap = self._snapshot()
        try:
            _run_deep_diagnostics(
                "set x 1\n",
                {},
                "tcl8.6",
                extra_commands=("mySentinelEdaCmd",),
            )
            # The worker forwarded the overlay into configure_signatures, so the
            # registry ContextVar now carries it (was () before the fix).
            assert "mySentinelEdaCmd" in _extra_commands_var.get()
        finally:
            self._restore(snap)

    def test_worker_reestablishes_workspace_stubs(self):
        from lsprotocol.types import Position, Range

        from compiler.registry.stub_comments import StubCommandDef, ambient_cmd_stubs
        from server.features.diagnostics import _run_deep_diagnostics

        rng = Range(start=Position(0, 0), end=Position(0, 0))
        stub = StubCommandDef(
            name="mySentinelStubCmd",
            args=(),
            range=rng,
            barrier=True,
            loop=False,
            pure=False,
            mutator=False,
            unsafe=False,
            scope_alias=False,
            subcommand=None,
        )

        snap = self._snapshot()
        try:
            _run_deep_diagnostics(
                "set x 1\n",
                {},
                "tcl8.6",
                stub_commands=(stub,),
            )
            assert any(s.name == "mySentinelStubCmd" for s in ambient_cmd_stubs())
        finally:
            self._restore(snap)
