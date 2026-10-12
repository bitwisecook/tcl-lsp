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

"""Derive native1 observations with per-job invocation/source ownership."""

import collections
import hashlib
import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[6]
sys.path.insert(0, str(ROOT / "scripts/dev/bigip-probes/name-patterns"))
import report

PHASES = ["old", "new", "rejected", "detached", "deleted", "recreated"]


def allowed(job, invocation):
    context, suffix = job.name.rsplit("-", 1)
    number = int(suffix)
    if context == "tmm-RULE_INIT":
        if number in {0, 9}:
            return invocation == "once"
        if number == 1:
            return invocation == "load-old-init"
        return invocation == PHASES[number - 2] + "-init"
    if context == "tmm-procedure":
        if number == 0:
            return (
                invocation == "load-old-init-procedure"
                or "-init-procedure" not in invocation
            )
        return invocation == PHASES[number - 1] + "-init-procedure"
    return "-init" not in invocation and invocation != "once"


def descriptor(path):
    return {
        "path": str(path.relative_to(ROOT)),
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
    }


def complete_record(record):
    if record["status"] != "observed" or record["setup"]["completion_code"] != 0:
        return False
    fields = record["lookup"]["capture_fields"]
    if not fields or any(not field["complete"] for field in fields):
        return False
    return record["lookup"]["completion_code"] != 1 or all(
        record["lookup"][name] is not None for name in ["errorcode", "errorinfo"]
    )


