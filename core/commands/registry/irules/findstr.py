# Enriched from F5 iRules reference documentation.
"""findstr -- Finds a string within another string and returns the string starting at the offset specified from the match."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/findstr.html"


@register
class FindstrCommand(CommandDef):
    name = "findstr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="findstr",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Finds a string within another string and returns the string starting at the offset specified from the match.",
                synopsis=("findstr STRING SEARCH_STRING (",),
                snippet=(
                    "A custom iRule function which finds a string within another string\n"
                    "and returns the string starting at the offset specified from the match."
                ),
                source=_SOURCE,
                examples=(
                    "when RULE_INIT {\n"
                    '  set static::payload {<meta HTTP-EQUIV="REFRESH" CONTENT="0; URL=https://host.domain.com/path/file.ext?...&var=val">}\n'
                    '  set static::term {">}\n'
                    "  set urlresponse [findstr $static::payload URL= 4 $static::term]\n"
                    '  log local0. "urlresponse $urlresponse"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="findstr STRING SEARCH_STRING (",
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
