# Enriched from F5 iRules reference documentation.
"""DIAMETER::session -- Gets or sets the session-id attribute-value pair."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__session.html"


@register
class DiameterSessionCommand(CommandDef):
    name = "DIAMETER::session"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::session",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the session-id attribute-value pair.",
                synopsis=("DIAMETER::session (SESSION_ID)?",),
                snippet=(
                    "This iRule command gets or sets the value of session-id AVP (code 263)\n"
                    "in the message."
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    '    log local0. "Received a DIAMETER message for session [DIAMETER::session]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::session (SESSION_ID)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER", "MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
