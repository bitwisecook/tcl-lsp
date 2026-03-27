"""Native C++ acceleration for core types.

Attempts to import SourcePosition, Range, and DocumentBuffer from the
compiled pybind11 extension module.  Falls back to the pure-Python
implementations when the native module is not available (e.g. the C++
code has not been built).
"""

try:
    from _tcl_lsp_native import DocumentBuffer  # type: ignore[import-not-found]
    from _tcl_lsp_native import Range  # type: ignore[import-not-found]
    from _tcl_lsp_native import SourcePosition  # type: ignore[import-not-found]

    NATIVE = True
except ImportError:
    from .analysis.semantic_model import Range
    from .common.document_buffer import DocumentBuffer
    from .parsing.tokens import SourcePosition

    NATIVE = False

__all__ = ["SourcePosition", "Range", "DocumentBuffer", "NATIVE"]
