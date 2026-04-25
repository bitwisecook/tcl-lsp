"""regexp/regsub WASM codegen hook.

The Zig runtime's ``tcl_cmd_regexp`` is a fixed 2-arg fast path
(``regexp PATTERN STRING`` returning 0/1).  Anything else — options
like ``-nocase`` / ``-inline`` / ``-indices`` / ``-all``, capture
vars after the subject, etc. — needs the interpreter's
``eval_regexp_cmd`` (see ``runtime/zig/valtypes/tcl_regex.zig``)
which has the full Tcl 9 semantics.

Routing: this hook detects option-bearing or capture-var-bearing
calls and falls through to the eval path; bare two-arg
``regexp PAT STR`` keeps the inline fast path.

Known limit (separate work):
- For statements like ``regexp {(\\w+)} "hi" m`` *at compiled top
  level*, ``m`` is set in the Tcl global table via the runtime's
  ``var_set`` but the compiled-side reader uses a WASM-local
  cache that doesn't refresh after eval-fallback.  So ``puts $m``
  immediately after may print empty.  Inside interpreted scripts
  (anything reached through ``tcl_eval`` — tcltest's bundled
  procs, ``[eval $script]`` blocks, ``proc`` bodies that hit the
  eval-fallback for any reason) the capture-var assignments
  surface correctly because reads in those contexts go through
  ``var_resolve`` which hits the global table.  See
  ``docs/design/runtime/zig-runtime-roadmap.md`` Phase 4.5
  follow-up.
"""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext


def _has_options_or_capture_vars(args: tuple[str, ...]) -> bool:
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
