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

"""WASM emit hook for ``info``."""

from __future__ import annotations

from compiler.registry import REGISTRY, EmitContext

from ..._ir import WasmOp
from ..._parsing import _parse_array_ref


def _emit_info(emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext) -> bool:
    """``info subcommand ?arg?`` — delegates to ``_emit_info_value``."""
    if not args:
        if context is EmitContext.VALUE:
            return False
        emitter._emit_unsupported_trap("info (no subcommand)")
        return True
    if context is EmitContext.VALUE:
        emitter._emit_info_value(args)
        return True
    emitter._emit_info_value(args)
    if defs:
        def_idx = emitter._intern_local(defs[0])
        emitter._emit_local_set(def_idx)
    else:
        emitter._emit(WasmOp.DROP)
    return True


REGISTRY.register_wasm_emitter("info", _emit_info)


def _emit_info_value(emitter, args: tuple[str, ...]) -> None:
    """Leave the i32 TclObj result of ``info <args>`` on the stack.

    ``info exists varName`` is inlined so that compile-time scope
    information (alias bindings, ``::``-qualified globals) informs the
    lookup; everything else falls through to the runtime
    ``info_dispatch`` helper for subcommands it understands.
    """
    if not args:
        emitter._emit_i32_const(0)
        return
    subcmd = args[0]

    # ``info level 0`` — return the current proc's invocation
    # list.  The prologue stashed the real argv via
    # ``frame_set_argv`` (a proper list TclObj built with
    # ``tcl_list``), so the inline shortcut just reads it back
    # with ``frame_get_argv(0)``.  This matches tclsh's
    # semantics exactly: the list's length, element contents,
    # and quoting are all consistent with the invocation site.
    #
    # Falls through to the eval fallback when the required
    # imports aren't available (``frame_get_argv`` is pulled
    # in with the other frame primitives whenever the module
    # defines procs, so this is really a belt-and-braces
    # guard).
    if (
        subcmd == "level"
        and len(args) == 2
        and args[1] == "0"
        and emitter._is_proc
        and emitter._proc_qname is not None
    ):
        get_argv_idx = emitter._shared_imports.get("tcl_frame_get_argv")
        if get_argv_idx is not None:
            # offset 0 = current (topmost) frame's argv.
            emitter._emit_i32_const(0)
            emitter._emit_call(get_argv_idx)
            return
        # Fall through when the import is absent — the generic
        # ``info`` dispatch below will pick it up and route to
        # ``info_dispatch`` at runtime.

    if subcmd == "exists" and len(args) >= 2:
        # ``info exists var`` — resolve the variable reference with
        # compile-time alias info so upvar'd / variable'd / ``::``-
        # qualified names hit the global table rather than searching
        # for an unrelated local.
        var = args[1]
        # ``info exists $dynamicName`` / ``info exists [cmd]`` —
        # the NAME of the variable to check is produced at runtime
        # from a substitution.  Resolve the name value at runtime
        # and dispatch through the runtime ``info_exists`` helper
        # so the check follows the name's actual string, not the
        # literal text after ``$``.
        is_dynamic = (var.startswith("$") or var.startswith("[")) and _parse_array_ref(var) is None
        if is_dynamic:
            info_exists_idx = emitter._shared_imports.get("tcl_info_exists")
            if info_exists_idx is not None:
                emitter._emit_value(var)
                emitter._emit_call(info_exists_idx)
                return
            # Helper missing — fall through to the interpreter so
            # the answer is correct even if less efficient.
            emitter._emit_eval_fallback("info", args)
            return
        # Normalise ``$name`` / ``${name}`` just in case the lowerer
        # hasn't stripped the sigil.
        resolved = emitter._resolve_var_name(var) or var
        # ``info exists arr(key)`` — array element lookup.  The array
        # name itself may be alias-bound (upvar/variable) so resolve
        # it through _emit_array_name_obj, then probe the element
        # table via tcl_array_element_exists.
        array_ref = _parse_array_ref(resolved)
        if array_ref is not None:
            elem_idx = emitter._shared_imports.get("tcl_array_element_exists")
            if elem_idx is not None:
                arr, key = array_ref
                emitter._emit_array_name_obj(arr)
                emitter._emit_value(key)
                emitter._emit_call(elem_idx)
                return
            # Runtime helper missing — conservative false.
            emitter._emit_i64_const(0)
            emitter._emit_box_int()
            return
        gexist_idx = emitter._shared_imports.get("tcl_global_exists")
        aexist_idx = emitter._shared_imports.get("tcl_array_exists")
        binding = emitter._aliases.get(resolved)
        if binding is not None and binding[0] == "frame_var":
            # ``upvar N other local`` — the runtime alias bucket is
            # registered in our frame; ``tcl_info_exists`` (which calls
            # ``local_exists``) follows the alias to the target frame.
            info_exists_idx = emitter._shared_imports.get("tcl_info_exists")
            if info_exists_idx is not None:
                emitter._emit_obj_literal(resolved)
                emitter._emit_call(info_exists_idx)
                return
        if binding is not None and binding[0] == "global":
            _, target_idx = binding
            if aexist_idx is not None and gexist_idx is not None:
                # ``info exists arr`` for an upvar global alias must return 1
                # when *either* the global scalar is set OR the array table
                # exists (pure-array variables have no scalar slot).
                ogi_idx = emitter._shared_imports["tcl_obj_get_int"]
                emitter._emit_local_get(target_idx)
                emitter._emit_call(aexist_idx)
                emitter._emit_call(ogi_idx)  # i64: 0 or 1
                emitter._emit_local_get(target_idx)
                emitter._emit_call(gexist_idx)
                emitter._emit_call(ogi_idx)  # i64: 0 or 1
                emitter._emit(WasmOp.I64_OR)  # i64: 1 if either exists
                emitter._emit_box_int()
                return
            if gexist_idx is not None:
                emitter._emit_local_get(target_idx)
                emitter._emit_call(gexist_idx)
                return
        if resolved.startswith("::") and gexist_idx is not None:
            emitter._emit_obj_literal(resolved)
            emitter._emit_call(gexist_idx)
            return
        # Existence answered against the authoritative global/frame
        # table, NOT the compiled WASM-local mirror, for the two scopes
        # whose mirror goes stale on ``unset``:
        #   * top-level (script-scope) names — an eval-fallback ``unset``
        #     nulls the runtime var without clearing the mirror;
        #   * ``global``-declared names inside a proc — the alias targets
        #     the global table, which a sibling proc (or ``unset``) can
        #     mutate behind the mirror's back.
        # Both mirror the strict read paths in ``_emit_var_read_obj``
        # (``not self._is_proc`` → ``tcl_global_get_or_error``; ``name in
        # self._globals`` → ditto).  ``set x 1; unset x; info exists x``
        # returned 1 because the plain-local path below read the mirror,
        # which ``set x 1`` left non-null and ``unset`` never
        # invalidated.  ``tcl_info_exists`` (``var_exists``) follows the
        # ALIAS_GLOBAL frame bucket / falls through to ``global_exists``,
        # so both scalar and pure-array globals resolve correctly.
        if not emitter._is_proc or resolved in emitter._globals:
            info_exists_idx = emitter._shared_imports.get("tcl_info_exists")
            if info_exists_idx is not None:
                emitter._emit_obj_literal(resolved)
                emitter._emit_call(info_exists_idx)
                return
        # FRAME-only vars (routed through tcl_local_set by the
        # escape-aware writer) don't land in a WASM local, so the
        # pointer-nullness check below would always say "no".
        # Dispatch through the runtime ``tcl_info_exists`` helper,
        # which probes the frame table by name and returns a
        # boxed TclObj 0/1 (matching the other existence paths).
        if emitter._is_frame_only_var(resolved):
            info_exists_idx = emitter._shared_imports.get("tcl_info_exists")
            if info_exists_idx is not None:
                emitter._emit_obj_literal(resolved)
                emitter._emit_call(info_exists_idx)
                return
        # Plain proc-local: the compiled proc uses WASM locals
        # that are zero-initialised, so "exists" is approximated
        # as "value pointer is non-null" — matches writes that
        # land through _emit_var_write_obj.
        idx = emitter._local_index.get(resolved)
        if idx is None:
            # Never referenced through the static-name path — but
            # a callee might still have written this name into the
            # frame via ``uplevel N set <name> ...`` from an inner
            # proc.  Dispatch through the runtime helper which
            # probes the frame table; falls back to a boxed 0 only
            # when the helper is unavailable.
            info_exists_idx = emitter._shared_imports.get("tcl_info_exists")
            if info_exists_idx is not None:
                emitter._emit_obj_literal(resolved)
                emitter._emit_call(info_exists_idx)
                return
            emitter._emit_i64_const(0)
            emitter._emit_box_int()
            return
        emitter._emit_local_get(idx)
        emitter._emit_i32_const(0)
        emitter._emit(WasmOp.I32_NE)
        emitter._emit(WasmOp.I64_EXTEND_I32_S)
        emitter._emit_box_int()
        return

    # Subcommands dispatched via the runtime's info_dispatch helper.
    info_dispatch_idx = emitter._shared_imports.get("tcl_info_dispatch")
    if info_dispatch_idx is not None and subcmd in ("body", "args", "exists"):
        emitter._emit_obj_literal(subcmd)
        if len(args) >= 2:
            emitter._emit_value(args[1])
        else:
            emitter._emit_i32_const(0)
        emitter._emit_call(info_dispatch_idx)
        return

    # Unknown subcommand — fall back to the interpreter.
    emitter._emit_eval_fallback("info", args)


class _CmdInfoMixin:
    """Expose the migrated helpers as methods for MRO composition."""

    _emit_info_value = _emit_info_value
