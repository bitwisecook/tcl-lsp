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

"""Run the deterministic performance check suite against one server binary.

Drives a real `tcl-lsp-server` over LSP through a fixed, ordered set of
checks — workspace scan, opening documents, a typing storm, navigation,
code actions, symbol rename, external file rename — while sampling the
server process's resident memory and cumulative CPU on a fixed interval.
Writes one JSON result file that `report.py` turns into graphs.

Determinism is the point, so that a graph comparing v2.1.5 to v2.1.16
attributes its differences to the server and not to the harness:

* the corpus is pinned (`MANIFEST.toml`) and staged into a scratch copy,
  so the external-rename checks cannot mutate the pinned checkout;
* documents are chosen by sorted relative path, never by directory order;
* every edit, position and rename is derived from a fixed seed;
* the check list and its order are hard-coded, not discovered.

What is *not* deterministic, and cannot be, is the wall time itself. Run
the whole version sweep on one machine, otherwise the bar graph compares
hardware rather than releases.

Usage:
    bench.py --server PATH --version 2.1.14 [--scope small] [--out DIR]
"""

from __future__ import annotations

import argparse
import json
import os
import platform
import random
import shutil
import subprocess
import sys
import threading
import time
import tomllib
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
REPO_ROOT = HERE.parent.parent
sys.path.insert(0, str(REPO_ROOT / ".claude/skills/lsp-client"))

from lsp_client import (  # noqa: E402
    SEMANTIC_TOKEN_MODIFIERS,
    SEMANTIC_TOKEN_TYPES,
    LspClient,
)

SCHEMA = 1

#: How many documents the suite keeps open. Small enough to stay realistic
#: for an editor session, large enough that cross-file work is exercised.
OPEN_DOCS = 4

#: Keystrokes in the typing-storm check. The memory question is whether RSS
#: plateaus, which needs enough edits for warm-up to finish and a late window
#: to exist — 400 gives four clean quartiles at a few seconds of runtime.
EDIT_COUNT = 400

#: Sampling period for the memory/CPU timeline. 0.25 s resolves the shape of
#: a leak curve without the sampler itself perturbing the measurement.
SAMPLE_INTERVAL_S = 0.25

#: Per-request ceiling. A request that exceeds this is recorded as a failed
#: check with `ok: false` rather than hanging the suite — several releases
#: have shipped navigation requests that never return (#1297), and the
#: benchmark has to survive them and *report* them.
REQUEST_TIMEOUT_S = 30.0

SEED = 20260807

#: Outer wall-clock budget for one bench.py run (one version), in seconds.
#: Independent of REQUEST_TIMEOUT_S: that bounds a single LSP round-trip
#: (and, since lsp_client.py's `_send` fix, the write side of it too); this
#: bounds the *whole process* — corpus staging, the workspace scan, every
#: check, teardown — so a fault outside any individually-bounded wait still
#: produces a clean, attributed failure instead of relying on an external
#: killer (sweep.sh's `timeout --foreground`, CI's job timeout) with no idea
#: which phase died. See issue #1399.  Generous for `--scope small` (a
#: healthy run finishes in well under a minute per sweep.sh's own comment);
#: override with `--deadline` or `BENCH_DEADLINE_S` for a larger scope or a
#: slower host.
DEFAULT_DEADLINE_S = 240.0


# --------------------------------------------------------------------------
# Process sampling


def _proc_sample(pid: int) -> tuple[int, float] | None:
    """`(rss_kib, cumulative_cpu_seconds)` for `pid`, or None if it is gone.

    Uses `ps` rather than an allocator hook so the figure includes arena and
    `mmap` growth, which is exactly the kind a query-database leak produces
    and an allocator counter would miss.
    """
    r = subprocess.run(
        ["ps", "-o", "rss=,cputime=", "-p", str(pid)],
        capture_output=True,
        text=True,
        check=False,
    )
    parts = r.stdout.split()
    if len(parts) < 2:
        return None
    rss = int(parts[0])
    # `[[dd-]hh:]mm:ss.cc`
    clock = parts[1]
    days = 0
    if "-" in clock:
        d, clock = clock.split("-", 1)
        days = int(d)
    bits = [float(b) for b in clock.split(":")]
    while len(bits) < 3:
        bits.insert(0, 0.0)
    cpu = days * 86400 + bits[0] * 3600 + bits[1] * 60 + bits[2]
    return rss, cpu


