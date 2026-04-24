#!/usr/bin/env python3
"""Audit WASM command parity between the Python registry and the Zig runtime.

Phase A.1 (current): inventory mode.  Walks every location where a Tcl
command is named and writes a JSON snapshot to ``tmp/wasm_command_inventory.json``.

Locations covered:

- Python command registry under ``core/commands/registry/tcl/`` (filtered to
  the Tcl 8.4-9.0 dialect set)
- Zig builtins assembled in ``runtime/zig/tcl_cmd_table.zig`` from
  ``runtime/zig/cmds/*.zig``
- Zig stub fallback dispatch in ``runtime/zig/tcl_cmd_dispatch.zig``
- Zig stub exports in ``runtime/zig/tcl_*_stubs.zig``
- Every ``pub export fn`` across ``runtime/zig/`` (FFI surface)
- ``_imports.py`` runtime/import tables consumed by the WASM emitter

Later phases (A.2-A.6) add a status classifier, dangling-entry detection,
baseline diffing, and CI wiring.
"""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
ZIG_DIR = ROOT / "runtime" / "zig"
TMP_DIR = ROOT / "tmp"

TCL_DIALECTS: frozenset[str] = frozenset({"tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0"})


# Zig source parsers

_REGISTRATION_NAME_RE = re.compile(r'\.name\s*=\s*"([^"]+)"')
_REGISTRATION_FULL_RE = re.compile(
    r'\.name\s*=\s*"([^"]+)"\s*,\s*\.handler\s*=\s*&(\w+)',
)
_DISPATCH_TRAP_RE = re.compile(r'eql\(c,\s*"([^"]+)"\)\s*\)\s*return\s+trap\(')
_DISPATCH_FALSE_RE = re.compile(r'eql\(c,\s*"([^"]+)"\)\s*\)\s*return\s+false')
# Data-table form: ``const STUB_TRAP: []const []const u8 = &.{ "a", "b", ... };``
# Captures the body between ``&.{`` and the closing ``}``.
_STUB_TRAP_TABLE_RE = re.compile(
    r"const\s+STUB_TRAP\s*:\s*\[\]const\s+\[\]const\s+u8\s*=\s*&\.\{([^}]+)\}"
)
_QUOTED_NAME_RE = re.compile(r'"([^"]+)"')
# Matches both ``pub export fn name(...)`` and the keyword-escape form
# ``pub export fn @"name"(...)`` Zig requires when exporting a symbol
# whose name collides with a Zig keyword (e.g. ``namespace``).
_EXPORT_FN_RE = re.compile(r'pub export fn\s+(?:@"([^"]+)"|(\w+))\s*\(')


def _export_name(match: re.Match[str]) -> str:
    return match.group(1) or match.group(2)


# Handlers in ``cmds/stubs_.zig`` that silently return an empty/zero
# value instead of trapping.  The user wants every unimplemented
# command to raise a runtime trap, so commands routed through these
# handlers are flagged ``SILENT_STUB`` for cleanup.
SILENT_STUB_HANDLERS: frozenset[str] = frozenset(
    {
        "eval_auto_load",
        "eval_auto_noop",
        "eval_after",
        "eval_package",
    }
)


def parse_zig_builtins(zig_dir: Path) -> dict[str, dict[str, str]]:
    """Return ``{command_name: {module, handler}}`` for every BUILTINS entry.

    Reads every ``cmds/*.zig`` file and extracts ``.name = "…", .handler = &…``
    pairs from registration arrays.  The handler name is captured so the
    classifier can flag silent-stub registrations (see
    :data:`SILENT_STUB_HANDLERS`).
    """
    result: dict[str, dict[str, str]] = {}
    cmds_dir = zig_dir / "cmds"
    for src in sorted(cmds_dir.glob("*.zig")):
        text = src.read_text()
        for match in _REGISTRATION_FULL_RE.finditer(text):
            name, handler = match.group(1), match.group(2)
            result.setdefault(name, {"module": src.name, "handler": handler})
    return result


