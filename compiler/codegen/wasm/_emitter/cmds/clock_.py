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

"""Command helpers for ``clock`` — subcommand dispatch via the registry."""

from __future__ import annotations

from ..._imports import subcommand_runtime_import_for


def _emit_clock_value(emitter, args: tuple[str, ...]) -> None:
    """Emit a ``clock <subcmd>`` expression; leaves i32 TclObj on stack.

    ``seconds``/``clicks``/``milliseconds`` call the WASI-backed
    runtime helpers directly.  Anything else falls through to the
    interpreter (which will likely trap for ``format``/``scan`` in
    the sandbox — that's fine as a clear diagnostic until we ship a
    timezone-aware formatter).
    """
    if not args:
        emitter._emit_i32_const(0)
        return
    subcmd = args[0]
    sri = subcommand_runtime_import_for("clock", subcmd)
    if sri is not None:
        import_key = sri.import_key
        func_idx = emitter._shared_imports.get(import_key)
        if func_idx is not None:
            emitter._emit_call(func_idx)
            return
    # Fall back to the interpreter for unsupported subcommands.
    emitter._emit_eval_fallback("clock", args)


class _CmdClockMixin:
    """Expose the migrated helpers as methods for MRO composition."""

    _emit_clock_value = _emit_clock_value
