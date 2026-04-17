"""base64 -- Base64 encoding and decoding (tcllib)."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
from ._base import register

_SOURCE = "tcllib base64 package"
_PACKAGE = "base64"


@register
class Base64EncodeCommand(CommandDef):
    name = "base64::encode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Encode binary data as a base64 string.",
                synopsis=("base64::encode ?-maxlen maxlen? ?-wrapchar wrapchar? data",),
                source=_SOURCE,
                examples="set encoded [base64::encode $binaryData]",
                return_value="A base64-encoded string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="base64::encode ?-maxlen maxlen? ?-wrapchar wrapchar? data",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 5)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )


@register
class Base64DecodeCommand(CommandDef):
    name = "base64::decode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Decode a base64-encoded string back to binary data.",
                synopsis=("base64::decode encodedData",),
                source=_SOURCE,
                examples="set binary [base64::decode $encodedString]",
                return_value="The decoded binary data.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="base64::decode encodedData"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )
