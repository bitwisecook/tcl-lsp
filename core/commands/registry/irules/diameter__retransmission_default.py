# Enriched from F5 iRules reference documentation.
"""DIAMETER::retransmission_default -- Gets of sets the current connection's retransmission settings."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__retransmission_default.html"


@register
class DiameterRetransmissionDefaultCommand(CommandDef):
    name = "DIAMETER::retransmission_default"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::retransmission_default",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets of sets the current connection's retransmission settings.",
                synopsis=("DIAMETER::retransmission_default action",),
                snippet=(
                    "This command allows the setting or getting of the current\n"
                    "connection\\'s retransmission settings. All request messages on the\n"
                    "current connection will be initailized with the connection\\'s setings.\n"
                    "The messages\\'s settings may be changed with the\n"
                    "DIAMETER::retransmission command.\n"
                    "        \n"
                    "Gets the current connection\\'s retransmission action.\n"
                    "Possible actions are:\n"
                    "\n"
                    ' * "disabled" - request messages will not be queued for retransmission\n'
                    "\n"
                    ' * "busy" - when retransmission is triggered for a request message an\n'
                    "   answer message with a DIAMETER_TOO_BUSY result code will be\n"
                    "   returned to the originator."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n    DIAMETER::retransmission_default action busy\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::retransmission_default action",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                also_in=frozenset({"CLIENT_ACCEPTED", "SERVER_CONNECTED"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
