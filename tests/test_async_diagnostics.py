"""Tests for the async diagnostic scheduler."""

from __future__ import annotations

import asyncio
import sys
import time
from pathlib import Path
from typing import Any

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from lsprotocol import types

from lsp.async_diagnostics import DiagnosticScheduler


def _diag(code: str) -> types.Diagnostic:
    """Create a minimal Diagnostic for testing."""
    return types.Diagnostic(
        range=types.Range(
            start=types.Position(line=0, character=0),
            end=types.Position(line=0, character=1),
        ),
        message=code,
        code=code,
    )


def _make_deep_fn(codes: list[str] | None = None, delay: float = 0) -> Any:
    """Create a synchronous deep-diagnostics function for testing."""
    result = [_diag(c) for c in (codes or [])]

    def fn() -> list[types.Diagnostic]:
        if delay:
            time.sleep(delay)
        return list(result)

    return fn


def _make_publish_fn() -> Any:
    """Create a mock publish function that records calls."""
    calls: list[tuple[str, list[types.Diagnostic], int | None]] = []

    def fn(uri: str, diagnostics: list[types.Diagnostic], version: int | None) -> None:
        calls.append((uri, diagnostics, version))

    return fn, calls


def _codes(diags: list[types.Diagnostic]) -> list[str]:
    """Extract codes from a list of diagnostics."""
    return [d.code for d in diags if d.code]  # type: ignore[misc]


class TestSchedulerBasic:
    """Basic scheduling and publishing."""

    def test_schedule_runs_deep_fn(self):
        """Scheduling a deep pass should call the deep function."""

        async def _run():
            scheduler = DiagnosticScheduler()
            deep_fn = _make_deep_fn(["D1"])
            basic = [_diag("B1")]
            publish_fn, calls = _make_publish_fn()

            scheduler.schedule("file:///a.tcl", 1, basic, deep_fn, publish_fn)
            await asyncio.sleep(0.2)

            assert len(calls) == 1
            uri, diags, version = calls[0]
            assert uri == "file:///a.tcl"
            assert _codes(diags) == ["B1", "D1"]
            assert version == 1

        asyncio.run(_run())

    def test_merges_basic_and_deep(self):
        """Published diagnostics should be basic_diags + deep_diags."""

        async def _run():
            scheduler = DiagnosticScheduler()
            basic = [_diag("B1"), _diag("B2")]
            deep_fn = _make_deep_fn(["D1", "D2", "D3"])
            publish_fn, calls = _make_publish_fn()

            scheduler.schedule("file:///a.tcl", 5, basic, deep_fn, publish_fn)
            await asyncio.sleep(0.2)

            assert len(calls) == 1
            assert _codes(calls[0][1]) == ["B1", "B2", "D1", "D2", "D3"]
            assert calls[0][2] == 5

        asyncio.run(_run())

    def test_empty_deep_result(self):
        """When deep pass returns no diagnostics, only basic are published."""

        async def _run():
            scheduler = DiagnosticScheduler()
            basic = [_diag("B1")]
            publish_fn, calls = _make_publish_fn()
            scheduler.schedule("file:///a.tcl", 1, basic, _make_deep_fn([]), publish_fn)
            await asyncio.sleep(0.2)

            assert len(calls) == 1
            assert _codes(calls[0][1]) == ["B1"]

        asyncio.run(_run())

    def test_version_none(self):
        """Version can be None (for unsaved documents)."""

        async def _run():
            scheduler = DiagnosticScheduler()
            publish_fn, calls = _make_publish_fn()
            scheduler.schedule("file:///a.tcl", None, [], _make_deep_fn(["D1"]), publish_fn)
            await asyncio.sleep(0.2)

            assert len(calls) == 1
            assert calls[0][2] is None

        asyncio.run(_run())


