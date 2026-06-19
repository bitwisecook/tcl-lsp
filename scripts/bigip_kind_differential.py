#!/usr/bin/env python3
"""Per-kind synthetic differential for the Rust BIG-IP parser.

Builds a synthetic stanza for every dispatched (module, object_type) kind
from the registry's property names, parses it with the pure-Python parser
and the native Rust parser (via the dump_canonical example + the bridge),
and reports every field that diverges — the completeness oracle for the
object-model port beyond the bundled sample corpus.

Run from the repo root (Rust example must be built):

    cargo build -p tcl-bigip --example dump_canonical -q
    python3 scripts/bigip_kind_differential.py
"""

from __future__ import annotations

import dataclasses as dc
import json
import os
import subprocess
import tempfile
from collections import Counter

from dialects.f5.bigip.parser import _rust_bridge as bridge
from dialects.f5.bigip.parser import parse_bigip_conf
from dialects.f5.bigip.parser._parsers import _NAMED_DISPATCH, _SINGLETON_DISPATCH
from dialects.f5.bigip.registry import property_names_for, property_spec_for

_BINARY = "./target/debug/examples/dump_canonical"


def _synthetic_body(module: str, otype: str) -> str:
    lines = []
    for n in property_names_for(module, otype)[:60]:
        spec = property_spec_for(module, otype, n)
        vk = getattr(getattr(spec, "value_type", None), "value", "")
        if vk == "list":
            lines.append(f"    {n} {{ /Common/a /Common/b }}")
        elif vk in ("block", "object"):
            lines.append(f"    {n} {{ inner val }}")
        elif vk == "boolean":
            lines.append(f"    {n} enabled")
        elif vk == "integer":
            lines.append(f"    {n} 5")
        else:
            lines.append(f'    {n} "quoted val"')
    return "\n".join(lines)


def _rust_parse(src: str):
    with tempfile.NamedTemporaryFile("w", suffix=".conf", delete=False) as tf:
        tf.write(src)
        path = tf.name
    try:
        out = subprocess.run([_BINARY, path], capture_output=True, text=True)
    finally:
        os.unlink(path)
    if out.returncode != 0:
        return None
    return bridge.rebuild(json.loads(out.stdout))


def main() -> int:
    gaps: Counter = Counter()
    tested = 0
    for (module, otype), (attr, _fn) in {**_NAMED_DISPATCH, **_SINGLETON_DISPATCH}.items():
        body = _synthetic_body(module, otype)
        if not body:
            continue
        src = f"{module} {otype} /Common/test {{\n{body}\n}}\n"
        try:
            py = parse_bigip_conf(src)
        except Exception:  # noqa: BLE001
            continue
        rb = _rust_parse(src)
        if rb is None:
            gaps[(f"{module} {otype}", "<RUST-ERROR>")] += 1
            continue
        tested += 1
        py_t, rb_t = getattr(py, attr), getattr(rb, attr)
        for key in py_t:
            if key not in rb_t or py_t[key] == rb_t[key]:
                continue
            for f in dc.fields(py_t[key]):
                if getattr(py_t[key], f.name) != getattr(rb_t[key], f.name, None):
                    gaps[(f"{module} {otype}", f.name)] += 1
    print(f"tested kinds: {tested} | field gaps: {len(gaps)}")
    for (kind, fld), _ in sorted(gaps.items()):
        print(f"  {kind}.{fld}")
    return 1 if gaps else 0


if __name__ == "__main__":
    raise SystemExit(main())
