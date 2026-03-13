# Enriched from F5 iRules reference documentation.
"""DIAMETER::length -- Gets diameter message length."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__length.html"


@register
class DiameterLengthCommand(CommandDef):
    name = "DIAMETER::length"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::length",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets diameter message length.",
                synopsis=("DIAMETER::length",),
                snippet=(
                    "This iRule command returns the length of the current message,\n"
                    "including the message header.\n"
                    "\n"
                    "The value returned reflects the current length of the message at the\n"
                    "instant the iRule command is executed: if you store the length of a\n"
                    "message in a variable and then modify the message, your stored length\n"
                    "may be incorrect."
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    '    log local0. "Received a Diameter message of [DIAMETER::length] bytes"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::length",
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
