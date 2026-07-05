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

from compiler.registry import REGISTRY, EmitContext


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
