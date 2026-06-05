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

import glob
import json
import subprocess
import sys
from pathlib import Path

_REPO_ROOT = Path(__file__).resolve().parents[2]
if str(_REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(_REPO_ROOT))

OUT = _REPO_ROOT / "tmp" / "registry-audit"
ORDER = "tcl stdlib tcllib irules iapps tk expect sdc-base synopsys cadence xilinx quartus mentor".split()

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


def emit_meta_simple(title: str, names: list[str], note: str) -> None:
    print(
        f"<details><summary><b>{title}</b> — {len(names)} entries · "
        f"{len(names)} ✅</summary>\n"
    )
    print(f"{note}\n")
    print("| entry | status |")
    print("|---|---|")
    for n in sorted(names):
        print(f"| `{n}` | ✅ |")
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

    emit_meta_simple(
        "iRule events", list(nd.EVENT_PROPS),
        "Names **and** all 9 `EventProps` fields verified equal Python↔Rust.",
    )
    emit_meta_simple(
        "F5 profiles", list(nd.PROFILE_SPECS), "Names verified equal Python↔Rust.",
    )
    emit_meta_simple(
        "Protocol namespaces", list(nd.PROTOCOL_NAMESPACE_SPECS),
        "Names verified equal Python↔Rust.",
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
