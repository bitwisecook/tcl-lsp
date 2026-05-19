# Scaffolded from lindex.n -- refine and commit
"""lindex -- Retrieve an element from a list."""

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
from .const_fold import fold_lindex

_SOURCE = "Tcl man page lindex.n"


@register
class LindexCommand(CommandDef):
    name = "lindex"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lindex",
            hover=HoverSnippet(
                summary="Retrieve an element from a list",
                synopsis=("lindex list ?index ...?",),
                snippet="The lindex command accepts a parameter, list, which it treats as a Tcl list.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lindex list ?index ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            pure=True,
            const_fold=fold_lindex,
            cse_candidate=True,
            return_type=TclType.STRING,
            arg_types={
                0: ArgTypeHint(expected=TclType.LIST, shimmers=True),
                1: ArgTypeHint(expected=TclType.INT, shimmers=True),
            },
            side_effect_hints=(),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_list_index",
                argc=2,
                params=("i32", "i32"),
                results=("i32",),
            ),
        )
