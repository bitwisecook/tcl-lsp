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

"""Record the non-gating Now-1 baseline for plan item 106a.

Run only on a benchmark runner, with release binaries. No latency or size
threshold is enforced. Missing interfaces and unbuilt artefacts remain explicit
unavailable rows; invalid or provisional answers never enter a percentile.
"""

from __future__ import annotations

import argparse
import json
import math
import os
import statistics
import subprocess
import tempfile
import time
from collections.abc import Callable
from pathlib import Path
from typing import Any

from bench import REPO_ROOT, host_info, initialize
from dialect_pair_bench import DialectClient, rss_kib, semantic_tokens
from interp_instructions import BENCHMARKS, engine_command, render

SCHEMA = 2
CASES = ("small", "large", "source", "tcllib", "engines")
ARTEFACTS = (
    "server.native",
    "server.wasm",
    "vsix",
    "jetbrains.zip",
    "web.bundle",
    "server.wasi",
)
WORKSPACE_METRICS = (
    "start_ms",
    "start_rss_kib",
    "cold_ms",
    "point_warm_ms",
    "collection_ms",
    "transform_ms",
    "point_after_edit_ms",
    "rss_per_10000_lines_kib",
    "restart_ms",
)
DEFERRED = {
    "binding.cold_10513_lines_ms": "Binding-only timing is not exposed; workspace cold timings include parsing and other analysis.",
    "binding.from_disk_ms": "The on-disk binding tier belongs to item 103 and is not exposed.",
    "summaries.per_10000_lines_ms": "Summary-only timing is not exposed; item 483 owns the summary framework.",
    "packs.warm_50_ms": "The 50-user-pack warm-load interface and fixture belong to item 416.",
    "engine.runtime": "No standalone runtime release runner supplied.",
}


def unavailable(reason: str) -> dict[str, Any]:
    return {"status": "unavailable", "reason": reason}


def provisional(value: Any) -> bool:
    if isinstance(value, dict):
        return (
            value.get("provisional") is True
            or value.get("kind") == "provisional"
            or any(provisional(v) for v in value.values())
        )
    return isinstance(value, list) and any(provisional(v) for v in value)


def distribution(samples: list[float]) -> dict[str, Any]:
    if not samples:
        raise ValueError("an empty sample set has no percentile")
    if any(not math.isfinite(value) or value < 0 for value in samples):
        raise ValueError("samples must be finite nonnegative durations")
    ordered = sorted(samples)
    return {
        "status": "measured",
        "samples": len(samples),
        "median_ms": statistics.median(samples),
        "p95_ms": ordered[math.ceil(len(ordered) * 0.95) - 1],
        "min_ms": ordered[0],
        "max_ms": ordered[-1],
    }


class Measurements:
    def __init__(self) -> None:
        self.rows: dict[str, dict[str, Any]] = {
            **{key: unavailable(reason) for key, reason in DEFERRED.items()},
            **{
                f"workspace.{name}.{metric}": unavailable("Measurement not reached")
                for name in ("small", "large", "source")
                for metric in WORKSPACE_METRICS
            },
            "tcllib.end_to_end_ms": unavailable("Measurement not reached"),
            **{
                f"engine.tclvm.{source.stem}_ms": unavailable("Measurement not reached")
                for source in BENCHMARKS.glob("*.tcl")
            },
        }

    def observe(
        self,
        key: str,
        operation: Callable[[], Any],
        valid: Callable[[Any], bool],
        repeats: int = 1,
    ) -> None:
        samples: list[float] = []
        counts: list[int] = []
        failures: list[str] = []
        attempts = 0
        for _ in range(repeats):
            attempts += 1
            start = time.perf_counter()
            try:
                value = operation()
                elapsed = (time.perf_counter() - start) * 1000
                if provisional(value) or not valid(value):
                    raise ValueError("provisional, missing or invalid response")
                samples.append(elapsed)
                counts.append(len(value) if isinstance(value, (list, dict)) else 1)
            except (
                RuntimeError,
                TimeoutError,
                ValueError,
                OSError,
                subprocess.SubprocessError,
            ) as error:
                failures.append(f"{type(error).__name__}: {error}")
                if isinstance(error, (TimeoutError, subprocess.TimeoutExpired)):
                    break
        row = (
            distribution(samples)
            if samples
            else unavailable("No complete valid sample")
        )
        row.update(
            {
                "attempts": attempts,
                "requested": repeats,
                "accepted": len(samples),
                "result_counts": counts,
                "excluded": failures,
            }
        )
        if failures and samples:
            row["status"] = "partial"
        self.rows[key] = row


