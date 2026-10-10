// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Ordered source compiler allocations, separate from physical frame layouts.

use super::{Arc, CommandAllocationSite, SourceExecutionContext};
use crate::command_binding::formal_topology::OriginalFormalTopology;
use tcl_core_types::NameBytes;
use tcl_registry::native_compilation::{
    NativeCompilationContext, NativeCompilationFrame, NativeCompilationMode,
};
use tcl_syntax::naming::{
    NativeCompiledVariableEnvironment, NativeCompiledVariableLookup, NativeCompiledVariableProtocol,
};

const MAX_SOURCE_COMPILER_LOCALS: usize = 4096;

/// A complete bounded traversal of one authentic procedure compilation.
/// Anonymous entries retain slot order. Named entries keep counted primary
/// bytes even when the selected compiler compares through an embedded NUL.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct SourceCompilerLocalInventory {
    chunk: CommandAllocationSite,
    end: u32,
    config: tcl_lexer::LexerConfig,
    frame: crate::var_resolve::VariableExecutionFrame,
    compilation: NativeCompilationContext,
    protocol: NativeCompiledVariableProtocol,
    formals: Arc<OriginalFormalTopology>,
    locals: Vec<Option<NameBytes>>,
}

impl SourceCompilerLocalInventory {
    pub(super) fn at_entry(
        chunk: CommandAllocationSite,
        length: usize,
        state: &crate::command_binding::ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        if context.compilation.mode != NativeCompilationMode::BytecodeObject
            || context.compilation.frame != NativeCompilationFrame::ProcedureCode
            || state.variable_frame != *context.frame
        {
            return None;
        }
        let formals = Arc::clone(state.source_variables.original_formal_topology.as_ref()?);
        if formals.original_input().source_image() != chunk.source.source_image()
            || formals.original_input().lexer_config() != context.config
            || formals.parameters().len() > MAX_SOURCE_COMPILER_LOCALS
        {
            return None;
        }
        let options = crate::command_binding::SourceAnalysisOptions {
            native_entry: state.baseline.native_entry.as_deref(),
            invocation_dialect: state.baseline.dialect,
            compiled_variable_provider: state.baseline.compiled_variable_provider,
            ..Default::default()
        };
        let protocol = options.compiled_variable_protocol()?;
        protocol
            .supports_environment(NativeCompiledVariableEnvironment::DeclareProcedure)
            .then_some(())?;
        let locals = formals
            .parameters()
            .iter()
            .map(|formal| Some(NameBytes::from(formal.name.as_slice())))
            .collect();
        Some(Self {
            end: chunk.offset.checked_add(u32::try_from(length).ok()?)?,
            chunk,
            config: context.config,
            frame: context.frame.clone(),
            compilation: context.compilation,
            protocol,
            formals,
            locals,
        })
    }

    pub(super) fn declare(&mut self, name: Option<&[u8]>) -> bool {
        if let Some(name) = name
            && self.locals.iter().flatten().any(|existing| {
                self.protocol
                    .compiled_local_names_equal(existing.as_bytes(), name)
            })
        {
            return true;
        }
        if self.locals.len() >= MAX_SOURCE_COMPILER_LOCALS {
            return false;
        }
        self.locals.push(name.map(NameBytes::from));
        true
    }

    pub(super) fn command_name(&mut self, name: &[u8]) -> bool {
        let lookup = self
            .protocol
            .command_lookup(name, NativeCompiledVariableEnvironment::DeclareProcedure);
        lookup != NativeCompiledVariableLookup::CreateLocal || self.declare(Some(name))
    }

    pub(super) fn substitution_name(&mut self, name: &[u8], separate_index: bool) -> bool {
        let lookup = self.protocol.substitution_lookup(
            name,
            !separate_index,
            NativeCompiledVariableEnvironment::DeclareProcedure,
        );
        lookup != NativeCompiledVariableLookup::CreateLocal || self.declare(Some(name))
    }

    /// Match the retained source owner before exposing its first compiler
    /// primary. Runtime local-cache comparison and physical layouts differ.
    pub(crate) fn owns(
        &self,
        site: &CommandAllocationSite,
        config: tcl_lexer::LexerConfig,
        frame: &crate::var_resolve::VariableExecutionFrame,
        compilation: NativeCompilationContext,
        protocol: NativeCompiledVariableProtocol,
    ) -> bool {
        site.source == self.chunk.source
            && self.chunk.offset <= site.offset
            && site.offset < self.end
            && config == self.config
            && frame == &self.frame
            && protocol == self.protocol
            && compilation.mode == self.compilation.mode
            && compilation.frame == self.compilation.frame
    }

