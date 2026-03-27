#!/usr/bin/env python3
"""Performance tracking: benchmark, store, and graph LSP server metrics.

Collects time-to-tokens, analysis time, memory, token count, diagnostic
count, and optimisation count across tagged versions and the current
branch.  Results are stored in a SQLite database for historical tracking
and rendered as publication-quality graphs.

Usage::

    # Benchmark the current branch and store results
    python3 scripts/perf_track.py bench

    # Benchmark a specific version
    python3 scripts/perf_track.py bench --version v1.2.4

    # Render graphs from stored data
    python3 scripts/perf_track.py graph

    # Benchmark + graph in one step
    python3 scripts/perf_track.py all

    # List stored results
    python3 scripts/perf_track.py list
"""

from __future__ import annotations

import argparse
import json
import os
import sqlite3
import subprocess
import sys
from datetime import datetime, timezone
from pathlib import Path

SCRIPT_DIR = Path(__file__).resolve().parent
PROJECT_DIR = SCRIPT_DIR.parent
DB_PATH = PROJECT_DIR / "perf_history.sqlite3"

# Sample files used for benchmarking (shipped with the repo).
BENCH_FILES = [
    ("irules_tcp (139 lines)", "samples/irules/cookbook_tcp_collect.irul"),
    ("long_code (539 lines)", "samples/tcl/09_long_code.tcl"),
    ("references (350 lines)", "samples/for_screenshots/16-references-long.tcl"),
]


def _init_db(db: sqlite3.Connection) -> None:
    db.executescript("""
        CREATE TABLE IF NOT EXISTS runs (
            id          INTEGER PRIMARY KEY AUTOINCREMENT,
            version     TEXT NOT NULL,
            git_sha     TEXT,
            timestamp   TEXT NOT NULL,
            file_label  TEXT NOT NULL,
            file_path   TEXT NOT NULL,
            lines       INTEGER,
            tokens      INTEGER,
            open_to_tokens_ms  REAL,
            request_to_response_ms REAL,
            update_ms   REAL,
            semantic_tokens_ms REAL,
            diagnostics INTEGER,
            optimisations INTEGER,
            memory_mb   REAL,
            raw_json    TEXT
        );
        CREATE INDEX IF NOT EXISTS idx_runs_version ON runs(version);
        CREATE INDEX IF NOT EXISTS idx_runs_file ON runs(file_label);
    """)


def get_db() -> sqlite3.Connection:
    db = sqlite3.connect(str(DB_PATH))
    db.row_factory = sqlite3.Row
    _init_db(db)
    return db


def _git_sha() -> str:
    try:
        return subprocess.check_output(
            ["git", "rev-parse", "--short", "HEAD"],
            cwd=str(PROJECT_DIR),
            text=True,
        ).strip()
    except Exception:
        return "unknown"


def _git_version_label() -> str:
    """Return the current version label (tag or branch name)."""
    try:
        desc = subprocess.check_output(
            ["git", "describe", "--tags", "--always", "--dirty=-dev"],
            cwd=str(PROJECT_DIR),
            text=True,
        ).strip()
        return desc
    except Exception:
        return "dev"


def _run_bench(server_dir: str, file_path: str, iterations: int = 2) -> dict | None:
    """Run perf_semantic_tokens.py against a file."""
    perf_script = os.path.join(server_dir, "scripts", "perf_semantic_tokens.py")
    if not os.path.exists(perf_script):
        return None
    abs_file = os.path.abspath(file_path)
    try:
        result = subprocess.run(
            [
                "uv",
                "run",
                "--directory",
                server_dir,
                "--no-dev",
                "python",
                perf_script,
                "--iterations",
                str(iterations),
                "--json",
                "--server-dir",
                server_dir,
                abs_file,
            ],
            capture_output=True,
            text=True,
            timeout=180,
            cwd=server_dir,
        )
        if result.returncode != 0:
            return None
        data = json.loads(result.stdout)
        if data and not data[0].get("error"):
            return data[0]
        return None
    except Exception:
        return None


