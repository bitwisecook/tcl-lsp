# Enriched from F5 iRules reference documentation.
"""sha1 -- Returns the SHA version 1.0 message digest of the specified string."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/sha1.html"


@register
class Sha1Command(CommandDef):
    name = "sha1"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="sha1",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the SHA version 1.0 message digest of the specified string.",
                synopsis=("sha1 ANY_CHARS",),
                snippet="Returns the Secure Hash Algorithm version 1.0 (SHA1) message digest of the specified string, or if an error occurs, an empty string. Used to ensure data integrity.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    binary scan [sha1 [HTTP::host]] w1 key\n"
                    "\n"
                    "    set key [expr {$key & 1}]\n"
                    "    switch $key {\n"
                    "        0 { pool my_pool member 1.2.3.4:80 }\n"
                    "        1 { pool my_pool member 5.6.7.8:80 }\n"
                    "    }\n"
                    "}"
                ),
                return_value="sha1 <string> Returns the Secure Hash Algorithm version 1.0 (SHA1) message digest of the specified string, or if an error occurs, an empty string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="sha1 ANY_CHARS",
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
