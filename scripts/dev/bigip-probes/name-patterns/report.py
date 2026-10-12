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
"""Describe retained naming captures without deploying or promoting evidence.

build_records reads run artefacts only. write_report writes records.json and
ANSWERS.md in the supplied run; it never changes question or proof manifests.
Repeated chunks stay unjoined and no runtime error implies unsupported status.
"""

import argparse
from collections import defaultdict
import hashlib
import json
from pathlib import Path
import re

from parse_capture import parse_capture


ROOT = Path(__file__).resolve().parents[4]
SCRIPT_LOG_CONTEXTS = {"iapp-implementation", "icall", "iapp-apl"}


def _json(path, default=None):
    return json.loads(path.read_text(encoding="utf-8")) if path.exists() else default


def _text(path):
    return path.read_text(encoding="utf-8", errors="replace") if path.exists() else ""


def _exit(path):
    text = _text(path).strip()
    return int(text) if re.fullmatch(r"-?[0-9]+", text) else None


def _descriptor(path, root, role=None):
    if not path.exists():
        return None
    result = {
        "path": path.resolve().relative_to(root).as_posix(),
        "sha256": hashlib.sha256(path.read_bytes()).hexdigest(),
    }
    if role:
        result["role"] = role
    return result


def _evidence(paths, root, role=None):
    return [item for path in paths if (item := _descriptor(path, root, role))]


def _provider(run, context, root):
    version = _text(run / "version.stdout")
    details = {}
    for key, label in [
        ("version", "Version"),
        ("build", "Build"),
        ("hotfix", "Hotfix"),
        ("edition", "Edition"),
    ]:
        match = re.search(r"^\s*" + label + r"\s+([^\r\n]+)", version, re.MULTILINE)
        details[key] = match[1].strip() if match else None
    details.update(
        {
            "context": context,
            "event": "HTTP_REQUEST"
            if context == "tmm-traffic-event"
            else "RULE_INIT"
            if context.startswith("tmm-")
            else None,
            "tmm_unit": None,
            "execution_namespace": None,
            "interpreter_path": "/usr/bin/tclsh" + context.removeprefix("stock-")
            if context.startswith("stock-")
            else None,
            "interpreter_patchlevel": None,
            "evidence": _evidence(
                [
                    run / name
                    for name in [
                        "version.stdout",
                        "platform.stdout",
                        "tmm-roster.stdout",
                        "failover.stdout",
                        "provenance.json",
                    ]
                ],
                root,
                "provider",
            ),
        }
    )
    return details


def _load(job, context, root):
    if context.startswith("stock-"):
        status = "not-applicable"
    else:
        code = _exit(job / "load.exit")
        output = _text(job / "load.stdout") + _text(job / "load.stderr")
        status = (
            "not-tested"
            if code is None
            else "warn"
            if code == 0 and re.search(r"warning", output, re.I)
            else "accept"
            if code == 0
            else "reject"
            if re.search(r"error|invalid|disabled|undefined|syntax", output, re.I)
            else "infrastructure-failure"
        )
    return {
        "status": status,
        "exit_code": _exit(job / "load.exit"),
        "transcript": _descriptor(job / "load.stdout", root),
        "stderr": _descriptor(job / "load.stderr", root),
    }


def _jobs(run, context):
    pattern = re.compile(re.escape(context) + r"-[0-9]+$")
    return sorted(
        (p for p in run.iterdir() if p.is_dir() and pattern.fullmatch(p.name)),
        key=lambda p: int(p.name.rsplit("-", 1)[1]),
    )


def _job_for(run, context, case_id):
    marker = "|" + case_id + "|" + context + "|"
    return [job for job in _jobs(run, context) if marker in _text(job / "input.tcl")]


