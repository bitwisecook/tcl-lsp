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
"""AES::encrypt -- Encrypts the data using the previously-created AES key."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.namespace_models import EventRequires
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import _IRULES_ONLY, register

_SOURCE = "https://clouddocs.f5.com/api/irules/AES__encrypt.html"


@register
class AesEncryptCommand(CommandDef):
    name = "AES::encrypt"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="AES::encrypt",
            dialects=_IRULES_ONLY,
            hover=HoverSnippet(
                summary="Encrypts the data using the previously-created AES key.",
                synopsis=("AES::encrypt KEY DATA",),
                snippet="Encrypt the data using an AES key.",
                source=_SOURCE,
                examples=(
                    "when SERVER_DATA {\n"
                    '  set key "AES 128 43047ad71173be644498b98de6a32fe3"\n'
                    "  set encryptedData [AES::encrypt $key [TCP::payload]]\n"
                    "  TCP::payload replace 0 [TCP::payload length] $encryptedData\n"
                    "}"
                ),
                return_value="Returns the encrypted data.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="AES::encrypt KEY DATA",
                ),
            ),
            validation=ValidationSpec(
                arity=Arity(),
            ),
            event_requires=EventRequires(),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )
