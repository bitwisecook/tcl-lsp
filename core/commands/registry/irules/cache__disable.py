# Enriched from F5 iRules reference documentation.
"""CACHE::disable -- Disables caching for this request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__disable.html"


@register
class CacheDisableCommand(CommandDef):
    name = "CACHE::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disables caching for this request.",
                synopsis=("CACHE::disable",),
                snippet=(
                    "Disables caching for this request.\n"
                    "\n"
                    "CACHE::disable\n"
                    "\n"
                    "     * Disables caching for this request."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  # Disable caching if the URI does not contain the string "images"\n'
                    '  if { not ([HTTP::uri] contains "images") } {\n'
                    "    CACHE::disable\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::disable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
