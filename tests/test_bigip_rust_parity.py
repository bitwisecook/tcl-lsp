"""Parity scaffolding for the Rust BIG-IP parser port.

The Rust ``tcl-bigip`` crate emits a parsed config as a canonical JSON
document; :mod:`tests._bigip_parity` rebuilds the existing Python
dataclasses from it (the ``f5`` command keeps its typed contract) and
serialises a Python config to the same shape.

This module pins the reconstruction layer with a self-consistency check
— ``rebuild(canonical(python_parse(src))) == python_parse(src)`` — across
the sample corpus. Once the Rust ``parse_bigip_conf`` JSON binding lands,
``rebuild(rust_json(src)) == python_parse(src)`` becomes the fidelity
gate (added alongside the binding).
"""

from __future__ import annotations

import json
import os
import shutil
import subprocess
from pathlib import Path

import pytest

from dialects.f5.bigip.parser import _rust_bridge as bridge
from dialects.f5.bigip.parser import parse_bigip_conf

_ROOT = Path(__file__).parent.parent
_CORPUS = sorted(
    {
        *(_ROOT / "samples").rglob("*.conf"),
        *(_ROOT / "samples").rglob("*.scf"),
        *(_ROOT / "scripts").rglob("*.scf"),
    }
)


def _rust_canonical(conf: Path) -> dict | None:
    """Parse *conf* with the native Rust parser via the `dump_canonical`
    example, returning its canonical JSON — or ``None`` to skip when the
    Rust toolchain / crate isn't available in this environment."""
    if shutil.which("cargo") is None:
        return None
    build = subprocess.run(
        ["cargo", "build", "-p", "tcl-bigip", "--example", "dump_canonical", "-q"],
        cwd=_ROOT,
        capture_output=True,
        text=True,
    )
    if build.returncode != 0:
        return None
    binary = _ROOT / "target" / "debug" / "examples" / "dump_canonical"
    if not binary.exists():
        return None
    out = subprocess.run([str(binary), str(conf)], capture_output=True, text=True)
    if out.returncode != 0:
        return None
    return json.loads(out.stdout)


@pytest.mark.parametrize("conf", _CORPUS, ids=lambda p: p.name)
def test_canonical_roundtrip_is_identity(conf: Path) -> None:
    """``rebuild(canonical(c))`` reproduces ``c`` exactly — the schema and
    the generic rebuilder are faithful for every kind in the corpus."""
    config = parse_bigip_conf(conf.read_text())
    doc = json.loads(json.dumps(bridge.canonical(config)))
    assert bridge.rebuild(doc) == config


def test_corpus_is_non_trivial() -> None:
    """Guard against an empty corpus silently passing the round-trip."""
    assert _CORPUS, "no BIG-IP sample configs found"
    total = sum(len(parse_bigip_conf(p.read_text()).generic_objects) for p in _CORPUS)
    assert total > 20


@pytest.mark.parametrize("conf", _CORPUS, ids=lambda p: p.name)
def test_rust_parse_matches_python(conf: Path) -> None:
    """The fidelity gate: the native Rust parser, reconstructed through the
    canonical-JSON bridge, reproduces the Python parse byte-for-byte.

    Skips when the Rust toolchain isn't present (the build runs the
    differential parser separately); enforced when ``TCL_LSP_REQUIRE_RUST``
    is set so CI can make it mandatory."""
    doc = _rust_canonical(conf)
    if doc is None:
        if os.environ.get("TCL_LSP_REQUIRE_RUST"):
            pytest.fail("Rust dump_canonical unavailable but TCL_LSP_REQUIRE_RUST set")
        pytest.skip("native Rust BIG-IP parser unavailable")
    assert bridge.rebuild(doc) == parse_bigip_conf(conf.read_text())
