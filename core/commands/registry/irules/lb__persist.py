# Enriched from F5 iRules reference documentation.
"""LB::persist -- Forces the system to make a persistence decision."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__persist.html"


_av = make_av(_SOURCE)


@register
class LbPersistCommand(CommandDef):
    name = "LB::persist"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::persist",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Forces the system to make a persistence decision.",
                synopsis=("LB::persist ((key) |",),
                snippet=(
                    "This command forces the system to make a persistence decision, and returns a string that can be evaluated to activate that selection, or with the use of the parameter, returns a persistence key that may be used in conjunction with the persist command to manipulate the persistence table.\n"
                    "\n"
                    "This enables an iRule to evaluate the pending load balancing/persistence decision early, and use that information to manage the connection."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::persist ((key) |",
                    arg_values={0: (_av("key", "LB::persist key", "LB::persist ((key) |"),)},
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.PERSISTENCE_TABLE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
