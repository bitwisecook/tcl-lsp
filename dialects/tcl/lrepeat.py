"""lrepeat -- Build a list by repeating elements."""

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
from compiler.side_effects import StorageType
from compiler.types import TclType

from ._base import register
from .const_fold import fold_lrepeat

_SOURCE = "Tcl man page lrepeat.n"


@register
class LrepeatCommand(CommandDef):
    name = "lrepeat"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lrepeat",
            frameless_runtime=True,
            hover=HoverSnippet(
                summary="Build a list by repeating elements",
                synopsis=("lrepeat count ?element ...?",),
                snippet="The lrepeat command creates a list of size count * number of elements by repeating count times the sequence of elements element ....",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lrepeat count ?element ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            pure=True,
            const_fold=fold_lrepeat,
            return_type=TclType.LIST,
            inferred_storage_type=StorageType.LIST,
            arg_types={0: ArgTypeHint(expected=TclType.INT, shimmers=True)},
            side_effect_hints=(),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_list_repeat",
                argc=2,
                params=("i32", "i32"),
                results=("i32",),
            ),
        )
