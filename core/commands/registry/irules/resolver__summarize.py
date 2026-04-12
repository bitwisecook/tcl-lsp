# Enriched from F5 iRules reference documentation.
"""RESOLVER::summarize -- Returns a summary of the response."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/RESOLVER-summarize.html"


@register
class ResolverSummarizeCommand(CommandDef):
    name = "RESOLVER::summarize"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="RESOLVER::summarize",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns a summary of the response.",
                synopsis=("RESOLVER::summarize DNS_MESSAGE",),
                snippet="Takes a dns_message structure and returns a summary as a list of resource records.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '        set result [RESOLVER::name_lookup "/Common/r1" www.abc.com a]\n'
                    "        set rrs [RESOLVER::summarize $result]\n"
                    "}"
                ),
                return_value="The summary will be a TCL list of resource record objects of the type specified in the query. Individual resource record objects are usable by the DNSMSG::record iRule command.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="RESOLVER::summarize DNS_MESSAGE",
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
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
