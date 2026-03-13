"""Pipeline-level tests for shared CompilationUnit usage in explorer."""

from __future__ import annotations

from unittest.mock import patch

from explorer.pipeline import run_pipeline


def test_run_pipeline_compiles_once():
    source = "set x [expr {1 + 2}]\n"

    from core.compiler import compilation_unit as cu_module

    real_compile_source = cu_module.compile_source
    with patch(
        "core.compiler.compilation_unit.compile_source", wraps=real_compile_source
    ) as mocked_compile:
        result = run_pipeline(source)

        assert mocked_compile.call_count == 1
        assert result.source == source
