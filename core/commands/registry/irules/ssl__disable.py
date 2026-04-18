# Enriched from F5 iRules reference documentation.
"""SSL::disable -- Disables SSL processing."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__disable.html"


_av = make_av(_SOURCE)


@register
class SslDisableCommand(CommandDef):
    name = "SSL::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disables SSL processing.",
                synopsis=("SSL::disable (clientside | serverside)?",),
                snippet=(
                    "Disables SSL processing. This command is useful when using a virtual server that services both SSL and non-SSL traffic, or when you want to selectively re-encrypt traffic to pool members.\n"
                    "\n"
                    "Note: Disabling SSL on the serverside only applies before serverside connection has been established (SERVER_CONNECTED) or when the clientside of the connection is in a detached state (e.g., oneconnect, LB::detach)."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "    if { $usessl == 0 } {\n"
                    "        SSL::disable\n"
                    "    }\n"
                    "}"
                ),
                return_value="SSL::disable [clientside | serverside] Disables SSL processing on one side of the LTM. Sends an SSL alert to the peer requesting termination of SSL processing.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::disable (clientside | serverside)?",
                    arg_values={
                        0: (
                            _av(
                                "clientside",
                                "SSL::disable clientside",
                                "SSL::disable (clientside | serverside)?",
                            ),
                            _av(
                                "serverside",
                                "SSL::disable serverside",
                                "SSL::disable (clientside | serverside)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            diagram_action=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
