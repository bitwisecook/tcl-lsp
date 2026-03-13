# Enriched from F5 iRules reference documentation.
"""CRYPTO::hash -- Generates a hash on a piece of data."""

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

_SOURCE = "https://clouddocs.f5.com/api/irules/CRYPTO__hash.html"


_av = make_av(_SOURCE)


@register
class CryptoHashCommand(CommandDef):
    name = "CRYPTO::hash"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CRYPTO::hash",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Generates a hash on a piece of data.",
                synopsis=(
                    "CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'",
                ),
                snippet=(
                    "This iRules command generates a hash on a piece of data\n"
                    "\n"
                    "CRYPTO::hash [-alg <>] [-ctx <> [-final]] [<data>]\n"
                    "\n"
                    "     * Generates a hash on a piece of data\n"
                    "\n"
                    "Algorithm List\n"
                    "\n"
                    "     * md5\n"
                    "     * ripemd160\n"
                    "     * sha1\n"
                    "     * sha224\n"
                    "     * sha256\n"
                    "     * sha384\n"
                    "     * sha512"
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "if {[class match [b64encode [CRYPTO::hash -alg sha384 [HTTP::host][HTTP::path]]] equals HASH ]} {\n"
                    '    log local0. " this FQDN + PATH is mathing - [HTTP::host][HTTP::path]"\n'
                    "}\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'",
                    options=(OptionSpec(name="-alg", detail="Option -alg.", takes_value=True),),
                    arg_values={
                        0: (
                            _av(
                                "md5",
                                "CRYPTO::hash md5",
                                "CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'",
                            ),
                            _av(
                                "ripemd160",
                                "CRYPTO::hash ripemd160",
                                "CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'",
                            ),
                            _av(
                                "sha1",
                                "CRYPTO::hash sha1",
                                "CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'",
                            ),
                            _av(
                                "sha224",
                                "CRYPTO::hash sha224",
                                "CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'",
                            ),
                            _av(
                                "sha256",
                                "CRYPTO::hash sha256",
                                "CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'",
                            ),
                            _av(
                                "sha384",
                                "CRYPTO::hash sha384",
                                "CRYPTO::hash (('-alg' ('md5' | 'ripemd160' | 'sha1' | 'sha224' | 'sha256' | 'sha384'",
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
