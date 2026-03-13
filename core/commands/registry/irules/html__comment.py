# Enriched from F5 iRules reference documentation.
"""HTML::comment -- Query and update HTML comment."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HTML__comment.html"


_av = make_av(_SOURCE)


@register
class HtmlCommentCommand(CommandDef):
    name = "HTML::comment"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HTML::comment",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Query and update HTML comment.",
                synopsis=("HTML::comment ((append STRING) | (prepend STRING) | remove)?",),
                snippet=(
                    "Queries, removes HTML comment or appends/prepends it by a string.\n"
                    "\n"
                    "HTML::comment\n"
                    "Return the entire HTML comment, including the opening and the closing delimiter.\n"
                    "\n"
                    "HTML::comment append <string>\n"
                    "Insert a string after the closing delimiter of the HTML comment; when multiple appends are issued, the inserted strings are ordered according to the sequence of the append commands as they are issued for the given comment."
                ),
                source=_SOURCE,
                examples=('when HTML_COMMENT_MATCHED {\n    HTML::comment append "some_string"\n}'),
                return_value="HTML::comment returns the entire HTML comment; others do not return anything.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HTML::comment ((append STRING) | (prepend STRING) | remove)?",
                    arg_values={
                        0: (
                            _av(
                                "append",
                                "HTML::comment append",
                                "HTML::comment ((append STRING) | (prepend STRING) | remove)?",
                            ),
                            _av(
                                "prepend",
                                "HTML::comment prepend",
                                "HTML::comment ((append STRING) | (prepend STRING) | remove)?",
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
