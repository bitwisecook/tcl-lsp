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

"""WORK-only isolated baseline14, stack1 and separate actual other103 diagnostics."""

import argparse
import collections
import hashlib
import importlib.util
import json
import os
import re
import resource
import time
from pathlib import Path

STACK_FRAMES_SHA = "bfea2077ec3e3c166970478d3eb7d51d1d66c804d99609c70d211f478c5b203e"
EXECUTOR_SHA = "88125bd5267ac897dda4b5f665872c5ae0ab9984b0f42a41edeadf718873d811"
ROSTER_SHA = "aa5b03f494b6eca91cf00b0b1d6d34e044dd231c76b16d7384e4e9399e6a493f"
OTHER_ROSTER_SHA = "cda2e5059ae902b92ce9881852358f2fc4188e64d9c14518501cf2b5a6d24821"
FAILURE_LOG_FILE_SHA = (
    "88dd41eae670e31bd0f08d857f3fc35f2ccf53164d0127096f5a30c08e675ab5"
)
FAILURE_LOG_SHA = "616b04e54b17eb137bb6f7942d3e07dcc691ee807a6233d0c47419f63a34967c"
GROUPS = ("baseline14", "stack_abort1", "other103")
CAP = 20 * 1024**2 - 65536
FLOOR = 2 * 1024**3
BASE = Path(__file__).resolve().parent


def require(value, message):
    if not value:
        raise RuntimeError(message)


def sha(path):
    with Path(path).open("rb") as stream:
        return hashlib.file_digest(stream, "sha256").hexdigest()


def save(path, value):
    Path(path).write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def inventories():
    baseline_path = BASE / "runtime-baseline-supplement-roster.json"
    require(sha(baseline_path) == ROSTER_SHA, "exact supplemental roster changed")
    roster = json.loads(baseline_path.read_text())
    other_path = BASE / "runtime-other-failures-roster.json"
    require(sha(other_path) == OTHER_ROSTER_SHA, "exact other103 roster changed")
    other = json.loads(other_path.read_text())
    log = BASE / "runtime-a26-failed-originals.log.json"
    require(sha(log) == FAILURE_LOG_FILE_SHA, "lossless failure log container changed")
    original_log = json.loads(log.read_text())
    original_bytes = original_log["text"].encode("utf-8")
    require(
        original_log["encoding"] == "utf-8"
        and original_log["raw_byte_count"] == len(original_bytes) == 326773
        and original_log["raw_sha256"] == FAILURE_LOG_SHA
        and hashlib.sha256(original_bytes).hexdigest() == FAILURE_LOG_SHA,
        "authenticated original failure log bytes changed",
    )
    require(
        other["observed_source"] == "a26ab0ffd2b1dc1681710dbf14e518025545ec33"
        and other["observed_run"] == 40039952540
        and other["observed_job"] == 120708509600
        and other["authenticated_decoded_log_sha256"] == FAILURE_LOG_SHA
        and other["baseline_roster_sha256"] == ROSTER_SHA,
        "original source/log/roster join differs",
    )
    failed = re.findall(
        r"\btest ([A-Za-z0-9_:]+) \.\.\. FAILED\b", original_log["text"]
    )
    require(
        len(failed) == len(set(failed)) == other["failed117_count"] == 117
        and set(roster["baseline14"]).issubset(failed),
        "exact authenticated117 failed identities required",
    )
    expected = [name for name in failed if name not in roster["baseline14"]]
    require(
        other["other103"] == expected and len(expected) == len(set(expected)) == 103,
        "other103 is not exact original117 minus baseline14",
    )
    roster["other103"] = expected
    return roster, other


def listed(suite, name):
    case = suite["testcases"].get(name)
    return case is not None and case["kind"] == "test" and not case["ignored"]


