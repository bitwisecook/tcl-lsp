# Enriched from F5 iRules reference documentation.
"""nexthop -- Sets the nexthop of an IP connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/nexthop.html"


_av = make_av(_SOURCE)


@register
class NexthopCommand(CommandDef):
    name = "nexthop"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="nexthop",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the nexthop of an IP connection.",
                synopsis=(
                    "nexthop ((IP_ADDR) | ((VLAN_OBJ_NOT_IP_ADDR) (IP_ADDR | MAC_ADDR | transparent)?))",
                ),
                snippet=(
                    "Sets the nexthop of an IP connection. The nexthop is the destination\n"
                    "for packets going from the BIG-IP to the server. This is usually\n"
                    "determined by the IP routing table. This command lets you specify the\n"
                    "nexthop to use for a particular connection.\n"
                    "\n"
                    "Note: In 11.6, you can use the 'nexthop' command to direct traffic over\n"
                    "    IPIP tunnels.  In 13.0, you can use the 'nexthop' command to make\n"
                    "    connections L2 transparent (preserve source and destination MAC address)."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n  nexthop external 01:23:45:ab:cd:ef\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="nexthop ((IP_ADDR) | ((VLAN_OBJ_NOT_IP_ADDR) (IP_ADDR | MAC_ADDR | transparent)?))",
                    arg_values={
                        0: (
                            _av(
                                "transparent",
                                "nexthop transparent",
                                "nexthop ((IP_ADDR) | ((VLAN_OBJ_NOT_IP_ADDR) (IP_ADDR | MAC_ADDR | transparent)?))",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(client_side=True, also_in=frozenset({"PERSIST_DOWN"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
