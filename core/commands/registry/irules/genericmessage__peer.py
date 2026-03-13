# Enriched from F5 iRules reference documentation.
"""GENERICMESSAGE::peer -- Returns or sets the peer's route name."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GENERICMESSAGE__peer.html"


@register
class GenericmessagePeerCommand(CommandDef):
    name = "GENERICMESSAGE::peer"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GENERICMESSAGE::peer",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets the peer's route name.",
                synopsis=("GENERICMESSAGE::peer name (NAME)?",),
                snippet=(
                    "The GENERICMESSAGE::peer command returns or sets the peer's route name\n"
                    "in the message routing framework. The peer name will be automatically\n"
                    "set as the source address of each message."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '    GENERICMESSAGE::peer name "[IP::remote_addr]:[TCP::remote_port]"\n'
                    "}"
                ),
                return_value="Returns the peer's route name.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GENERICMESSAGE::peer name (NAME)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
