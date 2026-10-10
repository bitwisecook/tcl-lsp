#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Merge reviewed question records and render their individual documents."""

import argparse
import importlib.util
import json
from pathlib import Path

DIRECTORY = Path("docs/design/analysis/name-resolution-proofs")


def merge_classifications(root, supplied, gate):
    """Admit a reviewed path only while its whole artifact bytes still match."""
    path = root / DIRECTORY / "corpus-inventory.json"
    census = json.loads(path.read_text())
    rows = {row["path"]: row for row in census["files"]}
    identifiers = {
        record["id"]
        for record in json.loads((root / DIRECTORY / "manifest.json").read_text())[
            "proofs"
        ]
    }
    updates = []
    seen = set()
    for row in supplied:
        name = row["path"]
        if name in seen:
            raise ValueError(f"repeated classification path: {name}")
        seen.add(name)
        actual = gate.relative_file(root, name)
        prior = rows.get(name)
        if prior is None or not prior.get("candidate"):
            raise ValueError(f"path is not an inventoried candidate: {name}")
        if row["sha256"] != prior["sha256"] or gate.digest(actual) != row["sha256"]:
            raise ValueError(f"classification input digest changed: {name}")
        classification = row["classification"]
        if classification not in gate.CLASSIFICATIONS - {"pending", "unavailable"}:
            raise ValueError(f"unsupported reviewed classification: {name}")
        if not row.get("reason", "").strip():
            raise ValueError(f"classification lacks inspected reason: {name}")
        ids = row.get("proof_ids", [])
        if (
            classification
            in {"native-observation", "authored-contract", "duplicate-evidence"}
            and not ids
        ):
            raise ValueError(f"classification lacks a question: {name}")
        if set(ids) - identifiers:
            raise ValueError(f"classification names an unknown question: {name}")
        if classification == "duplicate-evidence":
            duplicate = gate.relative_file(root, row.get("duplicate_of", ""))
            if gate.digest(duplicate) != row["sha256"]:
                raise ValueError(f"claimed duplicate has different bytes: {name}")
        updates.append((prior, row))
    # Validate every proposed update before publishing any classification.
    for prior, row in updates:
        prior.update(
            {
                key: row[key]
                for key in ["classification", "proof_ids", "reason", "duplicate_of"]
                if key in row
            }
        )
        if row["classification"] != "duplicate-evidence":
            prior.pop("duplicate_of", None)
    path.write_text(json.dumps(census, indent=2, ensure_ascii=False) + "\n")
    return len(updates)


def render(record):
    text = f"# {record['id']}\n\nKind: `{record['kind']}`\n\n"
    for heading, field in [
        ("Problem statement", "problem_statement"),
        ("Question", "question"),
        ("Conclusion", "conclusion"),
        ("Scope", "scope"),
    ]:
        text += f"## {heading}\n\n{record[field]}\n\n"
    text += "## Provider answers\n\n"
    for provider in record["providers"]:
        text += f"### {provider['id']}\n\nStatus: `{provider['status']}`. Version: {provider['version']}. Build: {provider['build']}. Channel: {provider['input_channel']}. Dialect: {provider['dialect']}.\n\n{provider['answer']}\n\n"
    text += "## Exact evidence\n\n"
    for item in record["evidence"]:
        selector = (
            f" JSON pointer `{item['json_pointer']}`." if "json_pointer" in item else ""
        )
        if "lines" in item:
            selector += f" Lines {item['lines'][0]}–{item['lines'][1]}."
        text += f"- `{item['id']}` ({item['role']}): [{item['path']}](../../../../{item['path']}). SHA-256 `{item['sha256']}`.{selector} {item['description']}\n"
    if not record["evidence"]:
        text += "No evidence artifact is attached.\n"
    text += "\n## Source inspection\n\n"
    if not record["source_anchors"]:
        text += "No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.\n"
    for anchor in record["source_anchors"]:
        text += f"{anchor['provider']} {anchor['version']}, revision `{anchor['revision']}`, `{anchor['original_path']}`, function `{anchor['function']}`, lines {anchor['lines'][0]}–{anchor['lines'][1]}. Full-source SHA-256 `{anchor['source_sha256']}`; snippet SHA-256 `{anchor['snippet_sha256']}`; retained evidence `{anchor['evidence']}`.\n\n```text\n{anchor['snippet']}\n```\n\n"
    text += "\n## Consumer bindings\n\n"
    for owner in record["owners"]:
        text += f"- [{owner['path']}](../../../../{owner['path']}), `{owner['symbol']}`: {owner['purpose']}\n"
    for test in record["tests"]:
        text += f"- [{test['path']}](../../../../{test['path']}), `{test['selector']}` ({test['status']}): {test['assertion']}\n"
    if not record["tests"] and not record["owners"]:
        text += "No implementation binding is claimed by this observation record.\n"
    text += "\nA named test is a coverage binding, not a claim that it executed.\n\n## Replay\n\n"
    if record["replay"]["command"]:
        text += (
            "Recorded argument vector:\n\n```json\n"
            + json.dumps(record["replay"]["command"], indent=2)
            + "\n```\n\n"
        )
    text += record["replay"]["limitations"] + "\n"
    return text


