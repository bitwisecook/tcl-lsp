# Enriched from F5 iRules reference documentation.
"""http_method -- F5 iRules command `http_method`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .http__method import HttpMethodCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/http_method.html"


@register
class DeprecatedHttpMethodCommand(CommandDef):
    name = "http_method"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http_method",
            deprecated_replacement=HttpMethodCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the action of the HTTP request.",
                synopsis=("http_method",),
                snippet=(
                    "Returns the action of the HTTP request. Common values are GET and\n"
                    "POST. This command is a BIG-IP version 4.X variable, provided for\n"
                    "backward-compatibility. You can use the equivalent 9.Xcommand\n"
                    "HTTP::method instead."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="http_method",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_METHOD,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
