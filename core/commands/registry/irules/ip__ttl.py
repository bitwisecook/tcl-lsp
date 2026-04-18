# Enriched from F5 iRules reference documentation.
"""IP::ttl -- Returns the TTL of the latest IP packet received."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IP__ttl.html"


@register
class IpTtlCommand(CommandDef):
    name = "IP::ttl"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IP::ttl",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the TTL of the latest IP packet received.",
                synopsis=("IP::ttl",),
                snippet="Returns the TTL of the latest IP packet received.",
                source=_SOURCE,
                return_value="Returns the TTL of the latest IP packet received.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IP::ttl",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(client_side=True),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
