# Enriched from F5 iRules reference documentation.
"""lasthop -- Sets the lasthop of an IP connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/lasthop.html"


@register
class LasthopCommand(CommandDef):
    name = "lasthop"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="lasthop",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets the lasthop of an IP connection.",
                synopsis=("lasthop (VLAN_OBJ)? (IP_ADDR | MAC_ADDR)",),
                snippet=(
                    "Sets the lasthop of a IP connection. The lasthop is the MAC destination\n"
                    "for packets going back to the client. This is usually the router\n"
                    "(gateway) that forwards the client's packets to the BIG-IP (if \"auto\n"
                    'lasthop" is set), or is determined by the IP routing table. This\n'
                    "command lets you specify the lasthop to use for a particular\n"
                    "connection."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n  lasthop external 01:23:45:ab:cd:ef\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="lasthop (VLAN_OBJ)? (IP_ADDR | MAC_ADDR)",
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
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
