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
"""imid -- Returns an i-mode identifier string."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/imid.html"


@register
class ImidCommand(CommandDef):
    name = "imid"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="imid",
            deprecated_replacement="IMID::id",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns an i-mode identifier string.",
                synopsis=("imid",),
                snippet=(
                    "Parses the BIG-IP 4.X http_uri variable and the user-agent header field to return an i-mode identifier string that can be used for i-mode session persistence. This is a BIG-IP 4.X function, provided for backward compatibility.\n"
                    "\n"
                    "The imid function takes no arguments and simply returns the string representing the i-mode identifier or the empty string, if none is found."
                ),
                source=_SOURCE,
                return_value="Parses the BIG-IP 4.X http_uri variable and the '''user-agent* header field to return an i-mode identifier string that can be used for i-mode session persistence.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="imid",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(transport="tcp", profiles=frozenset({"HTTP", "FASTHTTP"})),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.TCP_STATE,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
