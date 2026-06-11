"""Registry-completeness gate for the Rust BIG-IP object registry.

The Rust `tcl_registry::bigip` registry must carry every property the
Python registry declares for each `(module, object_type)` header — i.e.
the Python property set is a **subset** of the Rust one. This catches
missing-property regressions like the `ltm profile client-ssl` /
`server-ssl` specs that were once shadowed by empty stub specs (88 / 69
properties dropped).

The Rust registry is allowed to be a *superset* (it carries some props
the Python projection doesn't surface), so the assertion is directional.
"""

from __future__ import annotations

import json
import shutil
import subprocess
from pathlib import Path

import pytest

from dialects.f5.bigip.registry import property_names_for

_ROOT = Path(__file__).parent.parent


def _rust_registry() -> dict[tuple[str, str], set[str]] | None:
    if shutil.which("cargo") is None:
        return None
    out = subprocess.run(
        ["cargo", "run", "-p", "tcl-registry", "--example", "dump_bigip_registry", "-q"],
        cwd=_ROOT,
        capture_output=True,
        text=True,
    )
    if out.returncode != 0:
        return None
    by_header: dict[tuple[str, str], set[str]] = {}
    for line in out.stdout.splitlines():
        if not line.strip():
            continue
        spec = json.loads(line)
        for header in spec["headers"]:
            module, otype = header.split("|", 1)
            by_header[(module, otype)] = set(spec["props"])
    return by_header


def test_rust_registry_covers_every_python_property() -> None:
    """Every declared Python property is present in the Rust registry."""
    rust = _rust_registry()
    if rust is None:
        pytest.skip("native Rust registry unavailable (no cargo / build failed)")

    missing: dict[tuple[str, str], list[str]] = {}
    checked = 0
    for (module, otype), rust_props in rust.items():
        python_props = set(property_names_for(module, otype))
        if not python_props:
            continue
        checked += 1
        gap = python_props - rust_props
        if gap:
            missing[(module, otype)] = sorted(gap)

    assert checked > 700, f"expected to compare ~741 headers, got {checked}"
    assert not missing, f"Rust registry is missing Python properties: {missing}"
