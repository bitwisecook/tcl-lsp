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


def _se_map(seset):
    """`(target, side) -> (reads, writes)` (max), enum-name normalised."""
    m = {}
    for s in seset or []:
        p = s.split("|")
        if len(p) != 4:
            continue
        key = (norm(p[0]), norm(p[3]))
        r, w = int(p[1]), int(p[2])
        pr, pw = m.get(key, (0, 0))
        m[key] = (max(pr, r), max(pw, w))
    return m


def se_missing(py_ses, ru_ses):
    """Python side-effects (target/side + reads/writes) the Rust set does
    not cover (Rust extras and richer reads/writes are allowed)."""
    pm, rm = _se_map(py_ses), _se_map(ru_ses)
    miss = []
    for (t, side), (r, w) in pm.items():
        rr = rm.get((t, side))
        if rr is None or rr[0] < r or rr[1] < w:
            miss.append(f"{t}|{r}|{w}|{side}")
    return sorted(miss)


def load_py(group: str) -> dict:
    out = subprocess.run(
        [sys.executable, str(ROOT / "scripts/registry-audit/dump_python_deep.py"), group],
        capture_output=True,
        text=True,
        cwd=ROOT,
    )
    if out.returncode:
        raise SystemExit(f"python deep dump failed for {group}: {out.stderr[-2000:]}")
    return {json.loads(line)["name"]: json.loads(line) for line in out.stdout.splitlines()}


def load_rust(group: str) -> dict:
    out = subprocess.run([str(RUST_BIN), f"deep-{group}"], capture_output=True, text=True)
    if out.returncode:
        raise SystemExit(f"rust deep dump failed for {group}: {out.stderr[-2000:]}")
    return {json.loads(line)["name"]: json.loads(line) for line in out.stdout.splitlines()}


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


def opt_missing(py_opts, ru_opts):
    """Options ("name|takes_value|value_hint") where Python carries data
    Rust lacks: a missing option name, a differing takes_value, or a
    value_hint Python has but Rust dropped. A *differing* (both non-empty)
    value_hint is not a deficiency — Rust's hint may be more descriptive."""

    def parse(opts):
        m = {}
        for o in opts or []:
            p = (o.split("|") + ["", ""])[:3]
            m[p[0]] = (p[1], p[2])
        return m

    pm, rm = parse(py_opts), parse(ru_opts)
    miss = []
    for name, (tv, hint) in pm.items():
        if name not in rm:
            miss.append(f"{name} (absent)")
        elif rm[name][0] != tv:
            miss.append(f"{name} (takes_value py={tv} rust={rm[name][0]})")
        elif hint and not rm[name][1]:
            miss.append(f"{name} (value_hint dropped)")
    return sorted(miss)


def _argmap_defs(name, label, pymap, rumap, defs):
    """Compare an `{index: value|[values]}` map; Rust must match/⊇ Python."""
    pymap = pymap or {}
    rumap = rumap or {}
    for idx, pv in pymap.items():
        rv = rumap.get(idx)
        if isinstance(pv, list):
            if set(map(norm_token, pv)) - set(map(norm_token, rv or [])):
                defs.append((name, label, f"[{idx}] missing {pv}"))
        elif rv is None or norm(pv) != norm(rv):
            defs.append((name, label, f"[{idx}] py={pv} rust={rv}"))


def check_sub(name, sub_py, sub_ru, defs):
    for f in ("return_type",):
        if sub_py.get(f) and norm(sub_py[f]) != norm(sub_ru.get(f) or ""):
            defs.append((name, f"sub:{f}", f"py={sub_py[f]} rust={sub_ru.get(f)}"))
    for f in (
        "pure",
        "mutator",
        "destructive",
        "returns_path",
        "is_unescape",
        "loop_list_header",
        "creates_scope_alias",
        "safe_on_uninit",
    ):
        if sub_py.get(f) and not sub_ru.get(f):
            defs.append((name, f"sub:{f}", "py=True rust=False"))
    if (sub_py.get("n_forms") or 0) > (sub_ru.get("n_forms") or 0):
        defs.append(
            (name, "sub:n_forms", f"py={sub_py.get('n_forms')} rust={sub_ru.get('n_forms')}")
        )
    # arg_types / arg_roles / arg_values by CONTENT (per index)
    _argmap_defs(name, "sub:arg_types", sub_py.get("arg_types"), sub_ru.get("arg_types"), defs)
    _argmap_defs(name, "sub:arg_roles", sub_py.get("arg_roles"), sub_ru.get("arg_roles"), defs)
    _argmap_defs(name, "sub:arg_values", sub_py.get("arg_values"), sub_ru.get("arg_values"), defs)
    # options: name + takes_value required; a dropped value_hint flagged
    om = opt_missing(sub_py.get("options"), sub_ru.get("options"))
    if om:
        defs.append((name, "sub:options", f"missing {om}"))
    # side_effects by CONTENT (target|reads|writes|side multiset)
    miss = se_missing(sub_py.get("side_effects"), sub_ru.get("side_effects"))
    if miss:
        defs.append((name, "sub:side_effects", f"missing {miss}"))
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
        # side_effects by CONTENT (target|reads|writes|side multiset)
        miss = se_missing(p.get("side_effects"), r.get("side_effects"))
        if miss:
            defs.append((name, "side_effects", f"missing {miss}"))
        # options: name + takes_value required; a dropped value_hint flagged
        om = opt_missing(p.get("options"), r.get("options"))
        if om:
            defs.append((name, "options", f"missing {om}"))
        # command-level arg_types / arg_roles by content
        _argmap_defs(name, "arg_types", p.get("arg_types"), r.get("arg_types"), defs)
        _argmap_defs(name, "arg_roles", p.get("arg_roles"), r.get("arg_roles"), defs)
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
                by_field.setdefault(field.split(":")[0] if ":" in field else field, []).append(
                    (nm, field, detail)
                )
            for cat, items in sorted(by_field.items()):
                ex = "; ".join(f"{n}[{f}]" for n, f, _ in items[:6])
                print(f"  {cat} ({len(items)}): {ex}{' …' if len(items) > 6 else ''}")
        else:
            print(f"{g}: ✅ deep-complete")
    print(f"\nTOTAL deep deficiencies: {total}")
    sys.exit(1 if total else 0)


if __name__ == "__main__":
    main()