def render_index(root, records):
    """Keep the reader's question links aligned with the exact central records."""
    path = root / DIRECTORY / "README.md"
    if not path.exists():
        return
    prefix = path.read_text().split("\n## Question index\n", 1)[0].rstrip()
    text = prefix + "\n\n## Question index\n\n"
    text += "Provider-specific answers, original artifacts and limits are in each linked record.\n\n"
    text += "| Question record | Kind | Precise question |\n| --- | --- | --- |\n"
    for record in records:
        question = record["question"].replace("|", "\\|").replace("\n", " ")
        text += f"| [{record['id']}]({Path(record['document']).name}) | `{record['kind']}` | {question} |\n"
    path.write_text(text)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[2]
    )
    parser.add_argument(
        "--records",
        type=Path,
        help="Reviewed array of records, or manifest-shaped object.",
    )
    parser.add_argument(
        "--classifications",
        type=Path,
        help="Reviewed exact candidate path/digest classifications, after their questions are merged.",
    )
    parser.add_argument(
        "--replace",
        action="store_true",
        help="Explicitly replace existing same-ID records.",
    )
    parser.add_argument(
        "--render",
        action="store_true",
        help="Write the individual documents for these records.",
    )
    args = parser.parse_args()
    if (args.records is None) == (args.classifications is None):
        parser.error("select exactly one of --records or --classifications")
    if args.classifications and (args.replace or args.render):
        parser.error("--replace/--render apply only to question records")
    root = args.root.resolve()
    spec = importlib.util.spec_from_file_location(
        "proof_gate", root / "scripts/dev/check-name-resolution-proofs.py"
    )
    gate = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(gate)
    if args.classifications:
        try:
            count = merge_classifications(
                root, json.loads(args.classifications.read_text()), gate
            )
        except (KeyError, ValueError) as exc:
            parser.error(str(exc))
        print(
            f"Merged {count} reviewed exact artifact classifications. Run the complete gate."
        )
        return
    supplied = json.loads(args.records.read_text())
    records = supplied["proofs"] if isinstance(supplied, dict) else supplied
    schema = json.loads((root / DIRECTORY / "schema.json").read_text())
    errors = gate.schema_errors(
        {"schema_version": 1, "proofs": records}, schema, schema, "supplied"
    )
    if errors:
        parser.error("; ".join(errors))
    path = root / DIRECTORY / "manifest.json"
    manifest = json.loads(path.read_text())
    existing = {record["id"]: record for record in manifest["proofs"]}
    if len({record["id"] for record in records}) != len(records):
        parser.error("supplied records repeat an ID")
    for record in records:
        if record["id"] in existing and not args.replace:
            parser.error(f"refusing implicit replacement of {record['id']}")
        if args.render:
            destination = root / record["document"]
            if destination.parent.resolve() != (root / DIRECTORY).resolve():
                parser.error(
                    "proof documents must live directly in the owned proof directory"
                )
    for record in records:
        existing[record["id"]] = record
        if args.render:
            (root / record["document"]).write_text(render(record))
    manifest["proofs"] = sorted(existing.values(), key=lambda record: record["id"])
    path.write_text(json.dumps(manifest, indent=2, ensure_ascii=False) + "\n")
    render_index(root, manifest["proofs"])
    print(
        f"Merged {len(records)} reviewed questions; {len(existing)} total. Run the complete gate."
    )


if __name__ == "__main__":
    main()
