#!/usr/bin/env python3
"""Split lsp/features/semantic_tokens.py into _semantic_tokens/ package.

Usage: python _split_semantic_tokens.py
"""
from pathlib import Path

SRC = Path("lsp/features/semantic_tokens.py")
DST = Path("lsp/features/_semantic_tokens")
DST.mkdir(exist_ok=True)

text = SRC.read_text()
lines = text.splitlines(keepends=True)
total = len(lines)
print(f"Source: {total} lines")


def get(start: int, end: int) -> str:
    """Return lines start..end inclusive (1-indexed)."""
    return "".join(lines[i - 1] for i in range(start, min(end, total) + 1))


# ---------------------------------------------------------------------------
# Shared imports block — included verbatim in every submodule.
# All originals use absolute core.* paths so no depth adjustment needed.
# ---------------------------------------------------------------------------

SHARED_IMPORTS = """\
from __future__ import annotations

import logging
import re
import time
from bisect import bisect_right

from core.analysis.semantic_model import AnalysisResult
from core.bigip.apl_parser import AplTokenKind, tokenise_apl
from core.bigip.iapp_extract import find_embedded_iapp_sections
from core.bigip.irules_refs import extract_irules_object_references
from core.bigip.rule_extract import find_embedded_rules
from core.commands.registry.command_registry import REGISTRY
from core.commands.registry.runtime import (
    SIGNATURES,
    ArgRole,
    SubcommandSig,
    arg_indices_for_role,
    arg_indices_for_roles,
    iter_switch_case_list,
    options_with_value,
    regexp_pattern_index,
    skip_options,
)
from core.common.dialect import active_dialect
from core.common.document_buffer import DocumentBuffer
from core.common.ranges import position_from_offset
from core.parsing.expr_lexer import ExprTokenType, tokenise_expr
from core.parsing.known_commands import known_command_names
from core.parsing.lexer import TclLexer
from core.parsing.recovery import compute_virtual_insertions
from core.parsing.token_positions import token_content_base, token_content_shift
from core.parsing.tokens import SourcePosition, Token, TokenType
"""

# ---------------------------------------------------------------------------
# _constants.py  — token type/modifier lists, keyword sets, core regexes
# Lines 51–231 of original
# ---------------------------------------------------------------------------

CONSTANTS_HEADER = SHARED_IMPORTS + """\
log = logging.getLogger(__name__)

"""

constants_body = get(51, 231)
(DST / "_constants.py").write_text(CONSTANTS_HEADER + constants_body)
print(f"wrote _constants.py ({len((CONSTANTS_HEADER + constants_body).splitlines())} lines)")

# ---------------------------------------------------------------------------
# _bigip.py  — BIG-IP constants + all BIG-IP / APL / iRules functions
# Lines 232–879 of original
# _collect_apl_tokens and _collect_embedded_tcl_tokens both call _collect_tokens.
# We import it at the top level — no circular dep since _collect.py never
# imports from _bigip.py.
# ---------------------------------------------------------------------------

BIGIP_HEADER = SHARED_IMPORTS + """\
from ._constants import (
    _TYPE_INDEX,
)
from ._collect import _collect_tokens

log = logging.getLogger(__name__)

"""

bigip_body = get(232, 879)
(DST / "_bigip.py").write_text(BIGIP_HEADER + bigip_body)
print(f"wrote _bigip.py ({len((BIGIP_HEADER + bigip_body).splitlines())} lines)")

# ---------------------------------------------------------------------------
# _primitives.py  — core token emission helpers
# Lines 881–1090 of original
# NOTE: _collect_expression_tokens (lines 1091-1230) is intentionally excluded
# here and instead placed in _collect.py to avoid a circular import
# (_collect_expression_tokens calls _collect_tokens, which lives in _collect.py,
# and _collect.py imports from _primitives.py).
# ---------------------------------------------------------------------------

PRIMITIVES_HEADER = SHARED_IMPORTS + """\
from ._constants import (
    _TYPE_INDEX,
    _MOD_INDEX,
    _LANGUAGE_KEYWORDS,
    _OPERATORS,
    _BUILTIN_COMMANDS,
    _ESCAPE_RE,
)

log = logging.getLogger(__name__)

"""

primitives_body = get(881, 1090)
(DST / "_primitives.py").write_text(PRIMITIVES_HEADER + primitives_body)
print(f"wrote _primitives.py ({len((PRIMITIVES_HEADER + primitives_body).splitlines())} lines)")

# ---------------------------------------------------------------------------
# _format_args.py  — format/pattern arg-index finders + their collectors
# Lines 1232–2230 of original
# ---------------------------------------------------------------------------

FORMAT_ARGS_HEADER = SHARED_IMPORTS + """\
from ._constants import (
    _TYPE_INDEX,
    _MOD_INDEX,
    _BINARY_FORMAT_SPECIFIERS,
    _BINARY_INT_SPECIFIERS,
    _EVENT_RE,
)
from ._primitives import (
    _append_token,
    _append_text_token,
    _emit_string_with_escapes,
)

log = logging.getLogger(__name__)

"""

