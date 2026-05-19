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

from compiler.registry import REGISTRY, EmitContext


def _split_options_and_positionals(
    args: tuple[str, ...],
) -> tuple[list[str], list[str]]:
    """Separate leading ``-foo`` options from the trailing positional words.

    ``-start`` consumes a following value; ``--`` ends option parsing.
    Numbers and bare ``-`` aren't options.
    """
    options: list[str] = []
    positionals: list[str] = []
    i = 0
    in_options = True
    while i < len(args):
        a = args[i]
        if in_options and a.startswith("-") and len(a) > 1 and not (a[1].isdigit() or a[1] == "."):
            if a == "--":
                in_options = False
                i += 1
                continue
            options.append(a)
            if a == "-start" and i + 1 < len(args):
                options.append(args[i + 1])
                i += 2
                continue
            i += 1
            continue
        in_options = False
        positionals.append(a)
        i += 1
    return options, positionals


def _capture_vars_for(cmd: str, args: tuple[str, ...]) -> list[str]:
    """Identify capture-target var names so the proc's readback can sync them.

    For ``regexp ?opts? PAT STR ?varname ...?`` — every positional after
    PAT/STR.
    For ``regsub ?opts? PAT STR REPL ?varname?`` — the optional varname.
    Returns names for which the capture write happens via the
    interpreter's ``frames.var_set``; without pre-interning these,
    ``_emit_frame_readback`` skips them and the compiled body keeps
    reading the stale wasm-local.
    """
    options, positionals = _split_options_and_positionals(args)
    inline_mode = "-inline" in options
    if cmd == "regexp":
        # PAT, SUBJECT, then capture vars (skipped under -inline).
        if inline_mode or len(positionals) <= 2:
            return []
        return [p for p in positionals[2:] if p and not p.startswith("$")]
    # regsub
    if len(positionals) <= 3:
        return []
    return [p for p in positionals[3:] if p and not p.startswith("$")]


def _hook_regexp(emitter, args, defs, context):
    # Route through ``eval_regexp_cmd`` whenever options / capture
    # vars are present OR the arg count is too low for the fast
    # path's strict ``regexp PAT STR`` signature.  The fast path
    # silently returns 0 for malformed calls; the eval path raises
    # Tcl 9's ``wrong # args: should be "regexp ?-option ...?
    # exp string ..."`` so regexp.test 6.1 / 6.2 see the correct
    # error wording.
    if _has_options_or_capture_vars(args) or len(args) < 2:
        # Pre-intern any capture-var names so the proc's frame readback
        # after eval-fallback reloads them into the wasm-local cache.
        # Without this, ``regexp -indices PAT STR all`` inside a compiled
        # proc body sets ``all`` in the runtime frame but the next
        # ``$all`` read sees the stale (empty) wasm-local.
        for vname in _capture_vars_for("regexp", args):
            emitter._intern_local(vname)
        emitter._emit_eval_fallback("regexp", args)
        if context is EmitContext.STATEMENT:
            from ..._ir import WasmOp

            emitter._emit(WasmOp.DROP)
        return True
    emitter._emit_cmd_runtime("regexp", args, defs, context)
    return True


def _hook_regsub(emitter, args, defs, context):
    # As above: route to eval-path on too-few / too-many args so the
    # ``wrong # args`` wording reaches the user (regexp.test 11.1-11.4).
    if _has_options_or_capture_vars(args) or len(args) < 3 or len(args) > 4:
        for vname in _capture_vars_for("regsub", args):
            emitter._intern_local(vname)
        emitter._emit_eval_fallback("regsub", args)
        if context is EmitContext.STATEMENT:
            from ..._ir import WasmOp

            emitter._emit(WasmOp.DROP)
        return True
    emitter._emit_cmd_runtime("regsub", args, defs, context)
    return True


def _has_options_or_capture_vars(args: tuple[str, ...]) -> bool:
    n_positional = 0
    for a in args:
        if a.startswith("-") and len(a) > 1:
            return True
        n_positional += 1
        if n_positional > 2:
            return True
    return False


REGISTRY.register_wasm_emitter("regexp", _hook_regexp)
REGISTRY.register_wasm_emitter("regsub", _hook_regsub)
