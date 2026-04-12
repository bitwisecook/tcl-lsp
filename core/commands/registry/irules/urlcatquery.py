# Enriched from F5 iRules reference documentation.
"""urlcatquery -- Query the URL for URL categorization."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register
from .category__lookup import CategoryLookupCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/urlcatquery.html"


@register
class UrlcatqueryCommand(CommandDef):
    name = "urlcatquery"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="urlcatquery",
            deprecated_replacement=CategoryLookupCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Query the URL for URL categorization.",
                synopsis=("urlcatquery URL_STRING",),
                snippet=(
                    "This command is similar in functionality to whereis command of geoip.\n"
                    "This will be available from HTTP_REQUEST irule event. It takes the URL\n"
                    "as the input. The input could be a URL string or an IPV4 address. IPV6\n"
                    "addresses are not currently supported. iRule returns the URL categories\n"
                    "returned by the urlcat library."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    set input_url [HTTP::host][HTTP::uri]\n"
                    "    set urlcat [urlcatquery  $input_url]\n"
                    '    log local0. "INPUT-URL: $input_url"\n'
                    '    log local0. "Category - $urlcat"\n'
                    "    CLASSIFY::urlcat add $urlcat\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="urlcatquery URL_STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DATA_GROUP,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
