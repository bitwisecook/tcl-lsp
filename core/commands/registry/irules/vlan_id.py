# Enriched from F5 iRules reference documentation.
"""vlan_id -- Returns the VLAN tag of the packet."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/vlan_id.html"


@register
class VlanIdCommand(CommandDef):
    name = "vlan_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="vlan_id",
            deprecated_replacement="VLAN::id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the VLAN tag of the packet.",
                synopsis=("vlan_id",),
                snippet=(
                    "Returns the VLAN tag of the packet. This is a BIG-IP 4.X variable,\n"
                    "provided for backward-compatibility. You can use the equivalent 9.X\n"
                    "command LINK::vlan_id instead."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="vlan_id",
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
