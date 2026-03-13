# Enriched from F5 iRules reference documentation.
"""GENERICMESSAGE::message -- Returns or sets values for messages in the generic message profile."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GENERICMESSAGE__message.html"


_av = make_av(_SOURCE)


@register
class GenericmessageMessageCommand(CommandDef):
    name = "GENERICMESSAGE::message"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GENERICMESSAGE::message",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets values for messages in the generic message profile.",
                synopsis=(
                    "GENERICMESSAGE::message (len | length)",
                    "GENERICMESSAGE::message (src | source | dst | dest | destination) (SRC_DST)?",
                    "GENERICMESSAGE::message is_request (BOOLEAN)?",
                    "GENERICMESSAGE::message data (DATA)?",
                ),
                snippet=(
                    "The GENERICMESSAGE::message command returns or sets values from\n"
                    "the current message being processed by the generic message profile."
                ),
                source=_SOURCE,
                examples=(
                    "when GENERICMESSAGE_INGRESS {\n"
                    "    GENERICMESSAGE::message src us\n"
                    "    GENERICMESSAGE::message dst them\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GENERICMESSAGE::message (len | length)",
                    arg_values={
                        0: (
                            _av(
                                "len",
                                "GENERICMESSAGE::message len",
                                "GENERICMESSAGE::message (len | length)",
                            ),
                            _av(
                                "length",
                                "GENERICMESSAGE::message length",
                                "GENERICMESSAGE::message (len | length)",
                            ),
                            _av(
                                "src",
                                "GENERICMESSAGE::message src",
                                "GENERICMESSAGE::message (src | source | dst | dest | destination) (SRC_DST)?",
                            ),
                            _av(
                                "source",
                                "GENERICMESSAGE::message source",
                                "GENERICMESSAGE::message (src | source | dst | dest | destination) (SRC_DST)?",
                            ),
                            _av(
                                "dst",
                                "GENERICMESSAGE::message dst",
                                "GENERICMESSAGE::message (src | source | dst | dest | destination) (SRC_DST)?",
                            ),
                            _av(
                                "dest",
                                "GENERICMESSAGE::message dest",
                                "GENERICMESSAGE::message (src | source | dst | dest | destination) (SRC_DST)?",
                            ),
                            _av(
                                "destination",
                                "GENERICMESSAGE::message destination",
                                "GENERICMESSAGE::message (src | source | dst | dest | destination) (SRC_DST)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"GENERICMSG", "MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