    /// The first counted compiler entry, with the selected engine comparison.
    /// Only the small-index prefix is exposed; larger slots require the
    /// selected instruction's independent large-index fallback recipe.
    pub(crate) fn primary_for(
        &self,
        requested: &[u8],
        compilation: NativeCompilationContext,
    ) -> Option<&NameBytes> {
        (compilation.mode == self.compilation.mode && compilation.frame == self.compilation.frame)
            .then_some(())?;
        let (slot, primary) = self.locals.iter().enumerate().find_map(|(slot, local)| {
            let local = local.as_ref()?;
            self.protocol
                .compiled_local_names_equal(local.as_bytes(), requested)
                .then_some((slot, local))
        })?;
        // This common facade does not select a particular instruction's large
        // local-index fallback. The small prefix has no such ambiguity.
        u8::try_from(slot).is_ok().then_some(primary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};

    fn inventory(source: &[u8]) -> SourceCommandBindings {
        inventory_in(source, "tcl8.6")
    }

    fn inventory_in(source: &[u8], engine: &str) -> SourceCommandBindings {
        let selected = tcl_registry::model::ingress::static_context_for(engine);
        let registry = selected.commands();
        let profile = registry.profile().unwrap();
        SourceCommandBindings::analyse_image_in_frame_with_options(
            &tcl_lexer::SourceImage::native(source),
            &crate::var_resolve::VariableExecutionFrame::Global,
            tcl_lexer::LexerConfig::from_grammar(profile.grammar),
            registry,
            SourceAnalysisOptions {
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: NativeCompilationContext {
                    mode: NativeCompilationMode::BytecodeObject,
                    frame: NativeCompilationFrame::ScriptCode,
                    ..Default::default()
                },
                ..Default::default()
            },
        )
        .unwrap()
    }

    #[test]
    fn ordered_source_locals_keep_counted_primary_and_namespace_attempts() {
        // Implementation contract: naming.source.ordered-compiler-local-inventory
        // docs/design/analysis/name-resolution-proofs/ordered-compiler-local-inventory.md
        let source = b"proc p {formal} {set k\0a 1; set k\0b 2; global g; set g 3}; p VALUE";
        let bindings = inventory(source);
        let local = bindings
            .compiler_invocations
            .values()
            .flatten()
            .find_map(|invocation| invocation.source_locals.as_deref())
            .expect("complete authentic procedure compilation");
        assert_eq!(local.locals[0].as_ref().unwrap().as_bytes(), b"formal");
        assert_eq!(
            local
                .primary_for(b"k\0b", local.compilation)
                .unwrap()
                .as_bytes(),
            b"k\0a"
        );
        assert!(local.primary_for(b"k\0bb", local.compilation).is_none());
        assert!(local.primary_for(b"g", local.compilation).is_some());
        assert!(
            local
                .primary_for(
                    b"k\0b",
                    NativeCompilationContext {
                        mode: NativeCompilationMode::Direct,
                        ..local.compilation
                    }
                )
                .is_none()
        );
        let mut foreign = local.chunk.clone();
        foreign.source = Arc::new(crate::command_binding::SourceOriginId::authored_image(
            tcl_lexer::SourceImage::document(std::str::from_utf8(source).unwrap()),
        ));
        assert!(!local.owns(
            &foreign,
            local.config,
            &local.frame,
            local.compilation,
            local.protocol
        ));
    }

    #[test]
    fn source_locals_retain_recursive_word_allocations_and_anonymous_slot_order() {
        // Implementation contract: naming.source.ordered-compiler-local-inventory
        // docs/design/analysis/name-resolution-proofs/ordered-compiler-local-inventory.md
        let recursive = inventory(b"proc p {} {set [set k\0a first] 1; set k\0b 2}; p");
        let local = recursive
            .compiler_invocations
            .values()
            .flatten()
            .find_map(|invocation| invocation.source_locals.as_deref())
            .expect("original nested word compiles before its outer receiver");
        assert_eq!(
            local
                .primary_for(b"k\0b", local.compilation)
                .unwrap()
                .as_bytes(),
            b"k\0a"
        );
        for (engine, anonymous) in [("tcl8.5", true), ("tcl8.6", false)] {
            let each = inventory_in(
                b"proc p {} {foreach item {A B} {set k\0a $item}; set k\0b 2}; p",
                engine,
            );
            let local = each
                .compiler_invocations
                .values()
                .flatten()
                .find_map(|invocation| invocation.source_locals.as_deref())
                .expect("Registry-selected recursive control preparations");
            // C8.5 reserves foreach temporaries in the local table; C8.6
            // retains its independent stack-based preparation instead.
            assert_eq!(
                local.locals.iter().any(Option::is_none),
                anonymous,
                "{engine}"
            );
            assert_eq!(
                local
                    .primary_for(b"k\0b", local.compilation)
                    .unwrap()
                    .as_bytes(),
                b"k\0a"
            );
            let mut foreign_config = local.config;
            foreign_config.strict_quoting = !foreign_config.strict_quoting;
            assert!(!local.owns(
                &local.chunk,
                foreign_config,
                &local.frame,
                local.compilation,
                local.protocol
            ));
        }
    }

    #[test]
    fn source_locals_preserve_partial_namespace_allocations_and_withdraw_missing_attempts() {
        // Implementation contract: naming.source.ordered-compiler-local-inventory
        // docs/design/analysis/name-resolution-proofs/ordered-compiler-local-inventory.md
        let partial = inventory(b"proc p {} {global first {} later; set k\0a 1; set k\0b 2}; p");
        let local = partial
            .compiler_invocations
            .values()
            .flatten()
            .find_map(|invocation| invocation.source_locals.as_deref())
            .expect("Registry retains the declined namespace compiler prefix");
        assert_eq!(local.locals[0].as_ref().unwrap().as_bytes(), b"first");
        assert_eq!(
            local
                .primary_for(b"k\0b", local.compilation)
                .unwrap()
                .as_bytes(),
            b"k\0a"
        );
        let missing =
            inventory(b"proc p {} {upvar 1 outer first other ::bad; set k\0a 1; set k\0b 2}; p");
        assert!(
            missing
                .compiler_invocations
                .values()
                .flatten()
                .all(|invocation| invocation.source_locals.is_none())
        );
    }
}
