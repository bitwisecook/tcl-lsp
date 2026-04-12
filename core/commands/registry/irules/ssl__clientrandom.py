# Enriched from F5 iRules reference documentation.
"""SSL::clientrandom -- Return the ClientRandom value from the Client hello."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__clientrandom.html"


@register
class SslClientrandomCommand(CommandDef):
    name = "SSL::clientrandom"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::clientrandom",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Return the ClientRandom value from the Client hello.",
                synopsis=("SSL::clientrandom",),
                snippet="Return the ClientRandom value from the Client hello.",
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_HANDSHAKE {\n"
                    '    log local0.info "negotiated protocol: [SSL::clientrandom]"\n'
                    "}"
                ),
                return_value="The ClientRandom value.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::clientrandom",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
