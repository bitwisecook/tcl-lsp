# Enriched from F5 iRules reference documentation.
"""JSON::type -- Gets the type of the given JSON element (aka."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/JSON__type.html"


@register
class JsonTypeCommand(CommandDef):
    name = "JSON::type"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="JSON::type",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets the type of the given JSON element (aka.",
                synopsis=("JSON::type JSON_ELEMENT",),
                snippet="A JSON value can be one of many types. This command allows you to determine the type of value in the element.",
                source=_SOURCE,
                examples=(
                    "when JSON_REQUEST {\n"
                    "    set rootval [JSON::root]\n"
                    "    JSON::set $rootval string HelloWorld\n"
                    "    set type [JSON::type $rootval]\n"
                    "}"
                ),
                return_value="Returns a string representing the JSON type ('null' | 'boolean' | 'integer' | 'literal' | 'string' | 'object' | 'array' | 'invalid').",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="JSON::type JSON_ELEMENT",
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
