"""yaml -- YAML parsing (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
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
