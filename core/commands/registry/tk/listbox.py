"""listbox -- Create and manipulate listbox widgets."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page listbox.n"


@register
class ListboxCommand(CommandDef):
    name = "listbox"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="listbox",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a listbox widget.",
                synopsis=("listbox pathName ?option value ...?",),
                snippet=(
                    "Displays a list of strings, one per line, and "
                    "allows the user to select one or more of them."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="listbox pathName ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-listvariable",
                            takes_value=True,
                            detail="Name of a variable containing the list of values to display.",
                        ),
                        OptionSpec(
                            name="-selectmode",
                            takes_value=True,
                            detail="Selection mode: single, browse, multiple, or extended.",
                        ),
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            detail="Desired width of the listbox in characters.",
                        ),
                        OptionSpec(
                            name="-height",
                            takes_value=True,
                            detail="Desired height of the listbox in lines.",
                        ),
                        OptionSpec(
                            name="-font",
                            takes_value=True,
                            detail="Font to use for text in the listbox.",
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
                            name="-selectbackground",
                            takes_value=True,
                            detail="Background colour for selected items.",
                        ),
                        OptionSpec(
                            name="-selectborderwidth",
                            takes_value=True,
                            detail="Width of the border around selected items.",
                        ),
                        OptionSpec(
                            name="-selectforeground",
                            takes_value=True,
                            detail="Foreground colour for selected items.",
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
                            name="-activestyle",
                            takes_value=True,
                            detail="Style for the active element: dotbox, none, or underline.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            detail="Cursor to display when the mouse is over the listbox.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            detail="Whether the listbox accepts focus during keyboard traversal.",
                        ),
                        OptionSpec(
                            name="-highlightbackground",
                            takes_value=True,
                            detail="Colour of the highlight region when the listbox does not have focus.",
                        ),
                        OptionSpec(
                            name="-highlightcolor",
                            takes_value=True,
                            detail="Colour of the highlight region when the listbox has focus.",
                        ),
                        OptionSpec(
                            name="-highlightthickness",
                            takes_value=True,
                            detail="Width of the highlight rectangle drawn around the listbox.",
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
