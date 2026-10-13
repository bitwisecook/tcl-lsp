#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Rebuild compact F5 question views and replay recipes from retained captures."""

from __future__ import annotations

import collections
import difflib
import gzip
import hashlib
import importlib.util
import json
import re
from pathlib import Path

from oracle_store import catalogue, retained_file

ROOT = Path(__file__).resolve().parents[2]
DEST = ROOT / "experiments/oracles/f5"
SPEC = importlib.util.spec_from_file_location("materialise", DEST / "materialise.py")
materialise = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(materialise)


def read(name):
    return retained_file(ROOT, name).read_bytes()


def compile_time_inputs(files):
    retained = collections.defaultdict(list)
    for name in [
        "rust/tcl-registry/src/f5/evidence/resolution_contexts.rs",
        "rust/tcl-vm/src/interp/authored_tmm_static.rs",
    ]:
        source = ROOT / name
        for match in re.finditer(r'include_str!\(\s*"([^"]+)"', source.read_text()):
            path = (source.parent / match[1]).resolve()
            if not path.is_relative_to(ROOT):
                continue
            logical = path.relative_to(ROOT).as_posix()
            if logical not in files:
                continue
            data = read(logical)
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(data)
            retained[logical].append(name)
    write_json(
        DEST / "captures/compile-time-inputs.json",
        {
            "schema_version": 1,
            "reason": "Small byte-exact include_str inputs remain expanded at historical paths; all other expanded captures are removed. This evidence migration needs no Rust implementation change.",
            "files": dict(retained),
        },
    )


def document(name):
    return json.loads(read(name))


def write_json(path, value):
    path.parent.mkdir(parents=True, exist_ok=True)
    data = (
        json.dumps(
            value, ensure_ascii=True, sort_keys=True, separators=(",", ":")
        ).encode()
        + b"\n"
    )
    path.write_bytes(gzip.compress(data, mtime=0) if path.suffix == ".gz" else data)


def group_for(name):
    if name.startswith("scripts/"):
        return "context-and-lexical-matrices"
    tag = name.split("/answers/", 1)[-1].split("/", 1)[0]
    for fragment, group in [
        ("literal", "literal-loading"),
        ("nsowner", "namespace-ownership"),
        ("persist", "persistent-cells"),
        ("native", "native-resolution"),
        ("app", "native-resolution"),
        ("stock", "stock-interpreters"),
        ("repair", "package-lifecycle"),
        ("harness", "package-lifecycle"),
        ("cap", "capability-and-lookup"),
        ("broad", "capability-and-lookup"),
    ]:
        if fragment in tag:
            return group
    return "portable-patterns"


def stage(value):
    if not isinstance(value, dict):
        return value
    return {
        key: value[key]
        for key in [
            "status",
            "exit_code",
            "returncode",
            "transcript",
            "stderr",
            "reached",
            "completion_code",
            "result_encoding",
            "result_hex",
            "errorcode",
            "errorinfo",
            "return_options",
            "return_options_availability",
            "unavailable_fields",
        ]
        if key in value
    }


def coverage(row):
    if (
        row.get("status") == "observed"
        and row.get("reached") is True
        and row.get("answer_record")
    ):
        return "complete"
    if row.get("attempt_record") or row.get("resolution_record"):
        return "partial"
    return "none"


def render_result(value):
    lookup = value.get("lookup", {})
    if not isinstance(lookup, dict):
        return "No separate lookup field."
    code = lookup.get("completion_code")
    raw = lookup.get("result_hex")
    text = "not captured"
    if isinstance(raw, str):
        try:
            text = (
                bytes.fromhex(raw).decode("utf-8")
                if lookup.get("result_encoding") == "utf-8"
                else "hex:" + raw
            )
        except (ValueError, UnicodeError):
            text = "hex:" + raw
    load = value.get("load") or {}
    setup = value.get("setup") or {}
    return f"Load {load.get('status', 'not captured')} (exit {load.get('exit_code', 'not captured')}); setup completion {setup.get('completion_code', 'not captured')}; lookup completion {code}; result {json.dumps(text, ensure_ascii=True)}."