def parse_zig_stub_dispatch(zig_dir: Path) -> dict[str, str]:
    """Return ``{command_name: action}`` from ``tcl_cmd_dispatch.zig``.

    *action* is ``"trap"`` when the command traps via
    ``stubs.unsupported("…")`` — either through the legacy if-chain form
    (``if (eql(c, "X")) return trap("X");``) or through the data-table
    form (``const STUB_TRAP = &.{ "X", "Y", ... };``).
    *action* is ``"fallthrough"`` for names in the legacy
    ``return false`` form (used for commands served elsewhere).
    """
    src = (zig_dir / "tcl_cmd_dispatch.zig").read_text()
    out: dict[str, str] = {}
    # Legacy per-line forms.
    for match in _DISPATCH_TRAP_RE.finditer(src):
        out[match.group(1)] = "trap"
    for match in _DISPATCH_FALSE_RE.finditer(src):
        out.setdefault(match.group(1), "fallthrough")
    # Data-table form — extract quoted names from inside the STUB_TRAP
    # slice literal.
    for tbl in _STUB_TRAP_TABLE_RE.finditer(src):
        for name_match in _QUOTED_NAME_RE.finditer(tbl.group(1)):
            out[name_match.group(1)] = "trap"
    return out


def parse_zig_stub_exports(zig_dir: Path) -> dict[str, str]:
    """Return ``{export_fn_name: source_module}`` for ``tcl_*_stubs.zig`` files."""
    result: dict[str, str] = {}
    for src in sorted(zig_dir.glob("tcl_*_stubs.zig")):
        text = src.read_text()
        for match in _EXPORT_FN_RE.finditer(text):
            result[_export_name(match)] = src.name
    return result


def parse_zig_all_exports(zig_dir: Path) -> dict[str, str]:
    """Return ``{export_fn_name: relative_path}`` for every ``pub export fn``."""
    result: dict[str, str] = {}
    for src in sorted(zig_dir.rglob("*.zig")):
        text = src.read_text()
        for match in _EXPORT_FN_RE.finditer(text):
            result[_export_name(match)] = str(src.relative_to(zig_dir))
    return result


# Python registry / imports introspection


def collect_registry_commands() -> dict[str, dict]:
    """Return ``{name: {dialects, has_wasm_codegen, subcommands_with_wasm_codegen}}``.

    The set is restricted to **Tcl core** commands — those declared under
    ``core/commands/registry/tcl/`` — and further filtered to specs that
    apply to at least one of Tcl 8.4, 8.5, 8.6, or 9.0.  Tk, Tcllib, EDA,
    iRules, and other dialect packs are intentionally excluded: they have
    no business in the WASM runtime.
    """
    sys.path.insert(0, str(ROOT))
    # Importing the WASM emitter cmds package fires the
    # ``REGISTRY.register_wasm_emitter("…", _emit_…)`` calls inside each
    # ``cmds/*.py`` module, which is what populates ``codegens["wasm"]``
    # on the affected specs.  Without this the registry knows nothing
    # about the WASM hooks and every command looks unimplemented.
    import core.compiler.codegen.wasm._emitter.cmds  # noqa: F401, PLC0415
    from core.commands.registry import REGISTRY  # noqa: PLC0415
    from core.commands.registry.tcl._base import _REGISTRY as TCL_DEFS  # noqa: PLC0415

    # Commands keyed by name, with their source-module short name (e.g.
    # ``mathop``, ``oo``, ``set_``).  Used by the classifier to bucket
    # auto-generated ``tcl::mathop::*`` operator commands separately from
    # ordinary Tcl commands.
    tcl_core_sources: dict[str, str] = {}
    for cls in TCL_DEFS:
        module = cls.__module__.rsplit(".", 1)[-1]
        tcl_core_sources.setdefault(cls.name, module)

    out: dict[str, dict] = {}
    for name in sorted(tcl_core_sources):
        specs = REGISTRY.specs_by_name.get(name)
        if specs is None:
            continue
        applicable_dialects: set[str] | None = None
        has_wasm = False
        sub_wasm: set[str] = set()
        in_tcl = False
        for spec in specs:
            spec_dialects = spec.dialects
            if spec_dialects is None:
                in_tcl = True
                applicable_dialects = None  # ``None`` means "all"
            elif TCL_DIALECTS & spec_dialects:
                in_tcl = True
                inter = TCL_DIALECTS & spec_dialects
                if applicable_dialects is not None:
                    applicable_dialects |= inter
                elif not in_tcl:
                    applicable_dialects = set(inter)
            if "wasm" in spec.codegens:
                has_wasm = True
            for sub_name, sub in spec.subcommands.items():
                if "wasm" in sub.codegens:
                    sub_wasm.add(sub_name)
        if not in_tcl:
            continue
        dialects = (
            sorted(applicable_dialects) if applicable_dialects is not None else sorted(TCL_DIALECTS)
        )
        out[name] = {
            "dialects": dialects,
            "source_module": tcl_core_sources[name],
            "has_wasm_codegen": has_wasm,
            "subcommands_with_wasm_codegen": sorted(sub_wasm),
        }
    return out


