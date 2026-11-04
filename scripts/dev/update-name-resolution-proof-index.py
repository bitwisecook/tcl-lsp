#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Legacy proof-record helpers retained for historical migration."""

import json
import re
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


def historical_build(value):
    """Keep recorded build facts without printing legacy digest values."""
    pattern = r"(?i)\b(?:[a-z0-9_/-]+_)?sha-?256\s*(?:=|\s)\s*`?[0-9a-f]{64}`?"
    if not re.search(pattern, value):
        return value
    value = re.sub(pattern, "", value)
    value = "; ".join(part.strip(" ,") for part in value.split(";") if part.strip(" ,"))
    value = re.sub(r"\s+,", ",", value)
    value = re.sub(r",\s*,", ",", value)
    value = re.sub(r"\s+\.", ".", value)
    value = re.sub(r"\s{2,}", " ", value).strip(" ,;")
    return value or "See historical references."


def render(record):
    text = f"# {record['id']}\n\nKind: `{record['kind']}`\n\n"
    for heading, field in [
        ("Problem statement", "problem_statement"),
        ("Question", "question"),
        ("Conclusion", "conclusion"),
        ("Scope", "scope"),
    ]:
        text += f"## {heading}\n\n{record[field].rstrip()}\n\n"
    text += "## Provider answers\n\n"
    for provider in record["providers"]:
        text += f"### {provider['id']}\n\nStatus: `{provider['status']}`. Version: {provider['version']}. Build: {historical_build(provider['build'])}. Channel: {provider['input_channel']}. Dialect: {provider['dialect']}.\n\n{provider['answer']}\n\n"
    text += "## Historical references\n\n"
    for item in record["evidence"]:
        selector = (
            f" JSON pointer `{item['json_pointer']}`." if "json_pointer" in item else ""
        )
        if "lines" in item:
            selector += f" Lines {item['lines'][0]}–{item['lines'][1]}."
        text += f"- `{item['id']}` ({item['role']}): [{item['path']}](../../../../{item['path']}).{selector} {item['description']}\n"
    if not record["evidence"]:
        text += "No evidence artifact is attached.\n"
    text += "\n## Source inspection\n\n"
    if not record["source_anchors"]:
        text += "No implementation source excerpt is attached. Native outputs do not supply an implementation explanation.\n"
    for anchor in record["source_anchors"]:
        geometry = ""
        if "line_unit" in anchor:
            geometry += f" Line unit: {anchor['line_unit']}."
        if "byte_range" in anchor:
            geometry += f" Original byte range [{anchor['byte_range'][0]}, {anchor['byte_range'][1]})."
        text += f"{anchor['provider']} {anchor['version']}, revision `{anchor['revision']}`, `{anchor['original_path']}`, function `{anchor['function']}`, lines {anchor['lines'][0]}–{anchor['lines'][1]}.{geometry} Retained reference `{anchor['evidence']}`.\n\n```text\n{anchor['snippet']}\n```\n\n"
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
    raise SystemExit(
        "The legacy proof index is retired. Record new executions in evidence/tcl "
        "using docs/design/contracts/tcl-execution-evidence.md."
    )


if __name__ == "__main__":
    main()
