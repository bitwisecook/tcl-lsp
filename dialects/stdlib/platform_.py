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

"""platform -- Tcl stdlib platform identification package (package require platform)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_PKG = "platform"
_SOURCE = "Tcl stdlib platform package"


@register
class PlatformIdentify(CommandDef):
    name = "platform::identify"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="platform::identify",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return the platform identifier for the current machine.",
                synopsis=("platform::identify",),
                snippet=(
                    "Returns a string like ``linux-x86_64`` or ``macosx-arm`` "
                    "that specifically identifies the current platform, including "
                    "CPU details and libc version where relevant."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0, 0)),
            pure=True,
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.UNKNOWN, reads=True, connection_side=ConnectionSide.NONE
                ),
            ),
        )


@register
class PlatformGeneric(CommandDef):
    name = "platform::generic"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="platform::generic",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return the generic platform identifier (less specific than identify).",
                synopsis=("platform::generic",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0, 0)),
            pure=True,
        )


@register
class PlatformPatterns(CommandDef):
    name = "platform::patterns"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="platform::patterns",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return a list of platform patterns that match the given identifier.",
                synopsis=("platform::patterns id",),
                snippet=(
                    "Given a platform *id* from ``platform::identify``, "
                    "returns a list of platform patterns in order of "
                    "decreasing specificity."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
            pure=True,
        )


# platform::shell sub-package


@register
class PlatformShellIdentify(CommandDef):
    name = "platform::shell::identify"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="platform::shell::identify",
            required_package="platform::shell",
            hover=HoverSnippet(
                summary="Return the platform identifier for a given Tcl shell.",
                synopsis=("platform::shell::identify shell",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class PlatformShellGeneric(CommandDef):
    name = "platform::shell::generic"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="platform::shell::generic",
            required_package="platform::shell",
            hover=HoverSnippet(
                summary="Return the generic platform identifier for a given Tcl shell.",
                synopsis=("platform::shell::generic shell",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )
