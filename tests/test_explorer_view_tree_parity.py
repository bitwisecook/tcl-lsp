"""Differential parity for the explorer TUI tree builders.

`tcl-explorer::view_tree::build_view` is a port of
`tooling/explorer/tui_views.py::build_view`. Both consume the *same*
serialised result, so this feeds the Python `serialise_result` JSON to both
the Python builder and the Rust builder (via `explore_json --view-tree-stdin`)
and asserts the resulting `ViewNode` trees are identical — isolating the
builder port from any underlying serialise divergence.
"""

from __future__ import annotations

import json
import shutil
import subprocess
from pathlib import Path

import pytest

from tooling.cli.serialise import serialise_result
from tooling.explorer.pipeline import run_pipeline
from tooling.explorer.tui_views import TREE_VIEWS, build_view

_ROOT = Path(__file__).parent.parent

_CORPUS = {
    "two_sets": "set x 1\nset y 2",
    "if_else": "if {$x > 0} { puts hi } else { puts lo }",
    "for_loop": "for {set i 0} {$i < 10} {incr i} { puts $i }",
    "proc": "proc f {x} { return [expr {$x + 1}] }\nf 2",
    "switch": "switch -- $x { a { puts A } default { puts D } }",
}


def _node_json(n) -> dict:
    return {
        "label": n.label,
        "detail": [[k, str(v)] for k, v in n.detail],
        "children": [_node_json(c) for c in n.children],
        "style": n.style,
    }


def _rust_binary() -> Path | None:
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


@pytest.mark.parametrize("name", sorted(_CORPUS), ids=lambda n: n)
def test_rust_build_view_matches_python(name: str) -> None:
    binary = _rust_binary()
    if binary is None:
        pytest.skip("Rust toolchain / explore_json example unavailable")

    data = json.loads(json.dumps(serialise_result(run_pipeline(_CORPUS[name], dialect="tcl8.6"))))
    payload = json.dumps(data)
    for view in sorted(TREE_VIEWS):
        # The Rust explorer intentionally drops the `greentree` view (Rust has
        # a single red-green CST, no separate legacy green tree), so there is
        # no Rust builder to compare against.
        if view == "greentree":
            continue
        py = [_node_json(n) for n in build_view(view, data)]
        out = subprocess.run(
            [str(binary), "--view-tree-stdin", view],
            input=payload,
            capture_output=True,
            text=True,
        )
        assert out.returncode == 0, f"{view}: {out.stderr}"
        rust = json.loads(out.stdout)
        assert rust == py, f"build_view({view!r}) differs for snippet {name!r}"
