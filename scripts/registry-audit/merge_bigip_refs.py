#!/usr/bin/env python3
"""Restore reference-graph data lost in the BigIP reconcile.

The reconcile adopted `origin/main` for shared object specs, which on 64
specs dropped `references`-bearing properties the rust-branch `origin/rust`
core carried (e.g. `ltm_virtual`'s top-level `persist` → `ltm_persistence_*`).
This merges them back: for every shared spec, any property from old core
that declares `references=` and is not already covered in the current
(main-derived) spec is appended to its `properties=(...)` tuple. Additive
only — main's richer schema is preserved; duplicate property names are
fine (the registry aggregates references by key, as it already does for
e.g. bgp's repeated `distance`).

Run from repo root.
"""

from __future__ import annotations

import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
SPECS = ROOT / "core/bigip/registry/specs"
OLD_REF = "origin/rust"
OLD_BASE = "core/bigip/registry/specs"


def git_show(ref: str, path: str) -> str | None:
    r = subprocess.run(["git", "show", f"{ref}:{path}"], capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else None


def prop_blocks(text: str):
    """Yield (name, refs_set, block_text) for each BigipPropertySpec(...)."""
    for m in re.finditer(r"BigipPropertySpec\(", text):
        i, depth, in_str, q, esc = m.end(), 1, False, "", False
        while i < len(text) and depth:
            c = text[i]
            if in_str:
                if esc:
                    esc = False
                elif c == "\\":
                    esc = True
                elif c == q:
                    in_str = False
            elif c in "\"'":
                in_str, q = True, c
            elif c == "(":
                depth += 1
            elif c == ")":
                depth -= 1
            i += 1
        block = text[m.start() : i]
        nm = re.search(r'name\s*=\s*"((?:[^"\\]|\\.)*)"', block)
        if not nm:
            continue
        refs = (
            set(
                re.findall(r'"([^"]+)"', re.search(r"references\s*=\s*\(([^)]*)\)", block).group(1))
            )
            if "references=" in block.replace(" ", "")
            else set()
        )
        yield nm.group(1), refs, block


def has_refs_covered(cur_props, name, refs):
    for n, r, _ in cur_props:
        if n == name and refs <= r:
            return True
    return False


def main() -> None:
    merged = 0
    restored = 0
    for path in sorted(SPECS.glob("*.py")):
        if path.name in ("__init__.py", "_base.py"):
            continue
        old = git_show(OLD_REF, f"{OLD_BASE}/{path.name}")
        if old is None:
            continue  # not in old core (main-only spec) — nothing to merge
        old_props = list(prop_blocks(old))
        old_ref_props = [(n, r, b) for n, r, b in old_props if r]
        if not old_ref_props:
            continue
        cur_text = path.read_text()
        cur_props = list(prop_blocks(cur_text))
        to_add = [(n, b) for n, r, b in old_ref_props if not has_refs_covered(cur_props, n, r)]
        if not to_add:
            continue
        # Insert the missing reference-properties right after `properties=(`.
        m = re.search(r"properties\s*=\s*\(", cur_text)
        if not m:
            continue
        indent = "            "
        blocks = "\n".join(f"{indent}{b}," for _, b in to_add)
        cur_text = cur_text[: m.end()] + "\n" + blocks + cur_text[m.end() :]
        # Ensure BigipPropertySpec is imported (main-derived empty specs
        # only imported the kinds they used).
        if not re.search(r"\bBigipPropertySpec\b", cur_text.split("register_spec")[0]):
            cur_text = re.sub(
                r"from \.\.models import \(",
                "from ..models import (\n    BigipPropertySpec,",
                cur_text,
                count=1,
            )
        path.write_text(cur_text)
        merged += 1
        restored += len(to_add)
    print(f"merged reference properties into {merged} specs (+{restored} properties restored)")


if __name__ == "__main__":
    main()
