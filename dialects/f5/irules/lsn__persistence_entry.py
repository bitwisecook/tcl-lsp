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
"""LSN::persistence-entry -- Create or lookup LSN translation address."""

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

_SOURCE = "https://clouddocs.f5.com/api/irules/LSN__persistence-entry.html"


_av = make_av(_SOURCE)


@register
class LsnPersistenceEntryCommand(CommandDef):
    name = "LSN::persistence-entry"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LSN::persistence-entry",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Create or lookup LSN translation address.",
                synopsis=(
                    "LSN::persistence-entry (delete|get) CLIENT_ADDR",
                    "LSN::persistence-entry create (-override)? LSN_POOL CLIENT_ADDR TRANSLATION_ADDR (TIMEOUT)?",
                ),
                snippet=(
                    "Create or lookup LSN translation address. Those commands are linked to CGNAT module introduced in 11.3. You need to license and provision this module to use this command.\n"
                    "\n"
                    "LSN::persistence-entry create [-override] <client_address>[:<client_port>] [<translation_address>[:<translation_port>]]\n"
                    "LSN::persistence-entry get <client_address>[:<client_port>]\n"
                    "\n"
                    "v11.4+\n"
                    "LSN::persistence-entry create [-override] <lsn_pool>  <client_address>[:<port>] <translation_address>[:<port>]]  [timeout]\n"
                    "\n"
                    "v11.5+\n"
                    "LSN::persistence-entry delete <client_address>"
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n    set clientIP [IP::client_addr]\n}"),
                return_value="LSN::persistence-entry create",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LSN::persistence-entry (delete|get) CLIENT_ADDR",
                    options=(
                        OptionSpec(name="-override", detail="Option -override.", takes_value=False),
                    ),
                    arg_values={
                        0: (
                            _av(
                                "delete",
                                "LSN::persistence-entry delete",
                                "LSN::persistence-entry (delete|get) CLIENT_ADDR",
                            ),
                            _av(
                                "get",
                                "LSN::persistence-entry get",
                                "LSN::persistence-entry (delete|get) CLIENT_ADDR",
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
