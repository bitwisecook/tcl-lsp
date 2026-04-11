# Enriched from F5 iRules reference documentation.
"""URI::compare -- Compares two URI's for equality."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/URI__compare.html"


@register
class UriCompareCommand(CommandDef):
    name = "URI::compare"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="URI::compare",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Compares two URI's for equality.",
                synopsis=("URI::compare URI_STRING URI_STRING",),
                snippet=(
                    "Compares two URI's as recommended by RFC2616 section 3.2.3.\n"
                    "\n"
                    "3.2.3 URI Comparison\n"
                    "\n"
                    "   When comparing two URIs to decide if they match or not, a client\n"
                    "   SHOULD use a case-sensitive octet-by-octet comparison of the entire\n"
                    "   URIs, with these exceptions:\n"
                    "\n"
                    "      - A port that is empty or not given is equivalent to the default\n"
                    "        port for that URI-reference;\n"
                    "\n"
                    "        - Comparisons of host names MUST be case-insensitive;\n"
                    "\n"
                    "        - Comparisons of scheme names MUST be case-insensitive;\n"
                    "\n"
                    '        - An empty abs_path is equivalent to an abs_path of "/".'
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  set uri_to_check "/dir1/somepath"\n'
                    "  if { [URI::compare [HTTP::uri] $uri_to_check] } {\n"
                    '    log local0. "URI\'s are equal!"\n'
                    "  }\n"
                    "}"
                ),
                return_value="Returns 1 if URIs match; 0 otherwise.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="URI::compare URI_STRING URI_STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_URI,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
