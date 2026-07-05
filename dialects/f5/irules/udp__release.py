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
"""UDP::release -- Allow client-side ingress to flow following a call to UDP::hold."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/UDP__release.html"


@register
class UdpReleaseCommand(CommandDef):
    name = "UDP::release"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="UDP::release",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Allow client-side ingress to flow following a call to UDP::hold.",
                synopsis=("UDP::release",),
                snippet="Called at some point after UDP::hold was called.  Unblock ingress on client side.",
                source=_SOURCE,
                examples=("when LB_SELECTED {\n    UDP::release\n}"),
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="UDP::release",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(
                transport="udp",
                also_in=frozenset(
                    {"SIP_REQUEST", "SIP_REQUEST_SEND", "SIP_RESPONSE", "STREAM_MATCHED"}
                ),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UDP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
