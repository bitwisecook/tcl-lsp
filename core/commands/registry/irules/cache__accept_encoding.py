# Enriched from F5 iRules reference documentation.
"""CACHE::accept_encoding -- Overrides the accept_encoding value used by the cache to store the cached content."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CACHE__accept_encoding.html"


@register
class CacheAcceptEncodingCommand(CommandDef):
    name = "CACHE::accept_encoding"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CACHE::accept_encoding",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Overrides the accept_encoding value used by the cache to store the cached content.",
                synopsis=("CACHE::accept_encoding ENCODING_STRING",),
                snippet=(
                    "Overrides the accept_encoding value used by the cache to store the\n"
                    "cached content. You can use this command to group various user encoding\n"
                    "values into a single group, to minimize duplicated cached content.\n"
                    "\n"
                    "CACHE::accept_encoding <string>\n"
                    "\n"
                    "     * Overrides the accept_encoding value used by the cache to store the\n"
                    "       cached content, according to the specified string."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CACHE::accept_encoding ENCODING_STRING",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