def _capture(run, job, context, run_id, case_id, root):
    name = (
        "runtime.stdout"
        if context.startswith("stock-")
        else "invoke.stdout"
        if context == "tmsh-cli-script"
        else "runtime-logs.stdout"
    )
    ordinary = job / name
    final = run / "final-runtime.stdout"
    candidates = (
        [final, ordinary]
        if final.exists()
        and (context.startswith("tmm-") or context in SCRIPT_LOG_CONTEXTS)
        else [ordinary]
    )
    selected = None
    for path in candidates:
        if not path.exists():
            continue
        parsed = parse_capture([path], context, run_id, root)
        parsed["rows"] = [row for row in parsed["rows"] if row["case_id"] == case_id]
        parsed["fields"] = [
            field for field in parsed["fields"] if field["case_id"] == case_id
        ]
        # Original field row_indices still identify the full parser input. Raw
        # rows are retained individually, so callers need not infer positions.
        for field in parsed["fields"]:
            field.pop("row_indices", None)
        parsed["rejected_rows"] = [
            row
            for row in parsed["rejected_rows"]
            if marker_bytes(run_id, case_id, context) in bytes.fromhex(row["line_hex"])
        ]
        selected = parsed
        if parsed["rows"]:
            break
    return selected or {"rows": [], "fields": [], "sources": [], "rejected_rows": []}


def marker_bytes(run_id, case_id, context):
    return ("NHO|" + run_id + "|" + case_id + "|" + context + "|").encode("ascii")


def _stage(fields):
    by_name = {field["field"]: field for field in fields}
    value = by_name.get("result")
    metadata = by_name.get("metadata")
    metadata_text = metadata.get("text") if metadata and metadata["complete"] else None
    unavailable = []
    options_field = by_name.get("returnoptions") or by_name.get("return_options")
    options_availability = re.search(r"return-options ([^\s]+)", metadata_text or "")
    return_options = None
    if (
        options_field
        and options_field["complete"]
        and (not options_availability or options_availability[1] == "captured")
    ):
        return_options = {
            "encoding": options_field["encoding"],
            "hex": options_field["value_hex"],
        }
    else:
        unavailable.append("return_options")
    errors = {}
    for name in ["errorcode", "errorinfo"]:
        field = by_name.get(name)
        present = re.search(name + r"-present ([01])", metadata_text or "")
        if field and field["complete"] and present and present[1] == "1":
            errors[name] = {"encoding": field["encoding"], "hex": field["value_hex"]}
        else:
            errors[name] = None
            unavailable.append(name)
    if not value or not value["complete"]:
        unavailable.append("result_hex")
    return {
        "reached": True if value else None,
        "completion_code": value["completion_code"] if value else None,
        "result_encoding": value["encoding"] if value else None,
        "result_hex": value["value_hex"] if value and value["complete"] else None,
        **errors,
        "return_options": return_options,
        "return_options_availability": options_availability[1]
        if options_availability
        else "captured-field"
        if return_options
        else "unobserved",
        "unavailable_fields": unavailable,
        "capture_fields": fields,
    }


def _no_stage(reached=None):
    return {
        "reached": reached,
        "completion_code": None,
        "result_encoding": None,
        "result_hex": None,
        "errorcode": None,
        "errorinfo": None,
        "return_options": None,
        "unavailable_fields": [
            "completion_code",
            "result_hex",
            "errorcode",
            "errorinfo",
            "return_options",
        ],
        "capture_fields": [],
    }


def _cleanup(job, fields, root):
    files = sorted(
        p
        for p in job.iterdir()
        if p.is_file()
        and (
            p.name.startswith(("before-", "delete-", "after-"))
            or p.name == "save-status.txt"
        )
    )
    proofs = []
    for path in sorted(job.glob("after-*.exit")):
        stem = path.stem
        output = _text(job / (stem + ".stdout")) + _text(job / (stem + ".stderr"))
        absent = (
            True if "was not found" in output else False if _exit(path) == 0 else None
        )
        proofs.append({"check": stem, "exit_code": _exit(path), "absent": absent})
    return {
        "evidence": _evidence(files, root),
        "object_absence_checks": proofs,
        "runtime": _stage(fields) if fields else _no_stage(),
    }


