# Enriched from F5 iRules reference documentation.
"""JSON::array -- A group of subcommands that operate on a JSON array."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/JSON__array.html"


@register
class JsonArrayCommand(CommandDef):
    name = "JSON::array"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="JSON::array",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="A group of subcommands that operate on a JSON array.",
                synopsis=("JSON::array (",),
                snippet="A group of subcommands that operate on a JSON array (first parameter of each subcommand).",
                source=_SOURCE,
                examples=(
                    "when JSON_REQUEST {\n"
                    "    set rootval [JSON::root]\n"
                    "    set ary [JSON::get $rootval array]\n"
                    "\n"
                    "    set size [JSON::array size $ary]\n"
                    "    set type_at_idx [JSON::array type $ary 2]\n"
                    "    set myint [JSON::array get $ary 1 integer]\n"
                    "    JSON::array set $ary 0 integer 500\n"
                    "    JSON::array insert $ary 5 string John\n"
                    "    JSON::array append $ary null\n"
                    "    JSON::array remove $ary 7\n"
                    "    set myvaluelist [JSON::array values $ary]\n"
                    "}"
                ),
                return_value="Return depends on subcommand. See syntax description for detail.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="JSON::array (",
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
