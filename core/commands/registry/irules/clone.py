# Enriched from F5 iRules reference documentation.
"""clone -- Causes the system to clone traffic to the specified pool, pool member or vlan regardless of monitor status."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/clone.html"


@register
class CloneCommand(CommandDef):
    name = "clone"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="clone",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Causes the system to clone traffic to the specified pool, pool member or vlan regardless of monitor status.",
                synopsis=(
                    "clone pool POOL_OBJ (member IP_ADDR (PORT)?)?",
                    "clone nexthop VLAN_OBJ",
                ),
                snippet="Causes the system to clone traffic to the specified pool, pool member or vlan regardless of monitor status. (Pool member status may be determined by the use of the LB::status command. Failure to select a server because none are available may be prevented by using the active_members command to test the number of active members in the target pool before choosing it.) Any responses to cloned traffic from pool members will be ignored.",
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n   clone nexthop tap_vlan\n}"),
                return_value="clone pool <pool_name> Specifies the pool to which you want to send the cloned traffic.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="clone pool POOL_OBJ (member IP_ADDR (PORT)?)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.CONNECTION_CONTROL,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
