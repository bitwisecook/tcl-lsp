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

//! `open` — open a file, device, or command-pipeline channel.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const FORMS: &[FormSpec] = &[
    FormSpec {
        synopsis: "open fileName ?access? ?permissions?",
        ..FormSpec::DEFAULT
    },
    FormSpec {
        synopsis: "open |command ?access?",
        ..FormSpec::DEFAULT
    },
];

/// `access` (position 1) takes one of the six mode strings below, each
/// optionally followed by a `b` (e.g. `rb`, `r+b`) to configure the
/// channel for binary translation up front — added in Tcl 8.5; Tcl 8.4
/// has no `b` suffix, and a value like `rb` is rejected there with an
/// `illegal access mode` error (`TclGetOpenMode`, generic/tclIOUtil.c —
/// not the similarly-named `invalid access mode` error, which is a
/// distinct 8.4 error path for a bad flag inside the POSIX list form),
/// so binary channels on 8.4 must call
/// `fconfigure -translation binary` after `open` instead. `access` can
/// also be a Tcl list combining one of RDONLY/WRONLY/RDWR with any of
/// the other POSIX flags (e.g. `{RDWR CREAT}`). A list value can't be
/// checked against a flat per-word enum, so this is completion/hover
/// data only — `open` has no `closed_value_args` entry for this
/// position, since a legitimate multi-flag list would otherwise be
/// misreported as an invalid value.
const ACCESS_VALUES: &[ArgValue] = &[
    ArgValue {
        value: "r",
        detail: "Read only; the file must already exist. The default when access is omitted.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "r+",
        detail: "Read and write; the file must already exist.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "w",
        detail: "Write only; truncates an existing file, creates a new one if absent.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "w+",
        detail: "Read and write; truncates an existing file, creates a new one if absent.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "a",
        detail: "Write only; creates the file if absent, and seeks to the end before every write.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "a+",
        detail: "Read and write; creates the file if absent, with the initial position at the end.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "RDONLY",
        detail: "List form: open read-only. Combine with other flags in a list, e.g. {RDONLY NOCTTY}.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "WRONLY",
        detail: "List form: open write-only.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "RDWR",
        detail: "List form: open for both reading and writing.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "APPEND",
        detail: "List form: seek to the end of the file before each write.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "BINARY",
        detail: "List form: configure the channel as if by `fconfigure -translation binary`.",
        min_tcl: Some(TclVersion::V8_5),
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "CREAT",
        detail: "List form: create the file if it does not already exist.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "EXCL",
        detail: "List form: used with CREAT, fail if the file already exists.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "NOCTTY",
        detail: "List form: if fileName refers to a terminal device, prevent it becoming the process's controlling terminal.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "NONBLOCK",
        detail: "List form: open in non-blocking mode (matters mainly when fileName is a FIFO); use `fconfigure -blocking` to control blocking afterwards.",
        ..ArgValue::DEFAULT
    },
    ArgValue {
        value: "TRUNC",
        detail: "List form: truncate the file to zero length if it already exists.",
        ..ArgValue::DEFAULT
    },
];

fn discovery_file_open(arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    if !(1..=3).contains(&arguments.len())
        || arguments
            .literal_at(0)
            .is_some_and(|path| path.starts_with('|'))
    {
        return transitions;
    }
    let writable = match arguments.literal_at(1) {
        None => arguments.len() >= 2,
        Some("r" | "rb") => false,
        Some(mode) => !arguments
            .dialect()
            .map_or_else(
                || tcl_syntax::list::split_list(mode),
                |dialect| dialect.word_values.split_list(mode),
            )
            .is_ok_and(|flags| {
                flags.iter().any(|flag| flag == "RDONLY")
                    && flags.iter().all(|flag| {
                        !matches!(
                            flag.as_ref(),
                            "CREAT" | "TRUNC" | "WRONLY" | "RDWR" | "APPEND"
                        )
                    })
            }),
    };
    if writable {
        transitions.push(StateTransition::Package(
            crate::model::binding::PackageTransition::DiscoveryDependencyChanged {
                dependency: crate::model::binding::PackageResolverDependency::Filesystem,
            },
        ));
    }
    transitions
}

