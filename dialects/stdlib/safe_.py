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

"""safe -- Tcl stdlib Safe Base package (auto-loaded, no package require needed)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_SOURCE = "Tcl stdlib Safe Base"

# Safe interpreters are part of the Tcl core library but loaded via
# ``package require safe`` or auto-loaded when any safe:: command is invoked.
_PKG = "safe"


@register
class SafeInterpCreate(CommandDef):
    name = "safe::interpCreate"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="safe::interpCreate",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Create a safe child interpreter with restricted capabilities.",
                synopsis=("safe::interpCreate ?child? ?options...?",),
                snippet=(
                    "Creates a safe interpreter.  Options include "
                    "``-accessPath``, ``-statics``, ``-noStatics``, "
                    "``-nested``, ``-noNested``, ``-deleteHook``."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class SafeInterpInit(CommandDef):
    name = "safe::interpInit"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="safe::interpInit",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Initialise an existing interpreter as a safe interpreter.",
                synopsis=("safe::interpInit child ?options...?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )


@register
class SafeInterpConfigure(CommandDef):
    name = "safe::interpConfigure"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="safe::interpConfigure",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Get or set the configuration of a safe interpreter.",
                synopsis=("safe::interpConfigure child ?options...?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1)),
        )


@register
class SafeInterpDelete(CommandDef):
    name = "safe::interpDelete"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="safe::interpDelete",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Delete a safe interpreter and release its resources.",
                synopsis=("safe::interpDelete child",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class SafeInterpAddToAccessPath(CommandDef):
    name = "safe::interpAddToAccessPath"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="safe::interpAddToAccessPath",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Add a directory to a safe interpreter's access path.",
                synopsis=("safe::interpAddToAccessPath child directory",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(2, 2)),
        )


@register
class SafeInterpFindInAccessPath(CommandDef):
    name = "safe::interpFindInAccessPath"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="safe::interpFindInAccessPath",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Return the token for a directory in a safe interpreter's access path.",
                synopsis=("safe::interpFindInAccessPath child directory",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(2, 2)),
        )


@register
class SafeSetLogCmd(CommandDef):
    name = "safe::setLogCmd"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="safe::setLogCmd",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Set or query the logging command for Safe Base messages.",
                synopsis=("safe::setLogCmd ?cmd arg...?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0)),
        )


@register
class SafeSetSyncMode(CommandDef):
    name = "safe::setSyncMode"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="safe::setSyncMode",
            required_package=_PKG,
            hover=HoverSnippet(
                summary="Set or query the synchronous-source mode for a safe interpreter.",
                synopsis=("safe::setSyncMode ?child? ?boolean?",),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(0, 2)),
        )
