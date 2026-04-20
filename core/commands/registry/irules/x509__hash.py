# Enriched from F5 iRules reference documentation.
"""X509::hash -- Returns the MD5 hash (fingerprint) of an X509 certificate."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/X509__hash.html"


@register
class X509HashCommand(CommandDef):
    name = "X509::hash"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="X509::hash",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the MD5 hash (fingerprint) of an X509 certificate.",
                synopsis=("X509::hash CERTIFICATE",),
                snippet="Returns the MD5 hash (fingerprint) of the specified X509 certificate.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    "  if { [info exist cert_hash] } {\n"
                    '    if { $cert_hash equals "XX:XX:XX:XX:XX:XX:XX:XX:XX:XX:XX:XX:XX:XX:XX:XX"} {\n'
                    '      HTTP::redirect "https://somesite/"\n'
                    "    } else {\n"
                    '      HTTP::redirect "https://someothersite/"\n'
                    "    }\n"
                    "  }\n"
                    "}"
                ),
                return_value="Returns the MD5 hash (fingerprint) of an X509 certificate.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="X509::hash CERTIFICATE",
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
