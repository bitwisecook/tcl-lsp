# Enriched from F5 iRules reference documentation.
"""URI::decode -- Returns a decoded version of a given URI."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/URI__decode.html"


@register
class UriDecodeCommand(CommandDef):
    name = "URI::decode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="URI::decode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a decoded version of a given URI.",
                synopsis=("URI::decode URI_STRING",),
                snippet=(
                    "Returns a URI decoded version of a given URI.\n"
                    "For details on URI encoding, see RFC3986, section 2.1. Percent-Encoding.\n"
                    "\n"
                    "This command is equivalent to the BIG-IP 4.X variable decode_uri."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  log local0. "The decoded version of \\"[HTTP::query]\\" is \\"[URI::decode [HTTP::query]]\\""\n'
                    "}"
                ),
                return_value="Returns a decoded version of a given URI.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="URI::decode URI_STRING",
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
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
