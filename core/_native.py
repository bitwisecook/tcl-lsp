"""Native C++ acceleration for core types.

Attempts to import SourcePosition, Range, and DocumentBuffer from the
compiled pybind11 extension module.  Falls back to the pure-Python
implementations when the native module is not available (e.g. the C++
code has not been built).
"""

try:
    from _tcl_lsp_native import (  # type: ignore[import-not-found]
        DocumentBuffer,
        Range,
        SourcePosition,
    )

    NATIVE = True
except ImportError:
    from .analysis.semantic_model import Range
    from .common.document_buffer import DocumentBuffer
    from .parsing.tokens import SourcePosition

    NATIVE = False

__all__ = ["SourcePosition", "Range", "DocumentBuffer", "NATIVE"]
