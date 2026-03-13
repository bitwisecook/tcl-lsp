"""Parity tests for source-only vs prebuilt CompilationUnit pass invocation."""

from __future__ import annotations

import sys
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

from core.analysis.analyser import analyse
from core.commands.registry.runtime import configure_signatures
from core.compiler.compilation_unit import compile_source
from core.compiler.gvn import find_redundant_computations
from core.compiler.irules_flow import find_irules_flow_warnings
from core.compiler.optimiser import find_optimisations
from core.compiler.shimmer import find_shimmer_warnings
from core.compiler.taint import find_taint_warnings
from lsp.features.diagnostics import get_basic_diagnostics, get_deep_diagnostics


def _diag_key(d):
    return (
        str(d.code) if d.code is not None else "",
        d.message,
        d.range.start.line,
        d.range.start.character,
        d.range.end.line,
        d.range.end.character,
    )


def test_analyse_parity_with_prebuilt_compilation_unit():
    source = "proc foo {x} {\n  set y [expr {$x + 1}]\n  return $y\n}\nfoo 1\n"
    cu = compile_source(source)

    result_from_source = analyse(source)
    result_from_cu = analyse(source, cu=cu)

    source_diags = sorted(
        (d.code, d.message, d.range.start.line) for d in result_from_source.diagnostics
    )
    cu_diags = sorted((d.code, d.message, d.range.start.line) for d in result_from_cu.diagnostics)

    assert source_diags == cu_diags
    assert any(code == "W211" for code, _msg, _line in source_diags)


def test_basic_diagnostics_parity_with_prebuilt_compilation_unit():
    source = "expr $x + 1\nset y 1\n"
    cu = compile_source(source)

    diags_from_source, _result_source, suppressed_source = get_basic_diagnostics(source)
    diags_from_cu, _result_cu, suppressed_cu = get_basic_diagnostics(source, cu=cu)

    source_keys = sorted(_diag_key(d) for d in diags_from_source)
    cu_keys = sorted(_diag_key(d) for d in diags_from_cu)

    assert source_keys == cu_keys
    assert any(code == "W100" for code, *_rest in source_keys)
    assert suppressed_source == suppressed_cu


def test_deep_diagnostics_parity_with_prebuilt_compilation_unit():
    source = 'set x [string tolower "HELLO"]\nset z [expr {1 + 2}]\n'
    cu = compile_source(source)

    diags_from_source = get_deep_diagnostics(source, {})
    diags_from_cu = get_deep_diagnostics(source, {}, cu=cu)

    source_keys = sorted(_diag_key(d) for d in diags_from_source)
    cu_keys = sorted(_diag_key(d) for d in diags_from_cu)

    assert source_keys == cu_keys
    assert any(code.startswith("O") for code, *_rest in source_keys)


def test_optimiser_parity_with_prebuilt_compilation_unit():
    source = "set x [expr {1 + 2}]\n"
    cu = compile_source(source)

    from_source = find_optimisations(source)
    from_cu = find_optimisations(source, cu=cu)

    source_opts = [(o.code, o.replacement, o.range.start.offset) for o in from_source]
    cu_opts = [(o.code, o.replacement, o.range.start.offset) for o in from_cu]

    assert source_opts == cu_opts
    assert len(source_opts) > 0


def test_shimmer_parity_with_prebuilt_compilation_unit():
    source = 'set x "hello world"\nset n [llength $x]\n'
    cu = compile_source(source)

    from_source = find_shimmer_warnings(source)
    from_cu = find_shimmer_warnings(source, cu=cu)

    source_warnings = [(w.code, w.message, w.range.start.line) for w in from_source]
    cu_warnings = [(w.code, w.message, w.range.start.line) for w in from_cu]

    assert source_warnings == cu_warnings
    assert len(source_warnings) > 0