def _count_diagnostics(server_dir: str, file_path: str) -> tuple[int, int]:
    """Count diagnostics and optimisations for a file.

    Returns (diagnostic_count, optimisation_count).
    """
    try:
        lsp_client = os.path.join(server_dir, ".claude", "skills", "lsp-client", "lsp_client.py")
        if not os.path.exists(lsp_client):
            return 0, 0
        result = subprocess.run(
            [
                "uv",
                "run",
                "--directory",
                server_dir,
                "--no-dev",
                "python",
                lsp_client,
                "diagnostics",
                os.path.abspath(file_path),
            ],
            capture_output=True,
            text=True,
            timeout=60,
            cwd=server_dir,
        )
        if result.returncode != 0:
            return 0, 0
        diag_count = 0
        opt_count = 0
        for line in result.stdout.splitlines():
            line = line.strip()
            if not line or line.startswith("===") or line.startswith("File:"):
                continue
            if line.startswith("INFO") and any(f"O{x}" in line for x in range(100, 200)):
                opt_count += 1
            elif any(line.startswith(sev) for sev in ("ERROR", "WARNING", "HINT", "INFO")):
                diag_count += 1
        return diag_count, opt_count
    except Exception:
        return 0, 0


def bench_version(
    version: str | None = None,
    iterations: int = 2,
) -> list[dict]:
    """Benchmark a version and return results."""
    server_dir = str(PROJECT_DIR)
    version_label = version or _git_version_label()
    sha = _git_sha()
    now = datetime.now(timezone.utc).isoformat()

    results = []
    for label, rel_path in BENCH_FILES:
        file_path = os.path.join(str(PROJECT_DIR), rel_path)
        if not os.path.exists(file_path):
            continue
        print(f"  {label}...", end=" ", flush=True)
        bench = _run_bench(server_dir, file_path, iterations)
        if bench is None:
            print("FAILED")
            continue

        ott = bench["open_to_tokens_ms"]["median"]
        rtr = bench["request_to_response_ms"]["median"]
        st = bench.get("server_timings", {})
        update_ms = st.get("workspace_state.update", 0)
        sem_ms = st.get("semantic_tokens_full", st.get("semanticTokens/full", 0))
        tokens = bench.get("tokens", 0)
        n_lines = bench.get("lines", 0)

        diags, opts = _count_diagnostics(server_dir, file_path)

        row = {
            "version": version_label,
            "git_sha": sha,
            "timestamp": now,
            "file_label": label,
            "file_path": rel_path,
            "lines": n_lines,
            "tokens": tokens,
            "open_to_tokens_ms": ott,
            "request_to_response_ms": rtr,
            "update_ms": update_ms,
            "semantic_tokens_ms": sem_ms,
            "diagnostics": diags,
            "optimisations": opts,
            "memory_mb": 0,
            "raw_json": json.dumps(bench),
        }
        results.append(row)
        print(f"open→tok={ott:.0f}ms  tokens={tokens}  diags={diags}  opts={opts}")

    return results


def store_results(results: list[dict]) -> None:
    """Store benchmark results in the SQLite database."""
    db = get_db()
    for row in results:
        db.execute(
            """
            INSERT INTO runs (version, git_sha, timestamp, file_label, file_path,
                              lines, tokens, open_to_tokens_ms, request_to_response_ms,
                              update_ms, semantic_tokens_ms, diagnostics, optimisations,
                              memory_mb, raw_json)
            VALUES (:version, :git_sha, :timestamp, :file_label, :file_path,
                    :lines, :tokens, :open_to_tokens_ms, :request_to_response_ms,
                    :update_ms, :semantic_tokens_ms, :diagnostics, :optimisations,
                    :memory_mb, :raw_json)
        """,
            row,
        )
    db.commit()
    db.close()
    print(f"  Stored {len(results)} results in {DB_PATH}")


