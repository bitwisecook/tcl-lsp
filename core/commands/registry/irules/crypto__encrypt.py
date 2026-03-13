# Enriched from F5 iRules reference documentation.
"""CRYPTO::encrypt -- Encrypts data."""

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

_SOURCE = "https://clouddocs.f5.com/api/irules/CRYPTO__encrypt.html"


_av = make_av(_SOURCE)


@register
class CryptoEncryptCommand(CommandDef):
    name = "CRYPTO::encrypt"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CRYPTO::encrypt",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This iRules command encrypts data.",
                synopsis=("CRYPTO::encrypt (('-padding' (pkcs | oaep | none) )",),
                snippet=(
                    "This iRules command encrypts data. A ciphertext encrypted with this\n"
                    "command should be decryptable by third party software.\n"
                    "\n"
                    "CRYPTO::encrypt [-alg <>] [-ctx <> [-final]] [-key[hex] <>] [-iv[hex] <>] [<data>]\n"
                    '                [-padding <"pkcs" | "oaep" | "none">]\n'
                    "\n"
                    "     * encrypts data based on several parameters\n"
                    "          + alg - algorithm. ASCII string from a given list (see below)\n"
                    "            The spelling is lowercase and the iRule will fail for anything\n"
                    "            not in the list. In ctx mode, alg must be given in the first\n"
                    "            CRYPTO:: command and cannot be modified."
                ),
                source=_SOURCE,
                examples=(
                    "Encrypt an MSISDN header\n"
                    "# Encrypt the MSISDN header for each request.\n"
                    "# The encryption is deliberately designed to be insecure;\n"
                    "# that is, the same MSISDN will always be encrypted to\n"
                    "# the same ciphertext. And since the IV will always be\n"
                    "# the same for each encryption, there's no need to send\n"
                    "# it out with the ciphertext.\n"
                    "#\n"
                    "when SIP_REQUEST {\n"
                    '    set key "abed1ddc04fbb05856bca4a0ca60f21e"\n'
                    '    set iv "d78d86d9084eb9239694c9a733904037"'
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CRYPTO::encrypt (('-padding' (pkcs | oaep | none) )",
                    options=(
                        OptionSpec(name="-padding", detail="Option -padding.", takes_value=True),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "pkcs",
                                "CRYPTO::encrypt pkcs",
                                "CRYPTO::encrypt (('-padding' (pkcs | oaep | none) )",
                            ),
                            _av(
                                "oaep",
                                "CRYPTO::encrypt oaep",
                                "CRYPTO::encrypt (('-padding' (pkcs | oaep | none) )",
                            ),
                            _av(
                                "none",
                                "CRYPTO::encrypt none",
                                "CRYPTO::encrypt (('-padding' (pkcs | oaep | none) )",
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
