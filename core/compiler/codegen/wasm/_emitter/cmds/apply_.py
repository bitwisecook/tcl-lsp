"""WASM emit hook for ``apply``.

The 2-arg ``tcl_cmd_apply(lambda, args)`` runtime export expects its
``args`` argument to be a *Tcl list* of every positional argument
the user passed — it list-parses ``args`` to recover individual
``words[2..]`` for the lambda.

The generic ``_emit_cmd_runtime`` path passes the call-site arg
verbatim, which is fine for a single bare-word arg
(``apply $f hello``) but mis-fires for any whitespace-bearing arg:

  apply {d {return $d}} {a 1 c 2}

is emitted as ``tcl_cmd_apply(lambda, "a 1 c 2")``.  The runtime
list-parses ``"a 1 c 2"`` into four elements and binds only the
first to ``d``.  Hence the lambda saw ``d == "a"``.

Fix: build a 1-or-more-element Tcl list TclObj from the call-site
args (re-using ``_emit_args_list``, which already handles the
literal/dynamic split) and pass that.
"""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext
from ..._ir import WasmOp


def _is_brace_quoted_lambda(value: str) -> bool:
    """True when *value* looks like ``{paramList} {body} ?{ns}?`` — the
    canonical apply-lambda shape with each component as a braced word.

    This is the IR shape the lifter produces when the source word was
    a single brace-quoted lambda like ``{{x} {puts $x}}`` — the IR
    strips the outer ``{`` / ``}`` from the word, leaving the inner
    ``{x} {body}`` (multi-pair braces) intact.  ``_emit_value``'s
    generic ``startswith('{') and endswith('}')`` check would strip
    the leading-pair braces incorrectly (``{x} {body}`` becomes
    ``x} {body``) and the runtime ``list_count_elements`` would see
    a malformed list, so we bypass ``_emit_value`` and emit the
    lambda verbatim through ``_emit_obj_literal``.

    The check walks the brace nesting from the leading ``{``: if the
    matching ``}`` is the *last* character we have a single braced
    word and ``_emit_value`` handles it correctly, but if the closing
    ``}`` arrives before the end the value carries 2+ braced words
    glued by whitespace and we must short-circuit.
    """
    if not value or value[0] != "{":
        return False
    depth = 1
    i = 1
    n = len(value)
    while i < n:
        ch = value[i]
        if ch == "\\" and i + 1 < n:
            i += 2
            continue
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
            if depth == 0:
                return i < n - 1  # closing brace before the last char
        i += 1
    return False


def _emit_apply(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    """``apply lambda ?arg ...?`` — pack args into a Tcl list."""
    if not args:
        return False  # let the eval fallback raise the ``wrong # args`` error
    func_idx = emitter._shared_imports.get("tcl_apply")
    if func_idx is None:
        return False
    # First arg is the lambda; remaining are positional args.
    # When the IR captured a brace-quoted lambda (``{paramList}
    # {body}``) as a single word, ``_emit_value``'s generic
    # brace-strip would mangle it — emit the bytes verbatim instead.
    if _is_brace_quoted_lambda(args[0]):
        emitter._emit_obj_literal(args[0])
    else:
        emitter._emit_value(args[0])
    emitter._emit_args_list(tuple(args[1:]))
    emitter._emit_call(func_idx)
    if context is EmitContext.VALUE:
        return True
    if defs:
        def_idx = emitter._intern_local(defs[0])
        emitter._emit_local_set(def_idx)
    else:
        emitter._emit(WasmOp.DROP)
    return True


REGISTRY.register_wasm_emitter("apply", _emit_apply)
