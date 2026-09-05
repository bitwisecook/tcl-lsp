#!/usr/bin/env python3
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
"""LSP-API-driven stress suite for tcl-lsp-server — issue #829 robustness suite,
"through the front end" half.

Drives the *real* `tcl-lsp-server` binary over stdio JSON-RPC — exactly what an
editor does — under adversarial concurrent load, to prove the semantic-token
prioritisation / W120 workspace-scan fixes hold up outside the tidy conditions
of a unit or single-shot e2e test. See
`rust/tcl-lsp-db/examples/stress_concurrent_analysis.rs` for the sibling
"direct infrastructure, no LSP" half of this suite.

Only the Python standard library is used, so this runs on any host with
Python 3.9+ and a built `tcl-lsp-server` binary — no Rust toolchain, no
project checkout beyond the binary itself, required to *run* it (building the
binary still needs the workspace, as always).

Usage:
    python3 scripts/stress/stress_lsp.py --server-bin target/release/tcl-lsp-server
    python3 scripts/stress/stress_lsp.py --server-bin <bin> --scenario tokens --duration 30
    python3 scripts/stress/stress_lsp.py --server-bin <bin> --all --duration 20

Scenarios:
    tokens   Many large documents, concurrent rapid-edit + immediate
             semanticTokens/full bursts. Asserts every response arrives within
             a hard ceiling (never starved — issue #829's core complaint) and
             that responses are always well-formed.
    startup  Reproduces the exact race from issue #829's screenshots: a
             workspace with a `source`-ancestor file that requires a package,
             and a module using that package with no local `package require`.
             Opens the module immediately (racing the server's own workspace
             scan, the same way an editor restoring tabs races
             `initialized`), then asserts the false-positive W120 the ancestor
             should suppress is gone once the server settles — not still
             standing forever.
    chaos    Many documents opened / edited / closed / reopened rapidly and
             concurrently for the whole run, checking only that the server
             stays alive and responsive throughout (a final round-trip
             request succeeds) — a crash/hang/deadlock smoke test.

Exit code is 0 iff every requested scenario passed. A concise PASS/FAIL
summary with timing percentiles is always printed.

On any failure, look for a line starting `STRESS_FAILURE:` — it names a
self-contained reproduction bundle directory (default under
`$TMPDIR/tcl-lsp-stress-artifacts/`, override with `--artifacts-dir` or
`TCL_LSP_STRESS_ARTIFACTS`) containing everything needed to reconstruct the
failure without re-running the (inherently timing-dependent) stress harness:
the exact document text(s) involved, a JSON-RPC replay transcript, recent
server stderr, and (for `startup`) the whole on-disk workspace that would
otherwise be deleted the instant its `TemporaryDirectory` goes out of scope.
Every bundle's `FAILURE.md` also suggests where in the codebase to turn it
into a permanent regression test. The sibling Rust suite
(`stress_concurrent_analysis.rs`) uses the identical `STRESS_FAILURE:` marker,
so `grep STRESS_FAILURE:` over both suites' combined output finds every
bundle from a single command — the intended way to run this from an
automated (e.g. Claude Code) harness that needs to see and act on failures
without a human re-running anything by hand.
"""

from __future__ import annotations

import argparse
import json
import os
import random
import shutil
import subprocess
import sys
import tempfile
import threading
import time
from collections import deque
from dataclasses import dataclass

# -- failure artifacts: self-contained reproduction bundles ------------------
#
# Mirrors `rust/tcl-lsp-db/examples/stress_concurrent_analysis.rs`'s
# `dump_repro_bundle`: on any scenario failure, write everything needed to
# reconstruct it — the exact document text(s), the JSON-RPC messages sent
# (a replay log), server stderr, and a plain-English description — to a
# discoverable directory, and print a single `STRESS_FAILURE:` line naming
# it. Both halves of the #829 stress suite use the identical marker string
# so a person or an agent driving either one can `grep STRESS_FAILURE:` the
# combined output and go straight to the bundle, rather than re-running an
# inherently timing-dependent stress harness to reproduce what it saw.

ARTIFACTS_ROOT = os.environ.get(
    "TCL_LSP_STRESS_ARTIFACTS",
    os.path.join(tempfile.gettempdir(), "tcl-lsp-stress-artifacts"),
)


