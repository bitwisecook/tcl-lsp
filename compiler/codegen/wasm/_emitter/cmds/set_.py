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

"""WASM emit hooks for ``set`` and ``incr``."""

from __future__ import annotations

from compiler.registry import REGISTRY, EmitContext

from ..._ir import WasmOp
from ..._parsing import _parse_array_ref
from .._variables import _is_dynamic_var_name


def _emit_set(emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext) -> bool:
    """``set varName ?value?`` — read or write a local/global/aliased variable."""
    if not (1 <= len(args) <= 2):
        return False
    var = args[0]

    if context is EmitContext.VALUE:
        # At tail / value position the call's result must stay on the
        # stack.  Fast-path proc-locals via ``local.tee``; for aliases,
        # array elements (``arr(key)``), and unqualified names inside
        # a ``namespace eval`` body, route through the full
        # keep-on-stack write — ``_intern_local`` would otherwise
        # create a scalar slot named ``arr(key)`` and miss the array
        # hash table.
        array_ref = _parse_array_ref(var)
        array_base_aliased = array_ref is not None and array_ref[0] in emitter._aliases
        in_ns_block = (
            not emitter._is_proc
            and emitter._block_namespace is not None
            and emitter._block_namespace != "::"
            and array_ref is None
        )
        # Top-level vars must use the var path so the global mirror is
        # kept consistent with reads (which go through ``tcl_global_get``
        # at top level — see _variables.py Phase 4.5 finalisation).
        # A bare ``local.tee`` would update only the WASM local while
        # leaving globals stale.
        #
        # ``global``-declared names inside a proc also need the var
        # path so reads consult the global table (the WASM-local
        # mirror goes stale whenever a callee modifies the global
        # through its own ``global`` declaration — see the symmetric
        # branch in :meth:`_emit_var_read_obj`).
        at_top_level = not emitter._is_proc
        is_global = var in emitter._globals
        use_var_path = (
            at_top_level
            or is_global
            or var in emitter._aliases
            or array_base_aliased
            or array_ref is not None
            or in_ns_block
            or emitter._is_frame_only_var(var)
            or _is_dynamic_var_name(var)
        )
        if use_var_path:
            if len(args) >= 2:
                emitter._emit_value(args[1])
                emitter._emit_var_write_obj_keep(var)
            else:
                emitter._emit_var_read_obj(var)
            return True
        idx = emitter._intern_local(var)
        if len(args) >= 2:
            emitter._emit_value(args[1])
            emitter._emit_local_tee(idx)
        else:
            emitter._emit_local_get(idx)
        return True

    # STATEMENT context — result is either stored into ``defs[0]`` or
    # dropped.  ``_emit_var_write_obj`` already resolves aliases,
    # array refs, namespace qualification, and frame-only vars.
    if len(args) >= 2:
        ownership = emitter._emit_value(args[1])
        emitter._emit_var_write_obj(var, source=ownership)
    else:
        emitter._emit_var_read_obj(var)
        # Dynamic ``defs[0]`` (``::${n}`` etc.) has no static storage
        # key — drop the mirror.  See the matching guard in ``_emit_incr``.
        if defs and not _is_dynamic_var_name(defs[0]):
            def_idx = emitter._intern_local(defs[0])
            emitter._emit_local_set(def_idx)
        else:
            emitter._emit(WasmOp.DROP)
    return True