def collect_imports_tables() -> dict[str, object]:
    """Pull tables from ``_imports.py`` via direct import."""
    sys.path.insert(0, str(ROOT))
    # ``cmd_runtime`` used to mirror the retired ``_CMD_RUNTIME`` dict.
    # Its spec-backed successor is reconstructed from every
    # ``CommandSpec.wasm_runtime_import`` declaration in the registry
    # (populated by phase B.7c).  ``cmd_runtime_nontrapping`` is derived
    # from the same source.
    from core.commands.registry import REGISTRY  # noqa: PLC0415
    from core.compiler.codegen.wasm import _imports as imp  # noqa: PLC0415

    cmd_runtime: dict[str, dict[str, object]] = {}
    cmd_runtime_nontrapping: list[str] = []
    # Per-parent maps of ``<parent> <subcommand> -> import_key``, also
    # derived from the registry (populated by B.8).
    string_subcmd_import: dict[str, str] = {}
    dict_subcmd_import: dict[str, str] = {}
    clock_subcmd_import: dict[str, str] = {}
    subcmd_targets = {
        "string": string_subcmd_import,
        "dict": dict_subcmd_import,
        "clock": clock_subcmd_import,
    }
    for name, specs in REGISTRY.specs_by_name.items():
        for spec in specs:
            rimp = spec.wasm_runtime_import
            if rimp is not None:
                cmd_runtime.setdefault(
                    name, {"import": rimp.import_key, "argc": rimp.argc}
                )
                if rimp.nontrapping and name not in cmd_runtime_nontrapping:
                    cmd_runtime_nontrapping.append(name)
            target = subcmd_targets.get(name)
            if target is not None:
                for sub_name, sub in spec.subcommands.items():
                    if sub.wasm_runtime_import is not None:
                        target.setdefault(sub_name, sub.wasm_runtime_import.import_key)

    return {
        "runtime_imports": {
            key: {
                "module": v[0],
                "export": v[1],
                "params": [p.name for p in v[2]],
                "results": [r.name for r in v[3]],
            }
            for key, v in imp._RUNTIME_IMPORTS.items()  # noqa: SLF001
        },
        "cmd_runtime": cmd_runtime,
        "cmd_runtime_nontrapping": sorted(cmd_runtime_nontrapping),
        "string_subcmd_import": string_subcmd_import,
        "string_is_import": dict(imp._STRING_IS_IMPORT),  # noqa: SLF001
        "dict_subcmd_import": dict_subcmd_import,
        "clock_subcmd_import": clock_subcmd_import,
        "scope_nop_commands": sorted(
            name
            for name, specs in REGISTRY.specs_by_name.items()
            if any(spec.wasm_emits_nothing for spec in specs)
        ),
    }


def build_inventory() -> dict[str, object]:
    return {
        "tcl_dialects": sorted(TCL_DIALECTS),
        "registry_commands": collect_registry_commands(),
        "zig_builtins": parse_zig_builtins(ZIG_DIR),
        "zig_stub_dispatch": parse_zig_stub_dispatch(ZIG_DIR),
        "zig_stub_exports": parse_zig_stub_exports(ZIG_DIR),
        "zig_all_exports": parse_zig_all_exports(ZIG_DIR),
        "imports_py": collect_imports_tables(),
    }


# Status classification

# Status values:
#   IMPLEMENTED   — handled by a real Zig handler in BUILTINS
#   TRAPPING_STUB — fallback dispatch raises ``unsupported command: X``
#   SILENT_STUB   — BUILTINS entry routes to a silent no-op handler
#                   (eval_auto_noop / eval_after / eval_package / eval_auto_load)
#   MISSING       — neither handler nor stub; calling traps with
#                   ``unknown command: X`` only by virtue of the proc-lookup miss
#   NOT_REQUIRED  — explicitly out of scope (mathop prefix-form operators)

