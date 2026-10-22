#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Build, compare both switch modes, and delete each test executable."""

import argparse
import json
import os
from pathlib import Path
import re
import subprocess
import time

parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("root", type=Path)
parser.add_argument("--target-dir", type=Path)
parser.add_argument("--stage", choices=("lexer", "compiler", "spectcl", "server", "all"), default="all")
parser.add_argument("--suite", action="append", help="Rerun only these suites; preserve other saved outcomes")
args = parser.parse_args()
root = args.root.resolve()
evidence = root / "docs/work/name-handling/handoff/reviews/merge-inventory-20261019/perf/evidence"
env = dict(os.environ, CARGO_INCREMENTAL="0", CARGO_PROFILE_TEST_DEBUG="0",
           TCL_LSP_REQUIRE_TCLSH="9.0,8.6", TCL_LSP_JIMSH=str(root / "tmp/jimtcl-oracle/jimsh"),
           CARGO_TARGET_DIR=str((args.target_dir or root / "target").resolve()))
env.pop("ZZ_OLD_ENV", None)
filters = ["spectcl_source", "original_name_value", "formal_call_advice", "realm",
           "interprocedural", "w216", "w143", "incomplete_body"]
stages = {
    "lexer": ("tcl-lexer", ["--lib"], [("lib", [])]),
    "compiler": ("tcl-compiler", ["--lib"], [("lib", filters)]),
    "spectcl": ("tcl-spectcl", ["--test", "eval_loader", "--test", "golden_packs"],
                [("eval_loader", []), ("golden_packs", [])]),
    "server": ("tcl-lsp-server", ["--lib"], [
        ("startup", ["startup_index_and_factory_reindex_keep_checked_document_roles"]),
        ("workspace_symbol", ["tests::workspace_symbol_caps_its_answer"]),
    ]),
}
if args.suite:
    available = {suite for stage, (_, _, suites) in stages.items()
                 if args.stage in (stage, "all") for suite, _ in suites}
    assert set(args.suite) <= available, f"unknown suites: {set(args.suite) - available}"
summary = {}
failed = False
for stage, (package, targets, suites) in stages.items():
    if args.stage not in (stage, "all"):
        continue
    if args.suite:
        suites = [(suite, selected) for suite, selected in suites if suite in args.suite]
        if not suites:
            continue
        saved_summary = evidence / f"tests-{stage}-summary.json"
        if saved_summary.exists():
            summary.update(json.loads(saved_summary.read_text()))
    build_json = Path(f"/tmp/perf-{stage}-test-build.jsonl")
    build_log = Path(f"/tmp/perf-{stage}-test-build.log")
    print(f"BUILD {stage}", flush=True)
    with build_json.open("w") as out, build_log.open("w") as err:
        built = subprocess.run(["cargo", "test", "--locked", "-j2", "-p", package, *targets,
                                "--no-run", "--message-format=json"], cwd=root, env=env,
                               stdout=out, stderr=err)
    artifacts = {}
    warnings = []
    for line in build_json.read_text().splitlines():
        item = json.loads(line)
        if item.get("reason") == "compiler-artifact" and item.get("executable"):
            artifacts[item["target"]["name"]] = Path(item["executable"])
        if item.get("reason") == "compiler-message" and item["message"]["level"] == "warning":
            warnings.append(item["message"]["rendered"])
    if built.returncode or warnings:
        for exe in artifacts.values():
            exe.unlink(missing_ok=True)
        raise RuntimeError(f"{stage} build: exit={built.returncode}, warnings={len(warnings)}; {build_log}, {build_json}")
    try:
        assert artifacts, f"no test executables for {stage}"
        for suite, selected in suites:
            exe = (artifacts[suite] if stage == "spectcl" else next(iter(artifacts.values())))
            outcomes = {}
            for mode in ("new", "old"):
                log = evidence / f"tests-{stage}-{suite}-{mode}.out"
                package_root = root / "rust" / package
                mode_env = env | {"CARGO_MANIFEST_DIR": str(package_root)} | ({"ZZ_OLD_ENV": "1"} if mode == "old" else {})
                started = time.monotonic()
                print(f"RUN {stage}/{suite}/{mode}", flush=True)
                with log.open("w") as out:
                    try:
                        result = subprocess.run([str(exe), *selected, "--test-threads=2"],
                                                cwd=package_root, env=mode_env, stdout=out,
                                                stderr=subprocess.STDOUT,
                                                timeout=180 if stage == "server" and mode == "new" else None)
                        code = result.returncode
                    except subprocess.TimeoutExpired:
                        code = "timeout180"
                text = log.read_text()
                rows = dict(re.findall(r"^test (.+?) \.\.\. (ok|FAILED|ignored)$", text, re.M))
                assert rows or code == "timeout180", f"no test outcomes in {log}"
                outcomes[mode] = rows
                summary[f"{stage}/{suite}/{mode}"] = {
                    "returncode": code, "seconds": round(time.monotonic() - started, 3),
                    "passed": sum(x == "ok" for x in rows.values()),
                    "failed": sum(x == "FAILED" for x in rows.values()),
                    "ignored": sum(x == "ignored" for x in rows.values()),
                    "log": log.name,
                }
                if code:
                    failed = True
                print(json.dumps(summary[f"{stage}/{suite}/{mode}"]), flush=True)
                (evidence / f"tests-{stage}-summary.json").write_text(json.dumps(summary, indent=2) + "\n")
            regressions = [name for name, state in outcomes["old"].items()
                           if state == "ok" and outcomes["new"].get(name) != "ok"]
            assert not regressions, f"pass-to-fail: {regressions}"
            assert set(outcomes["new"]) == set(outcomes["old"]), "test inventories differ"
            print(f"PASS zero pass-to-fail {stage}/{suite}", flush=True)
    finally:
        for exe in artifacts.values():
            exe.unlink(missing_ok=True)
if failed:
    raise SystemExit("One or more test runs failed; inspect the complete saved output")
