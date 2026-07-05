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

"""Stub definitions for Tcl commands disabled in iRules without full defs.

These are auto-loading helpers, internal Tcl support commands, and utilities
that BIG-IP does not expose to iRules.  They only need enough metadata to
mark them as excluded from the f5-irules dialect so the registry's
``command_status()`` returns ``DISALLOWED``.
"""

from __future__ import annotations

from compiler.registry._base import CommandDef
from compiler.registry.dialects import DIALECTS_EXCEPT_IRULES
from compiler.registry.models import CommandSpec, HoverSnippet, ValidationSpec, WasmRuntimeImport
from compiler.registry.signatures import Arity

from ._base import register

_NOT_IRULES = DIALECTS_EXCEPT_IRULES


@register
class AutoExecokCommand(CommandDef):
    name = "auto_execok"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="auto_execok",
            dialects=_NOT_IRULES,
            hover=HoverSnippet(summary="Return path of executable, or empty string"),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class AutoImportCommand(CommandDef):
    name = "auto_import"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="auto_import",
            dialects=_NOT_IRULES,
            hover=HoverSnippet(summary="Import auto-loaded commands into namespace"),
            validation=ValidationSpec(arity=Arity(1)),
        )


@register
class AutoLoadCommand(CommandDef):
    name = "auto_load"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="auto_load",
            dialects=_NOT_IRULES,
            hover=HoverSnippet(summary="Auto-load a command from the library"),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class AutoMkindexCommand(CommandDef):
    name = "auto_mkindex"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="auto_mkindex",
            dialects=_NOT_IRULES,
            hover=HoverSnippet(summary="Generate tclIndex from Tcl source files"),
            validation=ValidationSpec(arity=Arity(1)),
        )


@register
class AutoMkindexOldCommand(CommandDef):
    name = "auto_mkindex_old"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="auto_mkindex_old",
            dialects=_NOT_IRULES,
            hover=HoverSnippet(summary="Legacy tclIndex generator"),
            validation=ValidationSpec(arity=Arity(1)),
        )


@register
class AutoQualifyCommand(CommandDef):
    name = "auto_qualify"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="auto_qualify",
            dialects=_NOT_IRULES,
            hover=HoverSnippet(summary="Compute fully-qualified names for auto-loading"),
            validation=ValidationSpec(arity=Arity(2, 2)),
        )


@register
class AutoResetCommand(CommandDef):
    name = "auto_reset"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="auto_reset",
            dialects=_NOT_IRULES,
            hover=HoverSnippet(summary="Reset auto-loading state"),
            validation=ValidationSpec(arity=Arity(0, 0)),
        )


@register
class BgerrorCommand(CommandDef):
    name = "bgerror"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="bgerror",
            dialects=_NOT_IRULES,
            hover=HoverSnippet(summary="Handle background errors"),
            validation=ValidationSpec(arity=Arity(1, 1)),
        )


@register
class FilenameCommand(CommandDef):
    name = "filename"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="filename",
            dialects=_NOT_IRULES,
            hover=HoverSnippet(summary="File name conventions"),
        )


@register
class HttpCommand(CommandDef):
    name = "http"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="http",
            dialects=_NOT_IRULES,
            hover=HoverSnippet(summary="HTTP client implementation (package http)"),
        )


@register
class MemoryCommand(CommandDef):
    name = "memory"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="memory",
            dialects=_NOT_IRULES,
            hover=HoverSnippet(summary="Memory debugging (debug builds only)"),
            validation=ValidationSpec(arity=Arity(1)),
        )


@register
class PkgCreateCommand(CommandDef):
    name = "pkg::create"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="pkg::create",
            dialects=_NOT_IRULES,
            hover=HoverSnippet(summary="Create a package ifneeded script"),
            validation=ValidationSpec(arity=Arity(1)),
        )


@register
class PkgMkindexCommand(CommandDef):
    name = "pkg_mkindex"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="pkg_mkindex",
            dialects=_NOT_IRULES,
            hover=HoverSnippet(summary="Build pkgIndex.tcl for a directory of packages"),
            validation=ValidationSpec(arity=Arity(1)),
        )


@register
class PwdCommand(CommandDef):
    name = "pwd"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="pwd",
            dialects=_NOT_IRULES,
            hover=HoverSnippet(summary="Return current working directory"),
            validation=ValidationSpec(arity=Arity(0, 0)),
            returns_path=True,
            wasm_runtime_import=WasmRuntimeImport(
                import_key="tcl_pwd",
                argc=0,
                params=(),
                results=("i32",),
            ),
        )


@register
class TclFindLibraryCommand(CommandDef):
    name = "tcl_findLibrary"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="tcl_findLibrary",
            dialects=_NOT_IRULES,
            hover=HoverSnippet(summary="Locate a Tcl library directory"),
            validation=ValidationSpec(arity=Arity(5, 6)),
        )
