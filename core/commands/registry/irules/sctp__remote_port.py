# Enriched from F5 iRules reference documentation.
"""SCTP::remote_port -- Returns the remote SCTP port/service number."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SCTP__remote_port.html"


_av = make_av(_SOURCE)


@register
class SctpRemotePortCommand(CommandDef):
    name = "SCTP::remote_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SCTP::remote_port",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the remote SCTP port/service number.",
                synopsis=("SCTP::remote_port (clientside | serverside)?",),
                snippet="Returns the remote SCTP port/service number. Can specify the port value on clientside or serverside.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    SCTP::remote_port\n"
                    "    set x [SCTP::remote_port]\n"
                    "    SCTP::remote_port clientside\n"
                    "    SCTP::remote_port serverside\n"
                    "    SCTP::remote_port client\n"
                    "    SCTP::remote_port server\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SCTP::remote_port (clientside | serverside)?",
                    arg_values={
                        0: (
                            _av(
                                "clientside",
                                "SCTP::remote_port clientside",
                                "SCTP::remote_port (clientside | serverside)?",
                            ),
                            _av(
                                "serverside",
                                "SCTP::remote_port serverside",
                                "SCTP::remote_port (clientside | serverside)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