def workspace_sources() -> dict[str, dict[str, str]]:
    small = "proc target {value} {\nreturn $value\n}\nset input 7\nset output [target $input]\nputs $output\n"
    large = ["proc target {value} {", "return $value", "}"]
    for index in range(999):
        large.extend(
            [
                f"proc consumer_{index:04d} {{value}} {{",
                "set local $value",
                "set second $local",
                "set third $second",
                "set fourth $third",
                "set fifth $fourth",
                "set sixth $fifth",
                "set seventh $sixth",
                "return [target $seventh]",
                "}",
            ]
        )
    large.extend(["# stable workload padding"] * (10_000 - len(large) - 2))
    large.extend(["set input 7", "set output [target $input]"])
    return {
        "small": {"main.tcl": small},
        "large": {"main.tcl": "\n".join(large) + "\n"},
        "source": {
            "main.tcl": "source [file join [file dirname [info script]] lib.tcl]\nset input 7\nif {1} {\nset output [target $input]\n}\nputs $output\n",
            "lib.tcl": "proc target {value} {\nreturn $value\n}\n",
        },
    }


def stage_workspaces(root: Path) -> dict[str, Path]:
    result = {}
    for name, files in workspace_sources().items():
        directory = root / name
        directory.mkdir(parents=True)
        for filename, text in files.items():
            (directory / filename).write_text(text)
        result[name] = directory
    return result


class Session:
    def __init__(
        self,
        server: Path,
        root: Path,
        dialect: str,
        timeout: float,
        scan_timeout: float | None = None,
    ) -> None:
        self.client = DialectClient(str(root), [str(server)], dialect)
        self.timeout = timeout
        self.scan_timeout = scan_timeout if scan_timeout is not None else timeout

    def start(self) -> Any:
        self.client.start()
        initialize(self.client, Path(self.client.server_dir))
        return self.client.process is not None

    def scan(self) -> str:
        return self.client.wait_for_workspace_scan(timeout=self.scan_timeout)

    def open(self, path: Path, version: int = 1, text: str | None = None) -> str:
        uri = path.resolve().as_uri()
        self.client.send_notification(
            "textDocument/didOpen",
            {
                "textDocument": {
                    "uri": uri,
                    "languageId": "tcl",
                    "version": version,
                    "text": text if text is not None else path.read_text(),
                }
            },
        )
        self.ready(uri, version)
        return uri

    def ready(self, uri: str, version: int) -> None:
        semantic_tokens(self.client, uri, self.timeout)
        deadline = time.monotonic() + self.timeout
        while time.monotonic() < deadline:
            for notification in self.client.collect_notifications(
                "textDocument/publishDiagnostics", timeout=0
            ):
                params = notification.get("params", {})
                if (
                    params.get("uri") == uri
                    and params.get("version") == version
                    and not provisional(params)
                ):
                    return
            time.sleep(0.01)
        raise TimeoutError(f"no complete diagnostics for document version {version}")

    def request(self, method: str, params: dict[str, Any]) -> Any:
        return self.client.send_request(method, params, timeout=self.timeout)

    def memory(self) -> int | None:
        process = self.client.process
        return rss_kib(process.pid).get("VmRSS") if process else None

    def close(self) -> None:
        self.client.shutdown()


def location_for(text: str) -> dict[str, int]:
    offset = text.rindex("target ")
    return {
        "line": text.count("\n", 0, offset),
        "character": offset - text.rfind("\n", 0, offset) - 1,
    }


