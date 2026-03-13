# Enriched from F5 iRules reference documentation.
"""CRYPTO::sign -- Provides a digital signature of a block of data."""

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

_SOURCE = "https://clouddocs.f5.com/api/irules/CRYPTO__sign.html"


_av = make_av(_SOURCE)


@register
class CryptoSignCommand(CommandDef):
    name = "CRYPTO::sign"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CRYPTO::sign",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Provides a digital signature of a block of data.",
                synopsis=(
                    "CRYPTO::sign (('-alg' ('hmac-md5' | 'hmac-ripemd160' | 'hmac-sha1' | 'hmac-sha224'",
                ),
                snippet=(
                    "This iRules command is used to provide a digital signature of a block\n"
                    "of data.\n"
                    "\n"
                    "CRYPTO::sign [-alg <>] [-ctx <> [-final]] [-key[hex] [<data>]\n"
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
                    synopsis="CRYPTO::sign (('-alg' ('hmac-md5' | 'hmac-ripemd160' | 'hmac-sha1' | 'hmac-sha224'",
                    options=(OptionSpec(name="-alg", detail="Option -alg.", takes_value=True),),
                    arg_values={
                        0: (
                            _av(
                                "hmac-md5",
                                "CRYPTO::sign hmac-md5",
                                "CRYPTO::sign (('-alg' ('hmac-md5' | 'hmac-ripemd160' | 'hmac-sha1' | 'hmac-sha224'",
                            ),
                            _av(
                                "hmac-ripemd160",
                                "CRYPTO::sign hmac-ripemd160",
                                "CRYPTO::sign (('-alg' ('hmac-md5' | 'hmac-ripemd160' | 'hmac-sha1' | 'hmac-sha224'",
                            ),
                            _av(
                                "hmac-sha1",
                                "CRYPTO::sign hmac-sha1",
                                "CRYPTO::sign (('-alg' ('hmac-md5' | 'hmac-ripemd160' | 'hmac-sha1' | 'hmac-sha224'",
                            ),
                            _av(
                                "hmac-sha224",
                                "CRYPTO::sign hmac-sha224",
                                "CRYPTO::sign (('-alg' ('hmac-md5' | 'hmac-ripemd160' | 'hmac-sha1' | 'hmac-sha224'",
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