def run_views(files):
    counts = collections.Counter()
    records_by_path = {}
    for name in files:
        if (
            "/answers/" not in name
            or not name.endswith("/records.json")
            or name.count("/", name.index("/answers/") + 9) != 1
        ):
            continue
        values = document(name)
        if not isinstance(values, list) or not all(
            isinstance(row, dict) for row in values
        ):
            continue
        run = name.split("/answers/", 1)[1].split("/", 1)[0]
        family = "patterns" if "/patterns/" in name else "names"
        key = family + "-" + run
        shared = {}
        rows = []
        for index, row in enumerate(values):
            result = {
                field: row[field]
                for field in [
                    "question_id",
                    "canonical_question_id",
                    "canonical_question",
                    "requested_canonical_question_id",
                    "scope",
                    "operation_scope",
                    "intended_lookup_reached",
                    "canonical_lookup_answer",
                    "case_id",
                    "variant_id",
                    "run_id",
                    "invocation_id",
                    "status",
                    "reason",
                    "lookup_stage",
                    "adaptation",
                    "original_input",
                    "actual_input",
                    "wrapper",
                    "configuration",
                    "case_metadata",
                    "invocation_evidence",
                ]
                if field in row
            }
            result["historical_record"] = {
                "path": name,
                "json_pointer": "/" + str(index),
                "sha256": files[name]["sha256"],
            }
            for field in ["provider", "cleanup", "cleanup_evidence", "raw_evidence"]:
                if field in row:
                    data = json.dumps(
                        row[field], sort_keys=True, separators=(",", ":")
                    ).encode()
                    reference = hashlib.sha256(data).hexdigest()
                    shared[reference] = row[field]
                    result[field + "_ref"] = reference
            for field in ["load", "setup", "lookup", "other_stages"]:
                if field in row:
                    result[field] = (
                        stage(row[field])
                        if field != "other_stages"
                        else {k: stage(v) for k, v in row[field].items()}
                        if isinstance(row[field], dict)
                        else row[field]
                    )
            rows.append(result)
        directory = DEST / "runs" / key
        directory.mkdir(parents=True, exist_ok=True)
        data = b"".join(
            json.dumps(row, sort_keys=True, separators=(",", ":")).encode() + b"\n"
            for row in rows
        )
        (directory / "observations.jsonl.gz").write_bytes(gzip.compress(data, mtime=0))
        write_json(directory / "shared.json.gz", shared)
        write_json(
            directory / "run.json",
            {
                "schema_version": 1,
                "historical_records": name,
                "historical_sha256": files[name]["sha256"],
                "observations": len(rows),
                "group": group_for(name),
                "projection": "Question/result fields retained; provider, raw-evidence and cleanup descriptors stored once by reference. Capture rows, chunks, full options and ordered receipts remain byte-exact in the indexed store.",
                "new_execution": False,
            },
        )
        records_by_path[name] = values
        counts[key] = len(rows)
    return counts, records_by_path


