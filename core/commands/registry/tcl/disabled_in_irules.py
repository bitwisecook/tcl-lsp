"""Stub definitions for Tcl commands disabled in iRules without full defs.

These are auto-loading helpers, internal Tcl support commands, and utilities
that BIG-IP does not expose to iRules.  They only need enough metadata to
mark them as excluded from the f5-irules dialect so the registry's
``command_status()`` returns ``DISALLOWED``.
"""

from __future__ import annotations

from .._base import CommandDef
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, HoverSnippet, ValidationSpec
from ..signatures import Arity
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
