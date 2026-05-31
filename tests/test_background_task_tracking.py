"""Fire-and-forget publish tasks must be kept alive until they finish.

asyncio holds only a weak reference to a bare ``create_task`` result, so an
untracked analysis task can be garbage-collected mid-flight and silently drop a
document's diagnostics.  ``spawn_publish_diagnostics`` holds a strong reference
in ``_background_tasks`` until completion.
"""

from __future__ import annotations

import asyncio
import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from server import diagnostics_pipeline as dp


def test_spawned_task_is_tracked_then_released(monkeypatch):
    started = asyncio.Event()
    release = asyncio.Event()
    calls: list[tuple] = []

    async def _fake_publish(uri, source, version=None, *, force_reanalyse=False):
        calls.append((uri, source, version, force_reanalyse))
        started.set()
        await release.wait()

    monkeypatch.setattr(dp, "_publish_diagnostics", _fake_publish)

    async def _drive():
        assert not dp._background_tasks
        task = dp.spawn_publish_diagnostics("file:///t.tcl", "set x 1\n", 1)
        await started.wait()
        # Held by the module while in flight — not relying on the caller's ref.
        assert task in dp._background_tasks
        release.set()
        await task
        # Done-callback discards it so the set doesn't leak across edits.
        assert task not in dp._background_tasks
        assert not dp._background_tasks

    asyncio.run(_drive())
    assert calls == [("file:///t.tcl", "set x 1\n", 1, False)]


def test_spawn_forwards_force_reanalyse_and_loop(monkeypatch):
    calls: list[tuple] = []

    async def _fake_publish(uri, source, version=None, *, force_reanalyse=False):
        calls.append((uri, source, version, force_reanalyse))

    monkeypatch.setattr(dp, "_publish_diagnostics", _fake_publish)

    async def _drive():
        loop = asyncio.get_running_loop()
        task = dp.spawn_publish_diagnostics(
            "file:///r.tcl", "set y 2\n", 7, force_reanalyse=True, loop=loop
        )
        await task

    asyncio.run(_drive())
    assert calls == [("file:///r.tcl", "set y 2\n", 7, True)]
