"""WASM emitter package — splits _WasmEmitter across functional submodules."""

from __future__ import annotations

from ._commands import _WasmEmitterCmdMixin
from ._control_flow import _WasmEmitterCtrlMixin
from ._core import _WasmEmitterBase
from ._expressions import _WasmEmitterExprMixin
from ._ops import _BINOP_WASM, _UNARYOP_WASM
from ._optimisation import _WasmEmitterOptMixin
from ._statements import _WasmEmitterStmtMixin
from ._values import _WasmEmitterValuesMixin
from ._variables import _WasmEmitterVarMixin


class _WasmEmitter(
    _WasmEmitterBase,
    _WasmEmitterValuesMixin,
    _WasmEmitterExprMixin,
    _WasmEmitterStmtMixin,
    _WasmEmitterVarMixin,
    _WasmEmitterCmdMixin,
    _WasmEmitterCtrlMixin,
    _WasmEmitterOptMixin,
):
    pass
