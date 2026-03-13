# Enriched from F5 iRules reference documentation.
"""BIGTCP::release_flow -- Releases a flow from BIGTCP's control."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BIGTCP__release_flow.html"


@register
class BigtcpReleaseFlowCommand(CommandDef):
    name = "BIGTCP::release_flow"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BIGTCP::release_flow",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Releases a flow from BIGTCP's control.",
                synopsis=("BIGTCP::release_flow",),
                snippet="This command releases a flow from BIGTCP's control. After calling this method, the flow will be in passthrough mode.  In this mode, no more data will be processed by any filters or iRules.",
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n    BIGTCP::release_flow;\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BIGTCP::release_flow",
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
