# Enriched from F5 iRules reference documentation.
"""LB::down -- Sets the status of a node or pool member as being down."""

# Introduced: BIG-IP v9+ (core load-balancing iRules command) (approximate, from F5 documentation)

from __future__ import annotations

from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__down.html"


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
class LbDownCommand(CommandDef):
    name = "LB::down"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::down",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the status of a node or pool member as being down.",
                synopsis=(
                    "LB::down",
                    "LB::down node <address>",
                    "LB::down pool <pool> member <address> <port>",
                ),
                snippet=(
                    "Sets the status of the specified node or pool member as being down. If you specify no arguments, the status of the currently-selected node is modified.\n"
                    "Note: Calling LB::down in an iRule triggers an immediate monitor probe regardless of the monitor interval settings.\n"
                    "\n"
                    "LB::down\n"
                    "    Sets the status of the currently-selected node as being down.\n"
                    "\n"
                    "LB::down node <address>\n"
                    "    Sets the status of the specified node as being down.\n"
                    "    Doesn't work. Use LB::down or LB::down pool <pool> member <address> <port>. Refer to BZ222047 for details."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    "    if { [HTTP::status] == 500 } {\n"
                    "        LB::down\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::down ?node <addr> | pool <pool> member <addr> <port>?",
                    arg_values={
                        0: (
                            _av("node", "Mark node as down.", "LB::down node <address>"),
                            _av(
                                "pool",
                                "Mark pool member as down.",
                                "LB::down pool <pool> member <address> <port>",
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
                    detail="Mark node as down.",
                    synopsis="LB::down node <address>",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
                "pool": SubCommand(
                    name="pool",
                    arity=Arity(3, 3),
                    detail="Mark pool member as down.",
                    synopsis="LB::down pool <pool> member <address> <port>",
                    mutator=True,
                    side_effect_hints=_WRITE_EFFECT,
                ),
            },
        )
