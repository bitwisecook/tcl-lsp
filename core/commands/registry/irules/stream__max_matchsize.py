# Enriched from F5 iRules reference documentation.
"""STREAM::max_matchsize -- Sets a maximum number of bytes that the system can buffer during partial matches."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/STREAM__max_matchsize.html"


@register
class StreamMaxMatchsizeCommand(CommandDef):
    name = "STREAM::max_matchsize"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="STREAM::max_matchsize",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sets a maximum number of bytes that the system can buffer during partial matches.",
                synopsis=("STREAM::max_matchsize SIZE",),
                snippet=(
                    "Sets the maximum size, in bytes, that the system can buffer during\n"
                    "partial matches. The default value is 4096.\n"
                    "The STREAM profile will buffer data for partial matches; if more than\n"
                    "max_matchsize would be buffered, the connection will be torn down. This\n"
                    "way a regex like foobarbaz+ won't keep matching until the box runs\n"
                    "out of memory. The default is 4K, and STREAM::max_matchsize can be\n"
                    "use to set it to something else."
                ),
                source=_SOURCE,
                examples=("when HTTP_RESPONSE {\n    STREAM::max_matchsize 2048\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="STREAM::max_matchsize SIZE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
