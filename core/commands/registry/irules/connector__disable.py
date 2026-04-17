# Enriched from F5 iRules reference documentation.
"""CONNECTOR::disable -- Disable all the connectors on chain."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CONNECTOR__disable.html"


@register
class ConnectorDisableCommand(CommandDef):
    name = "CONNECTOR::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CONNECTOR::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disable all the connectors on chain.",
                synopsis=("CONNECTOR::disable",),
                snippet="Disable all the connectors  on chain",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n                CONNECTOR::disable\n            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CONNECTOR::disable",
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