class Sampler(threading.Thread):
    """Background RSS/CPU sampler producing the timeline series."""

    def __init__(self, pid: int, interval: float = SAMPLE_INTERVAL_S) -> None:
        super().__init__(daemon=True)
        self.pid = pid
        self.interval = interval
        self.samples: list[dict] = []
        # NOT `_stop`: `threading.Thread` defines a private `_stop()`
        # method that `join(timeout=...)` calls internally once the
        # thread has finished, so an attribute of that name shadows it.
        # Every run died with `TypeError: 'Event' object is not
        # callable` in `Sampler.stop()` — raised from the `finally`
        # block *before* the result file is written, so the benchmark
        # completed all 15 checks and then produced nothing.
        self._halt = threading.Event()
        self._t0 = time.perf_counter()

    def run(self) -> None:
        while not self._halt.is_set():
            s = _proc_sample(self.pid)
            if s is None:
                break
            rss, cpu = s
            self.samples.append(
                {
                    "t_s": round(time.perf_counter() - self._t0, 3),
                    "rss_kib": rss,
                    "cpu_s": round(cpu, 2),
                }
            )
            self._halt.wait(self.interval)

    def stop(self) -> None:
        self._halt.set()
        self.join(timeout=2.0)

    def latest(self) -> dict | None:
        return self.samples[-1] if self.samples else None


# --------------------------------------------------------------------------
# Outer deadline (#1399)


class Progress:
    """Shared, best-effort state the watchdog reads to report *where* a run
    died.

    Written from the main thread only; the watchdog thread only ever reads
    it, and only after deciding the run is already dead — a torn read here
    produces a slightly-wrong diagnostic line at worst, never a wrong
    benchmark result, so this deliberately carries no lock.
    """

    def __init__(self, server: Path, version: str) -> None:
        self.server = server
        self.version = version
        self.phase = "startup"
        #: Set once the LspClient exists, so the watchdog can read its
        #: `last_request_id` / `last_request_method` directly instead of
        #: bench.py duplicating that bookkeeping.
        self.client: LspClient | None = None


class DeadlineExceeded(RuntimeError):
    """Raised (from the main thread, via the watchdog) when the outer
    `--deadline` elapses. In practice the watchdog reports and hard-exits
    before this would ever need to be caught — it exists mainly so the
    failure mode has a name in a traceback if `os._exit` is ever removed."""


class Watchdog(threading.Thread):
    """Kills the process if the run hasn't finished within *deadline*
    seconds of the watchdog starting.

    A per-request timeout (`REQUEST_TIMEOUT_S`, and now the write side of it
    too — see `lsp_client.LspClient._send`) bounds one LSP round-trip; this
    bounds the *whole run*, so a fault no individual wait can see — a chain
    of many separately-bounded waits that together outlast the sweep's
    external timeout, a hang in corpus staging or teardown, or simply a
    slower host than `--deadline` assumed — still ends in a clean,
    attributed nonzero exit instead of an external `timeout(1)` SIGKILL that
    explains nothing (see issue #1399 and sweep.sh's 8h45m wedge note).

    Deliberately a hard `os._exit`, not a raised exception the main thread
    could ignore or get stuck handling: the whole point is that this fires
    when the main thread is *not* responsive, so anything that depends on
    the main thread cooperating (a raised exception, a graceful shutdown)
    is not a safe assumption to build the escape hatch on.
    """

    def __init__(self, progress: Progress, deadline: float) -> None:
        super().__init__(daemon=True, name="bench-deadline-watchdog")
        self.progress = progress
        self.deadline = deadline
        self._done = threading.Event()

    def cancel(self) -> None:
        """Call once the run has genuinely finished (including teardown)."""
        self._done.set()

    def run(self) -> None:
        if self._done.wait(self.deadline):
            return  # the run finished (or was cancelled) before the deadline
        p = self.progress
        client = p.client
        last_id = client.last_request_id if client else None
        last_method = client.last_request_method if client else None
        server_pid = (
            client.process.pid if client and client.process is not None else None
        )
        print(
            "\n"
            f"!!! bench.py: --deadline {self.deadline:.0f}s exceeded !!!\n"
            f"    server binary: {p.server}\n"
            f"    version:       {p.version}\n"
            f"    phase:         {p.phase}\n"
            f"    server pid:    {server_pid}\n"
            f"    last request:  method={last_method} id={last_id}\n"
            "    This is either a slow host/scope (raise --deadline or set\n"
            "    BENCH_DEADLINE_S) or issue #1399's intermittent harness "
            "hang.\n",
            file=sys.stderr,
        )
        sys.stderr.flush()
        sys.stdout.flush()
        # os._exit below skips every `finally` in the main thread, including
        # the one that would normally kill the server subprocess — so do
        # that here first, or a killed run leaks a live server process (an
        # orphaned `tcl-lsp-server` per hang, silently accumulating across a
        # sweep). Best-effort: the process may already be gone, or this may
        # itself be part of what's wedged.
        if client is not None and client.process is not None:
            try:
                client.process.kill()
            except Exception:  # noqa: BLE001
                pass
        # A daemon thread cannot unwind the main thread's call stack, and the
        # main thread may be blocked in a call this fix does not (or cannot)
        # bound — os._exit is the one escape that does not depend on it
        # cooperating.
        os._exit(1)


