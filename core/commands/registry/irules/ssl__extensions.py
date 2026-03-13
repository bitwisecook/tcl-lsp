# Enriched from F5 iRules reference documentation.
"""SSL::extensions -- Returns or manipulates SSL extensions."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget, StorageScope
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__extensions.html"


@register
class SslExtensionsCommand(CommandDef):
    name = "SSL::extensions"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::extensions",
            dialects=_IRULES_ONLY,
            pure=True,
            hover=HoverSnippet(
                summary="Returns or manipulates SSL extensions.",
                synopsis=(
                    "SSL::extensions (count |",
                    "SSL::extensions insert OPAQUE_EXT",
                ),
                snippet="Returns or manipulates SSL extensions.",
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_CLIENTHELLO {\n"
                    '    set my_ext "Hello world!"\n'
                    "    set my_ext_type 62965\n"
                    "    SSL::extensions insert [binary format S1S1a* $my_ext_type [string length $my_ext] $my_ext]\n"
                    "}"
                ),
                return_value="SSL::extensions Returns the extensions sent by the peer as a single opaque byte array. Valid in all SSL handshake events (those other than *SSL_DATA).",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::extensions (count |",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                transport="tcp", profiles=frozenset({"CLIENTSSL", "SERVERSSL"})
            ),
            cse_candidate=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                    scope=StorageScope.EVENT,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
