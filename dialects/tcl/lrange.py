# Scaffolded from lrange.n -- refine and commit
"""lrange -- Return one or more adjacent elements from a list."""

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
from .const_fold import fold_lrange

_SOURCE = "Tcl man page lrange.n"


@register
class LrangeCommand(CommandDef):
    name = "lrange"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lrange",
            hover=HoverSnippet(
                summary="Return one or more adjacent elements from a list",
                synopsis=("lrange list first last",),
                snippet="List must be a valid Tcl list.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lrange list first last",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(3, 3),
            ),
            pure=True,
            const_fold=fold_lrange,
            cse_candidate=True,
            return_type=TclType.LIST,
            arg_types={
                0: ArgTypeHint(expected=TclType.LIST, shimmers=True),
                1: ArgTypeHint(expected=TclType.INT, shimmers=True),
                2: ArgTypeHint(expected=TclType.INT, shimmers=True),
            },
            side_effect_hints=(),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_list_range",
                argc=3,
                params=("i32", "i32", "i32"),
                results=("i32",),
            ),
        )
