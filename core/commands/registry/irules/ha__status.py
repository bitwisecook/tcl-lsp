# Enriched from F5 iRules reference documentation.
"""HA::status -- Returns true or false based on whether the unit the command is executed on is active or standby."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef, make_av
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/HA__status.html"


_av = make_av(_SOURCE)


@register
class HaStatusCommand(CommandDef):
    name = "HA::status"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="HA::status",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns true or false based on whether the unit the command is executed on is active or standby.",
                synopsis=("HA::status (active | standby)",),
                snippet=(
                    "This iRule command returns true or false based on whether the unit the\n"
                    "command is executed on is active or standby in the context of the\n"
                    "command used. The primary use-case is for iRules that utilize sideband\n"
                    "or HSL commands. This can be used to prevent the standby from opening\n"
                    "extra connections.\n"
                    "A Virtual IP (VIP) is bound to a Traffic Group, which handles failover\n"
                    'for the VIP. A unit can, at the same time, be "active" for one\n'
                    'traffic-group and "standby" for a different traffic-group.'
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    '    log local0. "active: [HA::status active]"\n'
                    '    log local0. "standby: [HA::status standby]"\n'
                    "}"
                ),
                return_value="HA::status active",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="HA::status (active | standby)",
                    arg_values={
                        0: (
                            _av("active", "HA::status active", "HA::status (active | standby)"),
                            _av("standby", "HA::status standby", "HA::status (active | standby)"),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            excluded_events=("RULE_INIT",),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.BIGIP_CONFIG,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
