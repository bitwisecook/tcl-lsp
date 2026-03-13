# Enriched from F5 iRules reference documentation.
"""LB::connlimit -- Set the connection limit for virtual/node/poolmember."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LB__connlimit.html"


_av = make_av(_SOURCE)


@register
class LbConnlimitCommand(CommandDef):
    name = "LB::connlimit"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LB::connlimit",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set the connection limit for virtual/node/poolmember.",
                synopsis=("LB::connlimit ('virtual' | 'node' | 'poolmember')",),
                snippet="Set the connection limit for virtual/node/poolmember",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LB::connlimit ('virtual' | 'node' | 'poolmember')",
                    arg_values={
                        0: (
                            _av(
                                "virtual",
                                "LB::connlimit virtual",
                                "LB::connlimit ('virtual' | 'node' | 'poolmember')",
                            ),
                            _av(
                                "node",
                                "LB::connlimit node",
                                "LB::connlimit ('virtual' | 'node' | 'poolmember')",
                            ),
                            _av(
                                "poolmember",
                                "LB::connlimit poolmember",
                                "LB::connlimit ('virtual' | 'node' | 'poolmember')",
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
