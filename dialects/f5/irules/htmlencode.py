"""htmlencode -- HTML-encode a string (iRules helper alias)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour

from ._base import _IRULES_ONLY, register


@register
class HtmlencodeCommand(CommandDef):
    name = "htmlencode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="htmlencode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="HTML-encode a string (alias for HTML::encode).",
                synopsis=("htmlencode STRING",),
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
                    synopsis="htmlencode STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            taint_transform=TaintColour.HTML_ESCAPED | TaintColour.CRLF_FREE,
            taint_double_encode_colour=TaintColour.HTML_ESCAPED,
        )
