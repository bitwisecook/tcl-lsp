"""sha1/sha2 -- SHA hash functions (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE_SHA1 = "tcllib sha1 package"
_SOURCE_SHA2 = "tcllib sha2 package"


@register
class Sha1Sha1Command(CommandDef):
    name = "sha1::sha1"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package="sha1",
            hover=HoverSnippet(
                summary="Compute the SHA-1 hash of a string or file.",
                synopsis=(
                    "sha1::sha1 ?-hex|-bin? ?-channel channel | -file filename | ?--? string?",
                ),
                source=_SOURCE_SHA1,
                return_value="The SHA-1 hash as a hex or binary string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="sha1::sha1 ?options? ?--? string",
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


@register
class Sha2Sha256Command(CommandDef):
    name = "sha2::sha256"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package="sha2",
            hover=HoverSnippet(
                summary="Compute the SHA-256 hash of a string or file.",
                synopsis=(
                    "sha2::sha256 ?-hex|-bin? ?-channel channel | -file filename | ?--? string?",
                ),
                source=_SOURCE_SHA2,
                return_value="The SHA-256 hash as a hex or binary string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="sha2::sha256 ?options? ?--? string",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
            pure=True,
        )
