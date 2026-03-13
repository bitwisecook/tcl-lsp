# Enriched from F5 iRules reference documentation.
"""GTP::forward -- Forwards GTP message to peer flow."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GTP__forward.html"


@register
class GtpForwardCommand(CommandDef):
    name = "GTP::forward"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GTP::forward",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Forwards GTP message to peer flow.",
                synopsis=("GTP::forward MESSAGE",),
                snippet="Forwards GTP message to peer flow.",
                source=_SOURCE,
                examples=(
                    "when GTP_SIGNALLING_INGRESS {\n"
                    "    set t2 [GTP::new 2 10]\n"
                    "    GTP::forward $t2\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GTP::forward MESSAGE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
