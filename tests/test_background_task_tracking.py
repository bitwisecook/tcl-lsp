# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

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
