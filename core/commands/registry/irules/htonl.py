# Enriched from F5 iRules reference documentation.
"""htonl -- Converts the unsigned integer from host byte order to network byte order."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/htonl.html"


@register
class HtonlCommand(CommandDef):
    name = "htonl"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="htonl",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Converts the unsigned integer from host byte order to network byte order.",
                synopsis=("htonl NUMBER",),
                snippet=(
                    "Convert the unsigned integer from host byte order to network byte\norder."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  set hostlong 12345678\n"
                    "  set netlong [htonl $hostlong]\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="htonl NUMBER",
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
