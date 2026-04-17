"""md5 -- MD5 hash function (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "tcllib md5 package"
_PACKAGE = "md5"


@register
class Md5Md5Command(CommandDef):
    name = "md5::md5"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Compute the MD5 hash of a string or file.",
                synopsis=(
                    "md5::md5 ?-hex|-bin? ?-channel channel | -file filename | ?--? string?",
                ),
                source=_SOURCE,
                return_value="The MD5 hash as a hex or binary string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="md5::md5 ?options? ?--? string",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
            pure=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