# Rank from worst to best — used by baseline diffing to detect
# regressions.  A current rank lower than the baseline rank for the same
# command is a regression and fails CI.
_STATUS_RANK: dict[str, int] = {
    "MISSING": 0,
    "SILENT_STUB": 1,
    "TRAPPING_STUB": 2,
    "IMPLEMENTED": 3,
    "NOT_REQUIRED": 4,
}


def classify(inventory: dict) -> dict[str, dict]:
    """Return ``{command_name: {status, …}}`` for every registry command."""
    registry = inventory["registry_commands"]
    builtins = inventory["zig_builtins"]
    dispatch = inventory["zig_stub_dispatch"]

    statuses: dict[str, dict] = {}
    for name, info in registry.items():
        source = info["source_module"]
        if source == "mathop":
            status = "NOT_REQUIRED"
            reason = "tcl::mathop::* prefix-form operator (handled at expr-compile time)"
            handler = None
        elif name in builtins:
            handler = builtins[name]["handler"]
            if handler in SILENT_STUB_HANDLERS:
                status = "SILENT_STUB"
                reason = f"BUILTINS handler {handler!r} returns empty value without trapping"
            else:
                status = "IMPLEMENTED"
                reason = f"BUILTINS handler {handler!r} in {builtins[name]['module']}"
        elif name in dispatch:
            handler = None
            action = dispatch[name]
            if action == "trap":
                status = "TRAPPING_STUB"
                reason = "tcl_cmd_dispatch.zig fallback raises unsupported command trap"
            else:
                status = "MISSING"
                reason = "tcl_cmd_dispatch.zig declines (return false) and no BUILTINS entry"
        else:
            handler = None
            status = "MISSING"
            reason = "no BUILTINS entry, no fallback dispatch entry"
        statuses[name] = {
            "status": status,
            "reason": reason,
            "handler": handler,
            "dialects": info["dialects"],
            "has_python_wasm_codegen": info["has_wasm_codegen"],
        }
    return statuses


# Dangling-entry detection


def find_dangling(inventory: dict) -> dict[str, list]:
    """Find entries in the WASM stack that aren't traceable to the registry.

    Reports:

    - ``orphan_builtins``: Zig BUILTINS entries not present in the
      Tcl-core registry (excluding entries we've classed NOT_REQUIRED
      such as mathop operators, which the registry does carry but for
      which a Zig handler would be unexpected).
    - ``orphan_dispatch``: ``tcl_cmd_dispatch.zig`` stub entries with no
      matching registry command.
    - ``orphan_cmd_runtime``: ``_imports.py:_CMD_RUNTIME`` keys not in
      the registry.
    - ``ffi_imports_without_export``: ``_RUNTIME_IMPORTS`` declarations
      whose Zig export name doesn't appear in any ``runtime/zig/*.zig``
      file (would produce a link error at WASM instantiation).
    - ``ffi_exports_without_import``: ``pub export fn`` symbols in
      ``runtime/zig/`` that no Python import declaration references.
      Sometimes intentional (helpers callable from compiled procs via
      different mechanisms) so this is an informational signal.
    """
    registry_names = set(inventory["registry_commands"].keys())
    builtins_names = set(inventory["zig_builtins"].keys())
    dispatch_names = set(inventory["zig_stub_dispatch"].keys())

    orphan_builtins = sorted(builtins_names - registry_names)
    orphan_dispatch = sorted(dispatch_names - registry_names)

    py = inventory["imports_py"]
    cmd_runtime = py["cmd_runtime"]
    runtime_imports = py["runtime_imports"]
    orphan_cmd_runtime = sorted(set(cmd_runtime) - registry_names)

    zig_exports = set(inventory["zig_all_exports"])
    declared_exports = {entry["export"] for entry in runtime_imports.values()}

    ffi_imports_without_export = sorted(
        {
            key: entry["export"]
            for key, entry in runtime_imports.items()
            if entry["export"] not in zig_exports
        }.items()
    )
    ffi_exports_without_import = sorted(zig_exports - declared_exports)

    return {
        "orphan_builtins": orphan_builtins,
        "orphan_dispatch": orphan_dispatch,
        "orphan_cmd_runtime": orphan_cmd_runtime,
        "ffi_imports_without_export": ffi_imports_without_export,
        "ffi_exports_without_import": ffi_exports_without_import,
    }


