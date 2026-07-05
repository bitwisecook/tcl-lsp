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
"""SIP::call_id -- Returns the value of the Call-ID header in a SIP request."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SIP__call_id.html"


@register
class SipCallIdCommand(CommandDef):
    name = "SIP::call_id"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SIP::call_id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the value of the Call-ID header in a SIP request.",
                synopsis=("SIP::call_id",),
                snippet=(
                    "Returns the value of the Call-ID header in a SIP request. Only the\n"
                    "first 256 bytes of the Call-ID will be returned."
                ),
                source=_SOURCE,
                examples=('when SIP_REQUEST_SEND {\n    log local0. "Call ID [SIP::call_id]"\n}'),
                return_value="Returns the value of the Call-ID header in a SIP request",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SIP::call_id",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(profiles=frozenset({"SIP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.NETWORK_IO,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
