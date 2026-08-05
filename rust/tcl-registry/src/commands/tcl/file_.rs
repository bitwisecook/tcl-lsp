// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! `file` — manipulate file names and attributes.

use crate::prelude::*;

const FORMS: &[FormSpec] = &[FormSpec {
    kind: FormKind::Default,
    synopsis: "file option name ?arg arg ...?",
    dialects: None,
}];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "atime",
        arity: Arity::new(1, 2),
        detail: "Returns a decimal string giving the time at which file name was last accessed; if time is given, sets the access time instead. On Windows FAT filesystems atime is unsupported; on zipfs it is mapped to the modification time.",
        synopsis: "file atime name ?time?",
        return_type: Some(TclType::Int),
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "attributes",
        arity: Arity::at_least(1),
        detail: "Query or set platform-specific file attributes: with no option, lists every attribute and its value; with an option only, returns that value; with option/value pairs, sets them. Available attributes are platform-specific (-permissions/-owner/-group on Unix, -archive/-hidden/-readonly/-system on Windows, -creator/-rsrclength on macOS); files inside a zipfs-mounted archive additionally expose read-only -archive/-compsize/-crc/-mount/-offset/-uncompsize attributes (Tcl 9.0+).",
        synopsis: "file attributes name\nfile attributes name option\nfile attributes name option value ?option value ...?",
        return_type: Some(TclType::List),
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "channels",
        arity: Arity::new(0, 1),
        detail: "Returns a list of names of all registered open channels in this interpreter, or only those matching pattern (using the same rules as string match) if given.",
        synopsis: "file channels ?pattern?",
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // Two forms in every version 8.4-9.1: `source target`, or one or
        // more `source`s copied into an existing `targetDir` (the second
        // form is selected automatically when target is a directory).
        name: "copy",
        arity: Arity::at_least(2),
        detail: "Copy a file or directory (recursively) to target, or one or more sources into an existing targetDir. Does not overwrite an existing destination unless -force is given, which also adjusts destination permissions if needed; overwriting a non-empty directory, a directory with a file, or a file with a directory is always an error. Symbolic links are copied as links, not their targets.",
        synopsis: "file copy ?-force? ?--? source target\nfile copy ?-force? ?--? source ?source ...? targetDir",
        return_type: Some(TclType::String),
        mutator: true,
        options: const {
            &[
                OptionSpec {
                    name: "-force",
                    value: OptionValue::flag(),
                    detail: "Overwrite an existing destination file, adjusting its permissions first if necessary.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "Marks the end of switches; the following word is treated as source even if it starts with -.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // TIP 323 (Tcl 8.6+) made the zero-argument form a legal no-op, so the
        // arity floor is 0 for the modern dialects this registry targets. The
        // tighter `>= 1` bound held only for 8.4/8.5, and the registry has no
        // dialect-split arity to express that exception (RUST_ISSUE_084).
        name: "delete",
        traits: Traits::FIRE_AND_FORGET_TEARDOWN,
        arity: Arity::at_least(0),
        detail: "Removes the file or directory specified by each pathname argument. A non-empty directory is removed only with -force. Operates on a symbolic link itself, never its target. A non-existent pathname is not an error; a read-only file is still deleted regardless of -force.",
        synopsis: "file delete ?-force? ?--? ?pathname ...?",
        return_type: Some(TclType::String),
        mutator: true,
        options: const {
            &[
                OptionSpec {
                    name: "-force",
                    value: OptionValue::flag(),
                    detail: "Remove a non-empty directory, adjusting permissions and relocating the process's cwd out of the path first if necessary.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "Marks the end of switches; the following word is treated as pathname even if it starts with -.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        destructive: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "dirname",
        arity: Arity::exact(1),
        detail: "Returns all of the path components in name excluding the last element.",
        synopsis: "file dirname name",
        pure: true,
        return_type: Some(TclType::String),
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "executable",
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is executable by the current user, 0 otherwise. On Windows, which has no executable attribute, every directory and any file with extension exe, com, cmd, or bat is treated as executable.",
        synopsis: "file executable name",
        return_type: Some(TclType::Boolean),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::exact(1),
        detail: "Returns 1 if file name exists and the current user has search privileges for the directories leading to it, 0 otherwise.",
        synopsis: "file exists name",
        return_type: Some(TclType::Boolean),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "extension",
        arity: Arity::exact(1),
        detail: "Returns all of the characters in name after and including the last dot.",
        synopsis: "file extension name",
        pure: true,
        return_type: Some(TclType::String),
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // New in Tcl 9.0 (TIP 543): absent from the 8.4/8.5/8.6 `file`
        // manpage subcommand list, present from the 9.0 manpage on.
        name: "home",
        arity: Arity::new(0, 1),
        detail: "Returns the home directory of the current user, or of username if given. Generally the value of $HOME (backslashes converted to forward slashes on Windows); a specific user's configured home directory may differ from $HOME even for the current user. Errors if $HOME is unset, or if username does not correspond to a real account.",
        synopsis: "file home ?username?",
        return_type: Some(TclType::String),
        dialects: Some(DialectSet::TCL90_PLUS),
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "isdirectory",
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is a directory, 0 otherwise.",
        synopsis: "file isdirectory name",
        return_type: Some(TclType::Boolean),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "isfile",
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is a regular file, 0 otherwise.",
        synopsis: "file isfile name",
        return_type: Some(TclType::Boolean),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "join",
        arity: Arity::at_least(1),
        detail: "Combines one or more file names using the correct path separator for the current platform.",
        synopsis: "file join name ?name ...?",
        pure: true,
        return_type: Some(TclType::String),
        // `[file join]` yields a portable (but not
        // canonicalised) path.
        taint_transform: Some(TaintColour::PATH_JOINED),
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // One argument (linkName) reads; two arguments (linkName target)
        // create a new link and error if linkName already exists or
        // target does not — present unchanged across 8.4-9.1.
        name: "link",
        arity: Arity::new(1, 2),
        detail: "With one argument, returns the target of the symbolic link linkName (an error if it is not a readable link). With two arguments, creates a new link linkName pointing at the existing target and returns target; errors if linkName already exists or target does not. -linktype (-symbolic or -hard) forces a specific link type when creating, otherwise the platform default is used.",
        synopsis: "file link ?-linktype? linkName ?target?",
        return_type: Some(TclType::String),
        mutator: true,
        options: const {
            &[
                OptionSpec {
                    name: "-symbolic",
                    value: OptionValue::flag(),
                    detail: "Create (or require) a symbolic link.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "-hard",
                    value: OptionValue::flag(),
                    detail: "Create (or require) a hard link; files only.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // varName became optional in Tcl 9.0 (manpage synopsis reads
        // `file lstat name varName` — required — through 8.4/8.5/8.6, and
        // `file lstat name ?varName?` from 9.0 onward, where omitting it
        // returns a dict instead of populating the array and returning "").
        // The registry has no dialect-split arity (RUST_ISSUE_084-style
        // gap): the floor of 1 below is only correct for 9.0+; 8.4-8.6
        // require exactly 2 args (name and varName).
        name: "lstat",
        arity: Arity::new(1, 2),
        detail: "Same as stat except uses the lstat kernel call instead of stat, so a symbolic link name reports information about the link itself rather than its target. varName is optional from Tcl 9.0 on; omitting it returns a dict of the stat fields instead of populating the array and returning an empty string (mandatory in 8.4-8.6).",
        synopsis: "file lstat name ?varName?",
        return_type: Some(TclType::String),
        arg_roles: &[(1, ArgRole::VarWrite)],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::FileIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::None,
                dialects: None,
            },
            SideEffect {
                target: SideEffectTarget::Variable,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::None,
                dialects: None,
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // TIP 323 (Tcl 8.6+): the zero-argument form is a legal no-op; the
        // `>= 1` bound held only for 8.4/8.5 (RUST_ISSUE_084).
        name: "mkdir",
        arity: Arity::at_least(0),
        detail: "Creates each directory specified, including any non-existing parent directories. An already-existing directory is not an error. Halts at the first error, if any, leaving earlier directories (processed in the order given) created.",
        synopsis: "file mkdir ?dir ...?",
        return_type: Some(TclType::String),
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        destructive: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mtime",
        arity: Arity::new(1, 2),
        detail: "Returns a decimal string giving the time at which file name was last modified; if time is given, sets the modification time instead (equivalent to Unix touch). On zipfs filesystems the modification time cannot be explicitly set.",
        synopsis: "file mtime name ?time?",
        return_type: Some(TclType::Int),
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "nativename",
        arity: Arity::exact(1),
        detail: "Returns the platform-specific name of the file.",
        synopsis: "file nativename name",
        pure: true,
        return_type: Some(TclType::String),
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "normalize",
        arity: Arity::exact(1),
        detail: "Returns a unique normalized absolute path for the file-system object: ../ and ./ removed, in the platform's standard form. Symbolic links along the path are resolved, except possibly the final component, so the link itself (rather than what it points to) can still be operated on.",
        synopsis: "file normalize name",
        return_type: Some(TclType::String),
        // `[file normalize]` canonicalises the path
        // (traversal-safe).
        taint_transform: Some(TaintColour::PATH_NORMALISED),
        returns_path: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "owned",
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is owned by the current user, 0 otherwise.",
        synopsis: "file owned name",
        return_type: Some(TclType::Boolean),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pathtype",
        arity: Arity::exact(1),
        detail: "Returns one of absolute, relative, volumerelative.",
        synopsis: "file pathtype name",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "readable",
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is readable by the current user, 0 otherwise.",
        synopsis: "file readable name",
        return_type: Some(TclType::Boolean),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "readlink",
        arity: Arity::exact(1),
        detail: "Returns the value of the symbolic link given by name; errors if name is not a readable symbolic link. Undefined on systems without symbolic-link support.",
        synopsis: "file readlink name",
        return_type: Some(TclType::String),
        returns_path: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // Two forms in every version 8.4-9.1: `source target`, or one or
        // more `source`s moved into an existing `targetDir`.
        name: "rename",
        dialects: None,
        arity: Arity::at_least(2),
        detail: "Rename or move a file or directory source to target, or one or more sources into an existing targetDir. Does not overwrite an existing destination unless -force is given; overwriting a non-empty directory, a directory with a file, or a file with a directory is always an error. Renames a symbolic link itself, never its target. There is no guarantee that metadata such as attributes or access control lists survive the rename.",
        synopsis: "file rename ?-force? ?--? source target\nfile rename ?-force? ?--? source ?source ...? targetDir",
        return_type: Some(TclType::String),
        mutator: true,
        options: const {
            &[
                OptionSpec {
                    name: "-force",
                    value: OptionValue::flag(),
                    detail: "Overwrite an existing destination file.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "Marks the end of switches; the following word is treated as source even if it starts with -.",
                    dialects: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        destructive: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "rootname",
        arity: Arity::exact(1),
        detail: "Returns all characters in name up to but not including the last dot in the last component.",
        synopsis: "file rootname name",
        pure: true,
        return_type: Some(TclType::String),
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "separator",
        arity: Arity::new(0, 1),
        detail: "Returns the character used to separate path segments for native files on this platform.",
        synopsis: "file separator ?name?",
        pure: true,
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "size",
        arity: Arity::exact(1),
        detail: "Returns a decimal string giving the size of file name in bytes.",
        synopsis: "file size name",
        return_type: Some(TclType::Int),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "split",
        arity: Arity::exact(1),
        detail: "Returns a list whose elements are the path components in name.",
        synopsis: "file split name",
        pure: true,
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // varName became optional in Tcl 9.0 (manpage synopsis is
        // `file stat name varName` — required — through 8.4/8.5/8.6, and
        // becomes `file stat name ?varName?` from 9.0 onward): 9.0+
        // returns a dict of the stat fields when varName is omitted, but
        // still returns the empty string and populates the array when it
        // is given, in every version. The registry has no dialect-split
        // arity (RUST_ISSUE_084-style gap): the floor of 1 below is only
        // correct for 9.0+; 8.4-8.6 require exactly 2 args (name and
        // varName).
        name: "stat",
        arity: Arity::new(1, 2),
        detail: "Invokes the stat kernel call on name. If varName is given, populates it as an array with atime/ctime/dev/gid/ino/mode/mtime/nlink/size/type/uid elements and returns an empty string. From Tcl 9.0, varName may be omitted, in which case the same fields are returned directly as a dict (mandatory in 8.4-8.6, where the array form is the only form).",
        synopsis: "file stat name ?varName?",
        return_type: Some(TclType::String),
        arg_roles: &[(1, ArgRole::VarWrite)],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::FileIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::None,
                dialects: None,
            },
            SideEffect {
                target: SideEffectTarget::Variable,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::None,
                dialects: None,
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "system",
        arity: Arity::exact(1),
        detail: "Returns a one- or two-element list: the name of the filesystem responsible for name (e.g. native), and if given, a filesystem-specific type string (e.g. NTFS, FAT on Windows, or vfs ftp for a mounted virtual filesystem). Errors if the file belongs to no filesystem.",
        synopsis: "file system name",
        return_type: Some(TclType::List),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tail",
        arity: Arity::exact(1),
        detail: "Returns all of the characters in the last filesystem component of name.",
        synopsis: "file tail name",
        pure: true,
        return_type: Some(TclType::String),
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // New in Tcl 9.0: absent from the 8.4/8.5/8.6 `file` manpage
        // subcommand list (which has `tempfile` from 8.6 but no
        // `tempdir`), present from the 9.0 manpage on.
        name: "tempdir",
        arity: Arity::new(0, 1),
        detail: "Creates a new, writable temporary directory and returns its name. template may give the containing directory (the part before the last directory separator) and/or a base-name prefix (the part after); defaults to a system-determined directory and the prefix \"tcl\".",
        synopsis: "file tempdir ?template?",
        return_type: Some(TclType::String),
        mutator: true,
        dialects: Some(DialectSet::TCL90_PLUS),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tempfile",
        traits: Traits::OPENS_CHANNEL,
        arity: Arity::new(0, 2),
        detail: "Creates a temporary file and returns a read-write channel opened on that file. If nameVar is given, the file's name is written there; otherwise Tcl arranges to delete the file once it is no longer needed. template may suggest a directory, base name, or extension, though some platforms ignore parts of it. Temporary files are only ever created on the native filesystem.",
        synopsis: "file tempfile ?nameVar? ?template?",
        return_type: Some(TclType::Channel),
        mutator: true,
        arg_roles: &[(0, ArgRole::VarWrite)],
        dialects: Some(DialectSet::TCL86_PLUS),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // New in Tcl 9.0: absent from the 8.4/8.5/8.6 `file` manpage
        // subcommand list, present from the 9.0 manpage on. Unlike
        // `[subst]`'s tilde handling, this is name-only, purely lexical
        // tilde substitution -- not gated on filesystem access.
        name: "tildeexpand",
        arity: Arity::exact(1),
        detail: "Returns the result of performing tilde substitution on name: a leading ~ followed immediately by a separator substitutes $HOME, and ~user substitutes that user's home directory. Returns name unmodified if it does not begin with a tilde. Errors if $HOME is unset or the named user does not exist.",
        synopsis: "file tildeexpand name",
        return_type: Some(TclType::String),
        dialects: Some(DialectSet::TCL90_PLUS),
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "type",
        arity: Arity::exact(1),
        detail: "Returns a string giving the type of file name: one of file, directory, characterSpecial, blockSpecial, fifo, link, or socket.",
        synopsis: "file type name",
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "volumes",
        arity: Arity::exact(0),
        detail: "Returns the absolute paths to the volumes mounted on the system, as a Tcl list. On Unix this is typically just \"/\" (or additionally a zipfs root); on Windows it is the available local drive letters. Any volumes mounted by a virtual filesystem are included too.",
        synopsis: "file volumes",
        return_type: Some(TclType::List),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "writable",
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is writable by the current user, 0 otherwise.",
        synopsis: "file writable name",
        return_type: Some(TclType::Boolean),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: false,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `file`.
///
/// `dialects` is `ALL_TCL` because `file` is unavailable in F5 iRules:
/// `"file"` is one of the K36322151 bans (the F5 TMM sandbox). `ALL_TCL`
/// does not carry the `IRULES` bit, so this spec never intersects the
/// bare `IRULES` availability mask — the exclusion is expressed directly
/// by the `dialects` group, with no disable list involved. No other
/// dialect here (Tk, Expect, tmsh, iApps, the EDA vendor shells) disables
/// or restricts `file`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "file",
        dialects: Some(DialectSet::ALL_TCL),
        traits: Traits::BYTE_COMPILED
            | Traits::HAS_DESTRUCTIVE_OPS
            | Traits::RETURNS_PATH
            | Traits::SAFE_INTERP_HIDDEN,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
            dialects: None,
        }],
        hover: Some(HoverSnippet {
            summary: "Manipulate file names and attributes",
            synopsis: &["file option name ?arg arg ...?"],
            snippet: "Dispatches on option to one of many operations on a file's name (dirname/tail/extension/join/split/normalize, all pure string or path-table operations) or its on-disk attributes (exists/isdirectory/isfile/readable/writable/executable/owned, atime/mtime/size/type/stat, copy/rename/delete/mkdir, link/readlink, attributes). name undergoes tilde substitution if it starts with ~. Any unique abbreviation of option is accepted, matching Tcl's ensemble-style dispatch.",
            source: "Tcl file(n)",
            examples: "file exists $path\nfile mkdir [file dirname $path]\nfile copy -force $src $dst\nset ext [file extension $path]\nfile stat $path st\nputs $st(size)",
            return_value: "Depends on option: a boolean (0/1), a path string, an integer, a list, a dict, or an empty string.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use crate::registry::CommandRegistry;

    #[test]
    fn file_delete_and_mkdir_allow_zero_args() {
        // RUST_ISSUE_084: TIP 323 (Tcl 8.6+) made `file delete` / `file mkdir`
        // with no pathname a legal no-op — the arity floor must be 0, not 1, so
        // a plain `file delete` draws no false wrong-#-args on 8.6/9.x.
        let reg = CommandRegistry::build_default();
        let file = reg.get("file").expect("file command");
        for sub in ["delete", "mkdir"] {
            let s = file.subcommand(sub).unwrap_or_else(|| panic!("file {sub}"));
            assert_eq!(s.arity.min, 0, "`file {sub}` arity floor must be 0");
        }
    }
}