def group_views(files):
    groups = collections.defaultdict(list)
    for name in files:
        if name.endswith((".conf", ".scf")):
            groups[group_for(name)].append(name)
    recipe_index = {}
    for group, names in sorted(groups.items()):
        directory = DEST / "groups" / group
        directory.mkdir(parents=True, exist_ok=True)
        candidates = sorted(
            names,
            key=lambda name: (
                not name.endswith("/prepared.conf"),
                not name.endswith("/applied.conf"),
                name,
            ),
        )
        baseline = materialise.normalise(read(candidates[0]))
        (directory / "template.scf").write_bytes(baseline)
        (directory / "group.scf").write_text(
            "sys folder /Common/__oracle/" + group + " {}\n"
        )
        before = baseline.splitlines(keepends=True)
        before_offsets = [0]
        for line in before:
            before_offsets.append(before_offsets[-1] + len(line))
        recipes = {}
        for name in sorted(names):
            target = materialise.normalise(read(name))
            after = target.splitlines(keepends=True)
            offsets = [0]
            for line in after:
                offsets.append(offsets[-1] + len(line))
            delta = []
            for opcode, i, j, k, end in difflib.SequenceMatcher(
                None, before, after, autojunk=False
            ).get_opcodes():
                if opcode == "equal":
                    delta.append(
                        [
                            "group",
                            before_offsets[i],
                            before_offsets[j] - before_offsets[i],
                        ]
                    )
                elif opcode in ("replace", "insert"):
                    delta.append(["test", offsets[k], offsets[end] - offsets[k]])
            rebuilt = b"".join(
                (baseline if part[0] == "group" else target)[
                    part[1] : part[1] + part[2]
                ]
                for part in delta
            )
            assert rebuilt == target, name
            identifier = hashlib.sha256(name.encode()).hexdigest()[:16]
            recipes[identifier] = {
                "historical_configuration": name,
                "historical_sha256": files[name]["sha256"],
                "replay_sha256": hashlib.sha256(target).hexdigest(),
                "delta": delta,
                "addresses": sorted(materialise.query_addresses_from_bytes(target)),
                "new_execution": False,
            }
            recipe_index[name] = {"group": group, "case": identifier}
        write_json(directory / "cases.json.gz", recipes)
        write_json(
            directory / "group.json",
            {
                "schema_version": 1,
                "group": group,
                "base": "../../base/base.scf",
                "configuration": "group.scf",
                "reconstruction_template": "template.scf",
                "historical_template": candidates[0],
                "template_sha256": files[candidates[0]]["sha256"],
                "template_role": "shared reconstruction model and initial test programme; loading this template can execute RULE_INIT",
                "cases": "cases.json.gz",
                "case_count": len(recipes),
                "configuration_status": "reconstructed replay; not appliance-measured",
                "reset": "Check owned-object absence before creating. Use the recorded invocation/teardown for the selected case. A stateful phase requires its recorded predecessors; deleting configuration alone does not establish native catalogue retirement.",
                "ordering": "Case IDs do not imply execution order. Ordered driver receipts in the historical capture own the phase order.",
            },
        )
        (directory / "group.md").write_text(
            f"# {group}\n\n{len(recipes)} retained configuration recipes share [group.scf](group.scf) and the [base](../../base/base.md).\n\nThe JSON group descriptor and compressed case table own the reconstruction inputs. Each case stores only changed source slices against its historical reconstruction template. The deployment group SCF contains an owned folder; it does not load the reconstruction template or execute its probe bodies. The F5 CLI supplies address rewriting and object comparisons. These configurations are replay projections; their generation is not an appliance acceptance or execution measurement.\n\nFor a stateful case, retain and follow the historical driver order. Source channels, failed loads, actual worker identities and cleanup limits remain separate. Do not deploy the group and another case concurrently without proving their owned names and destinations are disjoint.\n\n```sh\npython3 experiments/oracles/f5/materialise.py addresses --group {group}\npython3 experiments/oracles/f5/materialise.py materialise --group {group} --case CASE --network 192.168.9.0/24 --output /tmp/f5-case\n```\n"
        )
        print("Group", group, len(recipes), flush=True)
    write_json(DEST / "groups/configuration-index.json.gz", recipe_index)
    return recipe_index


