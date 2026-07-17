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
}];

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "atime",
        arity: Arity::new(1, 2),
        detail: "Returns a decimal string giving the time at which file name was last accessed.",
        synopsis: "file atime name ?time?",
        return_type: Some(TclType::Int),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "attributes",
        arity: Arity::at_least(1),
        detail: "Query or set file attributes.",
        synopsis: "file attributes name ?option? ?value ...?",
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "channels",
        arity: Arity::new(0, 1),
        detail: "Returns a list of names of all registered open channels in this interpreter.",
        synopsis: "file channels ?pattern?",
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "copy",
        arity: Arity::at_least(2),
        detail: "Copy files or directories.",
        synopsis: "file copy ?-force? ?--? source target",
        return_type: Some(TclType::String),
        mutator: true,
        options: const {
            &[
                OptionSpec {
                    name: "-force",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
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
        detail: "Removes the file or directory specified by each pathname argument.",
        synopsis: "file delete ?-force? ?--? ?pathname ...?",
        return_type: Some(TclType::String),
        mutator: true,
        options: const {
            &[
                OptionSpec {
                    name: "-force",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
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
        detail: "Returns 1 if file name is executable by the current user, 0 otherwise.",
        synopsis: "file executable name",
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        arity: Arity::exact(1),
        detail: "Returns 1 if file name exists and the current user has search privileges for the directories leading to it, 0 otherwise.",
        synopsis: "file exists name",
        return_type: Some(TclType::Boolean),
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
        name: "home",
        arity: Arity::new(0, 1),
        detail: "Returns the home directory of the current user.",
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
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "isfile",
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is a regular file, 0 otherwise.",
        synopsis: "file isfile name",
        return_type: Some(TclType::Boolean),
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
        name: "link",
        arity: Arity::new(1, 2),
        detail: "Returns the value of the link given by linkName, or creates a link.",
        synopsis: "file link ?-linktype? linkName ?target?",
        return_type: Some(TclType::String),
        options: const {
            &[
                OptionSpec {
                    name: "-symbolic",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "-hard",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "lstat",
        arity: Arity::new(1, 2),
        detail: "Same as stat except uses the lstat kernel call instead of stat.",
        synopsis: "file lstat name ?varName?",
        return_type: Some(TclType::String),
        arg_roles: &[(1, ArgRole::VarWrite)],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::FileIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::None,
            },
            SideEffect {
                target: SideEffectTarget::Variable,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::None,
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // TIP 323 (Tcl 8.6+): the zero-argument form is a legal no-op; the
        // `>= 1` bound held only for 8.4/8.5 (RUST_ISSUE_084).
        name: "mkdir",
        arity: Arity::at_least(0),
        detail: "Creates each directory specified.",
        synopsis: "file mkdir ?dir ...?",
        return_type: Some(TclType::String),
        mutator: true,
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        destructive: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mtime",
        arity: Arity::new(1, 2),
        detail: "Returns a decimal string giving the time at which file name was last modified.",
        synopsis: "file mtime name ?time?",
        return_type: Some(TclType::Int),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
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
        detail: "Returns a unique normalized path representation for the file-system object.",
        synopsis: "file normalize name",
        return_type: Some(TclType::String),
        // `[file normalize]` canonicalises the path
        // (traversal-safe).
        taint_transform: Some(TaintColour::PATH_NORMALISED),
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "owned",
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is owned by the current user, 0 otherwise.",
        synopsis: "file owned name",
        return_type: Some(TclType::Boolean),
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
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "readlink",
        arity: Arity::exact(1),
        detail: "Returns the value of the symbolic link given by name.",
        synopsis: "file readlink name",
        return_type: Some(TclType::String),
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "rename",
        dialects: None,
        arity: Arity::at_least(2),
        detail: "Rename or move files/directories.",
        synopsis: "file rename ?-force? ?--? source target",
        return_type: Some(TclType::String),
        mutator: true,
        options: const {
            &[
                OptionSpec {
                    name: "-force",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "",
                    dialects: None,
                    aliases: &[],
                    min_version: None,
                },
            ]
        },
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
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
        name: "stat",
        arity: Arity::new(1, 2),
        detail: "Invokes the stat kernel call on name and returns the information.",
        synopsis: "file stat name ?varName?",
        return_type: Some(TclType::String),
        arg_roles: &[(1, ArgRole::VarWrite)],
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::FileIo,
                reads: true,
                writes: false,
                connection_side: ConnectionSide::None,
            },
            SideEffect {
                target: SideEffectTarget::Variable,
                reads: false,
                writes: true,
                connection_side: ConnectionSide::None,
            },
        ],
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "system",
        arity: Arity::exact(1),
        detail: "Returns a list describing the filesystem type for the given file.",
        synopsis: "file system name",
        return_type: Some(TclType::List),
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
        name: "tempdir",
        arity: Arity::new(0, 1),
        detail: "Creates a temporary directory and returns its name.",
        synopsis: "file tempdir ?template?",
        return_type: Some(TclType::String),
        mutator: true,
        dialects: Some(DialectSet::TCL90_PLUS),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: false,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tempfile",
        arity: Arity::new(0, 2),
        detail: "Creates a temporary file and returns a read-write channel opened on that file.",
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
        }],
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tildeexpand",
        arity: Arity::exact(1),
        detail: "Returns the result of performing tilde substitution on name.",
        synopsis: "file tildeexpand name",
        return_type: Some(TclType::String),
        dialects: Some(DialectSet::TCL90_PLUS),
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "type",
        arity: Arity::exact(1),
        detail: "Returns a string giving the type of file name.",
        synopsis: "file type name",
        return_type: Some(TclType::String),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "volumes",
        arity: Arity::exact(0),
        detail: "Returns the absolute paths to the volumes mounted on the system.",
        synopsis: "file volumes",
        return_type: Some(TclType::List),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "writable",
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is writable by the current user, 0 otherwise.",
        synopsis: "file writable name",
        return_type: Some(TclType::Boolean),
        ..SubCommand::DEFAULT
    },
];

/// Command spec for `file`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "file",
        dialects: None,
        traits: Traits::BYTE_COMPILED | Traits::HAS_DESTRUCTIVE_OPS | Traits::RETURNS_PATH | Traits::SAFE_INTERP_HIDDEN,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            connection_side: ConnectionSide::None,
        }],
        hover: Some(HoverSnippet {
            summary: "Manipulate file names and attributes",
            synopsis: &["file option name ?arg arg ...?"],
            snippet: "This command provides several operations on a file's name or attributes.",
            source: "Tcl man page file.n",
            examples: "",
            return_value: "",
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
