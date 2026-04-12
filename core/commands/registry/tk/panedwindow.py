"""panedwindow -- Create and manipulate panedwindow widgets."""

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

_SOURCE = "Tk man page panedwindow.n"
_av = make_av(_SOURCE)


@register
class PanedwindowCommand(CommandDef):
    name = "panedwindow"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="panedwindow",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a panedwindow widget.",
                synopsis=("panedwindow pathName ?option value ...?",),
                snippet=(
                    "Displays a container that divides its space among "
                    "child widgets separated by movable sashes."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="panedwindow pathName ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-orient",
                            takes_value=True,
                            detail="Orientation of the panedwindow: horizontal or vertical.",
                        ),
                        OptionSpec(
                            name="-sashwidth",
                            takes_value=True,
                            detail="Width of each sash in screen units.",
                        ),
                        OptionSpec(
                            name="-sashrelief",
                            takes_value=True,
                            detail="Relief of the sashes: flat, groove, raised, ridge, solid, or sunken.",
                        ),
                        OptionSpec(
                            name="-sashpad",
                            takes_value=True,
                            detail="Extra padding on each side of a sash.",
                        ),
                        OptionSpec(
                            name="-sashcursor",
                            takes_value=True,
                            detail="Cursor to display when the mouse is over a sash.",
                        ),
                        OptionSpec(
                            name="-showhandle",
                            takes_value=True,
                            detail="Whether to display handles on the sashes.",
                        ),
                        OptionSpec(
                            name="-handlesize",
                            takes_value=True,
                            detail="Size of the sash handle in screen units.",
                        ),
                        OptionSpec(
                            name="-handlepad",
                            takes_value=True,
                            detail="Distance from the top or left end of a sash to the handle.",
                        ),
                        OptionSpec(
                            name="-opaqueresize",
                            takes_value=True,
                            detail="Whether panes are resized continuously as the sash is dragged.",
                        ),
                        OptionSpec(
                            name="-width",
                            takes_value=True,
                            detail="Desired width of the panedwindow in screen units.",
                        ),
                        OptionSpec(
                            name="-height",
                            takes_value=True,
                            detail="Desired height of the panedwindow in screen units.",
                        ),
                        OptionSpec(
                            name="-bg",
                            takes_value=True,
                            detail="Shorthand for -background.",
                        ),
                        OptionSpec(
                            name="-background",
                            takes_value=True,
                            detail="Background colour of the panedwindow.",
                        ),
                        OptionSpec(
                            name="-relief",
                            takes_value=True,
                            detail="3-D effect: flat, groove, raised, ridge, solid, or sunken.",
                        ),
                        OptionSpec(
                            name="-borderwidth",
                            takes_value=True,
                            detail="Width of the border around the panedwindow.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            detail="Cursor to display when the mouse is over the panedwindow.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            detail="Whether the panedwindow accepts focus during keyboard traversal.",
                        ),
                        OptionSpec(
                            name="-highlightbackground",
                            takes_value=True,
                            detail="Colour of the highlight region when the panedwindow does not have focus.",
                        ),
                        OptionSpec(
                            name="-highlightcolor",
                            takes_value=True,
                            detail="Colour of the highlight region when the panedwindow has focus.",
                        ),
                        OptionSpec(
                            name="-highlightthickness",
                            takes_value=True,
                            detail="Width of the highlight rectangle drawn around the panedwindow.",
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "add",
                                "Add one or more child windows to the panedwindow.",
                                "pathName add window ?window ...? ?option value ...?",
                            ),
                            _av(
                                "forget",
                                "Remove the specified pane from the panedwindow.",
                                "pathName forget window",
                            ),
                            _av(
                                "identify",
                                "Identify the panedwindow component at the given coordinates.",
                                "pathName identify x y",
                            ),
                            _av(
                                "proxy",
                                "Query or manipulate the sash-drag proxy.",
                                "pathName proxy ?args?",
                            ),
                            _av(
                                "paneconfigure",
                                "Query or modify configuration options of a pane.",
                                "pathName paneconfigure window ?option? ?value option value ...?",
                            ),
                            _av(
                                "panecget",
                                "Return the value of a configuration option for a pane.",
                                "pathName panecget window option",
                            ),
                            _av(
                                "panes",
                                "Return a list of all panes in the panedwindow.",
                                "pathName panes",
                            ),
                            _av(
                                "sash",
                                "Query or adjust sash positions.",
                                "pathName sash option ?arg ...?",
                            ),
                        ),
                    },
                ),
            ),
            subcommands={
                "add": SubCommand(
                    name="add",
                    arity=Arity(1),
                    detail="Add one or more child windows to the panedwindow.",
                    synopsis="pathName add window ?window ...? ?option value ...?",
                ),
                "forget": SubCommand(
                    name="forget",
                    arity=Arity(1, 1),
                    detail="Remove the specified pane from the panedwindow.",
                    synopsis="pathName forget window",
                ),
                "identify": SubCommand(
                    name="identify",
                    arity=Arity(2, 2),
                    detail="Identify the panedwindow component at the given coordinates.",
                    synopsis="pathName identify x y",
                ),
                "panecget": SubCommand(
                    name="panecget",
                    arity=Arity(2, 2),
                    detail="Return the value of a configuration option for a pane.",
                    synopsis="pathName panecget window option",
                ),
                "paneconfigure": SubCommand(
                    name="paneconfigure",
                    arity=Arity(1),
                    detail="Query or modify configuration options of a pane.",
                    synopsis="pathName paneconfigure window ?option? ?value option value ...?",
                ),
                "panes": SubCommand(
                    name="panes",
                    arity=Arity(0, 0),
                    detail="Return a list of all panes in the panedwindow.",
                    synopsis="pathName panes",
                ),
                "proxy": SubCommand(
                    name="proxy",
                    arity=Arity(0),
                    detail="Query or manipulate the sash-drag proxy.",
                    synopsis="pathName proxy ?args?",
                ),
                "sash": SubCommand(
                    name="sash",
                    arity=Arity(1),
                    detail="Query or adjust sash positions.",
                    synopsis="pathName sash option ?arg ...?",
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
