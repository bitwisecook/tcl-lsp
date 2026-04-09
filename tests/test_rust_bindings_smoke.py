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
