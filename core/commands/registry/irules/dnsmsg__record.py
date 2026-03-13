# Enriched from F5 iRules reference documentation.
"""DNSMSG::record -- Returns the specified field from a resource record object."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DNSMSG-record.html"


_av = make_av(_SOURCE)


@register
class DnsmsgRecordCommand(CommandDef):
    name = "DNSMSG::record"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DNSMSG::record",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the specified field from a resource record object.",
                synopsis=(
                    "DNSMSG::record RESOURCE_RECORD ('owner' | 'type' | 'ttl' | 'class' | 'rdata')",
                ),
                snippet="This iRule gets the specified field from a resource record object.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '        set result [RESOLVER::name_lookup "/Common/r1" www.abc.com a]\n'
                    "        set answer [DNSMSG::section $result answer]\n"
                    "        set first_rr [lindex $answer 1]\n"
                    "        set rdata [DNSMSG::record $first_rr rdata]\n"
                    "}"
                ),
                return_value="Returns the specified field from the resource record object.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DNSMSG::record RESOURCE_RECORD ('owner' | 'type' | 'ttl' | 'class' | 'rdata')",
                    arg_values={
                        0: (
                            _av(
                                "owner",
                                "DNSMSG::record owner",
                                "DNSMSG::record RESOURCE_RECORD ('owner' | 'type' | 'ttl' | 'class' | 'rdata')",
                            ),
                            _av(
                                "type",
                                "DNSMSG::record type",
                                "DNSMSG::record RESOURCE_RECORD ('owner' | 'type' | 'ttl' | 'class' | 'rdata')",
                            ),
                            _av(
                                "ttl",
                                "DNSMSG::record ttl",
                                "DNSMSG::record RESOURCE_RECORD ('owner' | 'type' | 'ttl' | 'class' | 'rdata')",
                            ),
                            _av(
                                "class",
                                "DNSMSG::record class",
                                "DNSMSG::record RESOURCE_RECORD ('owner' | 'type' | 'ttl' | 'class' | 'rdata')",
                            ),
                            _av(
                                "rdata",
                                "DNSMSG::record rdata",
                                "DNSMSG::record RESOURCE_RECORD ('owner' | 'type' | 'ttl' | 'class' | 'rdata')",
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
