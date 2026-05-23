"""lsort -- Sort the elements of a list."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    OptionSpec,
    ValidationSpec,
    WasmRuntimeImport,
)
from compiler.registry.signatures import Arity
from compiler.side_effects import StorageType
from compiler.types import TclType

from ._base import register
from .shimmer_resolvers import resolve_lsort


@register
class LsortCommand(CommandDef):
    name = "lsort"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lsort",
            byte_compiled=True,
            frameless_runtime=True,
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
                        # -indices: Tcl 8.5+ (rejected in 8.4).
                        OptionSpec(
                            name="-indices",
                            dialects=frozenset({"tcl8.5", "tcl8.6", "tcl9.0"}),
                        ),
                        OptionSpec(name="-unique"),
                        OptionSpec(name="-command", takes_value=True, value_hint="cmdPrefix"),
                        OptionSpec(name="-index", takes_value=True, value_hint="index"),
                        # -stride: Tcl 8.6+ (rejected in 8.4/8.5).
                        OptionSpec(
                            name="-stride",
                            takes_value=True,
                            value_hint="length",
                            dialects=frozenset({"tcl8.6", "tcl9.0"}),
                        ),
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
