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
"""LSN::inbound-entry -- This command creates and gets the inbound mapping for a translation address, translation port and protocol."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    OptionSpec,
    ValidationSpec,
)
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LSN__inbound-entry.html"


_av = make_av(_SOURCE)


@register
class LsnInboundEntryCommand(CommandDef):
    name = "LSN::inbound-entry"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LSN::inbound-entry",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command creates and gets the inbound mapping for a translation address, translation port and protocol.",
                synopsis=(
                    "LSN::inbound-entry (get | delete) IP_TUPLE IP_PROTOCOL",
                    "LSN::inbound-entry create (-mirror)?",
                ),
                snippet=(
                    "This command creates and gets the inbound mapping for a translation address, translation port and protocol.\n"
                    "\n"
                    "LSN::inbound-entry get <translation_address>:<translation_port> <protocol>\n"
                    "LSN::inbound-entry create [-mirror] [-override] [-dslite <dslite local address> <dslite remote address>] [-prefix <IPv6 address>] <LSN pool name> <timeout> <client IP:client port> <translation address:translation port> <protocol>\n"
                    "\n"
                    "v11.5+\n"
                    "LSN::inbound-entry delete <translation_address>:<translation_port> <protocol>"
                ),
                source=_SOURCE,
                return_value="LSN::inbound-entry get <translation IP>:<translation port> <protocol> - Gets inbound entry for the specified translation IP, translation port and protocol. Protocol can be set TCP or UDP. This command returns the client IP address, port and route domain ID.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LSN::inbound-entry (get | delete) IP_TUPLE IP_PROTOCOL",
                    options=(
                        OptionSpec(name="-mirror", detail="Option -mirror.", takes_value=False),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "get",
                                "LSN::inbound-entry get",
                                "LSN::inbound-entry (get | delete) IP_TUPLE IP_PROTOCOL",
                            ),
                            _av(
                                "delete",
                                "LSN::inbound-entry delete",
                                "LSN::inbound-entry (get | delete) IP_TUPLE IP_PROTOCOL",
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
                    target=SideEffectTarget.LSN_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
