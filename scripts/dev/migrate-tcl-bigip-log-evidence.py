#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Recover uniquely attributed iApp, iCall and TMM script log observations."""

from __future__ import annotations

import argparse
import json
import runpy
from collections import defaultdict
from pathlib import Path

from tcl_evidence_format import markdown_block, script_from_markdown, validate_document

ROOT = Path(__file__).resolve().parents[2]
question = runpy.run_path(str(ROOT / "scripts/dev/migrate-tcl-tmsh-evidence.py"))[
    "question"
]
CONTEXTS = (
    "iapp-implementation",
    "icall",
    "tmm-RULE_INIT",
    "tmm-procedure",
    "tmm-traffic-event",
)


def record_key(record: dict) -> tuple[str, ...]:
    provider = record["provider"]
    return (
        str(record.get("run_id") or ""),
        str(record.get("case_id") or ""),
        str(provider.get("context") or ""),
        str(provider.get("tmm_unit") or "none"),
        str(record.get("invocation_id") or "once"),
    )


def source_attribution(paths: set[str | None], selected: str) -> str:
    """Distinguish another script's invocation from an ambiguous attribution."""
    if None in paths:
        return "ambiguous"
    if paths and selected not in paths:
        return "other"
    return "selected" if paths == {selected} else "ambiguous"


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output", type=Path, default=ROOT / "evidence/tcl")
    args = parser.parse_args()
    runs: dict[Path, list[Path]] = defaultdict(list)
    for script in (ROOT / "experiments/name-handling").glob("**/input.tcl"):
        if any(script.parent.name.startswith(f"{name}-") for name in CONTEXTS):
            runs[script.parent.parent].append(script)
    # Keep equal script bytes together even when different contexts ran them.
    groups: dict[
        bytes, list[tuple[Path, list[tuple[tuple[str, ...], dict, bytes]]]]
    ] = defaultdict(list)
    skipped = 0
    for run, scripts in sorted(runs.items()):
        records_file = run / "records.json"
        if not records_file.is_file():
            skipped += len(scripts)
            continue
        records = json.loads(records_file.read_bytes())
        by_key: dict[tuple[str, ...], list[dict]] = defaultdict(list)
        by_path: dict[str, list[dict]] = defaultdict(list)
        for record in records:
            actual = record.get("actual_input") or {}
            if (
                not isinstance(actual, dict)
                or not actual.get("path")
                or not isinstance(record.get("provider"), dict)
            ):
                continue
            by_key[record_key(record)].append(record)
            by_path[actual["path"]].append(record)
        for script_path in sorted(scripts):
            directory = script_path.parent
            log = directory / "runtime-logs.stdout"
            config = directory / "applied.conf"
            script = script_path.read_bytes()
            path_name = str(script_path.relative_to(ROOT))
            if (
                not (log.is_file() and config.is_file())
                or script not in config.read_bytes()
            ):
                skipped += 1
                continue
            relevant = by_path[path_name]
            matched: list[tuple[tuple[str, ...], dict, bytes]] = []
            expected_prefixes = {record_key(record)[:3] for record in relevant}
            incomplete = False
            for raw in log.read_bytes().splitlines(keepends=True):
                offset = raw.find(b"NHO|")
                if offset < 0:
                    continue
                fields = raw[offset:].rstrip(b"\r\n").split(b"|")
                if len(fields) != 13:
                    continue
                try:
                    key = tuple(fields[i].decode("ascii") for i in (1, 2, 3, 6, 7))
                except UnicodeDecodeError:
                    continue
                if key[:3] not in expected_prefixes:
                    continue
                matches = by_key.get(key, [])
                paths = {(row.get("actual_input") or {}).get("path") for row in matches}
                # A shared log may contain an explicitly identified invocation
                # belonging solely to another retained script.
                attribution = source_attribution(paths, path_name)
                if attribution == "other":
                    continue
                if attribution == "ambiguous":
                    incomplete = True
                    continue
                selected = [row for row in relevant if record_key(row) == key]
                if len(selected) != 1 or question(selected[0]) is None:
                    incomplete = True
                    continue
                matched.append((key, selected[0], raw))
            if not matched or incomplete:
                skipped += 1
                continue
            groups[script].append((script_path, matched))
    output = args.output
    output.mkdir(parents=True, exist_ok=True)
    existing = {}
    for path in output.glob("bigip-log-*.md"):
        script = script_from_markdown(path.read_text())
        if script in existing:
            raise ValueError(f"duplicate existing script: {path}")
        existing[script] = path
    written = 0
    for script, captures in groups.items():
        # Each environment result contains only rows explicitly matched to its
        # script and recorded case; the full original log remains in Git.
        environments: dict[
            tuple[str, ...], list[tuple[tuple[str, ...], dict, bytes]]
        ] = defaultdict(list)
        case_questions: dict[str, set[str]] = defaultdict(set)
        case_scopes: dict[str, set[str]] = defaultdict(set)
        for _, rows in captures:
            for key, record, raw in rows:
                provider = record["provider"]
                env = (
                    key[2],
                    key[3],
                    key[4],
                    str(provider.get("version") or "Not recorded"),
                    str(provider.get("interpreter_patchlevel") or "Not recorded"),
                )
                environments[env].append((key, record, raw))
                case_questions[key[1]].add(question(record))
                case_scopes[key[1]].add(
                    str(record.get("scope") or "Not recorded").replace("\n", " ")
                )
        first = captures[0][0]
        parts = [
            f"# BIG-IP Tcl log: {first.parent.parent.name}, {first.parent.name}\n\n",
            (
                "## Question\n\nThe following cases investigate Tcl behavior relevant "
                "to name resolution and diagnostics in the LSP, compiler and VM. "
                "Each observation is limited to its own case and recorded context.\n"
            ),
        ]
        for identity, questions in sorted(case_questions.items()):
            for text in sorted(questions):
                parts.append(f"\n- **{identity}:** {text}\n")
        parts.append("\n## Assumptions\n\n")
        for identity, scopes in sorted(case_scopes.items()):
            for scope in sorted(scopes):
                parts.append(f"- **{identity}:** {scope}\n")
        parts.extend(
            [
                "\nOther assumptions were not separately recorded.\n\n## Script\n\n",
                markdown_block(script),
                "\n## Results\n",
            ]
        )
        for (context, unit, invocation, version, patchlevel), rows in sorted(
            environments.items()
        ):
            parts.extend(
                [
                    f"\n### {context}, unit {unit}, invocation {invocation}\n\n",
                    f"- Recorded version: BIG-IP {version}; Tcl patchlevel {patchlevel}.\n",
                    f"- Recorded input channel: {context}.\n",
                    "- Exit status: Not recorded.\n",
                    "- Standard output: Not recorded.\n",
                    "- Standard error: Not recorded.\n",
                    "- Recorded result:\n\n",
                    markdown_block(b"".join(row[2] for row in rows), "text-visible"),
                    (
                        "\nThese are log lines selected by the recorded run, case, context, "
                        "unit and invocation; they are not process stdout. Multiple log "
                        "snapshots may contain the same line, so duplicates do not by "
                        "themselves establish repeated executions. The following entries "
                        "decode each retained NHO payload chunk separately.\n"
                    ),
                ]
            )
            seen_cases = {}
            for key, record, raw in rows:
                seen_cases.setdefault(key[1], record.get("intended_lookup_reached"))
                fields = raw[raw.index(b"NHO|") :].rstrip(b"\r\n").split(b"|")
                payload = bytes.fromhex(fields[12].decode("ascii"))
                parts.extend(
                    [
                        (
                            f"\n- {key[1]}, {fields[4].decode()}, completion {fields[5].decode()}, "
                            f"{fields[8].decode()}, chunk {fields[10].decode()} of {fields[11].decode()}:\n\n"
                        ),
                        markdown_block(payload, "text-visible"),
                    ]
                )
            for identity, reached in sorted(seen_cases.items()):
                state = (
                    "reached"
                    if reached is True
                    else "not reached"
                    if reached is False
                    else "not recorded"
                )
                parts.append(f"\n- {identity}: intended operation {state}.\n")
        document = "".join(parts).rstrip() + "\n"
        errors = validate_document(document)
        if errors:
            raise ValueError(f"{first}: {errors}")
        path = (
            existing.get(script)
            or output / f"bigip-log-{first.parent.parent.name}-{first.parent.name}.md"
        )
        if path.exists() and path.read_text() != document:
            raise ValueError(f"refusing to overwrite {path}")
        path.write_text(document)
        written += 1
    print(
        f"Recovered {written} unique logged scripts; skipped {skipped} ambiguous or unobserved inputs"
    )


if __name__ == "__main__":
    main()