def run_workspace(
    args: argparse.Namespace, rows: Measurements, name: str, root: Path
) -> None:
    session = Session(args.server, root, args.dialect, args.timeout, args.scan_timeout)
    prefix = f"workspace.{name}"
    try:
        rows.observe(f"{prefix}.start_ms", session.start, bool)
        if rows.rows[f"{prefix}.start_ms"]["status"] != "measured":
            raise RuntimeError("Server initialisation did not complete")
        memory = session.memory()
        rows.rows[f"{prefix}.start_rss_kib"] = (
            {"status": "measured", "value": memory}
            if memory is not None
            else unavailable("RSS unavailable on this host")
        )
        rows.observe(f"{prefix}.cold_ms", session.scan, bool)
        if rows.rows[f"{prefix}.cold_ms"]["status"] != "measured":
            raise RuntimeError("Workspace scan did not complete")
        path = root / "main.tcl"
        text = path.read_text()
        uri = session.open(path)
        params = {"textDocument": {"uri": uri}, "position": location_for(text)}
        rows.observe(
            f"{prefix}.point_warm_ms",
            lambda: session.request("textDocument/definition", params),
            lambda value: isinstance(value, list) and bool(value),
            args.samples,
        )
        rows.observe(
            f"{prefix}.collection_ms",
            lambda: session.request(
                "textDocument/references",
                {**params, "context": {"includeDeclaration": True}},
            ),
            lambda value: isinstance(value, list) and bool(value),
            args.samples,
        )
        rows.observe(
            f"{prefix}.transform_ms",
            lambda: session.request(
                "textDocument/formatting",
                {
                    "textDocument": {"uri": uri},
                    "options": {"tabSize": 4, "insertSpaces": True},
                },
            ),
            lambda value: isinstance(value, list),
            args.samples,
        )
        version = 1

        def edit_point() -> Any:
            nonlocal version
            version += 1
            session.client.send_notification(
                "textDocument/didChange",
                {
                    "textDocument": {"uri": uri, "version": version},
                    "contentChanges": [{"text": text + f"# edit {version:08d}\n"}],
                },
            )
            session.ready(uri, version)
            return session.request("textDocument/definition", params)

        rows.observe(
            f"{prefix}.point_after_edit_ms",
            edit_point,
            lambda value: isinstance(value, list) and bool(value),
            args.samples,
        )
        memory = session.memory()
        lines = sum(p.read_text().count("\n") for p in root.glob("*.tcl"))
        rows.rows[f"{prefix}.rss_per_10000_lines_kib"] = (
            {
                "status": "measured",
                "value": memory * 10_000 / lines,
                "rss_kib": memory,
                "lines": lines,
                "open_documents": 1,
            }
            if memory is not None
            else unavailable("RSS unavailable on this host")
        )
    except (RuntimeError, TimeoutError, ValueError, OSError) as error:
        rows.rows[f"{prefix}.completion"] = unavailable(
            f"{type(error).__name__}: {error}"
        )
    finally:
        session.close()
    restart = Session(args.server, root, args.dialect, args.timeout, args.scan_timeout)
    try:

        def ready_restart() -> bool:
            restart.start()
            restart.scan()
            return True

        rows.observe(f"{prefix}.restart_ms", ready_restart, bool)
    finally:
        restart.close()


def run_tcllib(args: argparse.Namespace, rows: Measurements) -> None:
    if args.tcllib is None or not args.tcllib.is_dir():
        rows.rows["tcllib.end_to_end_ms"] = unavailable(
            "No verified Tcllib 2.0 source supplied"
        )
        return
    root = args.tcllib / "modules"
    if not root.is_dir():
        raise ValueError("Tcllib root must contain modules/")
    env = {**os.environ, "TCLLIB_2_0_DIR": str(args.tcllib.resolve())}
    env.pop("SKIP_TCLLIB", None)
    subprocess.run(
        [
            "bash",
            str(REPO_ROOT / "scripts/dev/ensure-test-deps.sh"),
            "--check",
            "--tcllib-only",
        ],
        env=env,
        check=True,
        timeout=30,
        capture_output=True,
        text=True,
    )
    files = sorted(root.rglob("*.tcl"))
    if not files:
        raise ValueError("Tcllib corpus is empty")
    session = Session(args.server, root, args.dialect, args.timeout, args.scan_timeout)
    session.scan_timeout = args.tcllib_timeout
    deadline = time.monotonic() + args.tcllib_timeout

    def analyse_all() -> list[str]:
        session.start()
        session.scan()
        completed = []
        for path in files:
            if time.monotonic() >= deadline:
                raise TimeoutError(
                    f"Tcllib analysis budget exhausted after {len(completed)}/{len(files)} files"
                )
            uri = session.open(path)
            completed.append(uri)
            session.client.send_notification(
                "textDocument/didClose", {"textDocument": {"uri": uri}}
            )
        return completed

    try:
        rows.observe(
            "tcllib.end_to_end_ms", analyse_all, lambda value: len(value) == len(files)
        )
        rows.rows["tcllib.end_to_end_ms"].update(
            {"release": "2.0", "files": len(files)}
        )
    finally:
        session.close()


