#!/usr/bin/env python3
"""Performance stats for SpiceGenTcl across all tagged versions and main.

Clones SpiceGenTcl, iterates every git tag plus ``main``, starts the
LSP server with the project as workspace root (dialect ``tcl9.0``),
waits for the background scan, then collects per-file semantic-token
timing and all diagnostics/optimisation findings.  Results are stored
in ``spicegentcl_perf.sqlite3``.

Usage::

    # Benchmark all tags + main
    python3 scripts/perf_spicegentcl.py bench

    # Benchmark a single tag
    python3 scripts/perf_spicegentcl.py bench --tag 0.71

    # Benchmark main only
    python3 scripts/perf_spicegentcl.py bench --tag main

    # Print summary table
    python3 scripts/perf_spicegentcl.py list
"""

from __future__ import annotations

import argparse
import json
import os
import re
import sqlite3
import subprocess
import threading
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_DIR = SCRIPT_DIR.parent
DB_PATH = PROJECT_DIR / "spicegentcl_perf.sqlite3"
CLONE_DIR = PROJECT_DIR / "tmp" / "SpiceGenTcl"
REPO_URL = "https://github.com/georgtree/SpiceGenTcl.git"

LSP_SEVERITY = {1: "ERROR", 2: "WARNING", 3: "INFO", 4: "HINT"}

# ---------------------------------------------------------------------------
# SQLite helpers
# ---------------------------------------------------------------------------


def _init_db(db: sqlite3.Connection) -> None:
    db.executescript("""
        CREATE TABLE IF NOT EXISTS scan_runs (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            tag                 TEXT NOT NULL,
            git_sha             TEXT,
            timestamp           TEXT NOT NULL,
            file_count          INTEGER,
            scan_ms             REAL,
            total_warnings      INTEGER DEFAULT 0,
            total_diagnostics   INTEGER DEFAULT 0,
            total_optimisations INTEGER DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS file_stats (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            run_id              INTEGER NOT NULL REFERENCES scan_runs(id),
            file_path           TEXT NOT NULL,
            lines               INTEGER,
            tokens              INTEGER,
            open_to_tokens_ms   REAL,
            update_ms           REAL,
            semantic_tokens_ms  REAL,
            diagnostics         INTEGER DEFAULT 0,
            optimisations       INTEGER DEFAULT 0
        );
        CREATE TABLE IF NOT EXISTS findings (
            id                  INTEGER PRIMARY KEY AUTOINCREMENT,
            run_id              INTEGER NOT NULL REFERENCES scan_runs(id),
            file_path           TEXT NOT NULL,
            line                INTEGER,
            character           INTEGER,
            end_line            INTEGER,
            end_character       INTEGER,
            severity            INTEGER,
            severity_label      TEXT,
            code                TEXT,
            message             TEXT,
            source              TEXT,
            category            TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_file_stats_run ON file_stats(run_id);
        CREATE INDEX IF NOT EXISTS idx_findings_run ON findings(run_id);
        CREATE INDEX IF NOT EXISTS idx_findings_code ON findings(code);
        CREATE INDEX IF NOT EXISTS idx_scan_runs_tag ON scan_runs(tag);
    """)


def get_db() -> sqlite3.Connection:
    db = sqlite3.connect(str(DB_PATH))
    db.row_factory = sqlite3.Row
    _init_db(db)
    return db


# ---------------------------------------------------------------------------
# LSP client (mirrors scripts/perf_semantic_tokens.py)
# ---------------------------------------------------------------------------


