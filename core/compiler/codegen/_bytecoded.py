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
        from core.commands.registry import REGISTRY

        spec = REGISTRY.get_any(cmd)
        if spec is not None and spec.codegen is not None:
            return spec.codegen(self, args)
        return False
