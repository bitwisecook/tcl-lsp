# Enriched from F5 iRules reference documentation.
"""DHCP::version -- This command returns version number (4 or 6) of DHCP packet."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DHCP__version.html"


@register
class DhcpVersionCommand(CommandDef):
    name = "DHCP::version"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DHCP::version",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command returns version number (4 or 6) of DHCP packet.",
                synopsis=("DHCP::version",),
                snippet=(
                    "This command returns DHCP version number (4 or 6) of DHCP packet\n"
                    "\n"
                    "Details (syntax):\n"
                    "DHCP::version"
                ),
                source=_SOURCE,
                examples=(
                    'when CLIENT_DATA {\n        log local0. "Version [DHCP::version]"\n    }'
                ),
                return_value="This command returns version number (4 or 6) of DHCP packet",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DHCP::version",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
