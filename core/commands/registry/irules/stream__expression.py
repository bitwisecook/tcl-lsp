# Enriched from F5 iRules reference documentation.
"""STREAM::expression -- Replaces the expression in a Stream profile with another expression."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/STREAM__expression.html"


@register
class StreamExpressionCommand(CommandDef):
    name = "STREAM::expression"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="STREAM::expression",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Replaces the expression in a Stream profile with another expression.",
                synopsis=("STREAM::expression EXPRESSION",),
                snippet=(
                    "Replaces the stream expression in the Stream profile with the specified\n"
                    "value. The syntax is identical to the profile syntax.\n"
                    "Note that this change affects this connection only and is sticky\n"
                    "for the duration of the connection."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "   # Disable the stream filter for all requests\n"
                    "   STREAM::disable\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="STREAM::expression EXPRESSION",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
