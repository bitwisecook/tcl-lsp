# Enriched from F5 iRules reference documentation.
"""CACHE::enable -- Forces the document to be cached."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__enable.html"


@register
class CacheEnableCommand(CommandDef):
    name = "CACHE::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Forces the document to be cached.",
                synopsis=("CACHE::enable",),
                snippet=(
                    "Forces the document to be cached. You can also use this command to\n"
                    "cache non-GET requests.\n"
                    "\n"
                    "Note: Should be used with extreme caution, as it allows caching of content marked private by server.\n"
                    "\n"
                    "CACHE::enable\n"
                    "\n"
                    "     * Forces the document to be cached."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  if { [HTTP::uri] contains "images" } {\n'
                    "    CACHE::enable\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::enable",
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