class LspClient:
    """Minimal LSP client over stdio for benchmarking."""

    def __init__(self, server_dir: str) -> None:
        self.server_dir = server_dir
        self.process: subprocess.Popen | None = None
        self._request_id = 0
        self._pending: dict[int, dict] = {}
        self._notifications: list[dict] = []
        self._lock = threading.Lock()
        self._reader_thread: threading.Thread | None = None
        self._stderr_lines: list[str] = []
        self._stderr_thread: threading.Thread | None = None
        self._running = False

    def start(self) -> None:
        self.process = subprocess.Popen(
            [
                "uv", "run", "--directory", self.server_dir,
                "--no-dev", "python", "-m", "lsp",
            ],
            stdin=subprocess.PIPE,
            stdout=subprocess.PIPE,
            stderr=subprocess.PIPE,
            cwd=self.server_dir,
        )
        self._running = True
        self._reader_thread = threading.Thread(target=self._reader_loop, daemon=True)
        self._reader_thread.start()
        self._stderr_thread = threading.Thread(target=self._stderr_loop, daemon=True)
        self._stderr_thread.start()

    def _stderr_loop(self) -> None:
        assert self.process and self.process.stderr
        for raw in self.process.stderr:
            try:
                line = raw.decode("utf-8", errors="replace").rstrip("\n\r")
                with self._lock:
                    self._stderr_lines.append(line)
            except Exception:
                break

    def _send(self, data: dict) -> None:
        body = json.dumps(data).encode("utf-8")
        header = f"Content-Length: {len(body)}\r\n\r\n".encode("utf-8")
        assert self.process and self.process.stdin
        self.process.stdin.write(header + body)
        self.process.stdin.flush()

    def _read_message(self) -> dict | None:
        assert self.process and self.process.stdout
        stdout = self.process.stdout
        content_length = 0
        while True:
            line = stdout.readline()
            if not line:
                return None
            line_str = line.decode("utf-8").rstrip("\r\n")
            if not line_str:
                break
            if line_str.lower().startswith("content-length:"):
                content_length = int(line_str.split(":", 1)[1].strip())
        if content_length == 0:
            return None
        body = stdout.read(content_length)
        if not body:
            return None
        return json.loads(body.decode("utf-8"))

    def _reader_loop(self) -> None:
        while self._running:
            try:
                msg = self._read_message()
            except Exception:
                break
            if msg is None:
                break
            if "id" in msg and "method" not in msg:
                rid = msg["id"]
                with self._lock:
                    if rid in self._pending:
                        entry = self._pending[rid]
                        entry["result"] = msg
                        entry["event"].set()
            elif "method" in msg and "id" not in msg:
                with self._lock:
                    self._notifications.append(msg)

    def send_request(self, method: str, params: dict, timeout: float = 120.0) -> Any:
        self._request_id += 1
        rid = self._request_id
        event = threading.Event()
        with self._lock:
            self._pending[rid] = {"event": event, "result": None}
        self._send({"jsonrpc": "2.0", "id": rid, "method": method, "params": params})
        if not event.wait(timeout):
            raise TimeoutError(f"Timeout waiting for {method} (id={rid})")
        with self._lock:
            entry = self._pending.pop(rid)
        result = entry["result"]
        if "error" in result:
            err = result["error"]
            raise RuntimeError(f"LSP error {err.get('code')}: {err.get('message')}")
        return result.get("result")

    def send_notification(self, method: str, params: dict | None = None) -> None:
        msg: dict = {"jsonrpc": "2.0", "method": method}
        if params is not None:
            msg["params"] = params
        self._send(msg)

    def get_notifications(self, method: str) -> list[dict]:
        with self._lock:
            return [n for n in self._notifications if n.get("method") == method]

    def clear_notifications(self) -> None:
        with self._lock:
            self._notifications.clear()

    def get_all_stderr(self) -> list[str]:
        with self._lock:
            return list(self._stderr_lines)

    def shutdown(self) -> None:
        self._running = False
        try:
            if self.process and self.process.poll() is None:
                try:
                    self.send_request("shutdown", {}, timeout=5.0)
                except Exception:
                    pass
                try:
                    self.send_notification("exit")
                except Exception:
                    pass
                try:
                    self.process.wait(timeout=3.0)
                except subprocess.TimeoutExpired:
                    self.process.kill()
                    self.process.wait(timeout=2.0)
        except Exception:
            if self.process:
                try:
                    self.process.kill()
                except Exception:
                    pass


# ---------------------------------------------------------------------------
# LSP helpers
# ---------------------------------------------------------------------------

