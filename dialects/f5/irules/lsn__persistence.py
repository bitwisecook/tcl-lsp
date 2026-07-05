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
"""LSN::persistence -- Set the translation address and port selection mode for the current connection, and the translation entry timeout."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, _LSN_EVENT_REQUIRES, register

_SOURCE = "https://clouddocs.f5.com/api/irules/LSN__persistence.html"


_av = make_av(_SOURCE)


@register
class LsnPersistenceCommand(CommandDef):
    name = "LSN::persistence"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="LSN::persistence",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Set the translation address and port selection mode for the current connection, and the translation entry timeout.",
                synopsis=(
                    "LSN::persistence none (TIMEOUT)?",
                    "LSN::persistence (address | address-port) TIMEOUT",
                ),
                snippet=(
                    "Set the translation address and port selection mode for the current connection, and the translation entry timeout.\n"
                    "\n"
                    "LSN::persistence <none|address|address-port|strict-address-port> <timeout>"
                ),
                source=_SOURCE,
                return_value="LSN::persistence none",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="LSN::persistence none (TIMEOUT)?",
                    arg_values={
                        0: (
                            _av(
                                "address",
                                "LSN::persistence address",
                                "LSN::persistence (address | address-port) TIMEOUT",
                            ),
                            _av(
                                "address-port",
                                "LSN::persistence address-port",
                                "LSN::persistence (address | address-port) TIMEOUT",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=_LSN_EVENT_REQUIRES,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.LSN_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
