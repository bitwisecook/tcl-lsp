# Enriched from F5 iRules reference documentation.
"""http_cookie -- Returns the value of the specified HTTP cookie."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .http__cookie import HttpCookieCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/http_cookie.html"


@register
class DeprecatedHttpCookieCommand(CommandDef):
    name = "http_cookie"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http_cookie",
            deprecated_replacement=HttpCookieCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the value of the specified HTTP cookie.",
                synopsis=("http_cookie ANY_CHARS",),
                snippet=(
                    "Returns the value in the Cookie: header for the specified cookie\n"
                    "name. This is a BIG-IP version 4.X variable, provided for\n"
                    "backward-compatibility. You can use the equivalent 9.X command\n"
                    "HTTP::cookie instead"
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="http_cookie ANY_CHARS",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_COOKIE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
