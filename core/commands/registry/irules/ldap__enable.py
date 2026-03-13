# Enriched from F5 iRules reference documentation.
"""LDAP::enable -- Enable LDAP STARTTLS."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from .._base import CommandDef
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from ..namespace_models import EventRequires
from ..signatures import Arity
from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LDAP__enable.html"


@register
class LdapEnableCommand(CommandDef):
    name = "LDAP::enable"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LDAP::enable",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enable LDAP STARTTLS.",
                synopsis=("LDAP::enable",),
                snippet="Enable LDAP STARTTLS",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                if { !([IP::addr [IP::client_addr] equals 10.0.0.0/8]) } {\n"
                    "                    LDAP::enable\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LDAP::enable",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                also_in=frozenset({"CLIENT_ACCEPTED", "SERVER_CONNECTED"})
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
