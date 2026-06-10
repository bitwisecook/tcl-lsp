"""regexp::quote -- Escape regex metacharacters in a string."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.dialects import DIALECTS_EXCEPT_IRULES
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.registry.taint_hints import TaintColour

from ._base import register


@register
class RegexpQuoteCommand(CommandDef):
    name = "regexp::quote"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="regexp::quote",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Escape regex metacharacters in a string.",
                synopsis=("regexp::quote STRING",),
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
                    synopsis="regexp::quote STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1, 1),
            ),
            taint_transform=TaintColour.REGEX_LITERAL,
            taint_double_encode_colour=TaintColour.REGEX_LITERAL,
        )
