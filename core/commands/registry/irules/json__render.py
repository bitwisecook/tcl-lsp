# Enriched from F5 iRules reference documentation.
"""JSON::render -- Returns a string containing a textual rendering of the JSON cache content."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/JSON__render.html"


@register
class JsonRenderCommand(CommandDef):
    name = "JSON::render"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="JSON::render",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a string containing a textual rendering of the JSON cache content.",
                synopsis=("JSON::render (JSON_CACHE)?",),
                snippet=(
                    "If a JSON cache handle is omitted, renders any JSON cache that preexists in the context in which this is executed. This is the normal case when the command is executed in a the JSON_REQUEST or JSON_RESPONSE event.\n"
                    "If a JSON cache handle is provided, renders that JSON cache. This is useful when a JSON profile is not being used.\n"
                    "NOTE: Rendering consumes the data in the cache, so after a render, no further value retrieval/modification/rendering may be done on this JSON cache instance."
                ),
                source=_SOURCE,
                examples=(
                    "when MR_INGRESS {\n"
                    "    set cache [JSON::create]\n"
                    "    set rootval [JSON::root $cache]\n"
                    "    JSON::set $rootval string HelloWorld\n"
                    "    set rendered [JSON::render $cache]\n"
                    "}"
                ),
                return_value="Returns the string containing the rendered JSON content.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="JSON::render (JSON_CACHE)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
