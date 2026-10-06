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

//! A compiled private name and its independently selected late handler.

use super::{
    CommandAllocationSite, CompiledExecutionCertainty, ModuleCommandBindings, SourceCommandTarget,
    SourceInvocationBinding, SourceNativeCompilationDependency,
};
use tcl_registry::native_compilation::NativeCompilerImplementationLookup;

/// Native compilation captures a command name rather than its implementation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceNamedInvocationProof {
    /// Original ensemble member and selected private name.
    pub lookup: Option<&'static NativeCompilerImplementationLookup>,
    /// Actual captured command name; mutable maps do not donate stock semantics.
    pub captured_name: String,
    /// Actual compiler and configuration selected independently of handler identity.
    pub compiler_prerequisite: Option<
        std::sync::Arc<tcl_runtime_api::native_compilation::NativeEnsembleCompilerPrerequisite>,
    >,
    /// Number of written post-head words consumed by ensemble selection.
    pub arguments_from: usize,
    /// Actual invocation layout selected by the compiler owner.
    pub protocol: tcl_registry::native_compilation::NativeNamedInvocationProtocol,
    /// Canonical ensemble words replacing the original literal member path.
    pub replacement_words: Vec<String>,
    /// Exact original command compilation site.
    pub compilation_site: CommandAllocationSite,
    /// Whether this name-selection branch covers every execution.
    pub certainty: CompiledExecutionCertainty,
    /// Original compiler implementation prerequisites, distinct from the late handler.
    pub dependencies: Vec<SourceNativeCompilationDependency>,
    /// Private-slot implementations resolved after every argv substitution.
    pub targets: Vec<SourceCommandTarget>,
    /// The late private lookup retains an unbounded alternative.
    pub unknown: bool,
    /// The late private lookup can fail to find a command.
    pub may_be_absent: bool,
}

impl SourceNamedInvocationProof {
    /// Actual command name captured before argument execution.
    #[must_use]
    pub fn captured_name(&self) -> &str {
        &self.captured_name
    }

    /// Exact late handler, with no captured-implementation claim.
    #[must_use]
    pub fn proved_target(&self) -> Option<&SourceCommandTarget> {
        if self.certainty != CompiledExecutionCertainty::Must || self.unknown || self.may_be_absent
        {
            return None;
        }
        let [target] = self.targets.as_slice() else {
            return None;
        };
        Some(target)
    }

    pub(super) fn with_late_lookup(&self, state: &ModuleCommandBindings) -> Self {
        let mut proof = self.clone();
        // The captured name remains a late lookup after substitutions. Its
        // root belongs to the actual table owner, not an authored "::" label.
        // Do not freeze the worker retained by the compilation prerequisite:
        // argument evaluation may replace or remove that exact private slot.
        let Some(root) = state.source_root_namespace_key() else {
            proof.targets.clear();
            proof.unknown = true;
            proof.may_be_absent = true;
            return proof;
        };
        let binding = super::source_binding_projection(state, self.captured_name(), &root);
        proof.targets = binding.targets;
        proof.unknown = binding.unknown;
        proof.may_be_absent = binding.may_be_absent;
        proof
    }

    fn same_plan(&self, other: &Self) -> bool {
        self.lookup == other.lookup
            && self.captured_name == other.captured_name
            && self.compiler_prerequisite == other.compiler_prerequisite
            && self.arguments_from == other.arguments_from
            && self.protocol == other.protocol
            && self.replacement_words == other.replacement_words
            && self.compilation_site == other.compilation_site
            && self.dependencies == other.dependencies
    }
}

impl SourceInvocationBinding {
    /// A unique compiler-selected name whose handler remains a late lookup.
    #[must_use]
    pub fn named_invocation(&self) -> Option<&SourceNamedInvocationProof> {
        if self.compiled_execution_unknown()
            || self.may_use_live_dispatch
            || !self.compiled_candidates.is_empty()
        {
            return None;
        }
        let [proof] = self.compiled_named_candidates.as_slice() else {
            return None;
        };
        (proof.certainty == CompiledExecutionCertainty::Must).then_some(proof)
    }

    pub(super) fn join_named_execution(&mut self, other: &Self) {
        for proof in &mut self.compiled_named_candidates {
            if let Some(incoming) = other
                .compiled_named_candidates
                .iter()
                .find(|incoming| proof.same_plan(incoming))
            {
                for target in &incoming.targets {
                    if !proof.targets.contains(target) {
                        proof.targets.push(target.clone());
                    }
                }
                proof.unknown |= incoming.unknown;
                proof.may_be_absent |= incoming.may_be_absent;
                if incoming.certainty == CompiledExecutionCertainty::May {
                    proof.certainty = CompiledExecutionCertainty::May;
                }
            } else {
                proof.certainty = CompiledExecutionCertainty::May;
            }
        }
        for incoming in &other.compiled_named_candidates {
            if !self
                .compiled_named_candidates
                .iter()
                .any(|proof| proof.same_plan(incoming))
            {
                let mut proof = incoming.clone();
                proof.certainty = CompiledExecutionCertainty::May;
                self.compiled_named_candidates.push(proof);
            }
        }
    }
}

