# Enriched from F5 iRules reference documentation.
"""CRYPTO::verify -- Verifies a signed block of data."""

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

_SOURCE = "https://clouddocs.f5.com/api/irules/CRYPTO__verify.html"


_av = make_av(_SOURCE)


@register
class CryptoVerifyCommand(CommandDef):
    name = "CRYPTO::verify"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CRYPTO::verify",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Verifies a signed block of data.",
                synopsis=(
                    "CRYPTO::verify (('-alg' ('hmac-md5' | 'hmac-ripemd160' | 'hmac-sha1' | 'hmac-sha224'",
                ),
                snippet=(
                    "This iRules command is used to verify a signed block of data.\n"
                    "\n"
                    "CRYPTO::verify [-alg <>] [-ctx <> [-final]] [-key[hex] [-signature <>] [<data>]\n"
                    "\n"
                    "     * Used to provide a digital signature of a block of data. Notes on\n"
                    "       the flags:\n"
                    "          + alg - algorithm. ASCII string from a given list (see below)\n"
                    "            The spelling is lowercase and the iRule will fail for anything\n"
                    "            not in the list. In ctx mode, alg must be given in the first\n"
                    "            CRYPTO::command and cannot be modified."
                ),
                source=_SOURCE,
                examples=(
                    'set secret_key "foobar1234"\n'
                    "\n"
                    'set data "This is my data"\n'
                    "\n"
                    "set signed_data [CRYPTO::sign -alg hmac-sha1 -key $secret_key $data]\n"
                    "\n"
                    "if { [CRYPTO::verify -alg hmac-sha1 -key $secret_key -signature $signed_data $data] } {\n"
                    '    log local0. "Data verified"\n'
                    "}\n"
                    "\n"
                    "The secret key will normally be some large string, size generally\n"
                    "dictated by algorithm. The data is just whatever content you want to\n"
                    "sign. The result of the CRYPTO::sign command will be a binary value, so"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CRYPTO::verify (('-alg' ('hmac-md5' | 'hmac-ripemd160' | 'hmac-sha1' | 'hmac-sha224'",
                    options=(OptionSpec(name="-alg", detail="Option -alg.", takes_value=True),),
                    arg_values={
                        0: (
                            _av(
                                "hmac-md5",
                                "CRYPTO::verify hmac-md5",
                                "CRYPTO::verify (('-alg' ('hmac-md5' | 'hmac-ripemd160' | 'hmac-sha1' | 'hmac-sha224'",
                            ),
                            _av(
                                "hmac-ripemd160",
                                "CRYPTO::verify hmac-ripemd160",
                                "CRYPTO::verify (('-alg' ('hmac-md5' | 'hmac-ripemd160' | 'hmac-sha1' | 'hmac-sha224'",
                            ),
                            _av(
                                "hmac-sha1",
                                "CRYPTO::verify hmac-sha1",
                                "CRYPTO::verify (('-alg' ('hmac-md5' | 'hmac-ripemd160' | 'hmac-sha1' | 'hmac-sha224'",
                            ),
                            _av(
                                "hmac-sha224",
                                "CRYPTO::verify hmac-sha224",
                                "CRYPTO::verify (('-alg' ('hmac-md5' | 'hmac-ripemd160' | 'hmac-sha1' | 'hmac-sha224'",
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
