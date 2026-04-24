"""Generic runtime-dispatch hook factory for built-in commands.

Provides :func:`register_generic_runtime_hooks`, invoked from
``cmds/__init__.py`` **after** every specialised hook module imports,
so commands with a ``CommandSpec.wasm_runtime_import`` that lack a
bespoke ``cmds/<cmd>_.py`` hook still dispatch to the Zig runtime
import via the generic push-args/call/settle-result path.

``register_wasm_emitter`` is first-writer-wins; the caller invokes
this last so specialised hooks always win.  The function is
idempotent — running it twice is a no-op (the second pass sees every
command already has a ``codegens["wasm"]`` entry and skips).
"""

from __future__ import annotations

from ......commands.registry import REGISTRY, EmitContext


def _make_runtime_hook(cmd: str):
    def _hook(emitter, args: tuple[str, ...], defs: tuple[str, ...], context: EmitContext) -> bool:
        emitter._emit_cmd_runtime(cmd, args, defs, context)
        return True

    _hook.__name__ = f"_hook_{cmd}"
    return _hook


def register_generic_runtime_hooks() -> None:
    """Register a generic runtime-dispatch hook for every spec that needs one.

    Skips commands whose spec already carries a ``codegens["wasm"]``
    entry (a specialised hook from ``cmds/<cmd>_.py``).
    """
    for name, specs in REGISTRY.specs_by_name.items():
        if not any(spec.wasm_runtime_import is not None for spec in specs):
            continue
        if any("wasm" in spec.codegens for spec in specs):
            continue
        REGISTRY.register_wasm_emitter(name, _make_runtime_hook(name))
