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

# Generated from F5 iRules reference documentation -- do not edit manually.
"""NAME::lookup -- Deprecated: Performs DNS query for A or PTR record corresponding to a hostname or IP address."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register
from .resolv__lookup import ResolvLookupCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/NAME__lookup.html"


@register
class NameLookupCommand(CommandDef):
    name = "NAME::lookup"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="NAME::lookup",
            deprecated_replacement=ResolvLookupCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Deprecated: Performs DNS query for A or PTR record corresponding to a hostname or IP address.",
                synopsis=("NAME::lookup",),
                snippet="Performs a DNS query, typically returning the A record for the indicated hostname, or the PTR record for the indicated IP address.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="NAME::lookup",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.DNS_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
