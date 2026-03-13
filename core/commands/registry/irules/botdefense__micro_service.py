# Enriched from F5 iRules reference documentation.
"""BOTDEFENSE::micro_service -- Returns the micro-service that matched the current request."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/BOTDEFENSE__micro_service.html"


_av = make_av(_SOURCE)


@register
class BotdefenseMicroServiceCommand(CommandDef):
    name = "BOTDEFENSE::micro_service"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="BOTDEFENSE::micro_service",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the micro-service that matched the current request.",
                synopsis=("BOTDEFENSE::micro_service (name | type)",),
                snippet="Returns the micro-service that matched the current request.",
                source=_SOURCE,
                examples=(
                    "when BOTDEFENSE_ACTION {\n"
                    "    set ms [BOTDEFENSE::micro_service name]\n"
                    '    if { $ms neq ""} {\n'
                    '        log.local0. "Request to micro_service $ms of type [BOTDEFENSE::micro_service type]\n'
                    "    }\n"
                    "}"
                ),
                return_value="Returns the name or type of the micro-service found for the current request",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="BOTDEFENSE::micro_service (name | type)",
                    arg_values={
                        0: (
                            _av(
                                "name",
                                "BOTDEFENSE::micro_service name",
                                "BOTDEFENSE::micro_service (name | type)",
                            ),
                            _av(
                                "type",
                                "BOTDEFENSE::micro_service type",
                                "BOTDEFENSE::micro_service (name | type)",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"BOTDEFENSE"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
