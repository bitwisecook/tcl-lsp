# Enriched from F5 iRules reference documentation.
"""CATEGORY::filetype -- Get mime type and mime subtype of payload."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CATEGORY__filetype.html"


@register
class CategoryFiletypeCommand(CommandDef):
    name = "CATEGORY::filetype"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CATEGORY::filetype",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get mime type and mime subtype of payload.",
                synopsis=("CATEGORY::filetype HTTP_PAYLOAD",),
                snippet="Checks for the mime type and mime subtype of an HTTP request payload and returns the values to specified variables; use one or both to specify them name of the variable that you want the value to be given to.",
                source=_SOURCE,
                examples=(
                    "when HTTP_RESPONSE {\n"
                    "    # Collect 256 bytes of payload and trigger HTTP_RESPONSE_DATA\n"
                    "    HTTP::collect 256\n"
                    "}"
                ),
                return_value="Sets the specified variables with the mime type or mime subtype",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CATEGORY::filetype HTTP_PAYLOAD",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CLASSIFICATION_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
