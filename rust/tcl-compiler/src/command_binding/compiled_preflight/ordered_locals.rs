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
    formals: SourceCompilerFormalOwner,
    locals: Vec<Option<NameBytes>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum SourceCompilerFormalOwner {
    Authored(Arc<OriginalFormalTopology>),
    Native(Arc<crate::command_binding::OriginalNativeCompilerFormalScope>),
}
impl SourceCompilerFormalOwner {
    fn is_live(&self) -> bool {
        match self {
            Self::Authored(_) => true,
            Self::Native(scope) => scope.is_live(),
        }
    }
    fn owns_source(
        &self,
        site: &CommandAllocationSite,
        frame: &crate::var_resolve::VariableExecutionFrame,
        config: tcl_lexer::LexerConfig,
    ) -> bool {
        match self {
            Self::Authored(_) => true,
            Self::Native(scope) => scope.owns_source(site, frame, config),
        }
    }
}

impl SourceCompilerLocalInventory {
    pub(super) fn at_entry(
        chunk: CommandAllocationSite,
        length: usize,
        state: &crate::command_binding::ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        if context.compilation().mode != NativeCompilationMode::BytecodeObject
            || context.compilation().frame != NativeCompilationFrame::ProcedureCode
            || state.variable_frame != *context.frame
        {
            return None;
        }
        let formals =
            if let Some(original) = state.source_variables.original_formal_topology.as_ref() {
                let original = Arc::clone(original);
                if original.original_input().source_image() != chunk.source.source_image()
                    || original.original_input().lexer_config() != context.config
                    || original.parameters().len() > MAX_SOURCE_COMPILER_LOCALS
                {
                    return None;
                }
                SourceCompilerFormalOwner::Authored(original)
            } else {
                SourceCompilerFormalOwner::Native(Arc::clone(
                    state
                        .source_variables
                        .original_native_compiler_formals
                        .as_ref()?,
                ))
            };
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
        let locals = match &formals {
            SourceCompilerFormalOwner::Authored(original) => original
                .parameters()
                .iter()
                .map(|formal| Some(NameBytes::from(formal.name.as_slice())))
                .collect(),
            SourceCompilerFormalOwner::Native(scope) => scope
                .parameters_for_entry(&chunk, length, state, context, protocol)?
                .iter()
                .cloned()
                .map(Some)
                .collect(),
        };
        Some(Self {
            end: chunk.offset.checked_add(u32::try_from(length).ok()?)?,
            chunk,
            config: context.config,
            frame: context.frame.clone(),
            compilation: context.compilation(),
            protocol,
            formals,
            locals,
        })
    }

    /// Preserve only already-issued Native slots for a complete arithmetic
    /// program. This issues no new local, runtime read, value or effect fact.
    pub(super) fn retains_existing_expression(
        &self,
        program: &tcl_registry::native_expression_program::NativeExpressionProgram,
        site: &CommandAllocationSite,
        config: tcl_lexer::LexerConfig,
        frame: &crate::var_resolve::VariableExecutionFrame,
        compilation: NativeCompilationContext,
    ) -> bool {
        use tcl_registry::native_expression_program::NativeExpressionTree;
        use tcl_syntax::expr::parser::NativeExprSyntax;
        use tcl_syntax::expr::{BinOp, ExprNode};
        use tcl_syntax::naming::{NativeCompiledVariableAuthority, NativeCompiledVariableRecipe};
        if !matches!(
            (self.protocol.recipe(), program.context.native_syntax),
            (NativeCompiledVariableRecipe::C(version), NativeExprSyntax::Tcl(parsed))
                if version == parsed
                    && matches!(version, tcl_dialect::TclVersion::V8_6 | tcl_dialect::TclVersion::V9_0)
        ) {
            return false;
        }
        if self.protocol.authority() != NativeCompiledVariableAuthority::Native
            || !matches!(self.formals, SourceCompilerFormalOwner::Native(_))
            || !self.owns(site, config, frame, compilation, self.protocol)
            || self
                .chunk
                .source
                .source_image()
                .bytes()
                .get(program.span.as_range())
                != Some(program.source.as_slice())
        {
            return false;
        }
        let NativeExpressionTree::Parsed(tree) = &program.tree else {
            return false;
        };
        let mut pending = vec![tree];
        while let Some(node) = pending.pop() {
            match node {
                ExprNode::Var { text, name, .. }
                    if text.strip_prefix(b"$") == Some(name.as_slice())
                        && self.protocol.substitution_lookup(
                            name,
                            true,
                            NativeCompiledVariableEnvironment::DeclareProcedure,
                        ) == NativeCompiledVariableLookup::CreateLocal
                        && self.primary_for(name, compilation).is_some() => {}
                ExprNode::Binary {
                    op: BinOp::Add | BinOp::Sub | BinOp::Mul,
                    left,
                    right,
                } => {
                    pending.push(right);
                    pending.push(left);
                }
                _ => return false,
            }
        }
        true
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
            && self.formals.owns_source(site, frame, config)
    }