def derive(run):
    original = report._capture

    def filtered(run_path, job, context, run_id, case_id, root):
        result = original(run_path, job, context, run_id, case_id, root)
        for key in ["rows", "fields"]:
            result[key] = [
                item for item in result[key] if allowed(job, item["invocation_id"])
            ]
        return result

    report._capture = filtered
    try:
        records = report.build_records(run)
    finally:
        report._capture = original
    main_job = run / "tmm-traffic-event-0"
    for record in records:
        invocation = record["invocation_id"] or ""
        if record["provider"]["context"] == "tmm-procedure" and not invocation:
            record["provider"]["event"] = None
            record["provider"]["invoking_action"] = None
            record["provider"]["configured_invoking_event"] = "RULE_INIT"
        phase = next(
            (
                phase
                for phase in PHASES
                if invocation in {phase + "-init", phase + "-init-procedure"}
            ),
            None,
        )
        if invocation.startswith(("load-old", "pair-")):
            phase = "old"
        elif invocation.startswith("phase-"):
            phase = PHASES[min(int(invocation.split("-")[1]) // 3, 5)]
        record["administrative_phase"] = phase
        record["execution_generation"] = (
            "fresh observer activation"
            if invocation.endswith(("-init", "-init-procedure"))
            and not invocation.startswith("load-")
            else "initial caller activation"
            if invocation.startswith("load-")
            else "continuing caller traffic"
            if phase
            else "explicitly unlabelled standard request or ownership control"
        )
        record["phase_evidence"] = [
            descriptor(path)
            for path in sorted(main_job.glob((phase or "NO_PHASE") + "-*.*"))
            if path.is_file()
        ]
    (run / "records.json").write_text(
        json.dumps(records, separators=(",", ":"), ensure_ascii=False) + "\n"
    )
    counts = collections.defaultdict(collections.Counter)
    selected = []
    identity_set = set()
    for index, record in enumerate(records):
        provider = record["provider"]
        identity = (
            record["case_id"],
            provider["context"],
            provider["tmm_unit"],
            record["invocation_id"],
        )
        if identity in identity_set and record["invocation_id"] is not None:
            raise ValueError("Invocation assigned to two jobs: " + str(identity))
        identity_set.add(identity)
        counts[(record["question_id"], provider["context"])][record["status"]] += 1
        if not complete_record(record):
            continue
        invocation = record["invocation_id"]
        # Bound representatives by actual operation/event/phase, retaining both
        # init-called and HTTP-called native procedure scopes.
        if "-init" not in invocation and not invocation.startswith(("phase-", "pair-")):
            continue
        phase = "representative"
        if record["case_id"] in {
            "provider-lifetime",
            "provider-shared-write-readback",
            "connection-identity-write-read",
        }:
            phase = (
                invocation.split("-init", 1)[0]
                if "-init" in invocation
                else invocation
                if invocation.startswith("pair-")
                else PHASES[min(int(invocation.split("-")[1]) // 3, 5)]
            )
        key = (record["case_id"], provider["context"], provider["event"], phase)
        if any(item["selection_key"] == list(key) for item in selected):
            continue
        selected.append(
            {
                "selection_key": list(key),
                "record_index": index,
                "case_id": record["case_id"],
                "question_id": record["question_id"],
                "context": provider["context"],
                "event": provider["event"],
                "unit": provider["tmm_unit"],
                "invocation": invocation,
                "source": record["actual_input"],
                "configuration": record["configuration"],
                "lookup": record["lookup"],
            }
        )
    (run / "candidate-proofs.json").write_text(
        json.dumps(
            {
                "review_status": "candidate; independent approval pending",
                "input_scope": "new native materialisations only",
                "records": descriptor(run / "records.json"),
                "candidates": selected,
            },
            indent=2,
        )
        + "\n"
    )
    expected = []
    metadata = {
        context: json.loads((run / (context + "-cases.json")).read_text())
        for context in ["tmm-RULE_INIT", "tmm-procedure", "tmm-traffic-event"]
    }
    for context in ["tmm-RULE_INIT", "tmm-procedure"]:
        invocations = (
            ["load-old-init", *[p + "-init" for p in PHASES]]
            if context == "tmm-RULE_INIT"
            else ["load-old-init-procedure", *[p + "-init-procedure" for p in PHASES]]
        )
        for case in metadata[context]:
            if case["id"].startswith("owned-"):
                continue
            for invocation in invocations:
                for unit in ["0:0", "0:1", "0:2", "0:3"]:
                    match = [
                        record
                        for record in records
                        if record["case_id"] == case["id"]
                        and record["provider"]["context"] == context
                        and record["provider"]["tmm_unit"] == unit
                        and record["invocation_id"] == invocation
                    ]
                    valid = bool(match) and complete_record(match[0])
                    expected.append(
                        {
                            "case_id": case["id"],
                            "context": context,
                            "unit": unit,
                            "invocation": invocation,
                            "complete_measured_setup_and_lookup": valid,
                            "record_index": records.index(match[0]) if match else None,
                            "missing_reason": None
                            if valid
                            else "No complete setup/result/error capture for this expected identity; missing logs are not an operation result.",
                        }
                    )
    (run / "expected-observations.json").write_text(
        json.dumps(expected, indent=2) + "\n"
    )
    lines = [
        "# Native question measurements: " + run.name,
        "",
        "These are new exact owned native materialisations, not portable-source answers. Per-job invocation filtering binds each measurement to its actual source/configuration. Complete bytes, raw lines and all invocation inventories remain in [records.json](records.json). [Expected initial identities](expected-observations.json) explicitly retain missing capture boundaries. Candidate proofs require independent review; this report is unsealed.",
        "",
        "| Question | Context | Observation states |",
        "| --- | --- | --- |",
    ]
    for (question, context), values in sorted(counts.items()):
        lines.append(
            "| "
            + question
            + " | "
            + context
            + " | "
            + "; ".join(str(n) + " " + state for state, n in sorted(values.items()))
            + " |"
        )
    lines += [
        "",
        "## Complete representative receipts",
        "",
        "A representative covers its specific observed unit/invocation. Missing other units are not inferred. Fresh init-called procedure and HTTP-called procedure receipts remain separate. Numeric standard-controller requests are explicitly unlabelled by phase; ordered proofs use phase-* or pair-* identities.",
        "",
        "| Case | Context/event | Unit/invocation | Completion | Result encoding / hex | Record |",
        "| --- | --- | --- | --- | --- | --- |",
    ]
    for item in selected:
        lookup = item["lookup"]
        lines.append(
            "| "
            + item["case_id"]
            + " | "
            + item["context"]
            + "/"
            + str(item["event"])
            + " | "
            + str(item["unit"])
            + "/"
            + item["invocation"]
            + " | "
            + str(lookup["completion_code"])
            + " | "
            + str(lookup["result_encoding"])
            + " / `"
            + str(lookup["result_hex"])
            + "` | [records.json](records.json) `/"
            + str(item["record_index"])
            + "` |"
        )
    lines += [
        "",
        "Provider, loaded source, exact configuration, metadata, raw line/chunk and configuration cleanup descriptors are retained per record. Full return options are unavailable on these contexts. Captured global errorcode/errorinfo fields require the source/error-stack review in README; unavailable or incomplete fields are not promoted.",
    ]
    (run / "ANSWERS.md").write_text("\n".join(lines) + "\n")
    return records, selected, expected


if __name__ == "__main__":
    records, selected, expected = derive(Path(__file__).resolve().parent)
    print(
        json.dumps(
            {
                "records": len(records),
                "candidate_proofs": len(selected),
                "expected_init_identities": len(expected),
                "complete_expected_init_identities": sum(
                    item["complete_measured_setup_and_lookup"] for item in expected
                ),
            }
        )
    )
