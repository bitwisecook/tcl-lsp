# Enriched from F5 iRules reference documentation.
"""getfield -- Splits a string on a character or string."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/getfield.html"


@register
class GetfieldCommand(CommandDef):
    name = "getfield"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="getfield",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Splits a string on a character or string.",
                synopsis=("getfield STRING SEPARATOR FIELD_NUMBER",),
                snippet=(
                    "A custom iRule function which splits a string on a character or\n"
                    "string, and returns the string corresponding to the specific field."
                ),
                source=_SOURCE,
                examples=('when HTTP_REQUEST {\n  set hostname [getfield [HTTP::host] ":" 1]\n}'),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="getfield STRING SEPARATOR FIELD_NUMBER",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
