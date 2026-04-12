# Generated from F5 iRules reference documentation -- do not edit manually.
"""SSL::nextproto -- Get or set the Next Protocol Negotiation (NPN) string."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .ssl__alpn import SslAlpnCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__nextproto.html"


@register
class SslNextprotoCommand(CommandDef):
    name = "SSL::nextproto"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::nextproto",
            deprecated_replacement=SslAlpnCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set the Next Protocol Negotiation (NPN) string.",
                synopsis=("SSL::nextproto",),
                snippet="Get or set the Next Protocol Negotiation (NPN) string.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::nextproto",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                client_side=True, transport="tcp", profiles=frozenset({"CLIENTSSL"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
