"""regex::quote -- Escape regex metacharacters in a string."""

from __future__ import annotations

from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ..taint_hints import TaintColour
from ._base import register


@register
class RegexQuoteCommand(CommandDef):
    name = "regex::quote"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="regex::quote",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Escape regex metacharacters in a string.",
                synopsis=("regex::quote STRING",),
                snippet=(
                    "Returns *STRING* with all regular-expression\n"
                    "metacharacters (``[ ] { } ( ) * + ? . \\\\ ^ $ |``)\n"
                    "backslash-escaped so it can be used as a literal\n"
                    "pattern in ``regexp`` or ``regsub``."
                ),
                examples=(
                    "set safe_pattern [regex::quote $user_input]\n"
                    "if {[regexp $safe_pattern $haystack]} { ... }"
                ),
                return_value="Returns a regex-escaped string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="regex::quote STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            taint_transform=TaintColour.REGEX_LITERAL,
            taint_double_encode_colour=TaintColour.REGEX_LITERAL,
        )
