# Enriched from F5 iRules reference documentation.
"""JSON::parse -- Parses JSON content into a JSON cache that can be manipulated using further JSON:: commands."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/JSON__parse.html"


@register
class JsonParseCommand(CommandDef):
    name = "JSON::parse"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="JSON::parse",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Parses JSON content into a JSON cache that can be manipulated using further JSON:: commands.",
                synopsis=("JSON::parse (JSON_STRING (JSON_MAX_ENTRIES)? )?",),
                snippet=(
                    "If a string is omitted, returns any JSON cache that preexists in the context in which this is executed. This is the normal case when the command is executed in the JSON_REQUEST or JSON_RESPONSE event.\n"
                    "If a string is provided, it is assumed to contain JSON and is parsed into a new JSON cache. This will be deleted when it is no longer referenced by a Tcl variable. This is useful when a JSON profile is not being used."
                ),
                source=_SOURCE,
                examples=("when JSON_REQUEST {\n    JSON::render\n}"),
                return_value="Returns a JSON cache instance handle to use for retrieving and overwriting content, and rendering.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="JSON::parse (JSON_STRING (JSON_MAX_ENTRIES)? )?",
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