# --------------------------------------------------------------------------
# Corpus staging and deterministic selection


def stage_corpus(src: Path, dest: Path) -> int:
    """Copy the pinned corpus's `.tcl` files into a scratch tree.

    The external-rename checks move files on disk. Staging keeps the pinned
    checkout clean, so `fetch_corpus.py --verify-only` still passes after a
    benchmark run and the next run starts from identical bytes.
    """
    if dest.exists():
        shutil.rmtree(dest)
    count = 0
    for f in sorted(src.rglob("*.tcl")):
        if ".git" in f.parts:
            continue
        rel = f.relative_to(src)
        out = dest / rel
        out.parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(f, out)
        count += 1
    return count


def choose_documents(
    root: Path,
    n: int,
    anchors: list[str] | None = None,
    *,
    lo: int = 4_000,
    hi: int = 120_000,
) -> list[Path]:
    """Pick `n` documents deterministically, anchors first.

    Anchors are manifest-listed relative paths that pin known pathological
    shapes (see `[anchors]` in `MANIFEST.toml`). They come first and are
    never displaced, because the whole point of the suite is to keep
    tracking a case once it has bitten — a purely seeded selection that
    happened to land on four cheap files would report a clean run while the
    regression it exists to watch sat untouched.

    The remainder is sorted by relative path so filesystem order can never
    leak in, then shuffled with the fixed seed so the fill-in files are not
    all from one directory (which would under-exercise cross-file work).
    """
    chosen: list[Path] = []
    for rel in anchors or []:
        p = root / rel
        if p.is_file():
            chosen.append(p)
    files = sorted(
        (
            p
            for p in root.rglob("*.tcl")
            if lo <= p.stat().st_size <= hi and p not in chosen
        ),
        key=lambda p: str(p.relative_to(root)),
    )
    random.Random(SEED).shuffle(files)
    return (chosen + files)[:n]


#: Keywords whose *next* word is a definition name worth navigating from.
#: `proc` alone is not enough: much of the corpus is TclOO, where the
#: definitions are `method` / `constructor` / `oo::class create`, and a
#: `proc`-only scan silently yields no positions at all — turning every
#: navigation check into a 0.000 s no-op that looks like a pass.
DEFINER_KEYWORDS = ("proc", "method", "classmethod", "typemethod", "forward")


def proc_positions(text: str, limit: int = 20) -> list[tuple[int, int]]:
    """Positions of up to `limit` definition names — stable navigation targets.

    Anchoring on definitions rather than random offsets means every version
    answers the same question, and the answers are non-trivial: a position
    in whitespace would make `references` cheap on every release and hide
    exactly the regressions this suite exists to catch.

    Indentation is allowed (TclOO members are nested inside a class body),
    and the column returned points *into* the name rather than at the
    keyword, which is what a cursor-position request needs.
    """
    out: list[tuple[int, int]] = []
    for i, line in enumerate(text.split("\n")):
        stripped = line.lstrip()
        if not stripped or stripped.startswith("#"):
            continue
        parts = stripped.split(None, 2)
        if len(parts) < 2 or parts[0] not in DEFINER_KEYWORDS:
            continue
        indent = len(line) - len(stripped)
        # Column of the first character of the name that follows the keyword.
        name_col = (
            indent
            + len(parts[0])
            + (len(stripped[len(parts[0]) :]) - len(stripped[len(parts[0]) :].lstrip()))
        )
        out.append((i, name_col))
        if len(out) >= limit:
            break
    return out


# --------------------------------------------------------------------------
# The suite


