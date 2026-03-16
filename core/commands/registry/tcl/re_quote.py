"""re_quote -- Escape regex metacharacters in a string."""

from __future__ import annotations

from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ..taint_hints import TaintColour
from ._base import register


@register
class ReQuoteCommand(CommandDef):
    name = "re_quote"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="re_quote",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Escape regex metacharacters in a string.",
                synopsis=("re_quote STRING",),
                snippet=(
                    "Returns *STRING* with all regular-expression\n"
                    "metacharacters backslash-escaped so it can be\n"
                    "used as a literal pattern in ``regexp`` or\n"
                    "``regsub``.  Alias for ``regex::quote``."
                ),
                return_value="Returns a regex-escaped string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="re_quote STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            taint_transform=TaintColour.REGEX_LITERAL,
            taint_double_encode_colour=TaintColour.REGEX_LITERAL,
        )
