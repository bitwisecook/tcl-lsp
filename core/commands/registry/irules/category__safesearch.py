# Enriched from F5 iRules reference documentation.
"""CATEGORY::safesearch -- Get safe search key and value pairs."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, OptionSpec, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CATEGORY__safesearch.html"


@register
class CategorySafesearchCommand(CommandDef):
    name = "CATEGORY::safesearch"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CATEGORY::safesearch",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get safe search key and value pairs.",
                synopsis=("CATEGORY::safesearch URL ('-ip' IP)?",),
                snippet="Checks for safe search parameters for the given URL, returns them in list form with the first entry being the key, and the second being the value. Repeated in list for multiple results. (requires SWG license)",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    set this_uri http://[HTTP::host][HTTP::uri]\n"
                    "    set reply [CATEGORY::safesearch $this_uri]\n"
                    "    set len [llength $reply]\n"
                    "    if { $len equals 2 } {\n"
                    '        log local0. "uri $this_uri returns safesearch key=[lindex $reply 0] and value=[lindex $reply 1]"\n'
                    '        if { not([HTTP::uri] contains "&[lindex $reply 0]=[lindex $reply 1]") } {\n'
                    "            HTTP::uri [HTTP::uri]&[lindex $reply 0]=[lindex $reply 1]\n"
                    "        }\n"
                    "    }\n"
                    "}"
                ),
                return_value="Returns a list of alternating key and value pairs. E.g.: [key1, value1, key2, value2]",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CATEGORY::safesearch URL ('-ip' IP)?",
                    options=(OptionSpec(name="-ip", detail="Option -ip.", takes_value=True),),
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"CATEGORY", "FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CLASSIFICATION_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
