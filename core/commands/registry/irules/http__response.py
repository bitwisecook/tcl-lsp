# Enriched from F5 iRules reference documentation.
"""HTTP::response -- Returns the raw HTTP response header block as a single string."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTTP__response.html"


@register
class HttpResponseCommand(CommandDef):
    name = "HTTP::response"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTTP::response",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the raw HTTP response header block as a single string.",
                synopsis=("HTTP::response",),
                snippet="Returns the raw HTTP response header block as a single string.",
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    "    # Send response header block to high speed logging\n"
                    "    HSL::send $hsl [HTTP::response]\n"
                    "}"
                ),
                return_value="Returns the raw HTTP response header block as a single string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTTP::response",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_HEADER,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
