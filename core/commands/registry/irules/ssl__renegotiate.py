# Enriched from F5 iRules reference documentation.
"""SSL::renegotiate -- Controls renegotiation of an SSL connection."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__renegotiate.html"


_av = make_av(_SOURCE)


@register
class SslRenegotiateCommand(CommandDef):
    name = "SSL::renegotiate"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::renegotiate",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Controls renegotiation of an SSL connection.",
                synopsis=("SSL::renegotiate (enable | disable)?",),
                snippet=(
                    "Controls renegotiation of an SSL connection, often used to enforce new encryption settings or certificate requirements.\n"
                    "\n"
                    "This command has different results depending on whether the BIG-IP system evaluates the command under a client-side or a server-side context. The command only succeeds if SSL is enabled on the connection; otherwise, the command returns an error."
                ),
                source=_SOURCE,
                examples=("when CLIENTSSL_HANDSHAKE {\n    SSL::renegotiate disable\n}"),
                return_value="SSL::renegotiate Renegotiates a client-side or server-side SSL connection, depending on the context. When the system evaluates the command under a client-side context, the system immediately renegotiates a request for the associated client-side connection, if client-side renegotiation is enabled.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::renegotiate (enable | disable)?",
                    arg_values={
                        0: (
                            _av(
                                "enable",
                                "SSL::renegotiate enable",
                                "SSL::renegotiate (enable | disable)?",
                            ),
                            _av(
                                "disable",
                                "SSL::renegotiate disable",
                                "SSL::renegotiate (enable | disable)?",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                transport="tcp", profiles=frozenset({"CLIENTSSL", "SERVERSSL"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
