# Enriched from F5 iRules reference documentation.
"""sha256 -- Returns the Secure Hash Algorithm (SHA2) 256-bit message digest of the specified string."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/sha256.html"


@register
class Sha256Command(CommandDef):
    name = "sha256"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="sha256",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the Secure Hash Algorithm (SHA2) 256-bit message digest of the specified string.",
                synopsis=("sha256 ANY_CHARS",),
                snippet="Returns the Secure Hash Algorithm (SHA2) 256-bit message digest of the specified string. If an error occurs, an empty string is returned. Used to ensure data integrity.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    binary scan [sha256 [HTTP::host]] w1 key\n"
                    "\n"
                    "    set key [expr {$key & 1}]\n"
                    "    switch $key {\n"
                    "        0 { pool my_pool member 1.2.3.4:80 }\n"
                    "        1 { pool my_pool member 5.6.7.8:80 }\n"
                    "    }\n"
                    "}"
                ),
                return_value="sha256 <string> Returns the Secure Hash Algorithm version 2.0 (SHA2) message digest of the specified string using 256 bit digest length. If an error occurs, an empty string is returned.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="sha256 ANY_CHARS",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(init_only=True),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
