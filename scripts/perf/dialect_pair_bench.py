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

"""Drive one server over one corpus under several forced dialects.

The same bytes are analysed once per dialect in a fresh server, so a
difference between rows is what the dialect's command surface costs — for
`jim` against `tcl8.6`, what a SpecTcl-authored surface costs against the
compiled-in one.

    python3 scripts/perf/dialect_pair_bench.py --server target/ci/tcl-lsp-server \\
        --corpus path/to/jimtcl --dialect jim --dialect tcl8.6 --out result.json
"""

from __future__ import annotations

import argparse
import json
import os
import statistics
import sys
import time
from pathlib import Path
from typing import Any

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent.parent / ".claude" / "skills" / "lsp-client"))

import lsp_client

DEFAULT_PATTERNS = ("*.tcl", "*.test", "*.cfg", "auto.def", "*.tclspec")


class DialectClient(lsp_client.LspClient):
    """An `LspClient` whose `workspace/configuration` answer pins a dialect."""

    def __init__(self, server_dir: str, launch_cmd: list[str], dialect: str) -> None:
        super().__init__(server_dir, launch_cmd)
        self.dialect = dialect

    def _answer_server_request(
        self, method: str, request_id: int, params: dict | None
    ) -> None:
        if method != "workspace/configuration":
            super()._answer_server_request(method, request_id, params)
            return
        items = (params or {}).get("items", []) or [{}]
        result = [self.settings_for(item.get("section")) for item in items]
        self._send({"jsonrpc": "2.0", "id": request_id, "result": result})

    def settings_for(self, section: str | None) -> dict[str, Any]:
        settings = {"dialect": self.dialect}
        if section in (None, ""):
            return {"tclLsp": settings}
        return settings if section == "tclLsp" else {}


def corpus_files(
    root: Path,
    patterns: tuple[str, ...],
    limit: int | None,
    exclude: tuple[str, ...] = (),
) -> list[Path]:
    found = sorted(
        {p for pattern in patterns for p in root.rglob(pattern) if p.is_file()}
    )
    found = [
        p
        for p in found
        if ".git" not in p.parts
        and not any(p.relative_to(root).match(glob) for glob in exclude)
    ]
    return found[:limit] if limit else found


def rss_kib(pid: int) -> dict[str, int]:
    out: dict[str, int] = {}
    try:
        with open(f"/proc/{pid}/status", encoding="ascii") as status:
            for line in status:
                key, _, value = line.partition(":")
                if key in ("VmRSS", "VmHWM"):
                    out[key] = int(value.split()[0])
    except OSError:
        pass
    return out


def percentile(values: list[float], pct: float) -> float:
    if not values:
        return 0.0
    ordered = sorted(values)
    index = min(len(ordered) - 1, round(pct / 100 * (len(ordered) - 1)))
    return ordered[index]


def summary(values: list[float]) -> dict[str, float]:
    if not values:
        return {"n": 0}
    return {
        "n": len(values),
        "total_ms": round(sum(values), 1),
        "mean_ms": round(statistics.fmean(values), 2),
        "p50_ms": round(percentile(values, 50), 2),
        "p95_ms": round(percentile(values, 95), 2),
        "max_ms": round(max(values), 2),
    }


def ms_since(start: float) -> float:
    return (time.perf_counter() - start) * 1000


