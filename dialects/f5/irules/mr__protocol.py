# Enriched from F5 iRules reference documentation.
"""MR::protocol -- Returns generic, sip or diameter."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__protocol.html"


@register
class MrProtocolCommand(CommandDef):
    name = "MR::protocol"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::protocol",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns generic, sip or diameter.",
                synopsis=("MR::protocol",),
                snippet="returns generic, sip or diameter",
                source=_SOURCE,
                examples=(
                    "when MR_INGRESS {\n"
                    '    log local0. "[MR::protocol] router instance [MR::instance]"\n'
                    "}"
                ),
                return_value="returns generic, sip or diameter",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::protocol",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
