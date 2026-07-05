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
"""SIPALG::hairpin_default -- Gets or sets the value of hairpin flag for the current connection."""

from __future__ import annotations

from compiler.registry._base import CommandDef, make_av
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIPALG__hairpin_default.html"


_av = make_av(_SOURCE)


@register
class SipalgHairpinDefaultCommand(CommandDef):
    name = "SIPALG::hairpin_default"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIPALG::hairpin_default",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Gets or sets the value of hairpin flag for the current connection.",
                synopsis=(
                    "SIPALG::hairpin_default",
                    "SIPALG::hairpin_default (detect | disable | enable)",
                ),
                snippet="Returns the value of the hairpin flag for the current connection.",
                source=_SOURCE,
                examples=(
                    "when SIP_REQUEST {\n"
                    '    log local0. "default hairpin mode [SIPALG::hairpin_default]"\n'
                    "}"
                ),
                return_value="Returns 'detect', 'disable', or 'enable'",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIPALG::hairpin_default",
                    arg_values={
                        0: (
                            _av(
                                "detect",
                                "SIPALG::hairpin_default detect",
                                "SIPALG::hairpin_default (detect | disable | enable)",
                            ),
                            _av(
                                "disable",
                                "SIPALG::hairpin_default disable",
                                "SIPALG::hairpin_default (detect | disable | enable)",
                            ),
                            _av(
                                "enable",
                                "SIPALG::hairpin_default enable",
                                "SIPALG::hairpin_default (detect | disable | enable)",
                            ),
                        )
                    },
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                profiles=frozenset({"SIP"}),
                also_in=frozenset({"CLIENT_ACCEPTED", "SERVER_CONNECTED"}),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