def render_graphs(output_dir: str | None = None) -> list[str]:
    """Render performance trend graphs from the SQLite database.

    Returns the list of generated file paths.
    """
    try:
        import matplotlib

        matplotlib.use("Agg")
        import matplotlib.pyplot as plt
        import seaborn as sns
    except ImportError:
        print("ERROR: matplotlib and seaborn required. Install with:", file=sys.stderr)
        print("  pip install matplotlib seaborn", file=sys.stderr)
        return []

    db = get_db()
    rows = db.execute("""
        SELECT version, file_label, lines, tokens,
               open_to_tokens_ms, update_ms, semantic_tokens_ms,
               diagnostics, optimisations
        FROM runs
        ORDER BY timestamp
    """).fetchall()
    db.close()

    # Separate connection for memory table (may not exist in old databases).
    db2 = None
    try:
        db2 = sqlite3.connect(str(DB_PATH))
        db2.row_factory = sqlite3.Row
        db2.execute("SELECT 1 FROM memory_usage LIMIT 1")
    except Exception:
        db2 = None

    if not rows:
        print("No data in database. Run 'bench' first.")
        return []

    if output_dir is None:
        output_dir = str(PROJECT_DIR / "docs" / "perf")
    os.makedirs(output_dir, exist_ok=True)

    # Organise data — sort versions semantically: tagged releases by
    # version number, then non-tag entries (master, branch names) at end.
    import re as _re

    def _version_sort_key(v: str) -> tuple:
        m = _re.match(r"v?(\d+)\.(\d+)\.(\d+)", v)
        if m:
            return (0, int(m.group(1)), int(m.group(2)), int(m.group(3)), v)
        if v == "master":
            return (1, 0, 0, 0, v)
        return (2, 0, 0, 0, v)

    versions = sorted(set(r["version"] for r in rows), key=_version_sort_key)

    file_labels = sorted(set(r["file_label"] for r in rows))

    # Build data tables
    data_by_file: dict[str, dict[str, dict]] = {}
    for r in rows:
        fl = r["file_label"]
        v = r["version"]
        if fl not in data_by_file:
            data_by_file[fl] = {}
        data_by_file[fl][v] = dict(r)

    # Style
    sns.set_theme(style="whitegrid", context="paper", font_scale=1.1)
    palette = sns.color_palette("husl", len(file_labels))

    generated = []

    # Graph 1: Open-to-tokens by version (grouped bar chart)
    fig, ax = plt.subplots(figsize=(12, 6))
    x_positions = range(len(versions))
    bar_width = 0.8 / max(len(file_labels), 1)

    for fi, fl in enumerate(file_labels):
        values = []
        for v in versions:
            d = data_by_file.get(fl, {}).get(v)
            values.append(d["open_to_tokens_ms"] if d else 0)
        offset = (fi - len(file_labels) / 2 + 0.5) * bar_width
        bars = ax.bar(
            [x + offset for x in x_positions],
            values,
            bar_width,
            label=fl,
            color=palette[fi],
            edgecolor="white",
            linewidth=0.5,
        )
        for bar, val in zip(bars, values):
            if val > 0:
                ax.text(
                    bar.get_x() + bar.get_width() / 2,
                    bar.get_height() + 5,
                    f"{val:.0f}",
                    ha="center",
                    va="bottom",
                    fontsize=7,
                )

    ax.set_xlabel("Version")
    ax.set_ylabel("Open-to-Tokens (ms)")
    ax.set_title("Time to Semantic Tokens by Version", fontweight="bold", fontsize=14)
    ax.set_xticks(x_positions)
    ax.set_xticklabels(versions, rotation=30, ha="right")
    ax.legend(loc="upper right", fontsize=8)
    ax.set_ylim(bottom=0)
    fig.tight_layout()
    path1 = os.path.join(output_dir, "open_to_tokens.png")
    fig.savefig(path1, dpi=150)
    plt.close(fig)
    generated.append(path1)
    print(f"  Generated {path1}")

    # Graph 2: Pipeline breakdown (stacked bar — update vs semantic tokens)
    fig, ax = plt.subplots(figsize=(12, 6))
    for fi, fl in enumerate(file_labels):
        update_vals = []
        sem_vals = []
        for v in versions:
            d = data_by_file.get(fl, {}).get(v)
            update_vals.append(d["update_ms"] if d else 0)
            sem_vals.append(d["semantic_tokens_ms"] if d else 0)

        offset = (fi - len(file_labels) / 2 + 0.5) * bar_width
        ax.bar(
            [x + offset for x in x_positions],
            update_vals,
            bar_width,
            label=f"{fl} — analysis" if fi == 0 else "",
            color=palette[fi],
            alpha=0.7,
            edgecolor="white",
            linewidth=0.5,
        )
        ax.bar(
            [x + offset for x in x_positions],
            sem_vals,
            bar_width,
            bottom=update_vals,
            label=f"{fl} — sem tokens" if fi == 0 else "",
            color=palette[fi],
            alpha=0.4,
            edgecolor="white",
            linewidth=0.5,
            hatch="//",
        )

    ax.set_xlabel("Version")
    ax.set_ylabel("Time (ms)")
    ax.set_title(
        "Pipeline Breakdown: Analysis + Semantic Token Generation", fontweight="bold", fontsize=14
    )
    ax.set_xticks(x_positions)
    ax.set_xticklabels(versions, rotation=30, ha="right")
    ax.set_ylim(bottom=0)
    fig.tight_layout()
    path2 = os.path.join(output_dir, "pipeline_breakdown.png")
    fig.savefig(path2, dpi=150)
    plt.close(fig)
    generated.append(path2)
    print(f"  Generated {path2}")

    # Graph 3: Diagnostics + Optimisations count
    fig, (ax1, ax2) = plt.subplots(1, 2, figsize=(14, 5))

    for fi, fl in enumerate(file_labels):
        diag_vals = [
            data_by_file.get(fl, {}).get(v, {}).get("diagnostics", 0) or 0 for v in versions
        ]
        opt_vals = [
            data_by_file.get(fl, {}).get(v, {}).get("optimisations", 0) or 0 for v in versions
        ]
        ax1.plot(versions, diag_vals, marker="o", label=fl, color=palette[fi], linewidth=2)
        ax2.plot(versions, opt_vals, marker="s", label=fl, color=palette[fi], linewidth=2)

    ax1.set_title("Diagnostics Found", fontweight="bold")
    ax1.set_xlabel("Version")
    ax1.set_ylabel("Count")
    ax1.legend(fontsize=8)
    ax1.tick_params(axis="x", rotation=30)

    ax2.set_title("Optimisations Found", fontweight="bold")
    ax2.set_xlabel("Version")
    ax2.set_ylabel("Count")
    ax2.legend(fontsize=8)
    ax2.tick_params(axis="x", rotation=30)

    fig.suptitle("Analysis Coverage by Version", fontweight="bold", fontsize=14, y=1.02)
    fig.tight_layout()
    path3 = os.path.join(output_dir, "diagnostics_optimisations.png")
    fig.savefig(path3, dpi=150, bbox_inches="tight")
    plt.close(fig)
    generated.append(path3)
    print(f"  Generated {path3}")

    # Graph 4: Tokens per millisecond (throughput)
    fig, ax = plt.subplots(figsize=(10, 5))
    for fi, fl in enumerate(file_labels):
        throughput = []
        for v in versions:
            d = data_by_file.get(fl, {}).get(v)
            if d and d["open_to_tokens_ms"] and d["open_to_tokens_ms"] > 0:
                throughput.append(d["tokens"] / d["open_to_tokens_ms"] * 1000)
            else:
                throughput.append(0)
        ax.plot(versions, throughput, marker="D", label=fl, color=palette[fi], linewidth=2)

    ax.set_xlabel("Version")
    ax.set_ylabel("Tokens / second")
    ax.set_title("Semantic Token Throughput", fontweight="bold", fontsize=14)
    ax.legend(fontsize=8)
    ax.tick_params(axis="x", rotation=30)
    ax.set_ylim(bottom=0)
    fig.tight_layout()
    path4 = os.path.join(output_dir, "throughput.png")
    fig.savefig(path4, dpi=150)
    plt.close(fig)
    generated.append(path4)
    print(f"  Generated {path4}")

    # Graph 5: Peak memory usage
    mem_rows = (
        db2.execute("""
        SELECT version, peak_rss_mb FROM memory_usage
        WHERE peak_rss_mb > 0
        GROUP BY version
        ORDER BY version
    """).fetchall()
        if db2
        else []
    )
    if mem_rows:
        mem_versions = []
        mem_values = []
        for r in mem_rows:
            if r["version"] in versions:
                mem_versions.append(r["version"])
                mem_values.append(r["peak_rss_mb"])
        if mem_versions:
            # Sort by the same order as the main version list
            sorted_pairs = sorted(
                zip(mem_versions, mem_values),
                key=lambda p: _version_sort_key(p[0]),
            )
            mem_versions, mem_values = zip(*sorted_pairs) if sorted_pairs else ([], [])

            fig, ax = plt.subplots(figsize=(10, 5))
            bars = ax.bar(
                mem_versions,
                mem_values,
                color=sns.color_palette("husl", len(mem_versions)),
                edgecolor="white",
                linewidth=0.5,
            )
            for bar, val in zip(bars, mem_values):
                ax.text(
                    bar.get_x() + bar.get_width() / 2,
                    bar.get_height() + 0.5,
                    f"{val:.0f}",
                    ha="center",
                    va="bottom",
                    fontsize=9,
                )
            ax.set_xlabel("Version")
            ax.set_ylabel("Peak RSS (MB)")
            ax.set_title("Server Peak Memory Usage (idle)", fontweight="bold", fontsize=14)
            ax.set_ylim(bottom=0)
            ax.tick_params(axis="x", rotation=30)
            fig.tight_layout()
            path5 = os.path.join(output_dir, "memory.png")
            fig.savefig(path5, dpi=150)
            plt.close(fig)
            generated.append(path5)
            print(f"  Generated {path5}")

    if db2:
        db2.close()

    return generated


