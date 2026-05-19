# Scaffolded from lreverse.n -- refine and commit
"""lreverse -- Reverse the order of a list."""

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
from .const_fold import fold_lreverse

_SOURCE = "Tcl man page lreverse.n"


@register
class LreverseCommand(CommandDef):
    name = "lreverse"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lreverse",
            hover=HoverSnippet(
                summary="Reverse the order of a list",
                synopsis=("lreverse list",),
                snippet="The lreverse command returns a list that has the same elements as its input list, list, except with the elements in the reverse order.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lreverse list",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 1),
            ),
            pure=True,
            const_fold=fold_lreverse,
            return_type=TclType.LIST,
            arg_types={0: ArgTypeHint(expected=TclType.LIST, shimmers=True)},
            side_effect_hints=(),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_list_reverse",
                argc=1,
                params=("i32",),
                results=("i32",),
            ),
        )
