# Enriched from F5 iRules reference documentation.
"""ONECONNECT::label -- Associate OneConnect keying information with connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ONECONNECT__label.html"


@register
class OneconnectLabelCommand(CommandDef):
    name = "ONECONNECT::label"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ONECONNECT::label",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Associate OneConnect keying information with connection.",
                synopsis=("ONECONNECT::label update KEY",),
                snippet="Associate OneConnect keying information with connection",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "   set keymatch [HTTP::uri]\n"
                    "   persist uie $keymatch\n"
                    "   ONECONNECT::select persist\n"
                    " }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ONECONNECT::label update KEY",
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
