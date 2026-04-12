"""tk_messageBox -- Display a message dialogue."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page tk_messageBox.n"

_av = make_av(_SOURCE)

_ICON_VALUES = (
    _av("error", "Display an error icon."),
    _av("info", "Display an informational icon."),
    _av("question", "Display a question icon."),
    _av("warning", "Display a warning icon."),
)

_TYPE_VALUES = (
    _av("abortretryignore", "Display Abort, Retry, and Ignore buttons."),
    _av("ok", "Display only an OK button."),
    _av("okcancel", "Display OK and Cancel buttons."),
    _av("retrycancel", "Display Retry and Cancel buttons."),
    _av("yesno", "Display Yes and No buttons."),
    _av("yesnocancel", "Display Yes, No, and Cancel buttons."),
)


@register
class TkMessageBoxCommand(CommandDef):
    name = "tk_messageBox"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tk_messageBox",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Pop up a message window and wait for user response.",
                synopsis=("tk_messageBox ?option value ...?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="tk_messageBox ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-default",
                            takes_value=True,
                            value_hint="buttonName",
                            detail="Name of the default button.",
                        ),
                        OptionSpec(
                            name="-detail",
                            takes_value=True,
                            value_hint="string",
                            detail="Supplemental message text.",
                        ),
                        OptionSpec(
                            name="-icon",
                            takes_value=True,
                            value_hint="iconImage",
                            detail="Icon to display (error, info, question, warning).",
                        ),
                        OptionSpec(
                            name="-message",
                            takes_value=True,
                            value_hint="string",
                            detail="Main message text to display.",
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
                            name="-type",
                            takes_value=True,
                            value_hint="predefinedType",
                            detail="Arrangement of buttons to display.",
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