_TIMING_RE = re.compile(r"\[timing\]\s+(\S+)\s+([\d.]+)ms")
_SCAN_RE = re.compile(r"background.scan\s+([\d.]+)ms")


def _initialize(client: LspClient, workspace_root: str) -> None:
    """Send initialize/initialized with *workspace_root* as rootUri."""
    root_uri = f"file://{workspace_root}"
    client.send_request(
        "initialize",
        {
            "processId": os.getpid(),
            "rootUri": root_uri,
            "workspaceFolders": [{"uri": root_uri, "name": Path(workspace_root).name}],
            "capabilities": {
                "textDocument": {
                    "semanticTokens": {
                        "dynamicRegistration": False,
                        "requests": {"full": True},
                        "tokenTypes": [],
                        "tokenModifiers": [],
                        "formats": ["relative"],
                    },
                    "publishDiagnostics": {"relatedInformation": False},
                },
            },
        },
    )
    client.send_notification("initialized", {})


def _set_dialect(client: LspClient, dialect: str = "tcl9.0") -> None:
    client.send_notification(
        "workspace/didChangeConfiguration",
        {"settings": {"tclLsp": {"dialect": dialect}}},
    )


def _get_timing_lines(client: LspClient) -> list[str]:
    """Collect ``[timing]`` lines from both window/logMessage and stderr."""
    lines: list[str] = []
    for n in client.get_notifications("window/logMessage"):
        msg = n.get("params", {}).get("message", "")
        if "[timing]" in msg:
            lines.append(msg)
    for line in client.get_all_stderr():
        if "[timing]" in line:
            lines.append(line)
    return lines


def _wait_for_scan(client: LspClient, timeout: float = 180.0) -> float:
    """Block until background scan completes.  Returns scan time in ms."""
    deadline = time.monotonic() + timeout
    while time.monotonic() < deadline:
        for line in _get_timing_lines(client):
            if "background scan" in line or "background_scan" in line:
                m = _SCAN_RE.search(line)
                return float(m.group(1)) if m else 0.0
        time.sleep(0.5)
    raise TimeoutError("Background scan did not complete within timeout")


def _extract_server_timings(client: LspClient) -> dict[str, float]:
    """Parse ``[timing]`` entries from notifications and stderr."""
    timings: dict[str, float] = {}
    for line in _get_timing_lines(client):
        m = _TIMING_RE.search(line)
        if m:
            timings[m.group(1)] = float(m.group(2))
    return timings


def _collect_diagnostics_for_uri(
    client: LspClient,
    uri: str,
    wait: float = 5.0,
) -> list[dict]:
    """Wait for publishDiagnostics notifications matching *uri*."""
    deadline = time.monotonic() + wait
    best: list[dict] = []
    while time.monotonic() < deadline:
        for n in client.get_notifications("textDocument/publishDiagnostics"):
            params = n.get("params", {})
            if params.get("uri") == uri:
                diags = params.get("diagnostics", [])
                if len(diags) >= len(best):
                    best = diags
        if best:
            # Give a little extra time for deep diagnostics to merge in.
            time.sleep(min(1.0, deadline - time.monotonic()))
            # Refresh after the extra wait.
            for n in client.get_notifications("textDocument/publishDiagnostics"):
                params = n.get("params", {})
                if params.get("uri") == uri:
                    diags = params.get("diagnostics", [])
                    if len(diags) >= len(best):
                        best = diags
            break
        time.sleep(0.3)
    return best


def _classify_code(code: str | None) -> str:
    """Classify a diagnostic code as 'optimisation', 'warning', or 'diagnostic'."""
    if not code:
        return "diagnostic"
    code_str = str(code)
    if re.match(r"O\d+", code_str):
        return "optimisation"
    if re.match(r"W\d+", code_str):
        return "warning"
    return "diagnostic"


# ---------------------------------------------------------------------------
# Git helpers
# ---------------------------------------------------------------------------


