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
    # Force even a tiny source down the cold *pool* path (the size fast-lane
    # would otherwise route a small fresh build in-thread, bypassing the
    # subprocess await this supersession guard protects).
    monkeypatch.setattr(dp, "_COLD_INTHREAD_MAX_BYTES", -1)
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


class TestColdBuildSizeFastLane:
    """A fresh build only goes to the cold *pool* when it's big; a small one
    runs in-thread so a trivial file never queues behind huge cold builds."""

    def _pool_used_for(self, monkeypatch, uri: str, src: str) -> bool:
        server = _FakeServer()
        monkeypatch.setattr(dp, "_server", server)
        monkeypatch.setattr(_state.diagnostic_scheduler, "schedule_async", lambda *a, **k: None)
        used = {"pool": False}

        def _fake_pool():
            used["pool"] = True
            return ThreadPoolExecutor(max_workers=1)

        monkeypatch.setattr(_state, "_get_process_pool", _fake_pool)
        asyncio.run(dp._publish_diagnostics(uri, src, version=1))
        return used["pool"]

    def test_small_fresh_build_skips_pool(self, monkeypatch):
        uri = "file:///fastlane_small.tcl"
        try:
            assert self._pool_used_for(monkeypatch, uri, "set x 1\n") is False
        finally:
            dp._release_publish_state(uri)
            _state.workspace_state.close(uri)

    def test_large_fresh_build_uses_pool(self, monkeypatch):
        uri = "file:///fastlane_large.tcl"
        big = "set x 1\n" * (dp._COLD_INTHREAD_MAX_BYTES // 8 + 16)
        assert len(big) > dp._COLD_INTHREAD_MAX_BYTES
        try:
            assert self._pool_used_for(monkeypatch, uri, big) is True
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
