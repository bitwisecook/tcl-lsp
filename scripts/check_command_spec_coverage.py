#!/usr/bin/env python3
"""Audit command-spec coverage across `rust/tcl-registry/src/commands/`.

Walks every spec file, parses the `CommandSpec { ... }` body via
regex (the specs are static-data structs, so a parser is not
needed), and reports per-command coverage of the per-command
checklist in
`docs/rust-rewrite.md` § "Command-spec tracking".

Usage:
    python3 scripts/check_command_spec_coverage.py [--check] [--out PATH]

Modes:
    (default)   Write a markdown report to
                `docs/generated/command-spec-coverage.md`.
    --check     Compare against the existing report and exit non-zero
                if it would change. Use in CI / make codegen.
    --out PATH  Override output path.

Status semantics for each row:

    *   `set`     — the spec sets the field (a non-`None` /
                    non-empty value).
    *   `default` — the field is left at the `..CommandSpec::DEFAULT`
                    value. The audit cannot tell whether the default
                    is correct or just unstamped, so this is treated
                    as "not yet verified".
    *   `n/a`     — the row is not applicable for this command's
                    dialect / role family. For example, runtime hooks
                    (`lowering_hook`, `codegen_hook`,
                    `wasm_codegen_hook`) are `n/a` for iRules / iApps /
                    EDA dialects because they have no runtime.

The `// VERIFIED: <source>` doc-line convention captures human
attestation that the spec matches the canonical reference (Tcl
manpage / F5 wiki / vendor docs). Specs without a VERIFIED line
appear with status `unverified` for that column.
"""

from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path
from typing import Any

REPO_ROOT = Path(__file__).resolve().parent.parent
SPEC_ROOT = REPO_ROOT / "rust" / "tcl-registry" / "src" / "commands"
DEFAULT_OUT = REPO_ROOT / "docs" / "generated" / "command-spec-coverage.md"


# Dialects without a Tcl-bytecode / WASM runtime — runtime hooks
# are marked `n/a` for these.
NO_RUNTIME_DIALECTS = {
    "irules",
    "iapps",
    "tk",
    "expect",
    "eda_synopsys",
    "eda_cadence",
    "eda_xilinx",
    "eda_quartus",
    "eda_mentor",
    "sdc_base",
}

# Spec-file fields the audit inspects.
FIELD_PATTERNS: dict[str, re.Pattern[str]] = {
    "name": re.compile(r'name:\s*"([^"]*)"'),
    "arity": re.compile(r"arity:\s*Arity::"),
    "traits": re.compile(r"traits:\s*Traits::"),
    "dialects": re.compile(r"dialects:\s*Some\("),
    "arg_roles": re.compile(r"arg_roles:\s*&\["),
    "arg_role_resolver": re.compile(r"arg_role_resolver:\s*Some\("),
    "options": re.compile(r"options:\s*&\["),
    "subcommands": re.compile(r"subcommands:\s*SUBCOMMANDS"),
    "command_forms": re.compile(r"command_forms:\s*&\["),
    "hover": re.compile(r"hover:\s*Some\("),
    "return_type": re.compile(r"return_type:\s*Some\("),
    "arg_types": re.compile(r"arg_types:\s*&\[\s*\("),
    "side_effects": re.compile(r"side_effects:\s*&\[\s*SideEffect"),
    "assigns_variable_at": re.compile(r"assigns_variable_at:\s*Some\("),
    "safe_on_uninit": re.compile(r"safe_on_uninit:\s*Some\("),
    "inferred_storage_type": re.compile(r"inferred_storage_type:\s*Some\("),
    "lowering_hook": re.compile(r"lowering_hook:\s*Some\("),
    "codegen_hook": re.compile(r"codegen_hook:\s*Some\("),
    "wasm_codegen_hook": re.compile(r"wasm_codegen_hook:\s*Some\("),
    "excluded_events": re.compile(r"excluded_events:\s*&\[\s*\""),
    "required_package": re.compile(r"required_package:\s*Some\("),
}

