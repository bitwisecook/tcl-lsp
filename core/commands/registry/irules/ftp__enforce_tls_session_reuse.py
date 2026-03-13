# Enriched from F5 iRules reference documentation.
"""FTP::enforce_tls_session_reuse -- Get or set the state of enforcing TLS session reuse."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/FTP__enforce_tls_session_reuse.html"


_av = make_av(_SOURCE)


@register
class FtpEnforceTlsSessionReuseCommand(CommandDef):
    name = "FTP::enforce_tls_session_reuse"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="FTP::enforce_tls_session_reuse",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set the state of enforcing TLS session reuse.",
                synopsis=("FTP::enforce_tls_session_reuse (enable | disable)?",),
                snippet="Enable or disable enforcing TLS session reuse, when enabled, Bigip rejects the data connection if it fails to reuse existed TLS session. Returns the current status if no option is specified.",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                FTP::enforce_tls_session_reuse enable\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="FTP::enforce_tls_session_reuse (enable | disable)?",
                    arg_values={
                        0: (
                            _av(
                                "enable",
                                "FTP::enforce_tls_session_reuse enable",
                                "FTP::enforce_tls_session_reuse (enable | disable)?",
                            ),
                            _av(
                                "disable",
                                "FTP::enforce_tls_session_reuse disable",
                                "FTP::enforce_tls_session_reuse (enable | disable)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FTP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
