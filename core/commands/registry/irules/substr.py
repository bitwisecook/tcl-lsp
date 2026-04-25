# Enriched from F5 iRules reference documentation.
"""substr -- Returns a substring from a string."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/substr.html"


@register
class SubstrCommand(CommandDef):
    name = "substr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="substr",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a substring from a string.",
                synopsis=("substr STRING SKIP_COUNT (TERMINATOR)?",),
                snippet=(
                    "A custom iRule function which returns a substring named <string>,\n"
                    "based on the values of the <skip_count> and <terminator> arguments.\n"
                    "Note the following:\n"
                    "  * The <skip_count> and <terminator> arguments are used in the same\n"
                    "    way as they are for the findstr command.\n"
                    "  * The <skip_count> argument is the index into <string> of the first\n"
                    "    character to be returned, where 0 indicates the first character of\n"
                    "    <string>.\n"
                    "  * The <terminator> argument can be either the subtring length or the\n"
                    "    substring terminating string."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  set uri [substr $uri 1 "?"]\n'
                    '  log local0. "Uri Part = $uri"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="substr STRING SKIP_COUNT (TERMINATOR)?",
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
