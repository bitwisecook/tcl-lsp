"""White-box hover internals that have no JSON-RPC surface.

The hover provider's request/response behaviour is exercised end-to-end against
the packaged server in ``tests/lsp_e2e/test_hover_e2e.py`` (and the F5 iRules
cases in ``tests/lsp_e2e/test_irules_e2e.py``).  What remains here is the one
thing that cannot be expressed over the wire: that the type/taint inference
helpers return identical results whether they read a cached ``CompilationUnit``
(the live hover path) or recompute from source (the fallback).
"""

from __future__ import annotations

import sys
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))


class TestInferVarReusesCachedCU:
    """``_infer_var_type`` / ``_infer_var_taint`` must yield identical results
    whether they read the cached ``CompilationUnit`` (the live hover path) or
    recompute the analysis from source (the fallback) — otherwise reusing the
    cache would silently change hover output."""

    SNIPPETS = [
        ('set x 1\nset x "str$x"\n', "x"),  # shimmer between int and string
        ("proc f {} { set y 0 ; set y [expr {$y + 1}] ; return $y }\n", "y"),  # int in a proc
        ("set p [exec something]\n", "p"),  # tainted source
        ("set untyped $env(HOME)\n", "untyped"),
    ]

    def test_type_matches_source_path(self):
        from compiler.compilation_unit import compile_source
        from server.features.hover import _infer_var_type

        for src, var in self.SNIPPETS:
            cu = compile_source(src)
            assert _infer_var_type(src, var, cu=cu) == _infer_var_type(src, var, cu=None), (
                src,
                var,
            )

    def test_taint_matches_source_path(self):
        from compiler.compilation_unit import compile_source
        from server.features.hover import _infer_var_taint

        for src, var in self.SNIPPETS:
            cu = compile_source(src)
            assert _infer_var_taint(src, var, cu=cu) == _infer_var_taint(src, var, cu=None), (
                src,
                var,
            )
