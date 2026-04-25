"""Differential parity harness for the C40 ``signature_scan`` Rust port.

For each fixture in :data:`CORPUS`, both the Python implementation
(``_extract_signatures_python``) and the Rust path (via
``signature_scan_extract`` + the ``_materialise_rust_signatures``
helper) are exercised and asserted to produce field-by-field
equivalent :class:`AnalysisResult` records on the subset of fields
``signature_scan`` populates.

Skipped wholesale when the ``tcl_lsp_rust`` binding cannot be
imported.
"""

from __future__ import annotations

import pytest

# Capture the binding as a module object — ``from tcl_lsp_rust
# import …`` would trip ``ty``'s ``unresolved-import`` check in
# environments where the rust wheel isn't installed (CI lints
# without building). Mirrors the pattern in
# ``tests/test_rust_bindings_smoke.py``.
tcl_lsp_rust = pytest.importorskip(
    "tcl_lsp_rust",
    reason="tcl_lsp_rust extension not built into this venv",
)

from core.analysis.semantic_model import AnalysisResult  # noqa: E402
from core.analysis.signature_scan import (  # noqa: E402
    _extract_signatures_python,
    _materialise_rust_signatures,
)


def _compare_results(py: AnalysisResult, rust: AnalysisResult) -> None:
    """Assert that ``py`` and ``rust`` are field-equal on the
    subset of :class:`AnalysisResult` populated by signature_scan.
    """
    assert py.all_procs == rust.all_procs, "all_procs mismatch"
    assert py.all_classes == rust.all_classes, "all_classes mismatch"
    assert py.package_requires == rust.package_requires, "package_requires mismatch"
    assert py.source_targets == rust.source_targets, "source_targets mismatch"
    assert py.command_aliases == rust.command_aliases, "command_aliases mismatch"
    assert py.namespace_imports == rust.namespace_imports, "namespace_imports mismatch"
    assert py.auto_path_entries == rust.auto_path_entries, "auto_path_entries mismatch"
    assert py.command_invocations == rust.command_invocations, "command_invocations mismatch"


def _assert_same(source: str) -> None:
    """Run both implementations on ``source`` and assert parity."""
    py = _extract_signatures_python(source)
    rust = _materialise_rust_signatures(source, tcl_lsp_rust.signature_scan_extract(source))
    _compare_results(py, rust)


def test_simple_proc_parity() -> None:
    """Sanity check: a single bare proc round-trips identically."""
    _assert_same("proc foo {} {}")


# Inline corpus — one tuple per fixture. Each tuple is
# ``(label, source)``; the label drives the pytest test ID so
# failures point at the offending shape directly.
CORPUS: list[tuple[str, str]] = [
    # -- procs / namespaces --
    ("bare_proc", "proc foo {} {}"),
    ("namespace_qualified_proc", "proc ::foo::bar {} {}"),
    ("namespace_eval_with_inner", "namespace eval ns { proc inner {} {} }"),
    (
        "nested_namespace_eval",
        "namespace eval outer { namespace eval inner { proc deep {} {} } }",
    ),
    (
        "absolute_proc_under_namespace",
        "namespace eval baz { proc ::foo::bar {} {} }",
    ),
    # -- classes --
    ("oo_class_no_body", "oo::class create MyCls"),
    ("oo_class_with_body", "oo::class create MyCls { method foo {} {} }"),
    ("oo_class_absolute", "oo::class create ::Top"),
    ("itcl_class", "itcl::class Foo { constructor {} {} }"),
    # -- packages --
    ("package_with_version", "package require Tcl 8.6"),
    ("package_without_version", "package require Tcl"),
    ("package_exact_flag", "package require -exact Tcl 8.6"),
    (
        "package_under_if_marked_conditional",
        "if {[catch {package require Tcl 8.6}]} { proc fallback {} {} }",
    ),
    # -- sources --
    ("source_literal_path", "source /abs/path.tcl"),
    # NOTE: a `source $script_dir/init.tcl` fixture (multi-token word
    # combining a `$var` substitution and a literal trailing path)
    # currently diverges between the Python and Rust segmenters —
    # Python widens `argv[i].end` to the end of the whole word, the
    # Rust segmenter keeps it at the first sub-token's end. The
    # divergence is in `rust/tcl-compiler/src/segmenter.rs` (the
    # `else if let Some(last_text) = …` branch), not in
    # `signature_scan` itself. Add a regression fixture once the
    # segmenter fix lands.
    ("source_with_encoding_option", "source -encoding utf-8 /a/b.tcl"),
    # -- interp aliases --
    (
        "interp_alias_local",
        "interp alias {} myalias {} puts hello",
    ),
    (
        "interp_alias_cross_skipped",
        "interp alias child foo {} puts",
    ),
    # -- namespace imports --
    ("namespace_import_direct", "namespace import ::tcl::mathfunc::*"),
    (
        "namespace_import_tcllib_wrapper",
        "term::ansi::send::import vt",
    ),
    # -- auto_path --
    ("lappend_auto_path", "lappend auto_path /opt/tclpkgs"),
    ("set_auto_path", "set auto_path /opt/tclpkgs"),
    # -- structured-control body recursion --
    (
        "if_branch_with_inner_proc",
        "if {$x} { proc thenproc {} {} } else { proc elseproc {} {} }",
    ),
    (
        "try_finally_with_inner_proc",
        "try { proc tryproc {} {} } finally { proc finallyproc {} {} }",
    ),
    (
        "catch_with_inner_proc",
        "catch { proc inner {} {} } result",
    ),
    # -- factory wrappers --
    (
        "tcllib_factory_wrapper",
        """
        proc DEFC {name args body} { proc $name $args $body }
        proc INIT {} {
            DEFC ChildA {x} {return 1}
            DEFC ChildB {y} {return 2}
        }
        """,
    ),
    (
        "factory_in_namespace",
        """
        namespace eval pkg {
            proc DEFC {name args body} { proc $name $args $body }
            proc INIT {} { DEFC Local {x} {return 1} }
        }
        """,
    ),
]