def artefacts(rows: Measurements, server: Path, supplied: list[str]) -> None:
    paths = {"server.native": server}
    for item in supplied:
        key, separator, value = item.partition("=")
        if not separator or key not in ARTEFACTS:
            raise ValueError(f"artefact must be a known key=path: {item}")
        paths[key] = Path(value)
    for key in ARTEFACTS:
        path = paths.get(key)
        rows.rows[f"size.{key}.bytes"] = (
            {"status": "measured", "value": path.stat().st_size}
            if path and path.is_file()
            else unavailable(
                "Artefact not built on this runner; no size inferred from another revision"
            )
        )


def engine_rows(args: argparse.Namespace, rows: Measurements, temporary: Path) -> None:
    for name, binary in [("tclvm", args.vm), ("runtime", args.runtime)]:
        if binary is None:
            rows.rows[f"engine.{name}"] = unavailable(
                "Release engine runner not supplied"
            )
            continue
        for source in sorted(BENCHMARKS.glob("*.tcl")):
            script = temporary / source.name
            script.write_text(render(source.read_text(), args.iterations))

            command = engine_command(name, str(binary), script)
            command[1:1] = ["--tcl-version", "9.0"]
            argv = tuple(command)

            def execute(
                command: tuple[str, ...] = argv,
            ) -> subprocess.CompletedProcess[str]:
                env = dict(os.environ)
                env.pop("TCL_LIBRARY", None)
                return subprocess.run(
                    command,
                    capture_output=True,
                    text=True,
                    check=True,
                    timeout=args.timeout,
                    env=env,
                )

            rows.observe(
                f"engine.{name}.{source.stem}_ms",
                execute,
                lambda result: result.returncode == 0,
                3,
            )
    if args.runtime is not None:
        rows.rows.pop("engine.runtime", None)


def markdown(result: dict[str, Any]) -> str:
    lines = [
        "# Now-1 split baseline",
        "",
        f"Revision: `{result['revision']}`. Profile: `release`. Dialect: `{result['dialect']}`.",
        "",
        "| Measurement | Status | Value |",
        "|---|---|---|",
    ]
    for key, row in sorted(result["rows"].items()):
        value = (
            str(round(row["p95_ms"], 3)) + " ms p95"
            if "p95_ms" in row
            else str(row.get("value", row.get("reason", "")))
        )
        lines.append(f"| `{key}` | {row['status']} | {value.replace('|', '/')} |")
    return "\n".join(lines) + "\n"


def owns_row(case: str, key: str) -> bool:
    if case in ("small", "large", "source"):
        return key.startswith(f"workspace.{case}.")
    if case == "tcllib":
        return key.startswith("tcllib.")
    return not key.startswith(("workspace.", "tcllib."))


def write_result(out: Path, result: dict[str, Any]) -> None:
    out.mkdir(parents=True, exist_ok=True)
    (out / "baseline.json").write_text(
        json.dumps(result, indent=2, sort_keys=True, allow_nan=False) + "\n"
    )
    (out / "baseline.md").write_text(markdown(result))


