"""glob -- Return names of files that match patterns."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    OptionTerminatorSpec,
    ValidationSpec,
)
from ..signatures import Arity
from ._base import register


@register
class GlobCommand(CommandDef):
    name = "glob"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="glob",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Return names of files that match patterns.",
                synopsis=("glob ?switches? ?--? pattern ?pattern ...?",),
                snippet=(
                    "Performs file name globbing similar to `csh`. Returns a "
                    "list of matching file names.\n\n"
                    "Use `-nocomplain` to return an empty list instead of an "
                    "error when no files match. Use `--` before patterns that "
                    "may start with `-`."
                ),
                source="Tcl glob(1)",
                return_value="A list of file names matching the patterns.",
            ),
            option_terminator_profiles=(
                OptionTerminatorSpec(
                    scan_start=0,
                    options_with_values=frozenset({"-directory", "-path", "-types"}),
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="glob ?switches? ?--? pattern ?pattern ...?",
                    options=(
                        OptionSpec(name="-directory", takes_value=True, value_hint="dir"),
                        OptionSpec(name="-join"),
                        OptionSpec(name="-nocomplain"),
                        OptionSpec(name="-path", takes_value=True, value_hint="pathPrefix"),
                        OptionSpec(name="-tails"),
                        OptionSpec(name="-types", takes_value=True, value_hint="typeList"),
                        OptionSpec(name="--"),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            return_type=TclType.LIST,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
