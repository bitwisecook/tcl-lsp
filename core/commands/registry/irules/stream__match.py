# Enriched from F5 iRules reference documentation.
"""STREAM::match -- Returns the matching characters."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/STREAM__match.html"


@register
class StreamMatchCommand(CommandDef):
    name = "STREAM::match"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="STREAM::match",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the matching characters.",
                synopsis=("STREAM::match",),
                snippet="Returns the matching characters.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "\n"
                    "   # Disable the stream filter for all requests\n"
                    "   STREAM::disable\n"
                    "}"
                ),
                return_value="Returns the matching characters.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="STREAM::match",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
