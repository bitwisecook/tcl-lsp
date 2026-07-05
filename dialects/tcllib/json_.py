# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

"""json -- JSON encoding and decoding (tcllib)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from compiler.types import TclType

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
