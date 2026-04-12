# Enriched from F5 iRules reference documentation.
"""MR::stream -- Start egressing bytes previously collected and stored."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__stream.html"


_av = make_av(_SOURCE)


@register
class MrStreamCommand(CommandDef):
    name = "MR::stream"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::stream",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Start egressing bytes previously collected and stored.",
                synopsis=("MR::stream ( 'end' )? (BYTES)",),
                snippet=(
                    "Start egressing bytes previously collected and stored say in sessionDB. If payload has been split in multiple segments, use end to indicate the final segment.\n"
                    "\n"
                    "SYNTAX\n"
                    "\n"
                    "MR::stream <payload>\n"
                    "    Stream payload segment.\n"
                    "\n"
                    "MR::stream end <payload>\n"
                    "    Stream payload segement. End indicates final segment."
                ),
                source=_SOURCE,
                examples=('when MR_EGRESS {\n    MR::stream end "abcd"\n}'),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::stream ( 'end' )? (BYTES)",
                    arg_values={
                        0: (_av("end", "MR::stream end", "MR::stream ( 'end' )? (BYTES)"),)
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