def dump_repro_bundle(
    scenario: str,
    tag: str,
    failure: str,
    *,
    files: dict[str, str] | None = None,
    copy_dirs: dict[str, str] | None = None,
    context: dict[str, object] | None = None,
) -> str:
    """Write a self-contained reproduction bundle and print the
    `STRESS_FAILURE:` marker line naming it.

    `files` is `{relative_path: content}` written verbatim (document text,
    JSON-RPC transcripts, server stderr, …). `copy_dirs` is
    `{dest_name: source_path}` — whole directories (e.g. a scenario's
    workspace, which would otherwise be deleted with its `TemporaryDirectory`
    the moment the failing `with` block exits) copied into the bundle.
    `context` is arbitrary structured data (args, seeds, counters) dumped as
    `context.json` for machine consumption. Returns the bundle directory.
    """
    stamp = time.strftime("%Y%m%dT%H%M%S", time.localtime())
    bundle = os.path.join(ARTIFACTS_ROOT, f"{scenario}-{tag}-{stamp}-{os.getpid()}")
    os.makedirs(bundle, exist_ok=True)
    with open(os.path.join(bundle, "FAILURE.md"), "w") as f:
        f.write(f"# {scenario} / {tag}\n\n## What happened\n\n{failure}\n")
        if context:
            f.write(
                "\n## Context\n\n```json\n"
                + json.dumps(context, indent=2, default=str)
                + "\n```\n"
            )
        if files:
            f.write("\n## Files in this bundle\n\n")
            f.writelines(f"- `{name}`\n" for name in sorted(files))
        if copy_dirs:
            f.write("\n## Directories in this bundle\n\n")
            f.writelines(
                f"- `{name}/` (copy of the scenario's on-disk workspace)\n"
                for name in sorted(copy_dirs)
            )
    for name, content in (files or {}).items():
        path = os.path.join(bundle, name)
        os.makedirs(os.path.dirname(path) or bundle, exist_ok=True)
        with open(path, "w") as f:
            f.write(content)
    for name, src in (copy_dirs or {}).items():
        dest = os.path.join(bundle, name)
        try:
            shutil.copytree(src, dest, dirs_exist_ok=True)
        except OSError as exc:
            with open(os.path.join(bundle, f"{name}.copy_error.txt"), "w") as f:
                f.write(f"failed to copy {src!r}: {exc}\n")
    if context is not None:
        with open(os.path.join(bundle, "context.json"), "w") as f:
            json.dump(context, f, indent=2, default=str)
    print(f"STRESS_FAILURE: [{scenario}/{tag}] reproduction bundle at {bundle}")
    return bundle


# -- minimal JSON-RPC client (stdlib only) -----------------------------------

_TRUNCATE_AT = 400


def _truncate_large_text_fields(payload: dict) -> dict:
    """Shrink `text` / `contentChanges[].text` fields in a JSON-RPC message
    before it goes into the (memory-bounded) transcript ring buffer — a
    single `didOpen`/`didChange` for a stress-scale document can be hundreds
    of KB, and the transcript only needs to show *that* a large document was
    sent, not carry every full copy. The failing document's actual current
    text is captured separately (in full) by whichever scenario calls
    `dump_repro_bundle`, so nothing is lost for reproduction purposes."""

    def shrink(v: str) -> str:
        if len(v) <= _TRUNCATE_AT:
            return v
        return f"{v[:_TRUNCATE_AT]}…[{len(v) - _TRUNCATE_AT} more chars truncated for transcript]"

    try:
        params = payload.get("params")
        if not isinstance(params, dict):
            return payload
        out = dict(payload)
        out_params = dict(params)
        td = out_params.get("textDocument")
        if isinstance(td, dict) and isinstance(td.get("text"), str):
            out_params["textDocument"] = {**td, "text": shrink(td["text"])}
        changes = out_params.get("contentChanges")
        if isinstance(changes, list):
            out_params["contentChanges"] = [
                {**c, "text": shrink(c["text"])}
                if isinstance(c, dict) and isinstance(c.get("text"), str)
                else c
                for c in changes
            ]
        out["params"] = out_params
        return out
    except (TypeError, AttributeError):
        return payload


class LspError(RuntimeError):
    pass


