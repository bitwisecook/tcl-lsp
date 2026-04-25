"""pack -- Geometry manager that packs slaves around edges of cavity."""

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

_SOURCE = "Tk man page pack.n"
_av = make_av(_SOURCE)


@register
class PackCommand(CommandDef):
    name = "pack"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="pack",
            required_package="Tk",
            hover=HoverSnippet(
                summary="Geometry manager that packs slaves around the edges of a cavity.",
                synopsis=(
                    "pack slave ?slave ...? ?option value ...?",
                    "pack configure slave ?slave ...? ?option value ...?",
                    "pack forget slave ?slave ...?",
                    "pack info slave",
                    "pack propagate master ?boolean?",
                    "pack slaves master",
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="pack option arg ?arg ...?",
                    options=(
                        OptionSpec(
                            name="-side",
                            takes_value=True,
                            value_hint="top|bottom|left|right",
                            detail="Specifies which side of the master the slave will be packed against.",
                        ),
                        OptionSpec(
                            name="-fill",
                            takes_value=True,
                            value_hint="none|x|y|both",
                            detail="If a slave's parcel is larger than its requested dimensions, this option may be used to stretch the slave.",
                        ),
                        OptionSpec(
                            name="-expand",
                            takes_value=True,
                            value_hint="boolean",
                            detail="Specifies whether the slave should be expanded to consume extra space in its master.",
                        ),
                        OptionSpec(
                            name="-anchor",
                            takes_value=True,
                            value_hint="n|ne|e|se|s|sw|w|nw|center",
                            detail="Anchor must be a valid anchor position: n, ne, e, se, s, sw, w, nw, or center.",
                        ),
                        OptionSpec(
                            name="-padx",
                            takes_value=True,
                            value_hint="amount",
                            detail="Specifies how much external horizontal padding to leave on each side of the slave.",
                        ),
                        OptionSpec(
                            name="-pady",
                            takes_value=True,
                            value_hint="amount",
                            detail="Specifies how much external vertical padding to leave on each side of the slave.",
                        ),
                        OptionSpec(
                            name="-ipadx",
                            takes_value=True,
                            value_hint="amount",
                            detail="Specifies how much internal horizontal padding to leave on each side of the slave.",
                        ),
                        OptionSpec(
                            name="-ipady",
                            takes_value=True,
                            value_hint="amount",
                            detail="Specifies how much internal vertical padding to leave on each side of the slave.",
                        ),
                        OptionSpec(
                            name="-in",
                            takes_value=True,
                            value_hint="master",
                            detail="Insert the slave at the end of the packing order for the master window.",
                        ),
                        OptionSpec(
                            name="-before",
                            takes_value=True,
                            value_hint="other",
                            detail="Insert the slave before the window given by other in the packing order.",
                        ),
                        OptionSpec(
                            name="-after",
                            takes_value=True,
                            value_hint="other",
                            detail="Insert the slave after the window given by other in the packing order.",
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "configure",
                                "Set or query the packing options for one or more slaves.",
                                "pack configure slave ?slave ...? ?option value ...?",
                            ),
                            _av(
                                "forget",
                                "Remove each slave from the packing order for its master.",
                                "pack forget slave ?slave ...?",
                            ),
                            _av(
                                "info",
                                "Return a list of the current configuration state of the slave.",
                                "pack info slave",
                            ),
                            _av(
                                "propagate",
                                "Control whether the master computes its geometry from slaves.",
                                "pack propagate master ?boolean?",
                            ),
                            _av(
                                "slaves",
                                "Return a list of all slaves in the packing order for the master.",
                                "pack slaves master",
                            ),
                        ),
                    },
                ),
            ),
            subcommands={
                "configure": SubCommand(
                    name="configure",
                    arity=Arity(1),
                    detail="Set or query the packing options for one or more slaves.",
                    synopsis="pack configure slave ?slave ...? ?option value ...?",
                ),
                "forget": SubCommand(
                    name="forget",
                    arity=Arity(1),
                    detail="Remove each slave from the packing order for its master.",
                    synopsis="pack forget slave ?slave ...?",
                ),
                "info": SubCommand(
                    name="info",
                    arity=Arity(1, 1),
                    detail="Return a list of the current configuration state of the slave.",
                    synopsis="pack info slave",
                ),
                "propagate": SubCommand(
                    name="propagate",
                    arity=Arity(1, 2),
                    detail="Control whether the master computes its geometry from slaves.",
                    synopsis="pack propagate master ?boolean?",
                ),
                "slaves": SubCommand(
                    name="slaves",
                    arity=Arity(1, 1),
                    detail="Return a list of all slaves in the packing order for the master.",
                    synopsis="pack slaves master",
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
