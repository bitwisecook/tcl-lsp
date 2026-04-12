# Enriched from F5 iRules reference documentation.
"""ip_ttl -- F5 iRules command `ip_ttl`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .ip__ttl import IpTtlCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/ip_ttl.html"


@register
class DeprecatedIpTtlCommand(CommandDef):
    name = "ip_ttl"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ip_ttl",
            deprecated_replacement=IpTtlCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Synonym for IP::ttl. Returns the TTL of the latest IP packet received.",
                synopsis=("ip_ttl",),
                snippet=("Synonym for IP::ttl. Returns the TTL of the latest IP packet\nreceived."),
                source=_SOURCE,
                examples=('when CLIENT_ACCEPTED {\n  log local0. "Client ttl: [ip_ttl]"\n}'),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ip_ttl",
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