    /// The first counted compiler entry, with the selected engine comparison.
    /// Only the small-index prefix is exposed; larger slots require the
    /// selected instruction's independent large-index fallback recipe.
    pub(crate) fn primary_for(
        &self,
        requested: &[u8],
        compilation: NativeCompilationContext,
    ) -> Option<&NameBytes> {
        (self.formals.is_live()
            && compilation.mode == self.compilation.mode
            && compilation.frame == self.compilation.frame)
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

#[cfg(test)]
mod original_native_request_fixtures {
    use crate as tcl_compiler;
    include!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/support/native_compiled_fixture.rs"
    ));
}

#[cfg(test)]
mod original_native_request_tests {
    use super::*;
    use std::hash::{Hash, Hasher};

    fn retained_locals(
        fixture: &super::original_native_request_fixtures::CompiledFixture,
    ) -> SourceCompilerLocalInventory {
        let module = fixture.module();
        assert!(module.source_metadata_input.is_none());
        assert!(
            module
                .source_entry
                .native_entry
                .as_ref()
                .unwrap()
                .compiled_local_layout
                .is_none()
        );
        let retained = module.retained_source_bindings.as_ref().unwrap();
        let bindings = retained
            .positioned_for_module(module, module.resolved_registry())
            .unwrap();
        let visits: Vec<_> = bindings.compiler_invocations.values().flatten().collect();
        assert!(!visits.is_empty());
        assert!(visits.iter().all(|visit| {
            visit
                .table
                .state
                .source_variables
                .original_formal_topology
                .is_none()
        }));
        visits
            .iter()
            .filter_map(|visit| visit.source_locals.as_deref())
            .filter(|locals| matches!(locals.formals, SourceCompilerFormalOwner::Native(_)))
            .max_by_key(|locals| locals.locals.len())
            .expect("actual request-specific compiler local traversal")
            .clone()
    }

    fn assert_original_expression_boundaries(
        fixture: &super::original_native_request_fixtures::CompiledFixture,
        locals: &SourceCompilerLocalInventory,
    ) {
        use tcl_registry::native_expression_program::NativeExpressionTree;
        use tcl_registry::native_instruction_plan::NativeInstructionPlan;
        let module = fixture.module();
        let bindings = module
            .retained_source_bindings
            .as_ref()
            .unwrap()
            .positioned_for_module(module, module.resolved_registry())
            .unwrap();
        let (site, program) = bindings
            .compiler_invocations
            .values()
            .flatten()
            .find_map(|visit| {
                let preparation = visit.structured.as_deref()?;
                let NativeInstructionPlan::Expression(expression) = &preparation.recipe else {
                    return None;
                };
                Some((&preparation.compilation_site, expression.program.as_ref()?))
            })
            .expect("actual selected original expression preparation");
        let accepts =
            |inventory: &SourceCompilerLocalInventory,
             program: &tcl_registry::native_expression_program::NativeExpressionProgram,
             site: &CommandAllocationSite| {
                inventory.retains_existing_expression(
                    program,
                    site,
                    locals.config,
                    &locals.frame,
                    locals.compilation,
                )
            };
        assert!(accepts(locals, program, site));
        let mut missing = locals.clone();
        missing.locals.clear();
        assert!(
            !accepts(&missing, program, site),
            "no new slot may be minted"
        );
        let mut foreign = site.clone();
        foreign.source = Arc::new(site.source.as_ref().clone());
        assert!(!accepts(locals, program, &foreign));
        let mut changed = program.clone();
        changed.source.push(b' ');
        assert!(!accepts(locals, &changed, site));
        let mut different_recipe = program.clone();
        different_recipe.context.native_syntax =
            tcl_syntax::expr::parser::NativeExprSyntax::Unknown;
        assert!(!accepts(locals, &different_recipe, site));
        let mut rejected = program.clone();
        rejected.tree = NativeExpressionTree::Rejected {
            message: Vec::new(),
            error_code: None,
        };
        assert!(!accepts(locals, &rejected, site));
    }

