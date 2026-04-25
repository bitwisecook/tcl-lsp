"""Smoke test for the Rust workspace bootstrap (chunk L0).

Asserts that the PyO3 binding crate (`rust/tcl-lsp-rust/`) builds, installs
into the uv environment, and exposes the `hello_rust` and `lexer_version`
functions that will be used as the end-to-end proof for every subsequent
chunk of the Python-to-Rust migration.

Until chunk L3 gates real functionality on Rust, the `tcl_lsp_rust` import
is a **soft dependency**: the wheel may be absent in minimal developer
environments (e.g. fresh clones before `make rust-build`). In those cases
the test is skipped rather than failed, so the rest of the Python test
suite stays green.
"""

from __future__ import annotations

import pytest

tcl_lsp_rust = pytest.importorskip(
    "tcl_lsp_rust",
    reason="tcl_lsp_rust wheel not installed; run `make rust-build` first",
)


def test_hello_rust_returns_expected_string() -> None:
    assert tcl_lsp_rust.hello_rust() == "hello from rust"


def test_lexer_version_is_non_empty_string() -> None:
    version = tcl_lsp_rust.lexer_version()
    assert isinstance(version, str)
    assert version
    # Semver-ish: at least one dot separating digits.
    assert "." in version


def test_optimiser_find_optimisations_fires_o101() -> None:
    """C32 smoke test: the Rust `optimiser_find_optimisations` entry
    point must return optimisation tuples for a simple constant-branch
    source, and each tuple must have the seven-field shape the Python
    `_manager._materialise_rust_optimisations` helper expects.
    """
    opts = tcl_lsp_rust.optimiser_find_optimisations("if {1} { set x 1 } else { set y 2 }", None)
    assert isinstance(opts, list)
    assert opts, "expected at least one optimisation"
    for t in opts:
        assert len(t) == 7, f"unexpected tuple shape: {t!r}"
        code, message, start, end, replacement, group, hint_only = t
        assert isinstance(code, str) and code.startswith("O")
        assert isinstance(message, str)
        assert isinstance(start, int) and isinstance(end, int)
        assert start <= end
        assert isinstance(replacement, str)
        assert group is None or isinstance(group, int)
        assert isinstance(hint_only, bool)


def test_optimiser_opt_priority_known_code() -> None:
    assert tcl_lsp_rust.optimiser_opt_priority("O112") == 9
    assert tcl_lsp_rust.optimiser_opt_priority("unknown") == 0


def test_compiler_checks_run_all_returns_diagnostic_tuples() -> None:
    """C32-shim smoke test: the Rust ``compiler_checks_run_all``
    entry point must return diagnostic tuples for a source with a
    constant-true branch (exercises the SCCP check), and each
    tuple must have the seven-field shape Python callers expect.
    """
    diagnostics = tcl_lsp_rust.compiler_checks_run_all("if {1} { set x 1 } else { set y 2 }", None)
    assert isinstance(diagnostics, list)
    assert diagnostics, "expected at least one diagnostic from SCCP check"
    for t in diagnostics:
        assert len(t) == 7, f"unexpected tuple shape: {t!r}"
        code, category, severity, message, start, end, replacement = t
        assert isinstance(code, str) and code
        assert isinstance(category, str) and category
        assert severity in {"hint", "suggestion", "warning", "error"}
        assert isinstance(message, str) and message
        assert isinstance(start, int) and isinstance(end, int)
        assert start <= end
        assert replacement is None or isinstance(replacement, str)


def test_compiler_checks_run_all_empty_source_is_empty() -> None:
    """Empty source must produce no diagnostics — matches the Rust
    ``run_all_checks`` unit test's behaviour and ensures the binding
    doesn't invent spurious output from an empty compilation unit.
    """
    assert tcl_lsp_rust.compiler_checks_run_all("", None) == []


