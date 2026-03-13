# Scaffolded from package.n -- refine and commit
"""package -- Facilities for package loading and version control."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef, make_av
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page package.n"


_av = make_av(_SOURCE)


@register
class PackageCommand(CommandDef):
    name = "package"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="package",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Facilities for package loading and version control",
                synopsis=(
                    "package files package",
                    "package forget ?package package ...?",
                    "package ifneeded package version ?script?",
                    "package names",
                ),
                snippet="This command keeps a simple database of the packages available for use by the current interpreter and how to load them into the interpreter.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="package files package",
                    arg_values={
                        0: (
                            _av(
                                "forget",
                                "Removes all information about each specified package from this interpreter, including information provided by both package ifneeded and package provide.",
                                "package forget ?package package ...?",
                            ),
                            _av(
                                "ifneeded",
                                "This command typically appears only in system configuration scripts to set up the package database.",
                                "package ifneeded package version ?script?",
                            ),
                            _av(
                                "names",
                                "Returns a list of the names of all packages in the interpreter for which a version has been provided (via package provide) or for which a package ifneeded script is available.",
                                "package names",
                            ),
                            _av(
                                "present",
                                "This command is equivalent to package require except that it does not try and load the package if it is not already loaded.",
                                "package present ?-exact? package ?requirement...?",
                            ),
                            _av(
                                "provide",
                                "This command is invoked to indicate that version version of package package is now present in the interpreter.",
                                "package provide package ?version?",
                            ),
                            _av(
                                "require",
                                "This command is typically invoked by Tcl code that wishes to use a particular version of a particular package.",
                                "package require package ?requirement...?",
                            ),
                            _av(
                                "unknown",
                                "This command supplies a command to invoke during package require if no suitable version of a package can be found in the package ifneeded database.",
                                "package unknown ?command?",
                            ),
                            _av(
                                "vcompare",
                                "Compares the two version numbers given by version1 and version2.",
                                "package vcompare version1 version2",
                            ),
                            _av(
                                "versions",
                                "Returns a list of all the version numbers of package for which information has been provided by package ifneeded commands.",
                                "package versions package",
                            ),
                            _av(
                                "vsatisfies",
                                "Returns 1 if the version satisfies at least one of the given requirements, and 0 otherwise.",
                                "package vsatisfies version requirement...",
                            ),
                            _av(
                                "files",
                                "Lists all files forming part of package.",
                                "package files package",
                            ),
                            _av(
                                "prefer",
                                "With no arguments, the commands returns either or whichever describes the current mode of selection logic used by package require.",
                                "package prefer ?latest|stable?",
                            ),
                        )
                    },
                ),
            ),
            subcommands={
                "forget": SubCommand(
                    name="forget",
                    arity=Arity(1),
                    detail="Removes all information about each specified package from this interpreter, including information provided by both package ifneeded and package provide.",
                    synopsis="package forget ?package package ...?",
                    return_type=TclType.STRING,
                ),
                "ifneeded": SubCommand(
                    name="ifneeded",
                    arity=Arity(2, 3),
                    detail="This command typically appears only in system configuration scripts to set up the package database.",
                    synopsis="package ifneeded package version ?script?",
                    return_type=TclType.STRING,
                    arg_roles={2: ArgRole.BODY},
                ),
                "names": SubCommand(
                    name="names",
                    arity=Arity(0, 0),
                    detail="Returns a list of the names of all packages in the interpreter for which a version has been provided (via package provide) or for which a package ifneeded script is available.",
                    synopsis="package names",
                    return_type=TclType.LIST,
                ),
                "present": SubCommand(
                    name="present",
                    arity=Arity(1),
                    detail="This command is equivalent to package require except that it does not try and load the package if it is not already loaded.",
                    synopsis="package present ?-exact? package ?requirement...?",
                    return_type=TclType.STRING,
                ),
                "provide": SubCommand(
                    name="provide",
                    arity=Arity(1, 2),
                    detail="This command is invoked to indicate that version version of package package is now present in the interpreter.",
                    synopsis="package provide package ?version?",
                    return_type=TclType.STRING,
                ),
                "require": SubCommand(
                    name="require",
                    arity=Arity(1),
                    detail="This command is typically invoked by Tcl code that wishes to use a particular version of a particular package.",
                    synopsis="package require package ?requirement...?",
                    return_type=TclType.STRING,
                ),
                "unknown": SubCommand(
                    name="unknown",
                    arity=Arity(0, 1),
                    detail="This command supplies a command to invoke during package require if no suitable version of a package can be found in the package ifneeded database.",
                    synopsis="package unknown ?command?",
                    return_type=TclType.STRING,
                ),
                "vcompare": SubCommand(
                    name="vcompare",
                    arity=Arity(2, 2),
                    detail="Compares the two version numbers given by version1 and version2.",
                    synopsis="package vcompare version1 version2",
                    return_type=TclType.INT,
                ),
                "versions": SubCommand(
                    name="versions",
                    arity=Arity(1, 1),
                    detail="Returns a list of all the version numbers of package for which information has been provided by package ifneeded commands.",
                    synopsis="package versions package",
                    return_type=TclType.LIST,
                ),
                "vsatisfies": SubCommand(
                    name="vsatisfies",
                    arity=Arity(2),
                    detail="Returns 1 if the version satisfies at least one of the given requirements, and 0 otherwise.",
                    synopsis="package vsatisfies version requirement...",
                    return_type=TclType.BOOLEAN,
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.INTERP_STATE,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
