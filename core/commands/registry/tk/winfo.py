"""winfo -- Return window-related information."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page winfo.n"
_av = make_av(_SOURCE)


@register
class WinfoCommand(CommandDef):
    name = "winfo"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="winfo",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Return window-related information.",
                synopsis=(
                    "winfo atom ?-displayof window? name",
                    "winfo atomname ?-displayof window? id",
                    "winfo cells window",
                    "winfo children window",
                    "winfo class window",
                    "winfo colormapfull window",
                    "winfo containing ?-displayof window? rootX rootY",
                    "winfo depth window",
                    "winfo exists window",
                    "winfo geometry window",
                    "winfo height window",
                    "winfo id window",
                    "winfo ismapped window",
                    "winfo manager window",
                    "winfo name window",
                    "winfo parent window",
                    "winfo pathname ?-displayof window? id",
                    "winfo pixels window number",
                    "winfo reqheight window",
                    "winfo reqwidth window",
                    "winfo rootx window",
                    "winfo rooty window",
                    "winfo screen window",
                    "winfo screenheight window",
                    "winfo screenwidth window",
                    "winfo toplevel window",
                    "winfo viewable window",
                    "winfo visual window",
                    "winfo width window",
                    "winfo x window",
                    "winfo y window",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="winfo option ?arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "atom",
                                "Return the integer identifier for the atom given by name.",
                                "winfo atom ?-displayof window? name",
                            ),
                            _av(
                                "atomname",
                                "Return the textual name for the atom given by integer id.",
                                "winfo atomname ?-displayof window? id",
                            ),
                            _av(
                                "cells",
                                "Return the number of cells in the colour map for the window.",
                                "winfo cells window",
                            ),
                            _av(
                                "children",
                                "Return a list of the path names of the window's children.",
                                "winfo children window",
                            ),
                            _av(
                                "class",
                                "Return the class name of the window.",
                                "winfo class window",
                            ),
                            _av(
                                "colormapfull",
                                "Return 1 if the colour map for the window is full, 0 otherwise.",
                                "winfo colormapfull window",
                            ),
                            _av(
                                "containing",
                                "Return the path name of the window containing the point.",
                                "winfo containing ?-displayof window? rootX rootY",
                            ),
                            _av(
                                "depth",
                                "Return the number of bits per pixel in the window.",
                                "winfo depth window",
                            ),
                            _av(
                                "exists",
                                "Return 1 if the window exists, 0 otherwise.",
                                "winfo exists window",
                            ),
                            _av(
                                "fpixels",
                                "Return the number of pixels corresponding to the distance as a float.",
                                "winfo fpixels window number",
                            ),
                            _av(
                                "geometry",
                                "Return the geometry of the window in WxH+X+Y format.",
                                "winfo geometry window",
                            ),
                            _av(
                                "height",
                                "Return the height of the window in pixels.",
                                "winfo height window",
                            ),
                            _av(
                                "id",
                                "Return the platform-specific window identifier.",
                                "winfo id window",
                            ),
                            _av(
                                "interps",
                                "Return a list of all Tcl interpreters on the display.",
                                "winfo interps ?-displayof window?",
                            ),
                            _av(
                                "ismapped",
                                "Return 1 if the window is currently mapped, 0 otherwise.",
                                "winfo ismapped window",
                            ),
                            _av(
                                "manager",
                                "Return the name of the geometry manager for the window.",
                                "winfo manager window",
                            ),
                            _av(
                                "name",
                                "Return the window's name (last component of its path).",
                                "winfo name window",
                            ),
                            _av(
                                "parent",
                                "Return the path name of the window's parent.",
                                "winfo parent window",
                            ),
                            _av(
                                "pathname",
                                "Return the path name of the window whose identifier is id.",
                                "winfo pathname ?-displayof window? id",
                            ),
                            _av(
                                "pixels",
                                "Return the number of pixels corresponding to the distance.",
                                "winfo pixels window number",
                            ),
                            _av(
                                "pointerx",
                                "Return the x-coordinate of the mouse pointer on the screen.",
                                "winfo pointerx window",
                            ),
                            _av(
                                "pointery",
                                "Return the y-coordinate of the mouse pointer on the screen.",
                                "winfo pointery window",
                            ),
                            _av(
                                "pointerxy",
                                "Return the x and y coordinates of the mouse pointer.",
                                "winfo pointerxy window",
                            ),
                            _av(
                                "reqheight",
                                "Return the requested height of the window in pixels.",
                                "winfo reqheight window",
                            ),
                            _av(
                                "reqwidth",
                                "Return the requested width of the window in pixels.",
                                "winfo reqwidth window",
                            ),
                            _av(
                                "rgb",
                                "Return the RGB values for a colour in the window.",
                                "winfo rgb window color",
                            ),
                            _av(
                                "rootx",
                                "Return the x-coordinate of the upper-left corner in the root window.",
                                "winfo rootx window",
                            ),
                            _av(
                                "rooty",
                                "Return the y-coordinate of the upper-left corner in the root window.",
                                "winfo rooty window",
                            ),
                            _av(
                                "screen",
                                "Return the display and screen of the window.",
                                "winfo screen window",
                            ),
                            _av(
                                "screencells",
                                "Return the number of cells in the default colour map for the screen.",
                                "winfo screencells window",
                            ),
                            _av(
                                "screendepth",
                                "Return the number of bits per pixel on the screen.",
                                "winfo screendepth window",
                            ),
                            _av(
                                "screenheight",
                                "Return the height of the screen in pixels.",
                                "winfo screenheight window",
                            ),
                            _av(
                                "screenmmheight",
                                "Return the height of the screen in millimetres.",
                                "winfo screenmmheight window",
                            ),
                            _av(
                                "screenmmwidth",
                                "Return the width of the screen in millimetres.",
                                "winfo screenmmwidth window",
                            ),
                            _av(
                                "screenvisual",
                                "Return the visual class of the screen.",
                                "winfo screenvisual window",
                            ),
                            _av(
                                "screenwidth",
                                "Return the width of the screen in pixels.",
                                "winfo screenwidth window",
                            ),
                            _av(
                                "server",
                                "Return information about the display server.",
                                "winfo server window",
                            ),
                            _av(
                                "toplevel",
                                "Return the path name of the top-level window containing the window.",
                                "winfo toplevel window",
                            ),
                            _av(
                                "viewable",
                                "Return 1 if the window and all ancestors are mapped.",
                                "winfo viewable window",
                            ),
                            _av(
                                "visual",
                                "Return the visual class of the window.",
                                "winfo visual window",
                            ),
                            _av(
                                "visualid",
                                "Return the X identifier for the visual of the window.",
                                "winfo visualid window",
                            ),
                            _av(
                                "visualsavailable",
                                "Return a list of available visual classes for the screen.",
                                "winfo visualsavailable window ?includeids?",
                            ),
                            _av(
                                "vrootheight",
                                "Return the height of the virtual root window.",
                                "winfo vrootheight window",
                            ),
                            _av(
                                "vrootwidth",
                                "Return the width of the virtual root window.",
                                "winfo vrootwidth window",
                            ),
                            _av(
                                "vrootx",
                                "Return the x offset of the virtual root window.",
                                "winfo vrootx window",
                            ),
                            _av(
                                "vrooty",
                                "Return the y offset of the virtual root window.",
                                "winfo vrooty window",
                            ),
                            _av(
                                "width",
                                "Return the width of the window in pixels.",
                                "winfo width window",
                            ),
                            _av(
                                "x",
                                "Return the x-coordinate of the upper-left corner of the window.",
                                "winfo x window",
                            ),
                            _av(
                                "y",
                                "Return the y-coordinate of the upper-left corner of the window.",
                                "winfo y window",
                            ),
                        ),
                    },
                ),
            ),
            subcommands={
                "atom": SubCommand(
                    name="atom",
                    arity=Arity(1, 3),
                    detail="Return the integer identifier for the atom given by name.",
                    synopsis="winfo atom ?-displayof window? name",
                ),
                "atomname": SubCommand(
                    name="atomname",
                    arity=Arity(1, 3),
                    detail="Return the textual name for the atom given by integer id.",
                    synopsis="winfo atomname ?-displayof window? id",
                ),
                "cells": SubCommand(
                    name="cells",
                    arity=Arity(1, 1),
                    detail="Return the number of cells in the colour map for the window.",
                    synopsis="winfo cells window",
                ),
                "children": SubCommand(
                    name="children",
                    arity=Arity(1, 1),
                    detail="Return a list of the path names of the window's children.",
                    synopsis="winfo children window",
                ),
                "class": SubCommand(
                    name="class",
                    arity=Arity(1, 1),
                    detail="Return the class name of the window.",
                    synopsis="winfo class window",
                ),
                "colormapfull": SubCommand(
                    name="colormapfull",
                    arity=Arity(1, 1),
                    detail="Return 1 if the colour map for the window is full, 0 otherwise.",
                    synopsis="winfo colormapfull window",
                ),
                "containing": SubCommand(
                    name="containing",
                    arity=Arity(2, 4),
                    detail="Return the path name of the window containing the point.",
                    synopsis="winfo containing ?-displayof window? rootX rootY",
                ),
                "depth": SubCommand(
                    name="depth",
                    arity=Arity(1, 1),
                    detail="Return the number of bits per pixel in the window.",
                    synopsis="winfo depth window",
                ),
                "exists": SubCommand(
                    name="exists",
                    arity=Arity(1, 1),
                    detail="Return 1 if the window exists, 0 otherwise.",
                    synopsis="winfo exists window",
                ),
                "fpixels": SubCommand(
                    name="fpixels",
                    arity=Arity(2, 2),
                    detail="Return the number of pixels corresponding to the distance as a float.",
                    synopsis="winfo fpixels window number",
                ),
                "geometry": SubCommand(
                    name="geometry",
                    arity=Arity(1, 1),
                    detail="Return the geometry of the window in WxH+X+Y format.",
                    synopsis="winfo geometry window",
                ),
                "height": SubCommand(
                    name="height",
                    arity=Arity(1, 1),
                    detail="Return the height of the window in pixels.",
                    synopsis="winfo height window",
                ),
                "id": SubCommand(
                    name="id",
                    arity=Arity(1, 1),
                    detail="Return the platform-specific window identifier.",
                    synopsis="winfo id window",
                ),
                "interps": SubCommand(
                    name="interps",
                    arity=Arity(0, 2),
                    detail="Return a list of all Tcl interpreters on the display.",
                    synopsis="winfo interps ?-displayof window?",
                ),
                "ismapped": SubCommand(
                    name="ismapped",
                    arity=Arity(1, 1),
                    detail="Return 1 if the window is currently mapped, 0 otherwise.",
                    synopsis="winfo ismapped window",
                ),
                "manager": SubCommand(
                    name="manager",
                    arity=Arity(1, 1),
                    detail="Return the name of the geometry manager for the window.",
                    synopsis="winfo manager window",
                ),
                "name": SubCommand(
                    name="name",
                    arity=Arity(1, 1),
                    detail="Return the window's name (last component of its path).",
                    synopsis="winfo name window",
                ),
                "parent": SubCommand(
                    name="parent",
                    arity=Arity(1, 1),
                    detail="Return the path name of the window's parent.",
                    synopsis="winfo parent window",
                ),
                "pathname": SubCommand(
                    name="pathname",
                    arity=Arity(1, 3),
                    detail="Return the path name of the window whose identifier is id.",
                    synopsis="winfo pathname ?-displayof window? id",
                ),
                "pixels": SubCommand(
                    name="pixels",
                    arity=Arity(2, 2),
                    detail="Return the number of pixels corresponding to the distance.",
                    synopsis="winfo pixels window number",
                ),
                "pointerx": SubCommand(
                    name="pointerx",
                    arity=Arity(1, 1),
                    detail="Return the x-coordinate of the mouse pointer on the screen.",
                    synopsis="winfo pointerx window",
                ),
                "pointerxy": SubCommand(
                    name="pointerxy",
                    arity=Arity(1, 1),
                    detail="Return the x and y coordinates of the mouse pointer.",
                    synopsis="winfo pointerxy window",
                ),
                "pointery": SubCommand(
                    name="pointery",
                    arity=Arity(1, 1),
                    detail="Return the y-coordinate of the mouse pointer on the screen.",
                    synopsis="winfo pointery window",
                ),
                "reqheight": SubCommand(
                    name="reqheight",
                    arity=Arity(1, 1),
                    detail="Return the requested height of the window in pixels.",
                    synopsis="winfo reqheight window",
                ),
                "reqwidth": SubCommand(
                    name="reqwidth",
                    arity=Arity(1, 1),
                    detail="Return the requested width of the window in pixels.",
                    synopsis="winfo reqwidth window",
                ),
                "rgb": SubCommand(
                    name="rgb",
                    arity=Arity(2, 2),
                    detail="Return the RGB values for a colour in the window.",
                    synopsis="winfo rgb window color",
                ),
                "rootx": SubCommand(
                    name="rootx",
                    arity=Arity(1, 1),
                    detail="Return the x-coordinate of the upper-left corner in the root window.",
                    synopsis="winfo rootx window",
                ),
                "rooty": SubCommand(
                    name="rooty",
                    arity=Arity(1, 1),
                    detail="Return the y-coordinate of the upper-left corner in the root window.",
                    synopsis="winfo rooty window",
                ),
                "screen": SubCommand(
                    name="screen",
                    arity=Arity(1, 1),
                    detail="Return the display and screen of the window.",
                    synopsis="winfo screen window",
                ),
                "screencells": SubCommand(
                    name="screencells",
                    arity=Arity(1, 1),
                    detail="Return the number of cells in the default colour map for the screen.",
                    synopsis="winfo screencells window",
                ),
                "screendepth": SubCommand(
                    name="screendepth",
                    arity=Arity(1, 1),
                    detail="Return the number of bits per pixel on the screen.",
                    synopsis="winfo screendepth window",
                ),
                "screenheight": SubCommand(
                    name="screenheight",
                    arity=Arity(1, 1),
                    detail="Return the height of the screen in pixels.",
                    synopsis="winfo screenheight window",
                ),
                "screenmmheight": SubCommand(
                    name="screenmmheight",
                    arity=Arity(1, 1),
                    detail="Return the height of the screen in millimetres.",
                    synopsis="winfo screenmmheight window",
                ),
                "screenmmwidth": SubCommand(
                    name="screenmmwidth",
                    arity=Arity(1, 1),
                    detail="Return the width of the screen in millimetres.",
                    synopsis="winfo screenmmwidth window",
                ),
                "screenvisual": SubCommand(
                    name="screenvisual",
                    arity=Arity(1, 1),
                    detail="Return the visual class of the screen.",
                    synopsis="winfo screenvisual window",
                ),
                "screenwidth": SubCommand(
                    name="screenwidth",
                    arity=Arity(1, 1),
                    detail="Return the width of the screen in pixels.",
                    synopsis="winfo screenwidth window",
                ),
                "server": SubCommand(
                    name="server",
                    arity=Arity(1, 1),
                    detail="Return information about the display server.",
                    synopsis="winfo server window",
                ),
                "toplevel": SubCommand(
                    name="toplevel",
                    arity=Arity(1, 1),
                    detail="Return the path name of the top-level window containing the window.",
                    synopsis="winfo toplevel window",
                ),
                "viewable": SubCommand(
                    name="viewable",
                    arity=Arity(1, 1),
                    detail="Return 1 if the window and all ancestors are mapped.",
                    synopsis="winfo viewable window",
                ),
                "visual": SubCommand(
                    name="visual",
                    arity=Arity(1, 1),
                    detail="Return the visual class of the window.",
                    synopsis="winfo visual window",
                ),
                "visualid": SubCommand(
                    name="visualid",
                    arity=Arity(1, 1),
                    detail="Return the X identifier for the visual of the window.",
                    synopsis="winfo visualid window",
                ),
                "visualsavailable": SubCommand(
                    name="visualsavailable",
                    arity=Arity(1, 2),
                    detail="Return a list of available visual classes for the screen.",
                    synopsis="winfo visualsavailable window ?includeids?",
                ),
                "vrootheight": SubCommand(
                    name="vrootheight",
                    arity=Arity(1, 1),
                    detail="Return the height of the virtual root window.",
                    synopsis="winfo vrootheight window",
                ),
                "vrootwidth": SubCommand(
                    name="vrootwidth",
                    arity=Arity(1, 1),
                    detail="Return the width of the virtual root window.",
                    synopsis="winfo vrootwidth window",
                ),
                "vrootx": SubCommand(
                    name="vrootx",
                    arity=Arity(1, 1),
                    detail="Return the x offset of the virtual root window.",
                    synopsis="winfo vrootx window",
                ),
                "vrooty": SubCommand(
                    name="vrooty",
                    arity=Arity(1, 1),
                    detail="Return the y offset of the virtual root window.",
                    synopsis="winfo vrooty window",
                ),
                "width": SubCommand(
                    name="width",
                    arity=Arity(1, 1),
                    detail="Return the width of the window in pixels.",
                    synopsis="winfo width window",
                ),
                "x": SubCommand(
                    name="x",
                    arity=Arity(1, 1),
                    detail="Return the x-coordinate of the upper-left corner of the window.",
                    synopsis="winfo x window",
                ),
                "y": SubCommand(
                    name="y",
                    arity=Arity(1, 1),
                    detail="Return the y-coordinate of the upper-left corner of the window.",
                    synopsis="winfo y window",
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