def question_views(files, records, recipes):
    bundle = document("experiments/name-handling/patterns/f5/questions.json")
    resolution = document(
        "experiments/name-handling/patterns/f5/answers/nh20261012finish1/resolutions.json"
    )
    resolution_rows = {
        (row["question_id"], row["context"]): row for row in resolution["rows"]
    }
    manifest = json.loads(
        (ROOT / "docs/design/analysis/name-resolution-proofs/manifest.json").read_text()
    )
    proofs = {row["id"]: row for row in manifest["proofs"]}
    destination = DEST / "questions"
    destination.mkdir(parents=True, exist_ok=True)
    summary = collections.Counter()
    for question in bundle["questions"]:
        name = question["id"]
        scoped = {}
        lines = [
            "# " + name,
            "",
            "## Question",
            "",
            question["question"],
            "",
            "## Answer",
            "",
            "The table reports coverage of each requested original lookup in its recorded context. Partial coverage retains a measured setup/capability boundary; it does not answer an unreached final lookup. Investigation completion is separate.",
            "",
            "| Context | Execution | Coverage | Investigation |",
            "| --- | --- | --- | --- |",
        ]
        for context, row in question["contexts"].items():
            copied = dict(row)
            copied["answer_coverage"] = coverage(row)
            summary[copied["answer_coverage"]] += 1
            closed = resolution_rows.get((name, context))
            if closed:
                copied["resolution"] = closed
            lines.append(
                f"| {context} | {row['status']} | {copied['answer_coverage']} | {row.get('investigation_status', 'not separately recorded')} |"
            )
            refs = []
            for field in ["answer_record", "attempt_record"]:
                reference = row.get(field)
                if reference:
                    values = records.get(reference["path"])
                    if values is None:
                        values = document(reference["path"])
                    measured = values
                    for part in reference["json_pointer"].lstrip("/").split("/"):
                        part = part.replace("~1", "/").replace("~0", "~")
                        measured = (
                            measured[int(part)]
                            if isinstance(measured, list)
                            else measured[part]
                        )
                    selected = {
                        key: measured[key]
                        for key in [
                            "actual_input",
                            "original_input",
                            "wrapper",
                            "configuration",
                            "invocation_evidence",
                            "scope",
                            "adaptation",
                            "status",
                            "reason",
                            "intended_lookup_reached",
                            "case_id",
                            "variant_id",
                            "invocation_id",
                        ]
                        if key in measured
                    }
                    selected["setup"] = stage(measured.get("setup"))
                    selected["lookup"] = stage(measured.get("lookup"))
                    selected["load"] = stage(measured.get("load"))
                    selected["provider_identity"] = {
                        key: measured.get("provider", {}).get(key)
                        for key in [
                            "version",
                            "build",
                            "context",
                            "event",
                            "tmm_unit",
                            "interpreter_path",
                            "interpreter_patchlevel",
                        ]
                    }
                    config = measured.get("configuration") or {}
                    if isinstance(config, dict) and config.get("path") in recipes:
                        selected["replay_recipe"] = recipes[config["path"]]
                    selected["historical_record"] = reference
                    raw = measured.get("raw_evidence") or []
                    selected["execution_files"] = {
                        "commands": measured.get("invocation_evidence", []),
                        "driver_receipts": [
                            item
                            for item in raw
                            if item.get("path", "").endswith(
                                ("/driver.json", "/invocation.sh")
                            )
                        ],
                        "client_measurements": [
                            item
                            for item in raw
                            if "/traffic-" in item.get("path", "")
                            or "keepalive" in item.get("path", "")
                        ],
                        "server_source": measured.get("wrapper")
                        or measured.get("configuration"),
                        "server_scope": "The applied hosted/iRule source is the measured server-side program. A generated HTTP::respond is not evidence of an origin backend.",
                        "queries_and_cleanup": measured.get("cleanup_evidence", []),
                        "pcap": "not captured in the retained naming oracle trees",
                    }
                    refs.append(selected)
            copied["selected_measurements"] = refs
            scoped[context] = copied
        lines += ["", "## Measured evidence", ""]
        for context, row in scoped.items():
            lines += ["### " + context, ""]
            for selected in row["selected_measurements"]:
                lines += [
                    render_result(selected),
                    "",
                    selected.get("reason")
                    or selected.get("scope")
                    or "Only the selected operation and invocation are covered.",
                    "",
                ]
            if row.get("reason"):
                lines += [row["reason"], ""]
            if not row["selected_measurements"]:
                lines += [
                    "No selected original lookup measurement is retained for this context.",
                    "",
                ]
        canonical = proofs.get(question.get("canonical_question_id"))
        if canonical:
            lines += [
                "## C Tcl and Jim Tcl controls",
                "",
                f"The [reviewed cross-provider proof](../../../../{canonical['document']}) retains independent provider scope. C Tcl version results are not substituted for appliance results. Jim's inner-script observations are distinct from C Tcltest PASS/SKIP outcomes.",
                "",
                "| Provider | Status | Scoped answer |",
                "| --- | --- | --- |",
            ]
            for provider in canonical["providers"]:
                if provider["id"] != "bigip":
                    answer = provider["answer"].replace("|", "\\|").replace("\n", " ")
                    lines.append(
                        f"| {provider['id']} | {provider['status']} | {answer} |"
                    )
            lines.append("")
        source = question.get("source")
        if source and source.get("path"):
            data = read(source["path"])
            if source.get("lines"):
                first, last = source["lines"]
                data = b"\n".join(data.split(b"\n")[first - 1 : last]) + b"\n"
                lines += [
                    "The following is the selected source range, not a standalone replacement for the real package/suite wrapper.",
                    "",
                ]
            if len(data) < 10000:
                lines += [
                    "## Exact source",
                    "",
                    "```tcl",
                    data.decode("utf-8", errors="backslashreplace").rstrip("\n"),
                    "```",
                    "",
                ]
        lines += [
            "## Configuration, scripts and decoding",
            "",
            f"[{name}.json]({name}.json) records the exact source, context rows, selected results, original record selectors and replay recipes. Historical record selectors retain client/server/invocation/query/cleanup references, capture completeness and decoding fields without copying them into every answer.",
            "",
            "Read an exact retained artefact or select a JSON measurement:",
            "",
            "```sh",
            "python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH",
            "python3 experiments/oracles/f5/materialise.py read --path RECORD_PATH --json-pointer /ROW",
            "```",
            "",
            "The stored bytes retain their existing sanitisation and hash meaning. Reconstructed addressing is labelled separately. Null or unavailable fields are not inferred, and an adapted control cannot fill an original-source gap.",
            "",
        ]
        (destination / (name + ".md")).write_text("\n".join(lines))
        write_json(
            destination / (name + ".json"),
            {
                "schema_version": 1,
                "id": name,
                "question": question["question"],
                "canonical_question_id": question.get("canonical_question_id"),
                "source": source,
                "case_id": question.get("case_id"),
                "contexts": scoped,
                "new_execution": False,
            },
        )
    write_json(
        DEST / "questions/index.json",
        {
            "schema_version": 1,
            "contexts": bundle["contexts"],
            "question_ids": [q["id"] for q in bundle["questions"]],
            "coverage": dict(summary),
            "historical_investigation_records": len(resolution["rows"]),
            "new_execution": False,
        },
    )
    return manifest, summary


