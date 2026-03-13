"""json -- JSON encoding and decoding (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "tcllib json package"
_PACKAGE = "json"


@register
class JsonJson2dictCommand(CommandDef):
    name = "json::json2dict"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Convert a JSON string to a Tcl dict.",
                synopsis=("json::json2dict jsonText",),
                snippet=(
                    "Parses a JSON-encoded string and returns a nested Tcl "
                    "dictionary. JSON objects become dicts, arrays become lists."
                ),
                source=_SOURCE,
                examples="set data [json::json2dict $jsonString]",
                return_value="A Tcl dict representing the JSON structure.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="json::json2dict jsonText"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )


@register
class JsonDict2jsonCommand(CommandDef):
    name = "json::dict2json"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Convert a Tcl dict to a JSON string.",
                synopsis=("json::dict2json dictValue",),
                snippet="Converts a Tcl dictionary to a JSON-encoded string.",
                source=_SOURCE,
                examples='set json [json::dict2json [dict create name "test" value 42]]',
                return_value="A JSON-encoded string.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="json::dict2json dictValue"),),
            validation=ValidationSpec(arity=Arity(1)),
        )
