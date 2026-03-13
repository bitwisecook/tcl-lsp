"""Shared server utilities."""

from .dialect import Dialect, active_dialect
from .lsp import find_var_in_scopes, to_lsp_location, to_lsp_range
from .naming import normalise_qualified_name, normalise_var_name
from .position import (
    find_command_at_position,
    find_token_in_command,
    offset_at_position,
    position_in_range,
)
from .ranges import position_from_relative, range_from_token, range_from_tokens
from .source_map import SourceMap, offset_to_line_col
from .suffix_array import build_lcp_array, build_suffix_array
from .text_edits import apply_edits, name_generator

__all__ = [
    "Dialect",
    "active_dialect",
    "apply_edits",
    "build_lcp_array",
    "build_suffix_array",
    "find_command_at_position",
    "find_token_in_command",
    "find_var_in_scopes",
    "name_generator",
    "normalise_qualified_name",
    "normalise_var_name",
    "offset_at_position",
    "offset_to_line_col",
    "position_from_relative",
    "position_in_range",
    "range_from_token",
    "range_from_tokens",
    "SourceMap",
    "to_lsp_location",
    "to_lsp_range",
]
