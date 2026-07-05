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

"""Lightweight data container for taint sink classification results."""

from __future__ import annotations

from dataclasses import dataclass


@dataclass(frozen=True, slots=True)
class TaintSinkInfo:
    """Result of a single-pass taint sink classification query.

    ``network_sink_args`` carries the argument positions that flow into
    the network-address slot (D5-T104):

    * ``None`` -> not a network sink at all.
    * empty tuple ``()`` -> whole-statement scan (any tainted arg is a
      network sink candidate -- used by iRules ``connect`` that takes
      the address via opaque options).
    * non-empty tuple -> the listed positional indexes are the network
      slots; T104 fires only when the tainted var sits in one of them.
    """

    is_code_sink: bool = False
    output_sink: str | None = None
    output_sink_is_subcommand_qualified: bool = False
    log_sink: str | None = None
    network_sink_args: tuple[int, ...] | None = None
    interp_eval_subcommands: frozenset[str] | None = None


_EMPTY_TAINT_SINK_INFO = TaintSinkInfo()
