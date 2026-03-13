# Enriched from F5 iRules reference documentation.
"""rmd160 -- Returns the RIPEMD-160 message digest of the specified string."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/rmd160.html"


@register
class Rmd160Command(CommandDef):
    name = "rmd160"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="rmd160",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the RIPEMD-160 message digest of the specified string.",
                synopsis=("rmd160 ANY_CHARS",),
                snippet="Returns the RIPEMD-160 (RACE Integrity Primitives Evaluation Message Digest) message digest of the specified string, or an empty string if an error occurs. Used to ensure data integrity.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    binary scan [rmd160 [HTTP::host]] w1 key\n"
                    "\n"
                    "    set key [expr {$key & 1}]\n"
                    "    switch $key {\n"
                    "        0 { pool my_pool member 1.2.3.4:80 }\n"
                    "        1 { pool my_pool member 5.6.7.8:80 }\n"
                    "    }\n"
                    "}"
                ),
                return_value="rmd160 <string> Returns the RIPEMD-160 message digest of the specified string, or an empty string if an error occurs.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="rmd160 ANY_CHARS",
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
