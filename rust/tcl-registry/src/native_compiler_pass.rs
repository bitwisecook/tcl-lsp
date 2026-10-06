// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Whole-unit C compiler replay selected from original emitted instructions.

use tcl_dialect::TclVersion;
use tcl_runtime_api::native_compilation::NativeInterpreterIdentity;
use tcl_runtime_api::native_compiler_pass::NativeCompilerPassEnvironment;

/// Native first-pass opcode classes that prevent command-marker compaction.
/// Producers classify emitted instructions, never source command spellings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompilerPassHazard {
    /// Stack, expanded or captured-prefix command invocation.
    Invocation,
    /// Runtime script evaluation.
    ScriptEvaluation,
    /// Runtime expression evaluation.
    ExpressionEvaluation,
    /// Coroutine yield.
    Yield,
    /// Yield followed by invocation; absent from the C86 hazard switch.
    YieldTo,
    /// Caller-frame variable link.
    Upvar,
    /// Namespace variable link.
    NamespaceUpvar,
    /// Procedure variable declaration/link.
    Variable,
}

/// Select the native second compiler pass. Missing or retired environments,
/// child interpreters and enabled limits refuse independently of opcode safety.
/// The actual procedure namespace exception applies only after those gates.
#[must_use]
pub fn native_compiler_replays(
    version: TclVersion,
    interpreter: NativeInterpreterIdentity,
    environment: Option<&NativeCompilerPassEnvironment>,
    hazards: impl IntoIterator<Item = NativeCompilerPassHazard>,
) -> bool {
    if version < TclVersion::V8_6 {
        return false;
    }
    let Some(environment) = environment else {
        return false;
    };
    if !environment.is_current_for(interpreter)
        || !environment.is_root()
        || environment.has_enabled_limits()
    {
        return false;
    }
    if environment.procedure().is_some_and(|procedure| {
        let name = procedure.namespace_full_name.as_bytes();
        name == b"::tcl" || name.starts_with(b"::tcl::")
    }) {
        return true;
    }
    !hazards
        .into_iter()
        .any(|hazard| hazard != NativeCompilerPassHazard::YieldTo || version >= TclVersion::V9_0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_runtime_api::native_compilation::NativeNamespaceContext;
    use tcl_runtime_api::native_compiler_pass::{
        NativeCompilerPassOwner, NativeCompilerPassProcedure,
    };

    fn identity() -> NativeInterpreterIdentity {
        NativeInterpreterIdentity {
            owner: 41,
            interpreter: 7,
        }
    }

    #[test]
    fn compiler_pass_requires_live_root_without_enabled_limits() {
        let owner = NativeCompilerPassOwner::new(identity());
        for version in [
            TclVersion::V8_4,
            TclVersion::V8_5,
            TclVersion::V8_6,
            TclVersion::V9_0,
            TclVersion::V9_1,
        ] {
            let root = owner.capture(true, false, false, None);
            assert_eq!(
                native_compiler_replays(version, identity(), Some(&root), []),
                version >= TclVersion::V8_6
            );
            for environment in [
                owner.capture(false, false, false, None),
                owner.capture(true, true, false, None),
                owner.capture(true, false, true, None),
            ] {
                assert!(!native_compiler_replays(
                    version,
                    identity(),
                    Some(&environment),
                    []
                ));
            }
            assert!(!native_compiler_replays(version, identity(), None, []));
            assert!(!native_compiler_replays(
                version,
                NativeInterpreterIdentity {
                    owner: 42,
                    ..identity()
                },
                Some(&root),
                []
            ));
        }
        let retired = owner.capture(true, false, false, None);
        drop(owner);
        assert!(!native_compiler_replays(
            TclVersion::V9_1,
            identity(),
            Some(&retired),
            []
        ));
    }

    #[test]
    fn raw_hazards_and_original_library_procedure_exception_are_independent() {
        let owner = NativeCompilerPassOwner::new(identity());
        let ordinary = owner.capture(true, false, false, None);
        for hazard in [
            NativeCompilerPassHazard::Invocation,
            NativeCompilerPassHazard::ScriptEvaluation,
            NativeCompilerPassHazard::ExpressionEvaluation,
            NativeCompilerPassHazard::Yield,
            NativeCompilerPassHazard::Upvar,
            NativeCompilerPassHazard::NamespaceUpvar,
            NativeCompilerPassHazard::Variable,
        ] {
            for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
                assert!(!native_compiler_replays(
                    version,
                    identity(),
                    Some(&ordinary),
                    [hazard]
                ));
            }
        }
        assert!(native_compiler_replays(
            TclVersion::V8_6,
            identity(),
            Some(&ordinary),
            [NativeCompilerPassHazard::YieldTo]
        ));
        assert!(!native_compiler_replays(
            TclVersion::V9_0,
            identity(),
            Some(&ordinary),
            [NativeCompilerPassHazard::YieldTo]
        ));
        for name in [
            b"::tcl".as_slice(),
            b"::tcl::library",
            b"::tclExtra",
            b"::N",
        ] {
            let procedure = NativeCompilerPassProcedure {
                namespace: NativeNamespaceContext {
                    interpreter: identity(),
                    token: 99,
                    path: tcl_runtime_api::ByteNamespacePath::from_segments([b"tcl".as_slice()]),
                },
                namespace_full_name: tcl_runtime_api::NameBytes::from(name),
            };
            let environment = owner.capture(true, false, false, Some(procedure));
            assert_eq!(
                native_compiler_replays(
                    TclVersion::V9_1,
                    identity(),
                    Some(&environment),
                    [NativeCompilerPassHazard::Invocation]
                ),
                name == b"::tcl" || name.starts_with(b"::tcl::")
            );
            assert!(!native_compiler_replays(
                TclVersion::V9_1,
                identity(),
                Some(&environment.without_procedure()),
                [NativeCompilerPassHazard::Invocation]
            ));
        }
    }
}
