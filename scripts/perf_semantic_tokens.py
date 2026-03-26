#!/usr/bin/env python3
"""Benchmark time-to-semantic-tokens for the Tcl LSP server.

Measures the wall-clock time from sending ``textDocument/didOpen`` to
receiving the ``textDocument/semanticTokens/full`` response, with
optional simulated network latency injected around each JSON-RPC
message.

Usage::

    # Basic benchmark of a single file
    python3 scripts/perf_semantic_tokens.py samples/for_screenshots/16-references-long.tcl

    # With 10ms simulated one-way latency
    python3 scripts/perf_semantic_tokens.py --latency-ms 10 path/to/file.tcl

    # Multiple files, 3 iterations each, JSON output
    python3 scripts/perf_semantic_tokens.py --iterations 3 --json file1.tcl file2.tcl

    # Batch: all .tcl files in a directory
    python3 scripts/perf_semantic_tokens.py --json path/to/dir/

Latency simulation wraps the stdio transport with configurable delay
on every send and receive, modelling a remote LSP server or
vscode-server over SSH.  This is deterministic and does not require
root privileges (unlike ``tc netem``).
"""

from __future__ import annotations

import argparse
import json
import os
import re
import subprocess
import sys
import threading
import time
from pathlib import Path
from typing import Any

# JSON-RPC transport with optional latency injection