def _ensure_clone() -> str:
    """Clone SpiceGenTcl if needed, fetch latest.  Returns clone path."""
    clone = str(CLONE_DIR)
    if os.path.isdir(os.path.join(clone, ".git")):
        print(f"  Using cached clone at {clone}")
        subprocess.run(
            ["git", "fetch", "--all", "--tags"],
            cwd=clone, capture_output=True, timeout=60,
        )
    else:
        os.makedirs(str(CLONE_DIR.parent), exist_ok=True)
        print(f"  Cloning {REPO_URL} ...")
        subprocess.run(
            ["git", "clone", REPO_URL, clone],
            check=True, timeout=120,
        )
    return clone


def _get_tags(clone_dir: str) -> list[str]:
    result = subprocess.run(
        ["git", "tag", "-l"],
        cwd=clone_dir, capture_output=True, text=True, timeout=10,
    )
    return [t.strip() for t in result.stdout.splitlines() if t.strip()]


def _checkout(clone_dir: str, ref: str) -> str:
    """Checkout *ref* and return the SHA."""
    subprocess.run(
        ["git", "checkout", ref, "--force"],
        cwd=clone_dir, capture_output=True, check=True, timeout=30,
    )
    result = subprocess.run(
        ["git", "rev-parse", "--short", "HEAD"],
        cwd=clone_dir, capture_output=True, text=True, timeout=10,
    )
    return result.stdout.strip()


def _tag_sort_key(tag: str) -> tuple:
    """Sort tags numerically, ``main`` last."""
    if tag == "main":
        return (1, 0, 0)
    parts = tag.split(".")
    try:
        nums = tuple(int(p) for p in parts)
        return (0,) + nums
    except ValueError:
        return (0, 0, 0)


# ---------------------------------------------------------------------------
# File discovery
# ---------------------------------------------------------------------------


def _collect_tcl_files(directory: str) -> list[str]:
    """Collect all .tcl files under *directory*, sorted by size descending."""
    files: list[str] = []
    for f in Path(directory).rglob("*.tcl"):
        files.append(str(f))
    files.sort(key=lambda f: os.path.getsize(f), reverse=True)
    return files


# ---------------------------------------------------------------------------
# Benchmark one tag
# ---------------------------------------------------------------------------


