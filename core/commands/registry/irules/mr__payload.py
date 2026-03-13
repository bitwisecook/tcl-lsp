# Enriched from F5 iRules reference documentation.
"""MR::payload -- Access data collected using MR::collect command."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__payload.html"


_av = make_av(_SOURCE)


@register
class MrPayloadCommand(CommandDef):
    name = "MR::payload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::payload",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Access data collected using MR::collect command.",
                synopsis=("MR::payload ( 'length' )?",),
                snippet=(
                    "This command can be used to access payload collected using the COLLECT command.\n"
                    "\n"
                    "SYNTAX\n"
                    "\n"
                    "MR::payload [length]\n"
                    "\n"
                    "MR::payload\n"
                    "    Returns the collected payload obtained as a result of a prior call to MR::collect.\n"
                    "\n"
                    "MR::payload length\n"
                    "    Returns the length of payload of a MR message."
                ),
                source=_SOURCE,
                examples=(
                    "when MR_DATA {\n"
                    '                log local0 "Payload: [MR::payload]"\n'
                    "            }"
                ),
                return_value="When called without an argument, this command returns the collected payload of an MR message.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::payload ( 'length' )?",
                    arg_values={
                        0: (_av("length", "MR::payload length", "MR::payload ( 'length' )?"),)
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

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
