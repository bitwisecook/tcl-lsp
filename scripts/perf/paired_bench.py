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

"""Run same-host, same-corpus baseline/candidate Tcl LSP performance pairs.

This tool never edits its source checkouts. Outputs and Cargo targets must be
outside both worktrees. It produces raw bench.py JSON plus a matched-pair JSON
summary; it defines no performance threshold.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import platform
import re
import shutil
import signal
import statistics
import subprocess
import sys
import threading
import time
from pathlib import Path

CHECK_IDS = [
    "scan.workspace",
    "open.docs",
    "edit.storm",
    "edit.tokens",
    "nav.hover",
    "nav.definition",
    "nav.references",
    "nav.completion",
    "nav.symbols",
    "lens.resolve",
    "action.codeaction",
    "refactor.rename",
    "refactor.filerename",
    "nav.after_rename",
    "close.docs",
]
COUNTED_CHECKS = {
    "nav.hover",
    "nav.definition",
    "nav.references",
    "nav.completion",
    "nav.after_rename",
}
LOG_CAP = 20 * 1024 * 1024
DISK_FLOOR = 2 * 1024 * 1024 * 1024
ESCALATION = ((signal.SIGINT, 5), (signal.SIGTERM, 5), (signal.SIGKILL, 10))
PINNED_HARNESS_FILES = {
    "scripts/perf/bench.py": "4d6dfd09a5c884b4f670f643c9cbbdb91ca9a8ed0e497b74bca67277ff53b01f",
    "scripts/perf/fetch_corpus.py": "b932f568c1194ae3858d54481c7e9568ca6987484e33677f12f41290ca4ba140",
    ".claude/skills/lsp-client/lsp_client.py": "c10be772eca42e36970aa90dc86908fa4457967130080b897b15f045d18dccc0",
    "scripts/perf/MANIFEST.toml": "956a8a562484da5356bfe340ec82531f699a18c65a7ea4a9e4efbdd9540b1f58",
}
FAILURE_STATE = {"out": None, "meta": None, "phase": "startup"}


class Invalid(RuntimeError):
    pass


def sha(path: Path) -> str:
    h = hashlib.sha256()
    with path.open("rb") as f:
        for b in iter(lambda: f.read(1024 * 1024), b""):
            h.update(b)
    return h.hexdigest()


def dump(path: Path, obj) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(obj, indent=2, sort_keys=True) + "\n")


def tree_head(path: Path) -> str:
    r = subprocess.run(
        ["git", "rev-parse", "HEAD"],
        cwd=path,
        capture_output=True,
        text=True,
        check=True,
    )
    return r.stdout.strip()


def source_status(path: Path) -> str:
    return subprocess.run(
        ["git", "status", "--porcelain"],
        cwd=path,
        capture_output=True,
        text=True,
        check=True,
    ).stdout.strip()


def compiler_processes() -> list[str]:
    found = []
    for name in ("cargo", "rustc"):
        r = subprocess.run(
            ["pgrep", "-x", name], check=False, capture_output=True, text=True
        )
        if r.returncode == 0 and r.stdout.strip():
            found.append(name)
        elif r.returncode not in (0, 1):
            raise Invalid(f"pgrep -x {name} failed: {r.returncode}")
    return found


def require_group_closed(process: dict, label: str, meta: dict, out: Path) -> bool:
    if process.get("group_empty"):
        return True
    meta["invalid_reasons"].append(
        f"{label}: owned process group remains live; stopping before the next build or trial"
    )
    dump(out / "paired-summary.json", meta)
    return False


def meaningful_counts(
    baseline: dict, candidate: dict, check_id: str
) -> tuple[str, bool]:
    if check_id not in COUNTED_CHECKS:
        return "not-reported-by-harness-for-this-check", True
    b = [baseline.get("ops"), baseline.get("items")]
    c = [candidate.get("ops"), candidate.get("items")]
    if any(not isinstance(x, int) or isinstance(x, bool) or x < 0 for x in b + c):
        return "missing-or-invalid", False
    if b[0] == 0 or c[0] == 0:
        return "zero-requests", False
    if b != c:
        return f"changed:{b[0]}/{b[1]}->{c[0]}/{c[1]}", False
    return f"matched:{b[0]}/{b[1]}", True


def preserve_failure(exc: Exception) -> None:
    out = FAILURE_STATE.get("out")
    meta = FAILURE_STATE.get("meta")
    if not isinstance(out, Path):
        return
    if not isinstance(meta, dict):
        meta = {
            "schema": "paired-tcl-corpus-perf-v2",
            "runs": [],
            "builds": [],
            "invalid_reasons": [],
        }
    reason = f"exception during {FAILURE_STATE.get('phase', 'unknown')}: {type(exc).__name__}: {exc}"
    meta.setdefault("invalid_reasons", []).append(reason)
    meta["failed_phase"] = FAILURE_STATE.get("phase")
    try:
        dump(out / "paired-summary.json", meta)
        (out / "comparison.md").write_text(
            "# Paired Tcl corpus benchmark\n\nINCOMPLETE: " + reason + "\n"
        )
    except OSError:
        pass


def group_alive(pgid: int) -> bool:
    try:
        os.killpg(pgid, 0)
        return True
    except ProcessLookupError:
        return False
    except PermissionError:
        return True


def stop_group(proc: subprocess.Popen, pgid: int) -> list[str]:
    sent = []
    for sig, seconds in ESCALATION:
        if not group_alive(pgid):
            break
        try:
            os.killpg(pgid, sig)
            sent.append(sig.name)
        except ProcessLookupError:
            break
        until = time.monotonic() + seconds
        while time.monotonic() < until:
            if proc.poll() is None:
                proc.poll()
            if not group_alive(pgid):
                break
            time.sleep(0.1)
    if proc.poll() is None:
        try:
            proc.wait(timeout=2)
        except subprocess.TimeoutExpired:
            pass
    return sent


def run_bounded(
    argv: list[str],
    *,
    cwd: Path,
    env: dict[str, str],
    out: Path,
    stem: str,
    timeout_s: int,
    cap_bytes: int = LOG_CAP,
) -> dict:
    out.mkdir(parents=True, exist_ok=True)
    logs = {k: out / f"{stem}.{k}" for k in ("stdout", "stderr")}
    hashes = {k: hashlib.sha256() for k in logs}
    wrote = 0
    total = {"stdout": 0, "stderr": 0}
    overflow = threading.Event()
    lock = threading.Lock()
    t0 = time.time()
    start = time.monotonic()
    proc = subprocess.Popen(
        argv,
        cwd=cwd,
        env=env,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        start_new_session=True,
    )
    pgid = proc.pid

    def drain(name, pipe):
        nonlocal wrote
        with logs[name].open("wb") as dst:
            while True:
                b = pipe.read(65536)
                if not b:
                    break
                hashes[name].update(b)
                total[name] += len(b)
                with lock:
                    room = max(0, cap_bytes - wrote)
                    keep = b[:room]
                    if keep:
                        dst.write(keep)
                        wrote += len(keep)
                    if len(keep) < len(b):
                        overflow.set()

    threads = [
        threading.Thread(target=drain, args=(n, p), daemon=True)
        for n, p in [("stdout", proc.stdout), ("stderr", proc.stderr)]
    ]
    for t in threads:
        t.start()
    reason = None
    signals = []
    while proc.poll() is None:
        if overflow.is_set():
            reason = "combined_log_cap"
            signals = stop_group(proc, pgid)
            break
        if time.monotonic() - start >= timeout_s:
            reason = "deadline"
            signals = stop_group(proc, pgid)
            break
        time.sleep(0.05)
    rc = proc.wait()
    for t in threads:
        t.join(timeout=5)
    if any(t.is_alive() for t in threads):
        reason = reason or "pipe_drain_incomplete"
    if overflow.is_set():
        reason = reason or "combined_log_cap"
    group_closed = not group_alive(pgid)
    if not group_closed:
        signals += stop_group(proc, pgid)
        group_closed = not group_alive(pgid)
    result = {
        "argv": argv,
        "cwd": str(cwd),
        "started_utc": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime(t0)),
        "elapsed_s": round(time.monotonic() - start, 3),
        "exit_code": rc,
        "stop_reason": reason,
        "cleanup_signals": signals,
        "group_empty": group_closed,
        "cap_bytes": cap_bytes,
        "combined_bytes_seen": sum(total.values()),
        "combined_bytes_retained": wrote,
        "truncated": overflow.is_set(),
        "streams": {
            n: {
                "path": str(logs[n]),
                "bytes_seen": total[n],
                "bytes_retained": logs[n].stat().st_size,
                "sha256_all": hashes[n].hexdigest(),
                "sha256_retained": sha(logs[n]),
            }
            for n in logs
        },
    }
    dump(out / f"{stem}.process.json", result)
    return result


def disk_ok(path: Path) -> int:
    free = shutil.disk_usage(path).free
    if free < DISK_FLOOR:
        raise Invalid(f"disk floor: {free} bytes free, floor {DISK_FLOOR}")
    return free


def load_result(path: Path, scope: str) -> dict:
    d = json.loads(path.read_text())
    ids = [x.get("id") for x in d.get("checks", [])]
    if d.get("scope") != scope or ids != CHECK_IDS:
        raise Invalid(f"{path}: scope/check roster mismatch")
    if len({x["path"] for x in d.get("documents", [])}) != len(d.get("documents", [])):
        raise Invalid(f"{path}: duplicate document path")
    return d


def main() -> int:
    ap = argparse.ArgumentParser(description=__doc__)
    ap.add_argument("--baseline-root", type=Path, required=True)
    ap.add_argument("--candidate-root", type=Path, required=True)
    ap.add_argument("--harness-root", type=Path, required=True)
    ap.add_argument("--corpus", type=Path, required=True)
    ap.add_argument("--baseline-sha", required=True)
    ap.add_argument("--candidate-sha", required=True)
    ap.add_argument("--scope", choices=["small", "medium", "full"], required=True)
    ap.add_argument("--work", type=Path, required=True)
    ap.add_argument("--out", type=Path, required=True)
    ap.add_argument("--bench-deadline", type=int, default=1800)
    ap.add_argument("--build-deadline", type=int, default=2700)
    a = ap.parse_args()
    FAILURE_STATE["out"] = a.out.resolve()
    meta = {
        "schema": "paired-tcl-corpus-perf-v2",
        "baseline_sha": a.baseline_sha.lower(),
        "candidate_sha": a.candidate_sha.lower(),
        "scope": a.scope,
        "runs": [],
        "builds": [],
        "invalid_reasons": [],
    }
    FAILURE_STATE["meta"] = meta
    FAILURE_STATE["phase"] = "validate inputs"
    for s in (a.baseline_sha, a.candidate_sha):
        if not re.fullmatch(r"[0-9a-fA-F]{40}", s):
            raise Invalid("SHAs must be exactly 40 hex characters")
    if a.baseline_sha.lower() == a.candidate_sha.lower():
        raise Invalid("baseline and candidate SHA must differ")
    for root, want in (
        (a.baseline_root, a.baseline_sha),
        (a.candidate_root, a.candidate_sha),
    ):
        if tree_head(root) != want.lower():
            raise Invalid(f"{root}: checkout HEAD differs from requested SHA")
    harness = a.harness_root.resolve()
    base = a.baseline_root.resolve()
    cand = a.candidate_root.resolve()
    work = a.work.resolve()
    out = a.out.resolve()
    corpus = a.corpus.resolve()
    if any(
        p == r or r in p.parents or p in r.parents
        for p in (work, out)
        for r in (base, cand, harness)
    ):
        raise Invalid(
            "output/work directory must be disjoint from all source checkouts"
        )
    for root in (base, cand, harness):
        if source_status(root):
            raise Invalid(
                f"{root}: source/harness checkout is not clean before benchmark"
            )
    for rel, want_hash in PINNED_HARNESS_FILES.items():
        actual = sha(harness / rel)
        if actual != want_hash:
            raise Invalid(f"harness input hash mismatch: {rel} {actual}")
    for root in (base, cand):
        if sha(root / "scripts/perf/MANIFEST.toml") != sha(
            harness / "scripts/perf/MANIFEST.toml"
        ):
            raise Invalid(
                "corpus manifests differ from fixed harness; comparison refused"
            )
    if not corpus.is_dir():
        raise Invalid("verified corpus directory missing")
    if work == out or work in out.parents or out in work.parents:
        raise Invalid("scratch and output directories must be distinct")
    if (work.exists() and any(work.iterdir())) or (out.exists() and any(out.iterdir())):
        raise Invalid("scratch/output directories must be fresh and empty")
    work.mkdir(parents=True, exist_ok=True)
    out.mkdir(parents=True, exist_ok=True)
    disk_ok(work)
    for r in (base, cand):
        dirty = subprocess.run(
            ["git", "status", "--porcelain"],
            cwd=r,
            capture_output=True,
            text=True,
            check=True,
        ).stdout
        if dirty.strip():
            raise Invalid(f"{r}: checkout is not clean before build")
    meta.update(
        {
            "schema": "paired-tcl-corpus-perf-v2",
            "baseline_sha": a.baseline_sha.lower(),
            "candidate_sha": a.candidate_sha.lower(),
            "harness_sha": tree_head(harness),
            "scope": a.scope,
            "platform": platform.platform(),
            "machine": platform.machine(),
            "python": sys.version,
            "rustup_toolchain": os.environ.get("RUSTUP_TOOLCHAIN", "default"),
            "trial_count": 3,
            "cold_warm_semantics": "first measured invocation vs later invocations only; no OS cache flush/control claimed",
            "performance_threshold": None,
            "manifest_sha256": sha(harness / "scripts/perf/MANIFEST.toml"),
            "tool_versions": {
                "cargo": subprocess.check_output(
                    ["cargo", "--version"], text=True
                ).strip(),
                "rustc": subprocess.check_output(
                    ["rustc", "--version"], text=True
                ).strip(),
                "rustc_verbose": subprocess.check_output(
                    ["rustc", "-vV"], text=True
                ).strip(),
            },
            "harness_file_sha256": {
                rel: sha(harness / rel) for rel in PINNED_HARNESS_FILES
            },
            "runs": [],
            "builds": [],
            "invalid_reasons": [],
        }
    )
    FAILURE_STATE["meta"] = meta
    cargo = "cargo"
    env = os.environ.copy()
    env["RUSTUP_TOOLCHAIN"] = os.environ.get("RUSTUP_TOOLCHAIN", "stable")
    binaries = {}
    for side, root, rev in [
        ("baseline", base, a.baseline_sha),
        ("candidate", cand, a.candidate_sha),
    ]:
        FAILURE_STATE["phase"] = f"{side} release build"
        disk_ok(work)
        target = work / "target" / side
        binpath = work / "bin" / side / "tcl-lsp-server"
        binpath.parent.mkdir(parents=True, exist_ok=True)
        argv = [
            cargo,
            "build",
            "--locked",
            "--release",
            "-p",
            "tcl-lsp-server",
            "--target-dir",
            str(target),
        ]
        br = run_bounded(
            argv,
            cwd=root,
            env=env,
            out=out / "builds",
            stem=side,
            timeout_s=a.build_deadline,
        )
        meta["builds"].append({"side": side, "sha": rev, "process": br})
        built = (
            target
            / "release"
            / ("tcl-lsp-server.exe" if os.name == "nt" else "tcl-lsp-server")
        )
        if (
            br["exit_code"] != 0
            or br["stop_reason"]
            or not br["group_empty"]
            or not built.is_file()
        ):
            meta["invalid_reasons"].append(
                f"{side} release build did not complete cleanly"
            )
            dump(out / "paired-summary.json", meta)
            return 1
        shutil.copy2(built, binpath)
        binaries[side] = binpath
        meta["builds"][-1]["binary_sha256"] = sha(binpath)
        if not require_group_closed(br, f"{side} build", meta, out):
            return 1
        active = compiler_processes()
        meta["builds"][-1]["global_compiler_processes_before_target_cleanup"] = active
        if active:
            meta["builds"][-1]["target_cleanup"] = (
                "skipped-global-cargo-or-rustc-active"
            )
            meta["invalid_reasons"].append(
                f"{side}: active cargo/rustc process detected globally; target retained and comparison stopped"
            )
            dump(out / "paired-summary.json", meta)
            return 1
        retained = sum(p.stat().st_size for p in target.rglob("*") if p.is_file())
        shutil.rmtree(target)
        meta["builds"][-1]["target_cleanup"] = (
            "removed-after-group-closed-and-no-global-compiler"
        )
        meta["builds"][-1]["target_bytes_removed"] = retained
    for r in (base, cand, harness):
        dirty = subprocess.run(
            ["git", "status", "--porcelain"],
            cwd=r,
            capture_output=True,
            text=True,
            check=True,
        ).stdout
        if dirty.strip():
            raise Invalid(f"{r}: build modified the source checkout")
    bench = harness / "scripts/perf/bench.py"
    manifest = harness / "scripts/perf/MANIFEST.toml"
    import tomllib

    corpus_manifest = tomllib.loads(manifest.read_text())
    for group in corpus_manifest["scope"][a.scope]["groups"]:
        for repo in (x for x in corpus_manifest["repo"] if x["group"] == group):
            checkout = corpus / group / repo["name"]
            head = subprocess.run(
                ["git", "rev-parse", "HEAD"],
                cwd=checkout,
                capture_output=True,
                text=True,
                check=True,
            ).stdout.strip()
            dirty = subprocess.run(
                ["git", "status", "--porcelain"],
                cwd=checkout,
                capture_output=True,
                text=True,
                check=True,
            ).stdout
            if head != repo["commit"] or dirty.strip():
                raise Invalid(
                    f"corpus pin/worktree mismatch: {repo['group']}/{repo['name']}"
                )
    meta["corpus_manifest_sha256"] = sha(manifest)
    meta["corpus_repositories"] = [
        {"group": x["group"], "name": x["name"], "commit": x["commit"]}
        for x in corpus_manifest["repo"]
        if x["group"] in corpus_manifest["scope"][a.scope]["groups"]
    ]
    run_order = [
        ("baseline", "candidate"),
        ("candidate", "baseline"),
        ("baseline", "candidate"),
    ]
    raw_runs = {}
    for pair, (first, second) in enumerate(run_order, 1):
        for side in (first, second):
            FAILURE_STATE["phase"] = f"{side} pair {pair} benchmark"
            label = f"{side}-{a.scope}-pair{pair}"
            runout = out / "results"
            stage = work / "stage" / label
            argv = [
                sys.executable,
                str(bench),
                "--server",
                str(binaries[side]),
                "--version",
                label,
                "--manifest",
                str(manifest),
                "--corpus",
                str(corpus),
                "--scope",
                a.scope,
                "--out",
                str(runout),
                "--stage",
                str(stage),
                "--fail-on-failed-check",
                "--deadline",
                str(a.bench_deadline),
            ]
            disk_free = disk_ok(work)
            pr = run_bounded(
                argv,
                cwd=harness,
                env=env,
                out=out / "logs",
                stem=label,
                timeout_s=a.bench_deadline + 60,
            )
            result_path = runout / (label + ".json")
            parsed = None
            try:
                parsed = load_result(result_path, a.scope)
            except (Invalid, OSError, ValueError, TypeError, KeyError) as exc:
                meta["invalid_reasons"].append(
                    f"{label}: invalid/missing result: {exc}"
                )
            bad_checks = (
                [
                    c["id"]
                    for c in parsed.get("checks", [])
                    if (not c.get("ok") or c.get("note") is not None)
                ]
                if parsed
                else []
            )
            rec = {
                "pair": pair,
                "phase": "first-measured" if pair == 1 else "subsequent",
                "side": side,
                "sha": meta[side + "_sha"],
                "binary_sha256": sha(binaries[side]),
                "process": pr,
                "disk_free_before": disk_free,
                "result_path": str(result_path),
                "result_sha256": sha(result_path) if result_path.is_file() else None,
                "result_valid": parsed is not None,
                "failed_checks": bad_checks,
                "peak_rss_kib": max(
                    (x.get("rss_kib", 0) for x in parsed.get("timeline", [])), default=0
                )
                if parsed
                else None,
            }
            meta["runs"].append(rec)
            raw_runs[(pair, side)] = parsed
            if not require_group_closed(pr, label, meta, out):
                return 1
            if (
                pr["stop_reason"]
                or not pr["group_empty"]
                or pr["exit_code"] not in (0, 1)
                or parsed is None
            ):
                meta["invalid_reasons"].append(
                    f"{label}: child incomplete, invalid, or process group not closed"
                )
            if bad_checks:
                meta["invalid_reasons"].append(
                    f"{label}: failed/annotated checks: {bad_checks}"
                )
            elif pr["exit_code"] == 1 and not rec["failed_checks"]:
                meta["invalid_reasons"].append(
                    f"{label}: nonzero exit without failed benchmark checks"
                )
    comparisons = []
    for cid in CHECK_IDS:
        values = []
        for pair in range(1, 4):
            b = raw_runs.get((pair, "baseline"))
            c = raw_runs.get((pair, "candidate"))
            if not b or not c:
                continue
            if (
                b["corpus_revision"] != c["corpus_revision"]
                or b["corpus_files"] != c["corpus_files"]
                or b["documents"] != c["documents"]
                or b["anchors"] != c["anchors"]
            ):
                meta["invalid_reasons"].append(
                    f"pair{pair}: corpus/doc/anchor identity mismatch"
                )
            bc = next(x for x in b["checks"] if x["id"] == cid)
            cc = next(x for x in c["checks"] if x["id"] == cid)
            count_status, counts_match = meaningful_counts(bc, cc, cid)
            if not counts_match:
                meta["invalid_reasons"].append(
                    f"pair{pair} {cid}: response counts incomparable ({count_status})"
                )
            matched = (
                bc["ok"]
                and cc["ok"]
                and bc.get("note") is None
                and cc.get("note") is None
                and counts_match
            )
            response_counts_present = (
                bc.get("ops") is not None
                and bc.get("items") is not None
                and cc.get("ops") is not None
                and cc.get("items") is not None
            )
            values.append(
                {
                    "pair": pair,
                    "baseline": {
                        k: bc.get(k)
                        for k in (
                            "wall_s",
                            "cpu_s",
                            "rss_before_kib",
                            "rss_after_kib",
                            "ops",
                            "items",
                            "ok",
                            "note",
                        )
                    },
                    "candidate": {
                        k: cc.get(k)
                        for k in (
                            "wall_s",
                            "cpu_s",
                            "rss_before_kib",
                            "rss_after_kib",
                            "ops",
                            "items",
                            "ok",
                            "note",
                        )
                    },
                    "response_counts_present": response_counts_present,
                    "count_comparison": count_status,
                    "like_for_like": matched,
                    "wall_delta_s": round(cc["wall_s"] - bc["wall_s"], 4)
                    if matched
                    else None,
                    "cpu_delta_s": round(cc["cpu_s"] - bc["cpu_s"], 2)
                    if matched
                    else None,
                    "rss_before_delta_kib": cc["rss_before_kib"] - bc["rss_before_kib"]
                    if matched
                    else None,
                    "rss_after_delta_kib": cc["rss_after_kib"] - bc["rss_after_kib"]
                    if matched
                    else None,
                }
            )
        comparisons.append({"check_id": cid, "pairs": values})
    for row in comparisons:
        valid = [p for p in row["pairs"] if p["like_for_like"]]
        row["paired_delta_summary"] = {}
        for metric in (
            "wall_delta_s",
            "cpu_delta_s",
            "rss_before_delta_kib",
            "rss_after_delta_kib",
        ):
            xs = [p[metric] for p in valid if p[metric] is not None]
            row["paired_delta_summary"][metric] = {
                "n": len(xs),
                "median": statistics.median(xs) if xs else None,
                "min": min(xs) if xs else None,
                "max": max(xs) if xs else None,
            }
    meta["comparisons"] = comparisons
    meta["comparable"] = not meta["invalid_reasons"] and len(raw_runs) == 6
    lines = [
        f"# Paired Tcl corpus benchmark: {a.scope}",
        "",
        f"Baseline `{meta['baseline_sha']}` vs candidate `{meta['candidate_sha']}` on `{meta['platform']}` / `{meta['machine']}`.",
        "",
        "No performance threshold is applied. Pair 1 is the first invocation; pairs 2–3 are subsequent invocations. OS page-cache state is not controlled.",
        "",
        "| Check | Paired wall Δ median (min…max, s) | Paired CPU Δ median (min…max, s) | Paired RSS-after Δ median (min…max, KiB) | comparable pairs |",
        "|---|---:|---:|---:|---:|",
    ]
    for cmp in comparisons:
        ag = cmp["paired_delta_summary"]

        def fmt(key, ag=ag):
            v = ag[key]
            return "n/a" if not v["n"] else f"{v['median']} ({v['min']}…{v['max']})"

        lines.append(
            f"| `{cmp['check_id']}` | {fmt('wall_delta_s')} | {fmt('cpu_delta_s')} | {fmt('rss_after_delta_kib')} | {ag['wall_delta_s']['n']}/3 |"
        )
        for pair in cmp["pairs"]:
            lines.append(
                f"| ↳ pair {pair['pair']} | wall {pair['wall_delta_s']} s | CPU {pair['cpu_delta_s']} s | RSS {pair['rss_after_delta_kib']} KiB | counts present={pair['response_counts_present']} | like-for-like={pair['like_for_like']} |"
            )
    if meta["invalid_reasons"]:
        lines += [
            "",
            "## Incomplete or invalid",
            *[f"- {x}" for x in meta["invalid_reasons"]],
        ]
    (out / "comparison.md").write_text("\n".join(lines) + "\n")
    dump(out / "paired-summary.json", meta)
    for r in (base, cand, harness):
        dirty = subprocess.run(
            ["git", "status", "--porcelain"],
            cwd=r,
            capture_output=True,
            text=True,
            check=True,
        ).stdout
        if dirty.strip():
            meta["invalid_reasons"].append(
                f"{r}: benchmark modified the source checkout"
            )
    meta["comparable"] = not meta["invalid_reasons"] and len(raw_runs) == 6
    if meta["invalid_reasons"]:
        lines += [
            "",
            "## Final source-checkout guard",
            *[f"- {x}" for x in meta["invalid_reasons"]],
        ]
        (out / "comparison.md").write_text("\n".join(lines) + "\n")
        dump(out / "paired-summary.json", meta)
        return 1
    dump(out / "paired-summary.json", meta)
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (
        Invalid,
        OSError,
        ValueError,
        TypeError,
        KeyError,
        subprocess.SubprocessError,
    ) as e:
        preserve_failure(e)
        print(f"paired_bench: {type(e).__name__}: {e}", file=sys.stderr)
        raise SystemExit(2)
