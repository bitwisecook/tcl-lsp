"""WASM emit hooks for ``uplevel``, ``clock``, ``array``, and ``unset``."""

from __future__ import annotations

from core.commands.registry import REGISTRY, EmitContext

from ..._ir import ValType, WasmOp
from ..._parsing import _parse_array_ref


def _emit_uplevel(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``uplevel ?level? body`` — delegates to _emit_cmd_uplevel."""
    if not args:
        return False
    if context is EmitContext.VALUE:
        emitter._emit_cmd_uplevel(args)
        return True
    emitter._emit_cmd_uplevel(args)
    if defs:
        def_idx = emitter._intern_local(defs[0])
        emitter._emit_local_set(def_idx)
    else:
        emitter._emit(WasmOp.DROP)
    return True


def _emit_clock(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``clock subcommand ...`` — delegates to _emit_clock_value."""
    if not args:
        return False
    if context is EmitContext.VALUE:
        emitter._emit_clock_value(args)
        return True
    emitter._emit_clock_value(args)
    if defs:
        def_idx = emitter._intern_local(defs[0])
        emitter._emit_local_set(def_idx)
    else:
        emitter._emit(WasmOp.DROP)
    return True


def _emit_array(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``array subcommand ...`` — delegates to _emit_array_subcmd_value."""
    if not args:
        return False
    if context is EmitContext.VALUE:
        emitter._emit_array_subcmd_value(args)
        return True
    emitter._emit_array_subcmd_value(args)
    if defs:
        def_idx = emitter._intern_local(defs[0])
        emitter._emit_local_set(def_idx)
    else:
        emitter._emit(WasmOp.DROP)
    return True


def _emit_unset(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``unset ?-nocomplain? ?--? varName ...`` — array-element unset.

    Handles statement and value (tail / implicit return) contexts;
    in value context an empty string TclObj is pushed after the
    unset so the proc's tail return slot stays well-typed.  Without
    this, a proc whose sole tail statement is ``unset arr($k)``
    would route through the eval-fallback (which lacks the
    compile-time namespace alias from ``variable``) and miss the
    unset entirely — opt.test ``OptKeyDelete`` (one-statement
    body) regressed exactly this way before the value-context
    branch landed.
    """
    if not args:
        return False
    handled = bool(emitter._emit_unset_array_elems(args))
    if not handled:
        return False
    if context is EmitContext.VALUE:
        # ``unset`` returns the empty string; push it for the tail
        # value slot.
        emitter._emit_obj_literal("")
    return True


REGISTRY.register_wasm_emitter("uplevel", _emit_uplevel)
REGISTRY.register_wasm_emitter("clock", _emit_clock)
REGISTRY.register_wasm_emitter("array", _emit_array)
REGISTRY.register_wasm_emitter("unset", _emit_unset)


def _emit_unset_array_elems(emitter, args: tuple[str, ...]) -> bool:
    """Handle ``unset`` for array-element and whole-variable forms.

    For ``unset arr(key)``: emits ``tcl_array_unset_element``.
    For ``unset arr`` when ``arr`` is an upvar global alias: emits
    ``array_unset(target)`` + ``global_set(target, 0)`` so that the
    entire array table is cleared in addition to nulling the global.
    For ``unset arr`` where ``arr`` is not an alias: returns False so
    the caller falls through to the eval fallback.

    Returns True if at least one unset was emitted; False otherwise.
    """
    had_any = False
    elem_fidx = emitter._shared_imports.get("tcl_array_unset_element")
    arr_fidx = emitter._shared_imports.get("tcl_array_unset")
    gset_fidx = emitter._shared_imports.get("tcl_global_set")
    # Skip ``-nocomplain`` / ``--`` option prefix.
    i = 0
    while i < len(args) and args[i].startswith("-"):
        if args[i] == "--":
            i += 1
            break
        i += 1
    for name in args[i:]:
        ref = _parse_array_ref(name)
        if ref is not None:
            # Array element: unset arr(key)
            had_any = True
            arr, key = ref
            if elem_fidx is None:
                continue
            emitter._emit_array_name_obj(arr)
            emitter._emit_value(key)
            emitter._emit_call(elem_fidx)
            emitter._emit(WasmOp.DROP)
            continue
        # Whole-variable unset: ``unset var``
        # Only handle when var is a global alias (upvar #0) — we know
        # the target name at compile time and can clear both the array
        # table and the global slot.  Scalars and frame locals fall
        # through to the eval fallback.
        binding = emitter._aliases.get(name)
        if binding is not None and binding[0] == "global":
            had_any = True
            target_idx = binding[1]
            if arr_fidx is not None:
                emitter._emit_local_get(target_idx)
                emitter._emit_call(arr_fidx)
                emitter._emit(WasmOp.DROP)
            if gset_fidx is not None:
                emitter._emit_local_get(target_idx)
                emitter._emit_i32_const(0)
                emitter._emit_call(gset_fidx)
                emitter._emit(WasmOp.DROP)
    return had_any


def _emit_cmd_uplevel(emitter, args: tuple[str, ...]) -> None:
    """``uplevel ?level? body`` — evaluate script in a caller's frame.

    ``level`` defaults to ``1``.  ``#0`` means absolute global
    scope; ``#N`` means N frames above global (Tcl only really
    uses ``#0``); a bare integer N means N frames up relative to
    the current frame.

    We emit:
        saved = frame_depth_stash(up)
        result = tcl_eval(body)
        frame_depth_restore(saved)

    ``frame_depth_stash`` clamps the shift at frame 0 so invalid
    levels degrade to global-scope eval rather than trapping.  If
    the caller chain is entirely compiled (no frames pushed), this
    is effectively a no-op and the eval runs at global scope — the
    ``#0`` behaviour Tcl scripts most commonly use.

    Multiple bodies concatenated as separate words (``uplevel 1 a b c``)
    are joined with spaces to form the script, matching Tcl's
    semantics of "concat all remaining arguments into a single
    script".
    """
    if not args:
        emitter._emit_i32_const(0)
        return
    # Parse the level specifier.  ``#N`` → absolute level N; a
    # bare integer N → relative N frames up; anything else means
    # no level, default 1, and args[0] is the first body word.
    level_spec = args[0]
    up: int
    body_start = 1
    use_abs = False
    abs_level: int = 0
    if level_spec.startswith("#"):
        try:
            abs_level = int(level_spec[1:])
            use_abs = True
            up = 0
        except ValueError:
            up = 1
    else:
        try:
            up = int(level_spec)
            # Numeric → relative.
        except ValueError:
            # Not a level — arg0 is part of the body.
            up = 1
            body_start = 0
    if body_start >= len(args):
        emitter._emit_i32_const(0)
        return

    body_parts = list(args[body_start:])

    eval_idx = emitter._shared_imports.get("tcl_eval")
    stash_idx = emitter._shared_imports.get("tcl_frame_depth_stash")
    stash_abs_idx = emitter._shared_imports.get("tcl_frame_depth_stash_abs")
    restore_idx = emitter._shared_imports.get("tcl_frame_depth_restore")
    if eval_idx is None or stash_idx is None or restore_idx is None:
        # Missing runtime helpers — fall back to plain eval, which
        # still works for compiled-to-compiled chains where
        # frame_depth is 0 throughout.
        emitter._emit_uplevel_body(body_parts)
        if eval_idx is not None:
            emitter._emit_call(eval_idx)
        else:
            emitter._emit_i32_const(0)
        return

    saved_local = emitter._add_extra_local(prefix="_uplevel_saved", val_type=ValType.I32)
    body_local = emitter._add_extra_local(prefix="_uplevel_body", val_type=ValType.I32)

    # Resolve the body string while still in the current frame so that
    # any $var/$[cmd] substitutions read from the correct (callee) frame,
    # not the stashed-to caller frame.
    emitter._emit_uplevel_body(body_parts)
    emitter._emit_local_set(body_local)

    # Choose the right stash variant.  ``#N`` is an *absolute*
    # level — only resolvable at runtime (relative shift depends on
    # the current call depth).  ``frame_depth_stash_abs`` does the
    # ``frame_depth - abs_level`` arithmetic on the runtime side
    # and routes through the relative path.  Bare integer ``N``
    # uses the simple relative stash.
    if use_abs and stash_abs_idx is not None:
        emitter._emit_i32_const(abs_level)
        emitter._emit_call(stash_abs_idx)
    else:
        # Fallback: ``#0`` gets the legacy "shift to zero" hack
        # via INT32_MAX/2; other ``#N`` values without the abs
        # helper degrade to relative-1 which matches the previous
        # behaviour.
        shift = 0x3FFF_FFFF if level_spec == "#0" else up
        emitter._emit_i32_const(shift)
        emitter._emit_call(stash_idx)
    emitter._emit_local_set(saved_local)

    emitter._emit_local_get(body_local)
    emitter._emit_call(eval_idx)
    # Result TclObj is on stack; stash temporarily so we can restore.
    result_local = emitter._add_extra_local(prefix="_uplevel_result", val_type=ValType.I32)
    emitter._emit_local_set(result_local)

    emitter._emit_local_get(saved_local)
    emitter._emit_call(restore_idx)

    emitter._emit_local_get(result_local)


def _emit_uplevel_body(emitter, parts: list[str]) -> None:
    """Push the uplevel body as a single TclObj, resolving any
    ``$var``/``[cmd]`` substitutions the parts contain before
    handing the final string to ``tcl_eval``.

    Typical cases:
      - ``uplevel #0 {set ::g 42}``  — parts = ["set ::g 42"], a
        literal script.  Emitted as an obj_literal.
      - ``uplevel "set ::$name $val"`` — parts = ["set ::$name $val"],
        an interpolated string.  Emitted via ``_emit_value`` so the
        concat chain resolves ``$name``/``$val`` before eval.
      - ``uplevel 1 a b c`` — parts = ["a","b","c"], Tcl concat-join
        with spaces.  Each part is resolved and joined.
    """
    if not parts:
        emitter._emit_obj_literal("")
        return
    if len(parts) == 1:
        emitter._emit_value(parts[0])
        return
    # Multi-part: emit each value obj and concat with spaces.
    # Simplest: use _emit_value for each and chain tcl_append.
    append_idx = emitter._shared_imports.get("tcl_append")
    if append_idx is None:
        # No concat helper — best-effort: fall back to the first part
        # (typical callers supply a single braced script).
        emitter._emit_value(parts[0])
        return
    emitter._emit_value(parts[0])
    for part in parts[1:]:
        emitter._emit_obj_literal(" ")
        emitter._emit_call(append_idx)
        emitter._emit_value(part)
        emitter._emit_call(append_idx)


class _CmdUplevelMixin:
    """Expose the migrated helpers as methods for MRO composition."""

    _emit_unset_array_elems = _emit_unset_array_elems
    _emit_cmd_uplevel = _emit_cmd_uplevel
    _emit_uplevel_body = _emit_uplevel_body
