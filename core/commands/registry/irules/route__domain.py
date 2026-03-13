# Enriched from F5 iRules reference documentation.
"""ROUTE::domain -- Returns the current routing domain of the current connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ROUTE__domain.html"


@register
class RouteDomainCommand(CommandDef):
    name = "ROUTE::domain"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ROUTE::domain",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the current routing domain of the current connection.",
                synopsis=("ROUTE::domain",),
                snippet=(
                    "Returns the current routing domain of the current connection. Several\n"
                    "commands allow an addition rt_domain option: node, snat, LB::status"
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set gateway 10.3.1.11\n"
                    "    set bandwidth [ROUTE::bandwidth [IP::remote_addr] $gateway%[ROUTE::domain]]\n"
                    "    if { $bandwidth > 0 } {\n"
                    '        log local0. "Destination found in cache, bandwidth = $bandwidth"\n'
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ROUTE::domain",
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
