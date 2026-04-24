"""WASM emit hooks for runtime-dispatched built-in commands.

Iterates every spec in ``REGISTRY`` and registers a generic emit hook
for commands that carry a ``CommandSpec.wasm_runtime_import``.  The
hook defers the actual call emission to
``_WasmEmitterCmdMixin._emit_cmd_runtime``, which looks the import up
again via :func:`runtime_import_for`.
"""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext


def _make_runtime_hook(cmd: str):
    def _hook(emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext) -> bool:
        if context is EmitContext.VALUE:
            # Tail-context runtime commands are served by the generic
            # dispatch in ``_emit_call_stmt_tail`` (which keeps the
            # result on the operand stack), not here.
            return False
        emitter._emit_cmd_runtime(cmd, args, defs)
        return True

    _hook.__name__ = f"_hook_{cmd}"
    return _hook


for _name, _specs in REGISTRY.specs_by_name.items():
    if any(spec.wasm_runtime_import is not None for spec in _specs):
        REGISTRY.register_wasm_emitter(_name, _make_runtime_hook(_name))
