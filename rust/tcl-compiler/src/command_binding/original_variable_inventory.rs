// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original variable operands retained at their actual handler boundary.

use super::{
    Arc, CommandAllocationSite, SourceCommandBindings, SourceInvocationBinding, SourceOriginId,
};
use crate::variable_bindings::OriginalVariableInvocation;
use std::collections::BTreeMap;

pub(super) type OriginalVariableInvocations =
    BTreeMap<CommandAllocationSite, Option<Arc<OriginalVariableInvocation>>>;

impl SourceCommandBindings {
    pub(super) fn retain_original_variable_operands(
        &mut self,
        origin: Option<&Arc<SourceOriginId>>,
        offset: u32,
        operands: &OriginalVariableInvocation,
    ) {
        let Some(origin) = origin else {
            return;
        };
        self.original_variable_invocations
            .entry(CommandAllocationSite {
                source: Arc::clone(origin),
                offset,
            })
            .and_modify(|previous| {
                if previous.as_deref() != Some(operands) {
                    *previous = None;
                }
            })
            .or_insert_with(|| Some(Arc::new(operands.clone())));
    }

    pub(super) fn attach_original_variable_operands(
        &self,
        mut binding: SourceInvocationBinding,
        origin: Option<&Arc<SourceOriginId>>,
        offset: u32,
    ) -> SourceInvocationBinding {
        binding.original_variable_operands = origin.and_then(|origin| {
            self.original_variable_invocations
                .get(&CommandAllocationSite {
                    source: Arc::clone(origin),
                    offset,
                })
                .cloned()
                .flatten()
        });
        binding
    }
}

impl SourceInvocationBinding {
    pub(crate) fn original_variable_operands_for_tokens(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<&Arc<OriginalVariableInvocation>> {
        self.original_lexer_config_for_tokens(tokens)?;
        self.original_variable_operands.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    // Implementation contract: naming.variable.original-ssa-definition-operand
    // docs/design/analysis/name-resolution-proofs/original-ssa-definition-operand.md
    fn original_operand_inventory_keeps_runtime_bytes_and_withdraws_missing_or_conflicting_owners()
    {
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::from_grammar(profile.grammar);
        let source = r"set v\uD800 42; set v\uD801 OTHER";
        let mut bindings = SourceCommandBindings::analyse_with_options(
            source,
            config,
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let segments = crate::segmenter::segment_commands_with_offset_and_config(source, 0, config);
        let tokens = |index: usize| {
            let segment = &segments[index];
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceMap::new(source),
                config,
                segment,
            );
            tokens.source_binding =
                Some(bindings.invocation_at_source("set", segment.span.start()));
            tokens
        };
        let first = tokens(0);
        let second = tokens(1);
        let left = first.source_binding.as_ref().unwrap();
        let right = second.source_binding.as_ref().unwrap();
        let left_operands = left.original_variable_operands_for_tokens(&first).unwrap();
        let right_operands = right
            .original_variable_operands_for_tokens(&second)
            .unwrap();
        assert_ne!(
            left_operands
                .input(0, &left.variable_context)
                .unwrap()
                .bytes(),
            right_operands
                .input(0, &right.variable_context)
                .unwrap()
                .bytes()
        );
        let normal =
            crate::registry_invocation::normal_transfer_invocation(registry, None, &first).unwrap();
        let definitions = normal.written_definition_places(&left.variable_context, registry);
        assert_eq!(definitions.len(), 1);
        assert_eq!(definitions[0].0, 0);
        assert!(crate::var_resolve::canonical_binding_value_key(&definitions[0].1).is_some());

        let mut missing = first.clone();
        missing
            .source_binding
            .as_mut()
            .unwrap()
            .original_variable_operands = None;
        assert!(
            crate::registry_invocation::normal_transfer_invocation(registry, None, &missing)
                .unwrap()
                .written_definition_places(&left.variable_context, registry)
                .is_empty()
        );

        let origin = bindings.root_origin.clone().unwrap();
        bindings.retain_original_variable_operands(Some(&origin), 0, right_operands);
        assert!(
            bindings
                .invocation_at_source("set", 0)
                .original_variable_operands
                .is_none()
        );
        bindings.retain_original_variable_operands(Some(&origin), 0, left_operands);
        assert!(
            bindings
                .invocation_at_source("set", 0)
                .original_variable_operands
                .is_none()
        );
    }
}
