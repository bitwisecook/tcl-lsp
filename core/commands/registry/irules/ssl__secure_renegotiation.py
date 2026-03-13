# Enriched from F5 iRules reference documentation.
"""SSL::secure_renegotiation -- Controls the SSL Secure Renegotiation mode."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__secure_renegotiation.html"


_av = make_av(_SOURCE)


@register
class SslSecureRenegotiationCommand(CommandDef):
    name = "SSL::secure_renegotiation"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::secure_renegotiation",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Controls the SSL Secure Renegotiation mode.",
                synopsis=("SSL::secure_renegotiation (request | require | require-strict)?",),
                snippet="Controls the SSL Secure Renegotiation mode.",
                source=_SOURCE,
                examples=(
                    "when CLIENTSSL_CLIENTHELLO {\n"
                    "                if { [SSL::secure_renegotiation] != 2 } {\n"
                    "                    SSL::secure_renegotiation require-strict\n"
                    "                }\n"
                    "            }"
                ),
                return_value="SSL::secure_renegotiation¶ Get the current Secure Renegotiation mode for the flow. A return value of zero denotes request mode. A value of one denotes require mode. A value of two denotes require-strict mode.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::secure_renegotiation (request | require | require-strict)?",
                    arg_values={
                        0: (
                            _av(
                                "request",
                                "SSL::secure_renegotiation request",
                                "SSL::secure_renegotiation (request | require | require-strict)?",
                            ),
                            _av(
                                "require",
                                "SSL::secure_renegotiation require",
                                "SSL::secure_renegotiation (request | require | require-strict)?",
                            ),
                            _av(
                                "require-strict",
                                "SSL::secure_renegotiation require-strict",
                                "SSL::secure_renegotiation (request | require | require-strict)?",
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
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
