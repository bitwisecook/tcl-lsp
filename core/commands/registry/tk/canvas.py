"""canvas -- Create and manipulate canvas widgets."""

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

_SOURCE = "Tk man page canvas.n"
_av = make_av(_SOURCE)


@register
class CanvasCommand(CommandDef):
    name = "canvas"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="canvas",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a canvas widget.",
                synopsis=("canvas pathName ?option value ...?",),
                snippet=(
                    "Displays a rectangular area for drawing graphics, "
                    "positioning widgets, and handling events on items."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="canvas pathName ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            detail="Desired width of the canvas in screen units.",
                        ),
                        OptionSpec(
                            name="-height",
                            takes_value=True,
                            detail="Desired height of the canvas in screen units.",
                        ),
                        OptionSpec(
                            name="-bg",
                            takes_value=True,
                            detail="Shorthand for -background.",
                        ),
                        OptionSpec(
                            name="-background",
                            takes_value=True,
                            detail="Background colour of the canvas.",
                        ),
                        OptionSpec(
                            name="-relief",
                            takes_value=True,
                            detail="3-D effect: flat, groove, raised, ridge, solid, or sunken.",
                        ),
                        OptionSpec(
                            name="-borderwidth",
                            takes_value=True,
                            detail="Width of the border around the canvas.",
                        ),
                        OptionSpec(
                            name="-bd",
                            takes_value=True,
                            detail="Shorthand for -borderwidth.",
                        ),
                        OptionSpec(
                            name="-scrollregion",
                            takes_value=True,
                            detail="Bounding box of the total scrollable area (left top right bottom).",
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
                            name="-xscrollincrement",
                            takes_value=True,
                            detail="Horizontal scrolling increment in screen units.",
                        ),
                        OptionSpec(
                            name="-yscrollincrement",
                            takes_value=True,
                            detail="Vertical scrolling increment in screen units.",
                        ),
                        OptionSpec(
                            name="-confine",
                            takes_value=True,
                            detail="Whether scrolling is confined to the scroll region.",
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
                            name="-closeenough",
                            takes_value=True,
                            detail="Proximity threshold for mouse cursor to be considered over an item.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            detail="Cursor to display when the mouse is over the canvas.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            detail="Whether the canvas accepts focus during keyboard traversal.",
                        ),
                        OptionSpec(
                            name="-highlightbackground",
                            takes_value=True,
                            detail="Colour of the highlight region when the canvas does not have focus.",
                        ),
                        OptionSpec(
                            name="-highlightcolor",
                            takes_value=True,
                            detail="Colour of the highlight region when the canvas has focus.",
                        ),
                        OptionSpec(
                            name="-highlightthickness",
                            takes_value=True,
                            detail="Width of the highlight rectangle drawn around the canvas.",
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "create",
                                "Create a new canvas item of the specified type.",
                                "pathName create type x y ?x y ...? ?option value ...?",
                            ),
                            _av(
                                "delete",
                                "Delete the items given by each tagOrId.",
                                "pathName delete ?tagOrId ...?",
                            ),
                            _av(
                                "move",
                                "Move each item by the given distance.",
                                "pathName move tagOrId xAmount yAmount",
                            ),
                            _av(
                                "coords",
                                "Query or set the coordinates of an item.",
                                "pathName coords tagOrId ?x y ...?",
                            ),
                            _av(
                                "itemconfigure",
                                "Query or modify configuration options of a canvas item.",
                                "pathName itemconfigure tagOrId ?option? ?value option value ...?",
                            ),
                            _av(
                                "find",
                                "Return item IDs matching a search specification.",
                                "pathName find searchCommand ?arg ...?",
                            ),
                            _av(
                                "type",
                                "Return the type of the item given by tagOrId.",
                                "pathName type tagOrId",
                            ),
                            _av(
                                "bbox",
                                "Return the bounding box of the items given by the tagOrIds.",
                                "pathName bbox tagOrId ?tagOrId ...?",
                            ),
                            _av(
                                "scan",
                                "Implement scanning for the canvas.",
                                "pathName scan option args",
                            ),
                            _av(
                                "xview",
                                "Query or change the horizontal view position.",
                                "pathName xview ?args?",
                            ),
                            _av(
                                "yview",
                                "Query or change the vertical view position.",
                                "pathName yview ?args?",
                            ),
                            _av(
                                "postscript",
                                "Generate a Postscript representation of the canvas.",
                                "pathName postscript ?option value ...?",
                            ),
                            _av(
                                "bind",
                                "Associate a command with a canvas item event.",
                                "pathName bind tagOrId ?sequence? ?command?",
                            ),
                            _av(
                                "dtag",
                                "Remove a tag from the items given by tagOrId.",
                                "pathName dtag tagOrId ?tagToDelete?",
                            ),
                            _av(
                                "addtag",
                                "Add a tag to items matching a search specification.",
                                "pathName addtag tag searchCommand ?arg ...?",
                            ),
                            _av(
                                "gettags",
                                "Return the tags associated with the item.",
                                "pathName gettags tagOrId",
                            ),
                            _av(
                                "raise",
                                "Raise the items given by tagOrId in the display list.",
                                "pathName raise tagOrId ?aboveThis?",
                            ),
                            _av(
                                "lower",
                                "Lower the items given by tagOrId in the display list.",
                                "pathName lower tagOrId ?belowThis?",
                            ),
                            _av(
                                "scale",
                                "Rescale the coordinates of items.",
                                "pathName scale tagOrId xOrigin yOrigin xScale yScale",
                            ),
                            _av(
                                "canvasx",
                                "Convert a window x-coordinate to a canvas x-coordinate.",
                                "pathName canvasx screenx ?gridspacing?",
                            ),
                            _av(
                                "canvasy",
                                "Convert a window y-coordinate to a canvas y-coordinate.",
                                "pathName canvasy screeny ?gridspacing?",
                            ),
                            _av(
                                "focus",
                                "Set or query the focus item for the canvas.",
                                "pathName focus ?tagOrId?",
                            ),
                        ),
                    },
                ),
            ),
            subcommands={
                "addtag": SubCommand(
                    name="addtag",
                    arity=Arity(2),
                    detail="Add a tag to items matching a search specification.",
                    synopsis="pathName addtag tag searchCommand ?arg ...?",
                ),
                "bbox": SubCommand(
                    name="bbox",
                    arity=Arity(1),
                    detail="Return the bounding box of the items given by the tagOrIds.",
                    synopsis="pathName bbox tagOrId ?tagOrId ...?",
                ),
                "bind": SubCommand(
                    name="bind",
                    arity=Arity(1),
                    detail="Associate a command with a canvas item event.",
                    synopsis="pathName bind tagOrId ?sequence? ?command?",
                ),
                "canvasx": SubCommand(
                    name="canvasx",
                    arity=Arity(1, 2),
                    detail="Convert a window x-coordinate to a canvas x-coordinate.",
                    synopsis="pathName canvasx screenx ?gridspacing?",
                ),
                "canvasy": SubCommand(
                    name="canvasy",
                    arity=Arity(1, 2),
                    detail="Convert a window y-coordinate to a canvas y-coordinate.",
                    synopsis="pathName canvasy screeny ?gridspacing?",
                ),
                "coords": SubCommand(
                    name="coords",
                    arity=Arity(1),
                    detail="Query or set the coordinates of an item.",
                    synopsis="pathName coords tagOrId ?x y ...?",
                ),
                "create": SubCommand(
                    name="create",
                    arity=Arity(3),
                    detail="Create a new canvas item of the specified type.",
                    synopsis="pathName create type x y ?x y ...? ?option value ...?",
                ),
                "delete": SubCommand(
                    name="delete",
                    arity=Arity(0),
                    detail="Delete the items given by each tagOrId.",
                    synopsis="pathName delete ?tagOrId ...?",
                ),
                "dtag": SubCommand(
                    name="dtag",
                    arity=Arity(1, 2),
                    detail="Remove a tag from the items given by tagOrId.",
                    synopsis="pathName dtag tagOrId ?tagToDelete?",
                ),
                "find": SubCommand(
                    name="find",
                    arity=Arity(1),
                    detail="Return item IDs matching a search specification.",
                    synopsis="pathName find searchCommand ?arg ...?",
                ),
                "focus": SubCommand(
                    name="focus",
                    arity=Arity(0, 1),
                    detail="Set or query the focus item for the canvas.",
                    synopsis="pathName focus ?tagOrId?",
                ),
                "gettags": SubCommand(
                    name="gettags",
                    arity=Arity(1, 1),
                    detail="Return the tags associated with the item.",
                    synopsis="pathName gettags tagOrId",
                ),
                "itemconfigure": SubCommand(
                    name="itemconfigure",
                    arity=Arity(1),
                    detail="Query or modify configuration options of a canvas item.",
                    synopsis="pathName itemconfigure tagOrId ?option? ?value option value ...?",
                ),
                "lower": SubCommand(
                    name="lower",
                    arity=Arity(1, 2),
                    detail="Lower the items given by tagOrId in the display list.",
                    synopsis="pathName lower tagOrId ?belowThis?",
                ),
                "move": SubCommand(
                    name="move",
                    arity=Arity(3, 3),
                    detail="Move each item by the given distance.",
                    synopsis="pathName move tagOrId xAmount yAmount",
                ),
                "postscript": SubCommand(
                    name="postscript",
                    arity=Arity(0),
                    detail="Generate a Postscript representation of the canvas.",
                    synopsis="pathName postscript ?option value ...?",
                ),
                "raise": SubCommand(
                    name="raise",
                    arity=Arity(1, 2),
                    detail="Raise the items given by tagOrId in the display list.",
                    synopsis="pathName raise tagOrId ?aboveThis?",
                ),
                "scale": SubCommand(
                    name="scale",
                    arity=Arity(5, 5),
                    detail="Rescale the coordinates of items.",
                    synopsis="pathName scale tagOrId xOrigin yOrigin xScale yScale",
                ),
                "scan": SubCommand(
                    name="scan",
                    arity=Arity(1),
                    detail="Implement scanning for the canvas.",
                    synopsis="pathName scan option args",
                ),
                "type": SubCommand(
                    name="type",
                    arity=Arity(1, 1),
                    detail="Return the type of the item given by tagOrId.",
                    synopsis="pathName type tagOrId",
                ),
                "xview": SubCommand(
                    name="xview",
                    arity=Arity(0),
                    detail="Query or change the horizontal view position.",
                    synopsis="pathName xview ?args?",
                ),
                "yview": SubCommand(
                    name="yview",
                    arity=Arity(0),
                    detail="Query or change the vertical view position.",
                    synopsis="pathName yview ?args?",
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
