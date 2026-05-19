"""WASM emit hook for ``string``."""

from __future__ import annotations

from compiler.registry import REGISTRY, EmitContext

from ..._imports import (
    _STRING_IS_IMPORT,
    import_signature,
    subcommand_min_arity_for,
    subcommand_runtime_import_for,
)
from ..._ir import WasmOp


def _looks_like_option(arg: str) -> bool:
    """Return True if *arg* looks like a sub-command option (``-foo``).

    Used to detect when ``string map -nocase MAP STR`` /
    ``string match -nocase PAT STR`` / ``string compare -nocase A B``
    needs to bypass the fixed-param-count fast path (which would
    silently pass ``-nocase`` as the first positional argument).
    """
    if not arg or len(arg) < 2 or arg[0] != "-":
        return False
    # ``-`` alone or ``--`` is not an option flag in our fast path.
    if arg in ("-", "--"):
        return False
    # ``-1`` / ``-0.5`` etc. are negative numbers, not options.
    second = arg[1]
    return not (second.isdigit() or second == ".")


def _was_braced(emitter, call_arg_idx: int) -> bool:
    """Probe ``_current_call_tokens`` for whether the call-site arg at
    *call_arg_idx* (0-indexed into the original ``args``, which excludes
    the command name) came from a ``{…}`` STR token.

    ``string match {\\?*} $name`` only works correctly when the IR
    value ``\\?*`` is emitted verbatim — without the probe, ``_emit_value``
    falls into the backslash-subst branch and turns the pattern into
    ``?*``, matching anything starting with any single char.  That
    surfaces as ``OptIsOpt cmd`` returning 1 instead of 0 and is the
    root cause of the opt-10.5..10 / 11.1 / 3.1 silent successes.
    """
    from compiler.parsing.tokens import TokenType

    tokens = getattr(emitter, "_current_call_tokens", None)
    if tokens is None or tokens.argv is None:
        return False
    tok_idx = call_arg_idx + 1
    if tok_idx >= len(tokens.argv):
        return False
    return tokens.argv[tok_idx].type == TokenType.STR


