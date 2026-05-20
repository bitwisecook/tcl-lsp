"""Assertion-strength ratchet for the test suite.

Audits every ``test_*`` function across ``tests/`` for the
"feature exists / not actually tested" smell: a test whose *only*
assertions are pure-existence checks — ``x is not None``, ``x is None``,
or ``len(x) > 0`` / ``>= 1`` — and which therefore never inspects the
*value* the feature produced.

Such a test passes as long as the call returns *something*, so it would
keep passing even if the feature regressed to a no-op or returned wrong
data.  This is distinct from a minimal-but-correct property check
(``assert r.tainted`` for a taint-join, ``assert result is None`` for a
negative case): those inspect a specific claimed property and are *not*
flagged — only tests that assert solely existence are.

The ``BASELINE`` below is a per-file ceiling captured from the tree at
the time this guard was added.  The guard asserts the count never
*grows* and that no *new* file introduces the smell, so the suite can
only get stronger.  When you strengthen a flagged test (assert the
actual value, range, or message it should produce), drop that file's
ceiling here — like the diagnostic-coverage ``NOT_YET_COVERED`` list,
this only shrinks.
"""

from __future__ import annotations

import ast
from pathlib import Path

import pytest

_TESTS_DIR = Path(__file__).parent


def _assert_kind(node: ast.Assert) -> str:
    """Classify a single ``assert`` as 'existence' or 'strong'.

    'existence' — ``x is [not] None``, ``x == None``, or
    ``len(x) > 0`` / ``len(x) >= 1``: checks only that *something* was
    produced.  Everything else (equality to a value, ``in`` membership,
    ordering, a truthy property/flag, a boolean-returning helper call) is
    'strong' — it inspects what the feature actually produced.
    """
    test = node.test
    if isinstance(test, ast.Compare) and len(test.ops) == 1:
        op = test.ops[0]
        comp = test.comparators[0]
        none_cmp = isinstance(comp, ast.Constant) and comp.value is None
        if isinstance(op, (ast.Is, ast.IsNot, ast.Eq, ast.NotEq)) and none_cmp:
            return "existence"
        if (
            isinstance(op, (ast.Gt, ast.GtE))
            and isinstance(comp, ast.Constant)
            and comp.value in (0, 1)
            and isinstance(test.left, ast.Call)
            and isinstance(test.left.func, ast.Name)
            and test.left.func.id == "len"
        ):
            return "existence"
    return "strong"


def _existence_only_tests(path: Path) -> list[str]:
    """Return names of ``test_*`` functions in *path* whose only asserts
    are pure-existence checks."""
    try:
        tree = ast.parse(path.read_text())
    except (SyntaxError, UnicodeDecodeError):
        return []
    flagged: list[str] = []
    for node in ast.walk(tree):
        if not isinstance(node, (ast.FunctionDef, ast.AsyncFunctionDef)):
            continue
        if not node.name.startswith("test"):
            continue
        asserts = [n for n in ast.walk(node) if isinstance(n, ast.Assert)]
        if not asserts:
            continue
        if all(_assert_kind(a) == "existence" for a in asserts):
            flagged.append(node.name)
    return flagged


