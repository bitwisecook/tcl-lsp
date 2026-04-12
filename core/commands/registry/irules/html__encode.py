# Enriched from F5 iRules reference documentation.
"""HTML::encode -- HTML-encodes a string, escaping special characters."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/"


@register
class HtmlEncodeCommand(CommandDef):
    name = "HTML::encode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTML::encode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="HTML-encodes a string, escaping special characters.",
                synopsis=("HTML::encode STRING",),
                snippet=(
                    "Replaces HTML-special characters (``<``, ``>``, ``&``,\n"
                    "``\"``, ``'``) with their entity equivalents so the\n"
                    "string is safe to embed in an HTML text context."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  set user_input [HTTP::query]\n"
                    '  HTTP::respond 200 content "<p>[HTML::encode $user_input]</p>"\n'
                    "}"
                ),
                return_value="Returns an HTML-escaped string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTML::encode STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            taint_transform=TaintColour.HTML_ESCAPED | TaintColour.CRLF_FREE,
            taint_double_encode_colour=TaintColour.HTML_ESCAPED,
        )
