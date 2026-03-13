# Enriched from F5 iRules reference documentation.
"""GTP::payload -- Returns the entire payload for G-PDU message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GTP__payload.html"


_av = make_av(_SOURCE)


@register
class GtpPayloadCommand(CommandDef):
    name = "GTP::payload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GTP::payload",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the entire payload for G-PDU message.",
                synopsis=(
                    "GTP::payload",
                    "GTP::payload COUNT",
                    "GTP::payload OFFSET COUNT",
                    "GTP::payload 'replace' ('-message' MESSAGE)? OFFSET COUNT NEW_VALUE",
                ),
                snippet=(
                    "Returns the payload, either complete or partial, for G-PDU message.\n"
                    "This command returns an empty value, in case of non-G-PDU messages."
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set payload [UDP::payload]\n"
                    "    set t2 [GTP::parse $payload]\n"
                    '    log local0. "GTP version [GTP::header version -message $t2]"\n'
                    '    log local0. "GTP type [GTP::header type -message $t2]"\n'
                    '    log local0. "GTP teid [GTP::header teid -message $t2]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GTP::payload",
                    options=(
                        OptionSpec(name="-message", detail="Option -message.", takes_value=True),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "replace",
                                "GTP::payload replace",
                                "GTP::payload 'replace' ('-message' MESSAGE)? OFFSET COUNT NEW_VALUE",
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
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
