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
"""ASM::fingerprint -- Returns the fingerprint (device id) of the client device."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/ASM__fingerprint.html"


@register
class AsmFingerprintCommand(CommandDef):
    name = "ASM::fingerprint"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="ASM::fingerprint",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the fingerprint (device id) of the client device.",
                synopsis=("ASM::fingerprint",),
                snippet=(
                    "Get the fingerprint of the client device as seen by ASM when it's available.\n"
                    "The fingerprint is a unique identifier given to specific client machine. The fingerprint will be available to iRule only for web application that have web scraping turned on with the finger print usage activated."
                ),
                source=_SOURCE,
                examples=("when ASM_REQUEST_DONE {\n    log local0.[ASM::fingerprint]\n}"),
                return_value="Returns the fingerprint of the client device or 0 if it's not available.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="ASM::fingerprint",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"ASM"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.ASM_STATE,
                    reads=True,
                    connection_side=ConnectionSide.CLIENT,
                ),
            ),
        )