def selection(out):
    roster, other = inventories()
    require(
        len(roster["baseline14"]) == len(set(roster["baseline14"])) == 14,
        "baseline14 identities",
    )
    require(
        roster["stack_abort1"]
        == [
            "interp::tests::deeply_nested_foreach_reaches_the_explicit_host_evaluation_limit"
        ],
        "stack identity",
    )
    wanted = roster["baseline14"] + roster["stack_abort1"]
    require(len(wanted) == len(set(wanted)) == 15, "duplicate supplemental identities")
    require(
        len(set(wanted + roster["other103"])) == 118,
        "other103 intersects baseline14 or stack1",
    )
    listing = json.loads((out / "selected.json").read_text())
    suites = [
        suite
        for suite in listing["rust-suites"].values()
        if suite["binary-id"] == "tcl-runtime"
    ]
    require(len(suites) == 1, "exact actual runtime ELF suite required")
    suite = suites[0]
    require(
        suite["kind"] == "lib" and suite["status"] == "listed",
        "actual lib suite required",
    )
    require(
        all(
            name in suite["testcases"]
            and suite["testcases"][name]["kind"] == "test"
            and not suite["testcases"][name]["ignored"]
            for name in wanted
        ),
        "supplement identity absent or ignored",
    )
    selected = {
        name
        for name, case in suite["testcases"].items()
        if case["filter-match"]["status"] == "matches"
    }
    require(
        len(selected) == 3
        and not selected.intersection(wanted)
        and selected.intersection(roster["other103"])
        == set(other["repeated_original3"])
        == {
            "builtins::tests::native_expression_and_increment_follow_selected_numeric_towers"
        },
        "original3 identities or explicitly repeated103 inventory differs",
    )
    binary = Path(suite["binary-path"]).resolve()
    target = Path(os.environ["CARGO_TARGET_DIR"]).resolve()
    require(
        binary.is_file() and binary.is_relative_to(target),
        "actual fresh-target ELF required",
    )
    return roster, suite, binary


def prepare(out, source):
    require(
        not (out / "runtime-elf-before-original3.json").exists(),
        "exclusive before-original3 ELF seal required",
    )
    _roster, suite, binary = selection(out)
    save(
        out / "runtime-elf-before-original3.json",
        {
            "source": source,
            "binary": str(binary),
            "elf_sha256": sha(binary),
            "listing_sha256": sha(out / "selected.json"),
            "cwd": suite["cwd"],
            "original_selected_count": 3,
            "supplemental14_count": 14,
            "separate_stack_count": 1,
            "separate_other103_count": 103,
            "other103_listing_unavailable": [
                name for name in _roster["other103"] if not listed(suite, name)
            ],
            "other103_roster_sha256": OTHER_ROSTER_SHA,
            "original_failure_log_sha256": FAILURE_LOG_SHA,
        },
    )

    save(
        out / "supplement-plan.json",
        [
            {"group": group, "selector": name, "status": "UNRUN"}
            for group in GROUPS
            for name in _roster[group]
        ],
    )
    save(
        out / "original3-plan.json",
        {
            "source": source,
            "rows": [
                {"selector": name, "status": "UNRUN"}
                for name, case in suite["testcases"].items()
                if case["filter-match"]["status"] == "matches"
            ],
            "scope": "Planned inventory only; actual original3 terminal events remain authoritative.",
        },
    )


def provider_log_bytes(out):
    paths = [
        out / "c84-provider/bootstrap.stdout",
        out / "c84-provider/bootstrap.stderr",
    ]
    return sum(path.stat().st_size for path in paths if path.is_file())


def stack_streams(out):
    streams = {}
    for name in ["worker.stdout", "worker.stderr"]:
        path = out / "stack-frames" / name
        streams[name] = {
            "bytes": path.stat().st_size if path.exists() else 0,
            "sha256": sha(path) if path.exists() else None,
        }
    return streams


