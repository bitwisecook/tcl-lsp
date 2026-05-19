"""writeFile -- Write contents to a text or binary file (Tcl 9.0+, TIP 670)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page library.n"


@register
class WriteFileCommand(CommandDef):
    name = "writeFile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="writeFile",
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Write contents to a file.",
                synopsis=("writeFile filename ?text|binary? contents",),
                snippet=(
                    "Writes *contents* to the file *filename*. The optional "
                    "mode is `text` (default; system text-file defaults) or "
                    "`binary` (uninterpreted bytes). A trailing newline, if "
                    "needed, must be included in *contents*. The result is "
                    "the empty string; the file is closed before the "
                    "procedure returns. Introduced by TIP 670 in Tcl 9.0."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="writeFile filename ?text|binary? contents",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(2, 3),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
