# Enriched from F5 iRules reference documentation.
"""AES::key -- Creates an AES key to encrypt/decrypt data."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AES__key.html"


@register
class AesKeyCommand(CommandDef):
    name = "AES::key"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AES::key",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Creates an AES key to encrypt/decrypt data.",
                synopsis=("AES::key ('128' | '192' | '256')?",),
                snippet=(
                    "Creates an AES key of the specified length for use in\n"
                    "encryption/decryption operations."
                ),
                source=_SOURCE,
                examples=("when RULE_INIT {\n    set ::key [AES::key 128]\n}"),
                return_value="Returns the created key.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AES::key ('128' | '192' | '256')?",
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
