# Enriched from F5 iRules reference documentation.
"""crc32 -- Returns the crc32 checksum for the specified string."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/crc32.html"


@register
class Crc32Command(CommandDef):
    name = "crc32"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="crc32",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the crc32 checksum for the specified string.",
                synopsis=("crc32 ANY_CHARS",),
                snippet=(
                    "The crc32 command calculates a 32-bit cyclic redundancy check value\n"
                    "(CRC) for the bytes in a string using the well-known CRC-32 (Ethernet\n"
                    "CRC) scheme. The polynomial is 0x04c11db7, the CRC register is\n"
                    "initialized with 0xffffffff, the input bytes are taken msb-first, and\n"
                    "the result is the complement of the final register value reflected.\n"
                    '(crc32 implements the scheme called "CRC-32" in this Catalogue of\n'
                    "Parametrised CRC Algorithms.)\n"
                    "crc32 returns a number, or the empty string if an error occurs."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "   # Create a hash value for the host based on crc32\n"
                    "   # This could also be based on md5 or any other implementation\n"
                    "   # of a hash like djb or something.\n"
                    "set key [crc32 [HTTP::host]]\n"
                    "\n"
                    "   # Modulo the hash value by 1 - odd goes to one member, even another\n"
                    "set key [expr {$key & 1}]\n"
                    "\n"
                    "   # Route the request to the pool member based on the modulus\n"
                    "   # of the hash value.\n"
                    "switch $key {\n"
                    "0 { pool my_pool member 1.2.3.4:80 }\n"
                    "1 { pool my_pool member 5.6.7.8:80 }\n"
                    "   }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="crc32 ANY_CHARS",
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
