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

"""WASM emit hook for ``return``."""

from __future__ import annotations

from compiler.registry import REGISTRY, EmitContext

from ..._ir import WasmOp


def _emit_return(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``return ?-code code? ?value?`` — delegates to _emit_cmd_return.

    Both STATEMENT and VALUE context route through ``_emit_cmd_return``
    so the full ``-code`` / ``-level`` / ``-errorcode`` semantics fire
    (in particular, ``return -code error msg`` must call
    ``tcl_cmd_error`` and raise, not just push ``"-code"`` onto the
    stack).  The WASM ``return`` instruction makes any code emitted
    after this unreachable — the tail dispatcher's expectation of a
    result on the stack is satisfied by the ``i32.const 0`` the
    return emitter pushes before ``WasmOp.RETURN``, but the function
    exits before the caller can observe it.
    """
    emitter._emit_cmd_return(args)
    return True


REGISTRY.register_wasm_emitter("return", _emit_return)


def _emit_cmd_return(emitter, args: tuple[str, ...]) -> None:
    """``return ?value?`` or ``return -code code ?value?``.

    The simple single-value form compiles to a WASM return.
    ``return -code error <msg>`` is special-cased inline: evaluate
    *msg* via :meth:`_emit_value` (so embedded ``$var`` /
    ``[cmd]`` substitutions work), then call ``tcl_cmd_error``
    — which sets the catch's ``error_flag``/``error_msg`` when
    inside a ``catch`` or traps otherwise.  Going through
    :meth:`_emit_eval_fallback` would brace-wrap the message to
    preserve list structure, blocking the substitutions the
    error text needs — a real hazard because tcltest's error
    messages embed ``$option`` / ``$values`` everywhere.

    Other ``-code`` forms (``return -code break``, ``return -code
    continue``, numeric codes, ``-level N``, ``-errorinfo``
    ``-errorcode``) are rarer and fall through to the eval
    fallback, whose argument quoting is safe for them because
    their payloads are typically literal keywords or numeric
    values without interpolation.
    """
    if args and len(args) >= 3 and args[0] == "-code" and args[1] == "error" and len(args) == 3:
        # return -code error <msg>
        emitter._emit_value(args[2])
        # Route through ``tcl_cmd_error_via_return`` (NOT plain
        # ``tcl_cmd_error``) so the surrounding proc's epilogue
        # skips the ``(procedure "X" line N)`` frame stamp —
        # Tcl 9 ``TclUpdateReturnInfo`` bypasses ``MakeProcError``
        # for the ``return -code error`` path (proc-old-7.2).
        err_idx = emitter._shared_imports.get("tcl_error_via_return")
        if err_idx is None:
            err_idx = emitter._shared_imports.get("tcl_error")
        if err_idx is None:
            emitter._emit_eval_fallback("return", args)
            return
        emitter._emit_call(err_idx)
        # tcl_cmd_error returns nothing; emit a null TclObj for
        # the WASM return value.  When inside a catch, error_flag
        # is now set and the catch body's has_error check will
        # trip on the next statement.  When outside a catch, the
        # runtime's tcl_cmd_error already traps and this return
        # is unreachable.
        emitter._emit_i32_const(0)
        emitter._emit(WasmOp.RETURN)
        return
    if args and args[0].startswith("-"):
        emitter._emit_eval_fallback("return", args)
        emitter._emit(WasmOp.DROP)
        return
    # Inside a compile-time ``catch`` body, route through the runtime
    # so ``return`` sets ``return_flag`` instead of emitting a WASM
    # ``return`` that would jump past ``catch_leave``.  ``catch`` is the
    # universal sink for non-OK return codes — without this, a
    # ``catch {return foo}`` exits the surrounding proc with ``foo``
    # instead of returning TCL_RETURN to the catch.
    if emitter._catch_depth > 0:
        emitter._emit_eval_fallback("return", args)
        emitter._emit(WasmOp.DROP)
        return
    if args:
        emitter._emit_value(args[0])
    else:
        emitter._emit_i32_const(0)
    emitter._emit(WasmOp.RETURN)


class _CmdReturnMixin:
    """Expose the migrated helpers as methods for MRO composition."""

    _emit_cmd_return = _emit_cmd_return
