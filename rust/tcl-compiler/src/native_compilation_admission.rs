// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native compiler admission shared by every executable representation.
//!
//! Error presentation and admission are independent: an unresolved compiler
//! traversal requires a genuine provider without inventing a Tcl completion.
//! Evaluating individual commands cannot discharge a chunk-entry obligation.

use std::sync::Arc;

use crate::command_binding::{ExecutedScriptSource, SourceNativeCompilationFailure};

/// Original compilation obligation, independent of a target instruction set.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct NativeCompilationAdmission {
    /// Exact evaluated chunk and its source instance, when retained.
    pub source: Option<ExecutedScriptSource>,
    /// Exact proved failure, including compiler-token dependencies and contexts.
    /// Its absence does not discharge the provider obligation.
    pub failure: Option<Arc<SourceNativeCompilationFailure>>,
    /// At least one native compiler path needs entry-time provider resolution.
    pub provider_required: bool,
}

pub use tcl_runtime_api::NativeCompilationAdmissionScope;

/// A target-neutral resolution of an admission obligation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeCompilationAdmissionPlan<'a> {
    /// No retained compiler obligation changes this entry.
    NoRetainedObligation,
    /// Evaluate the entire original script through its genuine native provider.
    /// This must precede native instructions and procedure-table installation.
    HostScript(&'a ExecutedScriptSource),
    /// Keep a source procedure whose host admits the body before its formals.
    HostProcedure(&'a ExecutedScriptSource),
    /// Execution cannot start without the original chunk/provider contract.
    RefuseMissingSource,
}

impl NativeCompilationAdmission {
    /// Whether a backend must resolve compilation before emitting body effects.
    #[must_use]
    pub fn requires_native_provider(&self) -> bool {
        self.provider_required || self.failure.is_some()
    }

    /// Select the real entry protocol, never infer it from a variable frame.
    #[must_use]
    pub fn plan(
        &self,
        scope: NativeCompilationAdmissionScope,
    ) -> NativeCompilationAdmissionPlan<'_> {
        self.plan_with_requirement(scope, self.requires_native_provider())
    }

    fn plan_with_requirement(
        &self,
        scope: NativeCompilationAdmissionScope,
        required: bool,
    ) -> NativeCompilationAdmissionPlan<'_> {
        if !required {
            return NativeCompilationAdmissionPlan::NoRetainedObligation;
        }
        let Some(source) = &self.source else {
            return NativeCompilationAdmissionPlan::RefuseMissingSource;
        };
        match scope {
            NativeCompilationAdmissionScope::Script => {
                NativeCompilationAdmissionPlan::HostScript(source)
            }
            NativeCompilationAdmissionScope::ProcedureBody => {
                NativeCompilationAdmissionPlan::HostProcedure(source)
            }
        }
    }
}

/// Select a script obligation, including legacy producers that retained only
/// a definite failure. Missing source never becomes an empty successful eval.
#[must_use]
pub fn script_admission_plan(
    script: &crate::ir::Script,
    scope: NativeCompilationAdmissionScope,
) -> NativeCompilationAdmissionPlan<'_> {
    script.native_compilation_admission.as_ref().map_or_else(
        || {
            if script.native_compilation_failure.is_some() {
                NativeCompilationAdmissionPlan::RefuseMissingSource
            } else {
                NativeCompilationAdmissionPlan::NoRetainedObligation
            }
        },
        |admission| {
            admission.plan_with_requirement(
                scope,
                admission.requires_native_provider() || script.native_compilation_failure.is_some(),
            )
        },
    )
}

/// Retain the canonical obligation when crossing into CFG or executable IR.
/// Legacy producers with only a proved failure cannot silently lose admission.
#[must_use]
pub fn retained_script_admission(
    script: &crate::ir::Script,
) -> Option<Arc<NativeCompilationAdmission>> {
    if let Some(admission) = &script.native_compilation_admission
        && (admission.requires_native_provider() || script.native_compilation_failure.is_none())
    {
        return Some(Arc::clone(admission));
    }
    script.native_compilation_failure.as_deref().map(|failure| {
        Arc::new(NativeCompilationAdmission {
            source: script
                .native_compilation_admission
                .as_ref()
                .and_then(|admission| admission.source.clone()),
            failure: Some(Arc::new(failure.clone())),
            provider_required: true,
        })
    })
}

