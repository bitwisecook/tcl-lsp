# Enriched from F5 iRules reference documentation.
"""CONNECTOR::enable -- Enable all the connectors on chain."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CONNECTOR__enable.html"


@register
class ConnectorEnableCommand(CommandDef):
    name = "CONNECTOR::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CONNECTOR::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enable all the connectors on chain.",
                synopsis=("CONNECTOR::enable",),
                snippet="Enable all the connectors on chain.",
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_DATA {\n                CONNECTOR::enable\n            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CONNECTOR::enable",
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