def _case_metadata(run):
    found = []
    for path in sorted(run.glob("*-cases.json")):
        context = path.name.removesuffix("-cases.json")
        for index, case in enumerate(_json(path, [])):
            found.append((context, case, path, index))
    if not found and (run / "capabilities.json").exists():
        path = run / "capabilities.json"
        contexts = sorted(
            {
                p.name.rsplit("-", 1)[0]
                for p in run.iterdir()
                if p.is_dir() and re.search(r"-[0-9]+$", p.name)
            }
        )
        for context in contexts:
            for index, case in enumerate(_json(path, [])):
                found.append((context, case, path, index))
    return found


def build_records(run_path, root=ROOT):
    """Read per-job evidence and return per-unit/invocation measurement records."""
    root = Path(root).resolve()
    run = Path(run_path).resolve()
    provenance = _json(run / "provenance.json", {})
    run_id = provenance.get("run_id", run.name)
    records = []
    for context, case, metadata_path, metadata_index in _case_metadata(run):
        case_id = case["id"]
        jobs = _job_for(run, context, case_id)
        # Missing controller/job evidence is retained rather than an invented
        # runtime failure. The unresolved job location has no input authority.
        for job in jobs or [None]:
            capture = (
                _capture(run, job, context, run_id, case_id, root)
                if job
                else {"rows": [], "fields": [], "sources": [], "rejected_rows": []}
            )
            identities = sorted(
                {
                    (field["tmm_unit"], field["invocation_id"])
                    for field in capture["fields"]
                },
                key=str,
            )
            for unit, invocation in identities or [(None, None)]:
                fields = [
                    field
                    for field in capture["fields"]
                    if (field["tmm_unit"], field["invocation_id"]) == (unit, invocation)
                ]
                stages = defaultdict(list)
                for field in fields:
                    stages[field["stage"]].append(field)
                setup = _stage(stages["setup"]) if stages["setup"] else _no_stage()
                lookup_stage = (
                    "body-return"
                    if stages["body-return"] and case_id.startswith("wiki-")
                    else "lookup"
                )
                lookup = (
                    _stage(stages[lookup_stage])
                    if stages[lookup_stage]
                    else _no_stage()
                )
                provider = _provider(run, context, root)
                provider["tmm_unit"] = None if unit in {None, "none"} else unit
                configuration = _text(job / "applied.conf") if job else ""
                provider["configured_rule_names"] = re.findall(
                    r"^ltm rule ([^\s{]+)", configuration, re.MULTILINE
                )
                provider["configured_profiles"] = (
                    ["tcp", "http"]
                    if context == "tmm-traffic-event"
                    and "profiles { tcp" in configuration
                    else []
                )
                provider["invoking_action"] = (
                    "definition"
                    if context in {"iapp-implementation", "iapp-apl"}
                    else "tmsh run cli script"
                    if context == "tmsh-cli-script"
                    else "call measure from RULE_INIT"
                    if context == "tmm-procedure"
                    else None
                )
                event_name = re.search(r"event-name ([^\s}]+)", configuration)
                if context == "icall" and event_name:
                    provider["configured_event_name"] = event_name[1]
                provider["applied_object_receipts"] = (
                    _evidence(sorted(job.glob("applied-*.*")), root, "provider")
                    if job
                    else []
                )
                raw_output = (
                    _text(job / "runtime.stdout")
                    if job and context.startswith("stock-")
                    else ""
                )
                patch = re.search(
                    r"^oracle ([^ ]+) library (.+?) tcltest ([^\r\n]+)",
                    raw_output,
                    re.MULTILINE,
                )
                if patch:
                    provider.update(
                        {
                            "interpreter_patchlevel": patch[1],
                            "tcl_library": patch[2],
                            "tcltest_version": patch[3],
                        }
                    )
                load = (
                    _load(job, context, root)
                    if job
                    else {"status": "not-tested", "transcript": None}
                )
                status = (
                    "observed"
                    if lookup["reached"]
                    else "unreached"
                    if setup["reached"]
                    else "unavailable"
                    if context == "iapp-apl"
                    else "infrastructure-failure"
                )
                reason = None
                if job is None:
                    reason = "No retained job/input matched this case and context."
                elif load["status"] == "reject":
                    status = "unreached"
                    lookup["reached"] = False
                    reason = "Configuration load rejected; this is not an intended runtime lookup result."
                elif load["status"] == "infrastructure-failure":
                    status = "infrastructure-failure"
                    reason = "Load exited unsuccessfully without a recognised compiler-rejection diagnostic."
                elif case_id.startswith("wiki-"):
                    skipped = re.search(
                        r"\+\+\+\+ " + re.escape(case_id) + r" SKIPPED:([^\r\n]*)",
                        raw_output,
                    )
                    passed = re.search(
                        r"\+\+\+\+ " + re.escape(case_id) + r" PASSED", raw_output
                    )
                    if skipped:
                        status = "unreached"
                        lookup = _no_stage(False)
                        reason = (
                            "Real tcltest skipped selected test: " + skipped[1].strip()
                        )
                    elif lookup_stage != "body-return":
                        suite_lookup = lookup
                        lookup = _no_stage(
                            False
                            if setup.get("completion_code") not in {None, 0}
                            else None
                        )
                        status = (
                            "unavailable"
                            if passed
                            else "unreached"
                            if lookup["reached"] is False
                            else "infrastructure-failure"
                        )
                        reason = "Observed harness completion is retained separately; no selected-test body-return capture exists. Passing test output does not supply an authoritative reached-body record or exact body result bytes/error options."
                        stages["suite-lookup"] = suite_lookup["capture_fields"]
                    elif (case.get("adaptation") or {}).get(
                        "kind"
                    ) == "execution-trace-observer":
                        lookup["errorcode"] = None
                        lookup["errorinfo"] = None
                        lookup["return_options"] = None
                        lookup["return_options_availability"] = (
                            "child-inner-unavailable"
                        )
                        lookup["unavailable_fields"] = sorted(
                            set(
                                lookup["unavailable_fields"]
                                + ["errorcode", "errorinfo", "return_options"]
                            )
                        )
                        reason = "Observer captures isolated leave completion/result; child inner error options are unavailable."
                if (
                    setup.get("completion_code") not in {None, 0}
                    and not stages[lookup_stage]
                ):
                    lookup["reached"] = False
                    status = "unreached"
                    reason = "Setup failed before the intended operation."
                if context == "iapp-apl" and not capture["rows"]:
                    status = (
                        "unavailable" if load["status"] != "reject" else "unreached"
                    )
                    reason = "No presentation callback capture exists; application implementation execution does not establish APL callback reachability."
                descriptor = _descriptor(metadata_path, root)
                descriptor["json_pointer"] = "/" + str(metadata_index)
                original = case.get("original_input")
                cleanup = (
                    _cleanup(job, stages["runtime-cleanup"], root)
                    if job
                    else {
                        "evidence": [],
                        "object_absence_checks": [],
                        "runtime": _no_stage(),
                    }
                )
                raw_paths = (
                    [
                        job / name
                        for name in [
                            "load.stdout",
                            "load.stderr",
                            "invoke.stdout",
                            "invoke.stderr",
                            "runtime.stdout",
                            "runtime.stderr",
                            "invocation.json",
                            "invocation.sh",
                            "driver.json",
                            "driver.stdout",
                            "driver.stderr",
                            "transport.stderr",
                        ]
                    ]
                    if job
                    else []
                )
                saved_text = _text(job / "save-status.txt") if job else ""
                saved = (
                    False
                    if "configuration_saved=false" in saved_text
                    else True
                    if "configuration_saved=true" in saved_text
                    else provenance.get("configuration_saved")
                )
                whole_program_error = (
                    case.get("variant_id") == "whole-portable-source-v1"
                    and lookup["completion_code"] == 1
                )
                intended_lookup_reached = lookup["reached"]
                question_id = case.get("question_id", "F5-N01-" + case_id)
                canonical_id = case.get("canonical_question_id")
                canonical_question = None
                scope = case.get("scope")
                if whole_program_error:
                    intended_lookup_reached = None
                    result_text = None
                    if (
                        lookup["result_encoding"] == "utf-8"
                        and lookup["result_hex"] is not None
                    ):
                        try:
                            result_text = bytes.fromhex(lookup["result_hex"]).decode(
                                "utf-8"
                            )
                        except UnicodeError:
                            pass
                    if (
                        case_id == "ensemble-prefix-map"
                        and re.search(
                            r'bad (?:option|subcommand) "ensemble"|unknown.*ensemble',
                            result_text or "",
                            re.I,
                        )
                    ) or (
                        case_id.startswith("oo-")
                        and re.search(
                            r'invalid command name "(?:::)?oo::class"',
                            result_text or "",
                        )
                    ):
                        intended_lookup_reached = False
                    question_id += "-whole-program-evaluation"
                    canonical_question = (
                        "What completion code and result bytes did evaluating the entire retained "
                        + case_id
                        + " program produce in this recorded context/invocation? "
                        "This question does not establish successful case setup or final intended lookup."
                    )
                    canonical_id = None
                    scope = "Entire-program evaluation stage only; an empty capture setup is not evidence that the program's own setup succeeded. Final intended lookup reachability is recorded separately."
                    reason = (
                        (reason + " " if reason else "")
                        + "Observed code 1 belongs to evaluation of the entire portable program. "
                        "It is not an accepted answer to its canonical final-lookup question."
                    )
                records.append(
                    {
                        "schema_version": 1,
                        "question_id": question_id,
                        "canonical_question_id": canonical_id,
                        "canonical_question": canonical_question,
                        "requested_canonical_question_id": case.get(
                            "canonical_question_id"
                        ),
                        "scope": scope,
                        "operation_scope": "entire-program"
                        if whole_program_error
                        else "captured-operation",
                        "intended_lookup_reached": intended_lookup_reached,
                        "canonical_lookup_answer": False
                        if whole_program_error
                        else None,
                        "case_id": case.get("case_id", case_id),
                        "variant_id": case.get(
                            "variant_id", "capability-runtime-control-v1"
                        ),
                        "run_id": run_id,
                        "invocation_id": invocation,
                        "provider": provider,
                        "original_input": original,
                        "actual_input": _descriptor(job / "input.tcl", root)
                        if job
                        else None,
                        "wrapper": _descriptor(job / "applied.conf", root)
                        or _descriptor(job / "input.tcl", root)
                        if job
                        else None,
                        "case_metadata": descriptor,
                        "configuration": _descriptor(job / "applied.conf", root)
                        if job
                        else None,
                        "invocation_evidence": _evidence(
                            [
                                job / "invocation.sh",
                                job / "invocation.json",
                                job / "invoke.stdout",
                                job / "invoke.stderr",
                            ],
                            root,
                        )
                        if job
                        else [],
                        "adaptation": case.get("adaptation"),
                        "load": load,
                        "setup": setup,
                        "lookup": lookup,
                        "lookup_stage": lookup_stage,
                        "status": status,
                        "reason": reason,
                        "raw_evidence": capture["sources"]
                        + _evidence(raw_paths, root)
                        + provider["evidence"]
                        + provider["applied_object_receipts"],
                        "capture_rows": [
                            row
                            for row in capture["rows"]
                            if (row["tmm_unit"], row["invocation_id"])
                            == (unit, invocation)
                        ],
                        "rejected_capture_rows": capture["rejected_rows"],
                        "other_stages": dict(stages),
                        "cleanup_evidence": cleanup["evidence"],
                        "cleanup": cleanup,
                        "configuration_saved": saved,
                        "limitations": "V1 captures have no invocation identity. Duplicate or missing chunks remain unjoined. No error is classified unsupported automatically; no unobserved setup, lookup, unit or cleanup is inferred.",
                    }
                )
    return records


