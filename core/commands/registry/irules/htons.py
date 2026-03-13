# Enriched from F5 iRules reference documentation.
"""htons -- Converts the unsigned short integer from host byte order to network byte order."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/htons.html"


@register
class HtonsCommand(CommandDef):
    name = "htons"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="htons",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Converts the unsigned short integer from host byte order to network byte order.",
                synopsis=("htons NUMBER",),
                snippet=(
                    "Convert the unsigned short integer from host byte order to network byte\n"
                    "order."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  set hostshort 1234\n"
                    "  set netshort [htons $hostshort]\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="htons NUMBER",
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