# Per-file ceiling: number of existence-only test functions tolerated.
# This list only shrinks — strengthen a test, then lower its file's count.
BASELINE: dict[str, int] = {
    "test_analyser.py": 9,
    "test_apl_model.py": 1,
    "test_auto_index.py": 1,
    "test_bigip_parser.py": 4,
    "test_bigip_port_names.py": 4,
    "test_bigip_registry_value_specs.py": 2,
    "test_bigip_typed_values.py": 3,
    "test_bytecode_identity.py": 1,
    "test_checks.py": 4,
    "test_class_hierarchy.py": 2,
    "test_command_registry.py": 10,
    "test_command_segmenter.py": 10,
    "test_compiler_explorer_web.py": 3,
    "test_complex_codegen.py": 2,
    "test_conf_wrapped_irules.py": 2,
    "test_dataflow_graph.py": 2,
    "test_debugger_backends.py": 1,
    "test_def_use.py": 3,
    "test_detect_dialect.py": 4,
    "test_diagnostic_phases.py": 3,
    "test_diagram_extract.py": 1,
    "test_docstring.py": 1,
    "test_document_links.py": 1,
    "test_event_flow_chains.py": 6,
    "test_event_registry.py": 7,
    "test_event_tree.py": 5,
    "test_expansion_dialect.py": 1,
    "test_f5_query_extensions.py": 1,
    "test_f5_query_python_api.py": 1,
    "test_f5_query_renderers.py": 1,
    "test_f5_query_tls_server.py": 1,
    "test_f5_trailer.py": 2,
    "test_folding.py": 2,
    "test_formatter.py": 1,
    "test_hover.py": 1,
    "test_incremental_update.py": 5,
    "test_inlay_hints.py": 2,
    "test_interprocedural.py": 1,
    "test_ip_utils.py": 6,
    "test_irule_test_framework.py": 2,
    "test_irules_checks.py": 2,
    "test_kcs_db.py": 2,
    "test_lexer_cursor_state.py": 1,
    "test_licm.py": 14,
    "test_linked_editing_range.py": 2,
    "test_memory_ssa.py": 2,
    "test_minifier.py": 6,
    "test_namespace_imports.py": 2,
    "test_optimiser.py": 7,
    "test_optimiser_coverage.py": 12,
    "test_package_loading.py": 12,
    "test_parser_edge_cases.py": 4,
    "test_proc_lookup.py": 1,
    "test_progress.py": 1,
    "test_recovery.py": 1,
    "test_refactoring.py": 17,
    "test_refactoring_applied.py": 1,
    "test_references.py": 2,
    "test_rename.py": 6,
    "test_rendered_properties.py": 1,
    "test_scanner.py": 2,
    "test_semantic_graph.py": 2,
    "test_semantic_tokens.py": 9,
    "test_semantic_tokens_delta.py": 3,
    "test_shimmer.py": 2,
    "test_signature_help.py": 3,
    "test_source_resolver.py": 2,
    "test_specialise_factories.py": 4,
    "test_static_loops.py": 1,
    "test_stdlib_registry.py": 11,
    "test_stub_comments.py": 4,
    "test_subst_nocommands.py": 9,
    "test_suppression_layers.py": 2,
    "test_taint.py": 70,
    "test_tcl9_diagnostics.py": 8,
    "test_tcl_corner_cases.py": 1,
    "test_tcl_expr_eval.py": 18,
    "test_tcl_parse.py": 22,
    "test_tcl_parse_expr.py": 5,
    "test_tcl_parse_old.py": 12,
    "test_tcllib.py": 4,
    "test_tk_registry.py": 1,
    "test_tricky_edge_cases.py": 11,
    "test_type_propagation.py": 4,
    "test_upstream_control_flow.py": 12,
    "test_upstream_namespace.py": 9,
    "test_upstream_parse.py": 15,
    "test_upstream_proc.py": 2,
    "test_upstream_var.py": 10,
    "test_user_config.py": 2,
    "test_variable_forms.py": 1,
    "test_vscode_tcl_issues.py": 8,
    "test_wasm_execution.py": 2,
    "test_workspace_file_ops.py": 2,
}


def _current_counts() -> dict[str, int]:
    counts: dict[str, int] = {}
    for path in sorted(_TESTS_DIR.glob("test_*.py")):
        n = len(_existence_only_tests(path))
        if n:
            counts[path.name] = n
    return counts


@pytest.mark.parametrize("filename", sorted(BASELINE))
def test_existence_only_count_does_not_grow(filename):
    """No file may gain existence-only tests beyond its frozen ceiling."""
    actual = len(_existence_only_tests(_TESTS_DIR / filename))
    ceiling = BASELINE[filename]
    assert actual <= ceiling, (
        f"{filename} has {actual} existence-only test(s) (ceiling {ceiling}). "
        f"New tests must assert the value/range/message the feature produces, "
        f"not merely that it returned something. Offenders: "
        f"{sorted(_existence_only_tests(_TESTS_DIR / filename))}"
    )


def test_no_new_files_introduce_existence_only_smell():
    """A test file not in the baseline must not introduce the smell."""
    current = _current_counts()
    new_files = sorted(set(current) - set(BASELINE))
    assert not new_files, (
        "These test files newly contain existence-only tests; assert real "
        f"behaviour or add them to the shrinking baseline: {new_files}"
    )


def test_baseline_has_no_stale_entries():
    """Baseline entries whose smell is fully fixed should be removed so the
    ratchet keeps tightening."""
    stale = []
    for filename, ceiling in BASELINE.items():
        path = _TESTS_DIR / filename
        if not path.exists():
            stale.append(filename)
            continue
        if len(_existence_only_tests(path)) == 0 and ceiling > 0:
            stale.append(filename)
    assert not stale, f"baseline entries no longer needed (remove them): {stale}"
