# Enriched from F5 iRules reference documentation.
"""DIAMETER::persist -- Returns the persistence key being used for the current message."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__persist.html"


@register
class DiameterPersistCommand(CommandDef):
    name = "DIAMETER::persist"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::persist",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the persistence key being used for the current message.",
                synopsis=(
                    "DIAMETER::persist",
                    "DIAMETER::persist reset",
                    "DIAMETER::persist use",
                ),
                snippet=(
                    "This iRule command returns the persistence key being used for the\n"
                    "current message. If new persist key is provided, the existing\n"
                    "persistence key will be replaced. The value of the new key MUST be the\n"
                    "value of a valid AVP in the message. An AVP attribute name should not\n"
                    "be given as the new key value.\n"
                    "\n"
                    "If bidirection is specified as false, disable(d), no, 0, or is\n"
                    "unspecified, then persistence is not bidirectional. If bidirection is\n"
                    "specified as true, enable(d), yes, or 1 this persistence entry is\n"
                    "bidirectional."
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_INGRESS {\n"
                    '    log local0. "Received a DIAMETER message, persistence key is [DIAMETER::persist]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::persist",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER", "MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.PERSISTENCE_TABLE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
