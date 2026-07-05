# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

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
        from compiler.command_trust import builtin_is_trusted
        from compiler.registry import REGISTRY

        # A builtin renamed / redefined in this unit no longer carries its core
        # semantics, so its ``vm`` codegen hook (a direct, name-keyed dispatch)
        # must not fire — return False so ``_emit_call`` falls through to the
        # generic ``invokeStk``, which resolves the name through the live,
        # rename-aware runtime command table (matching the WASM backend's
        # value/expr/statement gates).  The trust verdict is scoped around the
        # bytecode codegen entry in ``codegen_module``; outside that scope the
        # default "trust all" keeps standalone ``codegen_function`` unchanged.
        #
        # A builtin renamed *to a user proc* additionally needs interp→compiled-
        # proc dispatch in the VM runtime to *execute* correctly; until then the
        # generic invoke fails safe (the VM resolves the live command) rather
        # than emitting the wrong builtin inline.
        if not builtin_is_trusted(cmd):
            return False
        spec = REGISTRY.get_any(cmd)
        if spec is None:
            return False
        hook = spec.codegens.get("vm")
        if hook is None:
            return False
        return hook(self, args)
