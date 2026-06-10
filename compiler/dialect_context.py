"""Process-wide active-dialect context — a dependency-free leaf.

Holds the :class:`contextvars.ContextVar` carrying the dialect the current
analysis runs under.  It lives here, below the registry and the lexer, so the
lexer can consult the active dialect (for dialect-specific tokenisation rules)
without importing :mod:`compiler.registry.runtime` — which would close a
``lexer`` ↔ ``green_tree`` ↔ ``registry.runtime`` import cycle.

The scope manager that sets/resets this var lives in
:mod:`compiler.registry.dialect` (``dialect_scope``); ``registry.runtime``
re-exports the var so existing ``from compiler.registry.runtime import
_dialect_var`` call sites keep working.  See issue #407.
"""

from __future__ import annotations

import contextvars

# Exported (consumed by registry.runtime / registry.dialect / parsing.lexer /
# server.diagnostics_pipeline); declared in ``__all__`` so it reads as this
# leaf's public surface even though the name keeps its registry-internal
# underscore spelling at the call sites.
__all__ = ["_dialect_var"]

_dialect_var: contextvars.ContextVar[str] = contextvars.ContextVar(
    "tcl_lsp_dialect", default="tcl8.6"
)
