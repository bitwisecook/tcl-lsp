# Generated from F5 iRules reference documentation -- do not edit manually.
"""IP::ingress_rate_limit -- F5 iRules command `IP::ingress_rate_limit`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IP__ingress_rate_limit.html"


@register
class IpIngressRateLimitCommand(CommandDef):
    name = "IP::ingress_rate_limit"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IP::ingress_rate_limit",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="F5 iRules command `IP::ingress_rate_limit`.",
                synopsis=("IP::ingress_rate_limit",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IP::ingress_rate_limit",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(client_side=True),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
