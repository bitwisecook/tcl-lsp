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

"""sha1/sha2 -- SHA hash functions (tcllib)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_SOURCE_SHA1 = "tcllib sha1 package"
_SOURCE_SHA2 = "tcllib sha2 package"


@register
class Sha1Sha1Command(CommandDef):
    name = "sha1::sha1"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package="sha1",
            hover=HoverSnippet(
                summary="Compute the SHA-1 hash of a string or file.",
                synopsis=(
                    "sha1::sha1 ?-hex|-bin? ?-channel channel | -file filename | ?--? string?",
                ),
                source=_SOURCE_SHA1,
                return_value="The SHA-1 hash as a hex or binary string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="sha1::sha1 ?options? ?--? string",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
            pure=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )


@register
class Sha2Sha256Command(CommandDef):
    name = "sha2::sha256"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package="sha2",
            hover=HoverSnippet(
                summary="Compute the SHA-256 hash of a string or file.",
                synopsis=(
                    "sha2::sha256 ?-hex|-bin? ?-channel channel | -file filename | ?--? string?",
                ),
                source=_SOURCE_SHA2,
                return_value="The SHA-256 hash as a hex or binary string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="sha2::sha256 ?options? ?--? string",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1)),
            pure=True,
        )
