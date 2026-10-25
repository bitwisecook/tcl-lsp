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

//! Original readonly layouts for genuinely installed called statics.

use super::{CommandRegistry, ModuleCommandBindings, SharedSourceInventory, SourceCommandBindings};

impl SourceCommandBindings {
    pub(super) fn retain_called_static_declaration_layouts(
        &mut self,
        incoming: &ModuleCommandBindings,
        config: tcl_lexer::LexerConfig,
        registry: &CommandRegistry,
    ) {
        if !self.deferred.values().any(|root| {
            root.statics.is_some()
                && self
                    .called_implementations
                    .contains(&root.implementation_id())
        }) {
            return;
        }
        let entries = self
            .deferred_entry_states(incoming)
            .into_iter()
            .filter(|(root, _)| {
                self.called_implementations
                    .contains(&root.implementation_id())
                    && !root.receiver_method
                    && root.event.is_none()
                    && root.future_frame.is_none()
                    && root.statics.is_some()
            })
            .collect::<Vec<_>>();
        if entries.is_empty() {
            return;
        }
        // The same genuine body/frame producer supplies only readonly layouts.
        // Its results and all entered inventories remain in this private clone.
        let mut conditional = self.clone();
        conditional.conditional_procedure_results = SharedSourceInventory::default();
        conditional.active_calls.clear();
        conditional.active_variable_observers.clear();
        for (root, entry) in entries {
            conditional.walk_deferred_entry(&root, &entry, config, registry);
        }
        self.merge_original_declaration_layouts(&conditional);
    }
}

#[cfg(test)]
mod tests {
    use super::super::declaration_layout::{
        DeclarationLayoutIssuer, OriginalDiagnosticFrameEntry, original_declaration_layouts,
    };
    use super::*;
    use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};

    fn input() -> crate::analyser::ResolvedAnalysisInput {
        tcl_spectcl::core_surfaces::ensure();
        crate::analyser::Analyser::new()
            .prepare_analysis_input("jim")
            .unwrap()
    }

    fn unit(source: &str, input: &crate::analyser::ResolvedAnalysisInput) -> CompilationUnit {
        CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry: input.borrowed_context_registry().commands(),
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(input.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            input,
        )
    }

    #[test]
    fn called_static_body_keeps_a_separate_original_declaration_layout() {
        let input = input();
        let registry = input.borrowed_context_registry().commands();
        let source = "proc f {} {{n 0}} {return $n}; f";
        let unit = unit(source, &input);
        let bindings = unit
            .ir_module
            .retained_source_bindings
            .as_deref()
            .unwrap()
            .positioned_for_module(&unit.ir_module, registry)
            .unwrap();
        let offset = u32::try_from(source.find("return").unwrap()).unwrap();
        let site = super::super::CommandAllocationSite {
            source: super::super::Arc::clone(bindings.source_origin().unwrap()),
            offset,
        };
        let observations = bindings.declaration_layouts.get(&site).unwrap();
        assert!(
            observations
                .iter()
                .any(|row| row.issuer == DeclarationLayoutIssuer::EnteredActivation)
        );
        let originals = original_declaration_layouts(observations).unwrap();
        let original = originals.clone().next().unwrap();
        assert!(matches!(
            original.entry.as_ref(),
            OriginalDiagnosticFrameEntry::Body(_)
        ));
        assert!(
            originals
                .clone()
                .all(|row| row.issuer == DeclarationLayoutIssuer::OriginalDeclaration)
        );
        assert_eq!(
            original.snapshot.state.variable_frame,
            *original.entry.frame()
        );
        assert!(original.entry.owns_source(&site.source, offset));
    }

    #[test]
    fn called_static_layout_collection_does_not_publish_results_or_entered_inventories() {
        let input = input();
        let registry = input.borrowed_context_registry().commands();
        let source = "proc f {} {{n 0}} {return $n}; f";
        let unit = unit(source, &input);
        let mut before = unit
            .ir_module
            .retained_source_bindings
            .as_deref()
            .unwrap()
            .positioned_for_module(&unit.ir_module, registry)
            .unwrap()
            .clone();
        let offset = u32::try_from(source.find("return").unwrap()).unwrap();
        let site = super::super::CommandAllocationSite {
            source: super::super::Arc::clone(before.source_origin().unwrap()),
            offset,
        };
        before
            .declaration_layouts
            .get_mut(&site)
            .unwrap()
            .retain(|row| row.issuer != DeclarationLayoutIssuer::OriginalDeclaration);
        assert!(
            original_declaration_layouts(before.declaration_layouts.get(&site).unwrap()).is_none()
        );
        let incoming = super::super::Arc::clone(&before.final_state);
        let mut after = before.clone();
        after.retain_called_static_declaration_layouts(&incoming, input.lexer_config(), registry);
        assert!(
            original_declaration_layouts(after.declaration_layouts.get(&site).unwrap()).is_some()
        );
        after.declaration_layouts = before.declaration_layouts.clone();
        assert!(
            after == before,
            "only original declaration layouts may be published"
        );
    }

    #[test]
    fn static_layout_collection_does_not_reinterpret_other_callable_purposes() {
        let input = input();
        let registry = input.borrowed_context_registry().commands();
        for source in [
            "proc f {} {return 7}; f",
            "proc f {} {{n 0}} {return $n}",
            "proc f {} {missing} {return $missing}; f",
        ] {
            let unit = unit(source, &input);
            let before = unit
                .ir_module
                .retained_source_bindings
                .as_deref()
                .unwrap()
                .positioned_for_module(&unit.ir_module, registry)
                .unwrap()
                .clone();
            let incoming = super::super::Arc::clone(&before.final_state);
            let mut after = before.clone();
            after.retain_called_static_declaration_layouts(
                &incoming,
                input.lexer_config(),
                registry,
            );
            assert!(after == before, "no called static producer: {source}");
        }
    }
}
