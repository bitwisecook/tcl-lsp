"""Semantic token provider for Tcl source — implementation in _semantic_tokens/ package."""
from ._semantic_tokens import (
    # Public API (lsp/server.py, tests)
    SEMANTIC_TOKEN_TYPES,
    SEMANTIC_TOKEN_MODIFIERS,
    semantic_tokens_full,
    compute_semantic_tokens_edits,
    precompute_chunk_tokens,
    # Semi-public used by hover.py and inlay_hints.py
    _BINARY_FORMAT_SPECIFIERS,
    _CLOCK_FORMAT_RE,
    _GLOB_META_RE,
    _REGEX_PART_RE,
    _REGSUB_BACKREF_RE,
    _SPRINTF_RE,
    _binary_format_arg_index,
    _clock_format_arg_index,
    _glob_pattern_arg_indices,
    _regex_pattern_arg_index,
    _regsub_subspec_arg_index,
    _sprintf_format_arg_index,
    # Tested directly
    _recover_stray_close_bracket_in_flush,
)

__all__ = [
    "SEMANTIC_TOKEN_TYPES",
    "SEMANTIC_TOKEN_MODIFIERS",
    "semantic_tokens_full",
    "compute_semantic_tokens_edits",
    "precompute_chunk_tokens",
]