def _emit_string(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``string subcommand ...`` — dispatch to runtime import (i32 args)."""
    if context is EmitContext.VALUE:
        # Tail / implicit-return: the result must stay on the operand
        # stack.  Runtime imports that return ``i32`` leave one there;
        # void-result imports get an ``i32.const 0`` pushed to fill in
        # an empty-string result.  Unknown sub-commands / missing
        # imports fall through to the eval-fallback path so the caller
        # sees a real ``bad subcommand`` / ``unknown command`` error
        # rather than a silent empty result.
        if args:
            subcmd = args[0]
            # Fast-path ``string is <class> <value>`` only when there
            # are no intervening flags.  ``string is alpha -strict
            # {}`` must route through the eval-fallback so the
            # ``-strict`` flag is honoured (the fast-path imports
            # don't know about flags).
            if subcmd == "is" and len(args) == 3:
                is_key = _STRING_IS_IMPORT.get(args[1])
                if is_key is not None and is_key in emitter._shared_imports:
                    func_idx = emitter._shared_imports[is_key]
                    emitter._emit_value(args[-1])
                    emitter._emit_call(func_idx)
                    return True
            sri = subcommand_runtime_import_for("string", subcmd)
            if sri is not None and sri.import_key in emitter._shared_imports:
                sub_args = args[1:]
                # Under-arity literal calls (``string index`` with no
                # operand, ``string range`` with one, …) must fall
                # through to the eval-fallback.  The static path zero-
                # pads missing slots — ``tcl_string_index(0,0)`` then
                # silently returns the empty string and an enclosing
                # ``catch`` never sees the wrong-args error that the
                # runtime dispatcher would have raised.  Triggered by
                # error-1.3 / 1.6 / 1.7 / cmdAH-1.4 / 1.5 / list-1.x.
                min_arity = subcommand_min_arity_for("string", subcmd)
                if min_arity is not None and len(sub_args) < min_arity:
                    emitter._emit_eval_fallback("string", args)
                    return True
                # Option-bearing forms (``string map -nocase``,
                # ``string match -nocase``, ``string compare -nocase``)
                # have more sub-args than the fixed-param runtime import
                # accepts; fall through to eval so the runtime
                # dispatcher in ``cmds/string.zig`` parses options
                # correctly.  Without this, the leading ``-nocase``
                # was passed as the first positional argument and
                # silently corrupted the result.
                if sub_args and _looks_like_option(sub_args[0]):
                    emitter._emit_eval_fallback("string", args)
                    return True
                func_idx = emitter._shared_imports[sri.import_key]
                param_count = len(sri.params)
                for i in range(min(param_count, len(sub_args))):
                    # ``i`` is into ``sub_args`` (= ``args[1:]``), so the
                    # original call-arg index is ``i + 1`` (the
                    # subcommand at args[0] occupies the first slot).
                    emitter._emit_value(sub_args[i], was_braced=_was_braced(emitter, i + 1))
                for _ in range(param_count - len(sub_args)):
                    emitter._emit_i32_const(0)
                emitter._emit_call(func_idx)
                if not sri.results:
                    emitter._emit_i32_const(0)
                return True
        # Unknown subcommand (or no subcommand at all).  Delegate to
        # the interpreter so it raises Tcl's native ``bad subcommand``
        # error and the tail dispatcher's value-stack expectation is
        # satisfied by ``tcl_eval``'s i32 result.
        emitter._emit_eval_fallback("string", args)
        return True
    if not args:
        emitter._emit_unsupported_trap("string (no subcommand)")
        return True
    subcmd = args[0]

    if subcmd == "cat":
        sub_args = args[1:]
        if not sub_args:
            emitter._emit_obj_literal("")
        elif all(
            not a.startswith("$")
            and not a.startswith("[")
            and not emitter._has_embedded_subst(a)
            and a not in emitter._aliases
            and a not in emitter._local_index
            for a in sub_args
        ):
            emitter._emit_obj_literal("".join(sub_args))
        else:
            append_idx = emitter._shared_imports.get("tcl_append")
            if append_idx is None:
                emitter._emit_eval_fallback("string", args)
                if defs:
                    def_idx = emitter._intern_local(defs[0])
                    emitter._emit_local_set(def_idx)
                else:
                    emitter._emit(WasmOp.DROP)
                return True
            emitter._emit_value(sub_args[0])
            for rest in sub_args[1:]:
                emitter._emit_value(rest)
                emitter._emit_call(append_idx)
        if defs:
            def_idx = emitter._intern_local(defs[0])
            emitter._emit_local_set(def_idx)
        else:
            emitter._emit(WasmOp.DROP)
        return True

    if subcmd == "is" and len(args) == 3:
        class_name = args[1]
        is_key = _STRING_IS_IMPORT.get(class_name)
        if is_key is not None and is_key in emitter._shared_imports:
            func_idx = emitter._shared_imports[is_key]
            sig = import_signature(is_key)
            val_arg = args[-1]
            emitter._emit_value(val_arg)
            emitter._emit_call(func_idx)
            if defs:
                def_idx = emitter._intern_local(defs[0])
                emitter._emit_local_set(def_idx)
            elif sig is not None and sig[3]:
                emitter._emit(WasmOp.DROP)
            return True

    sri = subcommand_runtime_import_for("string", subcmd)
    if sri is not None and sri.import_key in emitter._shared_imports:
        sub_args = args[1:]
        # Same arity short-circuit as the value-context path above:
        # under-arity literal calls must surface the runtime
        # dispatcher's wrong-args wording, not silently resolve to
        # the zero-padded fast path.  See note on the value-context
        # branch for the failing tests this unblocks.
        min_arity = subcommand_min_arity_for("string", subcmd)
        if min_arity is not None and len(sub_args) < min_arity:
            emitter._emit_eval_fallback("string", args)
            if defs:
                def_idx = emitter._intern_local(defs[0])
                emitter._emit_local_set(def_idx)
            else:
                emitter._emit(WasmOp.DROP)
            return True
        # Same option detection as the value-context path above —
        # ``string map -nocase ...`` etc. need eval-fallback so the
        # runtime dispatcher parses the option bytes correctly.
        if sub_args and _looks_like_option(sub_args[0]):
            emitter._emit_eval_fallback("string", args)
            if defs:
                def_idx = emitter._intern_local(defs[0])
                emitter._emit_local_set(def_idx)
            else:
                emitter._emit(WasmOp.DROP)
            return True
        func_idx = emitter._shared_imports[sri.import_key]
        param_count = len(sri.params)
        for i in range(min(param_count, len(sub_args))):
            emitter._emit_value(sub_args[i], was_braced=_was_braced(emitter, i + 1))
        for _ in range(param_count - len(sub_args)):
            emitter._emit_i32_const(0)
        emitter._emit_call(func_idx)
        if defs:
            def_idx = emitter._intern_local(defs[0])
            emitter._emit_local_set(def_idx)
        elif sri.results:
            emitter._emit(WasmOp.DROP)
    else:
        emitter._emit_unsupported_trap(f"string {subcmd}")
    return True


REGISTRY.register_wasm_emitter("string", _emit_string)
