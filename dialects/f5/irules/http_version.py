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
"""http_version -- F5 iRules command `http_version`."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register
from .http__version import HttpVersionCommand

_SOURCE = "https://clouddocs.f5.com/api/irules/http_version.html"


@register
class DeprecatedHttpVersionCommand(CommandDef):
    name = "http_version"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http_version",
            deprecated_replacement=HttpVersionCommand,
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Returns the HTTP protocol version.",
                synopsis=("http_version",),
                snippet=(
                    'Returns the HTTP protocol version. Possible values are "HTTP/1.0" or\n'
                    '"HTTP/1.1". This is a BIG-IP version 4.X variable, provided for\n'
                    "backward compatibility. You can use the equivalent 9.X command,\n"
                    "HTTP::version instead."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="http_version",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.HTTP_STATUS,
                    reads=True,
                    connection_side=ConnectionSide.BOTH,
                ),
            ),
        )
