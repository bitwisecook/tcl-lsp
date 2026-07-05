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

"""Supersession guards in the diagnostics pipeline.

The process-pool ``await`` is a suspension point: a newer did_change can be
queued (bumping the latest-requested version) while an older version is still
analysing.  The older run must not apply its stale analysis to the document or
publish out-of-date diagnostics once it returns.
"""

from __future__ import annotations

import asyncio
import sys
from concurrent.futures import ThreadPoolExecutor
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import server.state as _state
from server import diagnostics_pipeline as dp
from server.workspace import document_state


class _FakeServer:
    """Records published diagnostics versions; no-ops the refresh hooks."""

    def __init__(self) -> None:
        self.published: list[tuple[str, int | None]] = []

    def text_document_publish_diagnostics(self, params) -> None:
        self.published.append((params.uri, params.version))

    def workspace_semantic_tokens_refresh(self, _arg) -> None:
        pass

    def workspace_folding_range_refresh(self, _arg) -> None:
        pass


def _run_publish_with_midflight_bump(monkeypatch, uri: str, src: str, version: int, bump_to: int):
    """Drive ``_publish_diagnostics`` for *version* where the analysis itself
    bumps the latest-requested version to *bump_to* (simulating a newer
    did_change queued mid-analysis).  Returns the FakeServer."""
    server = _FakeServer()
    monkeypatch.setattr(dp, "_server", server)
    # Force even a tiny source down the *cold* lane (size > threshold) so this
    # supersession guard exercises the subprocess await via _get_process_pool.
    monkeypatch.setattr(dp, "_SMALL_BUILD_MAX_BYTES", -1)
    # Run the cold-build "subprocess" in an in-process thread so the gated stub
    # is the one that executes.
    monkeypatch.setattr(_state, "_get_process_pool", lambda: ThreadPoolExecutor(max_workers=1))
    # Skip the background deep pass (it would schedule onto the pool too).
    monkeypatch.setattr(_state.diagnostic_scheduler, "schedule_async", lambda *a, **k: None)

    real_fresh = document_state._analyse_document_fresh

    def bump_then_analyse(**kwargs):
        # A newer version is requested while this analysis is in flight.
        dp._publish_latest_version[uri] = bump_to
        return real_fresh(**kwargs)

    monkeypatch.setattr(document_state, "_analyse_document_fresh", bump_then_analyse)
    asyncio.run(dp._publish_diagnostics(uri, src, version=version))
    return server


