#!/usr/bin/env python3
# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
# SPDX-License-Identifier: AGPL-3.0-or-later
"""Migrate unambiguous legacy Tcl captures to standalone Markdown files.

The old records contain many kinds of development material. This importer only
handles records that explicitly associate one Tcl input and one complete pair
of output streams with every observed environment. Other records are listed for
review; they are not silently treated as executions.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import re
import sys
from collections import Counter
from pathlib import Path
from typing import Any

from tcl_evidence_format import markdown_block, validate_document

LEGACY = Path("docs/design/analysis/name-resolution-proofs/manifest.json")


def retained_bytes(root: Path, item: dict[str, Any]) -> bytes | None:
    """Use an old recorded digest only to identify the captured version."""
    name = Path(item.get("path", ""))
    if not name.parts or name.is_absolute() or ".." in name.parts:
        return None
    path = (root / name).resolve()
    if not path.is_relative_to(root) or not path.is_file():
        return None
    data = path.read_bytes()
    recorded = item.get("sha256")
    if not isinstance(recorded, str) or hashlib.sha256(data).hexdigest() != recorded:
        return None
    return data


def stream_kind(path: str) -> str | None:
    name = Path(path).name
    for kind in ("stdout", "stderr"):
        if name in {kind, f"{kind}.txt", f"{kind}.bin"} or name.endswith(f".{kind}"):
            return kind
    return None


def capture_for_provider(
    root: Path, provider: dict[str, Any], evidence: dict[str, dict[str, Any]]
) -> tuple[tuple[Any, ...] | None, str | None]:
    refs = [evidence[name] for name in provider.get("evidence", []) if name in evidence]
    scripts = {
        item["path"]: item
        for item in refs
        if item.get("role") == "input" and item.get("path", "").endswith(".tcl")
    }
    if len(scripts) != 1:
        return None, "the executed Tcl script is missing or ambiguous"
    script_item = next(iter(scripts.values()))
    script = retained_bytes(root, script_item)
    if script is None:
        return None, "the retained script version cannot be identified"
    streams: dict[str, dict[str, dict[str, Any]]] = {"stdout": {}, "stderr": {}}
    for item in refs:
        if item.get("role") != "observation" or "json_pointer" in item:
            continue
        kind = stream_kind(item.get("path", ""))
        if kind:
            streams[kind][item["path"]] = item
    if any(len(items) != 1 for items in streams.values()):
        return None, "the recorded stdout or stderr stream is missing or ambiguous"
    stdout_item = next(iter(streams["stdout"].values()))
    stderr_item = next(iter(streams["stderr"].values()))
    stdout = retained_bytes(root, stdout_item)
    stderr = retained_bytes(root, stderr_item)
    if stdout is None or stderr is None:
        return None, "a retained output stream version cannot be identified"

    # A process status is used only when a retained receipt explicitly joins it
    # to these complete streams. An absent or ambiguous status stays unknown.
    statuses: set[int] = set()
    for item in refs:
        if item.get("role") != "provider" or not item.get("path", "").endswith(".json"):
            continue
        data = retained_bytes(root, item)
        if data is None:
            continue
        try:
            receipt = json.loads(data)
        except (UnicodeError, ValueError):
            continue
        if (
            isinstance(receipt, dict)
            and type(receipt.get("exit")) is int
            and receipt.get("stdout_sha256") == hashlib.sha256(stdout).hexdigest()
            and receipt.get("stderr_sha256") == hashlib.sha256(stderr).hexdigest()
        ):
            statuses.add(receipt["exit"])
    status = str(next(iter(statuses))) if len(statuses) == 1 else "Not recorded"
    return (
        (
            script_item["path"],
            script,
            str(provider.get("id", "Not recorded")),
            str(provider.get("version", "Not recorded")),
            str(provider.get("input_channel", "Not recorded")),
            stdout_item["path"],
            stdout,
            stderr_item["path"],
            stderr,
            status,
        ),
        None,
    )


def quote(text: str) -> str:
    """Keep the original wording without allowing it to create new headings."""
    return "\n".join("> " + line if line else ">" for line in text.strip().splitlines())


def render(
    records: list[dict[str, Any]],
    captures: list[tuple[Any, ...]],
    bigip_questions: dict[str, str] | None = None,
) -> str:
    parts = ["# Tcl execution evidence\n\n## Question\n\n"]
    for record in records:
        related = re.search(r"Related question (F5-N\d\d)", record["problem_statement"])
        if related and bigip_questions and related.group(1) in bigip_questions:
            parts.extend(
                [
                    quote(bigip_questions[related.group(1)]),
                    (
                        "\n\nThis record addresses only the case described below; "
                        "it does not answer every part of the broader question. "
                        "The distinction informs name resolution and diagnostics "
                        "in the LSP, compiler and VM.\n\n"
                    ),
                ]
            )
        parts.extend(
            [
                quote(record["problem_statement"]),
                "\n\n",
                quote(record["question"]),
                "\n\n",
            ]
        )
    parts.append(
        "## Assumptions\n\nAssumptions were not separately recorded in the retained capture.\n\n"
    )
    for record in records:
        parts.extend(["Recorded scope and limits:\n\n", quote(record["scope"]), "\n\n"])
    parts.extend(["## Script\n\n", markdown_block(captures[0][1]), "\n## Results\n"])
    for _, _, provider, version, channel, _, stdout, _, stderr, status in captures:
        label = provider.replace("\r", " ").replace("\n", " ")
        version = version.replace("\r", " ").replace("\n", " ")
        channel = channel.replace("\r", " ").replace("\n", " ").strip()
        parts.extend(
            [
                f"\n### {label}\n\n",
                f"- Recorded version: {version}\n",
                f"- Recorded input channel: {channel}\n",
                f"- Exit status: {status}\n",
            ]
        )
        for title, data in (("Standard output", stdout), ("Standard error", stderr)):
            if data:
                parts.extend(
                    [f"- {title}:\n\n", markdown_block(data, "text-visible"), "\n"]
                )
            else:
                parts.append(f"- {title}: empty.\n")
    return "".join(parts).rstrip() + "\n"


def migrate(root: Path, output: Path, write: bool) -> dict[str, Any]:
    manifest = json.loads((root / LEGACY).read_text())
    request = root / "experiments/name-handling/f5/REQUEST.md"
    bigip_questions = (
        dict(re.findall(r"(?m)^\| (F5-N\d\d) \| (.*?) \|", request.read_text()))
        if request.exists()
        else {}
    )
    groups: dict[
        tuple[Any, ...], tuple[list[dict[str, Any]], list[tuple[Any, ...]]]
    ] = {}
    skipped: Counter[str] = Counter()
    skipped_records: list[dict[str, str]] = []
    for record in manifest["proofs"]:
        if record.get("kind") not in {"native-observation", "native-attempt"}:
            continue
        evidence = {item["id"]: item for item in record.get("evidence", [])}
        measured = [
            p
            for p in record.get("providers", [])
            if p.get("status") in {"observed", "unsupported"}
        ]
        if not measured:
            reason = "no reached environment result is identified"
        else:
            captures = []
            reason = None
            for provider in measured:
                capture, reason = capture_for_provider(root, provider, evidence)
                if capture is None:
                    break
                captures.append(capture)
            if reason is None and len({capture[0] for capture in captures}) != 1:
                reason = "environments use different or ambiguous scripts"
        if reason is not None:
            skipped[reason] += 1
            skipped_records.append({"id": record["id"], "reason": reason})
            continue
        # Distinct input/stream paths represent distinct collected runs, even
        # when they happen to contain equal bytes.
        key = tuple((c[0], c[2], c[5], c[7]) for c in captures)
        if key not in groups:
            groups[key] = ([], captures)
        groups[key][0].append(record)

    documents = 0
    for records, captures in groups.values():
        identifier = re.sub(r"[^a-z0-9-]+", "-", records[0]["id"].lower()).strip("-")
        path = output / f"{identifier}.md"
        text = render(records, captures, bigip_questions)
        errors = validate_document(text)
        if errors:
            raise ValueError(
                f"{records[0]['id']}: generated invalid evidence: {'; '.join(errors)}"
            )
        if path.exists() and path.read_bytes() != text.encode("utf-8"):
            raise ValueError(f"refusing to overwrite existing evidence: {path}")
        if write and not path.exists():
            path.parent.mkdir(parents=True, exist_ok=True)
            path.write_bytes(text.encode("utf-8"))
        documents += 1
    return {
        "legacy_execution_records": sum(len(records) for records, _ in groups.values())
        + len(skipped_records),
        "converted_records": sum(len(records) for records, _ in groups.values()),
        "standalone_files": documents,
        "needs_review": len(skipped_records),
        "reasons": dict(skipped),
        "unconverted": skipped_records,
    }


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument(
        "--root", type=Path, default=Path(__file__).resolve().parents[2]
    )
    parser.add_argument(
        "--write", action="store_true", help="write the unambiguous Markdown files"
    )
    parser.add_argument(
        "--report",
        type=Path,
        help="write detailed migration results as development metadata",
    )
    args = parser.parse_args()
    root = args.root.resolve()
    try:
        report = migrate(root, root / "evidence/tcl", args.write)
        if args.report:
            args.report.write_text(json.dumps(report, indent=2) + "\n")
        print(
            f"{report['converted_records']} legacy execution record(s) map to "
            f"{report['standalone_files']} files; {report['needs_review']} require review."
        )
    except (OSError, ValueError, KeyError, TypeError) as error:
        print(f"cannot migrate Tcl evidence: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
