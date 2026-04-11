# Enriched from F5 iRules reference documentation.
"""REWRITE::disable -- Changes the REWRITE plugin from full patching mode to passthrough mode."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/REWRITE__disable.html"


@register
class RewriteDisableCommand(CommandDef):
    name = "REWRITE::disable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="REWRITE::disable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Changes the REWRITE plugin from full patching mode to passthrough mode.",
                synopsis=("REWRITE::disable",),
                snippet="Changes the REWRITE plugin from full patching to passthrough mode.",
                source=_SOURCE,
                examples=("when ACCESS_ACL_ALLOWED {\n  set host [HTTP::host]\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="REWRITE::disable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ACCESS", "FASTHTTP", "REWRITE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.STREAM_PROFILE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