def frames(out, source):
    _roster, _suite, binary = selection(out)
    prepared = json.loads((out / "runtime-elf-before-original3.json").read_text())
    require(
        prepared["source"] == source
        and prepared["binary"] == str(binary)
        and prepared["elf_sha256"] == sha(binary)
        and prepared["listing_sha256"] == sha(out / "selected.json"),
        "same prepared original3 ELF and listing required before inspection",
    )
    helper = BASE / "runtime-stack-frames.py"
    require(sha(helper) == STACK_FRAMES_SHA, "reviewed stack metadata helper changed")
    spec = importlib.util.spec_from_file_location("runtime_stack_metadata", helper)
    if spec is None or spec.loader is None:
        raise RuntimeError("stack metadata helper loader unavailable")
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    build_bytes = (out / "build.stderr").stat().st_size + provider_log_bytes(out)
    record = module.capture_stack_frames(
        out=out,
        binary=binary,
        source=source,
        expected_elf_sha256=prepared["elf_sha256"],
        environment=os.environ.copy(),
        absolute_deadline=float(os.environ["RUNTIME_DIAGNOSTIC_DEADLINE"]),
        remaining_log_bytes=max(0, 20 * 1024**2 - build_bytes),
    )
    streams = stack_streams(out)
    retained = sum(stream["bytes"] for stream in streams.values())
    process = record.get("process")
    closed = (
        record.get("status") == "UNRUN"
        if process is None
        else isinstance(process, dict)
        and process.get("group_empty_before_parent_reap") is True
        and process.get("consecutive_complete_empty_group_scans", 0) >= 2
        and process.get("owned_live_group_members") == []
    )
    unchanged = sha(binary) == prepared["elf_sha256"]
    complete = (
        record["source"] == source
        and record["binary"] == str(binary)
        and record["expected_elf_sha256"] == prepared["elf_sha256"]
        and record["status"] == "CAPTURED"
        and record["capture_incomplete"] is False
        and type(record["retained_bytes"]) is int
        and record["retained_bytes"] == retained
        and isinstance(process, dict)
        and closed
        and unchanged
    )
    save(
        out / "stack-frame-accounting.json",
        {
            "source": source,
            "elf_sha256": prepared["elf_sha256"],
            "helper_sha256": STACK_FRAMES_SHA,
            "summary_sha256": sha(out / "stack-frames/summary.json"),
            "streams": streams,
            "actual_retained_log_bytes": retained,
            "reported_retained_bytes": record["retained_bytes"],
            "owned_group_closed": closed,
            "elf_unchanged": unchanged,
            "complete": complete,
            "scope": "Static metadata only, not active stack/depth or a stack fix. Unknown/omitted data stays incomplete. Actual retained bytes join the existing combined budget; original3 stream caps stay unchanged and do not guarantee a retroactive aggregate cap.",
        },
    )
    if not closed or not unchanged:
        return 126
    return 0 if complete else 125


def original_outcome(stdout, receipt):
    code = receipt["exit_code"]
    if receipt["stop_reason"] == "selector_timeout":
        return "TIMEOUT"
    if not receipt["complete"]:
        return "INCOMPLETE"
    if code < 0:
        return "ABORT"
    terminals = re.findall(
        r"^test result: (ok|FAILED)\. (\d+) passed; (\d+) failed; (\d+) ignored; (\d+) measured; (\d+) filtered out;",
        stdout,
        re.MULTILINE,
    )
    require(
        len(terminals) == 1 and "running 1 test" in stdout,
        "exact one-test terminal missing",
    )
    label, passed, failed, ignored, measured, _filtered = terminals[0]
    require(
        (label, int(passed), int(failed), int(ignored), int(measured), code)
        in [("ok", 1, 0, 0, 0, 0), ("FAILED", 0, 1, 0, 0, 101)],
        "exact terminal/exit disagreement",
    )
    return "PASS" if label == "ok" else "FAIL"


