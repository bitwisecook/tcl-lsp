# Enriched from F5 iRules reference documentation.
"""IP::reputation -- Looks up the supplied IP address in the IP intelligence (reputation) database and returns a TCL list containing reputation categories."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IP__reputation.html"


@register
class IpReputationCommand(CommandDef):
    name = "IP::reputation"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IP::reputation",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Looks up the supplied IP address in the IP intelligence (reputation) database and returns a TCL list containing reputation categories.",
                synopsis=("IP::reputation (IP_ADDR)+",),
                snippet=(
                    "Performs a lookup of the supplied IP address against the IP reputation database. Returns a TCL list containing possible reputation categories:\n"
                    "\n"
                    "Category                     Description\n"
                    "Botnets                      IP addresses of computers that are infected with malicious software and are controlled as a group, and are now part of a botnet. Hackers can exploit botnets to send spam messages, launch various attacks, or cause target systems to behave in other unpredictable ways.\n"
                    "Cloud Provider Networks      IP addresses of cloud providers."
                ),
                source=_SOURCE,
                examples=(
                    "#Drop the packet after initial TCP handshake if the client has a bad reputation\n"
                    "when CLIENT_ACCEPTED {\n"
                    "    # Check if the IP reputation list for the client IP is not 0\n"
                    "    if {[llength [IP::reputation [IP::client_addr]]] != 0}{\n"
                    "        # Drop the connection\n"
                    "        drop\n"
                    "    }\n"
                    "}"
                ),
                return_value="Return a TCL list containing reputation categories.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IP::reputation (IP_ADDR)+",
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