def broad_question_views(manifest):
    request = read("experiments/name-handling/f5/REQUEST.md").decode()
    rows = []
    for line in request.splitlines():
        if not line.startswith("| F5-N"):
            continue
        fields = [field.strip() for field in line.strip("|").split("|")]
        identifier, question, seed = fields[:3]
        related = [
            proof
            for proof in manifest["proofs"]
            if identifier in proof["problem_statement"]
        ]
        status = "partial" if related else "none"
        lines = [
            "# " + identifier,
            "",
            "## Question",
            "",
            question,
            "",
            "## Answer",
            "",
            f"Coverage: **{status}** for this broad question. The {len(related)} linked proofs answer their own exact input/context/invocation questions. They do not establish every requested variant, worker or lifecycle condition.",
            "",
            "Starting inputs: " + seed,
            "",
            "## Scoped evidence",
            "",
            "| Exact proof question | Evidence |",
            "| --- | --- |",
        ]
        for proof in related:
            text = proof["question"].replace("|", "\\|").replace("\n", " ")
            lines.append(
                "| " + text + " | [Proof](../../../../" + proof["document"] + ") |"
            )
        if not related:
            lines += [
                "",
                "No proof has an explicit authored association with this broad question. Other retained measurements are not assigned by keyword similarity.",
            ]
        lines += [
            "",
            "## Configuration and reproduction",
            "",
            f"[{identifier}.json]({identifier}.json) lists the scoped proof IDs and documents. Each proof's adjacent JSON retains exact source/commands/capture selectors and configuration recipes. Historical source, client/server actions, queries, load receipts and cleanup remain independently selectable from the capture store.",
            "",
            "Unmeasured variants remain open. Neither completed investigation counts nor reconstructed configuration constitute new appliance execution.",
            "",
        ]
        (DEST / "questions" / (identifier + ".md")).write_text("\n".join(lines))
        write_json(
            DEST / "questions" / (identifier + ".json"),
            {
                "schema_version": 1,
                "id": identifier,
                "question": question,
                "seed": seed,
                "answer_coverage": status,
                "scoped_proofs": [
                    {"id": proof["id"], "document": proof["document"]}
                    for proof in related
                ],
                "new_execution": False,
            },
        )
        rows.append(identifier)
    return rows


def reports_view(files):
    lines = [
        "# Historical F5 reports",
        "",
        "These reports remain byte-exact in the indexed capture store. Current per-question answers are in [questions](questions/index.json); canonical proof pages retain their independent scope.",
        "",
        "Export a report using its historical logical path. Its original relative references belong to the same retained path namespace and can be exported with the read or extract command.",
        "",
    ]
    anchors = {}
    for name in sorted(files):
        if not name.endswith(".md"):
            continue
        anchor = "report-" + hashlib.sha256(name.encode()).hexdigest()[:16]
        anchors[name] = anchor
        title = read(name).decode(errors="replace").split("\n", 1)[0].lstrip("# ")
        lines += [
            '<a id="' + anchor + '"></a>',
            "",
            "## " + title,
            "",
            "```sh",
            "python3 experiments/oracles/f5/materialise.py read --path " + name,
            "```",
            "",
        ]
    (DEST / "reports.md").write_text("\n".join(lines))
    return anchors


