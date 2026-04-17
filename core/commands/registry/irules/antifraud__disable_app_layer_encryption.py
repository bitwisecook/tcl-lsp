# Enriched from F5 iRules reference documentation.
"""ANTIFRAUD::disable_app_layer_encryption -- Disables application layer encryption for the current transaction."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ANTIFRAUD__disable_app_layer_encryption.html"


@register
class AntifraudDisableAppLayerEncryptionCommand(CommandDef):
    name = "ANTIFRAUD::disable_app_layer_encryption"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ANTIFRAUD::disable_app_layer_encryption",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Disables application layer encryption for the current transaction.",
                synopsis=("ANTIFRAUD::disable_app_layer_encryption",),
                snippet="Disables application layer encryption for the current transaction.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '                if { [HTTP::header exists "Antifraud-Disable-AppLayerEncryption" ] } {\n'
                    "                    ANTIFRAUD::disable_app_layer_encryption\n"
                    '                    log local0. "Application Layer Encryption disabled"\n'
                    "                }\n"
                    "            }"
                ),
                return_value="Disables application layer encryption for the current transaction.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ANTIFRAUD::disable_app_layer_encryption",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    writes=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