class LspClient:
    """A minimal, thread-safe LSP JSON-RPC client over a subprocess's stdio.

    Deliberately small — this is a stress driver, not a general-purpose
    editor client. Requests block the calling thread only; multiple threads
    may call `request`/`notify` concurrently (each request gets its own id
    and waits on its own condition variable), matching how several editor
    panes can have requests in flight at once against one server process.
    """

    def __init__(self, server_bin: str, cwd: str):
        self.process = subprocess.Popen(
            [server_bin],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=cwd,
        )
        self._next_id = 1
        self._id_lock = threading.Lock()
        self._write_lock = threading.Lock()
        self._pending: dict[int, dict] = {}
        self._pending_lock = threading.Lock()
        self._pending_cv = threading.Condition(self._pending_lock)
        self._notifications: list[dict] = []
        self._server_requests: list[dict] = []
        self._notif_lock = threading.Lock()
        self._stderr_lines: list[str] = []
        self._alive = True
        # Bounded replay log of every JSON-RPC message crossing the wire in
        # either direction, timestamped relative to client construction — the
        # raw material for a `dump_repro_bundle` transcript. Bounded (not
        # unlimited) so a long-running chaos/tokens scenario doesn't grow
        # this without limit; a failure cares about recent history, not the
        # whole run.
        self._transcript: deque[dict] = deque(maxlen=4000)
        self._transcript_lock = threading.Lock()
        self._start_time = time.monotonic()
        self._reader = threading.Thread(target=self._read_loop, daemon=True)
        self._reader.start()
        self._stderr_reader = threading.Thread(target=self._stderr_loop, daemon=True)
        self._stderr_reader.start()

    def _read_loop(self) -> None:
        stdout = self.process.stdout
        assert stdout is not None
        while True:
            headers = {}
            while True:
                line = stdout.readline()
                if not line:
                    self._alive = False
                    with self._pending_cv:
                        self._pending_cv.notify_all()
                    return
                line = line.decode("utf-8", errors="replace").rstrip("\r\n")
                if line == "":
                    break
                if ":" in line:
                    k, v = line.split(":", 1)
                    headers[k.strip().lower()] = v.strip()
            length = int(headers.get("content-length", "0"))
            if length == 0:
                continue
            body = stdout.read(length)
            try:
                msg = json.loads(body)
            except json.JSONDecodeError:
                continue
            self._route(msg)

    def _record(self, direction: str, payload: dict) -> None:
        with self._transcript_lock:
            self._transcript.append(
                {
                    "t": round(time.monotonic() - self._start_time, 4),
                    "dir": direction,
                    "msg": payload,
                }
            )

    def transcript_jsonl(self) -> str:
        """The bounded replay log as newline-delimited JSON, oldest first —
        every JSON-RPC message sent or received recently enough to still be
        in the ring buffer, timestamped relative to client start. Intended
        for `dump_repro_bundle`, not for programmatic replay (there is no
        replay tool here) — it is read material for a person or an agent
        reconstructing what led to a failure."""
        with self._transcript_lock:
            return "\n".join(json.dumps(entry) for entry in self._transcript)

    def _route(self, msg: dict) -> None:
        self._record("recv", msg)
        has_id = "id" in msg and msg["id"] is not None
        is_request = "method" in msg
        if has_id and not is_request:
            with self._pending_cv:
                self._pending[msg["id"]] = msg
                self._pending_cv.notify_all()
        elif has_id and is_request:
            with self._notif_lock:
                self._server_requests.append(msg)
            self._auto_reply(msg)
        else:
            with self._notif_lock:
                self._notifications.append(msg)

    def _auto_reply(self, msg: dict) -> None:
        method = msg.get("method", "")
        result: object = None
        if method == "workspace/configuration":
            items = (msg.get("params") or {}).get("items") or []
            result = [None for _ in items]
        self._send({"jsonrpc": "2.0", "id": msg.get("id"), "result": result})

    def _stderr_loop(self) -> None:
        stderr = self.process.stderr
        assert stderr is not None
        for raw in stderr:
            self._stderr_lines.append(
                raw.decode("utf-8", errors="replace").rstrip("\n")
            )

    def _send(self, payload: dict) -> None:
        self._record("send", _truncate_large_text_fields(payload))
        body = json.dumps(payload).encode("utf-8")
        header = f"Content-Length: {len(body)}\r\n\r\n".encode("ascii")
        try:
            with self._write_lock:
                assert self.process.stdin is not None
                self.process.stdin.write(header + body)
                self.process.stdin.flush()
        except (BrokenPipeError, OSError) as exc:
            # The server process died (or its stdin closed) between the last
            # successful write and this one — a genuine, expected failure
            # mode under stress (issue #829 robustness suite), not a bug in
            # this harness. Mark dead and surface it the same way a timed-out
            # request does, rather than letting a raw `BrokenPipeError`
            # propagate out of a worker thread as an unhandled exception and
            # crash the whole stress run before it gets to report/dump
            # anything.
            self._alive = False
            with self._pending_cv:
                self._pending_cv.notify_all()
            raise LspError(f"write failed, server likely dead: {exc}") from exc

    def request(self, method: str, params: dict, timeout: float = 30.0) -> dict:
        with self._id_lock:
            req_id = self._next_id
            self._next_id += 1
        self._send({"jsonrpc": "2.0", "id": req_id, "method": method, "params": params})
        deadline = time.monotonic() + timeout
        with self._pending_cv:
            while req_id not in self._pending:
                remaining = deadline - time.monotonic()
                if remaining <= 0 or not self._alive:
                    raise LspError(
                        f"timed out after {timeout}s waiting for {method!r} "
                        f"(alive={self._alive}); stderr tail:\n"
                        + "\n".join(self._stderr_lines[-30:])
                    )
                self._pending_cv.wait(timeout=remaining)
            msg = self._pending.pop(req_id)
        if "error" in msg and msg["error"] is not None:
            raise LspError(f"{method} -> error {msg['error']}")
        result = msg.get("result")
        return result if isinstance(result, dict) else {}

    def notify(self, method: str, params: dict) -> None:
        try:
            self._send({"jsonrpc": "2.0", "method": method, "params": params})
        except LspError:
            # Best-effort: a notification has no response to fail on, so a dead
            # server is swallowed here rather than raised — the caller's *next*
            # `request()` call (every scenario's loop always follows a burst of
            # notifies with a request) hits the same dead-server condition and
            # raises `LspError` there, where it's already handled. This is what
            # stops a `didOpen`/`didChange`/`didClose` call site (none of which
            # wrap `notify()` in try/except, since normally there is nothing to
            # handle) from raising an unhandled exception straight out of a
            # worker thread the instant the server dies mid-run.
            pass

    def initialize(self, root_uri: str) -> dict:
        # Empty `capabilities`, matching the native e2e harness
        # (`rust/tcl-lsp-server/tests/e2e/common/mod.rs`): several LSP
        # capability sub-objects (e.g. `semanticTokens`) are deserialised
        # strictly, so declaring one at all requires populating every
        # non-optional field (`tokenTypes`, `tokenModifiers`, …) — omitting
        # the object entirely is simpler and the server does not gate
        # `semanticTokens/full` (or any other feature this suite exercises)
        # behind the client advertising it.
        result = self.request(
            "initialize",
            {
                "processId": os.getpid(),
                "rootUri": root_uri,
                "workspaceFolders": [{"uri": root_uri, "name": "stress"}],
                "capabilities": {},
            },
            timeout=60.0,
        )
        self.notify("initialized", {})
        return result

    def shutdown(self) -> None:
        try:
            self.request("shutdown", {}, timeout=5.0)
            self.notify("exit", {})
        except LspError:
            # A server that is already dead needs no polite shutdown; the
            # wait/kill below reaps it either way.
            pass
        try:
            self.process.wait(timeout=5.0)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait(timeout=5.0)

    def is_alive(self) -> bool:
        return self.process.poll() is None

    def stderr_tail(self, n: int = 40) -> str:
        return "\n".join(self._stderr_lines[-n:])


