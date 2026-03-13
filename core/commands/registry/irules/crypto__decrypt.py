# Enriched from F5 iRules reference documentation.
"""CRYPTO::decrypt -- Decrypts data."""

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

_SOURCE = "https://clouddocs.f5.com/api/irules/CRYPTO__decrypt.html"


_av = make_av(_SOURCE)


@register
class CryptoDecryptCommand(CommandDef):
    name = "CRYPTO::decrypt"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CRYPTO::decrypt",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This iRules command decrypts data.",
                synopsis=("CRYPTO::decrypt (('-padding' (pkcs | oaep | none) )",),
                snippet=(
                    "This iRules command decrypts data.\n"
                    "\n"
                    "CRYPTO::decrypt [-alg <>] [-ctx <> [-final]] [-key[hex] <>] [-iv[hex] <>] [<data>]\n"
                    '                [-padding <"pkcs" | "oaep" | "none">]\n'
                    "\n"
                    "     * decrypts data based on several parameters\n"
                    "          + alg - algorithm. ASCII string from a given list (see below)\n"
                    "            The spelling is lowercase and the iRule will fail for anything\n"
                    "            not in the list. In ctx mode, alg must be given in the first\n"
                    "            CRYPTO::command and cannot be modified."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CRYPTO::decrypt (('-padding' (pkcs | oaep | none) )",
                    options=(
                        OptionSpec(name="-padding", detail="Option -padding.", takes_value=True),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "pkcs",
                                "CRYPTO::decrypt pkcs",
                                "CRYPTO::decrypt (('-padding' (pkcs | oaep | none) )",
                            ),
                            _av(
                                "oaep",
                                "CRYPTO::decrypt oaep",
                                "CRYPTO::decrypt (('-padding' (pkcs | oaep | none) )",
                            ),
                            _av(
                                "none",
                                "CRYPTO::decrypt none",
                                "CRYPTO::decrypt (('-padding' (pkcs | oaep | none) )",
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
