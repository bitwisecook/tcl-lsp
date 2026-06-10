# Enriched from F5 iRules reference documentation.
"""LB::up -- Sets the status of a node or pool member as being up."""

# Introduced: BIG-IP v9+ (core load-balancing iRules command) (approximate, from F5 documentation)

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    SubCommand,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__up.html"


_av = make_av(_SOURCE)

_WRITE_EFFECT = (
    SideEffect(
        target=SideEffectTarget.POOL_SELECTION,
        reads=True,
        writes=True,
        connection_side=ConnectionSide.SERVER,
    ),
)


@register
class LbUpCommand(CommandDef):
    name = "LB::up"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::up",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the status of a node or pool member as being up.",
                synopsis=(
                    "LB::up",
                    "LB::up node <address>",
                    "LB::up pool <pool> member <address> <port>",
                ),
                snippet="Sets the status of the specified node or pool member as being up. If you specify no arguments, the status of the currently-selected node is modified.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::up ?node <addr> | pool <pool> member <addr> <port>?",
                    arg_values={
                        0: (
                            _av("node", "Mark node as up.", "LB::up node <address>"),
                            _av(
                                "pool",
                                "Mark pool member as up.",
                                "LB::up pool <pool> member <address> <port>",
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
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
            subcommands={
                "node": SubCommand(
                    name="node",
                    arity=Arity(1, 1),
                    detail="Mark node as up.",
                    synopsis="LB::up node <address>",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "pool": SubCommand(
                    name="pool",
                    arity=Arity(3, 3),
                    detail="Mark pool member as up.",
                    synopsis="LB::up pool <pool> member <address> <port>",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
            },
        )
