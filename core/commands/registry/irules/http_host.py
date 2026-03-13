# Enriched from F5 iRules reference documentation.
"""http_host -- Returns the value of the HTTP Host header."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .http__host import HttpHostCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/http_host.html"


@register
class DeprecatedHttpHostCommand(CommandDef):
    name = "http_host"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http_host",
            deprecated_replacement=HttpHostCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the value of the HTTP Host header.",
                synopsis=("http_host",),
                snippet=(
                    "Returns the value in the Host: header of the HTTP request. This is a\n"
                    "BIG-IP version 4.X variable, provided for backward-compatibility. You\n"
                    "can use the equivalent 9.X command HTTP::host instead."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="http_host",
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
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
