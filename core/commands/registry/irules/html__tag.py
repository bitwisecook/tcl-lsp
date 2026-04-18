# Enriched from F5 iRules reference documentation.
"""HTML::tag -- Query and update the HTML tag."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTML__tag.html"


_av = make_av(_SOURCE)


@register
class HtmlTagCommand(CommandDef):
    name = "HTML::tag"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTML::tag",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Query and update the HTML tag.",
                synopsis=(
                    "HTML::tag ((append STRING) | name | (prepend STRING) | remove)",
                    "HTML::tag append <data>",
                    "HTML::tag name",
                    "HTML::tag prepend <data>",
                ),
                snippet=(
                    "Queries, removes and changes attribute/value pairs of this HTML tag.\n"
                    "        \n"
                    "HTML::tag append <data>\n"
                    "Insert a string after the closing delimiter of the HTML tag; when multiple appends are issued, the inserted strings are ordered according to the sequence of the append commands as they are issued for the given tag.\n"
                    "\n"
                    "HTML::tag name\n"
                    'Return HTML tag name, where name is the HTML element if the tag is a start tag, and if the tag is an end tag, tag name returns "/" + the HTML element.'
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    set uri [HTTP::uri]\n"
                    '    HTTP::header replace "Host" "finance.yahoo.com"\n'
                    "}"
                ),
                return_value='"HTML::tag name" returns tag name. "HTML::tag attribute value <name>" returns the value of the attribute under this HTML tag. "HTML::tag attribute count" returns the number of attributes in this HTML tag.',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTML::tag ((append STRING) | name | (prepend STRING) | remove)",
                    arg_values={
                        0: (
                            _av(
                                "append",
                                "HTML::tag append",
                                "HTML::tag ((append STRING) | name | (prepend STRING) | remove)",
                            ),
                            _av(
                                "prepend",
                                "HTML::tag prepend",
                                "HTML::tag ((append STRING) | name | (prepend STRING) | remove)",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"HTML"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
