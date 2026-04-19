# Enriched from F5 iRules reference documentation.
"""PSC::subscriber_id -- Get or set the subscriber id."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PSC-subscriber-id.html"


@register
class PscSubscriberIdCommand(CommandDef):
    name = "PSC::subscriber_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PSC::subscriber_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set the subscriber id.",
                synopsis=("PSC::subscriber_id ((SUBSCRIBER_ID) (e164 |",),
                snippet="The PSC::subscriber_id command gets the subscriber id or sets the subscriber_id when the optional value is given.",
                source=_SOURCE,
                return_value="Return the subscriber id when no argument is given.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PSC::subscriber_id ((SUBSCRIBER_ID) (e164 |",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
