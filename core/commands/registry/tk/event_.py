"""event -- Generate, manage, and inspect virtual events."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    SubCommand,
    ValidationSpec,
)
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page event.n"
_av = make_av(_SOURCE)


@register
class EventCommand(CommandDef):
    name = "event"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="event",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Generate, manage, and inspect virtual events.",
                synopsis=(
                    "event add <<virtual>> sequence ?sequence ...?",
                    "event delete <<virtual>> ?sequence sequence ...?",
                    "event generate window event ?option value ...?",
                    "event info ?<<virtual>>?",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="event option ?arg ...?",
                    options=(
                        OptionSpec(
                            name="-above",
                            takes_value=True,
                            value_hint="window",
                            detail="Specifies the above field for the event (generate).",
                        ),
                        OptionSpec(
                            name="-borderwidth",
                            takes_value=True,
                            value_hint="size",
                            detail="Specifies the border width for the event (generate).",
                        ),
                        OptionSpec(
                            name="-button",
                            takes_value=True,
                            value_hint="number",
                            detail="Specifies the button number for a ButtonPress or ButtonRelease event.",
                        ),
                        OptionSpec(
                            name="-count",
                            takes_value=True,
                            value_hint="number",
                            detail="Specifies the count field for the event.",
                        ),
                        OptionSpec(
                            name="-data",
                            takes_value=True,
                            value_hint="string",
                            detail="Specifies user data for a virtual event.",
                        ),
                        OptionSpec(
                            name="-delta",
                            takes_value=True,
                            value_hint="number",
                            detail="Specifies the delta field for a MouseWheel event.",
                        ),
                        OptionSpec(
                            name="-detail",
                            takes_value=True,
                            value_hint="detail",
                            detail="Specifies the detail field for the event.",
                        ),
                        OptionSpec(
                            name="-focus",
                            takes_value=True,
                            value_hint="boolean",
                            detail="Specifies the focus field for the event.",
                        ),
                        OptionSpec(
                            name="-height",
                            takes_value=True,
                            value_hint="size",
                            detail="Specifies the height field for the event.",
                        ),
                        OptionSpec(
                            name="-keycode",
                            takes_value=True,
                            value_hint="number",
                            detail="Specifies the keycode field for the event.",
                        ),
                        OptionSpec(
                            name="-keysym",
                            takes_value=True,
                            value_hint="name",
                            detail="Specifies the keysym field for the event.",
                        ),
                        OptionSpec(
                            name="-mode",
                            takes_value=True,
                            value_hint="notify",
                            detail="Specifies the mode field for the event.",
                        ),
                        OptionSpec(
                            name="-override",
                            takes_value=True,
                            value_hint="boolean",
                            detail="Specifies the override-redirect field.",
                        ),
                        OptionSpec(
                            name="-place",
                            takes_value=True,
                            value_hint="where",
                            detail="Specifies the place field for the event.",
                        ),
                        OptionSpec(
                            name="-root",
                            takes_value=True,
                            value_hint="window",
                            detail="Specifies the root field for the event.",
                        ),
                        OptionSpec(
                            name="-rootx",
                            takes_value=True,
                            value_hint="coord",
                            detail="Specifies the x-coordinate relative to the root window.",
                        ),
                        OptionSpec(
                            name="-rooty",
                            takes_value=True,
                            value_hint="coord",
                            detail="Specifies the y-coordinate relative to the root window.",
                        ),
                        OptionSpec(
                            name="-sendevent",
                            takes_value=True,
                            value_hint="boolean",
                            detail="Specifies the send-event field.",
                        ),
                        OptionSpec(
                            name="-serial",
                            takes_value=True,
                            value_hint="number",
                            detail="Specifies the serial number field.",
                        ),
                        OptionSpec(
                            name="-state",
                            takes_value=True,
                            value_hint="state",
                            detail="Specifies the state field for the event.",
                        ),
                        OptionSpec(
                            name="-subwindow",
                            takes_value=True,
                            value_hint="window",
                            detail="Specifies the sub-window field.",
                        ),
                        OptionSpec(
                            name="-time",
                            takes_value=True,
                            value_hint="integer",
                            detail="Specifies the time field for the event.",
                        ),
                        OptionSpec(
                            name="-warp",
                            takes_value=True,
                            value_hint="boolean",
                            detail="Specifies whether the screen pointer should be warped.",
                        ),
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            value_hint="size",
                            detail="Specifies the width field for the event.",
                        ),
                        OptionSpec(
                            name="-when",
                            takes_value=True,
                            value_hint="now|tail|head|mark",
                            detail="Specifies when the event is processed.",
                        ),
                        OptionSpec(
                            name="-x",
                            takes_value=True,
                            value_hint="coord",
                            detail="Specifies the x field for the event.",
                        ),
                        OptionSpec(
                            name="-y",
                            takes_value=True,
                            value_hint="coord",
                            detail="Specifies the y field for the event.",
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "add",
                                "Associate a virtual event with one or more physical event sequences.",
                                "event add <<virtual>> sequence ?sequence ...?",
                            ),
                            _av(
                                "delete",
                                "Delete physical event sequences from a virtual event.",
                                "event delete <<virtual>> ?sequence sequence ...?",
                            ),
                            _av(
                                "generate",
                                "Generate an event and arrange for it to be processed.",
                                "event generate window event ?option value ...?",
                            ),
                            _av(
                                "info",
                                "Return information about virtual events.",
                                "event info ?<<virtual>>?",
                            ),
                        ),
                    },
                ),
            ),
            subcommands={
                "add": SubCommand(
                    name="add",
                    arity=Arity(2),
                    detail="Associate a virtual event with one or more physical event sequences.",
                    synopsis="event add <<virtual>> sequence ?sequence ...?",
                ),
                "delete": SubCommand(
                    name="delete",
                    arity=Arity(1),
                    detail="Delete physical event sequences from a virtual event.",
                    synopsis="event delete <<virtual>> ?sequence sequence ...?",
                ),
                "generate": SubCommand(
                    name="generate",
                    arity=Arity(2),
                    detail="Generate an event and arrange for it to be processed.",
                    synopsis="event generate window event ?option value ...?",
                ),
                "info": SubCommand(
                    name="info",
                    arity=Arity(0, 1),
                    detail="Return information about virtual events.",
                    synopsis="event info ?<<virtual>>?",
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