def proof_views(manifest, recipes):
    count = 0
    for proof in manifest["proofs"]:
        if not any(
            p["id"] == "bigip" and p["status"] != "not-tested"
            for p in proof["providers"]
        ):
            continue
        path = ROOT / proof["document"]
        # A proof owns its exact question. Provider statements already present
        # in the conclusion are referenced rather than repeated verbatim.
        sections = ["# " + proof["id"], "", "Kind: `" + proof["kind"] + "`", ""]
        for heading, field in [
            ("Problem statement", "problem_statement"),
            ("Question", "question"),
            ("Conclusion", "conclusion"),
            ("Scope", "scope"),
        ]:
            sections += ["## " + heading, "", proof[field], ""]
        sections += ["## Provider answers", ""]
        present = "\n".join(sections)
        for provider in proof["providers"]:
            sections += [
                "### " + provider["id"],
                "",
                "Status: `"
                + provider["status"]
                + "`. Version: "
                + provider["version"]
                + ". Build: "
                + provider["build"]
                + ".",
                "",
            ]
            if provider["answer"] not in present:
                sections += [provider["answer"], ""]
                present += "\n" + provider["answer"]
            else:
                sections += ["The scoped answer is stated above.", ""]
        sections += [
            "## Exact evidence",
            "",
            f"[{path.stem}.json]({path.stem}.json) contains the exact evidence descriptors, original hashes and selectors. The canonical manifest remains the proof authority. The indexed F5 store resolves historical paths without retaining expanded duplicate trees.",
            "",
            "```sh",
            "python3 experiments/oracles/f5/materialise.py read --path HISTORICAL_PATH --json-pointer /ROW",
            "```",
            "",
            "## Reproduction",
            "",
            "The argument vector below records the historical invocation. Restore retained driver directories with `materialise.py extract` before replay, and apply the selected case's current address bindings and state prerequisites.",
            "",
            proof["replay"]["limitations"],
            "",
            "```json",
            json.dumps(proof["replay"]["command"]),
            "```",
            "",
        ]
        path.write_text(
            "\n".join(line.rstrip(" \t") for line in "\n".join(sections).split("\n"))
        )
        write_json(
            path.with_suffix(".json"),
            {
                "schema_version": 1,
                "question_id": proof["id"],
                "question": proof["question"],
                "evidence": proof["evidence"],
                "replay_recipes": {
                    item["path"]: recipes[item["path"]]
                    for item in proof["evidence"]
                    if item["path"] in recipes
                },
                "answer_coverage": "partial"
                if proof["kind"] == "native-attempt"
                else "complete within the exact recorded question and scope",
                "coverage_limits": "An observed refusal or availability result answers only that scoped question. It supplies no result for an unreached lookup; use the requested-question context matrix for original-question coverage.",
                "new_execution": False,
            },
        )
        count += 1
    return count


def main():
    files = catalogue(ROOT)["files"]
    compile_time_inputs(files)
    counts, records = run_views(files)
    print("Run observations", sum(counts.values()), "runs", len(counts), flush=True)
    recipes = group_views(files)
    manifest, summary = question_views(files, records, recipes)
    broad = broad_question_views(manifest)
    reports_view(files)
    proofs = proof_views(manifest, recipes)
    write_json(
        DEST / "layout.json",
        {
            "schema_version": 1,
            "question_documents": "questions",
            "canonical_proof_documents": "docs/design/analysis/name-resolution-proofs",
            "proofs_with_bigip_evidence": proofs,
            "broad_question_ids": broad,
            "run_observations": sum(counts.values()),
            "run_count": len(counts),
            "configuration_recipes": len(recipes),
            "coverage": dict(summary),
            "historical_logical_files": len(files),
            "historical_bytes": sum(row["bytes"] for row in files.values()),
            "stored_capture_bytes": sum(
                path.stat().st_size for path in (DEST / "captures").iterdir()
            ),
            "new_execution": False,
        },
    )
    print("Question coverage", dict(summary), "proof pages", proofs, flush=True)


if __name__ == "__main__":
    main()