# -- fixture generators -------------------------------------------------------


def generate_big_tcl(n: int) -> str:
    """Same shape as the Rust suites' `generate_big_tcl` (n procs, ~10
    lines each) — kept independent so all three legs of the #829 stress
    suite (this script, the native lsp_e2e tests, and the direct-infra
    example) exercise comparable-weight fixtures without sharing code across
    language boundaries."""
    parts = ["namespace eval ::bench {\n    variable counter 0\n}\n\n"]
    for i in range(n):
        parts.append(
            f"# proc number {i}\n"
            f"proc ::bench::step{i} {{a b}} {{\n"
            f"    set v{i} [expr {{$a + $b}}]\n"
            f'    set msg "step {i} = $v{i}"\n'
            f"    if {{$v{i} > 10}} {{\n"
            f"        set v{i} [expr {{$v{i} + 1}}]\n"
            f"    }}\n"
            f"    return $v{i}\n"
            f"}}\n\n"
        )
    return "".join(parts)


def uri_for(path: str) -> str:
    return "file://" + os.path.abspath(path)


# -- scenario: tokens ---------------------------------------------------------


@dataclass
class LatencySample:
    ok: bool
    elapsed: float
    doc_id: int
    round_num: int
    detail: str = ""


# Cap on how many failures get a full repro bundle dumped — a genuinely
# broken build can fail every request, and a bundle per failure would flood
# disk and stdout for no extra diagnostic value once the first few have
# established the pattern. The cap itself is announced (never a silent
# truncation), and the summary always reports the true total failure count.
_MAX_REPRO_BUNDLES_PER_SCENARIO = 3


