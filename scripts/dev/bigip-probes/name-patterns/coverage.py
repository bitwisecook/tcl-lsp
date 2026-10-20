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
"""Index retained measurements and gaps without promoting original answers.

Snapshots retain measurement pointers and file hashes. Adapted controls, whole
program errors, constraint skips and original-source candidates remain distinct.
The original questions.json and all existing run artefacts are read-only.
"""

import argparse
import hashlib
import json
import re
from collections import Counter
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[4]
PATTERN_BASE = "experiments/name-handling/patterns/f5"
BROAD_BASE = "experiments/name-handling/f5"

REMAINING = {
    "F5-N01": "Literal load controls versus computed-head/eval for each unavailable command/subcommand; capability errors need their exact context/input channel. APL capability surface remains independent.",
    "F5-N02": "Complete current/path/global precedence, absolute bypass and relative qualifier/no-ancestor variants in every applicable context. A rejected setup does not establish the subsequent lookup order.",
    "F5-N03": "Colon-run/empty-tail/lone-colon and duplicate displayed-FQN slots in each context; distinguish an invocation slot from namespace which/origin output.",
    "F5-N04": "Read/write/create/exists/unset and declaration contrasts for missing namespace-scope variables and relative qualifiers; repeat relevant operations without reinitialising the tested cell.",
    "F5-N05": "Exact Unicode/NUL/high-byte names and array-index roundtrips in each input channel, with explicit encoding. The current constructed U+03BB/NUL examples are bounded variants.",
    "F5-N06": "Full global/variable/upvar 0/1/#0/namespace-upvar and eval/uplevel frame matrix, alias chains and namespace-eval callers; record exact caller frames.",
    "F5-N07": "Import/export/force/forget/origin rename/redefinition/deletion chains and conflicts, including global origin; successful reduced variants do not cover unsupported setup routes.",
    "F5-N08": "Alias anchoring, late binding, destination parent creation, describe/delete and child ownership across contexts; retain duplicate prefix/data and post-rename description as distinct stages.",
    "F5-N09": "Repeated lookup after callback mutation, command traces and namespace-unknown/unknown error options. Unknown replacement must stay in an isolated child.",
    "F5-N10": "Active old frames/compiled descendants, namespace path/import/OO/trace tokens and caches after deletion/recreation. Variable-cell and namespace-code examples alone do not settle these identities.",
    "F5-N11": "Existing canonical owner-route controls retain folders/partitions and replacement. The exact native computed-head/catalogue/owned-rename receipts are reviewed per recorded context/unit; keep activated native definitions distinct from ordinary Tcl proc slots and literal loader rejection. Unmeasured new route variants require their own retained providers.",
    "F5-N12": "Protocol-prefix identity under same-tail shadows, absolute/relative/computed heads, required profiles/events and actual flow execution; CLI/stock errors are independent controls.",
    "F5-N13": "Targeted once-seeded RULE_INIT/native-procedure/HTTP and paired-connection storage receipts have completed independent review. Preserve four initial TMM units versus actual traffic units, independent same-connection/fresh-connection identities, exact owned-cell cleanup and cross-rule canonical controls. Entered suspended frames and restart/reload after deletion remain unmeasured.",
    "F5-N14": "Mathfunc namespace/shadow/mutation and supported OO method/forward/metaclass context; capability failure versus intended expr/method lookup must remain separate.",
    "F5-N15": "Missing command/variable/namespace and link/rename/import conflicts with actual inner completion/result/error options; unavailable options stay explicit, especially real-tcltest isolated children.",
    "F5-N16": "Literal accepted names versus dynamically evaluated names, braced/quoted/substituted/list prefixes, brace-newline and displayed-name roundtrips. Literal load acceptance requires its own retained configuration.",
}


def _read(path):
    return json.loads(path.read_text(encoding="utf-8"))


def _sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


def _context(context):
    return (
        "appliance-stock-tclsh" if context and context.startswith("stock-") else context
    )


