# Enriched from F5 iRules reference documentation.
"""http_version -- F5 iRules command `http_version`."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .http__version import HttpVersionCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/http_version.html"


@register
class DeprecatedHttpVersionCommand(CommandDef):
    name = "http_version"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http_version",
            deprecated_replacement=HttpVersionCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the HTTP protocol version.",
                synopsis=("http_version",),
                snippet=(
                    'Returns the HTTP protocol version. Possible values are "HTTP/1.0" or\n'
                    '"HTTP/1.1". This is a BIG-IP version 4.X variable, provided for\n'
                    "backward compatibility. You can use the equivalent 9.X command,\n"
                    "HTTP::version instead."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="http_version",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_STATUS,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
