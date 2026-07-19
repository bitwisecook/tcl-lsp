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

from dataclasses import dataclass, field

from ..semantic_model import AnalysisResult, Range, Scope


@dataclass
class AnalyserSnapshot:
    """Checkpoint of ``Analyser`` state at a chunk boundary.

    Captures the cumulative ``AnalysisResult`` and internal tracking
    state so that the analyser can resume from a checkpoint when only
    later chunks have changed.  The scope tree is deep-copied so that
    the snapshot is fully independent of live analyser state.
    """

    result: AnalysisResult
    last_comment: str
    const_strings: dict[int, dict[str, tuple[str, Range]]]
    regex_vars: set[tuple[int, str]]
    current_event: str | None
    conditional_depth: int = 0
    # Command aliases: alias_name -> (target_cmd, prepended_args).
    command_aliases: dict[str, tuple[str, tuple[str, ...]]] = field(default_factory=dict)
    # Map from old scope id → scope object for scope identity reconstruction.
    scope_id_map: dict[int, Scope] = field(default_factory=dict)
    # Variable-as-command sites: (var_name, method_name_or_None,
    # token_range, in_method, is_single_token_word, positional_arg_count).
    var_command_sites: list[tuple[str, str | None, Range, bool, bool, int]] = field(
        default_factory=list
    )
    # Command-substitution-as-command sites: (cmd_text, method_name_or_None,
    # range, in_method, is_single_token_word).
    cmd_command_sites: list[tuple[str, str | None, Range, bool, bool]] = field(default_factory=list)
    # TclOO method-dispatch sites for find-references: (receiver, key,
    # method_name, method_name_range, enclosing_class).  See ``_core`` init.
    method_dispatch_sites: list[tuple[str, str | None, str, Range, str | None]] = field(
        default_factory=list
    )
    # Pending trace-callback registrations: tuples of candidate qualified
    # proc names for callbacks whose target may not yet be parsed.
    pending_trace_callbacks: list[tuple[str, ...]] = field(default_factory=list)
    # Cumulative cross-chunk state that must survive snapshot/restore, or an
    # incremental run drops it and diverges from a full rebuild:
    #   objdefined_vars  — vars given object-specific methods via oo::objdefine
    #                      (else a later ``$obj extra`` falsely warns W308).
    #   ensemble_namespaces — namespaces declared as ensembles.
    objdefined_vars: set[str] = field(default_factory=set)
    ensemble_namespaces: set[str] = field(default_factory=set)