def print_dangling_report(dangling: dict[str, list]) -> None:
    print()
    print("=== Dangling entries ===")
    print(f"  orphan_builtins:                 {len(dangling['orphan_builtins'])}")
    print(f"  orphan_dispatch:                 {len(dangling['orphan_dispatch'])}")
    print(f"  orphan_cmd_runtime:              {len(dangling['orphan_cmd_runtime'])}")
    print(f"  ffi_imports_without_export:      {len(dangling['ffi_imports_without_export'])}")
    print(f"  ffi_exports_without_import:      {len(dangling['ffi_exports_without_import'])}")

    for key in (
        "orphan_builtins",
        "orphan_dispatch",
        "orphan_cmd_runtime",
        "ffi_imports_without_export",
    ):
        items = dangling[key]
        if not items:
            continue
        print()
        print(f"=== {key} ({len(items)}) ===")
        for entry in items:
            if isinstance(entry, tuple):
                print(f"  {entry[0]:30}  → {entry[1]}")
            else:
                print(f"  {entry}")


def print_status_report(statuses: dict[str, dict]) -> None:
    """Print a human-readable summary of the classifier output."""
    counts: dict[str, int] = {}
    by_status: dict[str, list[str]] = {}
    for name, s in statuses.items():
        counts[s["status"]] = counts.get(s["status"], 0) + 1
        by_status.setdefault(s["status"], []).append(name)

    print()
    print("=== Command-status summary (Tcl 8.4-9.0 core) ===")
    for status in ("IMPLEMENTED", "TRAPPING_STUB", "SILENT_STUB", "MISSING", "NOT_REQUIRED"):
        n = counts.get(status, 0)
        print(f"  {status:14} {n:4}")

    for status in ("SILENT_STUB", "MISSING"):
        names = sorted(by_status.get(status, []))
        if not names:
            continue
        print()
        print(f"=== {status} ({len(names)}) ===")
        for n in names:
            entry = statuses[n]
            print(f"  {n:30}  {entry['reason']}")


# Baseline + diff


def build_snapshot(inventory: dict) -> dict[str, object]:
    """Return the deterministic snapshot to compare against a baseline.

    The snapshot includes per-command status (so regressions are visible
    per command) and the dangling sets (so new orphans fail).  Reasons
    and free-form descriptions are excluded — they would churn the
    snapshot every time a stub message changes.
    """
    statuses = classify(inventory)
    dangling = find_dangling(inventory)
    return {
        "schema_version": 1,
        "command_status": {name: s["status"] for name, s in statuses.items()},
        "dangling": {
            "orphan_builtins": dangling["orphan_builtins"],
            "orphan_dispatch": dangling["orphan_dispatch"],
            "orphan_cmd_runtime": dangling["orphan_cmd_runtime"],
            "ffi_imports_without_export": [
                {"key": k, "export": v} for k, v in dangling["ffi_imports_without_export"]
            ],
        },
    }


def diff_against_baseline(
    current: dict[str, object],
    baseline: dict[str, object],
) -> tuple[list[str], list[str]]:
    """Compare *current* snapshot to *baseline*.

    Returns ``(regressions, improvements)`` — lists of human-readable
    description strings.  A regression is anything that strictly
    worsens (status rank drops, new orphan, new FFI mismatch, or a new
    command lands with status worse than IMPLEMENTED without being
    explicitly allowed).  Anything else is treated as an improvement
    (status climbs, orphan removed, command removed from registry).
    """
    regressions: list[str] = []
    improvements: list[str] = []

    cur_status = current["command_status"]
    base_status = baseline["command_status"]

    for name in sorted(set(cur_status) | set(base_status)):
        cur = cur_status.get(name)
        base = base_status.get(name)
        if cur == base:
            continue
        if base is None:
            # New command in registry.  Only IMPLEMENTED, TRAPPING_STUB,
            # or NOT_REQUIRED are acceptable for new arrivals — anything
            # less means we shipped a registry entry without backing it
            # in the runtime.
            if _STATUS_RANK[cur] < _STATUS_RANK["TRAPPING_STUB"]:
                regressions.append(f"NEW {name!r}: status={cur} (must be at least TRAPPING_STUB)")
            else:
                improvements.append(f"NEW {name!r}: status={cur}")
            continue
        if cur is None:
            improvements.append(f"REMOVED {name!r} (was {base})")
            continue
        if _STATUS_RANK[cur] < _STATUS_RANK[base]:
            regressions.append(f"REGRESSION {name!r}: {base} → {cur}")
        else:
            improvements.append(f"IMPROVED {name!r}: {base} → {cur}")

    cur_d = current["dangling"]
    base_d = baseline["dangling"]
    for key in ("orphan_builtins", "orphan_dispatch", "orphan_cmd_runtime"):
        new = sorted(set(cur_d[key]) - set(base_d[key]))
        gone = sorted(set(base_d[key]) - set(cur_d[key]))
        for entry in new:
            regressions.append(f"NEW {key}: {entry}")
        for entry in gone:
            improvements.append(f"REMOVED {key}: {entry}")

    cur_ffi = {(d["key"], d["export"]) for d in cur_d["ffi_imports_without_export"]}
    base_ffi = {(d["key"], d["export"]) for d in base_d["ffi_imports_without_export"]}
    for key, exp in sorted(cur_ffi - base_ffi):
        regressions.append(f"NEW ffi_imports_without_export: {key} → {exp}")
    for key, exp in sorted(base_ffi - cur_ffi):
        improvements.append(f"REMOVED ffi_imports_without_export: {key} → {exp}")

    return regressions, improvements