VERIFIED_PATTERN = re.compile(r"^//[/!]?\s*VERIFIED:\s*(.+?)\s*$", re.MULTILINE)
NAME_PATTERN = re.compile(r'CommandSpec\s*\{\s*name:\s*"([^"]+)"')


def parse_spec(path: Path) -> dict[str, Any] | None:
    """Parse a spec file. Returns None for non-spec files."""
    text = path.read_text()
    name_m = NAME_PATTERN.search(text)
    if not name_m:
        return None

    fields: dict[str, bool] = {}
    for key, pat in FIELD_PATTERNS.items():
        fields[key] = bool(pat.search(text))

    verified_m = VERIFIED_PATTERN.search(text)

    dialect_dir = path.relative_to(SPEC_ROOT).parts[0]

    return {
        "path": str(path.relative_to(REPO_ROOT)),
        "name": name_m.group(1),
        "dialect_dir": dialect_dir,
        "fields": fields,
        "verified": verified_m.group(1) if verified_m else None,
    }


def collect_specs() -> list[dict[str, Any]]:
    out: list[dict[str, Any]] = []
    for path in sorted(SPEC_ROOT.rglob("*.rs")):
        if path.name == "mod.rs":
            continue
        spec = parse_spec(path)
        if spec is not None:
            out.append(spec)
    return out


def runtime_applicable(dialect_dir: str) -> bool:
    """True when runtime hooks (lowering / tclvm / wasm) apply.

    iRules, iApps, Tk, Expect, and the EDA family have no Rust /
    WASM / TclVM runtime — runtime hooks stay `n/a` for them.
    """
    return dialect_dir not in NO_RUNTIME_DIALECTS


def cell(value: str) -> str:
    """Markdown-table cell value."""
    return value


def status_for_field(spec: dict[str, Any], field: str) -> str:
    """Return a one-character status marker for `field` on `spec`."""
    if spec["fields"].get(field):
        return "✓"
    return "—"


def status_for_runtime_field(spec: dict[str, Any], field: str) -> str:
    """Same as `status_for_field` but yields `n/a` for dialects
    that have no runtime."""
    if not runtime_applicable(spec["dialect_dir"]):
        return "n/a"
    if spec["fields"].get(field):
        return "✓"
    return "—"


def status_for_verified(spec: dict[str, Any]) -> str:
    if spec["verified"]:
        return "✓"
    return "—"


# Universal + conditional + runtime columns rendered in the report.
COLUMNS: list[tuple[str, str, str]] = [
    # (column header, kind, field name)
    ("verified", "verified", ""),
    ("arity", "field", "arity"),
    ("hover", "field", "hover"),
    ("traits", "field", "traits"),
    ("arg_roles", "field", "arg_roles"),
    ("arg_role_resolver", "field", "arg_role_resolver"),
    ("return_type", "field", "return_type"),
    ("arg_types", "field", "arg_types"),
    ("options", "field", "options"),
    ("subcommands", "field", "subcommands"),
    ("command_forms", "field", "command_forms"),
    ("side_effects", "field", "side_effects"),
    ("assigns_var_at", "field", "assigns_variable_at"),
    ("safe_on_uninit", "field", "safe_on_uninit"),
    ("storage_type", "field", "inferred_storage_type"),
    ("excluded_events", "field", "excluded_events"),
    ("required_package", "field", "required_package"),
    ("lowering_hook", "runtime", "lowering_hook"),
    ("codegen_hook (tclvm)", "runtime", "codegen_hook"),
    ("wasm_codegen_hook", "runtime", "wasm_codegen_hook"),
]


