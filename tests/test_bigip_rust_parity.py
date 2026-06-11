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
from pathlib import Path

import pytest

from dialects.f5.bigip.parser import parse_bigip_conf
from tests import _bigip_parity as bridge

_CORPUS = sorted((Path(__file__).parent.parent / "samples" / "bigip").glob("*.conf"))


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
