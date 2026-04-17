# Enriched from F5 iRules reference documentation.
"""X509::version -- Returns the version number of an X509 certificate."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/X509__version.html"


@register
class X509VersionCommand(CommandDef):
    name = "X509::version"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="X509::version",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the version number of an X509 certificate.",
                synopsis=("X509::version CERTIFICATE",),
                snippet=(
                    "Returns the version number of the specified X509 certificate (an\ninteger)."
                ),
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '  log local0. "Cert version - [X509::version ssl_cert]"\n'
                    "  if { [X509::version ssl_cert] eq 3 } {\n"
                    "    pool v3_pool\n"
                    "  } else {\n"
                    "    pool default_pool\n"
                    "  }\n"
                    "}"
                ),
                return_value="Returns the version number of an X509 certificate.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="X509::version CERTIFICATE",
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