def render_per_command_table(specs: list[dict[str, Any]]) -> str:
    headers = ["dialect", "command", "spec file"] + [c[0] for c in COLUMNS]
    sep = ["---"] * len(headers)
    rows: list[list[str]] = [headers, sep]
    for spec in specs:
        row = [
            cell(spec["dialect_dir"]),
            cell(f"`{spec['name']}`"),
            cell(f"`{spec['path']}`"),
        ]
        for header, kind, field in COLUMNS:
            if kind == "verified":
                row.append(status_for_verified(spec))
            elif kind == "runtime":
                row.append(status_for_runtime_field(spec, field))
            else:
                row.append(status_for_field(spec, field))
        rows.append(row)
    return "\n".join("| " + " | ".join(r) + " |" for r in rows)


def render_summary(specs: list[dict[str, Any]]) -> str:
    """Per-dialect summary: counts of specs and per-column coverage."""
    by_dialect: dict[str, list[dict[str, Any]]] = {}
    for spec in specs:
        by_dialect.setdefault(spec["dialect_dir"], []).append(spec)

    headers = ["dialect", "spec count"] + [c[0] for c in COLUMNS]
    sep = ["---"] * len(headers)
    rows: list[list[str]] = [headers, sep]
    for dialect in sorted(by_dialect):
        ss = by_dialect[dialect]
        row = [dialect, str(len(ss))]
        for header, kind, field in COLUMNS:
            if kind == "verified":
                stamped = sum(1 for s in ss if s["verified"])
                row.append(f"{stamped}/{len(ss)}")
            elif kind == "runtime":
                if not runtime_applicable(dialect):
                    row.append("n/a")
                    continue
                stamped = sum(1 for s in ss if s["fields"].get(field))
                row.append(f"{stamped}/{len(ss)}")
            else:
                stamped = sum(1 for s in ss if s["fields"].get(field))
                row.append(f"{stamped}/{len(ss)}")
        rows.append(row)
    return "\n".join("| " + " | ".join(r) + " |" for r in rows)


HEADER = """\
# Command-spec coverage report

> Auto-generated by `scripts/check_command_spec_coverage.py`. Do
> not edit by hand — re-run the script and commit the result. The
> per-command checklist that drives the columns lives in
> [`docs/rust-rewrite.md`](../rust-rewrite.md) § *Command-spec
> tracking*.

Status legend:

* `✓` — field is set on the spec (non-`None` / non-empty).
* `—` — field is left at `..CommandSpec::DEFAULT`. The audit
  cannot tell if the default is correct or unstamped — treat as
  *not yet verified*.
* `n/a` — column does not apply to this dialect (e.g. runtime
  hooks for iRules / iApps / Tk / Expect / EDA, which have no
  Rust / WASM / TclVM runtime).

The `verified` column reflects the
`// VERIFIED: <canonical source>` doc-line convention. Add the
line above the spec when you have audited the spec against
manpages / F5 docs / vendor reference.
"""


def render_report(specs: list[dict[str, Any]]) -> str:
    parts = [HEADER, "## Per-dialect summary", "", render_summary(specs), ""]
    parts.append(f"Total specs audited: {len(specs)}")
    parts.append("")
    parts.append("## Per-command coverage")
    parts.append("")
    parts.append(render_per_command_table(specs))
    parts.append("")
    return "\n".join(parts)


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument(
        "--check",
        action="store_true",
        help="exit non-zero if the report would change",
    )
    parser.add_argument(
        "--out",
        type=Path,
        default=DEFAULT_OUT,
        help=f"output path (default: {DEFAULT_OUT.relative_to(REPO_ROOT)})",
    )
    args = parser.parse_args()

    specs = collect_specs()
    report = render_report(specs)

    if args.check:
        existing = args.out.read_text() if args.out.exists() else ""
        if existing != report:
            print(
                f"command-spec coverage report at {args.out} is stale.",
                file=sys.stderr,
            )
            print(
                "Re-run `python3 scripts/check_command_spec_coverage.py`.",
                file=sys.stderr,
            )
            return 1
        return 0

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(report)
    print(f"wrote {args.out.relative_to(REPO_ROOT)} ({len(specs)} specs)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
