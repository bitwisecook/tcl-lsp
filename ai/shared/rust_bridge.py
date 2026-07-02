"""Single import point for the Rust ``tcl_lsp_py`` facade surface.

The native wheel is the *terminal* product of the Rust rewrite (see
``tests/test_public_pyo3_api.py``): a small, semver-stable API the AI tools
call directly instead of the in-tree Python ``compiler`` / ``analyser`` /
``tooling`` engine that it replaces.

Import is soft — a missing wheel does not break module import, so unrelated
tools still load on a fresh checkout without ``make rust-build``. Call
:func:`require_rust` at use time to get the module or a clear error.
"""

from __future__ import annotations

from typing import Any

try:  # pragma: no cover - exercised via the built wheel in CI
    import tcl_lsp_py as _rust
except ImportError:  # pragma: no cover - wheel not built (fresh clone)
    _rust = None


def rust() -> Any | None:
    """The ``tcl_lsp_py`` module, or ``None`` when the wheel is not installed."""
    return _rust


def require_rust() -> Any:
    """Return the ``tcl_lsp_py`` module, or raise a clear build hint.

    Raises
    ------
    RuntimeError
        When the native extension is not importable — the caller should build
        it with ``make rust-build`` (or ``maturin develop`` in
        ``rust/tcl-lsp-py``).
    """
    if _rust is None:
        raise RuntimeError(
            "tcl_lsp_py native extension not available — build it with "
            "`make rust-build` (or `maturin develop` under rust/tcl-lsp-py)."
        )
    return _rust
