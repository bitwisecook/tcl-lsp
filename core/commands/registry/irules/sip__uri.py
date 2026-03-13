# Enriched from F5 iRules reference documentation.
"""SIP::uri -- Returns or sets the URI of the request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__uri.html"


@register
class SipUriCommand(CommandDef):
    name = "SIP::uri"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::uri",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or sets the URI of the request.",
                synopsis=("SIP::uri (URI_STRING)?",),
                snippet="Returns or sets the complete URI of the request.",
                source=_SOURCE,
                examples=(
                    'when SIP_REQUEST {\n  log local0. "uri: [SIP::uri] via [SIP::header Via 0]"\n}'
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::uri (URI_STRING)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"SIP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(source={None: TaintColour.TAINTED})
