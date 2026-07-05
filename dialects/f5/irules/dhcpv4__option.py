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
"""DHCPv4::option -- This command retrieves,sets or deletes the option by id number."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/DHCPv4__option.html"


_av = make_av(_SOURCE)


@register
class Dhcpv4OptionCommand(CommandDef):
    name = "DHCPv4::option"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="DHCPv4::option",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="This command retrieves,sets or deletes the option by id number.",
                synopsis=("DHCPv4::option (delete)? OPTION (VALUE)?",),
                snippet=(
                    "This command retrieves,sets or deletes the option by id number\n"
                    "\n"
                    "Details (syntax);\n"
                    "DHCPv4::option <id>\n"
                    "DHCPv4::option <id> <value>\n"
                    "DHCPv4::option delete <id>"
                ),
                source=_SOURCE,
                examples=(
                    'when CLIENT_DATA {\n        log local0. "Option [DHCPv4::option 18]"\n    }'
                ),
                return_value="This command returns value by option id number when retrieving",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="DHCPv4::option (delete)? OPTION (VALUE)?",
                    arg_values={
                        0: (
                            _av(
                                "delete",
                                "DHCPv4::option delete",
                                "DHCPv4::option (delete)? OPTION (VALUE)?",
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
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
