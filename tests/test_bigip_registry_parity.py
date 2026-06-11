"""Registry-parity gate for the Rust BIG-IP object registry.

The Rust `tcl_registry::bigip` registry must carry **exactly** the same
property set the Python registry declares for each `(module,
object_type)` header — no missing properties (the `client-ssl` /
`server-ssl` specs were once shadowed by empty stubs, dropping 88 / 69
props) and no extra ones (34 generation artifacts — module-name tokens
like `net`/`security`, a `timestamp` mis-linked to `net_route_domain`,
and `ltm policy` *rule* operands conflated onto the policy object — were
removed). Equality both ways keeps the Rust registry honest against the
Python source of truth.
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


def test_rust_registry_property_sets_match_python() -> None:
    """The Rust registry's property set equals Python's for every header."""
    rust = _rust_registry()
    if rust is None:
        pytest.skip("native Rust registry unavailable (no cargo / build failed)")

    missing: dict[tuple[str, str], list[str]] = {}
    extra: dict[tuple[str, str], list[str]] = {}
    checked = 0
    for (module, otype), rust_props in rust.items():
        python_props = set(property_names_for(module, otype))
        if not python_props:
            continue
        checked += 1
        if python_props - rust_props:
            missing[(module, otype)] = sorted(python_props - rust_props)
        if rust_props - python_props:
            extra[(module, otype)] = sorted(rust_props - python_props)

    assert checked > 700, f"expected to compare ~741 headers, got {checked}"
    assert not missing, f"Rust registry is missing Python properties: {missing}"
    assert not extra, f"Rust registry has properties absent from Python: {extra}"
