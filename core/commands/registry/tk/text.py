"""text -- Create and manipulate multi-line text widgets."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page text.n"


@register
class TextCommand(CommandDef):
    name = "text"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="text",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a multi-line text widget.",
                synopsis=("text pathName ?option value ...?",),
                snippet=(
                    "Displays one or more lines of text and allows the user to "
                    "edit them. Supports embedded images and windows."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="text pathName ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            detail="Desired width of the text widget in characters.",
                        ),
                        OptionSpec(
                            name="-height",
                            takes_value=True,
                            detail="Desired height of the text widget in lines.",
                        ),
                        OptionSpec(
                            name="-wrap",
                            takes_value=True,
                            detail="Line wrapping mode: none, char, or word.",
                        ),
                        OptionSpec(
                            name="-state",
                            takes_value=True,
                            detail="State of the text widget: normal or disabled.",
                        ),
                        OptionSpec(
                            name="-font",
                            takes_value=True,
                            detail="Font to use for text in the widget.",
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
                            name="-spacing1",
                            takes_value=True,
                            detail="Extra space above each line of text, in screen units.",
                        ),
                        OptionSpec(
                            name="-spacing2",
                            takes_value=True,
                            detail="Extra space between display lines within a logical line, in screen units.",
                        ),
                        OptionSpec(
                            name="-spacing3",
                            takes_value=True,
                            detail="Extra space below each line of text, in screen units.",
                        ),
                        OptionSpec(
                            name="-tabs",
                            takes_value=True,
                            detail="Tab stop positions and alignment for the text widget.",
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
                            name="-yscrollcommand",
                            takes_value=True,
                            detail="Command prefix for communicating with vertical scrollbars.",
                        ),
                        OptionSpec(
                            name="-exportselection",
                            takes_value=True,
                            detail="Whether the selection is exported to the X selection.",
                        ),
                        OptionSpec(
                            name="-setgrid",
                            takes_value=True,
                            detail="Whether this widget controls the resizing grid for its toplevel.",
                        ),
                        OptionSpec(
                            name="-padx",
                            takes_value=True,
                            detail="Extra horizontal padding inside the text widget.",
                        ),
                        OptionSpec(
                            name="-pady",
                            takes_value=True,
                            detail="Extra vertical padding inside the text widget.",
                        ),
                        OptionSpec(
                            name="-undo",
                            takes_value=True,
                            detail="Whether the undo mechanism is active.",
                        ),
                        OptionSpec(
                            name="-maxundo",
                            takes_value=True,
                            detail="Maximum number of compound undo actions on the undo stack.",
                        ),
                        OptionSpec(
                            name="-autoseparators",
                            takes_value=True,
                            detail="Whether undo separators are inserted automatically.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            detail="Cursor to display when the mouse is over the text widget.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            detail="Whether the text widget accepts focus during keyboard traversal.",
                        ),
                        OptionSpec(
                            name="-highlightbackground",
                            takes_value=True,
                            detail="Colour of the highlight region when the widget does not have focus.",
                        ),
                        OptionSpec(
                            name="-highlightcolor",
                            takes_value=True,
                            detail="Colour of the highlight region when the widget has focus.",
                        ),
                        OptionSpec(
                            name="-highlightthickness",
                            takes_value=True,
                            detail="Width of the highlight rectangle drawn around the widget.",
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