class Bench:
    def __init__(
        self,
        client: LspClient,
        sampler: Sampler,
        root: Path,
        progress: Progress | None = None,
    ) -> None:
        self.c = client
        self.s = sampler
        self.root = root
        self.checks: list[dict] = []
        self._ord = 0
        #: `--deadline` phase attribution (#1399). Optional so `Bench` stays
        #: usable without a watchdog wired up.
        self.progress = progress

    def request(self, method: str, params: dict) -> tuple[Any, bool]:
        """One request; `(result, ok)` where ok=False means timeout/error."""
        try:
            return self.c.send_request(method, params, timeout=REQUEST_TIMEOUT_S), True
        except TimeoutError:
            return None, False
        except Exception:  # noqa: BLE001
            return None, False

    def check(self, cid: str, label: str, fn) -> None:
        """Run one named check, recording wall, CPU and RSS either side.

        CPU is taken as the delta of the *process's* cumulative CPU across
        the check, so it captures the server's background workers too, not
        just the thread answering the request.
        """
        self._ord += 1
        if self.progress is not None:
            self.progress.phase = f"check {self._ord}: {cid} ({label})"
        before = self.s.latest() or {"rss_kib": 0, "cpu_s": 0.0}
        t0 = time.perf_counter()
        ok = True
        note = None
        try:
            note = fn()
        except Exception as exc:  # noqa: BLE001
            ok, note = False, f"{type(exc).__name__}: {exc}"
        wall = time.perf_counter() - t0
        # Let the sampler catch up so `rss_after` reflects the check.
        time.sleep(SAMPLE_INTERVAL_S * 1.5)
        after = self.s.latest() or before
        ops = items = None
        if isinstance(note, tuple):
            ok, note = note
        elif isinstance(note, dict):
            ok = note.get("ok", ok)
            ops, items = note.get("ops"), note.get("items")
            note = note.get("note")
        self.checks.append(
            {
                "ord": self._ord,
                "id": cid,
                "label": label,
                "wall_s": round(wall, 4),
                "cpu_s": round(max(0.0, after["cpu_s"] - before["cpu_s"]), 2),
                "rss_before_kib": before["rss_kib"],
                "rss_after_kib": after["rss_kib"],
                # Requests issued, and results they returned. A timing with
                # no payload count is uninterpretable: a request that answers
                # nothing is fast for the wrong reason, and comparing it
                # against one that resolved 37 locations is meaningless.
                "ops": ops,
                "items": items,
                "ok": ok,
                "note": note,
            }
        )
        flag = "" if ok else "   FAILED"
        print(
            f"  {self._ord:>2}. {label:<44} {wall:>8.3f}s  "
            f"{after['rss_kib'] / 1024:>8.1f} MiB{flag}"
        )


def _count(result: Any) -> int:
    """How many things an LSP result actually contained.

    Navigation responses are variously a list of locations, a single
    location, a `{items: [...]}` completion list, or null. Collapsing them
    to one number is what lets a fast check be distinguished from an empty
    one — the whole reason timings alone were not enough to explain a 20x
    gap between two runs over the same corpus.
    """
    if result is None:
        return 0
    if isinstance(result, list):
        return len(result)
    if isinstance(result, dict):
        if isinstance(result.get("items"), list):
            return len(result["items"])
        return 1
    return 1


