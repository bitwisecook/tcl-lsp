# Enriched from F5 iRules reference documentation.
"""JSON::set -- Sets a JSON element (aka."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/JSON__set.html"


@register
class JsonSetCommand(CommandDef):
    name = "JSON::set"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="JSON::set",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets a JSON element (aka.",
                synopsis=("JSON::set JSON_ELEMENT JSON_TYPE (JSON_VALUE)?",),
                snippet=(
                    "Sets the value (content) of a JSON element, replacing any existing value. The given value should be according to the given type, as described below:\n"
                    "\n"
                    "null: Omit. JSON type null has no value.\n"
                    "boolean: 0 (false) or 1 (true).\n"
                    "integer: A Tcl number representing an integer in the range -(2^63) through (2^63 - 1). Otherwise, use the literal type.\n"
                    "literal: A Tcl string not requiring JSON escape sequences.\n"
                    "string: A Tcl string without escape sequences (certain characters will be replaced by JSON escape sequences).\n"
                    "object: Omit. An empty object is created.\n"
                    "array: Omit. An empty array is created."
                ),
                source=_SOURCE,
                examples=(
                    "when JSON_REQUEST {\n"
                    "    set rootval [JSON::root]\n"
                    "    JSON::set $rootval string HelloWorld\n"
                    "}"
                ),
                return_value="Returns the JSON element whose value was set (same element as the first argument passed to the command).",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="JSON::set JSON_ELEMENT JSON_TYPE (JSON_VALUE)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
