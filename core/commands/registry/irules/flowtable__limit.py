# Enriched from F5 iRules reference documentation.
"""FLOWTABLE::limit -- Returns configured connection limits."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/FLOWTABLE__limit.html"


@register
class FlowtableLimitCommand(CommandDef):
    name = "FLOWTABLE::limit"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="FLOWTABLE::limit",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns configured connection limits.",
                synopsis=(
                    "FLOWTABLE::limit virtual (VIRTUAL_SERVER_OBJ)?",
                    "FLOWTABLE::limit route_domain (ROUTE_DOMAIN_NAME)?",
                ),
                snippet=(
                    "This iRules command returns configured connection limits\n"
                    "Note: When virtual server or route domain name is omitted the commands\n"
                    "use virtual or route domain of the current connection. Specifying the\n"
                    "name incurs significant performance hit."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="FLOWTABLE::limit virtual (VIRTUAL_SERVER_OBJ)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FLOW_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
