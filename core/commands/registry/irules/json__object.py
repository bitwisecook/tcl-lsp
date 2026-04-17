# Enriched from F5 iRules reference documentation.
"""JSON::object -- A group of subcommands that operate on a JSON object."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/JSON__object.html"


@register
class JsonObjectCommand(CommandDef):
    name = "JSON::object"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="JSON::object",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="A group of subcommands that operate on a JSON object.",
                synopsis=("JSON::object (",),
                snippet="A group of subcommands that operate on a JSON object (first parameter of each subcommand).",
                source=_SOURCE,
                examples=(
                    "when JSON_REQUEST {\n"
                    "    set rootval [JSON::root]\n"
                    "    set obj [JSON::get $rootval object]\n"
                    "\n"
                    "    set size [JSON::object size $obj]\n"
                    "    set type_at_key [JSON::object type $obj somekey]\n"
                    "    set myint [JSON::object get $obj intkey integer]\n"
                    "    JSON::object set $obj intkey integer 500\n"
                    "    JSON::object add $obj namekey string John\n"
                    "    JSON::object remove $obj intkey\n"
                    "    set mykeylist [JSON::object keys $obj]\n"
                    "    set myvaluelist [JSON::object values $obj]\n"
                    "}"
                ),
                return_value="Return depends on subcommand. See syntax description for detail.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="JSON::object (",
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
