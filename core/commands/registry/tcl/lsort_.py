"""lsort -- Sort the elements of a list."""

from __future__ import annotations

from ....compiler.side_effects import StorageType
from ....compiler.types import TclType
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, OptionSpec, ValidationSpec, WasmRuntimeImport
from ..signatures import Arity
from ._base import register
from .shimmer_resolvers import resolve_lsort


@register
class LsortCommand(CommandDef):
    name = "lsort"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lsort",
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lsort ?options? list",
                    options=(
                        OptionSpec(name="-ascii"),
                        OptionSpec(name="-dictionary"),
                        OptionSpec(name="-integer"),
                        OptionSpec(name="-real"),
                        OptionSpec(name="-nocase"),
                        OptionSpec(name="-increasing"),
                        OptionSpec(name="-decreasing"),
                        OptionSpec(name="-indices"),
                        OptionSpec(name="-unique"),
                        OptionSpec(name="-command", takes_value=True, value_hint="cmdPrefix"),
                        OptionSpec(name="-index", takes_value=True, value_hint="index"),
                        OptionSpec(name="-stride", takes_value=True, value_hint="length"),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            pure=True,
            cse_candidate=True,
            return_type=TclType.LIST,
            arg_type_resolver=resolve_lsort,
            inferred_storage_type=StorageType.LIST,
            side_effect_hints=(),
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_list_sort",
                argc=1,
                params=("i32",),
                results=("i32",),
            ),
        )
