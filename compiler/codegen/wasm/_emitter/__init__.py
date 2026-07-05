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

"""WASM emitter package — splits _WasmEmitter across functional submodules."""

from __future__ import annotations

from . import cmds as _cmds  # side-effect: registers all WASM emit hooks via REGISTRY
from ._commands import _WasmEmitterCmdMixin
from ._control_flow import _WasmEmitterCtrlMixin
from ._core import _WasmEmitterBase
from ._expressions import _WasmEmitterExprMixin
from ._ops import _BINOP_WASM, _UNARYOP_WASM
from ._optimisation import _WasmEmitterOptMixin
from ._statements import _WasmEmitterStmtMixin
from ._values import _WasmEmitterValuesMixin
from ._variables import _WasmEmitterVarMixin
from .cmds.array_ import _CmdArrayMixin
from .cmds.clock_ import _CmdClockMixin
from .cmds.info_ import _CmdInfoMixin
from .cmds.list_ import _CmdListMixin
from .cmds.return_ import _CmdReturnMixin
from .cmds.uplevel_ import _CmdUplevelMixin


class _WasmEmitter(
    _WasmEmitterValuesMixin,
    _WasmEmitterExprMixin,
    _WasmEmitterStmtMixin,
    _WasmEmitterVarMixin,
    _WasmEmitterCmdMixin,
    # Per-command mixins (Phase E.2 — migrated from _cmd_helpers.py so
    # each command's Python codegen lives in its cmds/*.py file).  The
    # order is alphabetical — no MRO dependency between them since
    # they only contribute method slots.
    _CmdArrayMixin,
    _CmdClockMixin,
    _CmdInfoMixin,
    _CmdListMixin,
    _CmdReturnMixin,
    _CmdUplevelMixin,
    _WasmEmitterCtrlMixin,
    _WasmEmitterOptMixin,
    _WasmEmitterBase,  # last — must follow all mixins that inherit from it (TYPE_CHECKING)
):
    pass
