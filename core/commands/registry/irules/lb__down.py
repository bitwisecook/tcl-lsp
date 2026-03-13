# Enriched from F5 iRules reference documentation.
"""LB::down -- Sets the status of a node or pool member as being down."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__down.html"


_av = make_av(_SOURCE)


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
                synopsis=("LB::down ((node IP_ADDR) |",),
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
                    synopsis="LB::down ((node IP_ADDR) |",
                    arg_values={0: (_av("node", "LB::down node", "LB::down ((node IP_ADDR) |"),)},
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
        )
