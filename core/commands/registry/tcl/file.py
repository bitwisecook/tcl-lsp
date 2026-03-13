# Scaffolded from file.n -- refine and commit
"""file -- Manipulate file names and attributes."""

from __future__ import annotations

from ....compiler.side_effects import ConnectionSide, SideEffect, SideEffectTarget
from ....compiler.types import TclType
from .._base import CommandDef, make_av
from ..dialects import DIALECTS_EXCEPT_IRULES
from ..models import CommandSpec, FormKind, FormSpec, HoverSnippet, SubCommand, ValidationSpec
from ..signatures import ArgRole, Arity
from ._base import register

_SOURCE = "Tcl man page file.n"


_av = make_av(_SOURCE)


@register
class FileCommand(CommandDef):
    name = "file"

    @classmethod
    def spec(cls) -> CommandSpec:
        return CommandSpec(
            name="file",
            dialects=DIALECTS_EXCEPT_IRULES,
            hover=HoverSnippet(
                summary="Manipulate file names and attributes",
                synopsis=("file option name ?arg arg ...?",),
                snippet="This command provides several operations on a file's name or attributes.",
                source=_SOURCE,
            ),
            forms=(
                FormSpec(
                    kind=FormKind.DEFAULT,
                    synopsis="file option name ?arg arg ...?",
                    arg_values={
                        0: (
                            _av(
                                "atime",
                                "Returns a decimal string giving the time at which file name was last accessed.",
                                "file atime name ?time?",
                            ),
                            _av(
                                "attributes",
                                "file attributes name ?option?",
                                "file attributes name",
                            ),
                            _av(
                                "channels",
                                "If pattern is not specified, returns a list of names of all registered open channels in this interpreter.",
                                "file channels ?pattern?",
                            ),
                            _av(
                                "copy",
                                "file copy ?-force?",
                                "file copy ?-force? ?-|-? source target",
                            ),
                            _av(
                                "delete",
                                "Removes the file or directory specified by each pathname argument.",
                                "file delete ?-force? ?-|-? ?pathname ... ?",
                            ),
                            _av(
                                "dirname",
                                "Returns a name comprised of all of the path components in name excluding the last element.",
                                "file dirname name",
                            ),
                            _av(
                                "executable",
                                "Returns 1 if file name is executable by the current user, 0 otherwise.",
                                "file executable name",
                            ),
                            _av(
                                "exists",
                                "Returns 1 if file name exists and the current user has search privileges for the directories leading to it, 0 otherwise.",
                                "file exists name",
                            ),
                            _av(
                                "extension",
                                "Returns all of the characters in name after and including the last dot in the last element of name.",
                                "file extension name",
                            ),
                            _av(
                                "isdirectory",
                                "Returns 1 if file name is a directory, 0 otherwise.",
                                "file isdirectory name",
                            ),
                            _av(
                                "isfile",
                                "Returns 1 if file name is a regular file, 0 otherwise.",
                                "file isfile name",
                            ),
                            _av(
                                "join",
                                "Takes one or more file names and combines them, using the correct path separator for the current platform.",
                                "file join name ?name ...?",
                            ),
                            _av(
                                "link",
                                "If only one argument is given, that argument is assumed to be linkName, and this command returns the value of the link given by linkName (i.e.",
                                "file link ?-linktype? linkName ?target?",
                            ),
                            _av(
                                "lstat",
                                "Same as stat option (see below) except uses the lstat kernel call instead of stat.",
                                "file lstat name ?varName?",
                            ),
                            _av(
                                "mkdir", "Creates each directory specified.", "file mkdir ?dir ...?"
                            ),
                            _av(
                                "mtime",
                                "Returns a decimal string giving the time at which file name was last modified.",
                                "file mtime name ?time?",
                            ),
                            _av(
                                "nativename",
                                "Returns the platform-specific name of the file.",
                                "file nativename name",
                            ),
                            _av(
                                "normalize",
                                "Returns a unique normalized path representation for the file-system object (file, directory, link, etc), whose string value can be used as a unique identifier for it.",
                                "file normalize name",
                            ),
                            _av(
                                "owned",
                                "Returns 1 if file name is owned by the current user, 0 otherwise.",
                                "file owned name",
                            ),
                            _av(
                                "pathtype",
                                "Returns one of absolute, relative, volumerelative.",
                                "file pathtype name",
                            ),
                            _av(
                                "readable",
                                "Returns 1 if file name is readable by the current user, 0 otherwise.",
                                "file readable name",
                            ),
                            _av(
                                "readlink",
                                "Returns the value of the symbolic link given by name (i.e.",
                                "file readlink name",
                            ),
                            _av(
                                "rename",
                                "file rename ?-force?",
                                "file rename ?-force? ?-|-? source target",
                            ),
                            _av(
                                "rootname",
                                "Returns all of the characters in name up to but not including the last character in the last component of name.",
                                "file rootname name",
                            ),
                            _av(
                                "separator",
                                "If no argument is given, returns the character which is used to separate path segments for native files on this platform.",
                                "file separator ?name?",
                            ),
                            _av(
                                "size",
                                "Returns a decimal string giving the size of file name in bytes.",
                                "file size name",
                            ),
                            _av(
                                "split",
                                "Returns a list whose elements are the path components in name.",
                                "file split name",
                            ),
                            _av(
                                "stat",
                                "Invokes the stat kernel call on name, and returns a dictionary with the information returned from the kernel call.",
                                "file stat name ?varName?",
                            ),
                            _av(
                                "system",
                                "Returns a list of one or two elements, the first of which is the name of the filesystem to use for the file, and the second, if given, an arbitrary string representing the filesystem-specific nature or type of the locat…",
                                "file system name",
                            ),
                            _av(
                                "tail",
                                "Returns all of the characters in the last filesystem component of name.",
                                "file tail name",
                            ),
                            _av(
                                "tempfile",
                                "Creates a temporary file and returns a read-write channel opened on that file.",
                                "file tempfile ?nameVar? ?template?",
                            ),
                            _av(
                                "type",
                                "Returns a string giving the type of file name, which will be one of file, directory, characterSpecial, blockSpecial, fifo, link, or socket.",
                                "file type name",
                            ),
                            _av(
                                "volumes",
                                "Returns the absolute paths to the volumes mounted on the system, as a proper Tcl list.",
                                "file volumes",
                            ),
                            _av(
                                "writable",
                                "Returns 1 if file name is writable by the current user, 0 otherwise.",
                                "file writable name",
                            ),
                            _av(
                                "home",
                                "If no argument is specified, the command returns the home directory of the current user.",
                                "file home ?username?",
                            ),
                            _av(
                                "tempdir",
                                "Creates a temporary directory (guaranteed to be newly created and writable by the current script) and returns its name.",
                                "file tempdir ?template?",
                            ),
                            _av(
                                "tildeexpand",
                                "Returns the result of performing tilde substitution on name.",
                                "file tildeexpand name",
                            ),
                        )
                    },
                ),
            ),
            subcommands={
                "atime": SubCommand(
                    name="atime",
                    arity=Arity(1, 2),
                    detail="Returns a decimal string giving the time at which file name was last accessed.",
                    synopsis="file atime name ?time?",
                    return_type=TclType.INT,
                ),
                "attributes": SubCommand(
                    name="attributes",
                    arity=Arity(1),
                    detail="file attributes name ?option?",
                    synopsis="file attributes name",
                    return_type=TclType.LIST,
                ),
                "channels": SubCommand(
                    name="channels",
                    arity=Arity(0, 1),
                    detail="If pattern is not specified, returns a list of names of all registered open channels in this interpreter.",
                    synopsis="file channels ?pattern?",
                    return_type=TclType.LIST,
                ),
                "copy": SubCommand(
                    name="copy",
                    arity=Arity(2),
                    detail="file copy ?-force?",
                    synopsis="file copy ?-force? ?-|-? source target",
                    return_type=TclType.STRING,
                ),
                "delete": SubCommand(
                    name="delete",
                    arity=Arity(1),
                    detail="Removes the file or directory specified by each pathname argument.",
                    synopsis="file delete ?-force? ?-|-? ?pathname ... ?",
                    return_type=TclType.STRING,
                    destructive=True,
                ),
                "dirname": SubCommand(
                    name="dirname",
                    arity=Arity(1, 1),
                    detail="Returns a name comprised of all of the path components in name excluding the last element.",
                    synopsis="file dirname name",
                    return_type=TclType.STRING,
                ),
                "executable": SubCommand(
                    name="executable",
                    arity=Arity(1, 1),
                    detail="Returns 1 if file name is executable by the current user, 0 otherwise.",
                    synopsis="file executable name",
                    return_type=TclType.BOOLEAN,
                ),
                "exists": SubCommand(
                    name="exists",
                    arity=Arity(1, 1),
                    detail="Returns 1 if file name exists and the current user has search privileges for the directories leading to it, 0 otherwise.",
                    synopsis="file exists name",
                    return_type=TclType.BOOLEAN,
                ),
                "extension": SubCommand(
                    name="extension",
                    arity=Arity(1, 1),
                    detail="Returns all of the characters in name after and including the last dot in the last element of name.",
                    synopsis="file extension name",
                    return_type=TclType.STRING,
                ),
                "isdirectory": SubCommand(
                    name="isdirectory",
                    arity=Arity(1, 1),
                    detail="Returns 1 if file name is a directory, 0 otherwise.",
                    synopsis="file isdirectory name",
                    return_type=TclType.BOOLEAN,
                ),
                "isfile": SubCommand(
                    name="isfile",
                    arity=Arity(1, 1),
                    detail="Returns 1 if file name is a regular file, 0 otherwise.",
                    synopsis="file isfile name",
                    return_type=TclType.BOOLEAN,
                ),
                "join": SubCommand(
                    name="join",
                    arity=Arity(1),
                    detail="Takes one or more file names and combines them, using the correct path separator for the current platform.",
                    synopsis="file join name ?name ...?",
                    return_type=TclType.STRING,
                ),
                "link": SubCommand(
                    name="link",
                    arity=Arity(1),
                    return_type=TclType.STRING,
                    detail="If only one argument is given, that argument is assumed to be linkName, and this command returns the value of the link given by linkName (i.e.",
                    synopsis="file link ?-linktype? linkName ?target?",
                ),
                "lstat": SubCommand(
                    name="lstat",
                    arity=Arity(2, 2),
                    detail="Same as stat option (see below) except uses the lstat kernel call instead of stat.",
                    synopsis="file lstat name ?varName?",
                    return_type=TclType.STRING,
                    arg_roles={1: ArgRole.VAR_NAME},
                ),
                "mkdir": SubCommand(
                    name="mkdir",
                    arity=Arity(1),
                    detail="Creates each directory specified.",
                    synopsis="file mkdir ?dir ...?",
                    return_type=TclType.STRING,
                    destructive=True,
                ),
                "mtime": SubCommand(
                    name="mtime",
                    arity=Arity(1, 2),
                    detail="Returns a decimal string giving the time at which file name was last modified.",
                    synopsis="file mtime name ?time?",
                    return_type=TclType.INT,
                ),
                "nativename": SubCommand(
                    name="nativename",
                    arity=Arity(1, 1),
                    detail="Returns the platform-specific name of the file.",
                    synopsis="file nativename name",
                    return_type=TclType.STRING,
                ),
                "normalize": SubCommand(
                    name="normalize",
                    arity=Arity(1, 1),
                    detail="Returns a unique normalized path representation for the file-system object (file, directory, link, etc), whose string value can be used as a unique identifier for it.",
                    synopsis="file normalize name",
                    return_type=TclType.STRING,
                ),
                "owned": SubCommand(
                    name="owned",
                    arity=Arity(1, 1),
                    detail="Returns 1 if file name is owned by the current user, 0 otherwise.",
                    synopsis="file owned name",
                    return_type=TclType.BOOLEAN,
                ),
                "pathtype": SubCommand(
                    name="pathtype",
                    arity=Arity(1, 1),
                    detail="Returns one of absolute, relative, volumerelative.",
                    synopsis="file pathtype name",
                    return_type=TclType.STRING,
                ),
                "readable": SubCommand(
                    name="readable",
                    arity=Arity(1, 1),
                    detail="Returns 1 if file name is readable by the current user, 0 otherwise.",
                    synopsis="file readable name",
                    return_type=TclType.BOOLEAN,
                ),
                "readlink": SubCommand(
                    name="readlink",
                    arity=Arity(1, 1),
                    detail="Returns the value of the symbolic link given by name (i.e.",
                    synopsis="file readlink name",
                    return_type=TclType.STRING,
                ),
                "rename": SubCommand(
                    name="rename",
                    arity=Arity(2),
                    detail="file rename ?-force?",
                    synopsis="file rename ?-force? ?-|-? source target",
                    return_type=TclType.STRING,
                    destructive=True,
                ),
                "rootname": SubCommand(
                    name="rootname",
                    arity=Arity(1, 1),
                    detail="Returns all of the characters in name up to but not including the last character in the last component of name.",
                    synopsis="file rootname name",
                    return_type=TclType.STRING,
                ),
                "separator": SubCommand(
                    name="separator",
                    arity=Arity(0, 1),
                    detail="If no argument is given, returns the character which is used to separate path segments for native files on this platform.",
                    synopsis="file separator ?name?",
                    return_type=TclType.STRING,
                ),
                "size": SubCommand(
                    name="size",
                    arity=Arity(1, 1),
                    detail="Returns a decimal string giving the size of file name in bytes.",
                    synopsis="file size name",
                    return_type=TclType.INT,
                ),
                "split": SubCommand(
                    name="split",
                    arity=Arity(1, 1),
                    detail="Returns a list whose elements are the path components in name.",
                    synopsis="file split name",
                    return_type=TclType.LIST,
                ),
                "stat": SubCommand(
                    name="stat",
                    arity=Arity(2, 2),
                    detail="Invokes the stat kernel call on name, and returns a dictionary with the information returned from the kernel call.",
                    synopsis="file stat name ?varName?",
                    return_type=TclType.STRING,
                    arg_roles={1: ArgRole.VAR_NAME},
                ),
                "system": SubCommand(
                    name="system",
                    arity=Arity(1, 1),
                    detail="Returns a list of one or two elements, the first of which is the name of the filesystem to use for the file, and the second, if given, an arbitrary string representing the filesystem-specific nature or type of the locat…",
                    synopsis="file system name",
                    return_type=TclType.STRING,
                ),
                "tail": SubCommand(
                    name="tail",
                    arity=Arity(1, 1),
                    detail="Returns all of the characters in the last filesystem component of name.",
                    synopsis="file tail name",
                    return_type=TclType.STRING,
                ),
                "tempfile": SubCommand(
                    name="tempfile",
                    arity=Arity(0, 2),
                    detail="Creates a temporary file and returns a read-write channel opened on that file.",
                    synopsis="file tempfile ?nameVar? ?template?",
                    return_type=TclType.STRING,
                    arg_roles={0: ArgRole.VAR_NAME},
                ),
                "type": SubCommand(
                    name="type",
                    arity=Arity(1, 1),
                    detail="Returns a string giving the type of file name, which will be one of file, directory, characterSpecial, blockSpecial, fifo, link, or socket.",
                    synopsis="file type name",
                    return_type=TclType.STRING,
                ),
                "volumes": SubCommand(
                    name="volumes",
                    arity=Arity(0, 0),
                    detail="Returns the absolute paths to the volumes mounted on the system, as a proper Tcl list.",
                    synopsis="file volumes",
                    return_type=TclType.LIST,
                ),
                "writable": SubCommand(
                    name="writable",
                    arity=Arity(1, 1),
                    detail="Returns 1 if file name is writable by the current user, 0 otherwise.",
                    synopsis="file writable name",
                    return_type=TclType.BOOLEAN,
                ),
            },
            validation=ValidationSpec(
                arity=Arity(1),
            ),
            side_effect_hints=(
                SideEffect(
                    target=SideEffectTarget.FILE_IO,
                    reads=True,
                    writes=True,
                    connection_side=ConnectionSide.NONE,
                ),
            ),
        )
