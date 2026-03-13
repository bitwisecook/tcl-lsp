# Enriched from F5 iRules reference documentation.
"""PSC::ip_address -- Get/set/remove ip address(es)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PSC__ip_address.html"


_av = make_av(_SOURCE)


@register
class PscIpAddressCommand(CommandDef):
    name = "PSC::ip_address"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PSC::ip_address",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get/set/remove ip address(es).",
                synopsis=(
                    "PSC::ip_address (IP_ADDR)*",
                    "PSC::ip_address 'add' IP_ADDR",
                    "PSC::ip_address 'remove' (IP_ADDR)?",
                ),
                snippet=(
                    "The PSC::ip_address commands get/set/remove the IP addresses.\n"
                    "\n"
                    "Note:IP address used in the commands below could be in IPv4 or IPv6 format. The route domain can be specified using % as a separator, e.g. 14.15.16.17%10."
                ),
                source=_SOURCE,
                return_value="Return the list of PSC ip addresses when no argument is given.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PSC::ip_address (IP_ADDR)*",
                    arg_values={
                        0: (
                            _av("add", "PSC::ip_address add", "PSC::ip_address 'add' IP_ADDR"),
                            _av(
                                "remove",
                                "PSC::ip_address remove",
                                "PSC::ip_address 'remove' (IP_ADDR)?",
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
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
