# Enriched from F5 iRules reference documentation.
"""AES::decrypt -- Decrypts the data using the previously-created AES key."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AES__decrypt.html"


@register
class AesDecryptCommand(CommandDef):
    name = "AES::decrypt"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AES::decrypt",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Decrypts the data using the previously-created AES key.",
                synopsis=("AES::decrypt KEY DATA",),
                snippet="Decrypt the data using an AES key.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  set key "AES 128 43047ad71173be644498b98de6a32fe3"\n'
                    "  set decryptedData [AES::decrypt $key $encryptedData]\n"
                    '  log local0. "The decrypted data is $decryptedData"\n'
                    "}"
                ),
                return_value="Returns the decrypted data.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AES::decrypt KEY DATA",
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