def build_suite(bench: Bench, docs: list[Path], root: Path) -> None:
    """The deterministic, ordered check list.

    Order matters and is deliberate: the session builds up the way a real
    one does (scan, open, type, navigate, act, rename), so each check sees
    the state its predecessors left. Reordering invalidates comparison with
    stored results, so treat this list as an interface.
    """
    c, state = bench.c, []

    def uri_of(p: Path) -> str:
        return f"file://{p}"

    # 1 — workspace scan. The cold cross-file index build.
    def scan():
        try:
            c.wait_for_workspace_scan(timeout=REQUEST_TIMEOUT_S * 4)
        except Exception as exc:  # noqa: BLE001
            return False, f"scan did not settle: {type(exc).__name__}"
        return None

    bench.check("scan.workspace", "Workspace scan (cold index)", scan)

    # 2 — open the documents and drive one full analysis each.
    def open_docs():
        for p in docs:
            text = p.read_text(errors="replace")
            c.send_notification(
                "textDocument/didOpen",
                {
                    "textDocument": {
                        "uri": uri_of(p),
                        "languageId": "tcl",
                        "version": 1,
                        "text": text,
                    }
                },
            )
            state.append(
                {
                    "path": p,
                    "uri": uri_of(p),
                    "base": text,
                    "version": 1,
                    "pos": proc_positions(text),
                }
            )
        fails = sum(
            not bench.request(
                "textDocument/semanticTokens/full", {"textDocument": {"uri": s["uri"]}}
            )[1]
            for s in state
        )
        return f"{fails} token request(s) failed" if fails else None

    bench.check("open.docs", f"Open {len(docs)} documents + analyse", open_docs)

    # 3 — the typing storm. Every keystroke a fresh, never-seen text of
    # constant length: the buffer never repeats (so no identical-input
    # short circuit) and never grows (so growth cannot be blamed on size).
    def edit_storm():
        for i in range(1, EDIT_COUNT + 1):
            s = state[i % len(state)]
            s["version"] += 1
            c.send_notification(
                "textDocument/didChange",
                {
                    "textDocument": {"uri": s["uri"], "version": s["version"]},
                    "contentChanges": [
                        {"text": s["base"] + f"\nset probe_v {i:08d}\n"}
                    ],
                },
            )
        c.collect_notifications("textDocument/publishDiagnostics", timeout=2.0)
        return None

    bench.check(
        "edit.storm", f"Typing storm ({EDIT_COUNT} edits, no requests)", edit_storm
    )

    # 4 — the same storm with the requests an editor actually issues, which
    # is what turns an edit into analysis work.
    def edit_storm_tokens():
        fails = 0
        for i in range(1, EDIT_COUNT // 4 + 1):
            s = state[i % len(state)]
            s["version"] += 1
            c.send_notification(
                "textDocument/didChange",
                {
                    "textDocument": {"uri": s["uri"], "version": s["version"]},
                    "contentChanges": [
                        {"text": s["base"] + f"\nset probe_w {i:08d}\n"}
                    ],
                },
            )
            fails += not bench.request(
                "textDocument/semanticTokens/full", {"textDocument": {"uri": s["uri"]}}
            )[1]
        return f"{fails} token request(s) failed" if fails else None

    bench.check(
        "edit.tokens",
        f"Typing + semantic tokens ({EDIT_COUNT // 4} edits)",
        edit_storm_tokens,
    )

    # 5-9 — navigation, one check per request kind so a regression lands on
    # the specific feature rather than in an aggregate.
    def nav(method: str, extra: dict | None = None):
        def run():
            fails = sent = items = 0
            for s in state:
                for line, col in s["pos"][:3]:
                    p = {
                        "textDocument": {"uri": s["uri"]},
                        "position": {"line": line, "character": col},
                    }
                    if extra:
                        p.update(extra)
                    sent += 1
                    got, ok = bench.request(method, p)
                    fails += not ok
                    items += _count(got)
            # A check that issued nothing is not a fast check, it is a broken
            # one — and a 0.000 s bar in the release-notes graph reading as
            # "this got faster" would be worse than no bar at all.
            if sent == 0:
                return {
                    "ok": False,
                    "note": "no navigation positions found: issued 0 requests",
                    "ops": 0,
                    "items": 0,
                }
            return {
                "ops": sent,
                "items": items,
                "note": f"{fails}/{sent} request(s) failed" if fails else None,
            }

        return run

    bench.check("nav.hover", "Hover at definition sites", nav("textDocument/hover"))
    bench.check("nav.definition", "Go to definition", nav("textDocument/definition"))
    bench.check(
        "nav.references",
        "Find references",
        nav("textDocument/references", {"context": {"includeDeclaration": True}}),
    )
    bench.check(
        "nav.completion",
        "Completion at definition sites",
        nav("textDocument/completion"),
    )

    def doc_wide():
        fails = 0
        for s in state:
            for m in ("textDocument/documentSymbol", "textDocument/foldingRange"):
                fails += not bench.request(m, {"textDocument": {"uri": s["uri"]}})[1]
        return f"{fails} request(s) failed" if fails else None

    bench.check("nav.symbols", "Document symbols + folding", doc_wide)

    # 10 — code lens, including resolve, which is where the reference counts
    # are actually computed (and where #1297 lives).
    def lenses():
        fails = 0
        for s in state:
            got, ok = bench.request(
                "textDocument/codeLens", {"textDocument": {"uri": s["uri"]}}
            )
            fails += not ok
            for lens in (got or [])[:3]:
                fails += not bench.request("codeLens/resolve", lens)[1]
        return f"{fails} request(s) failed" if fails else None

    bench.check("lens.resolve", "Code lens + resolve", lenses)

    # 11 — code actions at each position, the way an editor requests them on
    # every cursor move.
    def actions():
        fails = sent = 0
        for s in state:
            for line, col in s["pos"][:3]:
                rng = {
                    "start": {"line": line, "character": col},
                    "end": {"line": line, "character": col},
                }
                sent += 1
                fails += not bench.request(
                    "textDocument/codeAction",
                    {
                        "textDocument": {"uri": s["uri"]},
                        "range": rng,
                        "context": {"diagnostics": []},
                    },
                )[1]
        if sent == 0:
            return False, "no positions found: check issued 0 requests"
        return f"{fails}/{sent} request(s) failed" if fails else None

    bench.check("action.codeaction", "Code actions at definition sites", actions)

    # 12 — symbol rename across the workspace.
    def rename_symbol():
        fails = sent = 0
        for n, s in enumerate(state):
            if not s["pos"]:
                continue
            line, col = s["pos"][0]
            sent += 1
            fails += not bench.request(
                "textDocument/rename",
                {
                    "textDocument": {"uri": s["uri"]},
                    "position": {"line": line, "character": col},
                    "newName": f"bench_renamed_{n}",
                },
            )[1]
        if sent == 0:
            return False, "no positions found: check issued 0 requests"
        return f"{fails}/{sent} request(s) failed" if fails else None

    bench.check("refactor.rename", "Rename symbol (one per document)", rename_symbol)

    # 13 — external file rename: the explorer-rename sequence, including the
    # watched-file events, then following the buffer to the new URI.
    def rename_files():
        for n, s in enumerate(state):
            old = s["path"]
            new = old.with_name(f"{old.stem}__benchr{n}{old.suffix}")
            files = [{"oldUri": uri_of(old), "newUri": uri_of(new)}]
            bench.request("workspace/willRenameFiles", {"files": files})
            try:
                shutil.move(str(old), str(new))
            except OSError as exc:
                return False, f"move failed: {exc}"
            c.send_notification("workspace/didRenameFiles", {"files": files})
            c.send_notification(
                "workspace/didChangeWatchedFiles",
                {
                    "changes": [
                        {"uri": uri_of(old), "type": 3},
                        {"uri": uri_of(new), "type": 1},
                    ]
                },
            )
            c.send_notification(
                "textDocument/didClose", {"textDocument": {"uri": s["uri"]}}
            )
            c.send_notification(
                "textDocument/didOpen",
                {
                    "textDocument": {
                        "uri": uri_of(new),
                        "languageId": "tcl",
                        "version": 1,
                        "text": s["base"],
                    }
                },
            )
            s["path"], s["uri"], s["version"] = new, uri_of(new), 1
        c.collect_notifications("textDocument/publishDiagnostics", timeout=2.0)
        return None

    bench.check(
        "refactor.filerename", "External file rename (one per document)", rename_files
    )

    # 14 — navigation *after* the renames. Separate from check 8 on purpose:
    # a retire path that strands per-URI state shows up as this check being
    # dramatically slower than its pre-rename twin (#1298).
    bench.check(
        "nav.after_rename",
        "Find references after rename",
        nav("textDocument/references", {"context": {"includeDeclaration": True}}),
    )

    # 15 — close everything; the closed-file retention path (#1144).
    def close_all():
        for s in state:
            c.send_notification(
                "textDocument/didClose", {"textDocument": {"uri": s["uri"]}}
            )
        c.collect_notifications("textDocument/publishDiagnostics", timeout=2.0)
        return None

    bench.check("close.docs", "Close all documents", close_all)


# --------------------------------------------------------------------------


def initialize(client: LspClient, root: Path) -> None:
    client.send_request(
        "initialize",
        {
            "processId": os.getpid(),
            "rootUri": f"file://{root}",
            "workspaceFolders": [{"uri": f"file://{root}", "name": "corpus"}],
            "capabilities": {
                "workspace": {
                    "workspaceFolders": True,
                    "workspaceEdit": {"documentChanges": True},
                    "didChangeWatchedFiles": {"dynamicRegistration": True},
                    "fileOperations": {"willRename": True, "didRename": True},
                },
                "textDocument": {
                    "semanticTokens": {
                        "dynamicRegistration": False,
                        "requests": {"full": True},
                        "tokenTypes": SEMANTIC_TOKEN_TYPES,
                        "tokenModifiers": SEMANTIC_TOKEN_MODIFIERS,
                        "formats": ["relative"],
                    },
                    "publishDiagnostics": {"relatedInformation": False},
                    "documentSymbol": {"hierarchicalDocumentSymbolSupport": True},
                    "rename": {"prepareSupport": True},
                    "codeAction": {
                        "codeActionLiteralSupport": {
                            "codeActionKind": {"valueSet": ["quickfix", "refactor"]}
                        }
                    },
                },
            },
        },
        timeout=180.0,
    )
    client.send_notification("initialized", {})


def host_info() -> dict:
    """CPU model, core count and RAM, for the provenance line on every graph.

    Wall times are only comparable within one machine, so a result that
    cannot say which machine produced it is close to useless. Both Darwin
    and Linux are filled in properly — CI runs on Linux, and leaving those
    fields `None` there is how the first CI run failed.

    `mem_gb` is `0` rather than `None` when it cannot be determined, so
    consumers can format it without a null check.
    """

    def sysctl(key: str) -> str | None:
        r = subprocess.run(
            ["sysctl", "-n", key], capture_output=True, text=True, check=False
        )
        return r.stdout.strip() or None

    cpu = None
    mem = 0
    system = platform.system()
    if system == "Darwin":
        cpu = sysctl("machdep.cpu.brand_string")
        raw = sysctl("hw.memsize")
        if raw and raw.isdigit():
            mem = round(int(raw) / 1024**3)
    elif system == "Linux":
        try:
            for line in Path("/proc/cpuinfo").read_text().splitlines():
                if line.startswith("model name"):
                    cpu = line.split(":", 1)[1].strip()
                    break
        except OSError:
            pass
        try:
            for line in Path("/proc/meminfo").read_text().splitlines():
                if line.startswith("MemTotal:"):
                    mem = round(int(line.split()[1]) / 1024**2)
                    break
        except (OSError, ValueError, IndexError):
            pass
    # Machine load at the moment of measurement. A developer laptop has a
    # busy desktop on it (browser, editor, chat) and will never be quiet, so
    # rather than pretend otherwise or gate on a threshold that never
    # clears, record the contention alongside the numbers. Large differences
    # survive it; a 10% wall-time move on a loaded host means nothing, and
    # this is what lets a reader tell the two apart.
    try:
        load1 = round(os.getloadavg()[0], 2)
    except (OSError, AttributeError):
        load1 = None
    return {
        "cpu": cpu or platform.processor() or platform.machine(),
        "cores": os.cpu_count(),
        "mem_gb": mem,
        "load_1m": load1,
    }


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    # Resolved to an absolute path below: the client spawns the server with
    # its cwd set to the staged corpus, so a relative path here would be
    # looked up in the wrong directory and fail with a bare ENOENT.
    ap.add_argument("--server", required=True, type=Path)
    ap.add_argument("--version", required=True, help="e.g. 2.1.14")
    ap.add_argument("--manifest", type=Path, default=HERE / "MANIFEST.toml")
    ap.add_argument("--corpus", type=Path, default=HERE / "corpus")
    ap.add_argument("--scope", default="small")
    ap.add_argument("--out", type=Path, default=HERE / "results")
    ap.add_argument(
        "--stage",
        type=Path,
        default=None,
        help="scratch dir for the staged corpus copy",
    )
    # Off by default so a local exploratory run still writes its results when
    # a check times out — a timed-out check is data, and discarding the whole
    # run because of one would lose the timeline that explains it. CI opts in.
    ap.add_argument(
        "--fail-on-failed-check",
        action="store_true",
        help="exit non-zero if any check timed out or errored",
    )
    ap.add_argument(
        "--deadline",
        type=float,
        default=float(os.environ.get("BENCH_DEADLINE_S", DEFAULT_DEADLINE_S)),
        help=(
            "outer wall-clock budget in seconds for the whole run (issue "
            "#1399); on expiry, reports the phase, server binary and last "
            "request, then exits nonzero. Overrides BENCH_DEADLINE_S if "
            f"both are set (default {DEFAULT_DEADLINE_S:.0f}s). 0 disables "
            "it — do not disable in CI or a sweep."
        ),
    )
    args = ap.parse_args()
    args.server = args.server.resolve()
    if not args.server.is_file():
        print(f"no server binary at {args.server}", file=sys.stderr)
        return 2

    progress = Progress(args.server, args.version)
    watchdog: Watchdog | None = None
    if args.deadline > 0:
        watchdog = Watchdog(progress, args.deadline)
        watchdog.start()
    else:
        print(
            "   --deadline 0: outer deadline disabled, this run can hang "
            "forever (issue #1399)",
            file=sys.stderr,
        )

    try:
        return _run(args, progress)
    finally:
        if watchdog is not None:
            # The run genuinely finished (including teardown) — stop the
            # watchdog so it cannot fire after the fact.
            watchdog.cancel()


def _run(args: argparse.Namespace, progress: Progress) -> int:
    """The actual benchmark run, split out from `main` so every return path
    — including the early "no corpus" / "server failed to start" ones —
    passes through `main`'s single `finally: watchdog.cancel()`."""
    progress.phase = "corpus-staging"
    with args.manifest.open("rb") as fh:
        manifest = tomllib.load(fh)
    groups = manifest["scope"][args.scope]["groups"]

    stage = args.stage or (HERE / ".stage" / args.version)
    stage.mkdir(parents=True, exist_ok=True)
    total = 0
    for g in sorted(groups):
        src = args.corpus / g
        if src.exists():
            total += stage_corpus(src, stage / g)
    if total == 0:
        print(
            f"no corpus under {args.corpus} for scope {args.scope}; "
            f"run fetch_corpus.py first",
            file=sys.stderr,
        )
        return 2

    anchors = manifest.get("anchors", {}).get("paths", [])
    docs = choose_documents(stage, OPEN_DOCS, anchors)
    if len(docs) < OPEN_DOCS:
        print(
            f"only {len(docs)} candidate documents in scope {args.scope}",
            file=sys.stderr,
        )

    # Captured before the run, because the staged tree is deleted on the way
    # out (and the rename checks move files within it anyway).
    #
    # This is provenance, not decoration. Two runs over the same corpus can
    # differ by an order of magnitude depending on which documents were
    # picked and how many navigation positions each yielded, and neither is
    # recoverable from the timings afterwards. Recording the inputs turns a
    # surprising number into a question you can answer instead of one you
    # have to re-run to guess at.
    doc_meta = [
        {
            "path": str(d.relative_to(stage)),
            "bytes": d.stat().st_size,
            "positions": len(proc_positions(d.read_text(errors="replace"))),
        }
        for d in docs
    ]
    for m in doc_meta:
        print(f"   doc {m['path']}  ({m['bytes']}B, {m['positions']} positions)")

    print(f"== {args.version} | scope={args.scope} | {total} .tcl files ==")
    print(f"   server {args.server}")

    progress.phase = "server-start"
    client = LspClient(str(stage), launch_cmd=[str(args.server)])
    progress.client = client
    client.start()
    if client.process is None:
        print(f"server failed to start: {args.server}", file=sys.stderr)
        return 2
    sampler = Sampler(client.process.pid)
    sampler.start()
    try:
        progress.phase = "init"
        initialize(client, stage)
        bench = Bench(client, sampler, stage, progress)
        build_suite(bench, docs, stage)
    finally:
        progress.phase = "teardown"
        sampler.stop()
        try:
            client.shutdown()
        except Exception:  # noqa: BLE001
            pass
        shutil.rmtree(stage, ignore_errors=True)

    result = {
        "schema": SCHEMA,
        "version": args.version,
        "platform": f"{platform.system().lower()}-{platform.machine()}",
        "scope": args.scope,
        "corpus_revision": manifest["corpus"]["revision"],
        "corpus_files": total,
        "documents": doc_meta,
        "anchors": anchors,
        "host": host_info(),
        "checks": bench.checks,
        "timeline": sampler.samples,
    }
    args.out.mkdir(parents=True, exist_ok=True)
    dest = args.out / f"{args.version}.json"
    # `sort_keys` + a trailing newline so committed results diff cleanly.
    dest.write_text(json.dumps(result, indent=2, sort_keys=True) + "\n")

    failed = [c["id"] for c in bench.checks if not c["ok"]]
    peak = max((s["rss_kib"] for s in sampler.samples), default=0)
    print(
        f"\n   peak RSS {peak / 1024:.1f} MiB | "
        f"{len(bench.checks)} checks, {len(failed)} failed"
        + (f" ({', '.join(failed)})" if failed else "")
    )
    print(f"   wrote {dest}")
    return 1 if (failed and args.fail_on_failed_check) else 0


if __name__ == "__main__":
    raise SystemExit(main())
