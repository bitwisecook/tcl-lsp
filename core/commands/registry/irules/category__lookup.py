# Enriched from F5 iRules reference documentation.
"""CATEGORY::lookup -- Get category of URL."""

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

_SOURCE = "https://clouddocs.f5.com/api/irules/CATEGORY__lookup.html"


_av = make_av(_SOURCE)


@register
class CategoryLookupCommand(CommandDef):
    name = "CATEGORY::lookup"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CATEGORY::lookup",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get category of URL.",
                synopsis=(
                    "CATEGORY::lookup URL ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_custom')? ('-ip' IP)? ('-custom_cat_match' ANY_CHARS)?",
                ),
                snippet=(
                    "This command returns the category of the supplied URL. (requires SWG license)\n"
                    "The URL needs to be of the form:\n"
                    "scheme://domain:port/path?query_string#fragment_id\n"
                    "\n"
                    'The query_string and fragment_id are optional. The entire list of categories supported is available in the UI under "Secure Web Gateway" in the APM section. Examples of categories include Sports, Shopping, etc. The response is a list of category names in a TCL array. Most input URLs result in a single category but some will return more than one.'
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "        set this_uri http://[HTTP::host][HTTP::uri]\n"
                    "        set reply [CATEGORY::lookup $this_uri]\n"
                    '        log local0. "Category lookup for $this_uri give $reply"\n'
                    "    }"
                ),
                return_value="Returns a list of categories returned by the categorization engine depending on the category type specified (custom, request_default, or request_default_and_custom). If no type is specified, it will return request_default.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CATEGORY::lookup URL ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_custom')? ('-ip' IP)? ('-custom_cat_match' ANY_CHARS)?",
                    options=(
                        OptionSpec(name="-display", detail="Option -display.", takes_value=True),
                        OptionSpec(name="-id", detail="Option -id.", takes_value=True),
                        OptionSpec(name="-ip", detail="Option -ip.", takes_value=True),
                        OptionSpec(
                            name="-custom_cat_match",
                            detail="Option -custom_cat_match.",
                            takes_value=True,
                        ),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "custom",
                                "CATEGORY::lookup custom",
                                "CATEGORY::lookup URL ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_custom')? ('-ip' IP)? ('-custom_cat_match' ANY_CHARS)?",
                            ),
                            _av(
                                "request_default",
                                "CATEGORY::lookup request_default",
                                "CATEGORY::lookup URL ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_custom')? ('-ip' IP)? ('-custom_cat_match' ANY_CHARS)?",
                            ),
                            _av(
                                "request_default_and_custom",
                                "CATEGORY::lookup request_default_and_custom",
                                "CATEGORY::lookup URL ('-display' | '-id')? ('custom' | 'request_default' | 'request_default_and_custom')? ('-ip' IP)? ('-custom_cat_match' ANY_CHARS)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CLASSIFICATION_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
