# Enriched from F5 iRules reference documentation.
"""SOCKS::version -- This command gets the version of the SOCKS protocol."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SOCKS__version.html"


@register
class SocksVersionCommand(CommandDef):
    name = "SOCKS::version"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SOCKS::version",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command gets the version of the SOCKS protocol.",
                synopsis=("SOCKS::version",),
                snippet=(
                    'This command gets the version of the SOCKS protocol, returning one of "4", "4A" or "5".\n'
                    "\n"
                    "Details (Syntax):\n"
                    "SOCKS::version\n"
                    "    Gets the version of the protocol."
                ),
                source=_SOURCE,
                examples=(
                    "when SOCKS_REQUEST {\n"
                    '    log local0. "SOCKS is using version [SOCKS::version]"\n'
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SOCKS::version",
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
