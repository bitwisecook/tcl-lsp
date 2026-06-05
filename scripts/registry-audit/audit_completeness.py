#!/usr/bin/env python3
"""Per-entry completeness audit: assert every Rust command spec carries
**at least as much** information as its Python equivalent, for every
command in every registry group.

For each command present on both sides it checks, dimension by dimension,
that Rust ⊇ / ≥ Python:

  presence  : hover, summary, source, source_is_url, examples,
              return_value, required_package, return_type,
              event_requires_any
  counts ≥  : n_forms, n_subcommands, n_side_effects, n_arg_roles,
              n_arg_types, len(synopsis)
  superset  : options, subcommands, event_profiles, event_also_in,
              excluded_events

Reports each deficient (command, dimension) and exits non-zero if any
deficiency exists. (Pure value differences where both sides carry the
dimension — e.g. a reworded summary — are *not* deficiencies.)

Usage: python3 scripts/registry-audit/audit_completeness.py [group ...]
       (run scripts/registry-audit/run_all.sh first to refresh dumps)
"""

from __future__ import annotations

import json
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
OUT = ROOT / "tmp/registry-audit"
GROUPS = "tcl stdlib tcllib irules iapps tk expect sdc-base synopsys cadence xilinx quartus mentor".split()

PRESENCE = [
    "hover", "summary", "source", "source_is_url", "examples",
    "return_value", "required_package", "return_type", "event_requires_any",
]
COUNTS = ["n_forms", "n_subcommands", "n_side_effects", "n_arg_roles", "n_arg_types"]
SETS = ["options", "subcommands", "event_profiles", "event_also_in", "excluded_events"]


def truthy(v) -> bool:
    return bool(v) and v not in ("", None, "null")


def load(path: Path) -> dict:
    out = {}
    for line in path.read_text().splitlines():
        d = json.loads(line)
        out[d["name"]] = d
    return out


def check_group(group: str) -> list[tuple[str, str, str]]:
    py = load(OUT / f"{group}.python.jsonl")
    ru = load(OUT / f"{group}.rust.jsonl")
    defs: list[tuple[str, str, str]] = []
    for name, p in py.items():
        r = ru.get(name)
        if r is None:
            continue  # name parity handled elsewhere
        for dim in PRESENCE:
            if truthy(p.get(dim)) and not truthy(r.get(dim)):
                defs.append((name, dim, f"py={p.get(dim)!r} rust={r.get(dim)!r}"))
        for dim in COUNTS:
            if (r.get(dim) or 0) < (p.get(dim) or 0):
                defs.append((name, dim, f"py={p.get(dim)} rust={r.get(dim)}"))
        if len(r.get("synopsis") or []) < len(p.get("synopsis") or []):
            defs.append((name, "synopsis_len", f"py={len(p['synopsis'])} rust={len(r.get('synopsis') or [])}"))
        for dim in SETS:
            missing = set(p.get(dim) or []) - set(r.get(dim) or [])
            if missing:
                defs.append((name, dim, f"missing {sorted(missing)}"))
    return defs


def main() -> None:
    groups = sys.argv[1:] or GROUPS
    total = 0
    for g in groups:
        defs = check_group(g)
        total += len(defs)
        if defs:
            print(f"\n### {g}: {len(defs)} deficiencies")
            by_dim: dict[str, list] = {}
            for name, dim, detail in defs:
                by_dim.setdefault(dim, []).append((name, detail))
            for dim, items in sorted(by_dim.items()):
                names = ", ".join(n for n, _ in items[:12])
                more = f" … (+{len(items) - 12})" if len(items) > 12 else ""
                print(f"  {dim} ({len(items)}): {names}{more}")
        else:
            print(f"{g}: ✅ complete (every entry ≥ Python)")
    print(f"\nTOTAL deficiencies: {total}")
    sys.exit(1 if total else 0)


if __name__ == "__main__":
    main()
