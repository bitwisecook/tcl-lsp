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

//! `cd` — change the process's current working directory.

use crate::prelude::*;
use tcl_dialect::model::SpecSurface;

const FORMS: &[FormSpec] = &[FormSpec {
    synopsis: "cd ?dirName?",
    ..FormSpec::DEFAULT
}];

fn discovery_changed(arguments: InvocationArguments<'_>) -> StateTransitions {
    let mut transitions = StateTransitions::default();
    if arguments.len() <= 1 {
        transitions.push(StateTransition::Package(
            crate::model::binding::PackageTransition::DiscoveryDependencyChanged {
                dependency: crate::model::binding::PackageResolverDependency::WorkingDirectory,
            },
        ));
    }
    transitions
}

/// Command spec for `cd`.
pub fn spec() -> CommandSpec {
    CommandSpec {
        name: "cd",
        // Native compileProc registration: pinned C Tcl 8.4.20–9.1.0 tclBasic.c.
        native_compilation: Some(crate::native_compilation::NativeCompilationSpec {
            grammar: crate::native_compilation::NativeCompilationGrammar::NoHook,
            operation: crate::SemanticOperationId::Invoke,
            body: crate::native_compilation::NativeBodyCompilation::Inherit,
        }),
        // Universal core Tcl 8.4-9.1 (identical `cd ?dirName?` synopsis and
        // behaviour on every fetched manpage). Excluded from `f5-irules` (no
        // real per-request filesystem there) by this explicit
        // `Some(SpecSurface::ALL_TCL)` group: `ALL_TCL` carries no iRules row,
        // so the spec never intersects iRules' bare `IRULES` availability
        // point — the same way `open` is excluded, rather than by any disable
        // list — see `open_.rs`.
        surface: Some(SpecSurface::ALL_TCL),
        // `Traits::TAINT_SINK`: `dirName` unconditionally becomes the
        // process's current working directory — a per-process resource
        // shared by every interpreter and, in a threaded build, every
        // thread (all five manpages: "the current working directory is a
        // per-process resource; the cd command changes the working
        // directory for all interpreters and ... all threads"). Attacker-
        // influenced input reaching this argument lets the caller redirect
        // every later *relative*-path operation (`source`, `open`, `exec`,
        // `glob`, …) to a directory of its choosing — the same downstream
        // hazard noted on `auto_mkindex`, whose internal `cd $dir` call
        // this mirrors directly.
        traits: Traits::BYTE_COMPILED | Traits::SAFE_INTERP_HIDDEN | Traits::TAINT_SINK,
        arity: Arity::new(0, 1),
        state_transitions: Some(StateTransitionDescriptor {
            success_resolver: None,
            resolver: Some(discovery_changed),
            ..StateTransitionDescriptor::EMPTY
        }),
        return_type: Some(TclType::String),
        side_effects: &[SideEffect {
            target: SideEffectTarget::FileIo,
            writes: true,
            ..SideEffect::DEFAULT
        }],
        hover: Some(HoverSnippet {
            summary: "Change the process's current working directory.",
            synopsis: &["cd ?dirName?"],
            snippet: "Changes the current working directory to dirName, or to the user's home directory (as given by the HOME environment variable) when dirName is omitted; returns the empty string. The working directory is a per-process resource: the change is visible to every interpreter and, in a threaded build, every thread, not just the one that called cd. Errors if dirName does not exist, is not a directory, or cannot be entered. Through Tcl 8.6, a dirName beginning with ~ underwent automatic tilde substitution to a home directory (e.g. cd ~fred); Tcl 9.0 removed this implicit tilde substitution from file-name processing, so a ~ path must now be expanded explicitly first, e.g. with `file home` or `file tildeexpand`.",
            source: "Tcl cd(n)",
            examples: "cd /var/log\ncd ..\ncd ~fred              ;# Tcl 8.4-8.6: ~ expands automatically\ncd [file home fred]    ;# Tcl 9.0+: ~ is no longer expanded implicitly",
            return_value: "The empty string.",
        }),
        forms: FORMS,
        ..CommandSpec::DEFAULT
    }
}
