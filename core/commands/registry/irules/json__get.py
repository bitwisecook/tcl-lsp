# Enriched from F5 iRules reference documentation.
"""JSON::get -- Gets the value content of a JSON element."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/JSON__get.html"


@register
class JsonGetCommand(CommandDef):
    name = "JSON::get"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="JSON::get",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets the value content of a JSON element.",
                synopsis=("JSON::get JSON_ELEMENT (JSON_TYPE)?",),
                snippet=(
                    "A JSON value can be one of many types. This command returns the value (content) of an element according to its type, as described below:\n"
                    "\n"
                    "null : An empty Tcl list.\n"
                    "boolean : 1 for true or 0 for false.\n"
                    "integer : A Tcl number representing an integer in the range -(2^63) through (2^63 - 1).\n"
                    "literal: A Tcl string not requiring JSON escape sequences.\n"
                    "string : A Tcl string without escape sequences (having been replaced by the characters they represent).\n"
                    "object : A JSON object handle.\n"
                    "array : A JSON array handle."
                ),
                source=_SOURCE,
                examples=(
                    "when JSON_REQUEST {\n"
                    "    set rootval [JSON::root]\n"
                    "    set content [JSON::get $rootval integer]\n"
                    '    log local0. "$content"\n'
                    "}"
                ),
                return_value="Returns the content held within the JSON element, according to the types listed in the above description.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="JSON::get JSON_ELEMENT (JSON_TYPE)?",
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
