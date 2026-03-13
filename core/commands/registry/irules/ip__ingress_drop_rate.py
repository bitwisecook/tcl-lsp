# Generated from F5 iRules reference documentation -- do not edit manually.
"""IP::ingress_drop_rate -- Adds ip with specified drop rate to black list table."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/IP__ingress_drop_rate.html"


@register
class IpIngressDropRateCommand(CommandDef):
    name = "IP::ingress_drop_rate"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="IP::ingress_drop_rate",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Adds ip with specified drop rate to black list table.",
                synopsis=("IP::ingress_drop_rate",),
                snippet="This command adds ip with specified drop rate to black list table, table enforced per packet containing source ip for specified timeout.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="IP::ingress_drop_rate",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(client_side=True),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
