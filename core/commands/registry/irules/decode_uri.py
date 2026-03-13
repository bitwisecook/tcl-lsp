# Enriched from F5 iRules reference documentation.
"""decode_uri -- Decodes the specified string using HTTP URI encoding."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .uri__decode import UriDecodeCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/decode_uri.html"


@register
class DecodeUriCommand(CommandDef):
    name = "decode_uri"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="decode_uri",
            deprecated_replacement=UriDecodeCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Decodes the specified string using HTTP URI encoding.",
                synopsis=("decode_uri ANY_CHARS",),
                snippet=(
                    "Decodes the specified string using HTTP URI encoding per RFC2616 and\n"
                    "returns the result. This is a BIG-IP 4.x variable, provided for\n"
                    "backward-compatibiliy. You can use the equivalent 9.X commmand\n"
                    "URI::decode instead."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="decode_uri ANY_CHARS",
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
