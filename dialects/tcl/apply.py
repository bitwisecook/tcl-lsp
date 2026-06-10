# Scaffolded from apply.n -- refine and commit
"""apply -- Apply an anonymous function."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
    WasmRuntimeImport,
)
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page apply.n"


@register
class ApplyCommand(CommandDef):
    name = "apply"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="apply",
            byte_compiled=True,
            not_proc_factory=True,
            is_language_keyword=True,
            hover=HoverSnippet(
                summary="Apply an anonymous function",
                synopsis=("apply func ?arg1 arg2 ...?",),
                snippet="The command apply applies the function func to the arguments arg1 arg2 ...",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="apply func ?arg1 arg2 ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.PROC_DEFINITION, connection_side=ConnectionSide.NONE
                ),
            ),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_apply",
                argc=2,
                params=("i32", "i32"),
                results=("i32",),
            ),
        )
