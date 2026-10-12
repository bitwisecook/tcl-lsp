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

"""Check retained native source, raw line, phase, and ownership provenance."""

import collections
import hashlib
import json
from pathlib import Path

import derive


def main():
    run = Path(__file__).resolve().parent
    records = json.loads((run / "records.json").read_text())
    checked = {}
    line_cache = {}
    failures = []
    raw_rows = 0
    operation_inputs = 0
    error_stacks = 0

    def walk(value):
        if isinstance(value, dict):
            if isinstance(value.get("path"), str) and value.get("sha256"):
                path = derive.ROOT / value["path"]
                if path not in checked:
                    checked[path] = (
                        hashlib.sha256(path.read_bytes()).hexdigest()
                        if path.exists()
                        else None
                    )
                if checked[path] != value["sha256"]:
                    failures.append({"kind": "descriptor-hash", "path": value["path"]})
            for child in value.values():
                walk(child)
        elif isinstance(value, list):
            for child in value:
                walk(child)

    for index, record in enumerate(records):
        walk(record)
        source_descriptor = record["actual_input"]
        if source_descriptor:
            path = derive.ROOT / source_descriptor["path"]
            metadata_path = derive.ROOT / record["case_metadata"]["path"]
            case = json.loads(metadata_path.read_text())[
                int(record["case_metadata"]["json_pointer"].lstrip("/"))
            ]
            payload = case["operation"].encode().hex()
            if (
                payload not in path.read_text()
                and case["operation"] not in path.read_text()
            ):
                failures.append({"kind": "operation-not-in-source", "record": index})
            else:
                operation_inputs += 1
        for row in record["capture_rows"]:
            path = derive.ROOT / row["path"]
            if path not in line_cache:
                line_cache[path] = path.read_bytes().splitlines()
            actual = line_cache[path][row["line"] - 1]
            if actual.hex() != row["line_hex"]:
                failures.append({"kind": "raw-line", "record": index})
            raw_rows += 1
        if derive.complete_record(record) and record["lookup"]["completion_code"] == 1:
            stack = bytes.fromhex(record["lookup"]["errorinfo"]["hex"]).decode(
                "utf8", errors="replace"
            )
            if (
                'procedure "record"' in stack
                or "call record " in stack
                or "eval $nh_operation" not in stack
            ):
                failures.append(
                    {"kind": "error-stack-operation-boundary", "record": index}
                )
            error_stacks += 1
    guards = {}
    for case in [
        "owned-native-state-absence",
        "owned-state-seed",
        "owned-state-cleanup",
    ]:
        selected = [
            (index, record)
            for index, record in enumerate(records)
            if record["case_id"] == case and derive.complete_record(record)
        ]
        units = sorted({record["provider"]["tmm_unit"] for _, record in selected})
        if units != ["0:0", "0:1", "0:2", "0:3"]:
            failures.append({"kind": "four-unit-guard", "case": case})
        guards[case] = [
            {
                "record": index,
                "unit": record["provider"]["tmm_unit"],
                "result_hex": record["lookup"]["result_hex"],
            }
            for index, record in selected
        ]
    objects = []
    job = run / "tmm-traffic-event-0"
    for index, (kind, name) in enumerate(
        json.loads((run / "plan.json").read_text())["objects"]
    ):
        checks = {}
        for stage in ["before", "after"]:
            paths = [
                job / (stage + "-" + str(index) + "." + ext)
                for ext in ["stdout", "stderr"]
            ]
            checks[stage] = any(b"was not found" in path.read_bytes() for path in paths)
            walk([derive.descriptor(path) for path in paths])
        if not all(checks.values()):
            failures.append({"kind": "object-absence", "object": name})
        objects.append({"kind": kind, "name": name, "checks": checks})
    phases = []
    for phase in derive.PHASES:
        apply = job / (phase + "-apply.json")
        phases.append(
            {
                "phase": phase,
                "apply": json.loads(apply.read_text()) if apply.exists() else None,
                "provider": derive.descriptor(job / (phase + "-provider.stdout")),
                "attachment": derive.descriptor(job / (phase + "-attachment.stdout")),
                "diagnostic_limit": "Transport stderr was withheld; exit 1 alone does not preserve the rejected compiler or absent-object diagnostic."
                if phase in {"rejected", "deleted"}
                else None,
            }
        )
    result = {
        "review_status": "source/raw verification by preparation author; independent approval pending",
        "records": derive.descriptor(run / "records.json"),
        "unique_descriptor_files_checked": len(checked),
        "operation_source_checks": operation_inputs,
        "raw_rows_checked": raw_rows,
        "complete_error_stacks_checked": error_stacks,
        "error_stack_result": "Complete selected error stacks name the original call/eval operation and exclude the recorder callee.",
        "guards": guards,
        "objects": objects,
        "phases": phases,
        "record_states": dict(
            collections.Counter(record["status"] for record in records)
        ),
        "failures": failures,
        "limits": [
            "Missing capture identities and chunks remain missing.",
            "Native catalog retirement is not inferred from configuration deletion.",
            "Actual traffic unit 0:0 only; no cross-worker traffic inference.",
            "No original portable source promotion.",
            "Fresh observer source is bound only to its own invocation family.",
        ],
    }
    (run / "source-review.json").write_text(json.dumps(result, indent=2) + "\n")
    print(
        json.dumps(
            {
                "descriptor_files": len(checked),
                "raw_rows": raw_rows,
                "error_stacks": error_stacks,
                "failures": failures,
            }
        )
    )
    if failures:
        raise SystemExit(1)


if __name__ == "__main__":
    main()
