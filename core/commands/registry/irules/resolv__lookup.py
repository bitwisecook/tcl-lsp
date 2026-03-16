# Generated from F5 iRules reference documentation -- do not edit manually.
"""RESOLV::lookup -- Deprecated: The commands for making a DNS lookup."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RESOLV__lookup.html"


@register
class ResolvLookupCommand(CommandDef):
    name = "RESOLV::lookup"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RESOLV::lookup",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deprecated: The commands for making a DNS lookup.",
                synopsis=("RESOLV::lookup",),
                snippet="RESOLV::lookup performs a DNS query, returning one or more addresses (A records) for a hostname, a domain name (PTR record) for an IP address, or optionally one or more values for records of other types.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RESOLV::lookup ?@nameserver? ?-type? hostname",
                    options=(
                        OptionSpec(
                            name="-a",
                            detail="Query for type A (IPv4) records.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-aaaa",
                            detail="Query for type AAAA (IPv6) records.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-ptr",
                            detail="Query for PTR records.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-txt",
                            detail="Query for TXT records.",
                            takes_value=False,
                        ),
                        OptionSpec(
                            name="-mx",
                            detail="Query for MX records.",
                            takes_value=False,
                        ),
                    ),
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
