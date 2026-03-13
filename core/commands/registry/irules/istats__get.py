# Enriched from F5 iRules reference documentation.
"""ISTATS::get -- Retrieves the value associated with the given key."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ISTATS__get.html"


@register
class IstatsGetCommand(CommandDef):
    name = "ISTATS::get"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ISTATS::get",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Retrieves the value associated with the given key.",
                synopsis=("ISTATS::get KEY",),
                snippet="Reads in the value associated with the given iStats key",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '        if { [string tolower [HTTP::uri]] equals "/12345" } {\n'
                    '                ISTATS::incr "uri /12345 counter Requests" 1\n'
                    '                HTTP::uri "/"\n'
                    '                HTTP::redirect "http://www.mysite.com"\n'
                    '        } elseif { [string tolower [HTTP::uri]] equals "/stats" } {\n'
                    '                  HTTP::respond 200 content "<html><body>Requests for /12345: [ISTATS::get "uri /12345 counter Requests"]</body></html>"\n'
                    "        }\n"
                    "}"
                ),
                return_value="Returns the value associated with the given key.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ISTATS::get KEY",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ISTATS,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
