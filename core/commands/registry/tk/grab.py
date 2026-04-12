"""grab -- Confine pointer and keyboard events to a window sub-tree."""

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

_SOURCE = "Tk man page grab.n"
_av = make_av(_SOURCE)


@register
class GrabCommand(CommandDef):
    name = "grab"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="grab",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Confine pointer and keyboard events to a window sub-tree.",
                synopsis=(
                    "grab ?-global? window",
                    "grab current ?window?",
                    "grab release window",
                    "grab set ?-global? window",
                    "grab status window",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="grab option ?arg ...?",
                    options=(
                        OptionSpec(
                            name="-global",
                            takes_value=False,
                            detail="Make the grab global (applies to all displays).",
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "current",
                                "Return the path name of the current grab window, if any.",
                                "grab current ?window?",
                            ),
                            _av(
                                "release",
                                "Release the grab on the window.",
                                "grab release window",
                            ),
                            _av(
                                "set",
                                "Set a grab on the window, optionally global.",
                                "grab set ?-global? window",
                            ),
                            _av(
                                "status",
                                "Return the grab status of the window (none, local, or global).",
                                "grab status window",
                            ),
                        ),
                    },
                ),
            ),
            subcommands={
                "current": SubCommand(
                    name="current",
                    arity=Arity(0, 1),
                    detail="Return the path name of the current grab window, if any.",
                    synopsis="grab current ?window?",
                ),
                "release": SubCommand(
                    name="release",
                    arity=Arity(1, 1),
                    detail="Release the grab on the window.",
                    synopsis="grab release window",
                ),
                "set": SubCommand(
                    name="set",
                    arity=Arity(1, 2),
                    detail="Set a grab on the window, optionally global.",
                    synopsis="grab set ?-global? window",
                ),
                "status": SubCommand(
                    name="status",
                    arity=Arity(1, 1),
                    detail="Return the grab status of the window (none, local, or global).",
                    synopsis="grab status window",
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
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
