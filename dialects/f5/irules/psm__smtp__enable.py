# Enriched from F5 iRules reference documentation.
"""PSM::SMTP::enable -- To enable PSM for SMTP traffic."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PSM__SMTP__enable.html"


@register
class PsmSmtpEnableCommand(CommandDef):
    name = "PSM::SMTP::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PSM::SMTP::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="To enable PSM for SMTP traffic.",
                synopsis=("PSM::SMTP::enable",),
                snippet="To enable PSM for SMTP traffic",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PSM::SMTP::enable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
