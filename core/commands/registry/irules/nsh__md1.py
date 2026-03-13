# Enriched from F5 iRules reference documentation.
"""NSH::md1 -- Sets/Get the MD1 context for NSH."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/NSH__md1.html"


@register
class NshMd1Command(CommandDef):
    name = "NSH::md1"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="NSH::md1",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets/Get the MD1 context for NSH.",
                synopsis=("NSH::md1 DIRECTION UNSIGNED_INT UNSIGNED_INT (METADATA)?",),
                snippet=(
                    "Set: MD1 context for NSH. Offset, length and data string as arguments.\n"
                    "            Get: MD1 context from NSH. Only offset and length as arguments."
                ),
                source=_SOURCE,
                examples=(
                    "ntext for NSH.\n"
                    "            when CLIENT_ACCEPTED {\n"
                    "                set str {1234567890123456}\n"
                    "                NSH::md1 serverside_egress 1 16 [binary format a* $str]\n"
                    "                set myctx1 [NSH::md1 serverside_egress 1 16]\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="NSH::md1 DIRECTION UNSIGNED_INT UNSIGNED_INT (METADATA)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