@pytest.mark.parametrize(
    ("label", "source"),
    CORPUS,
    ids=[label for label, _ in CORPUS],
)
def test_signature_scan_matches_python(label: str, source: str) -> None:
    """Field-by-field parity for every corpus fixture."""
    del label  # only used as the pytest id
    _assert_same(source)


# Dispatcher gating tests. The corpus tests above call the Rust path
# directly; these assert the env-var-gated dispatcher in
# ``core/analysis/signature_scan.py::extract_signatures`` behaves
# correctly.

import core.analysis.signature_scan as signature_scan_module  # noqa: E402
from core.analysis.signature_scan import extract_signatures  # noqa: E402


def test_dispatcher_default_off_uses_python(monkeypatch) -> None:
    """With the env var unset, ``extract_signatures`` must use the
    Python implementation regardless of binding availability."""
    monkeypatch.delenv("TCL_LSP_RUST_SIGNATURE_SCAN", raising=False)

    def _boom(_source):
        raise AssertionError("rust path must not be called when env var unset")

    monkeypatch.setattr(signature_scan_module, "_rust_extract", _boom)
    result = extract_signatures("proc foo {} {}")
    assert "::foo" in result.all_procs


def test_dispatcher_env_var_on_uses_rust(monkeypatch) -> None:
    """With ``TCL_LSP_RUST_SIGNATURE_SCAN=1``, the Rust path runs and
    its result is materialised by ``_materialise_rust_signatures``.
    A sentinel raw dict from a patched ``_rust_extract`` proves the
    dispatcher consulted the binding rather than the Python
    implementation."""
    monkeypatch.setenv("TCL_LSP_RUST_SIGNATURE_SCAN", "1")
    sentinel_raw = {
        "procs": {
            "::sentinel": {
                "name": "sentinel",
                "qualified_name": "::sentinel",
                "params": [],
                "name_range": (0, 8),
                "body_range": (10, 20),
            }
        },
        # Empty collections — exercise the materialiser's defaulting.
    }
    monkeypatch.setattr(signature_scan_module, "_rust_extract", lambda _src: sentinel_raw)
    result = extract_signatures("source ignored — patched rust returns sentinel")
    assert "::sentinel" in result.all_procs
    # Python path would have produced no procs for that source.
    assert "::foo" not in result.all_procs


def test_dispatcher_env_var_zero_does_not_enable_rust(monkeypatch) -> None:
    """``TCL_LSP_RUST_SIGNATURE_SCAN=0`` must NOT activate the rust
    path. Pre-fix this regressed: ``os.environ.get(name)`` was truthy
    for any non-empty string including ``"0"``, opting users into
    the experimental path when they thought they were disabling it.
    The shared ``rust_shim_enabled`` helper now recognises ``0`` /
    ``false`` / ``no`` / ``off`` (and the empty string) as off."""
    monkeypatch.setenv("TCL_LSP_RUST_SIGNATURE_SCAN", "0")

    def _boom(_source):
        raise AssertionError("rust path must not be called for env var = '0'")

    monkeypatch.setattr(signature_scan_module, "_rust_extract", _boom)
    result = extract_signatures("proc foo {} {}")
    assert "::foo" in result.all_procs


def test_dispatcher_rust_exception_falls_back_to_python(monkeypatch, caplog) -> None:
    """If the Rust path raises, the dispatcher logs at DEBUG and the
    Python implementation runs as a safety net."""
    monkeypatch.setenv("TCL_LSP_RUST_SIGNATURE_SCAN", "1")

    def _raise(_source):
        raise RuntimeError("simulated rust crash")

    monkeypatch.setattr(signature_scan_module, "_rust_extract", _raise)
    result = extract_signatures("proc foo {} {}")
    # Python path produced the proc — fallback worked.
    assert "::foo" in result.all_procs
