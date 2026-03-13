# Enriched from F5 iRules reference documentation.
"""CATEGORY::matchtype -- Get the type of match found."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CATEGORY__matchtype.html"


@register
class CategoryMatchtypeCommand(CommandDef):
    name = "CATEGORY::matchtype"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CATEGORY::matchtype",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get the type of match found.",
                synopsis=("CATEGORY::matchtype TYPE",),
                snippet='This iRules command is intended to be used with the CATEGORY_MATCHED event and will store the match result in the specified variable. It will return one of "custom", "request_default", or "request_default_and_custom". This tells the admin what kind of match was made when the CATEGORY_MATCHED event was raised – custom category match, match from the Websense categorization engine, or both. (requires SWG license)',
                source=_SOURCE,
                examples=(
                    "when CATEGORY_MATCHED {\n"
                    "    CATEGORY::matchtype type_var\n"
                    '        if { $type_var eq "custom" } {\n'
                    '            log local0. "Custom category match was found."\n'
                    "        }\n"
                    "}"
                ),
                return_value='Returns one of "custom", "request_default", "request_default_and_custom"',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CATEGORY::matchtype TYPE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"CATEGORY"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CLASSIFICATION_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
