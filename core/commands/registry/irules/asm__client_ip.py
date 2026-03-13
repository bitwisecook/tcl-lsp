# Enriched from F5 iRules reference documentation.
"""ASM::client_ip -- Returns the IP address of the end client that sent the request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__client_ip.html"


@register
class AsmClientIpCommand(CommandDef):
    name = "ASM::client_ip"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::client_ip",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the IP address of the end client that sent the request.",
                synopsis=("ASM::client_ip",),
                snippet=(
                    "Returns the IP address of the end client that sent the request.\n"
                    "Note that this IP address is not necessarily equal to the address\n"
                    "returned by the command IP::client_addr, which is the IP address of the\n"
                    "immediate client found in the IP header as received by BIG-IP. The\n"
                    "latter can be a proxy, in which case the end client IP address is\n"
                    "extracted from one of the HTTP headers, typically, X-Forwarded-For."
                ),
                source=_SOURCE,
                examples=(
                    "when ASM_REQUEST_DONE {\n"
                    '  log local0. "Src IP: [IP::client_addr], End-client IP: [ASM::client_ip]"\n'
                    "}"
                ),
                return_value="Returns the IP address of the end client that sent the request.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::client_ip",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ASM"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
