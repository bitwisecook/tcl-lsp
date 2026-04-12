# Enriched from F5 iRules reference documentation.
"""SOCKS::allowed -- This command allows you to change whether the SOCKS request is allowed or not."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SOCKS__allowed.html"


@register
class SocksAllowedCommand(CommandDef):
    name = "SOCKS::allowed"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SOCKS::allowed",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command allows you to change whether the SOCKS request is allowed or not.",
                synopsis=("SOCKS::allowed ('0' | '1')?",),
                snippet=(
                    "This command allows you to reject a SOCKS request during the SOCKS_REQUEST event.\n"
                    "\n"
                    "Details (Syntax):\n"
                    "SOCKS::allowed '0' | '1'\n"
                    "    Sets the state of SOCKS based on the Boolean value."
                ),
                source=_SOURCE,
                examples=(
                    "# Reject all SOCKS requests:\nwhen SOCKS_REQUEST {\n    SOCKS::allowed 0\n}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SOCKS::allowed ('0' | '1')?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"SOCKS"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
