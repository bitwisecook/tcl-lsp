#!/usr/bin/env python3
"""Deep content-level completeness audit: for every command in every
registry, assert the Rust spec carries **at least as much data** as the
Python spec, field by field, down to per-subcommand content, option
attributes, side-effect targets, taint metadata and behavioural traits.

Compares the deep dumps from `dump_python_deep.py` and
`dump_specs deep-<group>`. Reports each (command[, subcommand], field)
where Python has data Rust lacks; exits non-zero if any.

Usage: python3 scripts/registry-audit/audit_deep.py [group ...]
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RUST_BIN = ROOT / "target/debug/examples/dump_specs"
GROUPS = "tcl stdlib tcllib irules iapps tk expect sdc-base synopsys cadence xilinx quartus mentor".split()


def norm(v):
    """Normalise an enum-ish token for cross-language compare."""
    return v.replace("_", "").lower() if isinstance(v, str) else v


def load_py(group: str) -> dict:
    out = subprocess.run(
        [sys.executable, str(ROOT / "scripts/registry-audit/dump_python_deep.py"), group],
        capture_output=True, text=True, cwd=ROOT,
    )
    if out.returncode:
        raise SystemExit(f"python deep dump failed for {group}: {out.stderr[-2000:]}")
    return {json.loads(l)["name"]: json.loads(l) for l in out.stdout.splitlines()}


def load_rust(group: str) -> dict:
    out = subprocess.run([str(RUST_BIN), f"deep-{group}"], capture_output=True, text=True)
    if out.returncode:
        raise SystemExit(f"rust deep dump failed for {group}: {out.stderr[-2000:]}")
    return {json.loads(l)["name"]: json.loads(l) for l in out.stdout.splitlines()}


def deficient_scalar(field, pv, rv) -> bool:
    """True when Python has data for a scalar/list the Rust side lacks."""
    if pv in (None, "", [], False):
        return False
    if isinstance(pv, list):
        return not set(map(norm_token, pv)) <= set(map(norm_token, rv or []))
    if isinstance(pv, bool):
        return bool(pv) and not bool(rv)
    # scalar string/enum/int
    if isinstance(pv, str):
        return norm(pv) != norm(rv) if rv else True
    return pv != rv and rv in (None, "")


def norm_token(x):
    return norm(x) if isinstance(x, str) else x


def check_sub(name, sub_py, sub_ru, defs):
    for f in ("return_type",):
        if sub_py.get(f) and norm(sub_py[f]) != norm(sub_ru.get(f) or ""):
            defs.append((name, f"sub:{f}", f"py={sub_py[f]} rust={sub_ru.get(f)}"))
    for f in ("pure", "mutator", "destructive", "returns_path", "is_unescape",
              "loop_list_header", "creates_scope_alias", "safe_on_uninit"):
        if sub_py.get(f) and not sub_ru.get(f):
            defs.append((name, f"sub:{f}", "py=True rust=False"))
    for f in ("n_arg_types", "n_arg_roles", "n_arg_values", "n_forms"):
        if (sub_py.get(f) or 0) > (sub_ru.get(f) or 0):
            defs.append((name, f"sub:{f}", f"py={sub_py.get(f)} rust={sub_ru.get(f)}"))
    # option names ⊆
    po = {o.split("|")[0] for o in sub_py.get("options") or []}
    ro = {o.split("|")[0] for o in sub_ru.get("options") or []}
    if po - ro:
        defs.append((name, "sub:options", f"missing {sorted(po - ro)}"))
    if len(sub_py.get("side_effects") or []) > len(sub_ru.get("side_effects") or []):
        defs.append((name, "sub:side_effects", f"py={len(sub_py['side_effects'])} rust={len(sub_ru.get('side_effects') or [])}"))
    if sub_py.get("credential_arg") is not None and sub_ru.get("credential_arg") is None:
        defs.append((name, "sub:credential_arg", "missing"))
    for f in ("sensitive_headers",):
        if set(sub_py.get(f) or []) - set(sub_ru.get(f) or []):
            defs.append((name, f"sub:{f}", "missing"))
    for f in ("cfg_rewrite_name", "taint_output_sink"):
        if sub_py.get(f) and not sub_ru.get(f):
            defs.append((name, f"sub:{f}", f"py={sub_py[f]}"))
    for f in ("taint_transform", "taint_double_encode", "inferred_storage_type"):
        pv = sub_py.get(f)
        if pv:
            rv = sub_ru.get(f)
            if isinstance(pv, list):
                if set(map(norm_token, pv)) - set(map(norm_token, rv or [])):
                    defs.append((name, f"sub:{f}", "missing"))
            elif not rv or norm(str(pv)) != norm(str(rv)):
                defs.append((name, f"sub:{f}", f"py={pv}"))


def check_group(group: str):
    py, ru = load_py(group), load_rust(group)
    defs = []
    for name, p in py.items():
        r = ru.get(name)
        if r is None:
            continue
        if p.get("snippet") and not r.get("snippet"):
            defs.append((name, "snippet", "py has snippet, rust empty"))
        if len(p.get("side_effects") or []) > len(r.get("side_effects") or []):
            defs.append((name, "side_effects", f"py={len(p['side_effects'])} rust={len(r.get('side_effects') or [])}"))
        po = {o.split("|")[0] for o in p.get("options") or []}
        ro = {o.split("|")[0] for o in r.get("options") or []}
        if po - ro:
            defs.append((name, "options", f"missing {sorted(po - ro)}"))
        # option attribute parity (takes_value/value_hint) on shared names
        rmap = {o.split("|")[0]: o for o in r.get("options") or []}
        for o in p.get("options") or []:
            n = o.split("|")[0]
            if n in rmap and o.split("|")[1] != rmap[n].split("|")[1]:
                defs.append((name, "option_takes_value", f"{n}: py={o} rust={rmap[n]}"))
        for f, pv in (p.get("scalars") or {}).items():
            rv = (r.get("scalars") or {}).get(f)
            if deficient_scalar(f, pv, rv):
                defs.append((name, f"scalar:{f}", f"py={pv!r} rust={rv!r}"))
        for f, pv in (p.get("bools") or {}).items():
            if pv and not (r.get("bools") or {}).get(f):
                defs.append((name, f"trait:{f}", "py=True rust=False"))
        for sn, sp in (p.get("subs") or {}).items():
            sr = (r.get("subs") or {}).get(sn)
            if sr is None:
                defs.append((name, "sub:MISSING", sn))
                continue
            check_sub(f"{name} {sn}", sp, sr, defs)
    return defs


def main():
    groups = sys.argv[1:] or GROUPS
    total = 0
    for g in groups:
        defs = check_group(g)
        total += len(defs)
        if defs:
            print(f"\n### {g}: {len(defs)} deep deficiencies")
            by_field = {}
            for nm, field, detail in defs:
                by_field.setdefault(field.split(":")[0] if ":" in field else field, []).append((nm, field, detail))
            for cat, items in sorted(by_field.items()):
                ex = "; ".join(f"{n}[{f}]" for n, f, _ in items[:6])
                print(f"  {cat} ({len(items)}): {ex}{' …' if len(items) > 6 else ''}")
        else:
            print(f"{g}: ✅ deep-complete")
    print(f"\nTOTAL deep deficiencies: {total}")
    sys.exit(1 if total else 0)


if __name__ == "__main__":
    main()
