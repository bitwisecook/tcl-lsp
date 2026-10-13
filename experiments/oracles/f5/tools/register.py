#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Prepare reviewed, narrowly scoped F5 proof records without changing the manifest."""

import argparse
import hashlib
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
sys.path.insert(0, str(ROOT / "scripts/dev"))
from oracle_store import retained_file

DIRECTORY = "docs/design/analysis/name-resolution-proofs"
PROVIDERS = ("tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim", "bigip")


def _slug(value):
    result = re.sub(r"[^a-z0-9._-]+", "-", str(value).lower()).strip(".-_")
    if not result:
        raise ValueError("an identity component is empty")
    return result


def make_proof(
    measurement,
    root=ROOT,
    question=None,
    identifier=None,
    extra_paths=(),
    record_path=None,
    record_pointer=None,
):
    """Hash reviewed reached observations; unavailable/unreached setup is not promoted."""
    root = Path(root).resolve()
    status = measurement["status"]
    setup = measurement["setup"]
    lookup = measurement["lookup"]
    if status not in {"observed", "unsupported"}:
        raise ValueError("only reviewed observed/unsupported records can be registered")
    if measurement.get("load", {}).get("status") not in {
        "accept",
        "warn",
        "not-applicable",
    }:
        raise ValueError("a rejected or unverified load cannot supply a lookup answer")
    if (
        setup.get("reached") is not True
        or type(setup.get("completion_code")) is not int
    ):
        raise ValueError("setup requires an explicitly reached integer completion")
    if setup["completion_code"] != 0 or lookup.get("reached") is not True:
        raise ValueError(
            "failed setup or unreached lookup cannot supply a proof answer"
        )
    if type(lookup.get("completion_code")) is not int or lookup["completion_code"] < 0:
        raise ValueError("lookup requires its actual integer completion code")
    result_hex = lookup.get("result_hex")
    encoding = lookup.get("result_encoding")
    if not isinstance(result_hex, str) or not re.fullmatch(
        r"(?:[0-9a-f]{2})*", result_hex
    ):
        raise ValueError("lookup requires exact result hex")
    if encoding not in {"utf-8", "binary-scan-tcl-value"}:
        raise ValueError("lookup requires its recorded result encoding")
    provider = measurement["provider"]
    context = provider["context"]
    for key in ("version", "build", "context"):
        if not isinstance(provider.get(key), str) or not provider[key].strip():
            raise ValueError(f"provider {key} must be explicit")
    run = measurement["run_id"]
    case = measurement.get("case_id") or measurement["question_id"]
    variant = measurement["variant_id"]
    unit = provider.get("tmm_unit")
    invocation = measurement.get("invocation_id")
    if context.startswith("tmm-") and measurement["load"]["status"] == "not-applicable":
        raise ValueError("TMM records require an actual accepted load")
    if context.startswith("tmm-") and unit in {None, ""}:
        raise ValueError("TMM proof requires the actual responding unit")
    if context == "tmm-traffic-event" and invocation in {None, ""}:
        raise ValueError("traffic proof requires a recorded invocation identifier")
    suffix = ".".join(_slug(value) for value in (case, context, variant, run))
    if unit is not None:
        suffix += ".unit-" + _slug(unit)
    if invocation is not None:
        suffix += ".invocation-" + _slug(invocation)
    identifier = identifier or "naming.bigip-pattern." + suffix
    if not re.fullmatch(r"naming\.[a-z0-9][a-z0-9._-]+", identifier):
        raise ValueError("invalid central proof identifier")
    document = DIRECTORY + "/" + identifier.removeprefix("naming.") + ".md"
    evidence = []

    def attach(label, role, supplied, description, pointer=None):
        descriptor = {"path": supplied} if isinstance(supplied, str) else supplied
        name = descriptor["path"]
        relative = Path(name)
        path = (root / relative).resolve()
        if (
            relative.is_absolute()
            or ".." in relative.parts
            or not path.is_relative_to(root)
        ):
            raise ValueError(f"evidence path escapes repository: {name}")
        sha = hashlib.sha256(retained_file(root, name).read_bytes()).hexdigest()
        if descriptor.get("sha256") is not None and descriptor["sha256"] != sha:
            raise ValueError(f"retained evidence digest changed: {name}")
        item = {
            "id": label,
            "role": role,
            "path": relative.as_posix(),
            "sha256": sha,
            "description": description,
        }
        if role not in {
            "input",
            "observation",
            "provider",
            "source-anchor",
            "implementation",
            "reconfirmation",
            "limitation",
        }:
            raise ValueError(f"invalid evidence role: {role}")
        selector = pointer if pointer is not None else descriptor.get("json_pointer")
        if selector is not None:
            item["json_pointer"] = selector
        evidence.append(item)

    for key in ("original_input", "actual_input", "wrapper"):
        attach(
            key.replace("_", "-"),
            "input",
            measurement[key],
            f"Complete retained {key} bytes.",
        )
    raw = measurement.get("raw_evidence", [])
    if not raw:
        raise ValueError("a reached proof requires retained raw capture evidence")
    for index, item in enumerate(raw):
        role = (
            item.get("role", "observation") if isinstance(item, dict) else "observation"
        )
        attach(
            f"raw-{index}", role, item, "Exact retained capture or provider receipt."
        )
    for index, item in enumerate(measurement.get("cleanup_evidence", [])):
        attach(
            f"cleanup-{index}",
            "observation",
            item,
            "Retained cleanup result; scope remains explicit.",
        )
    for index, item in enumerate(extra_paths):
        attach(
            f"extra-{index}",
            "provider",
            item,
            "Independent retained provider or run provenance.",
        )
    if record_path is not None:
        attach(
            "measurement",
            "observation",
            str(record_path),
            "Reviewed measurement record.",
            record_pointer,
        )
    qid = measurement.get("canonical_question_id") or measurement["question_id"]
    adaptation = measurement.get("adaptation")
    scope = (
        f"Only retained case {case}, variant {variant}, run {run}, context {context}; "
        f"event {provider.get('event')!r}, unit {unit!r}, invocation {invocation!r}. "
        "This independently scoped input does not replace an exact portable-input answer "
        "or establish other contexts, units, versions, recogniser coverage or source mechanisms."
    )
    scope += " Authored measurement scope: " + str(measurement.get("scope")) + ". "
    scope += (
        "Interpreter/action facts retained in the measurement: "
        + json.dumps(
            {
                key: provider.get(key)
                for key in (
                    "interpreter_path",
                    "interpreter_patchlevel",
                    "tcltest_version",
                    "execution_namespace",
                    "invoking_action",
                )
            },
            ensure_ascii=False,
        )
        + ". Null means not observed by this receipt. "
    )
    if adaptation is not None:
        scope += " Recorded adaptation: " + json.dumps(adaptation, ensure_ascii=False)
    result = (
        f"Setup reached with completion 0. Intended lookup reached with completion "
        f"{lookup['completion_code']}; result encoding {encoding}; exact result hex "
        f"{json.dumps(result_hex)}. Error metadata and availability: "
        + json.dumps(
            {
                key: lookup.get(key)
                for key in (
                    "errorcode",
                    "errorinfo",
                    "return_options",
                    "unavailable_fields",
                )
            },
            ensure_ascii=False,
        )
        + ". Only this recorded invocation is answered."
    )
    providers = []
    for name in PROVIDERS:
        providers.append(
            {
                "id": name,
                "version": "not recorded",
                "build": "not recorded",
                "input_channel": "not recorded",
                "dialect": "Jim Tcl"
                if name == "jim"
                else "F5 hosted Tcl"
                if name == "bigip"
                else "Tcl",
                "status": "not-tested",
                "answer": "No execution of this exact context/input/invocation is claimed for this provider.",
                "evidence": [],
            }
        )
    providers[-1].update(
        {
            "version": provider["version"],
            "build": provider["build"] + "; hotfix " + str(provider.get("hotfix")),
            "input_channel": scope,
            "status": status,
            "answer": result,
            "evidence": [item["id"] for item in evidence],
        }
    )
    return {
        "id": identifier,
        "kind": "native-observation",
        "document": document,
        "problem_statement": (
            f"Related question {qid}; this record retains one reviewed provider/input/context "
            "observation independently of the original C/Jim and exact portable evidence."
        ),
        "question": question
        or (
            f"What completion and result did retained variant {variant} of case {case} "
            f"produce in {context}, run {run}, unit {unit!r}, invocation {invocation!r}?"
        ),
        "conclusion": result,
        "scope": scope,
        "providers": providers,
        "evidence": evidence,
        "source_anchors": [],
        "tests": [],
        "owners": [],
        "replay": {
            "command": measurement.get("replay_command", []),
            "limitations": (
                "Replay requires the retained exact inputs, isolated appliance objects, "
                "recorded invoking context and independently captured load, runtime and "
                "cleanup outcomes. This registration performs no appliance execution."
            ),
        },
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("records", type=Path)
    parser.add_argument("--root", type=Path, default=ROOT)
    parser.add_argument("--output", required=True, type=Path)
    parser.add_argument("--extra", action="append", default=[])
    parser.add_argument("--reviewed", action="store_true", required=True)
    args = parser.parse_args()
    root = args.root.resolve()
    supplied = json.loads(args.records.read_text())
    rows = supplied if isinstance(supplied, list) else supplied["records"]
    record_path = args.records.resolve().relative_to(root).as_posix()
    proofs = []
    skipped = []
    for index, row in enumerate(rows):
        if row.get("status") not in {"observed", "unsupported"}:
            skipped.append(index)
            continue
        pointer = f"/{index}" if isinstance(supplied, list) else f"/records/{index}"
        proofs.append(
            make_proof(
                row,
                root=root,
                extra_paths=args.extra,
                record_path=record_path,
                record_pointer=pointer,
            )
        )
    if len({proof["id"] for proof in proofs}) != len(proofs):
        raise ValueError(
            "measurements repeat a proof identity; retain a distinct invocation"
        )
    with args.output.open("x") as stream:
        stream.write(json.dumps(proofs, indent=2, ensure_ascii=False) + "\n")
    print(json.dumps({"prepared": len(proofs), "unpromoted_indices": skipped}))


if __name__ == "__main__":
    main()
