#!/usr/bin/env python3
"""Generate the *entry-level* status section of rust-rewrite-registries.md.

Names every individual registry entry (one row per command / object / event)
with its Python↔Rust parity status, so drift can be tracked per entry — a
sorted, stable list is an exact `git diff` drift detector.

Output is a set of collapsible <details> blocks (one per registry). Reads the
per-group dumps in tmp/registry-audit/ (run scripts/registry-audit/run_all.sh
first) and introspects the Python BigIP / events / profiles registries directly.

Usage: python3 scripts/registry-audit/gen_entries.py >> rust-rewrite-registries.md
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))

OUT = _REPO_ROOT / "tmp" / "registry-audit"
ORDER = "tcl stdlib tcllib irules iapps tk expect sdc-base synopsys cadence xilinx quartus mentor".split()

_RUST_BIN = _REPO_ROOT / "target" / "debug" / "examples" / "dump_specs"


def ensure_rust_bin() -> str:
    """(Re)build the Rust dumper so meta dumps reflect current registry data.

    cargo is incremental, so this is cheap when up to date and guarantees the
    meta-registry comparison below never reads a stale binary.
    """
    subprocess.run(
        ["cargo", "build", "-q", "--example", "dump_specs", "-p", "tcl-registry"],
        cwd=_REPO_ROOT / "rust", check=True,
    )
    return str(_RUST_BIN)


def rust_meta_names(binpath: str, kind: str) -> set[str]:
    """Names emitted by `dump_specs meta-<kind>` (events/profiles/namespaces)."""
    out = subprocess.run(
        [binpath, f"meta-{kind}"], cwd=_REPO_ROOT,
        capture_output=True, text=True, check=True,
    ).stdout
    return {line.strip() for line in out.splitlines() if line.strip()}


def rust_event_props(binpath: str) -> dict[str, dict]:
    """Per-event props from `dump_specs meta-events-props`."""
    out = subprocess.run(
        [binpath, "meta-events-props"], cwd=_REPO_ROOT,
        capture_output=True, text=True, check=True,
    ).stdout
    props = {}
    for line in out.splitlines():
        if line.strip():
            rec = json.loads(line)
            props[rec["name"]] = rec
    return props

# Data-loss dimensions we surface per entry, with short status codes. Only
# "py has it, rust lacks it" counts as a gap (modelling-diff dims excluded).
GAP_DIMS: list[tuple[str, str]] = [
    ("forms", "forms"),
    ("options", "opt"),
    ("subcommands", "sub"),
    ("side_effects", "sfx"),
    ("event_requires", "evtreq"),
    ("event_profiles", "evtprof"),
    ("hover_examples", "ex"),
    ("hover_return_value", "ret"),
    ("hover_source_url", "srcurl"),
    ("required_package", "pkg"),
    ("excluded_events", "exev"),
    ("hover", "hover"),
]

DIM_PRED = {
    "forms": lambda r: r.get("n_forms", 0) > 0,
    "options": lambda r: bool(r.get("options")),
    "subcommands": lambda r: r.get("n_subcommands", 0) > 0,
    "side_effects": lambda r: r.get("n_side_effects", 0) > 0,
    "event_requires": lambda r: bool(r.get("event_requires_any")),
    "event_profiles": lambda r: bool(r.get("event_profiles")),
    "hover_examples": lambda r: bool(r.get("examples")),
    "hover_return_value": lambda r: bool(r.get("return_value")),
    "hover_source_url": lambda r: bool(r.get("source_is_url")),
    "required_package": lambda r: r.get("required_package") is not None,
    "excluded_events": lambda r: bool(r.get("excluded_events")),
    "hover": lambda r: bool(r.get("hover")),
}


def load(path: str) -> dict[str, dict]:
    out = {}
    for line in open(path):
        line = line.strip()
        if line:
            rec = json.loads(line)
            out[rec["name"]] = rec
    return out


def entry_status(py: dict | None, rs: dict | None) -> str:
    if py is None:
        return "➕ rust-only"
    if rs is None:
        return "✗ missing in rust"
    gaps = [code for dim, code in GAP_DIMS if DIM_PRED[dim](py) and not DIM_PRED[dim](rs)]
    if py.get("summary", "") != rs.get("summary", ""):
        gaps.append("sum≠")
    if py.get("synopsis", []) != rs.get("synopsis", []):
        gaps.append("syn≠")
    if (py.get("arity_min"), py.get("arity_max")) != (rs.get("arity_min"), rs.get("arity_max")):
        gaps.append("arity≠")
    return "✅" if not gaps else " ".join(f"`{g}`" for g in gaps)


def emit_group(group: str) -> None:
    py = load(str(OUT / f"{group}.python.jsonl"))
    rs = load(str(OUT / f"{group}.rust.jsonl"))
    names = sorted(set(py) | set(rs))
    statuses = {n: entry_status(py.get(n), rs.get(n)) for n in names}
    ok = sum(1 for s in statuses.values() if s == "✅")
    print(
        f"<details><summary><b>{group}</b> — {len(names)} entries · "
        f"{ok} ✅ · {len(names) - ok} need work</summary>\n"
    )
    print("| entry | status |")
    print("|---|---|")
    for n in names:
        print(f"| `{n}` | {statuses[n]} |")
    print("\n</details>\n")


def emit_bigip() -> None:
    from core.bigip.registry.data import OBJECT_SPECS  # type: ignore

    kinds = sorted(s.kind_spec.kind for s in OBJECT_SPECS)
    # main-vs-core drift via file stems
    def stems(ref: str, path: str) -> set[str]:
        out = subprocess.run(
            ["git", "ls-tree", "-r", "--name-only", ref, "--", path],
            capture_output=True, text=True, cwd=_REPO_ROOT,
        ).stdout
        return {
            line.rsplit("/", 1)[-1][:-3]
            for line in out.splitlines()
            if line.endswith(".py") and not line.endswith("__init__.py")
        }

    main_stems = stems("origin/main", "dialects/f5/bigip/registry/specs")
    core_stems = stems("HEAD", "core/bigip/registry/specs")
    main_only = sorted(main_stems - core_stems)
    core_only = sorted(core_stems - main_stems)
    shared = len(main_stems & core_stems)

    print(
        f"<details><summary><b>bigip object registry</b> — {len(kinds)} entries · "
        f"0 ✅ · {len(kinds)} unported (Rust has no BigIP registry)</summary>\n"
    )
    print("Every entry is **✗ unported**. Re-run after a Rust BigIP registry lands.\n")
    print("| object kind | status |")
    print("|---|---|")
    for k in kinds:
        print(f"| `{k}` | ✗ unported |")
    print("\n</details>\n")

    print(
        f"<details><summary><b>bigip — Python copy ↔ main divergence</b> — "
        f"{shared} shared · {len(main_only)} main-only · {len(core_only)} core-only "
        f"(spec file stems)</summary>\n"
    )
    print(
        "The rust-branch `core/bigip/registry/specs` and `origin/main` "
        "`dialects/f5/bigip/registry/specs` have **diverged in both directions** "
        "(sampled main-only kinds confirmed absent from core's `OBJECT_SPECS`). "
        "Reconcile before/with the Rust port.\n"
    )
    print(f"**main-only ({len(main_only)})** — present on main, missing from rust-branch core:\n")
    print(", ".join(f"`{k}`" for k in main_only) + "\n")
    print(
        f"**core-only ({len(core_only)})** — present on rust-branch core, not on main "
        "(verify renamed vs genuinely extra):\n"
    )
    print(", ".join(f"`{k}`" for k in core_only) + "\n")
    print("</details>\n")


def _name_status(n: str, py: set[str], rust: set[str]) -> str:
    if n in py and n in rust:
        return "✅"
    if n in py:
        return "✗ missing in rust"
    return "➕ rust-only"


def emit_meta_names(title: str, py_names: set[str], rust_names: set[str], note: str) -> None:
    """Per-entry name parity for a meta registry, diffed against the Rust dump."""
    names = sorted(py_names | rust_names)
    statuses = {n: _name_status(n, py_names, rust_names) for n in names}
    ok = sum(1 for s in statuses.values() if s == "✅")
    print(
        f"<details><summary><b>{title}</b> — {len(names)} entries · "
        f"{ok} ✅ · {len(names) - ok} need work</summary>\n"
    )
    print(f"{note}\n")
    print("| entry | status |")
    print("|---|---|")
    for n in names:
        print(f"| `{n}` | {statuses[n]} |")
    print("\n</details>\n")


def _norm_transport(t) -> list[str]:
    if t is None:
        return []
    if isinstance(t, str):
        return [t]
    return sorted(t)


def emit_events(py_props: dict, rust_props: dict[str, dict]) -> None:
    """Per-event name **and** EventProps-field parity, diffed against Rust."""
    names = sorted(set(py_props) | set(rust_props))
    statuses: dict[str, str] = {}
    for n in names:
        if n in py_props and n in rust_props:
            p, r = py_props[n], rust_props[n]
            checks = {
                "client_side": (p.client_side, r["client_side"]),
                "server_side": (p.server_side, r["server_side"]),
                "transport": (_norm_transport(p.transport), sorted(r["transport"])),
                "implied_profiles": (sorted(p.implied_profiles), sorted(r["implied_profiles"])),
                "flow": (p.flow, r["flow"]),
                "deprecated": (p.deprecated, r["deprecated"]),
                "hot": (p.hot, r["hot"]),
                "common": (p.common, r["common"]),
                "setup_event": (p.setup_event, r["setup_event"]),
            }
            diffs = [k for k, (a, b) in checks.items() if a != b]
            statuses[n] = "✅" if not diffs else "`props≠` " + ", ".join(diffs)
        else:
            statuses[n] = _name_status(n, set(py_props), set(rust_props))
    ok = sum(1 for s in statuses.values() if s == "✅")
    print(
        f"<details><summary><b>iRule events</b> — {len(names)} entries · "
        f"{ok} ✅ · {len(names) - ok} need work</summary>\n"
    )
    print("Names **and** all 9 `EventProps` fields compared Python↔Rust.\n")
    print("| event | status |")
    print("|---|---|")
    for n in names:
        print(f"| `{n}` | {statuses[n]} |")
    print("\n</details>\n")


def main() -> int:
    print("## Entry-level status (every registry entry, ticked)\n")
    print(
        "Generated by `scripts/registry-audit/gen_entries.py`. Status codes mark "
        "data the Python entry carries that the Rust entry drops: "
        "`forms` `opt`(options) `sub`(subcommands) `sfx`(side-effects) "
        "`evtreq`/`evtprof`(event-requires/profiles) `ex`(examples) `ret`(return-value) "
        "`srcurl`(doc URL) `pkg`(required-package) `exev`(excluded-events) "
        "`hover` · `sum≠`/`syn≠`/`arity≠` mark value mismatches. "
        "`✅` = Rust carries everything Python does (for the tracked dims).\n"
    )
    print("### Command registries\n")
    for g in ORDER:
        emit_group(g)
    print("### Meta / data registries\n")
    emit_bigip()
    import core.commands.registry.namespace_data as nd  # type: ignore

    # Build the Rust dumper and diff the meta registries against its output —
    # so this section is a real drift detector for events/profiles/namespaces,
    # not a Python-only self-report.
    binpath = ensure_rust_bin()
    emit_events(dict(nd.EVENT_PROPS), rust_event_props(binpath))
    emit_meta_names(
        "F5 profiles", set(nd.PROFILE_SPECS), rust_meta_names(binpath, "profiles"),
        "Names compared Python↔Rust (prop-level diff is a follow-up).",
    )
    emit_meta_names(
        "Protocol namespaces", set(nd.PROTOCOL_NAMESPACE_SPECS),
        rust_meta_names(binpath, "namespaces"),
        "Names compared Python↔Rust (prop-level diff is a follow-up).",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
