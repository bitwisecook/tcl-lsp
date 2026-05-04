"""WASM emit hook for ``catch``."""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext


def _emit_catch(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``catch ?body? ?resultVar?`` — re-parse body and emit with error flag.

    Statement context drops the 0/1 return code; tail position keeps it on
    the operand stack for implicit return.

    AOT-compiles any static body: braced literals (``{...}``), bare words
    (e.g. ``catch foo msg``), and plain scripts without leading ``$`` or
    ``[``.  Dynamic bodies (variable references ``$cmd``, bracket commands
    ``[expr ...]``) cannot be statically compiled because the body text is
    only known at runtime — returning False routes those through eval_fallback
    which calls ``eval_catch`` on the substituted value.

    The raw_args from CFG's IRCatch conversion are brace-stripped (the
    quoting delimiters are part of Tcl syntax, not the value), so we cannot
    use a brace check to detect static bodies.  Instead we reject only
    clearly-dynamic first args.
    """
    if not args:
        return False
    body = args[0].strip()
    # Strip outer braces if present (some call sites preserve them).
    if body.startswith("{") and body.endswith("}"):
        body_inner = body[1:-1].strip()
    else:
        body_inner = body
    # Reject bodies that are runtime-dynamic: variable substitutions
    # ($cmd) and bracket commands ([get_script]) can only be evaluated
    # by the runtime.  Bare words and braced scripts are static.
    if body_inner.startswith("$") or body_inner.startswith("["):
        return False
    keep = context is EmitContext.VALUE
    emitter._emit_catch_from_args(args, defs, keep_on_stack=keep)
    return True


REGISTRY.register_wasm_emitter("catch", _emit_catch)
