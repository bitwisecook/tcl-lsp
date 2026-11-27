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
use tcl_dialect::model::SpecSurface;

// The native file ensemble delegates TclCompileBasic*ArgCmd to its private
// worker with the original ensemble rewrite. C8.4/C8.5 use a monolithic
// handler; C8.6+ materialise ::tcl::file workers. Compiler arity is independent
// of the public handler's option grammar.
macro_rules! named_file_compiler {
    ($member:literal, no_hook) => {
        crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::WithImplementationPath {
                compiler: &crate::native_compilation::NativeCompilationSpec {
                    grammar: crate::native_compilation::NativeCompilationGrammar::NoHook,
                    operation: crate::SemanticOperationId::Invoke,
                    body: crate::native_compilation::NativeBodyCompilation::Inherit,
                },
                lookups: &[
                    crate::native_compilation::NativeCompilerImplementationLookup {
                        ensemble: "::file",
                        member: $member,
                        slot: concat!("::tcl::file::", $member),
                        command: "file",
                        prepended: &[$member],
                    },
                ],
                implementation_from: tcl_dialect::TclVersion::V8_6,
                monolithic_no_hook_before: false,
            },
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }
    };
    ($member:literal, $arity:expr, $first:expr $(,)?) => {
        crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::NamedEnsembleInvocation {
                lookup: &crate::native_compilation::NativeCompilerImplementationLookup {
                    ensemble: "::file",
                    member: $member,
                    slot: concat!("::tcl::file::", $member),
                    command: "file",
                    prepended: &[$member],
                },
                implementation_from: $first,
                hook_from: $first,
                arity: $arity,
                later_arity: None,
            },
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }
    };
}

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "file option name ?arg arg ...?",
    ..FormSpec::DEFAULT
}];

fn filesystem_changed(arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    if !arguments.is_empty() {
        transitions.push(StateTransition::Package(
            crate::model::binding::PackageTransition::DiscoveryDependencyChanged {
                dependency: crate::model::binding::PackageResolverDependency::Filesystem,
            },
        ));
    }
    transitions
}

fn filesystem_attributes_changed(arguments: InvocationArguments<'_>) -> StateTransitions {
    if arguments.len() >= 3 {
        filesystem_changed(arguments)
    } else {
        StateTransitions::default()
    }
}

fn filesystem_time_changed(arguments: InvocationArguments<'_>) -> StateTransitions {
    if arguments.len() == 2 {
        filesystem_changed(arguments)
    } else {
        StateTransitions::default()
    }
}

