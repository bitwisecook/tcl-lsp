"""regexp/regsub WASM codegen hook.

The Zig runtime's ``tcl_cmd_regexp`` is a fixed 2-arg fast path
(``regexp PATTERN STRING``) that returns 0/1.  Any other form —
options like ``-nocase`` / ``-inline`` / ``-indices`` / ``-all``,
capture vars after the subject, etc. — has to go through the
interpreter's ``eval_regexp_cmd`` (see ``runtime/zig/valtypes/
tcl_regex.zig``) which has the full Tcl 9 semantics.

This hook routes calls with options or capture vars through the
eval-fallback path; bare two-arg ``regexp PATTERN STRING`` keeps
the inline runtime fast path.
"""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext


def _has_options_or_capture_vars(args: tuple[str, ...]) -> bool:
    # Anything starting with '-' (other than the bare '-' literal) is
    # an option.  Anything past the first 2 positionals is a capture
    # var.  We also re-route the ``--`` end-of-options marker through
    # the eval path because it implies the caller is being explicit
    # about ambiguous option / pattern boundaries — best handled by
    # the full parser.
    n_positional = 0
    for a in args:
        if a.startswith("-") and len(a) > 1:
            return True
        n_positional += 1
        if n_positional > 2:
            return True
    return False


def _hook_regexp(emitter, args, defs, context):
    if _has_options_or_capture_vars(args):
        emitter._emit_eval_fallback("regexp", args)
        if context is EmitContext.STATEMENT:
            from ..._ir import WasmOp
            emitter._emit(WasmOp.DROP)
        return True
    # Two-arg fast path through the runtime import.
    emitter._emit_cmd_runtime("regexp", args, defs, context)
    return True


def _hook_regsub(emitter, args, defs, context):
    if _has_options_or_capture_vars(args):
        emitter._emit_eval_fallback("regsub", args)
        if context is EmitContext.STATEMENT:
            from ..._ir import WasmOp
            emitter._emit(WasmOp.DROP)
        return True
    emitter._emit_cmd_runtime("regsub", args, defs, context)
    return True


REGISTRY.register_wasm_emitter("regexp", _hook_regexp)
REGISTRY.register_wasm_emitter("regsub", _hook_regsub)
