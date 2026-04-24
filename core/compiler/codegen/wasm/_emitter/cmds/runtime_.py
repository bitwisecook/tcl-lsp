"""WASM emit hooks for all ``_CMD_RUNTIME`` built-in commands."""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext
from ..._imports import _CMD_RUNTIME


def _make_runtime_hook(cmd: str):
    def _hook(
        emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext
    ) -> bool:
        emitter._emit_cmd_runtime(cmd, args, defs)
        return True

    _hook.__name__ = f"_hook_{cmd}"
    return _hook


for _cmd in _CMD_RUNTIME:
    REGISTRY.register_wasm_emitter(_cmd, _make_runtime_hook(_cmd))