def wait_for_publish(
    client: lsp_client.LspClient, uri: str, after: int, timeout: float
) -> bool:
    """Wait for a `publishDiagnostics` for *uri* beyond the first *after* seen."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        with client._lock:
            seen = sum(
                1
                for n in client._notifications
                if n.get("method") == "textDocument/publishDiagnostics"
                and n.get("params", {}).get("uri") == uri
            )
        if seen > after:
            return True
        time.sleep(0.002)
    return False


def semantic_tokens(client: lsp_client.LspClient, uri: str, timeout: float) -> Any:
    """`semanticTokens/full`, retried on ContentModified as an editor retries it."""
    deadline = time.monotonic() + timeout
    while True:
        try:
            return client.send_request(
                "textDocument/semanticTokens/full",
                {"textDocument": {"uri": uri}},
                timeout=timeout,
            )
        except RuntimeError as error:
            if "-32801" not in str(error) or time.monotonic() > deadline:
                raise


def publish_count(client: lsp_client.LspClient, uri: str) -> int:
    with client._lock:
        return sum(
            1
            for n in client._notifications
            if n.get("method") == "textDocument/publishDiagnostics"
            and n.get("params", {}).get("uri") == uri
        )


def open_text(
    client: lsp_client.LspClient, path: Path, language_id: str
) -> tuple[str, str]:
    text = path.read_text(encoding="utf-8", errors="replace")
    uri = lsp_client.document_uri(str(path))
    client.send_notification(
        "textDocument/didOpen",
        {
            "textDocument": {
                "uri": uri,
                "languageId": language_id,
                "version": 1,
                "text": text,
            }
        },
    )
    return uri, text


def run_dialect(
    server: Path,
    corpus: Path,
    dialect: str,
    files: list[Path],
    edits: int,
    diag_timeout: float,
    scan_timeout: float,
    request_timeout: float,
    verbose: bool,
    wait_scan: bool,
) -> dict[str, Any]:
    client = DialectClient(str(corpus), [str(server)], dialect)
    t0 = time.perf_counter()
    client.start()
    assert client.process is not None
    pid = client.process.pid
    lsp_client.initialize(client)
    init_ms = ms_since(t0)
    rss_after_init = rss_kib(pid)
    scan_ms = float("nan")
    if wait_scan:
        try:
            client.wait_for_workspace_scan(timeout=scan_timeout)
            scan_ms = ms_since(t0)
        except TimeoutError:
            pass
    rss_after_scan = rss_kib(pid)

    tokens_ms: list[float] = []
    diag_ms: list[float] = []
    token_counts = 0
    first_tokens_ms: float | None = None
    opened: list[tuple[Path, str, str]] = []
    timeouts: list[str] = []
    per_file: list[tuple[str, float]] = []
    for path in files:
        start = time.perf_counter()
        before = publish_count(client, lsp_client.document_uri(str(path)))
        uri, text = open_text(client, path, "tcl")
        try:
            result = semantic_tokens(client, uri, request_timeout)
        except TimeoutError:
            timeouts.append(str(path.relative_to(corpus)))
            print(
                f"  {dialect} TIMEOUT {path.relative_to(corpus)}",
                file=sys.stderr,
                flush=True,
            )
            break
        tokens_ms.append(ms_since(start))
        per_file.append((str(path.relative_to(corpus)), round(tokens_ms[-1], 1)))
        if verbose:
            print(
                f"  {dialect} {tokens_ms[-1]:9.1f} ms {path.relative_to(corpus)}",
                file=sys.stderr,
                flush=True,
            )
        if first_tokens_ms is None:
            first_tokens_ms = ms_since(t0)
        token_counts += len((result or {}).get("data", [])) // 5
        if wait_for_publish(client, uri, before, diag_timeout):
            diag_ms.append(ms_since(start))
        opened.append((path, uri, text))
    rss_after_open = rss_kib(pid)

    edit_ms: list[float] = []
    if edits and opened and not timeouts:
        path, uri, text = max(opened, key=lambda row: len(row[2]))
        lines = text.split("\n")
        line = len(lines) // 2
        for version in range(2, edits + 2):
            start = time.perf_counter()
            col = len(lines[line])
            client.send_notification(
                "textDocument/didChange",
                {
                    "textDocument": {"uri": uri, "version": version},
                    "contentChanges": [
                        {
                            "range": {
                                "start": {"line": line, "character": col},
                                "end": {"line": line, "character": col},
                            },
                            "text": " ",
                        }
                    ],
                },
            )
            lines[line] += " "
            semantic_tokens(client, uri, request_timeout)
            edit_ms.append(ms_since(start))
        edit_file = str(path.relative_to(corpus))
    else:
        edit_file = None

    rss_final = rss_kib(pid)
    client.shutdown()
    return {
        "dialect": dialect,
        "files": len(files),
        "opened": len(opened),
        "lines": sum(t.count("\n") + 1 for _, _, t in opened),
        "tokens": token_counts,
        "init_ms": round(init_ms, 1),
        "scan_done_ms": round(scan_ms, 1),
        "first_tokens_ms": round(first_tokens_ms or 0.0, 1),
        "open_tokens": summary(tokens_ms),
        "open_diagnostics": summary(diag_ms),
        "diagnostics_missing": len(opened) - len(diag_ms),
        "timeouts": timeouts,
        "slowest": sorted(per_file, key=lambda row: -row[1])[:5],
        "edit_file": edit_file,
        "edit_tokens": summary(edit_ms),
        "rss_kib": {
            "init": rss_after_init.get("VmRSS"),
            "scan": rss_after_scan.get("VmRSS"),
            "open": rss_after_open.get("VmRSS"),
            "final": rss_final.get("VmRSS"),
            "peak": rss_final.get("VmHWM"),
        },
    }


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(description=(__doc__ or "").splitlines()[0])
    ap.add_argument("--server", required=True, type=Path)
    ap.add_argument("--corpus", required=True, type=Path)
    ap.add_argument("--dialect", action="append", required=True)
    ap.add_argument(
        "--pattern", action="append", help="glob; defaults to Tcl/Jim sources"
    )
    ap.add_argument("--limit", type=int, help="open at most this many files")
    ap.add_argument("--edits", type=int, default=50)
    ap.add_argument("--repeat", type=int, default=1)
    ap.add_argument("--diag-timeout", type=float, default=15.0)
    ap.add_argument("--scan-timeout", type=float, default=300.0)
    ap.add_argument("--request-timeout", type=float, default=120.0)
    ap.add_argument("--verbose", action="store_true")
    ap.add_argument(
        "--exclude", action="append", help="glob relative to the corpus root"
    )
    ap.add_argument(
        "--wait-scan",
        action="store_true",
        help="wait for the workspace scan before opening; default opens at once, as an editor does",
    )
    ap.add_argument("--out", type=Path)
    args = ap.parse_args(argv)

    corpus = args.corpus.resolve()
    files = corpus_files(
        corpus,
        tuple(args.pattern or DEFAULT_PATTERNS),
        args.limit,
        tuple(args.exclude or ()),
    )
    if not files:
        print(f"no files under {corpus}", file=sys.stderr)
        return 2
    rows = []
    for _ in range(args.repeat):
        for dialect in args.dialect:
            row = run_dialect(
                args.server.resolve(),
                corpus,
                dialect,
                files,
                args.edits,
                args.diag_timeout,
                args.scan_timeout,
                args.request_timeout,
                args.verbose,
                args.wait_scan,
            )
            row["corpus"] = corpus.name
            rows.append(row)
            print(json.dumps(row), flush=True)
    if args.out:
        args.out.write_text(json.dumps(rows, indent=2) + "\n", encoding="utf-8")
    return 0


if __name__ == "__main__":
    os.environ.setdefault("RUST_LOG", "warn")
    sys.exit(main())
