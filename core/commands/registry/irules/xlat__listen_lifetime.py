# Enriched from F5 iRules reference documentation.
"""XLAT::listen_lifetime -- Set/Get the listener lifetime."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/XLAT__listen_lifetime.html"


@register
class XlatListenLifetimeCommand(CommandDef):
    name = "XLAT::listen_lifetime"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="XLAT::listen_lifetime",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set/Get the listener lifetime.",
                synopsis=("XLAT::listen_lifetime (HANDLE)+ (XLAT_LIFETIME)?",),
                snippet=(
                    "Set/Get the listener lifetime.\n"
                    "Valid range is between 0 and 31536000 (365 days)."
                ),
                source=_SOURCE,
                examples=(
                    "when SERVER_CONNECTED {\n"
                    "    set listener [XLAT::listen 30 {\n"
                    "        proto [IP::protocol]\n"
                    "        bind -allow [serverside {LINK::vlan_id}] -ip [serverside {IP::local_addr}]\n"
                    "        server [IP::client_addr] [expr [TCP::local_port] + 1]\n"
                    "        allow [LB::server addr] 0\n"
                    "    }]\n"
                    '    log local0. "[XLAT::listen_lifetime $listener]"\n'
                    "}"
                ),
                return_value="Return the listener lifetime value.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="XLAT::listen_lifetime (HANDLE)+ (XLAT_LIFETIME)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LSN_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
