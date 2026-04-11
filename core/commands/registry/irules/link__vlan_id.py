# Enriched from F5 iRules reference documentation.
"""LINK::vlan_id -- Returns the VLAN tag of the packet."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LINK__vlan_id.html"


@register
class LinkVlanIdCommand(CommandDef):
    name = "LINK::vlan_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LINK::vlan_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the VLAN tag of the packet.",
                synopsis=("LINK::vlan_id",),
                snippet=(
                    "Returns the VLAN tag of the packet. This command is equivalent to the\n"
                    "BIG-IP 4.X variable vlan_id."
                ),
                source=_SOURCE,
                examples=(
                    "# log requests\n"
                    "when CLIENT_ACCEPTED {\n"
                    '    set info "client { [IP::client_addr]:[TCP::client_port] -> [IP::local_addr]:[TCP::local_port] }"\n'
                    '    append info " ethernet "\n'
                    '    append info " { [string range [LINK::lasthop] 0 16] -> [string range [LINK::nexthop] 0 16] "\n'
                    '    append info "tag [LINK::vlan_id] qos [LINK::qos] }"\n'
                    "    log local0. $info\n"
                    "}"
                ),
                return_value="LINK::vlan_id",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LINK::vlan_id",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
