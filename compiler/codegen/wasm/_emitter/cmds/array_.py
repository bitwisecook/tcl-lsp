"""Command helpers for ``array`` — subcmd value emit + ``array set`` literal folding."""

from __future__ import annotations

from ..._ir import WasmOp


def _emit_array_subcmd_value(emitter, args: tuple[str, ...]) -> None:
    """``array <subcmd> <arr> ?args?`` — leaves i32 TclObj on the stack.

    Supported subcommands:
      exists, size, unset, names, set, get.  Others fall back to
      the interpreter, which will see the compile-time snapshot
      of the array's state (via globals — interpreter doesn't
      touch per-array tables yet).
    """
    if not args:
        emitter._emit_i32_const(0)
        return
    subcmd = args[0]
    if subcmd == "exists" and len(args) >= 2:
        fidx = emitter._shared_imports.get("tcl_array_exists")
        if fidx is not None:
            emitter._emit_array_name_obj(args[1])
            emitter._emit_call(fidx)
            return
    elif subcmd == "size" and len(args) >= 2:
        fidx = emitter._shared_imports.get("tcl_array_size")
        if fidx is not None:
            emitter._emit_array_name_obj(args[1])
            emitter._emit_call(fidx)
            return
    elif subcmd == "unset" and len(args) >= 2:
        fidx = emitter._shared_imports.get("tcl_array_unset")
        if fidx is not None:
            emitter._emit_array_name_obj(args[1])
            emitter._emit_call(fidx)
            return
    elif subcmd == "names" and len(args) >= 2:
        fidx = emitter._shared_imports.get("tcl_array_names")
        if fidx is not None:
            emitter._emit_array_name_obj(args[1])
            # Optional glob pattern — ``array names arr`` → no
            # filter (null TclObj), ``array names arr pat`` →
            # use the supplied pattern.  ``-exact`` / ``-glob``
            # / ``-regexp`` modes fall through to the fallback
            # for now; the common case scripts use is positional.
            if len(args) >= 3:
                emitter._emit_value(args[2])
            else:
                emitter._emit_i32_const(0)
            emitter._emit_call(fidx)
            return
    elif subcmd == "set" and len(args) >= 3:
        # ``array set arr {key val key val ...}`` — iterate the list
        # literal at compile time when possible, otherwise fall back
        # to the interpreter.  Most real-world usage is literal.
        emitter._emit_array_set_list(args[1], args[2])
        return
    elif subcmd == "get" and len(args) >= 2:
        # ``array get arr ?pattern?`` — return a flat ``{k v k v
        # …}`` list, optionally glob-filtered.  Routed through
        # ``tcl_array_get_all`` so the array-name resolution uses
        # the same compile-time-emitted name path as the
        # ``array set`` / ``array names`` / ``array exists``
        # siblings.  Without this, ``array get`` fell into the
        # eval-fallback whose ``frame_resolve_array_name`` builds
        # a synthetic ``::__local::<depth>::arr`` lookup key — but
        # the AOT writer side stores under the bare unqualified
        # name, so the two halves of the read look at different
        # directory entries and ``[array get arr]`` always came
        # back empty inside a proc.
        fidx = emitter._shared_imports.get("tcl_array_get_all")
        if fidx is not None:
            emitter._emit_array_name_obj(args[1])
            if len(args) >= 3:
                emitter._emit_value(args[2])
            else:
                emitter._emit_i32_const(0)
            emitter._emit_call(fidx)
            return
    emitter._emit_eval_fallback("array", args)


def _emit_array_set_list(emitter, arr: str, kv_text: str) -> None:
    """``array set arr {k v k v ...}`` — compile-time list literal.

    Parses the inline list and emits one ``array_set`` call per
    pair.  Falls back to the interpreter for non-literal inputs.
    Leaves an empty string TclObj on the stack as the command's
    return value.
    """
    from ...._helpers import _split_list_simple

    fidx = emitter._shared_imports.get("tcl_array_set")
    if fidx is None:
        emitter._emit_eval_fallback("array", ("set", arr, kv_text))
        return
    # Strip an optional outer braces around the literal list.
    text = kv_text
    if text.startswith("{") and text.endswith("}"):
        text = text[1:-1]
    try:
        words = _split_list_simple(text)
    except Exception:
        emitter._emit_eval_fallback("array", ("set", arr, kv_text))
        return
    if len(words) & 1:
        emitter._emit_eval_fallback("array", ("set", arr, kv_text))
        return
    for i in range(0, len(words), 2):
        emitter._emit_array_name_obj(arr)
        emitter._emit_value(words[i])
        emitter._emit_value(words[i + 1])
        emitter._emit_call(fidx)
        emitter._emit(WasmOp.DROP)
    # ``array set`` returns empty string.
    emitter._emit_obj_literal("")


class _CmdArrayMixin:
    """Expose the migrated helpers as methods for MRO composition."""

    _emit_array_subcmd_value = _emit_array_subcmd_value
    _emit_array_set_list = _emit_array_set_list
