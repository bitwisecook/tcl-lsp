# Enriched from F5 iRules reference documentation.
"""X509::pem2der -- Returns an X509 certificate in DER format."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/X509__pem2der.html"


@register
class X509Pem2derCommand(CommandDef):
    name = "X509::pem2der"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="X509::pem2der",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns an X509 certificate in DER format.",
                synopsis=("X509::pem2der <cert-in-pem>",),
                snippet="Returns the specified X509 certificate, in its entirety, in DER format.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "    if { [SSL::cert count] > 0 } {\n"
                    "        SSL::c3d cert [X509::pem2der [X509::whole [SSL::cert 0]]]\n"
                    "    }\n"
                    "}"
                ),
                return_value="Returns an X509 certificate in DER format.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="X509::pem2der <cert-in-pem>",
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
