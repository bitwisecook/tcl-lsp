# Enriched from F5 iRules reference documentation.
"""PSC::imeisv -- Get or set imeisv value."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PSC__imeisv.html"


@register
class PscImeisvCommand(CommandDef):
    name = "PSC::imeisv"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PSC::imeisv",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set imeisv value.",
                synopsis=("PSC::imeisv (IMEISV)?",),
                snippet=(
                    "The PSC::imeisv command gets the imeisv or sets the imeisv when the\n"
                    "optional value is given."
                ),
                source=_SOURCE,
                return_value="Return the imeisv value when no argument is given.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PSC::imeisv (IMEISV)?",
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
