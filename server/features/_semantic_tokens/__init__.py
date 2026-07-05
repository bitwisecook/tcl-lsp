# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

from __future__ import annotations

from ._api import (
    _delta_encode,
    compute_semantic_tokens_edits,
    precompute_chunk_tokens,
    semantic_tokens_full,
)
from ._collect import _recover_stray_close_bracket_in_flush
from ._constants import (
    _BINARY_FORMAT_SPECIFIERS,
    SEMANTIC_TOKEN_MODIFIERS,
    SEMANTIC_TOKEN_TYPES,
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
