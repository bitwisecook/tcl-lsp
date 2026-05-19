"""json -- JSON encoding and decoding (tcllib)."""

from __future__ import annotations

from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

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
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class JsonManyJson2dictCommand(CommandDef):
    name = "json::many-json2dict"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Convert a string containing multiple JSON values to a list of dicts.",
                synopsis=("json::many-json2dict jsonText ?max?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(kind=FormKind.DEFAULT, synopsis="json::many-json2dict jsonText ?max?"),
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
            pure=True,
            return_type=TclType.LIST,
        )


@register
class JsonValidateCommand(CommandDef):
    name = "json::validate"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Validate whether a string is valid JSON.",
                synopsis=("json::validate jsonText",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="json::validate jsonText"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
            return_type=TclType.BOOLEAN,
        )


@register
class JsonList2jsonCommand(CommandDef):
    name = "json::list2json"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Convert a Tcl list to a JSON array.",
                synopsis=("json::list2json list",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="json::list2json list"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )


@register
class JsonString2jsonCommand(CommandDef):
    name = "json::string2json"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Convert a Tcl string to a JSON string value.",
                synopsis=("json::string2json string",),
                source=_SOURCE,
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="json::string2json string"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )
