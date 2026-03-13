# Enriched from F5 iRules reference documentation.
"""CATEGORY::result -- Returns the category or safesearch results retrieved during normal traffic flow."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CATEGORY__result.html"


_av = make_av(_SOURCE)


@register
class CategoryResultCommand(CommandDef):
    name = "CATEGORY::result"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CATEGORY::result",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the category or safesearch results retrieved during normal traffic flow.",
                synopsis=(
                    "CATEGORY::result (('category' ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_custom')?) | 'safesearch')",
                ),
                snippet=(
                    "This iRule command is useful for when it is necessary to know the category or safesearch parameters returned during the categorization in the Category Lookup Agent in the per-request policy. As opposed to CATEGORY::lookup and CATEGORY::safesearch, which each require an additional query to the categorization engine, CATEGORY::result will give back what was found and stored, eliminating the need for additional lookups.\n"
                    "\n"
                    'Choose which should be returned (either "category" or "safesearch"). If "category", additional specifications may apply: "-display" will return categories in display name format.'
                ),
                source=_SOURCE,
                examples=(
                    "when CATEGORY_MATCHED {\n"
                    "    set cat [CATEGORY::result category -display request_default_and_custom]\n"
                    '    log local0. "Category result retrieved: [lindex $cat 0]"\n'
                    "    set ss [CATEGORY::result safesearch]\n"
                    '    log local0. "Safe Search result retrieved: [lindex $ss 0], [lindex $ss 1]"\n'
                    "}"
                ),
                return_value="Returns a list of categories or safe search parameters. Return format is the same as CATEGORY::lookup and CATEGORY::safesearch.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CATEGORY::result (('category' ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_custom')?) | 'safesearch')",
                    options=(
                        OptionSpec(name="-display", detail="Option -display.", takes_value=True),
                        OptionSpec(name="-id", detail="Option -id.", takes_value=True),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "category",
                                "CATEGORY::result category",
                                "CATEGORY::result (('category' ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_custom')?) | 'safesearch')",
                            ),
                            _av(
                                "custom",
                                "CATEGORY::result custom",
                                "CATEGORY::result (('category' ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_custom')?) | 'safesearch')",
                            ),
                            _av(
                                "request_default",
                                "CATEGORY::result request_default",
                                "CATEGORY::result (('category' ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_custom')?) | 'safesearch')",
                            ),
                            _av(
                                "request_default_and_custom",
                                "CATEGORY::result request_default_and_custom",
                                "CATEGORY::result (('category' ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_custom')?) | 'safesearch')",
                            ),
                            _av(
                                "safesearch",
                                "CATEGORY::result safesearch",
                                "CATEGORY::result (('category' ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_custom')?) | 'safesearch')",
                            ),
                        )
                    },
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
