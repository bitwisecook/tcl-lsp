# Enriched from F5 iRules reference documentation.
"""PROFILE::tcp -- Returns the value of a TCP profile setting."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/PROFILE__tcp.html"


@register
class ProfileTcpCommand(CommandDef):
    name = "PROFILE::tcp"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="PROFILE::tcp",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the value of a TCP profile setting.",
                synopsis=("PROFILE::tcp ATTR",),
                snippet="Returns the current value of the specified setting in an assigned TCP profile.",
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "   # Log the idle timeout on the serverside TCP profile of the VIP (default of 300 seconds)\n"
                    '   log local0. "\\[PROFILE::tcp idle_timeout\\]: [PROFILE::tcp idle_timeout]"\n'
                    "}"
                ),
                return_value="Returns the current value of the specified setting in an assigned TCP profile.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="PROFILE::tcp ATTR",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.BIGIP_CONFIG,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