def _cell(value):
    return (
        str(value).replace("|", "\\|").replace("\n", " ")
        if value is not None
        else "unobserved"
    )


def _link(descriptor):
    if not descriptor:
        return "unretained"
    return (
        "["
        + descriptor["path"]
        + "](/"
        + str(ROOT).lstrip("/")
        + "/"
        + descriptor["path"]
        + ") SHA-256 `"
        + descriptor["sha256"]
        + "`"
    )


def write_report(run_path):
    """Write derived records and a reviewable Markdown report, without promotion."""
    run = Path(run_path).resolve()
    records = build_records(run)
    (run / "records.json").write_text(
        json.dumps(records, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
    )
    lines = [
        "# Retained naming measurements: " + run.name,
        "",
        "This derived report describes recorded case/context/unit/invocation evidence. It does not promote manifest answers or infer unavailable contexts. Byte fields are hex with their recorded encoding; duplicate chunks remain unjoined.",
        "",
        "| Case / variant | Context / unit / invocation | Load | Setup code | Intended operation | Runtime code | Encoding / exact result hex | Error availability | Status |",
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- |",
    ]
    for record in records:
        lookup = record["lookup"]
        provider = record["provider"]
        lines.append(
            "| "
            + " | ".join(
                _cell(value)
                for value in [
                    record["case_id"] + " / " + record["variant_id"],
                    provider["context"]
                    + " / "
                    + str(provider["tmm_unit"])
                    + " / "
                    + str(record["invocation_id"]),
                    record["load"]["status"],
                    record["setup"]["completion_code"],
                    lookup["reached"],
                    lookup["completion_code"],
                    str(lookup["result_encoding"])
                    + " / `"
                    + str(lookup["result_hex"])
                    + "`",
                    ", ".join(lookup["unavailable_fields"]),
                    record["status"],
                ]
            )
            + " |"
        )
    lines.extend(["", "## Inputs and evidence", ""])
    for index, record in enumerate(records):
        provider = record["provider"]
        lines.extend(
            [
                "### Record "
                + str(index)
                + ": "
                + record["case_id"]
                + " / "
                + provider["context"],
                "",
                "Provider: "
                + _cell(provider["version"])
                + ", build "
                + _cell(provider["build"])
                + ", edition "
                + _cell(provider["edition"])
                + ", hotfix "
                + _cell(provider["hotfix"])
                + ". Event "
                + _cell(provider["event"])
                + "; unit "
                + _cell(provider["tmm_unit"])
                + "; invocation "
                + _cell(record["invocation_id"])
                + ".",
                "",
                "Original: "
                + _link(record["original_input"])
                + ". Actual input: "
                + _link(record["actual_input"])
                + ". Wrapper: "
                + _link(record["wrapper"])
                + ".",
                "",
                "Case metadata: "
                + _link(record["case_metadata"])
                + ", JSON pointer `"
                + record["case_metadata"]["json_pointer"]
                + "`. Measurement: [records.json](records.json), JSON pointer `/"
                + str(index)
                + "`.",
                "",
            ]
        )
        if record["reason"]:
            lines.extend([record["reason"], ""])
        lines.extend(
            [
                "Raw/provider evidence: "
                + "; ".join(_link(item) for item in record["raw_evidence"])
                + ".",
                "",
                "Cleanup: "
                + "; ".join(_link(item) for item in record["cleanup_evidence"])
                + ". Runtime cleanup reached: "
                + _cell(record["cleanup"]["runtime"]["reached"])
                + ". Configuration saved: "
                + _cell(record["configuration_saved"])
                + ".",
                "",
            ]
        )
    (run / "ANSWERS.md").write_text("\n".join(lines), encoding="utf-8")
    return records


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("run_path", type=Path)
    args = parser.parse_args()
    write_report(args.run_path)


if __name__ == "__main__":
    main()
