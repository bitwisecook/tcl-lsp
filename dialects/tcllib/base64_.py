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

"""base64 -- Base64 encoding and decoding (tcllib)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, FormKind, FormSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_SOURCE = "tcllib base64 package"
_PACKAGE = "base64"


@register
class Base64EncodeCommand(CommandDef):
    name = "base64::encode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Encode binary data as a base64 string.",
                synopsis=("base64::encode ?-maxlen maxlen? ?-wrapchar wrapchar? data",),
                source=_SOURCE,
                examples="set encoded [base64::encode $binaryData]",
                return_value="A base64-encoded string.",
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="base64::encode ?-maxlen maxlen? ?-wrapchar wrapchar? data",
                ),
            ),
            validation=ValidationSpec(arity=Arity(1, 5)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )


@register
class Base64DecodeCommand(CommandDef):
    name = "base64::decode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name=cls.name,
            tcllib_package=_PACKAGE,
            hover=HoverSnippet(
                summary="Decode a base64-encoded string back to binary data.",
                synopsis=("base64::decode encodedData",),
                source=_SOURCE,
                examples="set binary [base64::decode $encodedString]",
                return_value="The decoded binary data.",
            ),
            forms=(FormSpec(kind=FormKind.DEFAULT, synopsis="base64::decode encodedData"),),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )
