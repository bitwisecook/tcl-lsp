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

"""WASM emit hook for ``fconfigure`` — channel option get/set.

The Zig ``tcl_cmd_fconfigure`` export takes a 2-arg signature
``(fd, opts_obj)``; this hook packs a variadic ``-option value``
sequence into one opts TclObj.  Fast path: all-literal options are
pre-joined at compile time as a canonical Tcl list (so empty
values like ``-eofchar {}`` round-trip through the runtime
``list_parse`` walk); mixed literal + ``$var`` / ``[cmd]``
references fall through to the eval interpreter, where
:func:`runtime/zig/cmds/chan.zig:eval_fconfigure` does the same
list-element quoting against the live values.
"""

from __future__ import annotations

from compiler.registry import REGISTRY, EmitContext
from shared.tcl_list import tcl_list_quote


def _emit_fconfigure(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    prep = emitter._runtime_prep("fconfigure", args)
    if prep is None:
        return False
    func_idx, rimp = prep

    if not args:
        emitter._emit_i32_const(0)
        emitter._emit_i32_const(0)
    else:
        emitter._emit_value(args[0])
        rest = args[1:]

        def _is_lit(a: str) -> bool:
            # Match the ``concat`` / ``string cat`` predicate: reject
            # bare ``$var`` / ``[cmd]`` prefixes, embedded
            # substitutions anywhere in the word, and any name that
            # would resolve to an alias or local variable.
            return (
                not a.startswith("$")
                and not a.startswith("[")
                and not emitter._has_embedded_subst(a)
                and a not in emitter._aliases
                and a not in emitter._local_index
            )

        if not rest:
            emitter._emit_i32_const(0)
        elif all(_is_lit(a) for a in rest):
            # Quote each word as a canonical Tcl list element so the
            # runtime parser (``tcl_list_parse``) recovers the
            # original bytes.  Plain ``" ".join(rest)`` would drop
            # empty values — ``fconfigure $fd -eofchar {}`` would
            # otherwise be emitted as bare ``-eofchar`` and silently
            # turn into a query at the runtime call.
            quoted = [tcl_list_quote(a, first=(i == 0)) for i, a in enumerate(rest)]
            emitter._emit_obj_literal(" ".join(quoted))
        else:
            # Mixed literal + dynamic — fall back to the eval
            # interpreter so the values reach
            # ``cmds/chan.zig:eval_fconfigure``, which list-quotes
            # each live word.  The previous chained-``tcl_concat``
            # build had the same empty-value-collapsing issue as the
            # naive literal join.
            emitter._emit_eval_fallback("fconfigure", args)
            return True
    emitter._emit_call(func_idx)
    emitter._runtime_call_end(rimp, defs, context)
    return True


REGISTRY.register_wasm_emitter("fconfigure", _emit_fconfigure)
