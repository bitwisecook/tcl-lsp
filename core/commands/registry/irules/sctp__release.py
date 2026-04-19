# Enriched from F5 iRules reference documentation.
"""SCTP::release -- Resumes processing and flushes collected data."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SCTP__release.html"


@register
class SctpReleaseCommand(CommandDef):
    name = "SCTP::release"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SCTP::release",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Resumes processing and flushes collected data.",
                synopsis=("SCTP::release (RELEASE_BYTES)?",),
                snippet="Causes SCTP to resume processing the connection and flush collected data.",
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n  SCTP::collect 15\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SCTP::release (RELEASE_BYTES)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