def _receipt(record, path, index, root):
    lookup = record.get("lookup", {})
    setup = record.get("setup", {})
    return {
        "path": path.relative_to(root).as_posix(),
        "json_pointer": "/" + str(index),
        "run_id": record.get("run_id"),
        "question_id": record.get("question_id"),
        "case_id": record.get("case_id"),
        "variant_id": record.get("variant_id"),
        "context": record.get("provider", {}).get("context"),
        "unit": record.get("provider", {}).get("tmm_unit"),
        "invocation_id": record.get("invocation_id"),
        "status": record.get("status"),
        "load": record.get("load", {}).get("status"),
        "setup_code": setup.get("completion_code"),
        "lookup_reached": lookup.get("reached"),
        "lookup_code": lookup.get("completion_code"),
        "result_encoding": lookup.get("result_encoding"),
        "result_hex": lookup.get("result_hex"),
        "intended_lookup_reached": record.get("intended_lookup_reached"),
        "adaptation": record.get("adaptation"),
        "operation_scope": record.get("operation_scope"),
        "reason": record.get("reason"),
        "unavailable_fields": lookup.get("unavailable_fields", []),
        "configuration_saved": record.get("configuration_saved"),
    }


def _candidate(record, question, root):
    """Identify candidates for review only; this never authorises promotion."""
    source = question.get("source")
    if not source or record.get("adaptation") is not None:
        return False
    if record.get("canonical_question_id") != question.get("canonical_question_id"):
        return False
    if (
        record.get("operation_scope") == "entire-program"
        or record.get("canonical_lookup_answer") is False
    ):
        return False
    lookup = record.get("lookup", {})
    if not (
        record.get("status") == "observed"
        and record.get("setup", {}).get("completion_code") == 0
        and lookup.get("reached") is True
        and isinstance(lookup.get("completion_code"), int)
        and isinstance(lookup.get("result_hex"), str)
    ):
        return False
    metadata = record.get("case_metadata")
    if not metadata:
        return False
    try:
        metadata_path = root / metadata["path"]
        if _sha(metadata_path) != metadata["sha256"]:
            return False
        case = _read(metadata_path)[int(metadata["json_pointer"].removeprefix("/"))]
        original_bytes = (root / source["path"]).read_bytes()
        if (
            not case.get("exact_source")
            or case.get("adaptation") is not None
            or case["operation"].encode("utf-8") != original_bytes
        ):
            return False
        if hashlib.sha256(original_bytes).hexdigest() != source["sha256"]:
            return False
        if record.get("original_input", {}).get("sha256") != source["sha256"]:
            return False
        for key in ["actual_input", "wrapper"]:
            descriptor = record[key]
            if _sha(root / descriptor["path"]) != descriptor["sha256"]:
                return False
    except (KeyError, ValueError, IndexError, OSError, TypeError):
        return False
    return True


def _broad_questions(root):
    rows = []
    for line in (
        (root / BROAD_BASE / "REQUEST.md").read_text(encoding="utf-8").splitlines()
    ):
        if re.match(r"\| F5-N[0-9]{2} \|", line):
            columns = [cell.strip() for cell in line.strip().strip("|").split("|")]
            rows.append(
                {
                    "question_id": columns[0],
                    "question": columns[1],
                    "starting_input": columns[2],
                }
            )
    return rows


