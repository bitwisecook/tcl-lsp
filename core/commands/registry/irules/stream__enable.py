# Enriched from F5 iRules reference documentation.
"""STREAM::enable -- Enables the stream filter for the life of the current TCP connection or until disabled."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/STREAM__enable.html"


@register
class StreamEnableCommand(CommandDef):
    name = "STREAM::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="STREAM::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables the stream filter for the life of the current TCP connection or until disabled.",
                synopsis=("STREAM::enable",),
                snippet=(
                    "Enables the stream filter for the life of the current TCP connection or\n"
                    "until disabled with STREAM::disable."
                ),
                source=_SOURCE,
                examples=(
                    "# This section only logs matches, and should be removed before using the rule in production.\n"
                    "when STREAM_MATCHED {\n"
                    '    log local0. "Matched: [STREAM::match]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="STREAM::enable",
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
