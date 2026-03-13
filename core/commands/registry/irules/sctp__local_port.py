# Enriched from F5 iRules reference documentation.
"""SCTP::local_port -- Returns the local SCTP port/service number."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SCTP__local_port.html"


_av = make_av(_SOURCE)


@register
class SctpLocalPortCommand(CommandDef):
    name = "SCTP::local_port"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SCTP::local_port",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the local SCTP port/service number.",
                synopsis=("SCTP::local_port (clientside | serverside)?",),
                snippet="Returns the local SCTP port/service number. Can specify the port value on clientside or serverside.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "        SCTP::collect\n"
                    '        log local0.info "Sctp local port is [SCTP::local_port]"\n'
                    '        log local0.info "Sctp client port is [SCTP::client_port]"\n'
                    '        log local0.info "Sctp mss is [SCTP::mss]"\n'
                    '        log local0.info "sctp ppi is [SCTP::ppi]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SCTP::local_port (clientside | serverside)?",
                    arg_values={
                        0: (
                            _av(
                                "clientside",
                                "SCTP::local_port clientside",
                                "SCTP::local_port (clientside | serverside)?",
                            ),
                            _av(
                                "serverside",
                                "SCTP::local_port serverside",
                                "SCTP::local_port (clientside | serverside)?",
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