/// Command spec for `open`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "open",
        // Native compileProc registration: pinned C Tcl 8.4.20–9.1.0 tclBasic.c.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::NoHook,
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        state_transitions: Some(StateTransitionDescriptor {
            commit: StateTransitionCommit::MayCommitBeforeAbruptCompletion,
            success_resolver: None,
            resolver: Some(discovery_file_open),
            ..StateTransitionDescriptor::EMPTY
        }),
        surface: Some(SpecSurface::ALL_TCL),
        traits: Traits::BYTE_COMPILED
            | Traits::OPENS_CHANNEL
            | Traits::SAFE_INTERP_HIDDEN
            | Traits::TAINT_SINK,
        arity: Arity::new(1, 3),
        return_type: Some(TclType::Channel),
        side_effects: &[
            SideEffect {
                target: SideEffectTarget::FileIo,
                reads: true,
                writes: true,
                ..SideEffect::DEFAULT
            },
            SideEffect {
                target: SideEffectTarget::Process,
                reads: true,
                writes: true,
                ..SideEffect::DEFAULT
            },
        ],
        hover: Some(HoverSnippet {
            summary: "Open a file, device, or command pipeline as a channel.",
            synopsis: &[
                "open fileName",
                "open fileName access",
                "open fileName access permissions",
                "open |command ?access?",
            ],
            snippet: "Returns a channel identifier for use with read, puts, gets, close, and friends. access defaults to \"r\" (read-only) when omitted. Since Tcl 8.5, any of the six mode strings may end with a trailing b (rb, r+b, …) — or the POSIX list form may include BINARY — to configure the channel for binary translation up front, equivalent to a later fconfigure -translation binary call; Tcl 8.4 has neither, so binary-safe code there must call fconfigure itself after opening. permissions (an integer, default 0666, combined with the process umask) only matters when access creates a new file. A fileName whose first character is | instead treats the rest of the word as a command pipeline, in the same style as exec's arguments: the channel then reads the pipeline's stdout or writes its stdin depending on access, and the child process's id is available via `pid channelId`.",
            source: "Tcl open(n)",
            examples: "open $path r\nopen $path rb\nopen $path {RDWR CREAT} 0644\nopen |[list grep -c foo] r",
            return_value: "A channel identifier.",
        }),
        forms: FORMS,
        arg_values: &[(1, ACCESS_VALUES)],
        // Only `fileName` (arg 0) is the code-execution sink: a leading `|`
        // there runs the rest of the word as a command pipeline (see the
        // hover snippet). The later `access` mode and octal `permissions`
        // arguments cannot turn a literal fileName into a command, so a
        // tainted mode/permissions value (`open /tmp/out $mode`) must not
        // draw T100 — without this, the whole-command sink default flags
        // every argument.
        taint_code_sink_args: Some(&[0]),
        ..CommandSpec::DEFAULT
    }
}

#[cfg(test)]
mod discovery_tests {
    use super::*;

    #[test]
    fn writable_open_revokes_loader_discovery_but_read_only_does_not() {
        let descriptor = spec()
            .state_transitions
            .expect("owned discovery descriptor");
        for (words, changed) in [
            (vec!["pkgIndex.tcl"], false),
            (vec!["pkgIndex.tcl", "r"], false),
            (vec!["pkgIndex.tcl", "rb"], false),
            (vec!["pkgIndex.tcl", "RDONLY BINARY"], false),
            (vec!["pkgIndex.tcl", "w"], true),
            (vec!["pkgIndex.tcl", "r+"], true),
            (vec!["pkgIndex.tcl", "RDWR CREAT"], true),
            (vec!["|worker", "w"], false),
        ] {
            let facts = descriptor.resolve(InvocationArguments::literals(&words));
            assert_eq!(!facts.facts().is_empty(), changed, "{words:?}");
            if changed {
                assert_eq!(
                    facts.facts()[0].commit,
                    StateTransitionCommit::MayCommitBeforeAbruptCompletion
                );
            }
        }
    }
}
