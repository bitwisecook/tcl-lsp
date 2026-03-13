# Generated from F5 iRules reference documentation -- do not edit manually.
"""LB::command -- LB::command"""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/lb__command.html"


_av = make_av(_SOURCE)


@register
class LbCommandCommand(CommandDef):
    name = "LB::command"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::command",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="LB::command",
                synopsis=("LB::command ('transparent_port')?",),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::command ('transparent_port')?",
                    arg_values={
                        0: (
                            _av(
                                "transparent_port",
                                "LB::command transparent_port",
                                "LB::command ('transparent_port')?",
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
                    target=SideEffectTarget.POOL_SELECTION,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.SERVER,
                ),
            ),
        )
