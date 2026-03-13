# Enriched from F5 iRules reference documentation.
"""DIAMETER::message -- Returns the whole Diameter message as a TCL string object."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__message.html"


@register
class DiameterMessageCommand(CommandDef):
    name = "DIAMETER::message"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::message",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the whole Diameter message as a TCL string object.",
                synopsis=("DIAMETER::message",),
                snippet=(
                    "This iRule command returns the current Diameter message as a TCL\n"
                    "string object.  This includes both the header and the payload."
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    '    log local0. "Received a DIAMETER message: [DIAMETER::message]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::message",
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
