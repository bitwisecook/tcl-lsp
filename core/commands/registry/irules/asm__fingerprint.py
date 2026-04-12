# Enriched from F5 iRules reference documentation.
"""ASM::fingerprint -- Returns the fingerprint (device id) of the client device."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__fingerprint.html"


@register
class AsmFingerprintCommand(CommandDef):
    name = "ASM::fingerprint"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::fingerprint",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the fingerprint (device id) of the client device.",
                synopsis=("ASM::fingerprint",),
                snippet=(
                    "Get the fingerprint of the client device as seen by ASM when it's available.\n"
                    "The fingerprint is a unique identifier given to specific client machine. The fingerprint will be available to iRule only for web application that have web scraping turned on with the finger print usage activated."
                ),
                source=_SOURCE,
                examples=("when ASM_REQUEST_DONE {\n    log local0.[ASM::fingerprint]\n}"),
                return_value="Returns the fingerprint of the client device or 0 if it's not available.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::fingerprint",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ASM"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
