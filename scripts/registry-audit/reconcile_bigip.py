#!/usr/bin/env python3
"""GAP-e stage 1+2: reconcile the Python BigIP object registry to the
canonical `origin/main` baseline.

The rust-branch `core/bigip/registry` drifted to thin property stubs and
fell behind main (245 specs missing, 539 shared specs differ). main is
the canonical *rich* baseline (`models.py`/`_base.py` are clean supersets
of core's). This:

  1. adopts main's `models.py` (adds `ValueKind` + rich `BigipPropertySpec`
     fields — superset, additive for core's consumers) and
     `specs/_base.py` (adds the curated `normalise_registry` post-pass);
  2. copies every main spec file into core (adds the 245 main-only,
     enriches the 539 drifted shared);
  3. KEEPS the 194 core-only specs (absent from main, unreferenced thin
     stubs — kept as a non-destructive superset so the analyser still
     recognises those object kinds);
  4. regenerates `specs/__init__.py` over the union and wires the
     `normalise_registry()` call like main.

Run from repo root: python3 scripts/registry-audit/reconcile_bigip.py
"""

from __future__ import annotations

import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
MAIN = "origin/main"
MAIN_REG = "dialects/f5/bigip/registry"
CORE_REG = ROOT / "core/bigip/registry"
SPECS = CORE_REG / "specs"


def git_show(path: str) -> str:
    r = subprocess.run(["git", "show", f"{MAIN}:{path}"], capture_output=True, text=True)
    if r.returncode != 0:
        raise SystemExit(f"git show failed for {path}: {r.stderr}")
    return r.stdout


def main_spec_stems() -> list[str]:
    out = subprocess.run(
        ["git", "ls-tree", "-r", "--name-only", MAIN, "--", f"{MAIN_REG}/specs"],
        capture_output=True,
        text=True,
    ).stdout
    return [
        line.split("/")[-1][:-3]
        for line in out.splitlines()
        if line.endswith(".py") and line.split("/")[-1] not in ("__init__.py", "_base.py")
    ]


def main() -> None:
    # 1. Adopt canonical infrastructure.
    (CORE_REG / "models.py").write_text(git_show(f"{MAIN_REG}/models.py"))
    (SPECS / "_base.py").write_text(git_show(f"{MAIN_REG}/specs/_base.py"))

    # 2. Copy every main spec (adds 245, enriches 539).
    main_stems = main_spec_stems()
    for stem in main_stems:
        (SPECS / f"{stem}.py").write_text(git_show(f"{MAIN_REG}/specs/{stem}.py"))

    # 3+4. Regenerate __init__ over the union of every spec file present.
    stems = sorted(p.stem for p in SPECS.glob("*.py") if p.name not in ("__init__.py", "_base.py"))
    imports = "\n".join(f"    {s},  # noqa: F401" for s in stems)
    init = (
        '"""BIG-IP object spec modules — imported here so each '
        "``@register`` fires.\n\n"
        "Reconciled to the canonical ``origin/main`` baseline (GAP-e); the\n"
        "245 main-only specs were added and the 539 drifted shared specs\n"
        "enriched. The 194 rust-branch-only specs (absent from main) are\n"
        'kept as a non-destructive superset."""\n\n'
        "from __future__ import annotations\n\n"
        "from . import (\n"
        f"{imports}\n"
        ")\n"
        "from ._base import _REGISTRY, normalise_registry\n\n\n"
        "def bigip_object_specs() -> tuple:\n"
        "    return tuple(_REGISTRY)\n\n\n"
        "# Apply curated post-processing overrides (list_operators / value_type\n"
        "# corrections that survive a spec regeneration).\n"
        "normalise_registry()\n\n"
        "OBJECT_SPECS = bigip_object_specs()\n\n"
        '__all__ = ["OBJECT_SPECS", "bigip_object_specs"]\n'
    )
    (SPECS / "__init__.py").write_text(init)
    print(
        f"reconciled: adopted main models/_base, copied {len(main_stems)} main specs, "
        f"__init__ now imports {len(stems)} spec modules"
    )


if __name__ == "__main__":
    main()