class LspBenchClient:
    """Minimal LSP client with latency-injected stdio transport."""

    def __init__(self, server_dir: str, *, latency_s: float = 0.0) -> None:
        self.server_dir = server_dir
        self.latency_s = latency_s
        self.process: subprocess.Popen | None = None
        self._request_id = 0
        self._pending: dict[int, dict] = {}
        self._notifications: list[dict] = []
        self._lock = threading.Lock()
        self._reader_thread: threading.Thread | None = None
        self._running = False
        self._stderr_lines: list[str] = []
        self._stderr_thread: threading.Thread | None = None

    def start(self) -> None:
        self.process = subprocess.Popen(
            [
                "uv",
                "run",
                "--directory",
                self.server_dir,
                "--no-dev",
                "python",
                "-m",
                "lsp",
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
        if self.latency_s > 0:
            time.sleep(self.latency_s)
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
        if self.latency_s > 0:
            time.sleep(self.latency_s)
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

    def send_request(self, method: str, params: dict, timeout: float = 60.0) -> Any:
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

    def get_timing_lines(self) -> list[str]:
        """Extract ``[timing]`` lines from ``window/logMessage`` notifications."""
        with self._lock:
            lines: list[str] = []
            for n in self._notifications:
                if n.get("method") == "window/logMessage":
                    msg = n.get("params", {}).get("message", "")
                    if "[timing]" in msg:
                        lines.append(msg)
            return lines

    def get_stderr_timing_lines(self) -> list[str]:
        with self._lock:
            return [ln for ln in self._stderr_lines if "[timing]" in ln]

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


# Benchmark helpers


def find_server_dir() -> str:
    cwd = Path.cwd()
    if (cwd / "lsp" / "__main__.py").exists():
        return str(cwd)
    script_dir = Path(__file__).resolve().parent.parent
    if (script_dir / "lsp" / "__main__.py").exists():
        return str(script_dir)
    raise FileNotFoundError("Cannot find tcl-lsp server directory")


def _initialize(client: LspBenchClient) -> None:
    client.send_request(
        "initialize",
        {
            "processId": os.getpid(),
            "rootUri": f"file://{client.server_dir}",
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
    # Give the server a moment to finish initialisation.
    time.sleep(0.3)


_TIMING_RE = re.compile(r"\[timing\]\s+(\S+)\s+([\d.]+)ms")


def _extract_server_timings(lines: list[str]) -> dict[str, float]:
    """Parse ``[timing] label Xms`` entries from server stderr."""
    timings: dict[str, float] = {}
    for line in lines:
        m = _TIMING_RE.search(line)
        if m:
            label = m.group(1)
            ms = float(m.group(2))
            # Use last occurrence per label (most recent operation).
            timings[label] = ms
    return timings


def benchmark_file(
    server_dir: str,
    file_path: str,
    *,
    latency_ms: float = 0.0,
    iterations: int = 1,
) -> dict:
    """Benchmark time-to-semantic-tokens for a single file.

    Returns a dict with timing stats and server-side breakdown.
    """
    abs_path = os.path.abspath(file_path)
    if not os.path.isfile(abs_path):
        raise FileNotFoundError(f"File not found: {abs_path}")
    with open(abs_path, encoding="utf-8", errors="replace") as f:
        content = f.read()

    n_lines = content.count("\n") + 1
    uri = f"file://{abs_path}"
    latency_s = latency_ms / 1000.0

    iteration_results: list[dict] = []

    for i in range(iterations):
        client = LspBenchClient(server_dir, latency_s=latency_s)
        client.start()
        try:
            _initialize(client)

            # Clear any startup stderr
            client.get_all_stderr()

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

            # Small delay to let the server start processing before we
            # send the semantic tokens request — mimics real client
            # behaviour where the request is queued after didOpen.
            if latency_s > 0:
                # With latency, the request naturally arrives later.
                pass
            else:
                time.sleep(0.01)

            t_req = time.perf_counter()
            result = client.send_request(
                "textDocument/semanticTokens/full",
                {"textDocument": {"uri": uri}},
            )
            t_done = time.perf_counter()

            data = result.get("data", []) if result else []
            n_tokens = len(data) // 5

            # Wait briefly for server-side log notifications to arrive.
            time.sleep(0.2)

            # Collect timing from both window/logMessage and stderr.
            timing_lines = client.get_timing_lines()
            timing_lines.extend(client.get_stderr_timing_lines())
            server_timings = _extract_server_timings(timing_lines)

            iteration_results.append(
                {
                    "open_to_tokens_ms": round((t_done - t_open) * 1000, 1),
                    "request_to_response_ms": round((t_done - t_req) * 1000, 1),
                    "tokens": n_tokens,
                    "server_timings": server_timings,
                    "timing_lines": timing_lines,
                }
            )
        finally:
            client.shutdown()

    # Aggregate across iterations.
    open_times = [r["open_to_tokens_ms"] for r in iteration_results]
    req_times = [r["request_to_response_ms"] for r in iteration_results]

    return {
        "file": file_path,
        "lines": n_lines,
        "latency_ms": latency_ms,
        "iterations": iterations,
        "tokens": iteration_results[-1]["tokens"],
        "open_to_tokens_ms": {
            "min": min(open_times),
            "max": max(open_times),
            "median": sorted(open_times)[len(open_times) // 2],
        },
        "request_to_response_ms": {
            "min": min(req_times),
            "max": max(req_times),
            "median": sorted(req_times)[len(req_times) // 2],
        },
        "server_timings": iteration_results[-1]["server_timings"],
        "timing_log": iteration_results[-1]["timing_lines"],
    }


def collect_tcl_files(path: str, max_files: int = 50) -> list[str]:
    """Collect .tcl and .irul files from a directory."""
    p = Path(path)
    if p.is_file():
        return [str(p)]
    files: list[str] = []
    for ext in ("*.tcl", "*.irul"):
        files.extend(str(f) for f in sorted(p.rglob(ext)))
    # Sort by size descending — benchmark the largest first.
    files.sort(key=lambda f: os.path.getsize(f), reverse=True)
    return files[:max_files]


def _print_human(result: dict) -> None:
    """Print a single benchmark result in human-readable form."""
    print(f"\n{'=' * 60}")
    print(f"  File: {result['file']}")
    print(f"  Lines: {result['lines']}")
    print(f"  Tokens: {result['tokens']}")
    print(f"  Latency: {result['latency_ms']}ms one-way")
    print(f"  Iterations: {result['iterations']}")
    print(f"{'=' * 60}")

    ott = result["open_to_tokens_ms"]
    rtr = result["request_to_response_ms"]
    print("\n  Open-to-tokens (wall clock):")
    print(f"    min={ott['min']:.1f}ms  median={ott['median']:.1f}ms  max={ott['max']:.1f}ms")
    print("  Request-to-response:")
    print(f"    min={rtr['min']:.1f}ms  median={rtr['median']:.1f}ms  max={rtr['max']:.1f}ms")

    st = result.get("server_timings", {})
    if st:
        print("\n  Server-side timings (last iteration):")
        for label, ms in sorted(st.items()):
            print(f"    {label}: {ms:.0f}ms")

    tl = result.get("timing_log", [])
    if tl:
        print(f"\n  Timing trace log ({len(tl)} entries):")
        for line in tl:
            print(f"    {line.strip()}")
    print()


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Benchmark time-to-semantic-tokens for the Tcl LSP server.",
    )
    parser.add_argument(
        "files",
        nargs="+",
        help="Tcl files or directories to benchmark",
    )
    parser.add_argument(
        "--latency-ms",
        type=float,
        default=0.0,
        help="Simulated one-way network latency in milliseconds (default: 0)",
    )
    parser.add_argument(
        "--iterations",
        type=int,
        default=1,
        help="Number of iterations per file (default: 1)",
    )
    parser.add_argument(
        "--json",
        action="store_true",
        help="Output results as JSON",
    )
    parser.add_argument(
        "--server-dir",
        default=None,
        help="Path to tcl-lsp server directory (auto-detected if omitted)",
    )
    args = parser.parse_args()

    server_dir = args.server_dir or find_server_dir()

    # Collect all files to benchmark.
    all_files: list[str] = []
    for path in args.files:
        all_files.extend(collect_tcl_files(path))

    if not all_files:
        print("No .tcl or .irul files found.", file=sys.stderr)
        sys.exit(1)

    results: list[dict] = []
    for fpath in all_files:
        if not args.json:
            print(f"Benchmarking: {fpath} ...", file=sys.stderr)
        try:
            result = benchmark_file(
                server_dir,
                fpath,
                latency_ms=args.latency_ms,
                iterations=args.iterations,
            )
            results.append(result)
            if not args.json:
                _print_human(result)
        except Exception as e:
            print(f"ERROR benchmarking {fpath}: {e}", file=sys.stderr)
            results.append({"file": fpath, "error": str(e)})

    if args.json:
        # Strip timing_log from JSON output (too verbose).
        for r in results:
            r.pop("timing_log", None)
        print(json.dumps(results, indent=2))


if __name__ == "__main__":
    main()
