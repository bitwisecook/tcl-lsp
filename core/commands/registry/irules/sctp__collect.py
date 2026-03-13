# Enriched from F5 iRules reference documentation.
"""SCTP::collect -- Collects the specified amount of content data."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SCTP__collect.html"


@register
class SctpCollectCommand(CommandDef):
    name = "SCTP::collect"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SCTP::collect",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Collects the specified amount of content data.",
                synopsis=("SCTP::collect (COLLECT_BYTES)?",),
                snippet=(
                    "Causes SCTP to start collecting the specified amount of content data. After collecting the data, event CLIENT_DATA will be triggered.\n"
                    "\n"
                    "SCTP::collect <length>\n"
                    "    Causes SCTP to start collecting the specified amount of content data. The parameter specifies the minimum number of bytes to collect.\n"
                    "\n"
                    "SCTP::collect\n"
                    "    When length is not specified, CLIENT_DATA will be triggered for every received packet. To stop collecting data, use SCTP::release."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n  SCTP::collect 15\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SCTP::collect (COLLECT_BYTES)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