DEFAULT_BASELINE = ROOT / "tests" / "baselines" / "wasm_command_parity.json"


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--out",
        type=Path,
        default=TMP_DIR / "wasm_command_inventory.json",
        help="Where to write the full inventory JSON (default: tmp/).",
    )
    parser.add_argument(
        "--report",
        action="store_true",
        help="Print a human-readable status + dangling report.",
    )
    parser.add_argument(
        "--baseline",
        type=Path,
        default=DEFAULT_BASELINE,
        help="Path to the parity baseline JSON.",
    )
    parser.add_argument(
        "--snapshot",
        action="store_true",
        help="Write the current snapshot to --baseline (overwriting any existing).",
    )
    parser.add_argument(
        "--check",
        action="store_true",
        help=(
            "Compare current snapshot against --baseline; exit 1 on regression."
            "  Improvements (statuses climbing, orphans removed) are reported"
            " but do not fail."
        ),
    )
    args = parser.parse_args()

    inventory = build_inventory()
    snapshot = build_snapshot(inventory)

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.write_text(json.dumps(inventory, indent=2, sort_keys=True))

    py = inventory["imports_py"]
    print(f"Inventory written to {args.out}")
    print(f"  registry commands (Tcl 8.4-9.0): {len(inventory['registry_commands'])}")
    print(f"  zig BUILTINS:                    {len(inventory['zig_builtins'])}")
    print(f"  zig stub dispatch entries:       {len(inventory['zig_stub_dispatch'])}")
    print(f"  zig stub exports (tcl_*_stubs):  {len(inventory['zig_stub_exports'])}")
    print(f"  zig pub export fn (total):       {len(inventory['zig_all_exports'])}")
    print(f"  _imports.py _CMD_RUNTIME:        {len(py['cmd_runtime'])}")
    print(f"  _imports.py _RUNTIME_IMPORTS:    {len(py['runtime_imports'])}")

    if args.report:
        statuses = classify(inventory)
        print_status_report(statuses)
        dangling = find_dangling(inventory)
        print_dangling_report(dangling)

    if args.snapshot:
        args.baseline.parent.mkdir(parents=True, exist_ok=True)
        args.baseline.write_text(json.dumps(snapshot, indent=2, sort_keys=True) + "\n")
        print(f"\nBaseline written to {args.baseline}")
        return 0

    if args.check:
        if not args.baseline.exists():
            print(f"\nERROR: baseline not found at {args.baseline}", file=sys.stderr)
            print("Run with --snapshot to create one.", file=sys.stderr)
            return 2
        baseline = json.loads(args.baseline.read_text())
        regressions, improvements = diff_against_baseline(snapshot, baseline)
        if improvements:
            print()
            print(f"=== Improvements ({len(improvements)}) ===")
            for line in improvements:
                print(f"  {line}")
        if regressions:
            print()
            print(f"=== Regressions ({len(regressions)}) ===", file=sys.stderr)
            for line in regressions:
                print(f"  {line}", file=sys.stderr)
            print(
                "\nWASM command parity check FAILED — see regressions above.",
                file=sys.stderr,
            )
            return 1
        print("\nWASM command parity check OK (no regressions vs baseline).")

    return 0


if __name__ == "__main__":
    sys.exit(main())
