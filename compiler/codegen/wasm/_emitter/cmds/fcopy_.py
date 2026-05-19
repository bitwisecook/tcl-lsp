"""WASM emit hook for ``fcopy`` — fall through on option-bearing forms.

The generic runtime-dispatch path for ``fcopy`` emits a 2-arg call
to ``tcl_cmd_fcopy(inputChan, outputChan)`` and silently drops any
trailing ``-size N`` / ``-command cb`` / unknown options because
the import only declares two parameters.

When the call site has option arguments (anything past the two
channel ids) we fall through to the interpreter's ``eval_fcopy``
shim in :file:`runtime/zig/cmds/io.zig`.  That shim validates the
option list (raises on bad option / non-integer size / `-command`)
and applies ``-size`` via ``fcopy_limited``.

For the bare 2-arg form we emit the direct runtime call ourselves
since registering this hook removes the generic auto-registration.
"""

from __future__ import annotations

from core.commands.registry import REGISTRY, EmitContext


def _emit_fcopy(
    emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
) -> bool:
    if len(args) > 2:
        # Option-bearing form — let the eval-fallback path handle it
        # so option validation runs.
        return False
    # No options: dispatch the 2-arg runtime call directly.
    emitter._emit_cmd_runtime("fcopy", args, defs, context)
    return True


REGISTRY.register_wasm_emitter("fcopy", _emit_fcopy)
