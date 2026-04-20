# Enriched from F5 iRules reference documentation.
"""http_header -- F5 iRules command `http_header`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .http__header import HttpHeaderCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/http_header.html"


@register
class DeprecatedHttpHeaderCommand(CommandDef):
    name = "http_header"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http_header",
            deprecated_replacement=HttpHeaderCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Evaluates the string following an HTTP header tag that you specify.",
                synopsis=("http_header",),
                snippet=(
                    "Evaluates the string following an HTTP header tag that you specify.\n"
                    "This command is a BIG-IP version 4.X variable, provided for\n"
                    "backward-compatibility. You can use the equivalent 9.X command\n"
                    "HTTP::header, instead."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="http_header",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_HEADER,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
