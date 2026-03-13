# Enriched from F5 iRules reference documentation.
"""SIP::header -- Gets or sets SIP header information."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__header.html"


_av = make_av(_SOURCE)


@register
class SipHeaderCommand(CommandDef):
    name = "SIP::header"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::header",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets SIP header information.",
                synopsis=(
                    "SIP::header SIP_HEADER_NAME (INDEX)?",
                    "SIP::header ('value' | 'remove') HEADER_NAME (INDEX)?",
                    "SIP::header ('insert') HEADER_NAME HEADER_VALUE (INDEX)?",
                    "SIP::header 'names'",
                ),
                snippet=(
                    "This set of commands allows you to get or set information in the SIP\n"
                    "header.\n"
                    "\n"
                    "Note: These commands still work on MBLB (Message Based Load\n"
                    "Balancing) SIP post 11.6+, but there are new commands that only\n"
                    "run on MRF (Message Routing Framework) SIP and were introduced\n"
                    "in 11.6."
                ),
                source=_SOURCE,
                examples=(
                    "when SIP_REQUEST_SEND {\n"
                    "  log local0. [SIP::method]\n"
                    '  SIP::header insert Via [format "SIP/2.0/TCP %s:%s" [IP::local_addr] [TCP::local_port]]\n'
                    '  SIP::header insert Y-Header "it is yyy"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::header SIP_HEADER_NAME (INDEX)?",
                    arg_values={
                        0: (
                            _av(
                                "value",
                                "SIP::header value",
                                "SIP::header ('value' | 'remove') HEADER_NAME (INDEX)?",
                            ),
                            _av(
                                "remove",
                                "SIP::header remove",
                                "SIP::header ('value' | 'remove') HEADER_NAME (INDEX)?",
                            ),
                            _av(
                                "insert",
                                "SIP::header insert",
                                "SIP::header ('insert') HEADER_NAME HEADER_VALUE (INDEX)?",
                            ),
                            _av("names", "SIP::header names", "SIP::header 'names'"),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"SIP"})),
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
