"""Differential parity gate for the Rust compiler-explorer serialiser.

The Rust ``tcl-explorer`` crate is a faithful port of
``tooling/cli/serialise.py``. This module pins that fidelity: for each
source snippet it runs the Rust serialiser (via the ``explore_json``
example) and the Python ``serialise_result``, then asserts the two agree
on **every contract key the Rust side currently emits**.

The comparison is intentionally key-scoped to the Rust output, so the gate
grows automatically: as each view family lands in the Rust serialiser, its
key starts appearing in ``rust_json`` and is compared from that commit on.
Keys not yet ported are simply absent on the Rust side and skipped, never
masking a regression on the keys that *are* ported.

The harness shells out to a built example binary, mirroring
``test_bigip_rust_parity.py``; it soft-skips when the Rust toolchain is
unavailable.
"""

from __future__ import annotations

import json
import shutil
import subprocess
from pathlib import Path

import pytest

from tooling.cli.serialise import serialise_result
from tooling.explorer.pipeline import run_pipeline

_ROOT = Path(__file__).parent.parent

# Curated source snippets. Kept small and deterministic; extend alongside
# each view family so the new key is exercised over representative shapes.
_CORPUS: dict[str, str] = {
    "empty": "",
    "two_sets": "set x 1\nset y 2",
    "proc_call": "proc greet {name} { return $name }\ngreet world",
    "if_else": "if {$x > 0} { puts hi } else { puts lo }",
    "for_loop": "for {set i 0} {$i < 10} {incr i} { puts $i }",
    "while_loop": "set i 0\nwhile {$i < 3} { incr i }",
    "foreach_loop": "foreach a {1 2 3} { puts $a }",
    "switch_stmt": "switch -- $x { a { puts A } b { puts B } default { puts D } }",
    "catch_stmt": "catch { error boom } msg",
    "expr_assign": "set y [expr {$x * 2 + 1}]",
}

_DIALECT = "tcl8.6"


def _rust_binary() -> Path | None:
    """Build and locate the ``explore_json`` example, or ``None`` to skip."""
    if shutil.which("cargo") is None:
        return None
    build = subprocess.run(
        ["cargo", "build", "-p", "tcl-explorer", "--example", "explore_json", "-q"],
        cwd=_ROOT,
        capture_output=True,
        text=True,
    )
    if build.returncode != 0:
        return None
    binary = _ROOT / "target" / "debug" / "examples" / "explore_json"
    return binary if binary.exists() else None


def _rust_json(binary: Path, source: str, dialect: str) -> dict:
    out = subprocess.run(
        [str(binary), "--dialect", dialect, "--source", source],
        capture_output=True,
        text=True,
    )
    assert out.returncode == 0, f"explore_json failed: {out.stderr}"
    return json.loads(out.stdout)


def _python_json(source: str, dialect: str) -> dict:
    # Round-trip through json to normalise tuples → lists, etc., so the
    # comparison is value-structural (not Python-type sensitive).
    return json.loads(json.dumps(serialise_result(run_pipeline(source, dialect=dialect))))


@pytest.mark.parametrize("name", sorted(_CORPUS), ids=lambda n: n)
def test_rust_serialiser_matches_python(name: str) -> None:
    binary = _rust_binary()
    if binary is None:
        pytest.skip("Rust toolchain / explore_json example unavailable")

    source = _CORPUS[name]
    rust = _rust_json(binary, source, _DIALECT)
    py = _python_json(source, _DIALECT)

    assert rust, "Rust serialiser emitted no keys"
    for key in rust:
        assert key in py, f"Rust emitted unknown contract key {key!r}"
        assert rust[key] == py[key], f"mismatch on {key!r} for snippet {name!r}"
