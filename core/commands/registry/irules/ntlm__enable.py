# Enriched from F5 iRules reference documentation.
"""NTLM::enable -- Enables processing for NTLM."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/NTLM__enable.html"


@register
class NtlmEnableCommand(CommandDef):
    name = "NTLM::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="NTLM::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables processing for NTLM.",
                synopsis=("NTLM::enable",),
                snippet="Enables processing for NTLM",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="NTLM::enable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.APM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