def test_gvn_parity_with_prebuilt_compilation_unit():
    source = "set x [expr {$a + 1}]\nset y [expr {$a + 1}]\n"
    cu = compile_source(source)

    from_source = find_redundant_computations(source)
    from_cu = find_redundant_computations(source, cu=cu)

    source_items = [(w.code, w.message, w.range.start.line) for w in from_source]
    cu_items = [(w.code, w.message, w.range.start.line) for w in from_cu]

    assert source_items == cu_items
    assert len(source_items) > 0


def test_irules_flow_parity_with_prebuilt_compilation_unit():
    configure_signatures(dialect="f5-irules")
    try:
        source = (
            "when HTTP_REQUEST {\n  HTTP::respond 200 content ok\n  HTTP::header value Host\n}\n"
        )
        cu = compile_source(source)

        from_source = find_irules_flow_warnings(source)
        from_cu = find_irules_flow_warnings(source, cu=cu)

        source_items = [(w.code, w.message, w.range.start.line) for w in from_source]
        cu_items = [(w.code, w.message, w.range.start.line) for w in from_cu]

        assert source_items == cu_items
        assert any(code == "IRULE1201" for code, _msg, _line in source_items)
    finally:
        # Restore default profile for subsequent tests in this module.
        configure_signatures(dialect="tcl")


def test_taint_parity_with_prebuilt_compilation_unit():
    source = "set x [IP::client_addr]\nlog local0. $x\n"
    cu = compile_source(source)

    from_source = find_taint_warnings(source)
    from_cu = find_taint_warnings(source, cu=cu)

    assert [(w.code, w.message, w.range.start.line) for w in from_source] == [
        (w.code, w.message, w.range.start.line) for w in from_cu
    ]


def test_get_diagnostics_compiles_once_without_prebuilt_compilation_unit():
    source = "set x [expr {1 + 2}]\n"

    from core.compiler import compilation_unit as cu_module

    real_compile_source = cu_module.compile_source
    with patch(
        "core.compiler.compilation_unit.compile_source", wraps=real_compile_source
    ) as mocked_compile:
        # Import locally to avoid widening module-level dependencies in this test.
        from lsp.features.diagnostics import get_diagnostics

        get_diagnostics(source)

        assert mocked_compile.call_count == 1


def test_compilation_unit_exposes_execution_intent_for_command_substitutions():
    source = 'set x "hello"\nset n [llength $x]\n'
    cu = compile_source(source)

    intents = cu.top_level.execution_intent.command_substitutions
    assert intents

    # Locate the command-substitution assignment intent emitted for `set n [llength $x]`.
    intent = next((i for i in intents.values() if i.command == "llength"), None)
    assert intent is not None
    assert intent.command == "llength"
    assert intent.args == ("$x",)


def test_execution_intent_skips_multi_command_substitution():
    source = "set x [llength $a; llength $b]\n"
    cu = compile_source(source)

    assert cu.top_level.execution_intent.command_substitutions == {}


def test_execution_intent_classifies_side_effect_escape_and_pressure():
    source = "set n [llength $x]\nset y [eval $script]\n"
    cu = compile_source(source)

    intents = list(cu.top_level.execution_intent.command_substitutions.values())
    llength_intent = next(i for i in intents if i.command == "llength")
    eval_intent = next(i for i in intents if i.command == "eval")

    assert llength_intent.side_effect.value == "pure"
    assert llength_intent.escape.value == "no_escape"
    assert llength_intent.shimmer_pressure > 0

    assert eval_intent.side_effect.value == "may_side_effect"
    assert eval_intent.escape.value == "may_escape"


def test_execution_intent_uses_args_for_side_effect_classification():
    configure_signatures(dialect="f5-irules")
    try:
        source = "set a [HTTP::uri -normalized]\nset b [HTTP::uri /new]\n"
        cu = compile_source(source)

        intents = list(cu.top_level.execution_intent.command_substitutions.values())
        getter = next(i for i in intents if i.command == "HTTP::uri" and i.args == ("-normalized",))
        setter = next(i for i in intents if i.command == "HTTP::uri" and i.args == ("/new",))

        assert getter.side_effect.value == "pure"
        assert setter.side_effect.value == "may_side_effect"
    finally:
        configure_signatures(dialect="tcl")
