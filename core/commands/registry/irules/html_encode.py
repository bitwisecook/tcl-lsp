"""html_encode -- HTML-encode a string (iRules helper alias)."""

from __future__ import annotations

from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ..taint_hints import TaintColour
from ._base import _IRULES_ONLY, register


@register
class HtmlEncodeAlias(CommandDef):
    name = "html_encode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="html_encode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="HTML-encode a string (alias for HTML::encode).",
                synopsis=("html_encode STRING",),
                snippet=(
                    "Replaces HTML-special characters with their entity\n"
                    "equivalents.  This is a convenience alias for\n"
                    "``HTML::encode``."
                ),
                return_value="Returns an HTML-escaped string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="html_encode STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            taint_transform=TaintColour.HTML_ESCAPED | TaintColour.CRLF_FREE,
            taint_double_encode_colour=TaintColour.HTML_ESCAPED,
        )
