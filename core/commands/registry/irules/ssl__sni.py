# Enriched from F5 iRules reference documentation.
"""SSL::sni -- Returns Server Name Indication information."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ..taint_hints import TaintColour, TaintHint
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__sni.html"


_av = make_av(_SOURCE)


@register
class SslSniCommand(CommandDef):
    name = "SSL::sni"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::sni",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns Server Name Indication information.",
                synopsis=("SSL::sni (name | required)",),
                snippet="Returns a Server Name Indication name, and require SNI support.",
                source=_SOURCE,
                examples=(
                    "when HTTP_REQUEST {\n"
                    '    log local0.info "SNI name: [SSL::sni name]"\n'
                    '    log local0.info "SNI required: [SSL::sni required]"\n'
                    "}"
                ),
                return_value="SSL::sni name Returns the current Server Name Indication as specified in the SSL profile. SSL::sni required Returns the require SNI support as specified in the SSL profile.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::sni (name | required)",
                    arg_values={
                        0: (
                            _av("name", "SSL::sni name", "SSL::sni (name | required)"),
                            _av("required", "SSL::sni required", "SSL::sni (name | required)"),
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
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )

    @classmethod
    def taint_hints(cls) -> TaintHint:
        return TaintHint(
            source={Arity(0, 0): TaintColour.TAINTED | TaintColour.FQDN},
        )
