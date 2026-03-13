# Enriched from F5 iRules reference documentation.
"""DIAMETER::retry -- Tries to send the Diameter message contained in the binary array \"binary_message\"."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DIAMETER__retry.html"


@register
class DiameterRetryCommand(CommandDef):
    name = "DIAMETER::retry"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DIAMETER::retry",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary='Tries to send the Diameter message contained in the binary array "binary_message".',
                synopsis=("DIAMETER::retry DIAMETER_MESSAGE (BOOL_ACROSS)?",),
                snippet=(
                    "This iRule command tries to send the Diameter message contained in the\n"
                    'binary array "binary_message".  This command, in conjunction with the\n'
                    "DIAMETER::message command, can be used to write an iRule that will\n"
                    "hold and retry messages.\n"
                    "\n"
                    'If the optional argument "across" is specified as 1, the message will\n'
                    "be sent through the proxy and trigger the various iRule events.  If it\n"
                    "is specified as 0, or not specified, the message will be sent directly\n"
                    "and not experience any iRules, persistence, or other processing."
                ),
                source=_SOURCE,
                examples=(
                    "when DIAMETER_EGRESS {\n"
                    "   if { [DIAMETER::is_request] } {\n"
                    "      set saved_message([DIAMETER::header hopid]) [DIAMETER::message]\n"
                    "   }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DIAMETER::retry DIAMETER_MESSAGE (BOOL_ACROSS)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"DIAMETER", "MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
