# Enriched from F5 iRules reference documentation.
"""SCTP::payload -- Returns or replaces SCTP data content."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SCTP__payload.html"


@register
class SctpPayloadCommand(CommandDef):
    name = "SCTP::payload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SCTP::payload",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or replaces SCTP data content.",
                synopsis=("SCTP::payload ( (PAYLOAD_LENGTH) |",),
                snippet=(
                    "Returns the accumulated SCTP data content, or replaces collected payload with the specified data.\n"
                    "\n"
                    "SCTP::payload [<number_of_bytes>]\n"
                    "    Returns the accumulated SCTP payload data content upto size bytes if provided.\n"
                    "\n"
                    "SCTP::payload <offset> <number_of_bytes>\n"
                    "    Returns upto number_of_bytes bytes of the accumulated SCTP payload data content starting from the offset provided.\n"
                    "\n"
                    "SCTP::payload replace <offset> <length> <data>\n"
                    "    Replaces collected payload data with the given data, starting at offset.\n"
                    "\n"
                    "SCTP::payload length\n"
                    "    Returns the amount of accumulated SCTP data content in bytes."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n  SCTP::collect 15\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SCTP::payload ( (PAYLOAD_LENGTH) |",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
