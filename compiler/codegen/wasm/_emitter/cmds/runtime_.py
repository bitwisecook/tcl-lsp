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

from compiler.registry import REGISTRY, EmitContext


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