def build_coverage(root=ROOT):
    """Read completed report files into a 400-row gap/receipt inventory."""
    root = Path(root).resolve()
    bundle_path = root / PATTERN_BASE / "questions.json"
    bundle = _read(bundle_path)
    inputs = [
        {"path": bundle_path.relative_to(root).as_posix(), "sha256": _sha(bundle_path)}
    ]
    for source_path in [
        root / BROAD_BASE / "REQUEST.md",
        root / "scripts/dev/bigip-probes/name-patterns/coverage.py",
    ]:
        inputs.append(
            {
                "path": source_path.relative_to(root).as_posix(),
                "sha256": _sha(source_path),
            }
        )
    reports = []
    problems = []
    records = []
    for base in [PATTERN_BASE, BROAD_BASE]:
        for path in sorted((root / base / "answers").glob("*/records.json")):
            try:
                retained = _read(path)
            except (OSError, ValueError) as exc:
                problems.append(
                    {"path": path.relative_to(root).as_posix(), "reason": str(exc)}
                )
                continue
            if not isinstance(retained, list):
                problems.append(
                    {
                        "path": path.relative_to(root).as_posix(),
                        "reason": "records are not a list",
                    }
                )
                continue
            source = {"path": path.relative_to(root).as_posix(), "sha256": _sha(path)}
            inputs.append(source)
            reports.append(
                {
                    **source,
                    "run_id": path.parent.name,
                    "record_count": len(retained),
                    "statuses": dict(Counter(r.get("status") for r in retained)),
                    "answer_document": (path.parent / "ANSWERS.md")
                    .relative_to(root)
                    .as_posix()
                    if (path.parent / "ANSWERS.md").exists()
                    else None,
                }
            )
            records.extend(
                (record, path, index) for index, record in enumerate(retained)
            )
    pattern_rows = []
    for question in bundle["questions"]:
        for context in bundle["contexts"]:
            observations = []
            for record, path, index in records:
                if _context(record.get("provider", {}).get("context")) != context:
                    continue
                matches = record.get("question_id") == question["id"] or (
                    question.get("case_id") is not None
                    and record.get("case_id") == question["case_id"]
                )
                if not matches:
                    continue
                receipt = _receipt(record, path, index, root)
                receipt["classification"] = (
                    "original-source-candidate-for-review"
                    if _candidate(record, question, root)
                    else "whole-program-stage"
                    if record.get("operation_scope") == "entire-program"
                    else "adapted-variant"
                    if record.get("adaptation") is not None
                    else "scoped-measurement-receipt"
                )
                observations.append(receipt)
            matrix = question["contexts"][context]
            pattern_rows.append(
                {
                    "question_id": question["id"],
                    "canonical_question_id": question.get("canonical_question_id"),
                    "case_id": question.get("case_id"),
                    "question": question["question"],
                    "context": context,
                    "original_matrix": dict(matrix),
                    "accepted_answer_pointer": matrix.get("answer_record"),
                    "exact_answer": "original-matrix-pointer-retained"
                    if matrix.get("answer_record")
                    else "pending-exact-answer-review",
                    "receipt_counts": dict(Counter(r["status"] for r in observations)),
                    "receipts": observations,
                    "remaining": "Exact original question/input/context authority remains pending; related adapted or whole-program measurements do not update the original matrix."
                    if not matrix.get("answer_record")
                    else "Existing authored answer pointer is retained without changing its authority.",
                }
            )
    broad_rows = []
    for question in _broad_questions(root):
        observations = []
        for record, path, index in records:
            qid = record.get("question_id", "")
            if re.match(re.escape(question["question_id"]) + r"(?:-|$)", qid):
                observations.append(_receipt(record, path, index, root))
        broad_rows.append(
            {
                **question,
                "receipt_counts": dict(Counter(r["status"] for r in observations)),
                "receipts": observations,
                "remaining": REMAINING[question["question_id"]],
                "related_native_context_boundaries": [
                    _receipt(record, path, index, root)
                    for record, path, index in records
                    if question["question_id"] in {"F5-N11", "F5-N13"}
                    and record.get("case_id") == "call"
                    and path.parent.name in {"nh20261012cap2", "nh20261012aplcap1"}
                    and not record.get("provider", {})
                    .get("context", "")
                    .startswith("tmm-")
                    and record.get("lookup", {}).get("result_hex")
                    == b'invalid command name "call"'.hex()
                ],
                "context_rows": [
                    {
                        "context": context,
                        "receipt_counts": dict(
                            Counter(
                                r["status"]
                                for r in observations
                                if _context(r["context"]) == context
                            )
                        ),
                        "reached_variant_count": len(
                            {
                                (r["case_id"], r["variant_id"])
                                for r in observations
                                if _context(r["context"]) == context
                                and r["status"] == "observed"
                                and r["lookup_reached"] is True
                            }
                        ),
                        "bounded_variant_count": len(
                            {
                                (r["case_id"], r["variant_id"])
                                for r in observations
                                if _context(r["context"]) == context
                            }
                        ),
                        "remaining": "Native rule/call context unavailable on independent exact call capability inputs; these boundary receipts do not answer native semantics."
                        if question["question_id"] in {"F5-N11", "F5-N13"}
                        and not context.startswith("tmm-")
                        else "The full numbered question remains broader than these bounded variants; unobserved variants remain pending.",
                    }
                    for context in bundle["contexts"]
                ],
                "coverage": "bounded-variants-returned"
                if observations
                else "no-new-matching-receipts",
            }
        )
    if len(pattern_rows) != 400 or len(broad_rows) != 16:
        raise ValueError("expected 50 x 8 pattern rows and 16 broad questions")
    return {
        "schema_version": 1,
        "generated_utc": datetime.now(timezone.utc).isoformat(),
        "inputs": inputs,
        "reports": reports,
        "read_problems": problems,
        "pattern_rows": pattern_rows,
        "broad_rows": broad_rows,
        "limitations": "Counts are derived receipt projections, never deduplicated runtime evidence. Stock 8.4 and 8.5 share one bundle context but remain distinct receipts. Original-source candidates require independent review; no original matrix is mutated.",
    }


