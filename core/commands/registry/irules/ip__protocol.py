# Enriched from F5 iRules reference documentation.
"""IP::protocol -- F5 iRules command `IP::protocol`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IP__protocol.html"


@register
class IpProtocolCommand(CommandDef):
    name = "IP::protocol"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IP::protocol",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns the IP protocol value.",
                synopsis=("IP::protocol",),
                snippet=(
                    "Returns the IP protocol value. This command replaces the BIG-IP 4.X variable ip_protocol.\n"
                    "For a list of the IP protocol numbers, see /etc/protocols or the L<IANA protocol number list|http://www.iana.org/assignments/protocol-numbers/protocol-numbers.xml>"
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "  if { [IP::protocol] == 6 } {\n"
                    "     pool tcp_pool\n"
                    "  } else {\n"
                    "     pool slow_pool\n"
                    "  }\n"
                    "}"
                ),
                return_value="IP protocol",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IP::protocol",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(client_side=True),
            cse_candidate=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                    scope=StorageScope.CONNECTION,
                ),
            ),
        )
