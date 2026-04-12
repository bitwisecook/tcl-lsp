# Enriched from F5 iRules reference documentation.
"""CLASSIFICATION::protocol -- Depreated: provides classification for the least explicit application name."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/CLASSIFICATION__protocol.html"


@register
class ClassificationProtocolCommand(CommandDef):
    name = "CLASSIFICATION::protocol"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="CLASSIFICATION::protocol",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Depreated: provides classification for the least explicit application name.",
                synopsis=("CLASSIFICATION::protocol",),
                snippet=(
                    "This command provides classification for the least explicit application\n"
                    "name. (Example: http, ssl)\n"
                    "\n"
                    "* Note: APM / AFM / PEM license is required for functionality to work.\n"
                    "\n"
                    "CLASSIFICATION::protocol"
                ),
                source=_SOURCE,
                examples=(
                    "when CLASSIFICATION_DETECTED {\n"
                    '  if { [CLASSIFICATION::protocol] == "http"}  {\n'
                    "    drop\n"
                    "  }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="CLASSIFICATION::protocol",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"CLASSIFICATION"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CLASSIFICATION_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
