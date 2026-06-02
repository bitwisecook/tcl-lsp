"""Mixin: bytecoded command optimisations.

Per-command codegen hooks are registered on the REGISTRY by the
``bytecoded`` subpackage.  This mixin dispatches to those hooks.
"""

from __future__ import annotations

from typing import TYPE_CHECKING

if TYPE_CHECKING:
    from ._emitter import _Emitter

# Register all per-command codegen hooks on first import.
from .bytecoded import register_all as _register_codegen_hooks

_register_codegen_hooks()


class _BytecodedMixin:
    """Mixin: bytecoded command optimisations dispatched via REGISTRY hooks."""

    def _try_bytecoded(self: _Emitter, cmd: str, args: tuple[str, ...]) -> bool:
        from compiler.registry import REGISTRY

        # TODO(command-binding): this bytecode backend direct-dispatches a
        # builtin by *name* via its registry ``vm`` codegen hook, ignoring any
        # ``rename`` / redefinition in the unit — the same unsoundness the WASM
        # backend had before it was gated (e.g. ``rename string ::s; string
        # length hi`` would still emit the builtin).  It needs the same fix:
        #   1. scope the lattice-derived trust verdict around the bytecode
        #      codegen entry (``with command_trust(module_command_trust(...))``,
        #      as compiler/codegen/wasm/api.py does), and
        #   2. gate this dispatch on ``builtin_is_trusted(cmd)`` so a rebound
        #      command falls through to the interpreter path instead.
        # And, as in WASM, full correctness for a builtin renamed *to a proc*
        # additionally needs interp→compiled-proc dispatch in the VM runtime.
        spec = REGISTRY.get_any(cmd)
        if spec is None:
            return False
        hook = spec.codegens.get("vm")
        if hook is None:
            return False
        return hook(self, args)
