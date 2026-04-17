# Enriched from F5 iRules reference documentation.
"""RESOLVER::name_lookup -- This command performs a DNS lookup."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RESOLV-lookup.html"


@register
class ResolverNameLookupCommand(CommandDef):
    name = "RESOLVER::name_lookup"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RESOLVER::name_lookup",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command performs a DNS lookup.",
                synopsis=("RESOLVER::name_lookup NET_RESOLVER_NAME NAME TYPE",),
                snippet=(
                    "RESOLVER::name_lookup performs a DNS query for a name and type using the network resolver specified.\n"
                    "\n"
                    "This command allows queries for all resource record types and provides access to the DNS services available on the BIGIP."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '        set result [RESOLVER::name_lookup "/Common/r1" www.abc.com a]\n'
                    '        set result [RESOLVER::name_lookup "/Common/r1" 206.6.177.2 ptr]\n'
                    "}"
                ),
                return_value="The response will be a dns_message structure usable by the RESOLVER::summarize, DNSMSG::header, and DNSMSG::section iRule commands.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RESOLVER::name_lookup NET_RESOLVER_NAME NAME TYPE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DNS_STATE,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