def bench_tag(tag: str, clone_dir: str) -> dict | None:
    """Benchmark a single tag/branch.  Returns summary dict or None on failure."""
    server_dir = str(PROJECT_DIR)
    print(f"\n{'=' * 60}")
    print(f"  Tag: {tag}")
    print(f"{'=' * 60}")

    sha = _checkout(clone_dir, tag)
    print(f"  SHA: {sha}")

    tcl_files = _collect_tcl_files(clone_dir)
    if not tcl_files:
        print("  No .tcl files found — skipping")
        return None
    print(f"  Files: {len(tcl_files)}")

    client = LspClient(server_dir)
    client.start()
    try:
        _initialize(client, clone_dir)
        _set_dialect(client, "tcl9.0")

        # Wait for background scan.
        print("  Waiting for background scan...", end=" ", flush=True)
        scan_ms = _wait_for_scan(client)
        print(f"{scan_ms:.0f}ms")

        now = datetime.now(timezone.utc).isoformat()

        # Per-file semantic tokens + diagnostics.
        file_rows: list[dict] = []
        all_findings: list[dict] = []
        total_warnings = 0
        total_diagnostics = 0
        total_optimisations = 0

        for i, fpath in enumerate(tcl_files, 1):
            rel_path = os.path.relpath(fpath, clone_dir)
            with open(fpath, encoding="utf-8", errors="replace") as f:
                content = f.read()
            n_lines = content.count("\n") + 1
            uri = f"file://{os.path.abspath(fpath)}"

            # Clear stale notifications before opening a new file.
            client.clear_notifications()

            # Open file.
            t_open = time.perf_counter()
            client.send_notification(
                "textDocument/didOpen",
                {
                    "textDocument": {
                        "uri": uri,
                        "languageId": "tcl",
                        "version": 1,
                        "text": content,
                    },
                },
            )
            time.sleep(0.01)

            # Request semantic tokens.
            try:
                result = client.send_request(
                    "textDocument/semanticTokens/full",
                    {"textDocument": {"uri": uri}},
                    timeout=60.0,
                )
            except Exception as exc:
                print(f"    [{i}/{len(tcl_files)}] {rel_path} — FAILED ({exc})")
                # Close the file before continuing.
                client.send_notification(
                    "textDocument/didClose", {"textDocument": {"uri": uri}},
                )
                continue
            t_done = time.perf_counter()

            data = result.get("data", []) if result else []
            n_tokens = len(data) // 5
            ott_ms = round((t_done - t_open) * 1000, 1)

            # Extract server-side timings for this file from stderr.
            server_timings = _extract_server_timings(client)
            update_ms = server_timings.get("workspace_state.update", 0)
            sem_ms = server_timings.get(
                "semantic_tokens_full",
                server_timings.get("semanticTokens/full", 0),
            )

            # Collect diagnostics (wait for deep pass).
            diags = _collect_diagnostics_for_uri(client, uri, wait=5.0)

            file_diags = 0
            file_opts = 0
            for d in diags:
                code = str(d.get("code", ""))
                sev = d.get("severity", 0)
                sev_label = LSP_SEVERITY.get(sev, "UNKNOWN")
                cat = _classify_code(code)
                r = d.get("range", {})
                s = r.get("start", {})
                e = r.get("end", {})

                if cat == "optimisation":
                    file_opts += 1
                elif cat == "warning":
                    file_diags += 1
                else:
                    file_diags += 1

                all_findings.append({
                    "file_path": rel_path,
                    "line": s.get("line", 0),
                    "character": s.get("character", 0),
                    "end_line": e.get("line", 0),
                    "end_character": e.get("character", 0),
                    "severity": sev,
                    "severity_label": sev_label,
                    "code": code,
                    "message": d.get("message", ""),
                    "source": d.get("source", ""),
                    "category": cat,
                })

            total_warnings += sum(1 for f in all_findings[-len(diags):] if f["category"] == "warning")
            total_diagnostics += file_diags
            total_optimisations += file_opts

            file_rows.append({
                "file_path": rel_path,
                "lines": n_lines,
                "tokens": n_tokens,
                "open_to_tokens_ms": ott_ms,
                "update_ms": update_ms,
                "semantic_tokens_ms": sem_ms,
                "diagnostics": file_diags,
                "optimisations": file_opts,
            })

            print(
                f"    [{i}/{len(tcl_files)}] {rel_path}  "
                f"tok={n_tokens}  ott={ott_ms:.0f}ms  "
                f"diag={file_diags}  opt={file_opts}"
            )

            # Close the file to free server resources.
            client.send_notification(
                "textDocument/didClose", {"textDocument": {"uri": uri}},
            )

    finally:
        client.shutdown()

    # Recount totals from findings for accuracy.
    total_warnings = sum(1 for f in all_findings if f["category"] == "warning")
    total_diagnostics_count = sum(1 for f in all_findings if f["category"] == "diagnostic")
    total_optimisations = sum(1 for f in all_findings if f["category"] == "optimisation")

    return {
        "tag": tag,
        "git_sha": sha,
        "timestamp": now,
        "file_count": len(tcl_files),
        "scan_ms": scan_ms,
        "total_warnings": total_warnings,
        "total_diagnostics": total_diagnostics_count,
        "total_optimisations": total_optimisations,
        "file_rows": file_rows,
        "findings": all_findings,
    }


