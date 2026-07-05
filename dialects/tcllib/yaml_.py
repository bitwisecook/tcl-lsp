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

"""yaml -- YAML parsing (tcllib)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_SOURCE = "tcllib yaml package"
_PACKAGE = "yaml"


@register
class YamlYaml2dictCommand(CommandDef):
    name = "yaml::yaml2dict"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Parse a YAML string and return a Tcl dict.",
                synopsis=("yaml::yaml2dict ?-file? yamlText",),
                source=_SOURCE,
                examples="set data [yaml::yaml2dict $yamlString]",
                return_value="A Tcl dict representing the YAML structure.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="yaml::yaml2dict ?-file? yamlText",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )


@register
class YamlDict2yamlCommand(CommandDef):
    name = "yaml::dict2yaml"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Convert a Tcl dict to a YAML string.",
                synopsis=("yaml::dict2yaml dictValue ?indent? ?wordwrap?",),
                source=_SOURCE,
                return_value="A YAML-formatted string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="yaml::dict2yaml dictValue ?indent? ?wordwrap?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 3)),
        )


@register
class YamlYaml2huddleCommand(CommandDef):
    name = "yaml::yaml2huddle"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Parse a YAML string and return a huddle object.",
                synopsis=("yaml::yaml2huddle ?-file? yamlText",),
                source=_SOURCE,
                return_value="A huddle object representing the YAML structure.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="yaml::yaml2huddle ?-file? yamlText",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 2)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    reads=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class YamlSetOptionsCommand(CommandDef):
    name = "yaml::setOptions"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Set options for YAML processing.",
                synopsis=("yaml::setOptions optionDict",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="yaml::setOptions optionDict",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class YamlList2yamlCommand(CommandDef):
    name = "yaml::list2yaml"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Convert a Tcl list to a YAML string.",
                synopsis=("yaml::list2yaml listValue ?indent? ?wordwrap?",),
                source=_SOURCE,
                return_value="A YAML-formatted string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="yaml::list2yaml listValue ?indent? ?wordwrap?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 3)),
            pure=True,
        )


@register
class YamlHuddle2yamlCommand(CommandDef):
    name = "yaml::huddle2yaml"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Convert a huddle object to a YAML string.",
                synopsis=("yaml::huddle2yaml huddle ?indent? ?wordwrap?",),
                source=_SOURCE,
                return_value="A YAML-formatted string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="yaml::huddle2yaml huddle ?indent? ?wordwrap?",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 3)),
        )
