# Enriched from F5 iRules reference documentation.
"""TCP::notify -- Sends a message to upper layers of iRule processing."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/TCP__notify.html"


_av = make_av(_SOURCE)


@register
class TcpNotifyCommand(CommandDef):
    name = "TCP::notify"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="TCP::notify",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Sends a message to upper layers of iRule processing.",
                synopsis=("TCP::notify (request | response | eom)",),
                snippet=(
                    "This command has two uses, which are unrelated to one another:\n"
                    "  1. to indicate the end of a message when TCP message-based load-balancing is in effect;\n"
                    "  2. to raise the USER_REQUEST or USER_RESPONSE event.\n"
                    "\n"
                    "The BIG-IP LTM module supports TCP message-based load-balancing. This is enabled by applying an mblb profile to an LTM Virtual Server that also has a tcp profile applied. Note that currently (up to and including 11.4.1), the mblb profile can only be added using tmsh. There is no mechanism for adding it via the Web UI."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "        set message_length              0\n"
                    "        set collected_length            0\n"
                    "        set at_msg_start                1\n"
                    "\n"
                    "        TCP::collect\n"
                    "}"
                ),
                return_value="None.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="TCP::notify (request | response | eom)",
                    arg_values={
                        0: (
                            _av(
                                "request",
                                "TCP::notify request",
                                "TCP::notify (request | response | eom)",
                            ),
                            _av(
                                "response",
                                "TCP::notify response",
                                "TCP::notify (request | response | eom)",
                            ),
                            _av("eom", "TCP::notify eom", "TCP::notify (request | response | eom)"),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp"),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
