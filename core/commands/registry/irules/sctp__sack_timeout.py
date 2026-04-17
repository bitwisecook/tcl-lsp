# Enriched from F5 iRules reference documentation.
"""SCTP::sack_timeout -- Returns the SCTP's delayed selective acknowledgement timeout."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SCTP__sack_timeout.html"


_av = make_av(_SOURCE)


@register
class SctpSackTimeoutCommand(CommandDef):
    name = "SCTP::sack_timeout"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SCTP::sack_timeout",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the SCTP's delayed selective acknowledgement timeout.",
                synopsis=("SCTP::sack_timeout (clientside | serverside)?",),
                snippet="Returns the SCTP's delayed selective acknowledgement timeout. Can specify the value on clientside or serverside.",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '        log local0.info "SCTP selective acknowledgement timeout value is [SCTP::sack_timeout]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SCTP::sack_timeout (clientside | serverside)?",
                    arg_values={
                        0: (
                            _av(
                                "clientside",
                                "SCTP::sack_timeout clientside",
                                "SCTP::sack_timeout (clientside | serverside)?",
                            ),
                            _av(
                                "serverside",
                                "SCTP::sack_timeout serverside",
                                "SCTP::sack_timeout (clientside | serverside)?",
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
