# Enriched from F5 iRules reference documentation.
"""ntohl -- Converts the unsigned integer from network byte order to host byte order."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ntohl.html"


@register
class NtohlCommand(CommandDef):
    name = "ntohl"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ntohl",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Converts the unsigned integer from network byte order to host byte order.",
                synopsis=("ntohl NUMBER",),
                snippet=(
                    "Convert the unsigned integer from network byte order to host byte\norder."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST{\n  set netlong 12345678\n  set hostlong [ntohl $netlong]\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ntohl NUMBER",
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
