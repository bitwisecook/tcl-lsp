# Enriched from F5 iRules reference documentation.
"""DIAMETER::payload -- Gets or sets DIAMETER message payload."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__payload.html"


_av = make_av(_SOURCE)


@register
class DiameterPayloadCommand(CommandDef):
    name = "DIAMETER::payload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::payload",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets DIAMETER message payload.",
                synopsis=("DIAMETER::payload ('replace' PAYLOAD)?",),
                snippet=(
                    "This iRule command gets or sets the current DIAMETER message\\'s\n"
                    "payload, as a byte string."
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    '    log local0. "Received a DIAMETER message, with payload [DIAMETER::payload]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::payload ('replace' PAYLOAD)?",
                    arg_values={
                        0: (
                            _av(
                                "replace",
                                "DIAMETER::payload replace",
                                "DIAMETER::payload ('replace' PAYLOAD)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER", "MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
