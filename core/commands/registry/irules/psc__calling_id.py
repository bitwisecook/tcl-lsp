# Enriched from F5 iRules reference documentation.
"""PSC::calling_id -- Get or set calling id."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PSC__calling_id.html"


@register
class PscCallingIdCommand(CommandDef):
    name = "PSC::calling_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PSC::calling_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set calling id.",
                synopsis=("PSC::calling_id (CALLING_ID)?",),
                snippet=(
                    "The PSC::calling_id command gets the calling station id or sets the\n"
                    "calling station id when the optional value is given."
                ),
                source=_SOURCE,
                return_value="Return the calling station id when no argument is given.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PSC::calling_id (CALLING_ID)?",
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