#[cfg(test)]
pub(super) mod tests {
    use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};
    use tcl_registry::native_compilation::{
        NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
    };

    fn analyse(source: &str) -> SourceCommandBindings {
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::default(),
            &tcl_registry::CommandRegistry::build_default(),
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V8_6,
                )),
                native_compilation: NativeCompilationContext {
                    mode: NativeCompilationMode::BytecodeObject,
                    frame: NativeCompilationFrame::ScriptCode,
                    loop_depth: 0,
                    catch_depth: Some(0),
                },
                ..SourceAnalysisOptions::default()
            },
        )
    }

    /// An actual modern C ensemble chooses a late custom worker. All lookup
    /// candidates are retained native slots, including a colon-named caller
    /// context which cannot be reconstructed from a displayed namespace.
    pub(in crate::command_binding) fn native_entry()
    -> tcl_runtime_api::native_compilation::NativeCompilationEntry {
        use tcl_core_types::{ByteNamespacePath, NativeByteCommandSlot};
        use tcl_runtime_api::native_compilation::*;
        let dialect = tcl_registry::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let public = NativeCompilationBinding {
            slot: NativeByteCommandSlot::new(ByteNamespacePath::root(), "info".into()),
            namespace_token: 0,
            token: 1,
            implementation_generation: 1,
            implementation: NativeCommandImplementation::Registry {
                identity: "info".into(),
                compiler_hook: true,
            },
            compiler_hook: NativeCompilerHookPresence::Present,
            compiler: Some(NativeCommandCompiler {
                registry_identity: "info".into(),
                ensemble: Some(NativeEnsembleCompiler {
                    namespace_token: 0,
                    map: vec![("body".into(), vec![Some("::helper".into())])],
                    subcommands: None,
                    prefixes: true,
                    parameters: Vec::new(),
                    unknown_handler: None,
                }),
            }),
            procedure_header: None,
            has_execution_trace: false,
        };
        let mut worker = public.clone();
        worker.slot.simple = "helper".into();
        worker.token = 2;
        worker.implementation = NativeCommandImplementation::Opaque;
        worker.compiler_hook = NativeCompilerHookPresence::Absent;
        worker.compiler = None;
        let mut commands = vec![public, worker];
        for (token, name) in [(3, "proc"), (4, "rename")] {
            let mut row = commands[1].clone();
            row.slot.simple = name.into();
            row.token = token;
            row.implementation = NativeCommandImplementation::Registry {
                identity: name.into(),
                compiler_hook: false,
            };
            commands.push(row);
        }
        let root = NativeCompilationNamespace {
            path: ByteNamespacePath::root(),
            jim_namespace_object: None,
            token: 0,
            visible: true,
            exports: Vec::new(),
            command_path: Vec::new(),
            unknown_handler: None,
        };
        let mut caller = root.clone();
        caller.token = 7;
        caller.path = ByteNamespacePath::from_segments([":"]);
        NativeCompilationEntry {
            interpreter: NativeInterpreterIdentity {
                owner: NativeInterpreterIdentity::fresh_owner(),
                interpreter: 0,
            },
            epoch: 0,
            profile: profile.cache_key(),
            invocation_policy: Some(profile.cache_key()),
            execution_point: dialect.execution_point(),
            name_protocol: tcl_syntax::naming::NamePolicyProtocol::for_native_point(
                dialect.execution_point().unwrap(),
            ),
            compiled_variable_protocol: None,
            compiled_local_layout: None,
            source_string_protocol: Some(tcl_syntax::native_string::NativeStringProtocol::C(
                tcl_dialect::TclVersion::V8_6,
            )),
            lexer_grammar: Some(profile.grammar),
            inline_compilation_disabled: false,
            math_functions: None,
            closed: true,
            commands,
            ensemble_target_objects: Some(vec![NativeEnsembleTargetObservation {
                ensemble_token: 1,
                member: "body".into(),
                prefix_index: 0,
                object_identity: 1,
                resident_name: Some("::helper".into()),
                primary: NativeEnsembleTargetPrimary::StockString,
            }]),
            namespaces: vec![root, caller],
            current_namespace: 7,
            namespace_variable_tables: None,
            variable_observers: NativeVariableObserverPresence::Absent,
            authored_tmm_static: None,
            frame: NativeCompilationFrame::Namespace,
        }
    }

    fn analyse_native_named(
        source: &str,
        entry: &tcl_runtime_api::native_compilation::NativeCompilationEntry,
    ) -> SourceCommandBindings {
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::default(),
            &tcl_registry::CommandRegistry::build_default(),
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(
                    tcl_dialect::TclVersion::V8_6,
                )),
                native_entry: Some(entry),
                native_compilation: NativeCompilationContext {
                    mode: NativeCompilationMode::BytecodeObject,
                    frame: NativeCompilationFrame::ScriptCode,
                    loop_depth: 0,
                    catch_depth: Some(0),
                },
                ..SourceAnalysisOptions::default()
            },
        )
    }

    #[test]
    fn named_native_worker_lookup_keeps_actual_root_and_late_substitution_changes() {
        let entry = native_entry();
        let initial = analyse_native_named("info body original", &entry);
        let proof = initial.invocation_at_source("info", 0);
        let named = proof
            .named_invocation()
            .expect("native ensemble retained its private name");
        assert_eq!(named.captured_name(), "::helper");
        assert_eq!(
            named
                .proved_target()
                .unwrap()
                .identity
                .as_ref()
                .unwrap()
                .runtime
                .unwrap()
                .token,
            2
        );
        assert!(!named.may_be_absent);
        assert!(!initial.native_compilation_provider_required_at(0));

        let replaced = analyse_native_named(
            "info body [rename ::helper {}; proc ::helper {args} {return CHANGED}]",
            &entry,
        );
        let proof = replaced.invocation_at_source("info", 0);
        let named = proof
            .named_invocation()
            .expect("argument replacement preserves captured name");
        assert_eq!(named.captured_name(), "::helper");
        assert!(
            named
                .proved_target()
                .is_some_and(|target| target.kind == super::super::BindingKind::Proc
                    && target.implementation_allocation.is_some())
        );
        assert!(!named.may_be_absent);

        let removed = analyse_native_named("info body [rename ::helper {}]", &entry);
        let proof = removed.invocation_at_source("info", 0);
        let named = proof
            .named_invocation()
            .expect("argument removal preserves captured name");
        assert!(named.targets.is_empty());
        assert!(named.may_be_absent);
        assert!(!named.unknown);
    }

    #[test]
    fn named_native_worker_missing_root_cannot_fall_back_to_authored_table() {
        use super::super::{ModuleCommandBindings, SourceCommandKey, SourceNamespaceKey};
        let mut entry = native_entry();
        entry.namespaces.retain(|namespace| namespace.token != 0);
        let registry = tcl_registry::CommandRegistry::build_default();
        let mut state = ModuleCommandBindings::initial_with_options(
            &registry,
            SourceAnalysisOptions {
                native_entry: Some(&entry),
                ..SourceAnalysisOptions::default()
            },
            None,
        );
        let authored = analyse("info body original");
        let binding = authored.invocation_at_source("info", 0);
        let mut named = binding.named_invocation().unwrap().clone();
        named.captured_name = "::helper".to_owned();
        // A same-spelled authored candidate cannot substitute for the missing
        // native root incarnation, regardless of its visible handler spelling.
        let key = SourceCommandKey::slot(SourceNamespaceKey::authored("::"), "helper".into());
        let bindings = ModuleCommandBindings::unmodified_bindings(
            &SourceCommandKey::authored("::set"),
            state.baseline.semantics.binding_names(),
        );
        std::sync::Arc::make_mut(&mut state.bindings).insert(key, bindings);
        let result = named.with_late_lookup(&state);
        assert!(result.targets.is_empty());
        assert!(result.unknown);
        assert!(result.may_be_absent);
    }

    #[test]
    fn typed_rebinding_candidates_preserve_known_lookup_and_withdraw_unknown_frames() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jimtcl"] {
            let owner = tcl_registry::model::ingress::static_context_for(engine);
            let registry = owner.commands();
            let profile = registry.profile().unwrap();
            let entry = crate::environment_ingress::captured_native_entry(profile);
            let analysis = SourceCommandBindings::analyse_with_options(
                "",
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                registry,
                SourceAnalysisOptions {
                    native_entry: Some(&entry),
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    ..Default::default()
                },
            );
            let mut state = (*analysis.final_state).clone();
            let root = state.native_root_namespace_key().unwrap();
            state.record_proc_rebound_candidates(
                "set",
                &crate::ir_helpers::ExecutionNamespace::SourceContext(root),
            );
            assert!(!state.has_opaque_domain(), "{engine}");
            let trust = state.proc_binding_trust_projection();
            assert!(!trust.has_dynamic_binding_transition(), "{engine}");
            assert!(!trust.trusts_proc_binding("::set"), "{engine}");
            state.record_proc_rebound_candidates(
                "set",
                &crate::ir_helpers::ExecutionNamespace::RuntimeSelected,
            );
            assert!(state.has_opaque_domain(), "{engine}");
            assert!(
                state
                    .proc_binding_trust_projection()
                    .has_dynamic_binding_transition()
            );
        }
    }

    #[test]
    fn no_body_handler_preserves_completion_without_selected_frame() {
        let owner = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let registry = owner.commands();
        let profile = registry.profile().unwrap();
        let entry = crate::environment_ingress::captured_native_entry(profile);
        let original = entry
            .commands
            .iter()
            .find(|row| row.slot.namespace.is_root() && row.slot.simple == "set")
            .expect("captured original root set row");
        let tcl_runtime_api::native_compilation::NativeCommandImplementation::Registry {
            identity,
            ..
        } = &original.implementation
        else {
            panic!("original set must carry its genuine registry implementation");
        };
        let expected_token = super::super::RuntimeCommandTokenIdentity {
            interpreter: entry.interpreter,
            token: original.token,
        };
        let analyse_native = |source| {
            SourceCommandBindings::analyse_with_options(
                source,
                tcl_lexer::LexerConfig::from_grammar(profile.grammar),
                registry,
                SourceAnalysisOptions {
                    native_entry: Some(&entry),
                    invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                    native_compilation: NativeCompilationContext {
                        mode: NativeCompilationMode::BytecodeObject,
                        frame: NativeCompilationFrame::ScriptCode,
                        loop_depth: 0,
                        catch_depth: Some(0),
                    },
                    ..Default::default()
                },
            )
        };
        let source = "rename ::set ::saved; saved final 1";
        let analysis = analyse_native(source);
        let offset = u32::try_from(source.find("saved final").unwrap()).unwrap();
        let binding = analysis.invocation_at_source("saved", offset);
        let target = binding
            .proved_execution_target()
            .expect("original set survives rename");
        assert_eq!(target.registry_identity(), Some(identity.as_str()));
        assert_eq!(target.kind, super::super::BindingKind::Builtin);
        assert_eq!(
            target
                .identity
                .as_ref()
                .and_then(|identity| identity.runtime),
            Some(expected_token)
        );

        // A real selected body still requires its original caller frame.
        let source = "uplevel 1 {rename ::set ::saved}; saved final 1";
        let analysis = analyse_native(source);
        let offset = u32::try_from(source.find("saved final").unwrap()).unwrap();
        assert!(
            analysis
                .invocation_at_source("saved", offset)
                .proved_execution_target()
                .is_none()
        );
    }

    #[test]
    fn named_private_invocation_resolves_after_argument_mutations() {
        let source = "info body [proc ::tcl::info::body {name} {rename ::set ::saved;return PRIVATE}]; saved final 1";
        let analysis = analyse(source);
        let binding = analysis.invocation_at_source("info", 0);
        let selected = binding
            .named_invocation()
            .expect("compiler selected a private command name");
        assert_eq!(selected.captured_name(), "::tcl::info::body");
        assert_eq!(selected.arguments_from, 1);
        assert!(
            selected
                .proved_target()
                .is_some_and(|target| !target.registry_backed)
        );
        assert_eq!(
            binding.compiled_candidates,
            [] as [crate::command_binding::SourceCompiledInvocationProof; 0]
        );
        let final_at = u32::try_from(source.rfind("saved final").unwrap()).unwrap();
        assert!(
            analysis
                .invocation_at_source("saved", final_at)
                .proved_execution_target()
                .is_some_and(|target| target.registry_backed && target.command == "::set")
        );
    }

    #[test]
    fn named_private_invocation_does_not_dispatch_a_replaced_public_head() {
        let source = "proc P {} {}; info body [proc info {args} {rename set saved;return CUSTOM};set name P]; set final 1";
        let analysis = analyse(source);
        let offset = u32::try_from(source.find("info body").unwrap()).unwrap();
        let binding = analysis.invocation_at_source("info", offset);
        assert!(
            binding
                .proved_target()
                .is_some_and(|target| !target.registry_backed)
        );
        let selected = binding
            .named_invocation()
            .expect("private name retained despite public replacement");
        assert!(
            selected
                .proved_target()
                .is_some_and(|target| target.registry_backed)
        );
        let final_at = u32::try_from(source.rfind("set final").unwrap()).unwrap();
        assert!(
            analysis
                .invocation_at_source("set", final_at)
                .proved_execution_target()
                .is_some_and(|target| target.registry_backed)
        );
    }
}
