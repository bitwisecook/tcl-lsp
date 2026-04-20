"""entry -- Create and manipulate single-line text entry widgets."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page entry.n"


@register
class EntryCommand(CommandDef):
    name = "entry"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="entry",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a single-line text entry widget.",
                synopsis=("entry pathName ?option value ...?",),
                snippet=(
                    "Displays a one-line text string and allows the user to "
                    "edit it using standard editing characters."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="entry pathName ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-textvariable",
                            takes_value=True,
                            detail="Name of a variable linked to the entry's contents.",
                        ),
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            detail="Desired width of the entry in average-size characters.",
                        ),
                        OptionSpec(
                            name="-state",
                            takes_value=True,
                            detail="State of the entry: normal, disabled, or readonly.",
                        ),
                        OptionSpec(
                            name="-show",
                            takes_value=True,
                            detail="Character to display instead of actual contents (e.g. '*' for passwords).",
                        ),
                        OptionSpec(
                            name="-font",
                            takes_value=True,
                            detail="Font to use for text in the entry.",
                        ),
                        OptionSpec(
                            name="-bg",
                            takes_value=True,
                            detail="Shorthand for -background.",
                        ),
                        OptionSpec(
                            name="-fg",
                            takes_value=True,
                            detail="Shorthand for -foreground.",
                        ),
                        OptionSpec(
                            name="-relief",
                            takes_value=True,
                            detail="3-D effect: flat, groove, raised, ridge, solid, or sunken.",
                        ),
                        OptionSpec(
                            name="-justify",
                            takes_value=True,
                            detail="Justification of text within the entry: left, center, or right.",
                        ),
                        OptionSpec(
                            name="-insertbackground",
                            takes_value=True,
                            detail="Colour of the insertion cursor.",
                        ),
                        OptionSpec(
                            name="-insertborderwidth",
                            takes_value=True,
                            detail="Width of the border around the insertion cursor.",
                        ),
                        OptionSpec(
                            name="-insertofftime",
                            takes_value=True,
                            detail="Milliseconds the insertion cursor is off during blinking.",
                        ),
                        OptionSpec(
                            name="-insertontime",
                            takes_value=True,
                            detail="Milliseconds the insertion cursor is on during blinking.",
                        ),
                        OptionSpec(
                            name="-insertwidth",
                            takes_value=True,
                            detail="Width of the insertion cursor in screen units.",
                        ),
                        OptionSpec(
                            name="-selectbackground",
                            takes_value=True,
                            detail="Background colour for selected text.",
                        ),
                        OptionSpec(
                            name="-selectborderwidth",
                            takes_value=True,
                            detail="Width of the border around selected text.",
                        ),
                        OptionSpec(
                            name="-selectforeground",
                            takes_value=True,
                            detail="Foreground colour for selected text.",
                        ),
                        OptionSpec(
                            name="-xscrollcommand",
                            takes_value=True,
                            detail="Command prefix for communicating with horizontal scrollbars.",
                        ),
                        OptionSpec(
                            name="-exportselection",
                            takes_value=True,
                            detail="Whether the selection is exported to the X selection.",
                        ),
                        OptionSpec(
                            name="-readonlybackground",
                            takes_value=True,
                            detail="Background colour when the entry is in readonly state.",
                        ),
                        OptionSpec(
                            name="-validate",
                            takes_value=True,
                            detail="Validation mode: none, focus, focusin, focusout, key, or all.",
                        ),
                        OptionSpec(
                            name="-validatecommand",
                            takes_value=True,
                            detail="Script to evaluate when validation is triggered.",
                        ),
                        OptionSpec(
                            name="-invalidcommand",
                            takes_value=True,
                            detail="Script to evaluate when validation fails.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            detail="Cursor to display when the mouse is over the entry.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            detail="Whether the entry accepts focus during keyboard traversal.",
                        ),
                        OptionSpec(
                            name="-highlightbackground",
                            takes_value=True,
                            detail="Colour of the highlight region when the entry does not have focus.",
                        ),
                        OptionSpec(
                            name="-highlightcolor",
                            takes_value=True,
                            detail="Colour of the highlight region when the entry has focus.",
                        ),
                        OptionSpec(
                            name="-highlightthickness",
                            takes_value=True,
                            detail="Width of the highlight rectangle drawn around the entry.",
                        ),
                    ),
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
