# Enriched from F5 iRules reference documentation.
"""LB::connlimit -- Set the connection limit for virtual/node/poolmember."""

# Introduced: BIG-IP v9+ (core load-balancing iRules command) (approximate, from F5 documentation)

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__connlimit.html"


_av = make_av(_SOURCE)

_RW_EFFECT = (
    SideEffect(
        target=SideEffectTarget.POOL_SELECTION,
        reads=True,
        writes=True,
        connection_side=ConnectionSide.SERVER,
    ),
)


@register
class LbConnlimitCommand(CommandDef):
    name = "LB::connlimit"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::connlimit",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set the connection limit for virtual/node/poolmember.",
                synopsis=(
                    "LB::connlimit ('virtual' | 'node' | 'poolmember') ?limit <value>? ?key <value>?",
                ),
                snippet="Set the connection limit for virtual/node/poolmember",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::connlimit <target> ?args?",
                    arg_values={
                        0: (
                            _av(
                                "virtual",
                                "Set/get virtual connection limit.",
                                "LB::connlimit virtual ?limit <value>? ?key <value>?",
                            ),
                            _av(
                                "node",
                                "Set/get node connection limit.",
                                "LB::connlimit node ?limit <value>? ?key <value>?",
                            ),
                            _av(
                                "poolmember",
                                "Set/get pool member connection limit.",
                                "LB::connlimit poolmember ?limit <value>? ?key <value>?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.POOL_SELECTION,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
            subcommands={
                sub: SubCommand(
                    name=sub,
                    arity=Arity(0),
                    detail=f"Set/get {sub} connection limit.",
                    synopsis=f"LB::connlimit {sub} ?limit <value>? ?key <value>?",
                    pure=True,
                    mutator=True,
                    side_effect_hints=_RW_EFFECT,
                )
                for sub in ("virtual", "node", "poolmember")
            },
        )
