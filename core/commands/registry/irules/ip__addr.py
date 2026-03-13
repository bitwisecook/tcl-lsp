# Enriched from F5 iRules reference documentation.
"""IP::addr -- IP address comparison."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IP__addr.html"


_av = make_av(_SOURCE)


@register
class IpAddrCommand(CommandDef):
    name = "IP::addr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IP::addr",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="IP address comparison.",
                synopsis=(
                    "IP::addr IP_ADDR_MASK 'equals' IP_ADDR_MASK",
                    "IP::addr 'parse' ((('-swap')? BINARY_FIELD (OFFSET)?) |",
                ),
                snippet=(
                    "IP address comparison\n"
                    "\n"
                    "Performs comparison of IP address/subnet/supernet to IP address/subnet/supernet.\n"
                    "\n"
                    "Returns 0 if no match, 1 for a match.\n"
                    "\n"
                    "Use of IP::addr is not necessary if the class (v10+) or matchclass (v9) command is used to perform the address-to-address comparison.\n"
                    "\n"
                    "Does NOT perform a string comparison. To perform a literal string comparison, simply compare the 2 strings with the appropriate operator (equals, contains, starts_with, etc) rather than using the IP::addr comparison.\n"
                    "\n"
                    "For versions 10.0 - 10.2."
                ),
                source=_SOURCE,
                examples=(
                    "# To select a specific pool for a specific client IP address.\n"
                    "when CLIENT_ACCEPTED {\n"
                    "   if { [IP::addr [IP::client_addr] equals 10.10.10.10] } {\n"
                    "      pool my_pool\n"
                    "   }\n"
                    "}"
                ),
                return_value="Returns 0 IF NO MATCH, 1 for a match.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IP::addr IP_ADDR_MASK 'equals' IP_ADDR_MASK",
                    options=(OptionSpec(name="-swap", detail="Option -swap.", takes_value=True),),
                    arg_values={
                        0: (
                            _av(
                                "equals",
                                "IP::addr equals",
                                "IP::addr IP_ADDR_MASK 'equals' IP_ADDR_MASK",
                            ),
                            _av(
                                "parse",
                                "IP::addr parse",
                                "IP::addr 'parse' ((('-swap')? BINARY_FIELD (OFFSET)?) |",
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
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
