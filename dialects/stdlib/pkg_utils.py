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

"""Tcl package management utilities (auto-loaded, no package require needed)."""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.models import CommandSpec, HoverSnippet, ValidationSpec
from compiler.registry.signatures import Arity
from compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget

from ._base import register

_SOURCE = "Tcl stdlib package utilities"


@register
class PkgMkIndex(CommandDef):
    name = "pkg_mkIndex"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="pkg_mkIndex",
            hover=HoverSnippet(
                summary="Build a ``pkgIndex.tcl`` file for one or more packages.",
                synopsis=(
                    "pkg_mkIndex ?-direct? ?-lazy? ?-load pkgPat? ?-verbose? dir ?pattern ...?",
                ),
                snippet=(
                    "Scans *dir* for Tcl source and binary files matching "
                    "*pattern* (default ``*.tcl *.{so,dll}``) and builds a "
                    "``pkgIndex.tcl`` that enables ``package require`` to "
                    "find them."
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(1)),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )


@register
class PkgCreate(CommandDef):
    name = "pkg::create"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="pkg::create",
            hover=HoverSnippet(
                summary="Generate a ``package ifneeded`` script for a package.",
                synopsis=(
                    "pkg::create -name packageName -version packageVersion ?-load filespec? ?-source filespec?",
                ),
                source=_SOURCE,
            ),
            validation=ValidationSpec(arity=Arity(4)),
        )
