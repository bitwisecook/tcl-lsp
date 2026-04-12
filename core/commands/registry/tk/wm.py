"""wm -- Communicate with the window manager."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "Tk man page wm.n"
_av = make_av(_SOURCE)


@register
class WmCommand(CommandDef):
    name = "wm"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="wm",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Communicate with the window manager.",
                synopsis=(
                    "wm aspect window ?minNumer minDenom maxNumer maxDenom?",
                    "wm attributes window ?option? ?value ...?",
                    "wm client window ?name?",
                    "wm colormapwindows window ?windowList?",
                    "wm command window ?value?",
                    "wm deiconify window",
                    "wm focusmodel window ?active|passive?",
                    "wm forget window",
                    "wm frame window",
                    "wm geometry window ?newGeometry?",
                    "wm grid window ?baseWidth baseHeight widthInc heightInc?",
                    "wm group window ?pathName?",
                    "wm iconbitmap window ?bitmap?",
                    "wm iconify window",
                    "wm iconmask window ?bitmap?",
                    "wm iconname window ?newName?",
                    "wm iconphoto window ?-default? image1 ?image2 ...?",
                    "wm iconposition window ?x y?",
                    "wm iconwindow window ?pathName?",
                    "wm manage window",
                    "wm maxsize window ?width height?",
                    "wm minsize window ?width height?",
                    "wm overrideredirect window ?boolean?",
                    "wm positionfrom window ?who?",
                    "wm protocol window ?name? ?command?",
                    "wm resizable window ?width height?",
                    "wm sizefrom window ?who?",
                    "wm stackorder window ?isabove|isbelow window?",
                    "wm state window ?newstate?",
                    "wm title window ?string?",
                    "wm transient window ?master?",
                    "wm withdraw window",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="wm option window ?arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "aspect",
                                "Set or query the desired aspect ratio for the window.",
                                "wm aspect window ?minNumer minDenom maxNumer maxDenom?",
                            ),
                            _av(
                                "attributes",
                                "Set or query platform-specific window attributes.",
                                "wm attributes window ?option? ?value ...?",
                            ),
                            _av(
                                "client",
                                "Set or query the WM_CLIENT_MACHINE property.",
                                "wm client window ?name?",
                            ),
                            _av(
                                "colormapwindows",
                                "Set or query the WM_COLORMAP_WINDOWS property.",
                                "wm colormapwindows window ?windowList?",
                            ),
                            _av(
                                "command",
                                "Set or query the WM_COMMAND property.",
                                "wm command window ?value?",
                            ),
                            _av(
                                "deiconify",
                                "Arrange for the window to be displayed in normal form.",
                                "wm deiconify window",
                            ),
                            _av(
                                "focusmodel",
                                "Set or query the focus model for the window.",
                                "wm focusmodel window ?active|passive?",
                            ),
                            _av(
                                "forget",
                                "Make the window no longer managed by wm.",
                                "wm forget window",
                            ),
                            _av(
                                "frame",
                                "Return the platform-specific window identifier of the outermost frame.",
                                "wm frame window",
                            ),
                            _av(
                                "geometry",
                                "Set or query the geometry of the window.",
                                "wm geometry window ?newGeometry?",
                            ),
                            _av(
                                "grid",
                                "Set or query the gridding information for the window.",
                                "wm grid window ?baseWidth baseHeight widthInc heightInc?",
                            ),
                            _av(
                                "group",
                                "Set or query the group leader for the window.",
                                "wm group window ?pathName?",
                            ),
                            _av(
                                "iconbitmap",
                                "Set or query the bitmap for when the window is iconified.",
                                "wm iconbitmap window ?bitmap?",
                            ),
                            _av(
                                "iconify",
                                "Arrange for the window to be iconified.",
                                "wm iconify window",
                            ),
                            _av(
                                "iconmask",
                                "Set or query the icon mask bitmap.",
                                "wm iconmask window ?bitmap?",
                            ),
                            _av(
                                "iconname",
                                "Set or query the icon name for the window.",
                                "wm iconname window ?newName?",
                            ),
                            _av(
                                "iconphoto",
                                "Set the window icon from one or more photo images.",
                                "wm iconphoto window ?-default? image1 ?image2 ...?",
                            ),
                            _av(
                                "iconposition",
                                "Set or query the icon position hint.",
                                "wm iconposition window ?x y?",
                            ),
                            _av(
                                "iconwindow",
                                "Set or query the icon window for the window.",
                                "wm iconwindow window ?pathName?",
                            ),
                            _av(
                                "manage",
                                "Make the window managed by wm.",
                                "wm manage window",
                            ),
                            _av(
                                "maxsize",
                                "Set or query the maximum size for the window.",
                                "wm maxsize window ?width height?",
                            ),
                            _av(
                                "minsize",
                                "Set or query the minimum size for the window.",
                                "wm minsize window ?width height?",
                            ),
                            _av(
                                "overrideredirect",
                                "Set or query the override-redirect flag.",
                                "wm overrideredirect window ?boolean?",
                            ),
                            _av(
                                "positionfrom",
                                "Set or query who specified the window position.",
                                "wm positionfrom window ?who?",
                            ),
                            _av(
                                "protocol",
                                "Register a handler for a window manager protocol.",
                                "wm protocol window ?name? ?command?",
                            ),
                            _av(
                                "resizable",
                                "Set or query whether the window is resizable.",
                                "wm resizable window ?width height?",
                            ),
                            _av(
                                "sizefrom",
                                "Set or query who specified the window size.",
                                "wm sizefrom window ?who?",
                            ),
                            _av(
                                "stackorder",
                                "Return the stacking order or compare two windows.",
                                "wm stackorder window ?isabove|isbelow window?",
                            ),
                            _av(
                                "state",
                                "Set or query the current state of the window.",
                                "wm state window ?newstate?",
                            ),
                            _av(
                                "title",
                                "Set or query the title of the window.",
                                "wm title window ?string?",
                            ),
                            _av(
                                "transient",
                                "Set or query the transient master for the window.",
                                "wm transient window ?master?",
                            ),
                            _av(
                                "withdraw",
                                "Withdraw the window so it is no longer mapped.",
                                "wm withdraw window",
                            ),
                        ),
                    },
                ),
            ),
            subcommands={
                "aspect": SubCommand(
                    name="aspect",
                    arity=Arity(1, 5),
                    detail="Set or query the desired aspect ratio for the window.",
                    synopsis="wm aspect window ?minNumer minDenom maxNumer maxDenom?",
                ),
                "attributes": SubCommand(
                    name="attributes",
                    arity=Arity(1),
                    detail="Set or query platform-specific window attributes.",
                    synopsis="wm attributes window ?option? ?value ...?",
                ),
                "client": SubCommand(
                    name="client",
                    arity=Arity(1, 2),
                    detail="Set or query the WM_CLIENT_MACHINE property.",
                    synopsis="wm client window ?name?",
                ),
                "colormapwindows": SubCommand(
                    name="colormapwindows",
                    arity=Arity(1, 2),
                    detail="Set or query the WM_COLORMAP_WINDOWS property.",
                    synopsis="wm colormapwindows window ?windowList?",
                ),
                "command": SubCommand(
                    name="command",
                    arity=Arity(1, 2),
                    detail="Set or query the WM_COMMAND property.",
                    synopsis="wm command window ?value?",
                ),
                "deiconify": SubCommand(
                    name="deiconify",
                    arity=Arity(1, 1),
                    detail="Arrange for the window to be displayed in normal form.",
                    synopsis="wm deiconify window",
                ),
                "focusmodel": SubCommand(
                    name="focusmodel",
                    arity=Arity(1, 2),
                    detail="Set or query the focus model for the window.",
                    synopsis="wm focusmodel window ?active|passive?",
                ),
                "forget": SubCommand(
                    name="forget",
                    arity=Arity(1, 1),
                    detail="Make the window no longer managed by wm.",
                    synopsis="wm forget window",
                ),
                "frame": SubCommand(
                    name="frame",
                    arity=Arity(1, 1),
                    detail="Return the platform-specific window identifier of the outermost frame.",
                    synopsis="wm frame window",
                ),
                "geometry": SubCommand(
                    name="geometry",
                    arity=Arity(1, 2),
                    detail="Set or query the geometry of the window.",
                    synopsis="wm geometry window ?newGeometry?",
                ),
                "grid": SubCommand(
                    name="grid",
                    arity=Arity(1, 5),
                    detail="Set or query the gridding information for the window.",
                    synopsis="wm grid window ?baseWidth baseHeight widthInc heightInc?",
                ),
                "group": SubCommand(
                    name="group",
                    arity=Arity(1, 2),
                    detail="Set or query the group leader for the window.",
                    synopsis="wm group window ?pathName?",
                ),
                "iconbitmap": SubCommand(
                    name="iconbitmap",
                    arity=Arity(1, 2),
                    detail="Set or query the bitmap for when the window is iconified.",
                    synopsis="wm iconbitmap window ?bitmap?",
                ),
                "iconify": SubCommand(
                    name="iconify",
                    arity=Arity(1, 1),
                    detail="Arrange for the window to be iconified.",
                    synopsis="wm iconify window",
                ),
                "iconmask": SubCommand(
                    name="iconmask",
                    arity=Arity(1, 2),
                    detail="Set or query the icon mask bitmap.",
                    synopsis="wm iconmask window ?bitmap?",
                ),
                "iconname": SubCommand(
                    name="iconname",
                    arity=Arity(1, 2),
                    detail="Set or query the icon name for the window.",
                    synopsis="wm iconname window ?newName?",
                ),
                "iconphoto": SubCommand(
                    name="iconphoto",
                    arity=Arity(1),
                    detail="Set the window icon from one or more photo images.",
                    synopsis="wm iconphoto window ?-default? image1 ?image2 ...?",
                ),
                "iconposition": SubCommand(
                    name="iconposition",
                    arity=Arity(1, 3),
                    detail="Set or query the icon position hint.",
                    synopsis="wm iconposition window ?x y?",
                ),
                "iconwindow": SubCommand(
                    name="iconwindow",
                    arity=Arity(1, 2),
                    detail="Set or query the icon window for the window.",
                    synopsis="wm iconwindow window ?pathName?",
                ),
                "manage": SubCommand(
                    name="manage",
                    arity=Arity(1, 1),
                    detail="Make the window managed by wm.",
                    synopsis="wm manage window",
                ),
                "maxsize": SubCommand(
                    name="maxsize",
                    arity=Arity(1, 3),
                    detail="Set or query the maximum size for the window.",
                    synopsis="wm maxsize window ?width height?",
                ),
                "minsize": SubCommand(
                    name="minsize",
                    arity=Arity(1, 3),
                    detail="Set or query the minimum size for the window.",
                    synopsis="wm minsize window ?width height?",
                ),
                "overrideredirect": SubCommand(
                    name="overrideredirect",
                    arity=Arity(1, 2),
                    detail="Set or query the override-redirect flag.",
                    synopsis="wm overrideredirect window ?boolean?",
                ),
                "positionfrom": SubCommand(
                    name="positionfrom",
                    arity=Arity(1, 2),
                    detail="Set or query who specified the window position.",
                    synopsis="wm positionfrom window ?who?",
                ),
                "protocol": SubCommand(
                    name="protocol",
                    arity=Arity(1, 3),
                    detail="Register a handler for a window manager protocol.",
                    synopsis="wm protocol window ?name? ?command?",
                ),
                "resizable": SubCommand(
                    name="resizable",
                    arity=Arity(1, 3),
                    detail="Set or query whether the window is resizable.",
                    synopsis="wm resizable window ?width height?",
                ),
                "sizefrom": SubCommand(
                    name="sizefrom",
                    arity=Arity(1, 2),
                    detail="Set or query who specified the window size.",
                    synopsis="wm sizefrom window ?who?",
                ),
                "stackorder": SubCommand(
                    name="stackorder",
                    arity=Arity(1, 3),
                    detail="Return the stacking order or compare two windows.",
                    synopsis="wm stackorder window ?isabove|isbelow window?",
                ),
                "state": SubCommand(
                    name="state",
                    arity=Arity(1, 2),
                    detail="Set or query the current state of the window.",
                    synopsis="wm state window ?newstate?",
                ),
                "title": SubCommand(
                    name="title",
                    arity=Arity(1, 2),
                    detail="Set or query the title of the window.",
                    synopsis="wm title window ?string?",
                ),
                "transient": SubCommand(
                    name="transient",
                    arity=Arity(1, 2),
                    detail="Set or query the transient master for the window.",
                    synopsis="wm transient window ?master?",
                ),
                "withdraw": SubCommand(
                    name="withdraw",
                    arity=Arity(1, 1),
                    detail="Withdraw the window so it is no longer mapped.",
                    synopsis="wm withdraw window",
                ),
            },
            validation=ValidationSpec(
                arity=Arity(2),
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
