# Enriched from F5 iRules reference documentation.
"""GTP::discard -- Discards the current message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GTP__discard.html"


@register
class GtpDiscardCommand(CommandDef):
    name = "GTP::discard"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GTP::discard",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Discards the current message.",
                synopsis=("GTP::discard",),
                snippet="Discards the current message",
                source=_SOURCE,
                examples=("when GTP_SIGNALLING_INGRESS {\n    GTP::discard\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GTP::discard",
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
