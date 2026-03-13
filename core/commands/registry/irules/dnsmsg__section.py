# Enriched from F5 iRules reference documentation.
"""DNSMSG::section -- Returns a section of a dns_message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNSMSG-section.html"


_av = make_av(_SOURCE)


@register
class DnsmsgSectionCommand(CommandDef):
    name = "DNSMSG::section"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNSMSG::section",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a section of a dns_message.",
                synopsis=(
                    "DNSMSG::section DNS_MESSAGE ('question' | 'answer' | 'authority' | 'additional' )",
                ),
                snippet="This iRule gets the specified section of a dns_message.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '        set result [RESOLVER::name_lookup "/Common/r1" www.abc.com a]\n'
                    "        set answer [DNSMSG::section $result answer]\n"
                    "}"
                ),
                return_value="Returns a TCL list of resource records from the specified section.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNSMSG::section DNS_MESSAGE ('question' | 'answer' | 'authority' | 'additional' )",
                    arg_values={
                        0: (
                            _av(
                                "question",
                                "DNSMSG::section question",
                                "DNSMSG::section DNS_MESSAGE ('question' | 'answer' | 'authority' | 'additional' )",
                            ),
                            _av(
                                "answer",
                                "DNSMSG::section answer",
                                "DNSMSG::section DNS_MESSAGE ('question' | 'answer' | 'authority' | 'additional' )",
                            ),
                            _av(
                                "authority",
                                "DNSMSG::section authority",
                                "DNSMSG::section DNS_MESSAGE ('question' | 'answer' | 'authority' | 'additional' )",
                            ),
                            _av(
                                "additional",
                                "DNSMSG::section additional",
                                "DNSMSG::section DNS_MESSAGE ('question' | 'answer' | 'authority' | 'additional' )",
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
                    target=SideEffectTarget.DNS_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
