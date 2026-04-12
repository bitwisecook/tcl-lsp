# Enriched from F5 iRules reference documentation.
"""PSC::attr -- Gets, sets or removes the custom attributes."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PSC__attr.html"


_av = make_av(_SOURCE)


@register
class PscAttrCommand(CommandDef):
    name = "PSC::attr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PSC::attr",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets, sets or removes the custom attributes.",
                synopsis=(
                    "PSC::attr ((NAME) | (NAME VALUE))?",
                    "PSC::attr 'remove' (NAME)?",
                ),
                snippet="The PSC::attr commands get/set/remove the custom attributes.",
                source=_SOURCE,
                return_value="* PSC::attr Return the list of custom attribute names when no argument is given.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PSC::attr ((NAME) | (NAME VALUE))?",
                    arg_values={
                        0: (_av("remove", "PSC::attr remove", "PSC::attr 'remove' (NAME)?"),)
                    },
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