class TestFreshBuildLaneRouting:
    """Every fresh build runs in a subprocess pool (in-thread CPU fan-out is
    GIL-bound — slower than serial).  A small build uses the small-file lane
    (``_get_small_pool``) so it never queues behind a workspace of multi-second
    cold builds; a large build uses the cold pool (``_get_process_pool``)."""

    def _lanes_used_for(self, monkeypatch, uri: str, src: str) -> dict[str, bool]:
        server = _FakeServer()
        monkeypatch.setattr(dp, "_server", server)
        monkeypatch.setattr(_state.diagnostic_scheduler, "schedule_async", lambda *a, **k: None)
        used = {"cold": False, "small": False}

        def _fake_cold():
            used["cold"] = True
            return ThreadPoolExecutor(max_workers=1)

        def _fake_small():
            used["small"] = True
            return ThreadPoolExecutor(max_workers=1)

        monkeypatch.setattr(_state, "_get_process_pool", _fake_cold)
        monkeypatch.setattr(_state, "_get_small_pool", _fake_small)
        asyncio.run(dp._publish_diagnostics(uri, src, version=1))
        return used

    def test_small_fresh_build_uses_small_lane(self, monkeypatch):
        uri = "file:///lane_small.tcl"
        try:
            used = self._lanes_used_for(monkeypatch, uri, "set x 1\n")
            assert used["small"] is True, "a small fresh build must use the small-file lane"
            assert used["cold"] is False, "a small fresh build must not touch the cold pool"
        finally:
            dp._release_publish_state(uri)
            _state.workspace_state.close(uri)

    def test_large_fresh_build_uses_cold_lane(self, monkeypatch):
        uri = "file:///lane_large.tcl"
        big = "set x 1\n" * (dp._SMALL_BUILD_MAX_BYTES // 8 + 16)
        assert len(big) > dp._SMALL_BUILD_MAX_BYTES
        try:
            used = self._lanes_used_for(monkeypatch, uri, big)
            assert used["cold"] is True, "a large fresh build must use the cold pool"
            assert used["small"] is False, "a large fresh build must not touch the small lane"
        finally:
            dp._release_publish_state(uri)
            _state.workspace_state.close(uri)


class TestSupersessionAfterAnalysis:
    def test_stale_result_not_applied_or_published(self, monkeypatch):
        uri = "file:///supersede_after_analysis.tcl"
        try:
            server = _run_publish_with_midflight_bump(
                monkeypatch, uri, "set x 1\n", version=1, bump_to=2
            )
            # v1 was superseded (latest became 2) while analysing → it must
            # neither publish nor apply its analysis to the document.
            assert all(v != 1 for (_u, v) in server.published), server.published
            st = _state.workspace_state.get(uri)
            assert st is None or st.analysis is None  # stale result not applied
        finally:
            dp._release_publish_state(uri)
            _state.workspace_state.close(uri)

    def test_current_version_still_publishes(self, monkeypatch):
        # Control: when the analysis bumps to its *own* version (no newer
        # request), the result is applied and published normally.
        uri = "file:///supersede_control.tcl"
        try:
            server = _run_publish_with_midflight_bump(
                monkeypatch, uri, "set x 1\n", version=3, bump_to=3
            )
            assert any(v == 3 for (_u, v) in server.published), server.published
            st = _state.workspace_state.get(uri)
            assert st is not None and st.analysis is not None
        finally:
            dp._release_publish_state(uri)
            _state.workspace_state.close(uri)


class TestSupersededBeforeSubmit:
    """A build already superseded *before* it submits to the pool must not burn a
    worker on a result that ``_superseded`` would discard after analysis anyway —
    submitting it would occupy a scarce subprocess slot the live edit needs, so
    the newer build queues behind a corpse.  The guard returns without invoking
    the analysis at all."""

    def test_superseded_cold_build_not_submitted(self, monkeypatch):
        uri = "file:///supersede_before_submit.tcl"
        server = _FakeServer()
        monkeypatch.setattr(dp, "_server", server)
        # Force the cold lane so the guard sits in front of _get_process_pool.
        monkeypatch.setattr(dp, "_SMALL_BUILD_MAX_BYTES", -1)
        monkeypatch.setattr(_state.diagnostic_scheduler, "schedule_async", lambda *a, **k: None)

        analysed: list[int | None] = []
        real_fresh = document_state._analyse_document_fresh

        def tracking_fresh(**kwargs):
            analysed.append(kwargs.get("version"))
            return real_fresh(**kwargs)

        monkeypatch.setattr(document_state, "_analyse_document_fresh", tracking_fresh)

        def _no_pool():
            raise AssertionError("pool must not be requested for a build superseded before submit")

        monkeypatch.setattr(_state, "_get_process_pool", _no_pool)

        try:
            # A newer version (v5) is already the latest-requested when this v1
            # build reaches its submit point.  Call _locked directly to bypass
            # the lock-entry supersession check and exercise the pre-submission
            # guard inside the fresh-build branch.
            dp._publish_latest_version[uri] = 5
            asyncio.run(dp._publish_diagnostics_locked(uri, "set x 1\n", version=1))
            assert analysed == [], "a superseded build must not run analysis"
            assert all(v != 1 for (_u, v) in server.published), server.published
        finally:
            dp._release_publish_state(uri)
            _state.workspace_state.close(uri)


class TestDidCloseEvictsPullDiagCache:
    """did_close (via _release_publish_state) must evict the pull-model
    diagnostics cache; otherwise every ever-opened URI keeps a full diagnostics
    list + result id resident for the whole session (an unbounded leak)."""

    def test_pull_diag_caches_evicted_on_release(self):
        from lsprotocol import types

        uri = "file:///pull_cache_leak.tcl"
        dp._pull_diag_cache[uri] = [
            types.Diagnostic(
                range=types.Range(types.Position(0, 0), types.Position(0, 1)),
                message="x",
                code="W100",
            )
        ]
        dp._pull_diag_result_ids[uri] = "tcl-lsp-diag-test"
        dp._publish_latest_version[uri] = 1

        dp._release_publish_state(uri)

        assert uri not in dp._pull_diag_cache
        assert uri not in dp._pull_diag_result_ids
        assert uri not in dp._publish_latest_version
