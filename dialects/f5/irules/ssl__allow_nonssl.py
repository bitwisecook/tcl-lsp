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
"""SSL::allow_nonssl -- Get or set Allow Non-SSL connections."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/SSL__allow_nonssl.html"


@register
class SslAllowNonsslCommand(CommandDef):
    name = "SSL::allow_nonssl"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="SSL::allow_nonssl",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Get or set Allow Non-SSL connections.",
                synopsis=("SSL::allow_nonssl (ZERO_ONE)?",),
                snippet=(
                    "SSL::allow_nonssl\n"
                    "  Returns the currently set value for Allow Non-SSL connections\n"
                    "SSL::allow_nonssl ( 0 | 1 )\n"
                    "  0 disables Non-SSL Connections, 1 enables it.\n"
                    "  Allow Non-ssl connections, sets SSL to passthrough mode."
                ),
                source=_SOURCE,
                examples=("when CLIENT_ACCEPTED {\n    SSL::allow_nonssl 1\n}"),
                return_value="SSL::allow_nonssl Returns the currently set Allow Non-SSL connections value. SSL::allow_nonssl [0|1] There is no return value.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="SSL::allow_nonssl (ZERO_ONE)?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.SSL_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
