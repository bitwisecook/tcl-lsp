# Enriched from F5 iRules reference documentation.
"""IP::hops -- Gives you the estimated number of hops the peer takes to get to you."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IP__hops.html"


@register
class IpHopsCommand(CommandDef):
    name = "IP::hops"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IP::hops",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gives you the estimated number of hops the peer takes to get to you.",
                synopsis=("IP::hops",),
                snippet="This command is used to give you the estimated number of hops between the peer in question, and the client machine making the request.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  if { [IP::hops] >= 10 } {\n"
                    "      COMPRESS::disable\n"
                    "  }\n"
                    "}"
                ),
                return_value="Number of hops",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IP::hops",
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
