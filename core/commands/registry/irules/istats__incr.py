# Enriched from F5 iRules reference documentation.
"""ISTATS::incr -- Increments the specified key by the given value."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ISTATS__incr.html"


@register
class IstatsIncrCommand(CommandDef):
    name = "ISTATS::incr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ISTATS::incr",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Increments the specified key by the given value.",
                synopsis=("ISTATS::incr KEY VALUE",),
                snippet=(
                    "Increments the specified key by the given value. The increment value must be non-negative for a counter.\n"
                    "\n"
                    "Note that text string iStats may not be incremented."
                ),
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
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ISTATS::incr KEY VALUE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ISTATS,
                    writes=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