def merge_results(root: Path, revision: str) -> dict[str, Any]:
    parts = [
        json.loads(path.read_text()) for path in sorted(root.rglob("baseline.json"))
    ]
    if not parts:
        raise ValueError("No case measurements available to combine")
    result = {
        key: parts[0][key]
        for key in ("schema", "profile", "dialect", "configuration", "notes")
    }
    result.update({"revision": revision, "case": "all", "hosts": {}, "rows": {}})
    seen: set[str] = set()
    for part in parts:
        case = part.get("case")
        if case not in CASES or case in seen:
            raise ValueError(f"Unknown or duplicate benchmark case: {case}")
        if part["revision"] != revision or any(
            part[key] != result[key]
            for key in ("schema", "profile", "dialect", "configuration")
        ):
            raise ValueError(
                "Cannot combine different revisions or benchmark configurations"
            )
        seen.add(case)
        result["hosts"][case] = part["host"]
        result["rows"].update(
            {key: row for key, row in part["rows"].items() if owns_row(case, key)}
        )
    missing = sorted(set(CASES) - seen)
    defaults = Measurements()
    for key in ARTEFACTS:
        defaults.rows[f"size.{key}.bytes"] = unavailable(
            "Benchmark case did not upload a result"
        )
    for source in BENCHMARKS.glob("*.tcl"):
        defaults.rows[f"engine.tclvm.{source.stem}_ms"] = unavailable(
            "Benchmark case did not upload a result"
        )
    for case in missing:
        for key in defaults.rows:
            if owns_row(case, key):
                result["rows"][key] = unavailable(
                    f"Case {case} did not upload a result"
                )
    result["missing_cases"] = missing
    return result


def main(argv: list[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--server", type=Path)
    parser.add_argument("--case", choices=("all", *CASES), default="all")
    parser.add_argument("--merge", type=Path)
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--revision", required=True)
    parser.add_argument("--dialect", default="tcl8.6")
    parser.add_argument("--samples", type=int, default=20)
    parser.add_argument("--timeout", type=float, default=120)
    parser.add_argument("--scan-timeout", type=float, default=600)
    parser.add_argument("--tcllib-timeout", type=float, default=1800)
    parser.add_argument("--tcllib", type=Path)
    parser.add_argument("--vm", type=Path)
    parser.add_argument("--runtime", type=Path)
    parser.add_argument("--iterations", type=int, default=5000)
    parser.add_argument("--artefact", action="append", default=[])
    args = parser.parse_args(argv)
    if (
        args.samples < 1
        or args.timeout <= 0
        or args.scan_timeout <= 0
        or args.tcllib_timeout <= 0
        or args.iterations < 1
    ):
        parser.error("sample counts, iterations and timeouts must be positive")
    if args.merge:
        write_result(args.out, merge_results(args.merge, args.revision))
        return 0
    if args.server is None:
        parser.error("--server is required when measuring")
    args.server = args.server.resolve(strict=True)
    args.vm = args.vm.resolve(strict=True) if args.vm else None
    args.runtime = args.runtime.resolve(strict=True) if args.runtime else None
    args.out.mkdir(parents=True, exist_ok=True)
    rows = Measurements()
    artefacts(rows, args.server, args.artefact)

    def save() -> None:
        result = {
            "schema": SCHEMA,
            "case": args.case,
            "revision": args.revision,
            "profile": "release",
            "dialect": args.dialect,
            "host": host_info(),
            "configuration": {
                "samples": args.samples,
                "engine_iterations": args.iterations,
                "request_timeout_s": args.timeout,
                "workspace_scan_timeout_s": args.scan_timeout,
                "tcllib_timeout_s": args.tcllib_timeout,
            },
            "notes": [
                "No budget gate; unavailable and excluded samples are explicit.",
                "After-edit latency includes readiness of versioned diagnostics before the definition response.",
                "Engine wall time includes process startup; Tcl release is 9.0 and TCL_LIBRARY is unset.",
            ],
            "rows": {
                key: row
                for key, row in rows.rows.items()
                if args.case == "all" or owns_row(args.case, key)
            },
        }
        write_result(args.out, result)

    save()
    try:
        with tempfile.TemporaryDirectory(prefix="tcl-split-baseline-") as name:
            temporary = Path(name)
            for workspace, root in stage_workspaces(
                temporary / "editor-actions"
            ).items():
                if args.case not in ("all", workspace):
                    continue
                print(f"Measuring {workspace}", flush=True)
                run_workspace(args, rows, workspace, root)
                save()
            if args.case in ("all", "tcllib"):
                run_tcllib(args, rows)
                save()
            if args.case in ("all", "engines"):
                engine_rows(args, rows, temporary)
    finally:
        save()
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