def _emit_incr_strict(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """Issue #262: route ``incr`` through the strict ``tcl_incr`` runtime
    helper so a non-integer current value (e.g. the float string ``"52.60"``
    or ``"abc"``) raises ``expected integer but got ...`` rather than being
    silently truncated/zeroed by the permissive inline unbox (``obj_get_int``).

    This mirrors the strict lowering the optimiser/CFG paths already use for
    ``IRIncr`` (``_optimisation.py`` / ``_control_flow.py`` / ``_statements.py``);
    the inline command emitter is reached for ``incr`` in value position and in
    un-optimised statement positions such as a ``catch {}`` body, which is where
    the permissiveness bug surfaced.

    Returns ``False`` when the runtime predates the ``tcl_incr`` import so the
    caller falls back to the legacy inline arithmetic.
    """
    incr_idx = emitter._shared_imports.get("tcl_incr")
    if incr_idx is None:
        return False
    var = args[0]
    array_ref = _parse_array_ref(var)
    base = array_ref[0] if array_ref else var

    # Compute the post-increment value (boxed TclObj) on the stack:
    #   tcl_incr(lenient_read(var), amount_obj)
    # Lenient read so ``incr x`` on an unset scalar initialises to 0
    # (Tcl 8.5+: returns the increment, doesn't raise) — same as the
    # optimised tail path.  ``tcl_incr`` enforces the strict-integer guard
    # on both the current value and a non-literal increment.
    emitter._emit_var_read_obj_lenient(var)
    if len(args) >= 2:
        try:
            emitter._emit_i64_const(int(args[1]))
            emitter._emit_box_int()
        except ValueError:
            emitter._emit_value(args[1])
    else:
        emitter._emit_i64_const(1)
        emitter._emit_box_int()
    emitter._emit_call(incr_idx)
    # Stack: [new_value_obj]

    in_ns_block = (
        not emitter._is_proc
        and emitter._block_namespace is not None
        and emitter._block_namespace != "::"
        and array_ref is None
    )
    use_var_path = (
        not emitter._is_proc
        or var in emitter._globals
        or var in emitter._aliases
        or base in emitter._aliases
        or array_ref is not None
        or in_ns_block
        or _is_dynamic_var_name(var)
    )

    if context is EmitContext.VALUE:
        # Keep the new value on the stack for the enclosing expression.
        if use_var_path:
            emitter._emit_var_write_obj_keep(var)
        else:
            idx = emitter._intern_local(var)
            emitter._emit_local_tee(idx)
        return True

    # STATEMENT context — write back (consume), mirroring the legacy path's
    # def-mirror capture for non-dynamic names.
    use_def_mirror = bool(defs) and not _is_dynamic_var_name(defs[0])
    if use_def_mirror:
        emitter._emit_var_write_obj_keep(var)
        def_idx = emitter._intern_local(defs[0])
        emitter._emit_local_set(def_idx)
    else:
        emitter._emit_var_write_obj(var)
    return True


def _emit_incr(emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext) -> bool:
    """``incr varName ?increment?`` — strict ``tcl_incr`` when available,
    otherwise legacy inline unbox / i64 add / rebox."""
    if not (1 <= len(args) <= 2):
        return False
    if _emit_incr_strict(emitter, args, defs, context):
        return True
    var = args[0]

    if context is EmitContext.VALUE:
        # Tail / value position: leave the new value on the stack.
        # Fast-path proc-local incr with ``local.tee``; otherwise read
        # via the var path, add, and write with keep.  ``arr(key)``
        # must go through the var path so the array hash table is
        # touched rather than an imaginary scalar local.
        array_ref = _parse_array_ref(var)
        base = array_ref[0] if array_ref else var
        in_ns_block = (
            not emitter._is_proc
            and emitter._block_namespace is not None
            and emitter._block_namespace != "::"
            and array_ref is None
        )
        # Top-level vars must use the var path so the global mirror is
        # kept consistent with reads (see set fast-path comment above).
        # ``global``-declared names need the var path too so the
        # global is the single source of truth.
        at_top_level = not emitter._is_proc
        is_global = var in emitter._globals
        if (
            at_top_level
            or is_global
            or var in emitter._aliases
            or base in emitter._aliases
            or array_ref is not None
            or in_ns_block
            or _is_dynamic_var_name(var)
        ):
            emitter._emit_var_read_obj(var)
            emitter._emit_unbox_int()
            amt = 1
            if len(args) >= 2:
                try:
                    amt = int(args[1])
                except ValueError:
                    emitter._emit_value(args[1])
                    emitter._emit_unbox_int()
                    emitter._emit(WasmOp.I64_ADD)
                    emitter._emit_box_int()
                    emitter._emit_var_write_obj_keep(var)
                    return True
            emitter._emit_i64_const(amt)
            emitter._emit(WasmOp.I64_ADD)
            emitter._emit_box_int()
            emitter._emit_var_write_obj_keep(var)
            return True
        idx = emitter._intern_local(var)
        emitter._emit_local_get(idx)
        emitter._emit_unbox_int()
        amt = 1
        if len(args) >= 2:
            try:
                amt = int(args[1])
            except ValueError:
                emitter._emit_value(args[1])
                emitter._emit_unbox_int()
                emitter._emit(WasmOp.I64_ADD)
                emitter._emit_box_int()
                emitter._emit_local_tee(idx)
                return True
        emitter._emit_i64_const(amt)
        emitter._emit(WasmOp.I64_ADD)
        emitter._emit_box_int()
        emitter._emit_local_tee(idx)
        return True

    # STATEMENT context — existing behaviour.
    # ``defs[0]`` carries the canonicalised IR name; for dynamic names
    # (``::${n}`` etc.) the captured-mirror local is meaningless — its
    # storage key isn't known until runtime — so we skip the mirror
    # capture and rely on the runtime var path alone.
    use_def_mirror = bool(defs) and not _is_dynamic_var_name(defs[0])
    emitter._emit_var_read_obj(var)
    emitter._emit_unbox_int()
    amt = 1
    if len(args) >= 2:
        try:
            amt = int(args[1])
        except ValueError:
            emitter._emit_value(args[1])
            emitter._emit_unbox_int()
            emitter._emit(WasmOp.I64_ADD)
            emitter._emit_box_int()
            if use_def_mirror:
                emitter._emit_var_write_obj_keep(var)
                def_idx = emitter._intern_local(defs[0])
                emitter._emit_local_set(def_idx)
            else:
                emitter._emit_var_write_obj(var)
            return True
    emitter._emit_i64_const(amt)
    emitter._emit(WasmOp.I64_ADD)
    emitter._emit_box_int()
    if use_def_mirror:
        emitter._emit_var_write_obj_keep(var)
        def_idx = emitter._intern_local(defs[0])
        emitter._emit_local_set(def_idx)
    else:
        emitter._emit_var_write_obj(var)
    return True


REGISTRY.register_wasm_emitter("set", _emit_set)
REGISTRY.register_wasm_emitter("incr", _emit_incr)
