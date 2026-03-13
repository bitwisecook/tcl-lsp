# Enriched from F5 iRules reference documentation.
"""SIP::via -- Gets SIP via header information."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__via.html"


_av = make_av(_SOURCE)


@register
class SipViaCommand(CommandDef):
    name = "SIP::via"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::via",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets SIP via header information.",
                synopsis=(
                    "SIP::via (proto | sent_by | received | branch | maddr | ttl)? (INDEX | top)?",
                ),
                snippet="This set of commands allows you to get information in the SIP via header.",
                source=_SOURCE,
                examples=(
                    "when SIP_RESPONSE {\n"
                    "  log local0. [SIP::via 0]\n"
                    "  SIP::header remove Via 0\n"
                    '  SIP::response rewrite 123 "no xxx"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::via (proto | sent_by | received | branch | maddr | ttl)? (INDEX | top)?",
                    arg_values={
                        0: (
                            _av(
                                "proto",
                                "SIP::via proto",
                                "SIP::via (proto | sent_by | received | branch | maddr | ttl)? (INDEX | top)?",
                            ),
                            _av(
                                "sent_by",
                                "SIP::via sent_by",
                                "SIP::via (proto | sent_by | received | branch | maddr | ttl)? (INDEX | top)?",
                            ),
                            _av(
                                "received",
                                "SIP::via received",
                                "SIP::via (proto | sent_by | received | branch | maddr | ttl)? (INDEX | top)?",
                            ),
                            _av(
                                "branch",
                                "SIP::via branch",
                                "SIP::via (proto | sent_by | received | branch | maddr | ttl)? (INDEX | top)?",
                            ),
                            _av(
                                "maddr",
                                "SIP::via maddr",
                                "SIP::via (proto | sent_by | received | branch | maddr | ttl)? (INDEX | top)?",
                            ),
                            _av(
                                "ttl",
                                "SIP::via ttl",
                                "SIP::via (proto | sent_by | received | branch | maddr | ttl)? (INDEX | top)?",
                            ),
                            _av(
                                "top",
                                "SIP::via top",
                                "SIP::via (proto | sent_by | received | branch | maddr | ttl)? (INDEX | top)?",
                            ),
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
