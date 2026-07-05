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
"""GTP::new -- Creates a new GTP message for given version & request-type."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/GTP__new.html"


@register
class GtpNewCommand(CommandDef):
    name = "GTP::new"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="GTP::new",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Creates a new GTP message for given version & request-type.",
                synopsis=("GTP::new VERSION TYPE",),
                snippet=(
                    "Creates a new GTP message for given version & request-type.\n"
                    "Valid values for version are 1 or 2 only.\n"
                    "Request-type: A value less than 256.\n"
                    'Returns a TCL object of type "GTP-Message"'
                ),
                source=_SOURCE,
                examples=(
                    "when CLIENT_ACCEPTED {\n"
                    "    set t2 [GTP::new 2 10]\n"
                    '    log local0. "GTP version [GTP::header version -message $t2]"\n'
                    '    log local0. "GTP type [GTP::header type -message $t2]"\n'
                    "}"
                ),
                return_value='Returns a TCL object of type "GTP-Message"',
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="GTP::new VERSION TYPE",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