def scenario_tokens(server_bin: str, duration: float, docs: int, procs: int) -> bool:
    print(f"[tokens] {docs} documents x {procs} procs, {duration:.0f}s")
    with tempfile.TemporaryDirectory(prefix="tcl-lsp-stress-tokens-") as root:
        client = LspClient(server_bin, root)
        client.initialize(uri_for(root))

        samples: list[LatencySample] = []
        samples_lock = threading.Lock()
        # The current text of each open document, kept only by that
        # document's own worker thread (single-writer, so no lock needed) —
        # read after `join()` by the failure-dump path to capture the exact
        # content a failing request was made against.
        doc_texts: dict[int, str] = {}
        doc_uris: dict[int, str] = {}
        stop_at = time.monotonic() + duration

        def worker(doc_id: int) -> None:
            path = os.path.join(root, f"doc{doc_id}.tcl")
            uri = uri_for(path)
            doc_uris[doc_id] = uri
            text = generate_big_tcl(procs)
            doc_texts[doc_id] = text
            client.notify(
                "textDocument/didOpen",
                {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "tcl",
                        "version": 1,
                        "text": text,
                    }
                },
            )
            version = 1
            round_num = 0
            while time.monotonic() < stop_at and client.is_alive():
                round_num += 1
                version += 1
                text = text + f"# edit {doc_id}/{round_num}\n"
                doc_texts[doc_id] = text
                client.notify(
                    "textDocument/didChange",
                    {
                        "textDocument": {"uri": uri, "version": version},
                        "contentChanges": [{"text": text}],
                    },
                )
                started = time.monotonic()
                try:
                    result = client.request(
                        "textDocument/semanticTokens/full",
                        {"textDocument": {"uri": uri}},
                        timeout=30.0,
                    )
                    elapsed = time.monotonic() - started
                    ok = bool(result and result.get("data"))
                    with samples_lock:
                        samples.append(
                            LatencySample(
                                ok,
                                elapsed,
                                doc_id,
                                round_num,
                                "" if ok else "empty tokens",
                            )
                        )
                except LspError as exc:
                    elapsed = time.monotonic() - started
                    with samples_lock:
                        samples.append(
                            LatencySample(False, elapsed, doc_id, round_num, str(exc))
                        )
            client.notify("textDocument/didClose", {"textDocument": {"uri": uri}})

        threads = [threading.Thread(target=worker, args=(i,)) for i in range(docs)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()

        alive = client.is_alive()

        if not samples:
            print("[tokens] FAIL — no samples collected")
            client.shutdown()
            return False

        failures = [s for s in samples if not s.ok]
        elapsed_sorted = sorted(s.elapsed for s in samples)
        p50 = elapsed_sorted[len(elapsed_sorted) // 2]
        p95 = elapsed_sorted[int(len(elapsed_sorted) * 0.95)]
        worst = elapsed_sorted[-1]
        print(
            f"[tokens] {len(samples)} requests, {len(failures)} failed, "
            f"server_alive={alive}, p50={p50 * 1000:.0f}ms p95={p95 * 1000:.0f}ms "
            f"max={worst * 1000:.0f}ms"
        )

        # Dump repro bundles — still inside the `with` block, and before
        # `shutdown()`, so the document text, the live transcript, and
        # server stderr are all still captureable.
        for f in failures[:_MAX_REPRO_BUNDLES_PER_SCENARIO]:
            dump_repro_bundle(
                "tokens",
                f"doc{f.doc_id}-round{f.round_num}",
                f"semanticTokens/full request for document {f.doc_id} "
                f"({doc_uris.get(f.doc_id, '?')}), edit round {f.round_num}, "
                f"failed after {f.elapsed:.2f}s: {f.detail}",
                files={
                    "failing_doc.tcl": doc_texts.get(f.doc_id, "<text unavailable>"),
                    "transcript.jsonl": client.transcript_jsonl(),
                    "server_stderr.txt": client.stderr_tail(500),
                },
                context={
                    "scenario_args": {
                        "duration": duration,
                        "docs": docs,
                        "procs": procs,
                    },
                    "doc_id": f.doc_id,
                    "round": f.round_num,
                    "elapsed_seconds": f.elapsed,
                    "detail": f.detail,
                },
            )
        if len(failures) > _MAX_REPRO_BUNDLES_PER_SCENARIO:
            print(
                f"[tokens] {len(failures) - _MAX_REPRO_BUNDLES_PER_SCENARIO} further failures "
                f"not individually bundled (capped at {_MAX_REPRO_BUNDLES_PER_SCENARIO})"
            )

        client.shutdown()

    if not alive:
        print("[tokens] FAIL — server process died during the run")
        return False
    if failures:
        print(
            f"[tokens] FAIL — {len(failures)} of {len(samples)} requests failed or timed out"
        )
        return False
    print("[tokens] PASS")
    return True


# -- scenario: startup (W120 workspace-scan race) -----------------------------


def scenario_startup(server_bin: str, iterations: int) -> bool:
    print(f"[startup] {iterations} iterations")
    ok_count = 0
    for i in range(iterations):
        with tempfile.TemporaryDirectory(prefix="tcl-lsp-stress-startup-") as root:
            http_dir = os.path.join(root, "http")
            os.makedirs(http_dir, exist_ok=True)
            with open(os.path.join(http_dir, "http.tcl"), "w") as f:
                f.write("package provide http 2.9\nproc http::register {a b c} {}\n")
            with open(os.path.join(http_dir, "pkgIndex.tcl"), "w") as f:
                f.write(
                    "package ifneeded http 2.9 [list source [file join $dir http.tcl]]\n"
                )
            lib_dir = os.path.join(root, "lib")
            os.makedirs(lib_dir, exist_ok=True)
            with open(os.path.join(lib_dir, "util.tcl"), "w") as f:
                f.write("http::register foo 80 bar\n")
            with open(os.path.join(root, "app.tcl"), "w") as f:
                f.write("package require http\nsource lib/util.tcl\n")
            # Pad the workspace with unrelated filler files so the scan has
            # real work to do and can plausibly still be running when the
            # module opens, reproducing the reported race rather than a
            # workspace so trivial the scan always wins.
            for n in range(200):
                with open(os.path.join(root, f"filler{n}.tcl"), "w") as f:
                    f.write(f"proc ::filler::p{n} {{}} {{ return {n} }}\n")

            client = LspClient(server_bin, root)
            client.initialize(uri_for(root))
            # Open the module *immediately*, the same instant `initialized`
            # (and the scan it triggers) fires — a client's real startup
            # sequence for a restored tab.
            uri = uri_for(os.path.join(lib_dir, "util.tcl"))
            with open(os.path.join(lib_dir, "util.tcl")) as f:
                text = f.read()
            client.notify(
                "textDocument/didOpen",
                {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "tcl",
                        "version": 1,
                        "text": text,
                    }
                },
            )

            # Settle: a synchronous pull request blocks until the server has
            # analysed the document's current content (see the native
            # harness's `settle_analysis` for the same technique), so this is
            # a deterministic "has it converged" barrier rather than a fixed
            # sleep.
            deadline = time.monotonic() + 20.0
            diagnostics: list[dict] = []
            while time.monotonic() < deadline:
                try:
                    report = client.request(
                        "textDocument/diagnostic",
                        {"textDocument": {"uri": uri}},
                        timeout=10.0,
                    )
                    diagnostics = (report or {}).get("items") or []
                except LspError:
                    diagnostics = []
                if not any(d.get("code") == "W120" for d in diagnostics):
                    break
                time.sleep(0.25)

            alive = client.is_alive()
            has_w120 = any(d.get("code") == "W120" for d in diagnostics)
            if has_w120 or not alive:
                print(
                    f"[startup] iteration {i}: FAIL — w120_present={has_w120} server_alive={alive}"
                )
                # Copy the whole workspace out *before* the `with` block
                # below deletes it with `root` — the ancestor/pkgIndex/filler
                # layout is exactly what would otherwise be lost, and is
                # what actually reproduces the race.
                dump_repro_bundle(
                    "startup",
                    f"iteration{i}",
                    (
                        f"After opening {uri} immediately at startup (racing the workspace "
                        f"scan `initialized` triggers) and settling for up to 20s, "
                        + (
                            "diagnostics still contained a false-positive W120 that the "
                            "http-requiring, util.tcl-sourcing app.tcl ancestor should have "
                            "suppressed."
                            if has_w120
                            else "the server process was no longer alive."
                        )
                    ),
                    files={
                        "diagnostics.json": json.dumps(diagnostics, indent=2),
                        "transcript.jsonl": client.transcript_jsonl(),
                        "server_stderr.txt": client.stderr_tail(500),
                    },
                    copy_dirs={"workspace": root},
                    context={
                        "iteration": i,
                        "has_w120": has_w120,
                        "server_alive": alive,
                        "opened_uri": uri,
                    },
                )
            else:
                ok_count += 1
            client.shutdown()

    print(f"[startup] {ok_count}/{iterations} converged to no false-positive W120")
    if ok_count != iterations:
        print("[startup] FAIL")
        return False
    print("[startup] PASS")
    return True


# -- scenario: chaos (open/edit/close churn) ----------------------------------


def scenario_chaos(server_bin: str, duration: float, docs: int) -> bool:
    print(f"[chaos] {docs} documents, {duration:.0f}s of random churn")
    # A bounded ring buffer of every action any worker took, in
    # approximate chronological order (append-only, one shared lock) — the
    # exact sequence a repro needs to know "what series of opens / edits /
    # closes / requests, on which documents, preceded a crash or hang", since
    # `chaos`'s whole point is that no single request's failure is the
    # story — the *accumulated* churn is.
    action_log: deque[str] = deque(maxlen=2000)
    action_log_lock = threading.Lock()
    seed = 0xC0FFEE

    def log_action(worker_id: int, action: str, uri: str, outcome: str = "") -> None:
        with action_log_lock:
            action_log.append(
                f"t={time.monotonic():.3f} worker={worker_id} {action} {uri} {outcome}"
            )

    with tempfile.TemporaryDirectory(prefix="tcl-lsp-stress-chaos-") as root:
        client = LspClient(server_bin, root)
        client.initialize(uri_for(root))
        rng = random.Random(seed)
        stop_at = time.monotonic() + duration
        errors: list[str] = []
        errors_lock = threading.Lock()

        def worker(worker_id: int) -> None:
            local_rng = random.Random(rng.randint(0, 2**31) + worker_id)
            local_open: set[str] = set()
            while time.monotonic() < stop_at and client.is_alive():
                action = local_rng.choice(["open", "edit", "close", "tokens", "hover"])
                doc_id = local_rng.randrange(docs)
                uri = uri_for(os.path.join(root, f"chaos{worker_id}_{doc_id}.tcl"))
                log_action(worker_id, action, uri)
                try:
                    if action == "open" and uri not in local_open:
                        text = f"proc p{doc_id} {{}} {{ set x {doc_id} }}\n"
                        client.notify(
                            "textDocument/didOpen",
                            {
                                "textDocument": {
                                    "uri": uri,
                                    "languageId": "tcl",
                                    "version": 1,
                                    "text": text,
                                }
                            },
                        )
                        local_open.add(uri)
                    elif action == "edit" and uri in local_open:
                        v = local_rng.randint(2, 10_000)
                        client.notify(
                            "textDocument/didChange",
                            {
                                "textDocument": {"uri": uri, "version": v},
                                "contentChanges": [
                                    {"text": f"proc p{doc_id} {{}} {{ set x {v} }}\n"}
                                ],
                            },
                        )
                    elif action == "close" and uri in local_open:
                        client.notify(
                            "textDocument/didClose", {"textDocument": {"uri": uri}}
                        )
                        local_open.discard(uri)
                    elif action == "tokens" and uri in local_open:
                        client.request(
                            "textDocument/semanticTokens/full",
                            {"textDocument": {"uri": uri}},
                            timeout=10.0,
                        )
                    elif action == "hover" and uri in local_open:
                        client.request(
                            "textDocument/hover",
                            {
                                "textDocument": {"uri": uri},
                                "position": {"line": 0, "character": 5},
                            },
                            timeout=10.0,
                        )
                except LspError as exc:
                    log_action(worker_id, action, uri, outcome=f"ERROR: {exc}")
                    with errors_lock:
                        errors.append(f"worker {worker_id} {action}: {exc}")

        threads = [threading.Thread(target=worker, args=(i,)) for i in range(4)]
        for t in threads:
            t.start()
        for t in threads:
            t.join()

        # Final liveness/responsiveness check: a plain round trip must still
        # succeed after the whole churn window.
        alive = client.is_alive()
        # `None`, not `True`, when dead: there is nothing to have been
        # responsive *to* — printing a stale default would misleadingly
        # suggest the round trip was attempted and succeeded.
        responsive: bool | None = None
        if alive:
            try:
                # `workspace/symbol` needs no document URI, so it can't
                # spuriously fail on an unopened/non-document URI the way a
                # textDocument/* request targeting uri_for(root) (a
                # directory, never didOpen'd) would -- this call must prove
                # the server is *responsive*, not exercise any particular
                # document.
                client.request("workspace/symbol", {"query": ""}, timeout=10.0)
                responsive = True
            except LspError:
                responsive = False

        if not alive or not responsive:
            with action_log_lock:
                action_log_text = "\n".join(action_log)
            dump_repro_bundle(
                "chaos",
                "unresponsive" if alive else "died",
                (
                    f"After {duration:.0f}s of concurrent open/edit/close/tokens/hover churn "
                    f"across {docs} logical documents (4 worker threads, RNG seed {hex(seed)}), "
                    + (
                        "the server stopped responding to a final round trip."
                        if alive
                        else "the server process had exited."
                    )
                ),
                files={
                    "action_log.txt": action_log_text,
                    "transcript.jsonl": client.transcript_jsonl(),
                    "server_stderr.txt": client.stderr_tail(500),
                },
                context={
                    "scenario_args": {"duration": duration, "docs": docs},
                    "seed": hex(seed),
                    "server_alive": alive,
                    "responsive": responsive,
                    "request_error_count": len(errors),
                },
            )

        client.shutdown()

    print(
        f"[chaos] {len(errors)} request errors, server_alive={alive}, responsive={responsive}"
    )
    for e in errors[:5]:
        print(f"[chaos]   {e}")
    if not alive or not responsive:
        print("[chaos] FAIL — server died or stopped responding under churn")
        return False
    print("[chaos] PASS")
    return True


# -- CLI -----------------------------------------------------------------------


def main() -> int:
    global ARTIFACTS_ROOT
    parser = argparse.ArgumentParser(
        description=__doc__, formatter_class=argparse.RawDescriptionHelpFormatter
    )
    parser.add_argument(
        "--server-bin", required=True, help="Path to the tcl-lsp-server binary"
    )
    parser.add_argument(
        "--scenario",
        choices=["tokens", "startup", "chaos"],
        action="append",
        dest="scenarios",
        help="Run only this scenario (repeatable). Default: all.",
    )
    parser.add_argument(
        "--all", action="store_true", help="Run all scenarios (default if none named)"
    )
    parser.add_argument(
        "--duration",
        type=float,
        default=20.0,
        help="Seconds per timed scenario (tokens, chaos)",
    )
    parser.add_argument(
        "--docs", type=int, default=6, help="Concurrent documents for tokens/chaos"
    )
    parser.add_argument(
        "--procs",
        type=int,
        default=300,
        help="Procs per document for the tokens scenario",
    )
    parser.add_argument(
        "--startup-iterations",
        type=int,
        default=5,
        help="Iterations for the startup scenario",
    )
    parser.add_argument(
        "--artifacts-dir",
        default=None,
        help=(
            "Where failure reproduction bundles are written (overrides "
            "TCL_LSP_STRESS_ARTIFACTS). Default: "
            f"{ARTIFACTS_ROOT!r}"
        ),
    )
    args = parser.parse_args()

    # Resolve to an absolute path up front: each scenario spawns the server
    # with `cwd` set to its own per-scenario temp workspace, so a relative
    # `--server-bin` (e.g. the README's `target/release/tcl-lsp-server`,
    # relative to the repo root) would otherwise be looked up against the
    # wrong directory and fail with a confusing FileNotFoundError.
    resolved_bin = shutil.which(args.server_bin) or (
        args.server_bin if os.path.isfile(args.server_bin) else None
    )
    if resolved_bin is None:
        print(f"error: no server binary at {args.server_bin!r}", file=sys.stderr)
        return 2
    args.server_bin = os.path.abspath(resolved_bin)

    if args.artifacts_dir:
        ARTIFACTS_ROOT = args.artifacts_dir

    scenarios = args.scenarios or ["tokens", "startup", "chaos"]
    results: dict[str, bool] = {}
    for name in scenarios:
        if name == "tokens":
            results["tokens"] = scenario_tokens(
                args.server_bin, args.duration, args.docs, args.procs
            )
        elif name == "startup":
            results["startup"] = scenario_startup(
                args.server_bin, args.startup_iterations
            )
        elif name == "chaos":
            results["chaos"] = scenario_chaos(args.server_bin, args.duration, args.docs)

    print("\n=== stress_lsp summary ===")
    for name, passed in results.items():
        print(f"  {name}: {'PASS' if passed else 'FAIL'}")
    return 0 if all(results.values()) else 1


if __name__ == "__main__":
    sys.exit(main())
