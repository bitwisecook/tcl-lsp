# Enriched from F5 iRules reference documentation.
"""MR::restore -- Returns the stored variables to the current context tcl variable store."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__restore.html"


@register
class MrRestoreCommand(CommandDef):
    name = "MR::restore"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::restore",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the stored variables to the current context tcl variable store.",
                synopsis=("MR::restore (VAR)*",),
                snippet="The MR::restore command retrieves one or more named Tcl variables previously stored with the message by the MR::store command. If no name is provided, it retrieves all stored variables from the current message context.",
                source=_SOURCE,
                examples=("when MR_EGRESS {\n    MR::restore client_addr\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::restore (VAR)*",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"MR"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