def list_results() -> None:
    """Print stored results."""
    db = get_db()
    rows = db.execute("""
        SELECT version, file_label, open_to_tokens_ms, update_ms,
               semantic_tokens_ms, tokens, diagnostics, optimisations, timestamp
        FROM runs ORDER BY timestamp, file_label
    """).fetchall()
    db.close()
    if not rows:
        print("No data in database.")
        return
    print(
        f"{'Version':<25s} {'File':<30s} {'OTT':>6s} {'Upd':>6s} {'Sem':>6s} {'Tok':>5s} {'Diag':>5s} {'Opt':>4s}"
    )
    print("-" * 110)
    for r in rows:
        print(
            f"{r['version']:<25s} {r['file_label']:<30s} "
            f"{r['open_to_tokens_ms']:>5.0f}ms {r['update_ms']:>5.0f}ms "
            f"{r['semantic_tokens_ms']:>5.0f}ms {r['tokens']:>5d} "
            f"{r['diagnostics']:>5d} {r['optimisations']:>4d}"
        )


def main() -> None:
    parser = argparse.ArgumentParser(description="Performance tracking for tcl-lsp")
    sub = parser.add_subparsers(dest="command")

    bench_p = sub.add_parser("bench", help="Run benchmarks and store results")
    bench_p.add_argument("--version", help="Override version label")
    bench_p.add_argument("--iterations", type=int, default=2)

    sub.add_parser("graph", help="Render performance graphs")
    sub.add_parser("list", help="List stored results")

    all_p = sub.add_parser("all", help="Benchmark current branch and render graphs")
    all_p.add_argument("--iterations", type=int, default=2)

    args = parser.parse_args()

    if args.command == "bench":
        print(f"Benchmarking {args.version or 'current'}...")
        results = bench_version(version=args.version, iterations=args.iterations)
        if results:
            store_results(results)

    elif args.command == "graph":
        print("Rendering graphs...")
        render_graphs()

    elif args.command == "list":
        list_results()

    elif args.command == "all":
        print("Benchmarking current branch...")
        results = bench_version(iterations=args.iterations)
        if results:
            store_results(results)
        print("\nRendering graphs...")
        render_graphs()

    else:
        parser.print_help()


if __name__ == "__main__":
    main()
