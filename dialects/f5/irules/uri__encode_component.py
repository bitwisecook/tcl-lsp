# Enriched from F5 iRules reference documentation.
"""URI::encode_component -- Percent-encodes a URI component."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/URI__encode.html"


@register
class UriEncodeComponentCommand(CommandDef):
    name = "URI::encode_component"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="URI::encode_component",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Percent-encodes a single URI component.",
                synopsis=("URI::encode_component STRING",),
                snippet=(
                    "Percent-encodes a single URI component (path segment, query\n"
                    "parameter name or value, fragment, etc.) according to RFC 3986\n"
                    "section 2.1.  Unlike ``URI::encode`` this encodes every\n"
                    "reserved delimiter (``/``, ``?``, ``&``, ``=``, …) so the\n"
                    "result is safe to embed inside a larger URI without altering\n"
                    "its structure."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  set value "key=value&other"\n'
                    '  HTTP::uri "/search?q=[URI::encode_component $value]"\n'
                    "}"
                ),
                return_value="Returns a percent-encoded string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="URI::encode_component STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            taint_transform=TaintColour.URL_ENCODED | TaintColour.CRLF_FREE,
            taint_double_encode_colour=TaintColour.URL_ENCODED,
        )
