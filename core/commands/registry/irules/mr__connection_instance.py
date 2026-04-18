# Enriched from F5 iRules reference documentation.
"""MR::connection_instance -- Returns the connection instance and the number of connections."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/MR__connection_instance.html"


@register
class MrConnectionInstanceCommand(CommandDef):
    name = "MR::connection_instance"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="MR::connection_instance",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the connection instance and the number of connections.",
                synopsis=("MR::connection_instance",),
                snippet=(
                    "returns the connection instance number of the current connection and the number of\n"
                    "connections as configured in the peer object used to create the connection.\n"
                    'The return will be formated as "<instance> of <num_connections>".\n'
                    'For incoming connections, it will return "0 of 1".'
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    '    log local0. "[MR::connection_instance] [MR::connection_mode]"\n'
                    "}"
                ),
                return_value='returns the connection instance number and the number of connections formatted as "<instance> of <num_connections>".',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="MR::connection_instance",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.MESSAGE_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
