"""tk_chooseDirectory -- Display a directory chooser dialogue."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page tk_chooseDirectory.n"


@register
class TkChooseDirectoryCommand(CommandDef):
    name = "tk_chooseDirectory"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tk_chooseDirectory",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Pop up a dialogue for the user to select a directory.",
                synopsis=("tk_chooseDirectory ?option value ...?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="tk_chooseDirectory ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-initialdir",
                            takes_value=True,
                            value_hint="dirName",
                            detail="Initial directory to display.",
                        ),
                        OptionSpec(
                            name="-mustexist",
                            takes_value=True,
                            value_hint="boolean",
                            detail="Whether the user must select an existing directory.",
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
