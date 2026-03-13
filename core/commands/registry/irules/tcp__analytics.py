# Enriched from F5 iRules reference documentation.
"""TCP::analytics -- Enable/disable AVR TCP stat reporting, and/or attach a user-defined string to categorize the connection for statistics collection purposes."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__analytics.html"


@register
class TcpAnalyticsCommand(CommandDef):
    name = "TCP::analytics"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::analytics",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enable/disable AVR TCP stat reporting, and/or attach a user-defined string to categorize the connection for statistics collection purposes.",
                synopsis=("TCP::analytics (enable | disable | key (KEY)?)",),
                snippet=(
                    'Enables or disables AVR TCP stat reporting ("analytics") for this connection and/or assigns user-defined keys.\n'
                    "\n"
                    "TCP::analytics enable\n"
                    "    Enables analytics on this connection. AVR must be provisioned and the virtual must have a tcp-analytics profile attached. Collection will use the configuration in the profile. If the profile is configured to disable analytics by default, this gives users the ability to collect statistics by exception only.\n"
                    "\n"
                    "TCP::analytics disable\n"
                    "    Disables analytics on this connection."
                ),
                source=_SOURCE,
                examples=(
                    "rt collection for one subnet only.\n"
                    "     when CLIENT_ACCEPTED {\n"
                    "         if [IP::addr [IP::client_addr]/8 equals 10.0.0.0] {\n"
                    "             TCP::analytics enable\n"
                    "         }\n"
                    "     }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::analytics (enable | disable | key (KEY)?)",
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
