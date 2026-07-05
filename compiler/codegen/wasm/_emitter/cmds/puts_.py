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

"""WASM emit hook for ``puts`` — channel write.

``puts ?-nonewline? ?channelId? string`` — three call shapes the
hook handles directly so quoted-string arguments survive
substitution intact:

* ``puts msg``                     → :func:`tcl_cmd_puts`
* ``puts -nonewline msg``          → :func:`tcl_cmd_puts_nonewline`
* ``puts $chan msg``               → :func:`tcl_cmd_puts_chan`
* ``puts -nonewline $chan msg``    → :func:`tcl_cmd_puts_chan` with
                                     the no-newline flag

The channel-aware paths used to fall back to the interpreter, which
round-tripped any quoted-string arg through ``tcl_list_quote``.
That helper braces words containing spaces / ``$`` / ``[``, and
braced words suppress substitution — so ``puts $chan "==== $name
FAILED"`` reached :func:`tcl_eval` as ``puts $chan {==== $name
FAILED}`` and emitted the literal ``$name``.  Emitting the runtime
import directly skips the round-trip and preserves substitution.
"""

from __future__ import annotations

from compiler.registry import REGISTRY, EmitContext


def _emit_puts(emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext) -> bool:
    prep = emitter._runtime_prep("puts", args)
    if prep is None:
        return False
    func_idx, rimp = prep

    nonewline = len(args) >= 1 and args[0] == "-nonewline"
    chan_form = (not nonewline and len(args) >= 2) or (nonewline and len(args) >= 3)

    if chan_form:
        chan_idx = emitter._shared_imports.get("tcl_puts_chan")
        if chan_idx is not None:
            chan_arg_idx = 1 if nonewline else 0
            msg_arg_idx = len(args) - 1
            emitter._emit_value(args[chan_arg_idx])
            emitter._emit_value(args[msg_arg_idx])
            emitter._emit_i32_const(1 if nonewline else 0)
            emitter._emit_call(chan_idx)
            emitter._runtime_call_end(rimp, defs, context)
            return True

    if nonewline:
        no_nl_idx = emitter._shared_imports.get("tcl_puts_nonewline")
        if no_nl_idx is not None:
            emitter._emit_value(args[-1])
            emitter._emit_call(no_nl_idx)
            emitter._runtime_call_end(rimp, defs, context)
            return True
    if args:
        emitter._emit_value(args[-1])
    else:
        emitter._emit_i32_const(0)
    emitter._emit_call(func_idx)
    emitter._runtime_call_end(rimp, defs, context)
    return True


REGISTRY.register_wasm_emitter("puts", _emit_puts)
