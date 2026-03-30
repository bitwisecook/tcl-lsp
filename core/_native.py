"""Native C++ acceleration for core types.

Attempts to import SourcePosition, Range, and DocumentBuffer from the
compiled pybind11 extension module.  Falls back to the pure-Python
implementations when the native module is not available (e.g. the C++
code has not been built).
"""

import logging as _logging

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

# Phase 3: segmenter + recovery types (optional, only when native is available).
NATIVE_SEGMENTER = False
try:
    from _tcl_lsp_native import (  # type: ignore[import-not-found]
        CodeFix,
        Diagnostic,
        NativeSegmentedCommand,
        Severity,
        TopLevelChunk,
        UnclosedDelimiter,
        compute_virtual_insertions,
        find_first_dirty_chunk,
        position_from_relative,
        segment_commands,
        segment_top_level_chunks,
        segment_with_recovery,
    )

    NATIVE_SEGMENTER = True
except ImportError:
    pass

# Phase 4: analyser types (optional, only when native is available).
NATIVE_ANALYSER = False
try:
    from _tcl_lsp_native import (  # type: ignore[import-not-found]
        NativeAnalysisResult,
        NativeCommandInvocation,
        NativeConfidence,
        NativePackageContext,
        NativePackageProvide,
        NativePackageRequire,
        NativeParamDef,
        NativeProcArgTrait,
        NativeProcDef,
        NativeRegexPattern,
        NativeScope,
        NativeScopeKind,
        NativeSourceTarget,
        NativeStubArgDef,
        NativeStubCommandDef,
        NativeStubExprDef,
        NativeUnknownProcInfo,
        NativeVarDef,
        native_analyse,
        native_infer_param_traits_shallow,
    )

    NATIVE_ANALYSER = True
except ImportError:
    pass

_log = _logging.getLogger(__name__)
_log.debug(
    "Native C++ acceleration: core_types=%s, segmenter=%s, analyser=%s",
    NATIVE,
    NATIVE_SEGMENTER,
    NATIVE_ANALYSER,
)

__all__ = [
    "SourcePosition",
    "Range",
    "DocumentBuffer",
    "NATIVE",
    "NATIVE_SEGMENTER",
    "NATIVE_ANALYSER",
    # Phase 3 re-exports (only available when NATIVE_SEGMENTER is True).
    "CodeFix",
    "Diagnostic",
    "NativeSegmentedCommand",
    "Severity",
    "TopLevelChunk",
    "UnclosedDelimiter",
    "compute_virtual_insertions",
    "find_first_dirty_chunk",
    "position_from_relative",
    "segment_commands",
    "segment_top_level_chunks",
    "segment_with_recovery",
    # Phase 4 re-exports (only available when NATIVE_ANALYSER is True).
    "NativeAnalysisResult",
    "NativeCommandInvocation",
    "NativeConfidence",
    "NativePackageContext",
    "NativePackageProvide",
    "NativePackageRequire",
    "NativeParamDef",
    "NativeProcArgTrait",
    "NativeProcDef",
    "NativeRegexPattern",
    "NativeScope",
    "NativeScopeKind",
    "NativeSourceTarget",
    "NativeStubArgDef",
    "NativeStubCommandDef",
    "NativeStubExprDef",
    "NativeUnknownProcInfo",
    "NativeVarDef",
    "native_analyse",
    "native_infer_param_traits_shallow",
]
