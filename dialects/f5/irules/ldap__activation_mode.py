# tcl-lsp — a language server and toolchain for Tcl
# Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
#
# This program is free software: you can redistribute it and/or modify
# it under the terms of the GNU Affero General Public License as published by
# the Free Software Foundation, either version 3 of the License, or
# (at your option) any later version.
#
# This program is distributed in the hope that it will be useful,
# but WITHOUT ANY WARRANTY; without even the implied warranty of
# MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
# GNU Affero General Public License for more details.
#
# You should have received a copy of the GNU Affero General Public License
# along with this program.  If not, see <https://www.gnu.org/licenses/>.
#
# SPDX-License-Identifier: AGPL-3.0-or-later

# Enriched from F5 iRules reference documentation.
"""LDAP::activation_mode -- Set the activation mode."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LDAP__activation_mode.html"


_av = make_av(_SOURCE)


@register
class LdapActivationModeCommand(CommandDef):
    name = "LDAP::activation_mode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LDAP::activation_mode",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set the activation mode.",
                synopsis=("LDAP::activation_mode (none | allow | require)",),
                snippet="Sets the activation mode to none (it will never activate), allow (if the SMTP client sends STARTTLS, we will activate TLS), or require (all commands will be rejected until STARTTLS is received).",
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "                if { !([IP::addr [IP::client_addr] ne 10.0.0.0/8) } {\n"
                    "                    LDAP::activation_mode require\n"
                    "                }\n"
                    "            }"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LDAP::activation_mode (none | allow | require)",
                    arg_values={
                        0: (
                            _av(
                                "none",
                                "LDAP::activation_mode none",
                                "LDAP::activation_mode (none | allow | require)",
                            ),
                            _av(
                                "allow",
                                "LDAP::activation_mode allow",
                                "LDAP::activation_mode (none | allow | require)",
                            ),
                            _av(
                                "require",
                                "LDAP::activation_mode require",
                                "LDAP::activation_mode (none | allow | require)",
                            ),
                        )
                    },
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
