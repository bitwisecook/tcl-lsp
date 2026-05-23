# Scaffolded from llength.n -- refine and commit
"""llength -- Count the number of elements in a list."""

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
from compiler.registry.type_hints import ArgTypeHint
from compiler.types import TclType

from ._base import register
from .const_fold import fold_llength

_SOURCE = "Tcl man page llength.n"


@register
class LlengthCommand(CommandDef):
    name = "llength"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="llength",
            byte_compiled=True,
            frameless_runtime=True,
            hover=HoverSnippet(
                summary="Count the number of elements in a list",
                synopsis=("llength list",),
                snippet="Treats list as a list and returns a decimal string giving the number of elements in it.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="llength list",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 1),
            ),
            pure=True,
            const_fold=fold_llength,
            cse_candidate=True,
            return_type=TclType.INT,
            arg_types={0: ArgTypeHint(expected=TclType.LIST, shimmers=True)},
            side_effect_hints=(),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_list_length",
                argc=1,
                params=("i32",),
                results=("i32",),
            ),
        )
