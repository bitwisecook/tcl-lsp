# Enriched from F5 iRules reference documentation.
"""AES::encrypt -- Encrypts the data using the previously-created AES key."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AES__encrypt.html"


@register
class AesEncryptCommand(CommandDef):
    name = "AES::encrypt"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AES::encrypt",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Encrypts the data using the previously-created AES key.",
                synopsis=("AES::encrypt KEY DATA",),
                snippet="Encrypt the data using an AES key.",
                source=_SOURCE,
                examples=(
                    "when SERVER_DATA {\n"
                    '  set key "AES 128 43047ad71173be644498b98de6a32fe3"\n'
                    "  set encryptedData [AES::encrypt $key [TCP::payload]]\n"
                    "  TCP::payload replace 0 [TCP::payload length] $encryptedData\n"
                    "}"
                ),
                return_value="Returns the encrypted data.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AES::encrypt KEY DATA",
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
