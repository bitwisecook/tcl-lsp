# Enriched from F5 iRules reference documentation.
"""X509::serial_number -- Returns the serial number of an X509 certificate."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/X509__serial_number.html"


@register
class X509SerialNumberCommand(CommandDef):
    name = "X509::serial_number"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="X509::serial_number",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the serial number of an X509 certificate.",
                synopsis=("X509::serial_number CERTIFICATE",),
                snippet="Returns the serial number of the specified X509 certificate.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    log local0. "Certificate Serial Number: [X509::serial_number cert_x]"\n'
                    "}"
                ),
                return_value="Returns the serial number of an X509 certificate.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="X509::serial_number CERTIFICATE",
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
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
