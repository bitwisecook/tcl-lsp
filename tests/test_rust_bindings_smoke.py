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
    opts = tcl_lsp_rust.optimiser_find_optimisations(
        "if {1} { set x 1 } else { set y 2 }", None
    )
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
