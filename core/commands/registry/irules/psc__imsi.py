# Enriched from F5 iRules reference documentation.
"""PSC::imsi -- Get or set the imsi value."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PSC__imsi.html"


@register
class PscImsiCommand(CommandDef):
    name = "PSC::imsi"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PSC::imsi",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set the imsi value.",
                synopsis=("PSC::imsi (IMSI)?",),
                snippet=(
                    "The PSC::imsi command gets the imsi or sets the imsi when the optional\n"
                    "is given."
                ),
                source=_SOURCE,
                return_value="Return the imsi value when no argument is given.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PSC::imsi (IMSI)?",
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
