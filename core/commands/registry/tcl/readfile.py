"""readFile -- Read the contents of a text or binary file (Tcl 9.0+, TIP 670)."""

from __future__ import annotations

from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

from .._base import CommandDef
from ..models import (
    ArgumentValueSpec,
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
)
from ..signatures import Arity
from ._base import register

_SOURCE = "Tcl man page library.n"

_MODES = (
    ArgumentValueSpec(value="text", detail="Read using system defaults for text files (default)."),
    ArgumentValueSpec(value="binary", detail="Read as uninterpreted bytes."),
)


@register
class ReadFileCommand(CommandDef):
    name = "readFile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="readFile",
            dialects=frozenset({"tcl9.0"}),
            hover=HoverSnippet(
                summary="Read the contents of a file and return them.",
                synopsis=("readFile filename ?text|binary?",),
                snippet=(
                    "Reads the file *filename* and returns its contents. The "
                    "optional mode is `text` (default; system text-file "
                    "defaults, includes any trailing newline) or `binary` "
                    "(uninterpreted bytes). The file is closed before the "
                    "procedure returns. Introduced by TIP 670 in Tcl 9.0."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="readFile filename ?text|binary?",
                    arg_values={1: _MODES},
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 2),
            ),
            return_type=TclType.STRING,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
