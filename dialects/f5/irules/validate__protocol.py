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
"""VALIDATE::protocol -- Performs validation of given application to match payload."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/VALIDATE__protocol.html"


@register
class ValidateProtocolCommand(CommandDef):
    name = "VALIDATE::protocol"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="VALIDATE::protocol",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Performs validation of given application to match payload.",
                synopsis=("VALIDATE::protocol CLASSIFY_APP_NAME ANY_CHARS",),
                snippet=(
                    "This command allows you to validate payload (traffic) to match given classification application.\n"
                    "\n"
                    "Note: the APM / AFM / PEM license is required for functionality to work."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n  TCP::collect 32\n}"),
                return_value="Returns TRUE in case of match, FALSE otherwise.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="VALIDATE::protocol CLASSIFY_APP_NAME ANY_CHARS",
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
