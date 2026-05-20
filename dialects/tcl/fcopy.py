# Scaffolded from fcopy.n -- refine and commit
"""fcopy -- Copy data from one channel to another."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.dialects import DIALECTS_EXCEPT_IRULES
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
    WasmRuntimeImport,
)
from compiler.registry.signatures import ArgRole, Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page fcopy.n"


@register
class FcopyCommand(CommandDef):
    name = "fcopy"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="fcopy",
            byte_compiled=True,
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Copy data from one channel to another",
                synopsis=("fcopy inputChan outputChan ?-size size? ?-command callback?",),
                snippet="The fcopy command copies data from one I/O channel, inchan, to another I/O channel, outchan.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="fcopy inputChan outputChan ?-size size? ?-command callback?",
                ),
            ),
            validation=ValidationSpec(
                # C Tcl 9.0 ``Tcl_FcopyObjCmd``: objc must be 3, 5, or 7
                # (inputChan + outputChan + optional ``-size N`` and/or
                # ``-command cb`` option pairs).  Args after command
                # name: 2..6, with only even values actually legal.
                arity=Arity(2, 6),
            ),
            arg_roles={0: frozenset({ArgRole.CHANNEL}), 1: frozenset({ArgRole.CHANNEL})},
            return_type=TclType.INT,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_fcopy",
                argc=2,
                params=("i32", "i32"),
                results=("i32",),
            ),
        )
