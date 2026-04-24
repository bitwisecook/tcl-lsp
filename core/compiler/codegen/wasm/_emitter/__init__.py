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
