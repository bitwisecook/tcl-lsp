"""menu -- Create and manipulate menu widgets."""

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

_SOURCE = "Tk man page menu.n"
_av = make_av(_SOURCE)

_ENTRY_TYPES = (
    _av("cascade", "A cascade entry that posts another menu."),
    _av("checkbutton", "A checkbutton entry with an on/off indicator."),
    _av("command", "A command entry that invokes a Tcl command."),
    _av("radiobutton", "A radiobutton entry with a mutual-exclusion indicator."),
    _av("separator", "A separator line between groups of entries."),
)


@register
class MenuCommand(CommandDef):
    name = "menu"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="menu",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Create and manipulate a menu widget.",
                synopsis=("menu pathName ?option value ...?",),
                snippet=(
                    "Displays a menu of commands, each of which may be "
                    "a cascade, checkbutton, command, radiobutton, or separator entry."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="menu pathName ?option value ...?",
                    options=(
                        OptionSpec(
                            name="-tearoff",
                            takes_value=True,
                            detail="Whether the menu should include a tear-off entry at the top.",
                        ),
                        OptionSpec(
                            name="-title",
                            takes_value=True,
                            detail="Title string for the tear-off menu window.",
                        ),
                        OptionSpec(
                            name="-type",
                            takes_value=True,
                            detail="Type of the menu: menubar, tearoff, or normal.",
                        ),
                        OptionSpec(
                            name="-bg",
                            takes_value=True,
                            detail="Shorthand for -background.",
                        ),
                        OptionSpec(
                            name="-background",
                            takes_value=True,
                            detail="Background colour of the menu.",
                        ),
                        OptionSpec(
                            name="-fg",
                            takes_value=True,
                            detail="Shorthand for -foreground.",
                        ),
                        OptionSpec(
                            name="-foreground",
                            takes_value=True,
                            detail="Foreground colour for menu entries.",
                        ),
                        OptionSpec(
                            name="-font",
                            takes_value=True,
                            detail="Font to use for text in the menu.",
                        ),
                        OptionSpec(
                            name="-activebackground",
                            takes_value=True,
                            detail="Background colour for the active menu entry.",
                        ),
                        OptionSpec(
                            name="-activeforeground",
                            takes_value=True,
                            detail="Foreground colour for the active menu entry.",
                        ),
                        OptionSpec(
                            name="-activeborderwidth",
                            takes_value=True,
                            detail="Width of the border drawn around active entries.",
                        ),
                        OptionSpec(
                            name="-disabledforeground",
                            takes_value=True,
                            detail="Foreground colour for disabled menu entries.",
                        ),
                        OptionSpec(
                            name="-selectcolor",
                            takes_value=True,
                            detail="Colour of the indicator for checkbutton and radiobutton entries.",
                        ),
                        OptionSpec(
                            name="-relief",
                            takes_value=True,
                            detail="3-D effect: flat, groove, raised, ridge, solid, or sunken.",
                        ),
                        OptionSpec(
                            name="-borderwidth",
                            takes_value=True,
                            detail="Width of the border around the menu.",
                        ),
                        OptionSpec(
                            name="-postcommand",
                            takes_value=True,
                            detail="Tcl command to invoke just before the menu is posted.",
                        ),
                        OptionSpec(
                            name="-tearoffcommand",
                            takes_value=True,
                            detail="Tcl command to invoke when the menu is torn off.",
                        ),
                        OptionSpec(
                            name="-cursor",
                            takes_value=True,
                            detail="Cursor to display when the mouse is over the menu.",
                        ),
                        OptionSpec(
                            name="-takefocus",
                            takes_value=True,
                            detail="Whether the menu accepts focus during keyboard traversal.",
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "add",
                                "Add a new entry to the bottom of the menu.",
                                "pathName add type ?option value ...?",
                            ),
                            _av(
                                "delete",
                                "Delete menu entries between index1 and index2 inclusive.",
                                "pathName delete index1 ?index2?",
                            ),
                            _av(
                                "entryconfigure",
                                "Query or modify options of a menu entry.",
                                "pathName entryconfigure index ?option value ...?",
                            ),
                            _av(
                                "entrycget",
                                "Return the value of a configuration option for a menu entry.",
                                "pathName entrycget index option",
                            ),
                            _av(
                                "index",
                                "Return the numerical index corresponding to the given index.",
                                "pathName index index",
                            ),
                            _av(
                                "insert",
                                "Insert a new entry before the entry at the given index.",
                                "pathName insert index type ?option value ...?",
                            ),
                            _av(
                                "invoke",
                                "Invoke the action of the menu entry at the given index.",
                                "pathName invoke index",
                            ),
                            _av(
                                "post",
                                "Display the menu at the given screen coordinates.",
                                "pathName post x y",
                            ),
                            _av(
                                "postcascade",
                                "Post the submenu associated with the cascade entry at the given index.",
                                "pathName postcascade index",
                            ),
                            _av(
                                "type",
                                "Return the type of the menu entry at the given index.",
                                "pathName type index",
                            ),
                            _av(
                                "unpost",
                                "Unmap the menu so it is no longer displayed.",
                                "pathName unpost",
                            ),
                            _av(
                                "clone",
                                "Create a clone of this menu.",
                                "pathName clone newPathname ?cloneType?",
                            ),
                            _av(
                                "yposition",
                                "Return the y-coordinate of the topmost pixel of the entry at the given index.",
                                "pathName yposition index",
                            ),
                        ),
                    },
                ),
            ),
            subcommands={
                "add": SubCommand(
                    name="add",
                    arity=Arity(1),
                    detail="Add a new entry to the bottom of the menu.",
                    synopsis="pathName add type ?option value ...?",
                    arg_values={0: _ENTRY_TYPES},
                ),
                "clone": SubCommand(
                    name="clone",
                    arity=Arity(1, 2),
                    detail="Create a clone of this menu.",
                    synopsis="pathName clone newPathname ?cloneType?",
                ),
                "delete": SubCommand(
                    name="delete",
                    arity=Arity(1, 2),
                    detail="Delete menu entries between index1 and index2 inclusive.",
                    synopsis="pathName delete index1 ?index2?",
                ),
                "entrycget": SubCommand(
                    name="entrycget",
                    arity=Arity(2, 2),
                    detail="Return the value of a configuration option for a menu entry.",
                    synopsis="pathName entrycget index option",
                ),
                "entryconfigure": SubCommand(
                    name="entryconfigure",
                    arity=Arity(1),
                    detail="Query or modify options of a menu entry.",
                    synopsis="pathName entryconfigure index ?option value ...?",
                ),
                "index": SubCommand(
                    name="index",
                    arity=Arity(1, 1),
                    detail="Return the numerical index corresponding to the given index.",
                    synopsis="pathName index index",
                ),
                "insert": SubCommand(
                    name="insert",
                    arity=Arity(2),
                    detail="Insert a new entry before the entry at the given index.",
                    synopsis="pathName insert index type ?option value ...?",
                    arg_values={1: _ENTRY_TYPES},
                ),
                "invoke": SubCommand(
                    name="invoke",
                    arity=Arity(1, 1),
                    detail="Invoke the action of the menu entry at the given index.",
                    synopsis="pathName invoke index",
                ),
                "post": SubCommand(
                    name="post",
                    arity=Arity(2, 2),
                    detail="Display the menu at the given screen coordinates.",
                    synopsis="pathName post x y",
                ),
                "postcascade": SubCommand(
                    name="postcascade",
                    arity=Arity(1, 1),
                    detail="Post the submenu associated with the cascade entry at the given index.",
                    synopsis="pathName postcascade index",
                ),
                "type": SubCommand(
                    name="type",
                    arity=Arity(1, 1),
                    detail="Return the type of the menu entry at the given index.",
                    synopsis="pathName type index",
                ),
                "unpost": SubCommand(
                    name="unpost",
                    arity=Arity(0, 0),
                    detail="Unmap the menu so it is no longer displayed.",
                    synopsis="pathName unpost",
                ),
                "yposition": SubCommand(
                    name="yposition",
                    arity=Arity(1, 1),
                    detail="Return the y-coordinate of the topmost pixel of the entry at the given index.",
                    synopsis="pathName yposition index",
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
