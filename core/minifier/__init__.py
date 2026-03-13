"""Tcl source code minifier."""

from .minifier import MinifyResult, SymbolMap, minify_tcl, unminify_error

__all__ = ["MinifyResult", "SymbolMap", "minify_tcl", "unminify_error"]
