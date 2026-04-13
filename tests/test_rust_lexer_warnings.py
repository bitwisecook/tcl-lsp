"""Parity test for lexer warnings in the Rust fast-path.

When the Rust ``tcl_lsp_rust`` wheel is installed,
``TclLexer.tokenise_all()`` dispatches to the Rust implementation.
Prior to L11-warnings, that path silently dropped the non-fatal
warnings (extra chars after close-brace / close-quote, unterminated
strings) that the Python lexer collected on ``self.warnings`` —
editors lose those diagnostics whenever the wheel is loaded.

This module verifies that a known set of warning-triggering inputs
produces the *same* ``(SourcePosition, message)`` warning tuples
whether or not the Rust fast-path is active.
"""

from __future__ import annotations

import sys
from pathlib import Path

import pytest

sys.path.insert(0, str(Path(__file__).resolve().parent.parent))

import core.parsing.lexer as _lexer_module
from core.parsing.lexer import TclLexer

# These sources all trigger a single recoverable warning on both
# the Python and Rust paths. Each is a minimal reproducer for one
# ``warn_or_error`` call site in the lexer whose warning offset
# already agrees between the two implementations. The ``${name``
# unterminated-braced-var case is omitted because Python reports
# the ``$`` start offset while Rust reports EOF — a pre-existing
# offset-tracking divergence unrelated to L11-warnings.
_WARNING_CASES: list[tuple[str, str]] = [
    ("set x {unclosed", "missing close-brace"),
    ('"hello', 'missing "'),
    ("[hello", "missing close-bracket"),
    ('"hello"world', "extra characters after close-quote"),
    ("{hello}world", "extra characters after close-brace"),
]


def _warnings_with_rust(source: str) -> list[tuple[int, str]]:
    """Tokenise via whichever path is active (Rust if installed)."""
    lexer = TclLexer(source)
    lexer.tokenise_all()
    return [(p.offset, msg) for p, msg in lexer.warnings]


def _warnings_python_only(source: str) -> list[tuple[int, str]]:
    """Tokenise forcing the Python fallback (Rust path disabled)."""
    saved = _lexer_module._rust_lexer_tokenise_cfg
    _lexer_module._rust_lexer_tokenise_cfg = None
    try:
        lexer = TclLexer(source)
        lexer.tokenise_all()
        return [(p.offset, msg) for p, msg in lexer.warnings]
    finally:
        _lexer_module._rust_lexer_tokenise_cfg = saved


@pytest.mark.parametrize("source,expected_msg", _WARNING_CASES)
def test_rust_path_collects_warning(source: str, expected_msg: str) -> None:
    """The Rust fast-path must populate ``TclLexer.warnings``."""
    if _lexer_module._rust_lexer_tokenise_cfg is None:
        pytest.skip("Rust wheel not installed; fast-path not exercised.")
    warnings = _warnings_with_rust(source)
    assert any(expected_msg in msg for _, msg in warnings), (
        f"expected warning containing {expected_msg!r} from Rust path, got {warnings!r}"
    )


@pytest.mark.parametrize("source,_expected_msg", _WARNING_CASES)
def test_warnings_match_python_fallback(source: str, _expected_msg: str) -> None:
    """Rust and Python paths must produce identical warning tuples."""
    if _lexer_module._rust_lexer_tokenise_cfg is None:
        pytest.skip("Rust wheel not installed; fast-path not exercised.")
    rust_warnings = _warnings_with_rust(source)
    python_warnings = _warnings_python_only(source)
    assert rust_warnings == python_warnings, (
        f"warning parity broken for {source!r}: "
        f"rust={rust_warnings!r} python={python_warnings!r}"
    )
