# Enriched from F5 iRules reference documentation.
"""VALIDATE::protocol -- Performs validation of given application to match payload."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/VALIDATE__protocol.html"


@register
class ValidateProtocolCommand(CommandDef):
    name = "VALIDATE::protocol"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="VALIDATE::protocol",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Performs validation of given application to match payload.",
                synopsis=("VALIDATE::protocol CLASSIFY_APP_NAME ANY_CHARS",),
                snippet=(
                    "This command allows you to validate payload (traffic) to match given classification application.\n"
                    "\n"
                    "Note: the APM / AFM / PEM license is required for functionality to work."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n  TCP::collect 32\n}"),
                return_value="Returns TRUE in case of match, FALSE otherwise.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="VALIDATE::protocol CLASSIFY_APP_NAME ANY_CHARS",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