format_args_body = get(1232, 2230)
(DST / "_format_args.py").write_text(FORMAT_ARGS_HEADER + format_args_body)
print(f"wrote _format_args.py ({len((FORMAT_ARGS_HEADER + format_args_body).splitlines())} lines)")

# ---------------------------------------------------------------------------
# _collect.py  — _collect_expression_tokens (moved here from line 1091 to
# break mutual recursion with _collect_tokens) + recovery helpers + main loop
# Lines 1091–1230 + 2231–2957 of original
# ---------------------------------------------------------------------------

COLLECT_HEADER = SHARED_IMPORTS + """\
from ._constants import (
    _TYPE_INDEX,
    _MOD_INDEX,
    _BUILTIN_COMMANDS,
    _EVENT_RE,
)
from ._primitives import (
    _classify_token,
    _classify_expr_token,
    _append_token,
    _append_text_token,
    _emit_namespace_qualified,
    _emit_string_with_escapes,
)
from ._format_args import (
    _proc_param_list_arg_index,
    _collect_param_list_tokens,
    _procedure_name_arg_index,
    _binary_format_arg_index,
    _collect_binary_format_spec_tokens,
    _subcommand_arg_index,
    _is_known_subcommand,
    _sprintf_format_arg_index,
    _collect_sprintf_format_spec_tokens,
    _clock_format_arg_index,
    _collect_clock_format_spec_tokens,
    _regsub_subspec_arg_index,
    _collect_regsub_subspec_tokens,
    _string_map_mapping_arg_index,
    _collect_string_map_pairs_tokens,
    _glob_pattern_arg_indices,
    _collect_glob_pattern_tokens,
    _option_arg_indices,
    _collect_switch_case_bodies,
    _emit_regex_token,
)

log = logging.getLogger(__name__)

"""

collect_body = get(1091, 1230) + get(2231, 2957)
(DST / "_collect.py").write_text(COLLECT_HEADER + collect_body)
print(f"wrote _collect.py ({len((COLLECT_HEADER + collect_body).splitlines())} lines)")

# ---------------------------------------------------------------------------
# _api.py  — public API
# Lines 2958–end of original
# ---------------------------------------------------------------------------

API_HEADER = SHARED_IMPORTS + """\
from ._collect import _collect_tokens
from ._bigip import (
    _collect_bigip_tokens,
    _collect_bigip_embedded_irules_object_tokens,
    _collect_irules_object_tokens,
    _collect_apl_tokens,
    _collect_embedded_tcl_tokens,
)

log = logging.getLogger(__name__)

"""

api_body = get(2958, total)
(DST / "_api.py").write_text(API_HEADER + api_body)
print(f"wrote _api.py ({len((API_HEADER + api_body).splitlines())} lines)")

# ---------------------------------------------------------------------------
# __init__.py  — re-exports all names the shim needs
# ---------------------------------------------------------------------------

INIT_CONTENT = """\
from __future__ import annotations

from ._constants import (
    SEMANTIC_TOKEN_TYPES,
    SEMANTIC_TOKEN_MODIFIERS,
    _BINARY_FORMAT_SPECIFIERS,
)
from ._format_args import (
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
)
from ._collect import _recover_stray_close_bracket_in_flush
from ._api import (
    semantic_tokens_full,
    precompute_chunk_tokens,
    _delta_encode,
    compute_semantic_tokens_edits,
)

__all__ = [
    "SEMANTIC_TOKEN_TYPES",
    "SEMANTIC_TOKEN_MODIFIERS",
    "semantic_tokens_full",
    "compute_semantic_tokens_edits",
    "precompute_chunk_tokens",
    "_BINARY_FORMAT_SPECIFIERS",
    "_CLOCK_FORMAT_RE",
    "_GLOB_META_RE",
    "_REGEX_PART_RE",
    "_REGSUB_BACKREF_RE",
    "_SPRINTF_RE",
    "_binary_format_arg_index",
    "_clock_format_arg_index",
    "_glob_pattern_arg_indices",
    "_regex_pattern_arg_index",
    "_regsub_subspec_arg_index",
    "_sprintf_format_arg_index",
    "_recover_stray_close_bracket_in_flush",
    "_delta_encode",
]
"""

(DST / "__init__.py").write_text(INIT_CONTENT)
print(f"wrote __init__.py ({len(INIT_CONTENT.splitlines())} lines)")

# ---------------------------------------------------------------------------
# Rewrite semantic_tokens.py as a re-export shim
# ---------------------------------------------------------------------------

SHIM = """\
\"\"\"Semantic token provider for Tcl source — implementation in _semantic_tokens/ package.\"\"\"
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
"""

SRC.write_text(SHIM)
print(f"rewrote semantic_tokens.py as shim ({len(SHIM.splitlines())} lines)")
print("Done.")
