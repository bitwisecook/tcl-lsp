"""tk_getSaveFile -- Display a file save dialogue."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page tk_getSaveFile.n"


@register
class TkGetSaveFileCommand(CommandDef):
    name = "tk_getSaveFile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tk_getSaveFile",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Pop up a dialogue for the user to select a file to save.",
                synopsis=("tk_getSaveFile ?option value ...?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="tk_getSaveFile ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-confirmoverwrite",
                            takes_value=True,
                            value_hint="boolean",
                            detail="Prompt for confirmation if the file already exists.",
                        ),
                        OptionSpec(
                            name="-defaultextension",
                            takes_value=True,
                            value_hint="extension",
                            detail="Default extension to append if the user does not type one.",
                        ),
                        OptionSpec(
                            name="-filetypes",
                            takes_value=True,
                            value_hint="filePatternList",
                            detail="List of file type patterns to display in the filter.",
                        ),
                        OptionSpec(
                            name="-initialdir",
                            takes_value=True,
                            value_hint="dirName",
                            detail="Initial directory to display.",
                        ),
                        OptionSpec(
                            name="-initialfile",
                            takes_value=True,
                            value_hint="fileName",
                            detail="Initial file name to populate in the dialogue.",
                        ),
                        OptionSpec(
                            name="-parent",
                            takes_value=True,
                            value_hint="window",
                            detail="Parent window for the dialogue.",
                        ),
                        OptionSpec(
                            name="-title",
                            takes_value=True,
                            value_hint="titleString",
                            detail="Title string for the dialogue window.",
                        ),
                        OptionSpec(
                            name="-typevariable",
                            takes_value=True,
                            value_hint="varName",
                            detail="Variable to store the selected file type.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(0),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
