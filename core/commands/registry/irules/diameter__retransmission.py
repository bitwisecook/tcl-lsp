# Enriched from F5 iRules reference documentation.
"""DIAMETER::retransmission -- Gets of sets the current message's retransmission settings."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__retransmission.html"


@register
class DiameterRetransmissionCommand(CommandDef):
    name = "DIAMETER::retransmission"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::retransmission",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets of sets the current message's retransmission settings.",
                synopsis=("DIAMETER::retransmission action",),
                snippet=(
                    "This command allows the setting or getting of the current message\\'s\n"
                    "retransmission settings.\n"
                    "        \n"
                    "Gets the current message\\'s retransmission action.\n"
                    "Possible actions are:\n"
                    "\n"
                    '* "disabled" - this request messages will not be queued for\n'
                    "  retransmission\n"
                    "\n"
                    '* "busy" - when retransmission is triggered for this request message\n'
                    "  an answer message with a 'busy' result code will be returned to the\n"
                    "  originator.\n"
                    "\n"
                    '* "unable" - when retransmission is triggered for this request message\n'
                    "  an answer message with a 'unable to deliver' result code will be\n"
                    "  returned to the originator."
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n    DIAMETER::retransmission action retransmit\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::retransmission action",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