static SUBCOMMANDS: &[SubCommand] = &[
    SubCommand {
        name: "atime",
        native_compilation: Some(named_file_compiler!(
            "atime",
            Arity::new(1, 2),
            tcl_dialect::TclVersion::V8_6,
        )),
        state_transitions: crate::knowledge::Knowledge::Partial(StateTransitionDescriptor {
            logical_source_resolver: None,
            commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
            success_resolver: None,
            resolver: Some(filesystem_time_changed),
            ..StateTransitionDescriptor::EMPTY
        }),
        arity: Arity::new(1, 2),
        detail: "Returns a decimal string giving the time at which file name was last accessed; if time is given, sets the access time instead. On Windows FAT filesystems atime is unsupported; on zipfs it is mapped to the modification time.",
        synopsis: "file atime name ?time?",
        return_type: Some(TclType::Int),
        mutator: true,
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "attributes",
        native_compilation: Some(named_file_compiler!("attributes", no_hook)),
        state_transitions: crate::knowledge::Knowledge::Partial(StateTransitionDescriptor {
            logical_source_resolver: None,
            commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
            success_resolver: None,
            resolver: Some(filesystem_attributes_changed),
            ..StateTransitionDescriptor::EMPTY
        }),
        arity: Arity::at_least(1),
        detail: "Query or set platform-specific file attributes: with no option, lists every attribute and its value; with an option only, returns that value; with option/value pairs, sets them. Available attributes are platform-specific (-permissions/-owner/-group on Unix, -archive/-hidden/-readonly/-system on Windows, -creator/-rsrclength on macOS); files inside a zipfs-mounted archive additionally expose read-only -archive/-compsize/-crc/-mount/-offset/-uncompsize attributes (Tcl 9.0+).",
        synopsis: "file attributes name\nfile attributes name option\nfile attributes name option value ?option value ...?",
        return_type: Some(TclType::List),
        mutator: true,
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "channels",
        native_compilation: Some(named_file_compiler!(
            "channels",
            Arity::new(0, 1),
            tcl_dialect::TclVersion::V8_6,
        )),
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
        native_compilation: Some(named_file_compiler!("copy", no_hook)),
        state_transitions: crate::knowledge::Knowledge::Partial(StateTransitionDescriptor {
            logical_source_resolver: None,
            commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
            success_resolver: None,
            resolver: Some(filesystem_changed),
            ..StateTransitionDescriptor::EMPTY
        }),
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
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                    effect: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "Marks the end of switches; the following word is treated as source even if it starts with -.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                    effect: None,
                },
            ]
        },
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // TIP 323 (Tcl 8.6+) made the zero-argument form a legal no-op, so the
        // arity floor is 0 for the modern dialects this registry targets. The
        // tighter `>= 1` bound held only for 8.4/8.5, and the registry has no
        // dialect-split arity to express that exception.
        name: "delete",
        native_compilation: Some(named_file_compiler!(
            "delete",
            Arity::at_least(0),
            tcl_dialect::TclVersion::V8_6,
        )),
        state_transitions: crate::knowledge::Knowledge::Partial(StateTransitionDescriptor {
            logical_source_resolver: None,
            commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
            success_resolver: None,
            resolver: Some(filesystem_changed),
            ..StateTransitionDescriptor::EMPTY
        }),
        traits: Traits::FIRE_AND_FORGET_TEARDOWN,
        arity: Arity::at_least(0),
        arity_windows: const {
            &[crate::arity::ArityWindow {
                lifecycle: crate::lifecycle::Lifecycle::UNSPECIFIED.retired_from("8.6"),
                arity: Arity::at_least(1),
            }]
        },
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
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                    effect: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "Marks the end of switches; the following word is treated as pathname even if it starts with -.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                    effect: None,
                },
            ]
        },
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        destructive: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "dirname",
        source_path_operation: Some(SourcePathOperation::Dirname),
        native_compilation: Some(named_file_compiler!(
            "dirname",
            Arity::exact(1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(1),
        detail: "Returns all of the path components in name excluding the last element.",
        synopsis: "file dirname name",
        return_type: Some(TclType::String),
        returns_path: true,
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::path::FILE_DIRNAME),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "executable",
        native_compilation: Some(named_file_compiler!(
            "executable",
            Arity::exact(1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is executable by the current user, 0 otherwise. On Windows, which has no executable attribute, every directory and any file with extension exe, com, cmd, or bat is treated as executable.",
        synopsis: "file executable name",
        return_type: Some(TclType::Boolean),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "exists",
        native_compilation: Some(named_file_compiler!(
            "exists",
            Arity::exact(1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(1),
        detail: "Returns 1 if file name exists and the current user has search privileges for the directories leading to it, 0 otherwise.",
        synopsis: "file exists name",
        return_type: Some(TclType::Boolean),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "extension",
        native_compilation: Some(named_file_compiler!(
            "extension",
            Arity::exact(1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(1),
        detail: "Returns all of the characters in name after and including the last dot.",
        synopsis: "file extension name",
        return_type: Some(TclType::String),
        returns_path: true,
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::path::FILE_EXTENSION),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // New in Tcl 9.0 (TIP 543): absent from the 8.4/8.5/8.6 `file`
        // manpage subcommand list, present from the 9.0 manpage on.
        name: "home",
        native_compilation: Some(named_file_compiler!(
            "home",
            Arity::new(0, 1),
            tcl_dialect::TclVersion::V9_0,
        )),
        arity: Arity::new(0, 1),
        detail: "Returns the home directory of the current user, or of username if given. Generally the value of $HOME (backslashes converted to forward slashes on Windows); a specific user's configured home directory may differ from $HOME even for the current user. Errors if $HOME is unset, or if username does not correspond to a real account.",
        synopsis: "file home ?username?",
        return_type: Some(TclType::String),
        surface: Some(SpecSurface::TCL90_PLUS),
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "isdirectory",
        native_compilation: Some(named_file_compiler!(
            "isdirectory",
            Arity::exact(1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is a directory, 0 otherwise.",
        synopsis: "file isdirectory name",
        return_type: Some(TclType::Boolean),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "isfile",
        native_compilation: Some(named_file_compiler!(
            "isfile",
            Arity::exact(1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is a regular file, 0 otherwise.",
        synopsis: "file isfile name",
        return_type: Some(TclType::Boolean),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "join",
        source_path_operation: Some(SourcePathOperation::Join),
        native_compilation: Some(named_file_compiler!(
            "join",
            Arity::at_least(1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::at_least(1),
        detail: "Combines one or more file names using the correct path separator for the current platform.",
        synopsis: "file join name ?name ...?",
        return_type: Some(TclType::String),
        // `[file join]` yields a portable (but not
        // canonicalised) path.
        taint_transform: Some(TaintColour::PATH_JOINED),
        returns_path: true,
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::path::FILE_JOIN),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // One argument (linkName) reads; two arguments (linkName target)
        // create a new link and error if linkName already exists or
        // target does not — present unchanged across 8.4-9.1.
        name: "link",
        native_compilation: Some(named_file_compiler!(
            "link",
            Arity::new(1, 3),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::new(1, 2).with_positionals(),
        reserved_trailing_words: 1,
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
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                    effect: None,
                },
                OptionSpec {
                    name: "-hard",
                    value: OptionValue::flag(),
                    detail: "Create (or require) a hard link; files only.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                    effect: None,
                },
            ]
        },
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // varName became optional in Tcl 9.0 (manpage synopsis reads
        // `file lstat name varName` — required — through 8.4/8.5/8.6, and
        // `file lstat name ?varName?` from 9.0 onward, where omitting it
        // returns a dict instead of populating the array and returning "").
        // The registry has no dialect-split arity (a known
        // gap): the floor of 1 below is only correct for 9.0+; 8.4-8.6
        // require exactly 2 args (name and varName).
        name: "lstat",
        native_compilation: Some(named_file_compiler!(
            "lstat",
            Arity::exact(2),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::new(1, 2),
        arity_windows: const {
            &[crate::arity::ArityWindow {
                lifecycle: crate::lifecycle::Lifecycle::UNSPECIFIED.retired_from("9.0"),
                arity: Arity::exact(2),
            }]
        },
        detail: "Same as stat except uses the lstat kernel call instead of stat, so a symbolic link name reports information about the link itself rather than its target. varName is optional from Tcl 9.0 on; omitting it returns a dict of the stat fields instead of populating the array and returning an empty string (mandatory in 8.4-8.6).",
        synopsis: "file lstat name ?varName?",
        return_type: Some(TclType::String),
        arg_roles: &[(1, ArgRole::VarWrite)],
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::FILE_LSTAT),
        side_effects: crate::knowledge::Knowledge::Partial(&[
            SideEffect {
                target: SideEffectTarget::FileIo,
                reads: true,
                ..SideEffect::DEFAULT
            },
            SideEffect {
                target: SideEffectTarget::Variable,
                writes: true,
                ..SideEffect::DEFAULT
            },
        ]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // TIP 323 (Tcl 8.6+): the zero-argument form is a legal no-op; the
        // `>= 1` bound held only for 8.4/8.5.
        name: "mkdir",
        native_compilation: Some(named_file_compiler!(
            "mkdir",
            Arity::at_least(0),
            tcl_dialect::TclVersion::V8_6,
        )),
        state_transitions: crate::knowledge::Knowledge::Partial(StateTransitionDescriptor {
            logical_source_resolver: None,
            commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
            success_resolver: None,
            resolver: Some(filesystem_changed),
            ..StateTransitionDescriptor::EMPTY
        }),
        arity: Arity::at_least(0),
        arity_windows: const {
            &[crate::arity::ArityWindow {
                lifecycle: crate::lifecycle::Lifecycle::UNSPECIFIED.retired_from("8.6"),
                arity: Arity::at_least(1),
            }]
        },
        detail: "Creates each directory specified, including any non-existing parent directories. An already-existing directory is not an error. Halts at the first error, if any, leaving earlier directories (processed in the order given) created.",
        synopsis: "file mkdir ?dir ...?",
        return_type: Some(TclType::String),
        mutator: true,
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        destructive: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "mtime",
        native_compilation: Some(named_file_compiler!(
            "mtime",
            Arity::new(1, 2),
            tcl_dialect::TclVersion::V8_6,
        )),
        state_transitions: crate::knowledge::Knowledge::Partial(StateTransitionDescriptor {
            logical_source_resolver: None,
            commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
            success_resolver: None,
            resolver: Some(filesystem_time_changed),
            ..StateTransitionDescriptor::EMPTY
        }),
        arity: Arity::new(1, 2),
        detail: "Returns a decimal string giving the time at which file name was last modified; if time is given, sets the modification time instead (equivalent to Unix touch). On zipfs filesystems the modification time cannot be explicitly set.",
        synopsis: "file mtime name ?time?",
        return_type: Some(TclType::Int),
        mutator: true,
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "nativename",
        native_compilation: Some(named_file_compiler!(
            "nativename",
            Arity::exact(1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(1),
        detail: "Returns the platform-specific name of the file.",
        synopsis: "file nativename name",
        return_type: Some(TclType::String),
        returns_path: true,
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::PLATFORM_DECIDED,
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "normalize",
        source_path_operation: Some(SourcePathOperation::Normalize),
        native_compilation: Some(named_file_compiler!(
            "normalize",
            Arity::exact(1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(1),
        detail: "Returns a unique normalized absolute path for the file-system object: ../ and ./ removed, in the platform's standard form. Symbolic links along the path are resolved, except possibly the final component, so the link itself (rather than what it points to) can still be operated on.",
        synopsis: "file normalize name",
        return_type: Some(TclType::String),
        // `[file normalize]` canonicalises the path
        // (traversal-safe).
        taint_transform: Some(TaintColour::PATH_NORMALISED),
        returns_path: true,
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        // The host decides: its working directory and its links.
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::PLATFORM_DECIDED,
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "owned",
        native_compilation: Some(named_file_compiler!(
            "owned",
            Arity::exact(1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is owned by the current user, 0 otherwise.",
        synopsis: "file owned name",
        return_type: Some(TclType::Boolean),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "pathtype",
        native_compilation: Some(named_file_compiler!(
            "pathtype",
            Arity::exact(1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(1),
        detail: "Returns one of absolute, relative, volumerelative.",
        synopsis: "file pathtype name",
        return_type: Some(TclType::String),
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::PLATFORM_DECIDED,
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "readable",
        native_compilation: Some(named_file_compiler!(
            "readable",
            Arity::exact(1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is readable by the current user, 0 otherwise.",
        synopsis: "file readable name",
        return_type: Some(TclType::Boolean),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "readlink",
        native_compilation: Some(named_file_compiler!(
            "readlink",
            Arity::exact(1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(1),
        detail: "Returns the value of the symbolic link given by name; errors if name is not a readable symbolic link. Undefined on systems without symbolic-link support.",
        synopsis: "file readlink name",
        return_type: Some(TclType::String),
        returns_path: true,
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // Two forms in every version 8.4-9.1: `source target`, or one or
        // more `source`s moved into an existing `targetDir`.
        name: "rename",
        native_compilation: Some(named_file_compiler!("rename", no_hook)),
        state_transitions: crate::knowledge::Knowledge::Partial(StateTransitionDescriptor {
            logical_source_resolver: None,
            commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
            success_resolver: None,
            resolver: Some(filesystem_changed),
            ..StateTransitionDescriptor::EMPTY
        }),
        surface: None,
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
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                    effect: None,
                },
                OptionSpec {
                    name: "--",
                    value: OptionValue::flag(),
                    detail: "Marks the end of switches; the following word is treated as source even if it starts with -.",
                    surface: None,
                    aliases: &[],
                    lifecycle: Lifecycle::UNSPECIFIED,
                    min_abbrev: None,
                    effect: None,
                },
            ]
        },
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        destructive: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "rootname",
        native_compilation: Some(named_file_compiler!(
            "rootname",
            Arity::exact(1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(1),
        detail: "Returns all characters in name up to but not including the last dot in the last component.",
        synopsis: "file rootname name",
        return_type: Some(TclType::String),
        returns_path: true,
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::path::FILE_ROOTNAME),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "separator",
        native_compilation: Some(named_file_compiler!(
            "separator",
            Arity::new(0, 1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::new(0, 1),
        detail: "Returns the character used to separate path segments for native files on this platform.",
        synopsis: "file separator ?name?",
        return_type: Some(TclType::String),
        semantics: SemanticsDeclaration::Declared(
            &crate::value_transfer::builtins::PLATFORM_DECIDED,
        ),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "size",
        native_compilation: Some(named_file_compiler!(
            "size",
            Arity::exact(1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(1),
        detail: "Returns a decimal string giving the size of file name in bytes.",
        synopsis: "file size name",
        return_type: Some(TclType::Int),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "split",
        native_compilation: Some(named_file_compiler!(
            "split",
            Arity::exact(1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(1),
        detail: "Returns a list whose elements are the path components in name.",
        synopsis: "file split name",
        return_type: Some(TclType::List),
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::path::FILE_SPLIT),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // varName became optional in Tcl 9.0 (manpage synopsis is
        // `file stat name varName` — required — through 8.4/8.5/8.6, and
        // becomes `file stat name ?varName?` from 9.0 onward): 9.0+
        // returns a dict of the stat fields when varName is omitted, but
        // still returns the empty string and populates the array when it
        // is given, in every version. The registry has no dialect-split
        // arity (a known gap): the floor of 1 below is only
        // correct for 9.0+; 8.4-8.6 require exactly 2 args (name and
        // varName).
        name: "stat",
        native_compilation: Some(named_file_compiler!(
            "stat",
            Arity::exact(2),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::new(1, 2),
        arity_windows: const {
            &[crate::arity::ArityWindow {
                lifecycle: crate::lifecycle::Lifecycle::UNSPECIFIED.retired_from("9.0"),
                arity: Arity::exact(2),
            }]
        },
        detail: "Invokes the stat kernel call on name. If varName is given, populates it as an array with atime/ctime/dev/gid/ino/mode/mtime/nlink/size/type/uid elements and returns an empty string. From Tcl 9.0, varName may be omitted, in which case the same fields are returned directly as a dict (mandatory in 8.4-8.6, where the array form is the only form).",
        synopsis: "file stat name ?varName?",
        return_type: Some(TclType::String),
        arg_roles: &[(1, ArgRole::VarWrite)],
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::FILE_STAT),
        side_effects: crate::knowledge::Knowledge::Partial(&[
            SideEffect {
                target: SideEffectTarget::FileIo,
                reads: true,
                ..SideEffect::DEFAULT
            },
            SideEffect {
                target: SideEffectTarget::Variable,
                writes: true,
                ..SideEffect::DEFAULT
            },
        ]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "system",
        native_compilation: Some(named_file_compiler!(
            "system",
            Arity::new(0, 1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(1),
        detail: "Returns a one- or two-element list: the name of the filesystem responsible for name (e.g. native), and if given, a filesystem-specific type string (e.g. NTFS, FAT on Windows, or vfs ftp for a mounted virtual filesystem). Errors if the file belongs to no filesystem.",
        synopsis: "file system name",
        return_type: Some(TclType::List),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tail",
        native_compilation: Some(named_file_compiler!(
            "tail",
            Arity::exact(1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(1),
        detail: "Returns all of the characters in the last filesystem component of name.",
        synopsis: "file tail name",
        return_type: Some(TclType::String),
        returns_path: true,
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::path::FILE_TAIL),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // New in Tcl 9.0: absent from the 8.4/8.5/8.6 `file` manpage
        // subcommand list (which has `tempfile` from 8.6 but no
        // `tempdir`), present from the 9.0 manpage on.
        name: "tempdir",
        native_compilation: Some(named_file_compiler!(
            "tempdir",
            Arity::new(0, 1),
            tcl_dialect::TclVersion::V9_0,
        )),
        arity: Arity::new(0, 1),
        detail: "Creates a new, writable temporary directory and returns its name. template may give the containing directory (the part before the last directory separator) and/or a base-name prefix (the part after); defaults to a system-determined directory and the prefix \"tcl\".",
        synopsis: "file tempdir ?template?",
        return_type: Some(TclType::String),
        mutator: true,
        surface: Some(SpecSurface::TCL90_PLUS),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "tempfile",
        native_compilation: Some(named_file_compiler!(
            "tempfile",
            Arity::new(0, 2),
            tcl_dialect::TclVersion::V8_6,
        )),
        traits: Traits::OPENS_CHANNEL.union(Traits::UNCONDITIONAL_VARIABLE_WRITE),
        arity: Arity::new(0, 2),
        detail: "Creates a temporary file and returns a read-write channel opened on that file. If nameVar is given, the file's name is written there; otherwise Tcl arranges to delete the file once it is no longer needed. template may suggest a directory, base name, or extension, though some platforms ignore parts of it. Temporary files are only ever created on the native filesystem.",
        synopsis: "file tempfile ?nameVar? ?template?",
        return_type: Some(TclType::Channel),
        mutator: true,
        arg_roles: &[(0, ArgRole::VarWrite)],
        semantics: SemanticsDeclaration::Declared(&crate::value_transfer::builtins::FILE_TEMPFILE),
        surface: Some(SpecSurface::TCL86_PLUS),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        // New in Tcl 9.0: absent from the 8.4/8.5/8.6 `file` manpage
        // subcommand list, present from the 9.0 manpage on. Unlike
        // `[subst]`'s tilde handling, this is name-only, purely lexical
        // tilde substitution -- not gated on filesystem access.
        name: "tildeexpand",
        native_compilation: Some(named_file_compiler!(
            "tildeexpand",
            Arity::exact(1),
            tcl_dialect::TclVersion::V9_0,
        )),
        arity: Arity::exact(1),
        detail: "Returns the result of performing tilde substitution on name: a leading ~ followed immediately by a separator substitutes $HOME, and ~user substitutes that user's home directory. Returns name unmodified if it does not begin with a tilde. Errors if $HOME is unset or the named user does not exist.",
        synopsis: "file tildeexpand name",
        return_type: Some(TclType::String),
        surface: Some(SpecSurface::TCL90_PLUS),
        returns_path: true,
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "type",
        native_compilation: Some(named_file_compiler!(
            "type",
            Arity::exact(1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(1),
        detail: "Returns a string giving the type of file name: one of file, directory, characterSpecial, blockSpecial, fifo, link, or socket.",
        synopsis: "file type name",
        return_type: Some(TclType::String),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "volumes",
        native_compilation: Some(named_file_compiler!(
            "volumes",
            Arity::exact(0),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(0),
        detail: "Returns the absolute paths to the volumes mounted on the system, as a Tcl list. On Unix this is typically just \"/\" (or additionally a zipfs root); on Windows it is the available local drive letters. Any volumes mounted by a virtual filesystem are included too.",
        synopsis: "file volumes",
        return_type: Some(TclType::List),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
    SubCommand {
        name: "writable",
        native_compilation: Some(named_file_compiler!(
            "writable",
            Arity::exact(1),
            tcl_dialect::TclVersion::V8_6,
        )),
        arity: Arity::exact(1),
        detail: "Returns 1 if file name is writable by the current user, 0 otherwise.",
        synopsis: "file writable name",
        return_type: Some(TclType::Boolean),
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            ..SideEffect::DEFAULT
        }]),
        ..SubCommand::DEFAULT
    },
];

/// Subcommands whose arguments are file names (TIP 602): a leading `~` is
/// expanded to a home directory through Tcl 8.6 and an ordinary character
/// from 9.0. `split` reads `~` the same way in both and is not listed.
const NAME_SUBCOMMANDS: &[&str] = &[
    "atime",
    "attributes",
    "copy",
    "delete",
    "dirname",
    "executable",
    "exists",
    "extension",
    "isdirectory",
    "isfile",
    "join",
    "link",
    "lstat",
    "mkdir",
    "mtime",
    "nativename",
    "normalize",
    "owned",
    "pathtype",
    "readable",
    "readlink",
    "rename",
    "rootname",
    "size",
    "stat",
    "tail",
    "tempfile",
    "type",
    "writable",
];

/// Report a literal file name that begins with `~` where the selected release
/// no longer expands it.
fn validate_tilde_names(
    arguments: crate::InvocationArguments<'_>,
) -> crate::literal_validation::LiteralArgumentValidation {
    use crate::literal_validation::{LiteralArgumentValidation, LiteralValidationDecline};
    let Some(version) = arguments.dialect().and_then(|dialect| dialect.tcl_version) else {
        return LiteralArgumentValidation::Abstain(LiteralValidationDecline::UnselectedGrammar);
    };
    if version.tilde_names_home() {
        return LiteralArgumentValidation::Valid;
    }
    let Some(word) = arguments.literal_at(0) else {
        return LiteralArgumentValidation::Abstain(LiteralValidationDecline::InvalidDiscriminator);
    };
    let exact = SUBCOMMANDS.iter().find(|sub| sub.name == word);
    let mut prefixed = SUBCOMMANDS.iter().filter(|sub| sub.name.starts_with(word));
    let selected = exact.or_else(|| match (prefixed.next(), prefixed.next()) {
        (Some(only), None) => Some(only),
        _ => None,
    });
    if !selected.is_some_and(|sub| NAME_SUBCOMMANDS.contains(&sub.name)) {
        return LiteralArgumentValidation::Valid;
    }
    for index in 1..arguments.len() {
        if arguments
            .literal_at(index)
            .is_some_and(|name| name.starts_with('~'))
        {
            return LiteralArgumentValidation::invalid_value(
                index,
                "file name",
                "a leading `~` is an ordinary character from Tcl 9.0; `file tildeexpand` names the home directory",
            );
        }
    }
    LiteralArgumentValidation::Valid
}

/// Command spec for `file`.
///
/// `dialects` is `ALL_TCL` because `file` is unavailable in F5 iRules:
/// `"file"` is one of the K36322151 bans (the F5 TMM sandbox). `ALL_TCL` does
/// not carry an iRules row, so this spec is simply absent under iRules — the
/// exclusion is expressed directly by the surface, with no disable list
/// involved. No other dialect here (Tk, Expect, tmsh, iApps, the EDA vendor
/// shells) disables or restricts `file`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "file",
        evaluation_contexts: Some(crate::evaluation_context::EvaluationContextSpec::CALLER),
        // Native compileProc registration: pinned C Tcl 8.4.20–9.1.0 tclBasic.c.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::HookFrom(
                tcl_dialect::TclVersion::V8_6,
            ),
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        runtime_backing: RuntimeBacking::shipped("file"),
        surface: Some(SpecSurface::ALL_TCL),
        traits: Traits::BYTE_COMPILED | Traits::RETURNS_PATH | Traits::SAFE_INTERP_HIDDEN,
        arity: Arity::at_least(1),
        subcommands: SUBCOMMANDS,
        side_effects: crate::knowledge::Knowledge::Partial(&[SideEffect {
            target: SideEffectTarget::FileIo,
            reads: true,
            writes: true,
            ..SideEffect::DEFAULT
        }]),
        hover: Some(HoverSnippet {
            summary: "Manipulate file names and attributes",
            synopsis: &["file option name ?arg arg ...?"],
            snippet: "Dispatches on option to one of many operations on a file's name (dirname/tail/extension/join/split/normalize, all pure string or path-table operations) or its on-disk attributes (exists/isdirectory/isfile/readable/writable/executable/owned, atime/mtime/size/type/stat, copy/rename/delete/mkdir, link/readlink, attributes). Any unique abbreviation of option is accepted, matching Tcl's ensemble-style dispatch.",
            source: "Tcl file(n)",
            examples: "file exists $path\nfile mkdir [file dirname $path]\nfile copy -force $src $dst\nset ext [file extension $path]\nfile stat $path st\nputs $st(size)",
            return_value: "Depends on option: a boolean (0/1), a path string, an integer, a list, a dict, or an empty string.",
        }),
        forms: FORMS,
        literal_argument_validator: Some(validate_tilde_names),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod tests {
    use crate::registry::CommandRegistry;

    #[test]
    fn the_file_hover_makes_no_claim_about_tilde_expansion() {
        // Item 192: Tcl 9.0 reads a leading `~` as an ordinary character
        // (TIP 602), so release-neutral prose names no expansion.
        let hover = super::spec().hover.expect("file hover");
        for text in [hover.summary, hover.snippet] {
            assert!(!text.to_lowercase().contains("tilde"), "{text}");
        }
    }

    #[test]
    fn file_compiler_roster_matches_all_190_native_registration_windows() {
        use tcl_dialect::TclVersion;
        let registry = CommandRegistry::build_default();
        let fixtures = [
            (
                TclVersion::V8_4,
                include_str!("../../../tests/data/native_file_compilers/8.4.20.tsv"),
            ),
            (
                TclVersion::V8_5,
                include_str!("../../../tests/data/native_file_compilers/8.5.19.tsv"),
            ),
            (
                TclVersion::V8_6,
                include_str!("../../../tests/data/native_file_compilers/8.6.18.tsv"),
            ),
            (
                TclVersion::V9_0,
                include_str!("../../../tests/data/native_file_compilers/9.0.4.tsv"),
            ),
            (
                TclVersion::V9_1,
                include_str!("../../../tests/data/native_file_compilers/9.1.0.tsv"),
            ),
        ];
        let mut checked = 0;
        for (version, fixture) in fixtures {
            let dialect = tcl_native_model::InvocationDialect::for_version(version);
            let slots = registry.stock_native_implementation_slots(dialect);
            for row in fixture.lines().skip(1) {
                let (identity, raw_hook) = tcl_test_support::fixture::tsv_split_once(row).unwrap();
                let hook: i32 = raw_hook.parse().unwrap();
                if hook < 0 {
                    assert!(
                        slots.iter().all(|lookup| lookup.slot != identity),
                        "{version:?}/{identity}"
                    );
                    assert!(
                        registry
                            .native_compilation_for_registration(identity, dialect)
                            .is_none(),
                        "{version:?}/{identity}"
                    );
                } else {
                    let selected = registry
                        .native_compilation_for_registration(identity, dialect)
                        .unwrap_or_else(|| panic!("{version:?}/{identity}"));
                    assert_eq!(
                        selected.compiler_hook_presence(dialect),
                        Some(hook != 0),
                        "{version:?}/{identity}"
                    );
                    if identity != "file" {
                        assert!(
                            slots.iter().any(|lookup| lookup.slot == identity),
                            "{version:?}/{identity}"
                        );
                    }
                }
                checked += 1;
            }
        }
        assert_eq!(checked, 190);
        let jim = tcl_native_model::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        assert!(
            registry
                .stock_native_implementation_slots(jim)
                .iter()
                .all(|lookup| lookup.command != "file")
        );
        assert!(
            registry
                .native_compilation_for_registration("::tcl::file::exists", jim)
                .is_none()
        );
    }

    #[test]
    fn file_delete_and_mkdir_allow_zero_args() {
        // TIP 323 (Tcl 8.6+) made `file delete` / `file mkdir`
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
