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
"""translate -- Enables, disables, or queries (as specified) destination address or port translation."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/translate.html"


_av = make_av(_SOURCE)


@register
class TranslateCommand(CommandDef):
    name = "translate"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="translate",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Enables, disables, or queries (as specified) destination address or port translation.",
                synopsis=(
                    "translate (address | port | service)",
                    "translate (address | port | service) ((enable | disable)",
                ),
                snippet=(
                    "Enables, disables, or queries (as specified) destination address or\n"
                    "port translation"
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    if { [IP::addr [IP::remote_addr] equals 10.0.8.0/24] } {\n"
                    "        translate address disable\n"
                    "    }\n"
                    "}"
                ),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="translate (address | port | service)",
                    arg_values={
                        0: (
                            _av(
                                "address",
                                "translate address",
                                "translate (address | port | service)",
                            ),
                            _av("port", "translate port", "translate (address | port | service)"),
                            _av(
                                "service",
                                "translate service",
                                "translate (address | port | service)",
                            ),
                            _av(
                                "enable",
                                "translate enable",
                                "translate (address | port | service) ((enable | disable)",
                            ),
                            _av(
                                "disable",
                                "translate disable",
                                "translate (address | port | service) ((enable | disable)",
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
                    target=SideEffectTarget.UNKNOWN,
                    reads=True,
                    connection_side=ConnectionSide.GLOBAL,
                ),
            ),
        )
