# Enriched from F5 iRules reference documentation.
"""http_uri -- F5 iRules command `http_uri`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .http__uri import HttpUriCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/http_uri.html"


@register
class DeprecatedHttpUriCommand(CommandDef):
    name = "http_uri"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http_uri",
            deprecated_replacement=HttpUriCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a URL, but does not include the protocol and the fully qualified domain name (FQDN).",
                synopsis=("http_uri",),
                snippet=(
                    "Returns a URL, but does not include the protocol and the fully\n"
                    "qualified domain name (FQDN). For example, if the URL is\n"
                    "http://www.mysite.com/buy.asp, then the URI is /buy.asp. This command\n"
                    "is a BIG-IP 4.X variable, provided for backward-compatibility. You can\n"
                    "use the equivalent 9.x command HTTP::uri instead."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="http_uri",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_URI,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