def _cell(value):
    return str(value).replace("|", "\\|").replace("\n", " ")


def _link(path, root):
    label = "/".join(Path(path).parts[-2:]) if "/answers/" in path else path
    return "[" + label + "](" + str(root / path) + ")"


def _receipt_links(receipts, root):
    """Bound Markdown links; complete unit/invocation inventory stays in JSON."""
    if not receipts:
        return "No matching returned receipt."
    representatives = []
    for state in [
        "observed",
        "unreached",
        "unavailable",
        "unsupported",
        "infrastructure-failure",
    ]:
        selected = next((r for r in receipts if r["status"] == state), None)
        if selected is not None:
            representatives.append(selected)
        if len(representatives) == 2:
            break
    if not representatives:
        representatives = receipts[:1]
    return (
        "Representative receipts: "
        + "; ".join(
            _link(r["path"], root)
            + " `"
            + r["json_pointer"]
            + "` ("
            + _cell(r["context"])
            + ", unit "
            + _cell(r["unit"])
            + ", invocation "
            + _cell(r["invocation_id"])
            + ", "
            + _cell(r["status"])
            + ")"
            for r in representatives
        )
        + ". Full independent unit/invocation inventory: [coverage.json](coverage.json)."
    )


def _document(data, kind, root):
    lines = [
        "# F5 naming answer coverage",
        "",
        "Snapshot UTC: "
        + data["generated_utc"]
        + ". Provider facts and result bytes remain in the linked per-invocation measurement records.",
        "",
        "The original portable-question matrix is preserved. An adapted variant, whole-program error, harness pass or constraint skip does not establish an exact original final-lookup answer. Original-source candidates are identified for review only. Contradictory measurements and repeated units/invocations remain separate.",
        "",
        "## Retained report inputs",
        "",
        "| Run | Records | Report | Records SHA-256 |",
        "| --- | --- | --- | --- |",
    ]
    for report in data["reports"]:
        lines.append(
            "| "
            + _cell(report["run_id"])
            + " | "
            + str(report["record_count"])
            + " | "
            + _link(report["answer_document"] or report["path"], root)
            + " | `"
            + report["sha256"]
            + "` |"
        )
    matrix_states = Counter(
        r["original_matrix"]["status"] for r in data["pattern_rows"]
    )
    accepted = sum(
        r["accepted_answer_pointer"] is not None for r in data["pattern_rows"]
    )
    attempted = sum(
        r["original_matrix"].get("attempt_record") is not None
        for r in data["pattern_rows"]
    )
    lines.extend(
        [
            "",
            "Original 400-row matrix states: "
            + ", ".join(
                str(n) + " " + state for state, n in sorted(matrix_states.items())
            )
            + ". Accepted original pointers: "
            + str(accepted)
            + "; measured attempt pointers without original answer: "
            + str(attempted)
            + ". Related receipt counts below count actual units/invocations separately and do not change these original states.",
            "",
        ]
    )
    rows = data["pattern_rows"] if kind == "patterns" else data["broad_rows"]
    if kind == "patterns":
        lines.extend(
            [
                "",
                "## Every original question and context",
                "",
                "The table has all 50 questions × 8 contexts. Both stock interpreters remain distinct in receipt details. A pending exact answer can have useful measured adapted controls.",
                "",
                "| Question | Context | Original matrix status | Returned receipt states | Bounded input variants | Original-source candidates | Original pointer |",
                "| --- | --- | --- | --- | --- | --- | --- |",
            ]
        )
        for row in rows:
            candidates = sum(
                r["classification"] == "original-source-candidate-for-review"
                for r in row["receipts"]
            )
            counts = (
                ", ".join(
                    str(n) + " " + state
                    for state, n in sorted(row["receipt_counts"].items())
                )
                or "none"
            )
            lines.append(
                "| "
                + " | ".join(
                    _cell(v)
                    for v in [
                        row["question_id"],
                        row["context"],
                        row["original_matrix"]["status"],
                        counts,
                        len({(r["case_id"], r["variant_id"]) for r in row["receipts"]}),
                        candidates,
                        (
                            _link(
                                (
                                    row["accepted_answer_pointer"]
                                    or row["original_matrix"].get("attempt_record")
                                )["path"],
                                root,
                            )
                            + " `"
                            + (
                                row["accepted_answer_pointer"]
                                or row["original_matrix"].get("attempt_record")
                            )["json_pointer"]
                            + "` ("
                            + (
                                "answer"
                                if row["accepted_answer_pointer"]
                                else "attempt"
                            )
                            + ")"
                        )
                        if row["accepted_answer_pointer"]
                        or row["original_matrix"].get("attempt_record")
                        else "No original pointer; related receipts in JSON",
                    ]
                )
                + " |"
            )
        lines.extend(
            [
                "",
                "Full authored questions, exact answer/attempt descriptors, related variant classifications and every independent unit/invocation receipt remain in [coverage.json](coverage.json). The matrix pointer table preserves original states without promoting related observations.",
                "",
            ]
        )
    else:
        lines.extend(
            [
                "",
                "## F5-N01 through F5-N16",
                "",
                "Each receipt answers its materialised bounded variant. A full numbered question remains broader than those variants; no complete all-context answer is inferred.",
                "",
                "| Question | Returned receipt states | Coverage | Remaining concrete variants |",
                "| --- | --- | --- | --- |",
            ]
        )
        for row in rows:
            counts = (
                ", ".join(
                    str(n) + " " + state
                    for state, n in sorted(row["receipt_counts"].items())
                )
                or "none"
            )
            lines.append(
                "| "
                + " | ".join(
                    _cell(v)
                    for v in [
                        row["question_id"],
                        counts,
                        row["coverage"],
                        row["remaining"],
                    ]
                )
                + " |"
            )
        for row in rows:
            lines.extend(
                [
                    "",
                    "## " + row["question_id"],
                    "",
                    row["question"],
                    "",
                    "Starting inputs: " + row["starting_input"],
                    "",
                    "Existing native evidence and remaining controls: "
                    + _link(
                        BROAD_BASE + "/answers/native-appliance-question-review.md",
                        root,
                    )
                    if row["question_id"] in {"F5-N11", "F5-N13"}
                    else "Bounded receipt counts retain all independent units and invocations.",
                    "",
                    "| Context | Returned receipt states | Bounded input variants | Reached bounded variants | Boundary/scope |",
                    "| --- | --- | --- | --- | --- |",
                    *[
                        "| "
                        + _cell(c["context"])
                        + " | "
                        + (
                            ", ".join(
                                str(n) + " " + state
                                for state, n in sorted(c["receipt_counts"].items())
                            )
                            or "none"
                        )
                        + " | "
                        + str(c["bounded_variant_count"])
                        + " | "
                        + str(c["reached_variant_count"])
                        + " | "
                        + _cell(c["remaining"])
                        + " |"
                        for c in row["context_rows"]
                    ],
                    "",
                    _receipt_links(row["receipts"], root),
                    "",
                    "Remaining: " + row["remaining"],
                    "",
                ]
            )
    lines.extend(
        [
            "",
            "## Integrity and interpretation",
            "",
            data["limitations"],
            "",
            "Input digests:",
            "",
        ]
    )
    for source in data["inputs"]:
        lines.append(
            "- " + _link(source["path"], root) + " SHA-256 `" + source["sha256"] + "`."
        )
    if data["read_problems"]:
        lines.extend(["", "Reports unavailable at snapshot time:", ""])
        for problem in data["read_problems"]:
            lines.append(
                "- " + _cell(problem["path"]) + ": " + _cell(problem["reason"])
            )
    return "\n".join(lines) + "\n"


def write_coverage(tag, root=ROOT):
    """Write new immutable answer snapshots beside both question destinations."""
    if not re.fullmatch(r"[A-Za-z][A-Za-z0-9_-]{0,63}", tag):
        raise ValueError("invalid coverage snapshot tag")
    root = Path(root).resolve()
    targets = [
        (root / base / "answers" / tag, kind)
        for base, kind in [(PATTERN_BASE, "patterns"), (BROAD_BASE, "broader")]
    ]
    if any(path.exists() for path, _ in targets):
        raise FileExistsError("coverage snapshot already exists; use a new tag")
    data = build_coverage(root)
    for path, kind in targets:
        path.mkdir(parents=True)
        (path / "coverage.py").write_bytes(Path(__file__).read_bytes())
        (path / "coverage.json").write_text(
            json.dumps(data, indent=2, ensure_ascii=False) + "\n", encoding="utf-8"
        )
        (path / "answers.md").write_text(_document(data, kind, root), encoding="utf-8")
    return [path for path, _ in targets]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tag")
    args = parser.parse_args()
    for path in write_coverage(args.tag):
        print(path)


if __name__ == "__main__":
    main()
