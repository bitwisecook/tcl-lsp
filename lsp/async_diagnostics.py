"""Async diagnostic scheduler — runs deep analysis in background threads.

The scheduler ensures that:

1. Basic diagnostics (analysis warnings, style checks) are published
   immediately on every edit, keeping the editor responsive.
2. Deep diagnostics (optimiser, shimmer, taint, GVN, iRules flow) run
   in a background thread via ``asyncio.to_thread`` and publish
   incrementally when complete.
3. If the user edits the document while deep analysis is in-flight, the
   pending task is cancelled and a new one is scheduled, avoiding
   wasted CPU on stale source text.
"""

from __future__ import annotations

import asyncio
import logging
from collections.abc import Callable
from dataclasses import dataclass, field

from lsprotocol import types

log = logging.getLogger(__name__)

# Type aliases for the two callback signatures.
DeepDiagnosticsFn = Callable[[], list[types.Diagnostic]]
PublishFn = Callable[[str, list[types.Diagnostic], int | None], None]


@dataclass
class _PendingTask:
    """Tracks an in-flight deep-diagnostic task for a single URI."""

    task: asyncio.Task[None]
    version: int | None


@dataclass
class DiagnosticScheduler:
    """Manages background deep-diagnostic tasks per document URI.

    Call :meth:`schedule` from an async handler whenever a document
    changes.  The scheduler will cancel any already-running deep pass
    for that URI and start a fresh one.
    """

    _pending: dict[str, _PendingTask] = field(default_factory=dict)

    def schedule(
        self,
        uri: str,
        version: int | None,
        basic_diags: list[types.Diagnostic],
        deep_fn: DeepDiagnosticsFn,
        publish_fn: PublishFn,
    ) -> None:
        """Schedule a deep diagnostic pass for *uri*.

        Parameters
        ----------
        uri:
            Document URI.
        version:
            Document version (used for staleness checks).
        basic_diags:
            Already-computed basic diagnostics — the deep pass will
            *prepend* these when publishing so that the full set is
            always consistent.
        deep_fn:
            A **synchronous** callable ``() -> list[types.Diagnostic]``
            that runs the expensive passes.  It will be executed via
            ``asyncio.to_thread``.
        publish_fn:
            A callable ``(uri, diagnostics, version) -> None`` that
            publishes diagnostics to the client.
        """
        # Cancel any existing task for this URI.
        prev = self._pending.pop(uri, None)
        if prev is not None and not prev.task.done():
            prev.task.cancel()
            log.debug("Cancelled stale deep diagnostics for %s (v%s)", uri, prev.version)

        task = asyncio.create_task(
            self._run(uri, version, basic_diags, deep_fn, publish_fn),
        )
        self._pending[uri] = _PendingTask(task=task, version=version)

    async def _run(
        self,
        uri: str,
        version: int | None,
        basic_diags: list[types.Diagnostic],
        deep_fn: DeepDiagnosticsFn,
        publish_fn: PublishFn,
    ) -> None:
        """Execute the deep pass in a thread and publish the merged result."""
        try:
            deep_diags: list[types.Diagnostic] = await asyncio.to_thread(deep_fn)
        except asyncio.CancelledError:
            return
        except Exception:
            log.debug("Deep diagnostics failed for %s", uri, exc_info=True)
            return
        finally:
            # Clean up _pending if we're still the active task.
            pending = self._pending.get(uri)
            if pending is not None and pending.version == version:
                self._pending.pop(uri, None)

        # Merge basic + deep and publish.
        publish_fn(uri, basic_diags + deep_diags, version)

    def cancel(self, uri: str) -> None:
        """Cancel any pending deep diagnostics for *uri*."""
        prev = self._pending.pop(uri, None)
        if prev is not None and not prev.task.done():
            prev.task.cancel()

    def cancel_all(self) -> None:
        """Cancel all pending tasks (e.g. on shutdown)."""
        for entry in self._pending.values():
            if not entry.task.done():
                entry.task.cancel()
        self._pending.clear()
