# Enriched from F5 iRules reference documentation.
"""DSLITE::remote_addr -- Returns the remote DS-Lite tunnel."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DSLITE__remote_addr.html"


@register
class DsliteRemoteAddrCommand(CommandDef):
    name = "DSLITE::remote_addr"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DSLITE::remote_addr",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the remote DS-Lite tunnel.",
                synopsis=("DSLITE::remote_addr",),
                snippet="Returns the remote DS-Lite tunnel endpoint IP address.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DSLITE::remote_addr",
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
