# Enriched from F5 iRules reference documentation.
"""ntohs -- Converts the unsigned short integer from network byte order to host byte order."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ntohs.html"


@register
class NtohsCommand(CommandDef):
    name = "ntohs"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ntohs",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Converts the unsigned short integer from network byte order to host byte order.",
                synopsis=("ntohs NUMBER",),
                snippet=(
                    "Convert the unsigned short integer from network byte order to host byte\n"
                    "order."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n  set netshort 1234\n  set hostshort [ntohs $netshort]\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ntohs NUMBER",
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
