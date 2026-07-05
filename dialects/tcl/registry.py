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

"""registry -- Windows registry access (Windows-only Tcl command)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import (
    CommandSpec,
    FormKind,
    FormSpec,
    HoverSnippet,
    ValidationSpec,
)
from compiler.registry.signatures import Arity
from compiler.types import TclType

from ._base import register

_SOURCE = "Tcl man page registry.n"


@register
class RegistryCommand(CommandDef):
    name = "registry"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="registry",
            dialects=frozenset({"tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0"}),
            hover=HoverSnippet(
                summary="Windows registry manipulation (Windows-only).",
                synopsis=(
                    "registry broadcast keyName ?-timeout ms?",
                    "registry delete keyName ?valueName?",
                    "registry get keyName valueName",
                    "registry keys keyName ?pattern?",
                    "registry set keyName ?valueName data ?type??",
                    "registry type keyName valueName",
                    "registry values keyName ?pattern?",
                ),
                snippet=(
                    "Not available in the WASM sandbox (Windows-specific) "
                    "— traps with ``unsupported command: registry``."
                ),
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="registry subcommand keyName ?args ...?",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            return_type=TclType.STRING,
        )