def store_result(result: dict) -> None:
    """Store a single tag's results in the database."""
    db = get_db()
    cursor = db.execute(
        """
        INSERT INTO scan_runs (tag, git_sha, timestamp, file_count, scan_ms,
                               total_warnings, total_diagnostics, total_optimisations)
        VALUES (:tag, :git_sha, :timestamp, :file_count, :scan_ms,
                :total_warnings, :total_diagnostics, :total_optimisations)
        """,
        result,
    )
    run_id = cursor.lastrowid

    for row in result["file_rows"]:
        db.execute(
            """
            INSERT INTO file_stats (run_id, file_path, lines, tokens,
                                    open_to_tokens_ms, update_ms, semantic_tokens_ms,
                                    diagnostics, optimisations)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                run_id, row["file_path"], row["lines"], row["tokens"],
                row["open_to_tokens_ms"], row["update_ms"], row["semantic_tokens_ms"],
                row["diagnostics"], row["optimisations"],
            ),
        )

    for f in result["findings"]:
        db.execute(
            """
            INSERT INTO findings (run_id, file_path, line, character,
                                  end_line, end_character, severity, severity_label,
                                  code, message, source, category)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            """,
            (
                run_id, f["file_path"], f["line"], f["character"],
                f["end_line"], f["end_character"], f["severity"], f["severity_label"],
                f["code"], f["message"], f["source"], f["category"],
            ),
        )

    db.commit()
    db.close()
    print(
        f"  Stored: {len(result['file_rows'])} files, "
        f"{len(result['findings'])} findings (run_id={run_id})"
    )


# ---------------------------------------------------------------------------
# List / summary
# ---------------------------------------------------------------------------


def list_results() -> None:
    """Print a summary of stored results."""
    db = get_db()
    runs = db.execute("""
        SELECT id, tag, git_sha, file_count, scan_ms,
               total_warnings, total_diagnostics, total_optimisations, timestamp
        FROM scan_runs ORDER BY timestamp
    """).fetchall()

    if not runs:
        print("No data in database.")
        db.close()
        return

    print(
        f"{'Tag':<10s} {'SHA':<10s} {'Files':>5s} {'Scan':>8s} "
        f"{'Warn':>5s} {'Diag':>5s} {'Opt':>5s} {'Timestamp'}"
    )
    print("-" * 85)
    for r in runs:
        print(
            f"{r['tag']:<10s} {r['git_sha'] or '':<10s} {r['file_count']:>5d} "
            f"{r['scan_ms']:>7.0f}ms {r['total_warnings']:>5d} "
            f"{r['total_diagnostics']:>5d} {r['total_optimisations']:>5d} "
            f"{r['timestamp'][:19]}"
        )

    # Per-file detail for most recent run.
    latest = runs[-1]
    print(f"\nLatest run: {latest['tag']} ({latest['git_sha']})")
    file_rows = db.execute("""
        SELECT file_path, lines, tokens, open_to_tokens_ms, diagnostics, optimisations
        FROM file_stats WHERE run_id = ? ORDER BY open_to_tokens_ms DESC
    """, (latest["id"],)).fetchall()

    if file_rows:
        print(
            f"  {'File':<50s} {'Lines':>6s} {'Tok':>6s} {'OTT':>8s} "
            f"{'Diag':>5s} {'Opt':>4s}"
        )
        print(f"  {'-' * 80}")
        for fr in file_rows:
            print(
                f"  {fr['file_path']:<50s} {fr['lines']:>6d} {fr['tokens']:>6d} "
                f"{fr['open_to_tokens_ms']:>7.0f}ms {fr['diagnostics']:>5d} "
                f"{fr['optimisations']:>4d}"
            )

    db.close()


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------


def main() -> None:
    parser = argparse.ArgumentParser(
        description="SpiceGenTcl performance stats for the Tcl LSP server",
    )
    sub = parser.add_subparsers(dest="command")

    bench_p = sub.add_parser("bench", help="Run benchmarks")
    bench_p.add_argument("--tag", help="Benchmark a single tag/branch (default: all)")

    sub.add_parser("list", help="List stored results")

    args = parser.parse_args()

    if args.command == "bench":
        clone_dir = _ensure_clone()

        if args.tag:
            tags = [args.tag]
        else:
            tags = sorted(_get_tags(clone_dir), key=_tag_sort_key)
            tags.append("main")

        print(f"Tags to benchmark: {', '.join(tags)}")

        for tag in tags:
            try:
                result = bench_tag(tag, clone_dir)
                if result:
                    store_result(result)
            except Exception as exc:
                print(f"  ERROR benchmarking {tag}: {exc}")

        print("\nDone.  Run 'list' to see results.")

    elif args.command == "list":
        list_results()

    else:
        parser.print_help()


if __name__ == "__main__":
    main()
