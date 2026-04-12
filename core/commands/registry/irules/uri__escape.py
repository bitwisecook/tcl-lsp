# Enriched from F5 iRules reference documentation.
"""URI::escape -- Percent-encodes a URI string (alias for URI::encode)."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/URI__encode.html"


@register
class UriEscapeCommand(CommandDef):
    name = "URI::escape"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="URI::escape",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Percent-encodes a URI string (alias for URI::encode).",
                synopsis=("URI::escape URI_STRING",),
                snippet=(
                    "Percent-encodes *URI_STRING* according to RFC 3986.\n"
                    "This is an alias for ``URI::encode``."
                ),
                source=_SOURCE,
                return_value="Returns a percent-encoded URI string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="URI::escape URI_STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            taint_transform=TaintColour.URL_ENCODED | TaintColour.CRLF_FREE,
            taint_double_encode_colour=TaintColour.URL_ENCODED,
        )