class TestSchedulerCancellation:
    """Cancellation of stale tasks."""

    def test_new_schedule_cancels_previous(self):
        """Scheduling for the same URI should cancel the previous task."""

        async def _run():
            scheduler = DiagnosticScheduler()
            publish_fn, calls = _make_publish_fn()

            # First schedule: slow deep function.
            scheduler.schedule(
                "file:///a.tcl",
                1,
                [_diag("B_v1")],
                _make_deep_fn(["SLOW"], delay=0.5),
                publish_fn,
            )

            # Second schedule: fast deep function (should cancel the first).
            await asyncio.sleep(0.01)
            scheduler.schedule(
                "file:///a.tcl",
                2,
                [_diag("B_v2")],
                _make_deep_fn(["FAST"]),
                publish_fn,
            )

            # Wait for the fast one to complete.
            await asyncio.sleep(0.2)

            # Only the second (fast) result should be published.
            uris = [c[0] for c in calls]
            assert uris.count("file:///a.tcl") == 1
            assert _codes(calls[-1][1]) == ["B_v2", "FAST"]
            assert calls[-1][2] == 2

        asyncio.run(_run())

    def test_different_uris_independent(self):
        """Tasks for different URIs should not cancel each other."""

        async def _run():
            scheduler = DiagnosticScheduler()
            publish_fn, calls = _make_publish_fn()

            scheduler.schedule("file:///a.tcl", 1, [_diag("A")], _make_deep_fn(["DA"]), publish_fn)
            scheduler.schedule("file:///b.tcl", 1, [_diag("B")], _make_deep_fn(["DB"]), publish_fn)

            await asyncio.sleep(0.3)

            uris = sorted(c[0] for c in calls)
            assert uris == ["file:///a.tcl", "file:///b.tcl"]

        asyncio.run(_run())

    def test_cancel_method(self):
        """Explicit cancel should prevent publishing."""

        async def _run():
            scheduler = DiagnosticScheduler()
            publish_fn, calls = _make_publish_fn()

            scheduler.schedule(
                "file:///a.tcl",
                1,
                [_diag("B")],
                _make_deep_fn(["SLOW"], delay=0.5),
                publish_fn,
            )

            await asyncio.sleep(0.01)
            scheduler.cancel("file:///a.tcl")

            await asyncio.sleep(0.2)
            assert len(calls) == 0

        asyncio.run(_run())

    def test_cancel_nonexistent_uri_is_safe(self):
        """Cancelling a URI with no pending task should be a no-op."""
        scheduler = DiagnosticScheduler()
        scheduler.cancel("file:///nonexistent.tcl")  # Should not raise

    def test_cancel_all(self):
        """cancel_all should stop all pending tasks."""

        async def _run():
            scheduler = DiagnosticScheduler()
            publish_fn, calls = _make_publish_fn()

            scheduler.schedule(
                "file:///a.tcl",
                1,
                [_diag("A")],
                _make_deep_fn(["DA"], delay=0.5),
                publish_fn,
            )
            scheduler.schedule(
                "file:///b.tcl",
                1,
                [_diag("B")],
                _make_deep_fn(["DB"], delay=0.5),
                publish_fn,
            )

            await asyncio.sleep(0.01)
            scheduler.cancel_all()

            await asyncio.sleep(0.2)
            assert len(calls) == 0

        asyncio.run(_run())


class TestSchedulerErrorHandling:
    """Error handling in deep functions."""

    def test_deep_fn_exception_does_not_crash(self):
        """If the deep function raises, we log and skip publishing."""

        async def _run():
            scheduler = DiagnosticScheduler()

            def failing_fn() -> list[types.Diagnostic]:
                raise ValueError("boom")

            publish_fn, calls = _make_publish_fn()
            scheduler.schedule("file:///a.tcl", 1, [_diag("B")], failing_fn, publish_fn)

            await asyncio.sleep(0.3)

            # No diagnostics should be published when deep pass fails.
            assert len(calls) == 0

        asyncio.run(_run())

    def test_deep_fn_exception_cleans_up_pending(self):
        """After a deep function fails, the pending entry should be cleaned up."""

        async def _run():
            scheduler = DiagnosticScheduler()

            def failing_fn() -> list[types.Diagnostic]:
                raise RuntimeError("oops")

            publish_fn, calls = _make_publish_fn()
            scheduler.schedule("file:///a.tcl", 1, [_diag("B")], failing_fn, publish_fn)

            await asyncio.sleep(0.3)

            # The pending dict should be empty after failure.
            assert "file:///a.tcl" not in scheduler._pending

        asyncio.run(_run())

    def test_subsequent_schedule_after_failure(self):
        """A new schedule should work after a previous one failed."""

        async def _run():
            scheduler = DiagnosticScheduler()

            def failing_fn() -> list[types.Diagnostic]:
                raise RuntimeError("oops")

            def good_fn() -> list[types.Diagnostic]:
                return [_diag("RECOVERED")]

            publish_fn, calls = _make_publish_fn()

            # First: fails
            scheduler.schedule("file:///a.tcl", 1, [_diag("B1")], failing_fn, publish_fn)
            await asyncio.sleep(0.2)

            # Second: should succeed
            scheduler.schedule("file:///a.tcl", 2, [_diag("B2")], good_fn, publish_fn)
            await asyncio.sleep(0.2)

            assert len(calls) == 1
            assert _codes(calls[0][1]) == ["B2", "RECOVERED"]
            assert calls[0][2] == 2

        asyncio.run(_run())


class TestSchedulerRapidUpdates:
    """Simulate rapid typing with many updates."""

    def test_rapid_updates_only_last_published(self):
        """With many rapid schedules, only the last one should publish."""

        async def _run():
            scheduler = DiagnosticScheduler()
            publish_fn, calls = _make_publish_fn()

            for i in range(10):
                scheduler.schedule(
                    "file:///a.tcl",
                    i,
                    [_diag(f"B{i}")],
                    _make_deep_fn([f"D{i}"], delay=0.05),
                    publish_fn,
                )
                await asyncio.sleep(0.005)

            # Wait for the last task to complete.
            await asyncio.sleep(0.5)

            # Only the final version should have been published.
            assert len(calls) == 1
            assert calls[0][2] == 9
            assert _codes(calls[0][1]) == ["B9", "D9"]

        asyncio.run(_run())
