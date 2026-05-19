"""``f5 validate`` — run lint rules across one or more BIG-IP configs."""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

from dialects.f5.bigip.lint import CATEGORIES, SEVERITIES, Finding, run_lint

from ._paths import load_paths
from ._registry import verb


@verb(
    "validate",
    aliases=("lint",),
    help="Run BIG-IP best-practice / structural checks (orphans, empty pools, etc.).",
    formatter_class=argparse.RawDescriptionHelpFormatter,
)
def _configure(p: argparse.ArgumentParser, *, prog_name: str, default_dialect: str) -> None:  # noqa: ARG001
    p.description = (
        "Apply a registry of lint rules to one or more bigip.conf / SCF\n"
        "files.  Reports orphan monitors, empty pools, virtuals without\n"
        "pools or iRules, deprecated iRule commands, unknown iRule\n"
        "events, and similar checks.  Exit code: 0 when there are no\n"
        "findings or only info-level findings, 1 if any warning-level\n"
        "findings are emitted, 2 if any error-level findings are emitted."
    )
    p.epilog = (
        "Examples:\n"
        "  f5 validate bigip.conf\n"
        "  f5 validate bigip.conf --severity error\n"
        "  f5 validate bigip.conf --category irule\n"
        "  f5 validate bigip.conf --format sarif -o validate.sarif\n"
        "  f5 validate bigip.conf --format json | jq '.findings | length'\n"
    )
    p.add_argument("paths", nargs="+", help="bigip.conf / SCF files (`-` for stdin).")
    p.add_argument(
        "--category",
        choices=CATEGORIES,
        help="Limit to one rule category (config | irule).",
    )
    p.add_argument(
        "--severity",
        choices=SEVERITIES,
        help="Show only findings of this severity.",
    )
    p.add_argument(
        "--format",
        choices=("text", "json", "sarif"),
        default="text",
        dest="fmt",
        help="Output format (default: text).",
    )
    p.add_argument("-o", "--output", metavar="FILE", help="Write here (default: stdout).")
    p.set_defaults(handler=_run_validate)


def _run_validate(args: argparse.Namespace) -> int:
    try:
        sources, configs = load_paths(args.paths)
    except OSError as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 2

    findings = run_lint(
        sources=sources,
        configs=configs,
        category=args.category,
        severity=args.severity,
    )

    if args.fmt == "json":
        output = json.dumps(_to_json(findings), indent=2) + "\n"
    elif args.fmt == "sarif":
        output = json.dumps(_to_sarif(findings), indent=2) + "\n"
    else:
        output = _to_text(findings) + "\n"

    if args.output:
        Path(args.output).write_text(output, encoding="utf-8")
    else:
        sys.stdout.write(output)

    has_error = any(f.severity == "error" for f in findings)
    has_warning = any(f.severity == "warning" for f in findings)
    if has_error:
        return 2
    if has_warning:
        return 1
    return 0


def _to_text(findings: list[Finding]) -> str:
    if not findings:
        return "validate: no findings"
    by_sev: dict[str, list[Finding]] = {}
    for f in findings:
        by_sev.setdefault(f.severity, []).append(f)
    lines = [
        f"validate: {len(findings)} finding(s) "
        + ", ".join(f"{sev}={len(by_sev.get(sev, []))}" for sev in SEVERITIES if sev in by_sev)
    ]
    for sev in SEVERITIES:
        bucket = by_sev.get(sev, [])
        for f in sorted(bucket, key=lambda x: (x.rule_id, x.full_path)):
            lines.append(f"  [{f.severity}] {f.rule_id}: {f.full_path}: {f.message}")
    return "\n".join(lines)


def _to_json(findings: list[Finding]) -> dict:
    return {
        "summary": {
            "total": len(findings),
            "bySeverity": {
                sev: sum(1 for f in findings if f.severity == sev) for sev in SEVERITIES
            },
        },
        "findings": [
            {
                "ruleId": f.rule_id,
                "severity": f.severity,
                "category": f.category,
                "fullPath": f.full_path,
                "message": f.message,
            }
            for f in findings
        ],
    }


def _to_sarif(findings: list[Finding]) -> dict:
    """Minimal SARIF 2.1.0 output for CI integration."""
    rules: dict[str, dict] = {}
    for f in findings:
        rules.setdefault(
            f.rule_id,
            {
                "id": f.rule_id,
                "shortDescription": {"text": f.rule_id.replace("-", " ")},
                "defaultConfiguration": {"level": _sarif_level(f.severity)},
            },
        )
    results = [
        {
            "ruleId": f.rule_id,
            "level": _sarif_level(f.severity),
            "message": {"text": f.message},
            "locations": [
                {
                    "physicalLocation": {
                        "artifactLocation": {"uri": f.full_path},
                    }
                }
            ],
        }
        for f in findings
    ]
    return {
        "version": "2.1.0",
        "$schema": "https://json.schemastore.org/sarif-2.1.0.json",
        "runs": [
            {
                "tool": {
                    "driver": {
                        "name": "f5-validate",
                        "rules": list(rules.values()),
                    }
                },
                "results": results,
            }
        ],
    }


def _sarif_level(severity: str) -> str:
    return {"error": "error", "warning": "warning", "info": "note"}.get(severity, "note")
