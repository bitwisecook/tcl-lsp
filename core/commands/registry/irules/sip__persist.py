# Enriched from F5 iRules reference documentation.
"""SIP::persist -- Returns or replaces the persistence key being used for the current message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__persist.html"


@register
class SipPersistCommand(CommandDef):
    name = "SIP::persist"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::persist",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns or replaces the persistence key being used for the current message.",
                synopsis=(
                    "SIP::persist",
                    "SIP::persist reset",
                    "SIP::persist use",
                    "SIP::persist replace",
                ),
                snippet=(
                    "The SIP::persist command returns the persistence key being used for the\n"
                    "current message. If new-persist-key is provided, the existing\n"
                    "persistence key is replaced. The value of the new-persist-key MUST be\n"
                    "one of valid header value in the message. A header name should not be\n"
                    "given as the new-persist-key value.\n"
                    "Only valid for MRF SIP (sipsession profile) in 11.6+"
                ),
                source=_SOURCE,
                examples=(
                    "when SIP_REQUEST {\n"
                    '                SIP::persist "[SIP::uri]-[SIP::from]-[SIP::to]"\n'
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::persist",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"SIP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.PERSISTENCE_TABLE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
