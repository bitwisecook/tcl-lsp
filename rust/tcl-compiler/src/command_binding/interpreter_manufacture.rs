// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Normal-edge parent command installation by the selected interpreter factory.

use super::{BindingKind, MayBinding, ModuleCommandBindings, ResolvedCommandTarget};

impl ModuleCommandBindings {
    pub(super) fn retain_created_interpreter_command(
        &mut self,
        transition: &tcl_registry::InterpreterTransition,
        declaration: u32,
    ) {
        let Some(name) = self
            .baseline
            .dialect
            .and_then(|dialect| transition.created_parent_command(dialect))
        else {
            return;
        };
        let Some(slot) = super::qualify_execution_name(
            &crate::ir_helpers::ExecutionNamespace::exact("::"),
            &name,
        ) else {
            return;
        };
        self.install(
            slot.clone(),
            MayBinding::Target(ResolvedCommandTarget {
                command: slot,
                prepended: Vec::new(),
                registry_backed: false,
                kind: BindingKind::Command,
                implementation_generation: declaration,
                implementation_allocation: None,
                terminal: true,
                target_lookup: tcl_registry::AliasTargetLookup::Global,
                token: None,
            }),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::super::{SourceAnalysisOptions, SourceCommandBindings, SourceCommandSlotPresence};

    #[test]
    fn interpreter_factory_installs_only_the_actual_parent_slot() {
        for version in tcl_dialect::TclVersion::ALL {
            let owner =
                tcl_registry::model::ingress::static_context_for(version.dialect_profile_name());
            let registry = owner.commands();
            let source = "namespace eval n {interp create -safe sandbox}; sandbox eval {set x 1}";
            let bindings = SourceCommandBindings::analyse_with_options(
                source,
                tcl_lexer::LexerConfig::for_profile(registry.profile()),
                registry,
                SourceAnalysisOptions {
                    invocation_dialect: Some(tcl_registry::InvocationDialect::for_version(version)),
                    native_compilation:
                        tcl_registry::native_compilation::NativeCompilationContext {
                            mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                            ..Default::default()
                        },
                    ..Default::default()
                },
            );
            let offset = u32::try_from(source.rfind("sandbox eval").unwrap()).unwrap();
            let call = bindings.invocation_at_source("sandbox", offset);
            assert_eq!(
                call.selected_slot_presence(),
                SourceCommandSlotPresence::Present,
                "{version:?}: {call:#?}"
            );
            let target = call.proved_target().expect("actual child command slot");
            assert_eq!(target.command, "::sandbox");
            assert_eq!(target.kind, super::BindingKind::Command);
            assert!(
                target
                    .identity
                    .as_ref()
                    .and_then(|identity| identity.allocation.as_ref())
                    .is_some()
            );
            assert!(call.command_reference("::n::sandbox").is_none());
            assert!(
                call.command_reference("sandbox")
                    .and_then(|reference| reference.definition().cloned())
                    .is_none()
            );
        }
    }
}
