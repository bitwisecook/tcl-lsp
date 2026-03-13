# Enriched from F5 iRules reference documentation.
"""CRYPTO::keygen -- Generates keys that can be used to encrypt and sign data."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CRYPTO__keygen.html"


_av = make_av(_SOURCE)


@register
class CryptoKeygenCommand(CommandDef):
    name = "CRYPTO::keygen"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CRYPTO::keygen",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Generates keys that can be used to encrypt and sign data.",
                synopsis=("CRYPTO::keygen (('-alg' ('random' | 'pbkdf2-md5' | 'rsa'))",),
                snippet=(
                    "This iRules command is used to generate keys that can be used to\n"
                    "encrypt and sign data.\n"
                    "\n"
                    "CRYPTO::keygen -alg <> -len <> [-passphrase <> -salt[hex] <> -rounds <>]\n"
                    "\n"
                    "     * Used to generate keys that can be used to encrypt and sign data.\n"
                    "          + -alg (Two options: random or pbkdf2-md5)\n"
                    "          + -len (Must be a multiple of 8, e.g."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CRYPTO::keygen (('-alg' ('random' | 'pbkdf2-md5' | 'rsa'))",
                    options=(OptionSpec(name="-alg", detail="Option -alg.", takes_value=True),),
                    arg_values={
                        0: (
                            _av(
                                "random",
                                "CRYPTO::keygen random",
                                "CRYPTO::keygen (('-alg' ('random' | 'pbkdf2-md5' | 'rsa'))",
                            ),
                            _av(
                                "pbkdf2-md5",
                                "CRYPTO::keygen pbkdf2-md5",
                                "CRYPTO::keygen (('-alg' ('random' | 'pbkdf2-md5' | 'rsa'))",
                            ),
                            _av(
                                "rsa",
                                "CRYPTO::keygen rsa",
                                "CRYPTO::keygen (('-alg' ('random' | 'pbkdf2-md5' | 'rsa'))",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
