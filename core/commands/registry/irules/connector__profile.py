# Enriched from F5 iRules reference documentation.
"""CONNECTOR::profile -- Get connector profile name."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CONNECTOR__profile.html"


@register
class ConnectorProfileCommand(CommandDef):
    name = "CONNECTOR::profile"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CONNECTOR::profile",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get connector profile name.",
                synopsis=("CONNECTOR::profile",),
                snippet=(
                    "CONNECTOR::profile\n    Get the connector profile name in the current context."
                ),
                source=_SOURCE,
                examples=(
                    "when CONNECTOR_OPEN {\n"
                    '                if {([CONNECTOR::profile] eq "/Common/connector_profile_1")} {\n'
                    '                    log local0. "CONNECTOR_OPEN raised by connector_profile_1"\n'
                    "                }\n"
                    "            }"
                ),
                return_value="CONNECTOR::profile Return the connector profile name.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CONNECTOR::profile",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