    fn fingerprint(locals: &SourceCompilerLocalInventory) -> u64 {
        let mut state = std::collections::hash_map::DefaultHasher::new();
        locals.hash(&mut state);
        state.finish()
    }

    #[test]
    fn original_native_request_formals_keep_actual_scope_and_withdraw_after_owner_retirement() {
        let source = "proc p {a b} {set n 1; return [expr {$a + $b}]}";
        let mut fixture =
            super::original_native_request_fixtures::compiled_fixture(source, "::p", &["3", "4"]);
        let locals = retained_locals(&fixture);
        assert_original_expression_boundaries(&fixture, &locals);
        assert!(locals.owns(
            &locals.chunk,
            locals.config,
            &locals.frame,
            locals.compilation,
            locals.protocol
        ));
        for name in [b"a".as_slice(), b"b".as_slice(), b"n".as_slice()] {
            assert_eq!(
                locals
                    .primary_for(name, locals.compilation)
                    .unwrap()
                    .as_bytes(),
                name
            );
        }
        let (_, body) = fixture.body();
        assert!(body.params_raw.is_empty());
        let before = fingerprint(&locals);
        fixture.drop_owner();
        assert_eq!(
            fingerprint(&locals),
            before,
            "retirement cannot mutate an immutable key"
        );
        assert!(!locals.owns(
            &locals.chunk,
            locals.config,
            &locals.frame,
            locals.compilation,
            locals.protocol
        ));
        assert!(locals.primary_for(b"a", locals.compilation).is_none());
    }

    #[test]
    fn original_native_request_formals_refuse_foreign_scope_and_keep_existing_body_boundaries() {
        let source = "proc p {a b} {set n 1; return [expr {$a + $b}]}";
        let fixture =
            super::original_native_request_fixtures::compiled_fixture(source, "::p", &["3", "4"]);
        let foreign =
            super::original_native_request_fixtures::compiled_fixture(source, "::p", &["3", "4"]);
        let locals = retained_locals(&fixture);
        let other = retained_locals(&foreign);
        assert_ne!(locals.formals, other.formals);
        assert!(!locals.owns(
            &other.chunk,
            locals.config,
            &locals.frame,
            locals.compilation,
            locals.protocol
        ));
        let mut reboxed = locals.chunk.clone();
        reboxed.source = Arc::new(locals.chunk.source.as_ref().clone());
        assert!(!locals.owns(
            &reboxed,
            locals.config,
            &locals.frame,
            locals.compilation,
            locals.protocol
        ));
        let mut changed = locals.config;
        changed.leading_bom = if changed.leading_bom == tcl_lexer::LeadingBom::Content {
            tcl_lexer::LeadingBom::Skip
        } else {
            tcl_lexer::LeadingBom::Content
        };
        assert!(!locals.owns(
            &locals.chunk,
            changed,
            &locals.frame,
            locals.compilation,
            locals.protocol
        ));
        let mut module = fixture.module().clone();
        let mut changed_source = module.source.bytes().to_vec();
        changed_source.push(b' ');
        module.source = tcl_lexer::SourceImage::from_bytes(changed_source, module.source.channel());
        assert!(fixture.refuses_module(&module));
        assert!(fixture.refuses_foreign_entry(&foreign));
        super::original_native_request_fixtures::assert_declaration_and_refusal_boundaries();
        super::original_native_request_fixtures::assert_dispatched_binding_boundaries();
    }
}
