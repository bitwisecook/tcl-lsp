# Enriched from F5 iRules reference documentation.
"""SSL::payload -- Returns and manipulates plaintext data collected via SSL::collect."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__payload.html"


@register
class SslPayloadCommand(CommandDef):
    name = "SSL::payload"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::payload",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns and manipulates plaintext data collected via SSL::collect.",
                synopsis=("SSL::payload (length |",),
                snippet="The SSL::payload commands allow you to return and manipulate the data collected via the SSL::collect command. This data is in plaintext format.",
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_HANDSHAKE {\n"
                    '    log local0. "[IP::client_addr]:[TCP::client_port]: SSL handshake completed, collecting SSL payload"\n'
                    "    SSL::collect\n"
                    "}"
                ),
                return_value="SSL::payload length Returns the amount of plaintext data collected by the SSL::collect command.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::payload (length |",
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
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
