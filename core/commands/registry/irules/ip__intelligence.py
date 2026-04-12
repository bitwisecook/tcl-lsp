# Enriched from F5 iRules reference documentation.
"""IP::intelligence -- Return a Tcl list of IP intelligence category names for a given IP address."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IP__intelligence.html"


@register
class IpIntelligenceCommand(CommandDef):
    name = "IP::intelligence"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IP::intelligence",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Return a Tcl list of IP intelligence category names for a given IP address.",
                synopsis=("IP::intelligence IP_ADDR",),
                snippet="This iRules command returns a Tcl list of IP intelligence category names for a given IP address. It checks up to 3 (configured) IP intelligence policies - global policy, policy attached to virtual server and policy attached to route domain. If any of the policies use IP reputation database, it will also be checked. This command is an extention of the IP::reputation command, which checked only IP reputation database available from external source.",
                source=_SOURCE,
                examples=(
                    "# This irule can be used to test IP Intelligence dwbl (feed lists).\n"
                    "# if a request comes in with a URI query:  ?ip=10.0.0.2, it returns the intelligence record.\n"
                    "# if no query is supplied, it returns the intelligence file.  You can use this in the feed list configuration.\n"
                    "when HTTP_REQUEST {\n"
                    "    set ip [URI::query [HTTP::uri] ip]\n"
                    '    if { $ip equals "" } {\n'
                    '        log local0. "Got a Feed List update request from [IP::client_addr]"\n'
                    "    HTTP::respond 200 content {10.0.0.2,32,bl,spam_sources"
                ),
                return_value="Return a Tcl list of IP intelligence category names for a given IP address",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IP::intelligence IP_ADDR",
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