/// Whether a direct/native body would bypass a retained compiler entry.
#[must_use]
pub fn script_requires_admission(script: &crate::ir::Script) -> bool {
    script_admission_plan(script, NativeCompilationAdmissionScope::Script)
        != NativeCompilationAdmissionPlan::NoRetainedObligation
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::{CommandAllocationSite, ExecutedScriptMapping, SourceOriginId};

    #[test]
    fn materialised_source_keeps_its_identity_at_both_entry_protocols() {
        let parent = CommandAllocationSite {
            source: Arc::new(SourceOriginId::authored(&Arc::from("eval $body"))),
            offset: 0,
        };
        let source =
            ExecutedScriptSource::materialised(parent, vec![0], "set before 1; if {0} {set}");
        let admission = NativeCompilationAdmission {
            source: Some(source.clone()),
            failure: None,
            provider_required: true,
        };
        assert_eq!(source.mapping, ExecutedScriptMapping::Materialised);
        assert_eq!(
            admission.plan(NativeCompilationAdmissionScope::Script),
            NativeCompilationAdmissionPlan::HostScript(&source)
        );
        assert_eq!(
            admission.plan(NativeCompilationAdmissionScope::ProcedureBody),
            NativeCompilationAdmissionPlan::HostProcedure(&source)
        );
        assert!(
            admission.failure.is_none(),
            "an unresolved compiler is not a Tcl error"
        );
    }

    #[test]
    fn missing_source_refuses_entry_instead_of_evaluating_an_empty_script() {
        let admission = NativeCompilationAdmission {
            source: None,
            failure: None,
            provider_required: true,
        };
        for scope in [
            NativeCompilationAdmissionScope::Script,
            NativeCompilationAdmissionScope::ProcedureBody,
        ] {
            assert_eq!(
                admission.plan(scope),
                NativeCompilationAdmissionPlan::RefuseMissingSource
            );
        }
    }

    #[test]
    fn lowered_and_cfg_entries_retain_the_same_compiler_obligation() {
        let registry = tcl_registry::registry::CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::find("tcl8.4").unwrap());
        let source = "set flag 0; set before 1; if {$flag} {set}";
        let direct = crate::compilation_unit::CompilationUnit::build_for_dialect(
            source, &registry, false, "tcl8.4",
        );
        assert!(
            direct
                .ir_module
                .top_level
                .native_compilation_admission
                .is_none()
        );
        let profile = tcl_dialect::DialectProfile::find("tcl8.4").unwrap();
        let entry = crate::command_binding::SourceAnalysisEntry {
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                frame: tcl_registry::native_compilation::NativeCompilationFrame::ScriptCode,
                loop_depth: 0,
                catch_depth: Some(0),
            },
            ..Default::default()
        };
        let unit = crate::compilation_unit::CompilationUnit::build_with_source_entry(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: &registry,
                defer_top_level: false,
                config: tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                dialect: Some(profile),
                external_call_sites: None,
                declared_commands: None,
            },
            &entry,
        );
        let admission = unit
            .ir_module
            .top_level
            .native_compilation_admission
            .as_ref()
            .expect("C8.4 rejects unreachable bad set at chunk entry");
        assert_eq!(
            admission.source.as_ref().unwrap().text.try_text().unwrap(),
            source
        );
        assert!(admission.failure.is_some());
        let retained = unit
            .top_level
            .cfg
            .native_compilation_admission
            .as_ref()
            .unwrap();
        assert!(Arc::ptr_eq(admission, retained));
        assert_eq!(
            admission.failure.as_ref().unwrap().dependencies,
            retained.failure.as_ref().unwrap().dependencies
        );
        let mut legacy = unit.ir_module.top_level.clone();
        legacy.native_compilation_admission = None;
        assert_eq!(
            script_admission_plan(&legacy, NativeCompilationAdmissionScope::Script),
            NativeCompilationAdmissionPlan::RefuseMissingSource
        );
        let restored = retained_script_admission(&legacy).unwrap();
        assert!(restored.requires_native_provider());
        assert!(restored.failure.is_some());
        assert!(restored.source.is_none());
    }
}
