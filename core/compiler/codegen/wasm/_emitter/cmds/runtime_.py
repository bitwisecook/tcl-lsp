"""WASM emit hooks for runtime-dispatched built-in commands.

The preferred source is ``CommandSpec.wasm_runtime_import`` (set on
individual spec files under ``core/commands/registry/tcl/``).  Until
every ``_CMD_RUNTIME`` entry has been migrated onto a spec the dict in
``_imports.py`` is still consulted as a fallback.  Entries present in
both sources resolve via the spec — the dict is ignored.
"""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext
from ..._imports import _CMD_RUNTIME


def _make_runtime_hook(cmd: str):
    def _hook(emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext) -> bool:
        if context is EmitContext.VALUE:
            # Tail-context not yet migrated — handled inline in _statements.py.
            return False
        emitter._emit_cmd_runtime(cmd, args, defs)
        return True

    _hook.__name__ = f"_hook_{cmd}"
    return _hook


# Spec-backed commands: the WASM import is declared on the
# ``CommandSpec.wasm_runtime_import`` field of the registered command.
_spec_backed: set[str] = set()
for _name, _specs in REGISTRY.specs_by_name.items():
    if any(spec.wasm_runtime_import is not None for spec in _specs):
        REGISTRY.register_wasm_emitter(_name, _make_runtime_hook(_name))
        _spec_backed.add(_name)

# Dict-backed commands: fallback for entries that haven't been migrated
# to ``CommandSpec.wasm_runtime_import`` yet.
for _cmd in _CMD_RUNTIME:
    if _cmd not in _spec_backed:
        REGISTRY.register_wasm_emitter(_cmd, _make_runtime_hook(_cmd))