def run(out, source):
    supplemental = out / "supplement"
    supplemental.mkdir()
    roster, _other = inventories()
    rows = [
        {"group": group, "selector": name, "status": "UNRUN"}
        for group in GROUPS
        for name in roster[group]
    ]
    save(supplemental / "outcomes.json", rows)
    roster, suite, binary = selection(out)
    errors = []
    started = time.monotonic()
    suite_deadline = started + 2400
    absolute = float(os.environ["RUNTIME_DIAGNOSTIC_DEADLINE"])
    require(
        time.time() < absolute <= time.time() + 3600,
        "actual bounded workflow deadline required",
    )
    prepared = json.loads((out / "runtime-elf-before-original3.json").read_text())
    require(
        prepared["source"] == source
        and prepared["binary"] == str(binary)
        and prepared["elf_sha256"] == sha(binary),
        "same original3 ELF required",
    )
    require(
        prepared["listing_sha256"] == sha(out / "selected.json"),
        "original full listing changed",
    )
    original = json.loads((out / "diagnostic-completeness.json").read_text())
    require(not original["incomplete"], "original3 capture incomplete")
    executor = BASE / "runtime-diagnostics-exec.py"
    require(sha(executor) == EXECUTOR_SHA, "reviewed owned-process executor changed")
    spec = importlib.util.spec_from_file_location("runtime_supplement_owner", executor)
    if spec is None or spec.loader is None:
        raise RuntimeError("process owner loader unavailable")
    owner = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(owner)
    resource.setrlimit(resource.RLIMIT_CORE, (0, 0))
    used = sum(
        path.stat().st_size
        for path in [
            out / "build.stderr",
            out / "terminal/nextest-stderr.txt",
            out / "terminal/reporter.raw.jsonl",
        ]
    )
    used += provider_log_bytes(out)
    frame_streams = stack_streams(out)
    used += sum(stream["bytes"] for stream in frame_streams.values())
    accounting_path = out / "stack-frame-accounting.json"
    if not accounting_path.exists():
        errors.append("stack metadata accounting missing")
    else:
        accounting = json.loads(accounting_path.read_text())
        if (
            accounting["source"] != source
            or accounting["elf_sha256"] != prepared["elf_sha256"]
            or accounting["helper_sha256"] != STACK_FRAMES_SHA
            or accounting["streams"] != frame_streams
            or accounting["summary_sha256"] != sha(out / "stack-frames/summary.json")
            or accounting["complete"] is not True
        ):
            errors.append("stack metadata unavailable, changed or incomplete")
    environment = os.environ.copy()
    environment["TCL_LSP_RUNTIME_BASELINE_DIAGNOSTIC"] = "1"
    for index, row in enumerate(rows):
        if not listed(suite, row["selector"]):
            row["unrun_reason"] = (
                "original selector absent or ignored in fresh ELF listing"
            )
            errors.append("fresh listing does not authenticate " + row["selector"])
            save(supplemental / "outcomes.json", rows)
            continue
        if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
            errors.append(
                "insufficient remaining time for unchanged 180s case plus 20s cleanup and 5s preflight reserve"
            )
            break
        if used >= CAP:
            errors.append("aggregate 20MiB log budget exhausted")
            break
        if sha(binary) != prepared["elf_sha256"]:
            errors.append("actual original3 ELF changed")
            break
        if suite_deadline - time.monotonic() < 205 or absolute - time.time() < 205:
            errors.append("full case/cleanup reserve unavailable after ELF hash")
            break
        prefix = supplemental / f"case-{index:02}"
        receipt = owner.run(
            [
                str(binary),
                "--exact",
                row["selector"],
                "--nocapture",
                "--test-threads=1",
            ],
            cwd=Path(suite["cwd"]),
            env=environment,
            stdout_path=str(prefix) + ".stdout",
            stderr_path=str(prefix) + ".stderr",
            receipt_path=str(prefix) + ".process.json",
            artifact_path=binary,
            deadline_seconds=180,
            cap_bytes=CAP - used,
            disk_floor_bytes=FLOOR,
            disk_paths=(
                Path.cwd(),
                out,
                binary.parent,
                Path(os.environ["RUNNER_TEMP"]),
            ),
        )
        used += receipt["retained_bytes"]
        try:
            row["status"] = original_outcome(
                Path(str(prefix) + ".stdout").read_text(), receipt
            )
        except (RuntimeError, ValueError) as error:
            row["status"] = "INCOMPLETE"
            row["error"] = str(error)
        row["process"] = receipt
        row["stdout_sha256"] = sha(str(prefix) + ".stdout")
        row["stderr_sha256"] = sha(str(prefix) + ".stderr")
        save(supplemental / "outcomes.json", rows)
        print(row["group"], row["selector"], row["status"], flush=True)
        if time.monotonic() > suite_deadline or time.time() > absolute:
            errors.append("suite or absolute diagnostic deadline exceeded after case")
            break
        if (
            not receipt["group_empty_before_parent_reap"]
            or not receipt["artifact_unchanged"]
            or row["status"] == "INCOMPLETE"
        ):
            errors.append("owned process/artifact/completeness guard failed")
            break
    unchanged = sha(binary) == prepared["elf_sha256"]
    if not unchanged:
        errors.append("ELF changed after supplemental cases")
    counts = {
        group: dict(
            collections.Counter(row["status"] for row in rows if row["group"] == group)
        )
        for group in GROUPS
    }
    complete = not errors and all(
        row["status"] not in ["UNRUN", "INCOMPLETE", "TIMEOUT"] for row in rows
    )
    save(
        supplemental / "summary.json",
        {
            "source": source,
            "binary": str(binary),
            "elf_sha256": prepared["elf_sha256"],
            "elf_unchanged": unchanged,
            "counts": counts,
            "errors": errors,
            "complete": complete,
            "original3_receipts": "../terminal/summary.json and ../process-outcomes.json remain separate",
            "suite_budget_seconds": 2400,
            "case_budget_seconds": 180,
            "workflow_bound_seconds": 3600,
            "aggregate_retained_log_bytes": used,
            "aggregate_log_cap_bytes": CAP + 65536,
            "inventory_complete": {
                group: all(
                    row["status"] not in ["UNRUN", "INCOMPLETE", "TIMEOUT"]
                    for row in rows
                    if row["group"] == group
                )
                for group in GROUPS
            },
            "other103_roster_sha256": OTHER_ROSTER_SHA,
            "original_failure_log_sha256": FAILURE_LOG_SHA,
            "stack_frame_retained_log_bytes": sum(
                stream["bytes"] for stream in frame_streams.values()
            ),
            "stack_frame_accounting": "../stack-frame-accounting.json",
            "scope": "Original3 remain separate; unchangedbaseline14 and stack1 precede distinctactualother103. One authenticated numeric selector repeats original3 without identity/count merging. No baseline substitution, fullsharedprocessgate or performance acceptance. Unstarted cases stayUNRUN; samebudgets.",
        },
    )
    return (
        125
        if not complete
        else 100
        if any(row["status"] != "PASS" for row in rows)
        else 0
    )


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("phase", choices=["prepare", "frames", "run"])
    parser.add_argument("--out", type=Path, required=True)
    parser.add_argument("--sha", required=True)
    args = parser.parse_args()
    require(re.fullmatch("[0-9a-f]{40}", args.sha), "full exact source SHA required")
    require(
        os.environ.get("GITHUB_SHA") == args.sha
        and os.environ.get("GITHUB_REF") == "refs/heads/work",
        "actual WORK source required",
    )
    out = args.out.resolve()
    require(
        out.is_dir() and not out.is_relative_to(Path.cwd()),
        "existing external diagnostic evidence required",
    )
    try:
        if args.phase == "prepare":
            prepare(out, args.sha)
            return 0
        if args.phase == "frames":
            return frames(out, args.sha)
        return run(out, args.sha)
    except (
        RuntimeError,
        OSError,
        ValueError,
        KeyError,
        TypeError,
        ImportError,
        AttributeError,
    ) as error:
        save(
            out / ("supplement-" + args.phase + "-error.json"),
            {"error": repr(error), "complete": False},
        )
        if args.phase == "run":
            directory = out / "supplement"
            directory.mkdir(exist_ok=True)
            outcomes = directory / "outcomes.json"
            if outcomes.exists():
                rows = json.loads(outcomes.read_text())
            else:
                roster = json.loads(
                    (BASE / "runtime-baseline-supplement-roster.json").read_text()
                )
                roster["other103"] = json.loads(
                    (BASE / "runtime-other-failures-roster.json").read_text()
                )["other103"]
                rows = [
                    {"group": group, "selector": name, "status": "UNRUN"}
                    for group in GROUPS
                    for name in roster[group]
                ]
                save(outcomes, rows)
            save(
                directory / "summary.json",
                {
                    "source": args.sha,
                    "complete": False,
                    "errors": [repr(error)],
                    "counts": {
                        group: dict(
                            collections.Counter(
                                row["status"] for row in rows if row["group"] == group
                            )
                        )
                        for group in GROUPS
                    },
                    "scope": "Original supplemental assertions not admitted; completed process observations remain in outcomes, all unstarted cases stay UNRUN.",
                },
            )
        return 125


if __name__ == "__main__":
    raise SystemExit(main())
