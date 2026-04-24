"""WASM emit hooks for runtime-dispatched built-in commands.

Iterates every spec in ``REGISTRY`` and registers a generic emit hook
for commands that carry a ``CommandSpec.wasm_runtime_import`` — the
hook defers the actual call emission to
``_WasmEmitterCmdMixin._emit_cmd_runtime``, which reads the import
metadata via :func:`runtime_import_for`.

Commands with a *specialised* emit hook (registered by their own
``cmds/<cmd>_.py`` file — ``format``, ``fconfigure``, ``puts``,
``concat``, ``linsert``, ``lreplace``, ``lsort``, ``append``,
``lappend``) are skipped here so the specialised hook wins.  This
file must therefore be imported *after* the specialised hooks in
``cmds/__init__.py`` — the import order is handled there.
"""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext


def _make_runtime_hook(cmd: str):
    def _hook(emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext) -> bool:
        emitter._emit_cmd_runtime(cmd, args, defs, context)
        return True

    _hook.__name__ = f"_hook_{cmd}"
    return _hook


for _name, _specs in REGISTRY.specs_by_name.items():
    if any(spec.wasm_runtime_import is not None for spec in _specs):
        # Skip if a specialised hook is already registered for this
        # command — check all specs for ``codegens["wasm"]``.
        if any("wasm" in spec.codegens for spec in _specs):
            continue
        REGISTRY.register_wasm_emitter(_name, _make_runtime_hook(_name))