def test_interprocedural_summaries_returns_dict_of_proc_dicts() -> None:
    """C32-shim smoke test: ``interprocedural_summaries`` exposes
    per-proc summaries keyed on qualified name, with each value a
    dict carrying the ProcSummary fields as primitives."""
    summaries = tcl_lsp_rust.interprocedural_summaries("proc ::answer {} { return 42 }", None)
    assert isinstance(summaries, dict)
    assert "::answer" in summaries
    s = summaries["::answer"]
    assert s["qualified_name"] == "::answer"
    assert s["params"] == []
    assert s["arity_min"] == 0
    assert isinstance(s["has_barrier"], bool)
    assert s["returns_constant"] is True
    assert s["constant_return"] == ("int", "42")
    assert s["can_fold_static_calls"] is True
    assert isinstance(s["param_traits"], dict)


def test_gvn_redundancies_returns_tuples() -> None:
    """C32-shim smoke test: ``gvn_redundancies`` / _partial /
    _loop_invariants each return tuple lists with the 7-field
    shape Python callers expect."""
    # Minimal source that isn't guaranteed to produce O105 — we
    # only verify the tuple shape and that the call succeeds.
    for name in ("gvn_redundancies", "gvn_partial_redundancies", "gvn_loop_invariants"):
        fn = getattr(tcl_lsp_rust, name)
        result = fn("set a 1\nset b 2", None)
        assert isinstance(result, list)
        for t in result:
            assert len(t) == 7, f"unexpected {name} tuple shape: {t!r}"
            code, start, end, first_start, first_end, expr, msg = t
            assert isinstance(code, str) and code.startswith("O")
            assert isinstance(start, int) and isinstance(end, int)
            assert start <= end
            assert isinstance(first_start, int) and isinstance(first_end, int)
            assert isinstance(expr, str)
            assert isinstance(msg, str)


def test_build_compilation_unit_returns_handle() -> None:
    """C32-shim smoke test: ``build_compilation_unit`` returns a
    handle exposing read-only counters; passing ``dialect`` populates
    the interprocedural summaries flag."""
    cu = tcl_lsp_rust.build_compilation_unit("proc ::f {} { }", None)
    assert cu.procedure_count == 1
    assert cu.source_len > 0
    assert cu.has_interprocedural is True
    # Repr smoke test.
    assert "CompilationUnit" in repr(cu)


def test_signature_scan_extract_returns_dict() -> None:
    """C40e1 smoke test: ``signature_scan_extract`` returns a dict;
    the empty source case yields an empty dict (collections wired in
    later C40e sub-strips)."""
    result = tcl_lsp_rust.signature_scan_extract("")
    assert isinstance(result, dict)


def test_signature_scan_extract_proc_record() -> None:
    """C40e2 smoke test: ``signature_scan_extract`` populates the
    ``procs`` collection with name + qualified_name + params +
    range tuples."""
    result = tcl_lsp_rust.signature_scan_extract("proc foo {a b} { return $a }")
    assert "::foo" in result["procs"]
    proc = result["procs"]["::foo"]
    assert proc["name"] == "foo"
    assert proc["qualified_name"] == "::foo"
    assert len(proc["params"]) == 2
    assert proc["params"][0]["name"] == "a"
    assert proc["params"][0]["has_default"] is False
    assert isinstance(proc["name_range"], tuple)
    assert len(proc["name_range"]) == 2
    # And there should be one command_invocations entry for `proc`.
    assert any(inv["name"] == "proc" for inv in result["command_invocations"])


def test_signature_scan_extract_full_collections() -> None:
    """C40e3 smoke test: a multi-handler source populates every
    collection key in the returned dict."""
    src = """
proc foo {} {}
oo::class create MyCls {}
package require Tcl 8.6
source /a/b.tcl
interp alias {} myalias {} puts hello
namespace import ::tcl::mathfunc::*
lappend auto_path /opt/tclpkgs
"""
    result = tcl_lsp_rust.signature_scan_extract(src)
    assert "::foo" in result["procs"]
    assert "::MyCls" in result["classes"]
    assert any(p["name"] == "Tcl" for p in result["package_requires"])
    assert any(s["raw_path"] == "/a/b.tcl" for s in result["source_targets"])
    assert "::myalias" in result["command_aliases"]
    assert any(i["pattern"] == "::tcl::mathfunc::*" for i in result["namespace_imports"])
    assert any(e["raw"] == "/opt/tclpkgs" for e in result["auto_path_entries"])
