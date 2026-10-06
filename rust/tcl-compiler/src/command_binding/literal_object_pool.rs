// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Ordinary literal objects in an actually fresh source interpreter.
//!
//! `TclRegisterLiteral` can reuse the same object across separately compiled
//! bodies. A host command can replace its intrep with an abstract-list type;
//! unchanged bytes then prove neither its type nor its method effects.
//! An actual empty registration-world observation supplies only effect closure
//! for original literals; stock class, contents and numeric caches remain separate.
//! Provenance intersects on joins and is never recreated after unknown effects,
//! even when a later constructor creates a new value epoch.

use super::{
    Arc, ModuleCommandBindings, SourceAnalysisOptions, SourceExecutionContext,
    SourceNativeInvocation, SourceOriginId,
};

/// Authored fresh-pool provenance or a same-capture empty native world.
/// No unenumerated host/object effects have withdrawn the original effect proof.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum LiteralPoolOrigin {
    Authored,
    Native(tcl_runtime_api::native_literal::NativeEmptyLiteralWorld),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct SourceOrdinaryLiteralPool {
    entry: Arc<SourceOriginId>,
    origin: LiteralPoolOrigin,
    numeric_representations_unmodified: bool,
    integer_contents:
        Option<tcl_registry::native_numeric_conversion::NativeStockIntegerContentsProtocol>,
}

impl SourceOrdinaryLiteralPool {
    pub(super) fn at_entry(
        state: &ModuleCommandBindings,
        options: SourceAnalysisOptions<'_>,
    ) -> Option<Self> {
        if options.unknown_entry
            || state.opaque_binding_mutation
            || options.invocation_dialect?.family() != Some(tcl_dialect::model::Family::Tcl)
        {
            return None;
        }
        let entry = state.current_source_origin.as_ref()?;
        if !matches!(entry.kind(), super::SourceOriginKind::Authored(_)) {
            return None;
        }
        let origin = Self::entry_origin(state, options)?;
        let authored = matches!(origin, LiteralPoolOrigin::Authored);
        Some(Self {
            entry: Arc::clone(entry),
            origin,
            numeric_representations_unmodified: authored,
            integer_contents: authored.then(|| {
                tcl_registry::native_numeric_conversion::NativeStockIntegerContentsProtocol::select(
                    options.invocation_dialect.expect("checked invocation dialect"),
                )
            }).flatten(),
        })
    }

    fn entry_origin(
        state: &ModuleCommandBindings,
        options: SourceAnalysisOptions<'_>,
    ) -> Option<LiteralPoolOrigin> {
        match (options.native_entry, state.baseline.native_entry.as_deref()) {
            (None, None) => Some(LiteralPoolOrigin::Authored),
            (Some(captured), Some(original)) if captured == original => {
                // Exact physical string policy remains independent of logical
                // simulation. An empty Jim/modelled table supplies no C pool.
                if !matches!(
                    original.source_string_protocol,
                    Some(tcl_syntax::native_string::NativeStringProtocol::C(_))
                ) {
                    return None;
                }
                let receipt = original.empty_literal_world.as_ref()?;
                receipt
                    .is_current_for(original.interpreter, original.epoch)
                    .then(|| LiteralPoolOrigin::Native(receipt.clone()))
            }
            _ => None,
        }
    }

    pub(super) const fn authored_objects(&self) -> bool {
        matches!(self.origin, LiteralPoolOrigin::Authored)
    }

    pub(super) fn effects_current(&self) -> bool {
        match &self.origin {
            LiteralPoolOrigin::Authored => true,
            LiteralPoolOrigin::Native(receipt) => receipt.is_current(),
        }
    }

    pub(super) const fn initial_numeric_representations(&self) -> bool {
        self.numeric_representations_unmodified
    }

    pub(super) fn withdraw_numeric_representations(&mut self) {
        self.numeric_representations_unmodified = false;
    }

    pub(super) fn joined(&self, other: &Self) -> Option<Self> {
        (self.entry == other.entry
            && self.origin == other.origin
            && self.effects_current()
            && other.effects_current()
            && self.integer_contents == other.integer_contents)
            .then(|| Self {
                entry: Arc::clone(&self.entry),
                origin: self.origin.clone(),
                integer_contents: self.integer_contents,
                numeric_representations_unmodified: self.numeric_representations_unmodified
                    && other.numeric_representations_unmodified,
            })
    }

    pub(super) fn note_command_lookup(
        &mut self,
        head: &str,
        dialect: tcl_registry::InvocationDialect,
    ) {
        // Normal numeric formatter output is in the selected numeric parser's
        // language. A CmdName conversion of such bytes can overwrite a shared
        // numeric pool object; other command bytes are disjoint from it.
        if tcl_syntax::number::parse_whole_with(
            head,
            tcl_syntax::number::ParseFlags::for_syntax(dialect.lexer_grammar.numbers),
        )
        .is_some()
        {
            self.withdraw_numeric_representations();
        }
    }
}

/// Effect-only proof for the original frozen literal operand. It licenses no
/// list representation, list validity, result, completion, or native opcode.
pub(super) struct SourceOrdinaryLiteralObject;

/// Stock class and cache/string consistency from an original fresh-pool literal.
/// No current representation, contents, numeric value or unique allocation is implied.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceStockLiteralObject {
    representation: tcl_syntax::value::ValueRepresentation,
    alternatives: Option<crate::native_numeric::ClosedContainerRepresentations>,
    integer_contents:
        Option<tcl_registry::native_numeric_conversion::NativeStockIntegerContentsProtocol>,
}

impl SourceStockLiteralObject {
    pub(crate) fn accepts_integer_contents(
        self,
        text: &str,
        dialect: tcl_registry::InvocationDialect,
    ) -> bool {
        self.integer_contents.is_some_and(|recipe| {
            Some(recipe) == tcl_registry::native_numeric_conversion::NativeStockIntegerContentsProtocol::select(dialect)
                && recipe.accepts_integer_contents(text)
        })
    }
    pub(crate) const fn representation(self) -> tcl_syntax::value::ValueRepresentation {
        self.representation
    }

    pub(crate) const fn converted(
        mut self,
        representation: tcl_syntax::value::ValueRepresentation,
    ) -> Self {
        self.representation = representation;
        self.alternatives = None;
        if matches!(
            representation,
            tcl_syntax::value::ValueRepresentation::Unknown
        ) {
            // An unqualified coercion describes effects, not the selected
            // numeric preparation that can retain integer acceptance.
            self.integer_contents = None;
        }
        self
    }

    pub(crate) fn container_alternatives(
        self,
    ) -> Option<crate::native_numeric::ClosedContainerRepresentations> {
        self.alternatives.or_else(|| {
            crate::native_numeric::ClosedContainerRepresentations::of(self.representation)
        })
    }

    pub(crate) fn possibly_converted(self, target: tcl_syntax::value::ValueRepresentation) -> Self {
        if self.representation == target {
            return self;
        }
        Self {
            representation: tcl_syntax::value::ValueRepresentation::Unknown,
            integer_contents: self.integer_contents,
            alternatives: self
                .container_alternatives()
                .zip(crate::native_numeric::ClosedContainerRepresentations::of(
                    target,
                ))
                .map(|(old, new)| old.joined(new)),
        }
    }

    pub(crate) fn joined(self, other: Self) -> Self {
        if self == other {
            return self;
        }
        Self {
            representation: tcl_syntax::value::ValueRepresentation::Unknown,
            integer_contents: self
                .integer_contents
                .filter(|recipe| Some(*recipe) == other.integer_contents),
            alternatives: self
                .container_alternatives()
                .zip(other.container_alternatives())
                .map(|(left, right)| left.joined(right)),
        }
    }
}

impl SourceOrdinaryLiteralObject {
    pub(super) fn capture_original_word(
        word: &crate::ir::WordExpr,
        value: &str,
        state: &ModuleCommandBindings,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Self> {
        if !state.ordinary_literal_pool.as_ref()?.effects_current() {
            return None;
        }
        if !matches!(
            word,
            crate::ir::WordExpr::Literal { .. } | crate::ir::WordExpr::BracedLiteral { .. }
        ) {
            return None;
        }
        let origin = state.current_source_origin.as_ref()?;
        super::ExecutedScriptSource::literal_word_base(
            super::executed_script_source::source_text(origin)?,
            word,
            value,
            config,
        )?;
        Some(Self)
    }

    pub(super) fn capture_word(
        word: &crate::ir::WordExpr,
        state: &ModuleCommandBindings,
    ) -> Option<SourceStockLiteralObject> {
        let pool = state.ordinary_literal_pool.as_ref()?;
        if !pool.authored_objects() {
            return None;
        }
        matches!(
            word,
            crate::ir::WordExpr::Literal { .. } | crate::ir::WordExpr::BracedLiteral { .. }
        )
        .then_some(SourceStockLiteralObject {
            representation: tcl_syntax::value::ValueRepresentation::Unknown,
            alternatives: None,
            integer_contents: pool.integer_contents,
        })
    }
    pub(super) fn capture_expression_literal(
        expression: &crate::expr_ast::ExprNode,
        state: &ModuleCommandBindings,
    ) -> Option<Self> {
        if !state.ordinary_literal_pool.as_ref()?.authored_objects() {
            return None;
        }
        matches!(expression, crate::expr_ast::ExprNode::Literal { .. }).then_some(Self)
    }

    pub(super) fn capture(
        native: SourceNativeInvocation<'_>,
        argument: usize,
        state: &ModuleCommandBindings,
        context: SourceExecutionContext<'_>,
    ) -> Option<Self> {
        // The immutable entry identity is retained even when the current
        // script is a separately entered body from that same interpreter.
        if !state.ordinary_literal_pool.as_ref()?.effects_current() {
            return None;
        }
        // Observed argv, aliases, and expanded operands have independent
        // frozen ownership; none can inherit an original literal receipt.
        context.written_representations?;
        super::native_result::numeric_call_arguments(context, native.target)?;
        let word = native.script_operands().written_word(argument)?;
        if !matches!(
            word,
            crate::ir::WordExpr::Literal { .. } | crate::ir::WordExpr::BracedLiteral { .. }
        ) {
            return None;
        }
        let value = native.invocation.arguments().literal_at(argument)?;
        let origin = state.current_source_origin.as_ref()?;
        super::ExecutedScriptSource::literal_word_base(
            super::executed_script_source::source_text(origin)?,
            word,
            value,
            context.config,
        )?;
        Some(Self)
    }
}

#[cfg(test)]
mod tests {
    use super::super::{SourceAnalysisOptions, SourceCommandBindings};
    use super::*;

    #[test]
    fn scalar_list_length_needs_input_class_on_every_c_release() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            for (source, closed) in [
                ("set items {a b}; llength $items", true),
                ("proc f {items} {llength $items}", false),
                (
                    "set items {a b}; unknown_host $items; llength $items",
                    false,
                ),
            ] {
                let bindings = SourceCommandBindings::analyse_with_options(
                    source,
                    tcl_lexer::LexerConfig::for_profile(registry.profile()),
                    registry,
                    SourceAnalysisOptions {
                        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                            registry.profile().unwrap(),
                        )),
                        native_compilation:
                            crate::environment_ingress::authoring_native_compilation(),
                        ..Default::default()
                    },
                );
                let offset = u32::try_from(source.rfind("llength").unwrap()).unwrap();
                assert_eq!(
                    bindings
                        .invocation_at_source("llength", offset)
                        .object_callback_effects_closed(),
                    closed,
                    "{profile}: {source}"
                );
            }
        }
    }

    #[test]
    fn stored_literal_class_closes_effects_without_representation_or_numeric_authority() {
        let source = "set items {a b}; llength $items; llength $items; set view $items";
        let bindings = analyse(source, false);
        for (offset, _) in source.match_indices("llength") {
            assert!(
                bindings
                    .invocation_at_source("llength", u32::try_from(offset).unwrap())
                    .object_callback_effects_closed()
            );
        }
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let offset = u32::try_from(source.rfind("$items").unwrap()).unwrap();
        let reads = bindings.variable_accesses_in_span(tcl_lexer::Span::new(offset, offset + 6));
        assert_eq!(reads.len(), 1);
        for context in reads[0].context_alternatives() {
            let place = reads[0].place_in_context(context, registry);
            assert!(
                context
                    .contents_stock_literal_object_at(&place, registry)
                    .is_some()
            );
            assert_eq!(
                context.contents_representation_at(&place),
                tcl_syntax::value::ValueRepresentation::Unknown
            );
            assert!(
                context
                    .contents_native_numeric_at(&place, registry)
                    .is_none()
            );
        }
        for source in [
            "set items {a b}; unknown_host $items; llength $items",
            "proc f {items} {llength $items}",
        ] {
            let bindings = analyse(source, false);
            let binding = bindings.invocation_at_source(
                "llength",
                u32::try_from(source.rfind("llength").unwrap()).unwrap(),
            );
            assert!(!binding.object_callback_effects_closed(), "{source}");
        }
    }

    #[test]
    fn stock_literal_classes_withdraw_on_unknown_coercion_and_missing_joins() {
        let source = "set item 7";
        let bindings = analyse(source, false);
        let registry = tcl_registry::model::ingress::static_context_for("tcl9.0").commands();
        let mut context = bindings.final_state.source_variables.as_ref().clone();
        let place = crate::var_resolve::resolve_literal_access(
            "item",
            &context,
            false,
            registry,
            tcl_registry::TraceOperation::Read,
        );
        assert!(
            context
                .contents_stock_literal_object_at(&place, registry)
                .is_some()
        );
        let mut other = context.clone();
        other.invalidate_shared_representations();
        assert!(
            other
                .contents_stock_literal_object_at(&place, registry)
                .is_none()
        );
        context.join(&other);
        assert!(
            context
                .contents_stock_literal_object_at(&place, registry)
                .is_none()
        );
    }

    fn analyse(source: &str, unknown_entry: bool) -> SourceCommandBindings {
        let owner = tcl_registry::model::ingress::static_context_for("tcl9.0");
        let profile = owner.commands().profile().unwrap();
        SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
            owner.commands(),
            SourceAnalysisOptions {
                unknown_entry,
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::BytecodeObject,
                    frame: tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode,
                    loop_depth: 0,
                    catch_depth: Some(0),
                },
                ..Default::default()
            },
        )
    }

    #[test]
    fn fresh_literal_iteration_preserves_the_nested_compiler_world() {
        let source = "foreach outer {a b} {foreach inner {a b} {set done YES}}";
        let bindings = analyse(source, false);
        for marker in ["foreach outer", "foreach inner"] {
            let binding = bindings.invocation_at_source(
                "foreach",
                u32::try_from(source.find(marker).unwrap()).unwrap(),
            );
            assert!(binding.object_callback_effects_closed(), "{marker}");
            assert!(
                matches!(
                    binding.native_compilation_admission_selection(),
                    tcl_registry::native_compilation::NativeCompilationSelection::Inline { .. }
                ),
                "{marker}: {:?}",
                binding.native_compilation_admission_selection(),
            );
        }
    }

    #[test]
    fn unknown_effects_and_new_object_epochs_do_not_restore_pool_provenance() {
        let source = "proc run {} {foreach x {a b} {set done YES}}; unknown_host {a b}; set fresh [dict create k 1]; run";
        let bindings = analyse(source, false);
        assert!(bindings.final_state.ordinary_literal_pool.is_none());
        let binding = bindings.invocation_at_source(
            "foreach",
            u32::try_from(source.find("foreach").unwrap()).unwrap(),
        );
        assert!(!binding.object_callback_effects_closed());
        assert!(
            analyse("foreach x {a b} {}", true)
                .final_state
                .ordinary_literal_pool
                .is_none()
        );
    }

    #[test]
    fn pool_provenance_intersects_and_runtime_tables_cannot_supply_it() {
        let source = "foreach x {a b} {}";
        let bindings = analyse(source, false);
        let mut joined = bindings.final_state.as_ref().clone();
        assert!(joined.ordinary_literal_pool.is_some());
        let mut unknown = joined.clone();
        unknown.mark_opaque_binding_mutation();
        Arc::make_mut(&mut unknown.source_variables).establish_created_representation_epoch();
        assert!(unknown.source_variables.representation_epoch.is_some());
        assert!(unknown.ordinary_literal_pool.is_none());
        assert!(joined.join(&unknown));
        assert!(joined.ordinary_literal_pool.is_none());
        let entry = tcl_runtime_api::NativeCompilationEntry {
            interpreter: tcl_runtime_api::native_compilation::NativeInterpreterIdentity {
                owner: 1,
                interpreter: 2,
            },
            epoch: 3,
            profile: tcl_dialect::DialectProfile::plain_tcl().cache_key(),
            invocation_policy: Some(tcl_dialect::DialectProfile::plain_tcl().cache_key()),
            expression_policy: None,
            execution_point: None,
            name_protocol: None,
            compiled_variable_protocol: None,
            compiled_local_layout: None,
            ensemble_target_objects: None,
            source_string_protocol: None,
            lexer_grammar: None,
            inline_compilation_disabled: false,
            authored_tmm_static: None,
            namespace_variable_tables: None,
            empty_literal_world: None,
            compiler_pass_environment: None,
            variable_observers:
                tcl_runtime_api::native_compilation::NativeVariableObserverPresence::Unknown,
            math_functions: None,
            closed: true,
            current_namespace: 0,
            frame: tcl_runtime_api::native_compilation::NativeCompilationFrame::Global,
            commands: Vec::new(),
            namespaces: Vec::new(),
        };
        let mut runtime = bindings.final_state.as_ref().clone();
        runtime.install_runtime_entry(&entry);
        assert!(runtime.ordinary_literal_pool.is_none());
        assert_ne!(runtime, bindings.final_state.as_ref().clone());
    }
    fn native_pool_state(entry: &tcl_runtime_api::NativeCompilationEntry) -> ModuleCommandBindings {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let options = SourceAnalysisOptions {
            native_entry: Some(entry),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                registry.profile().unwrap(),
            )),
            ..Default::default()
        };
        let mut state = ModuleCommandBindings::initial_with_options(registry, options, None);
        state.current_source_origin = Some(Arc::new(SourceOriginId::authored_image(
            tcl_lexer::SourceImage::native(b"proc fresh {} {}".as_slice()),
        )));
        state.ordinary_literal_pool = SourceOrdinaryLiteralPool::at_entry(&state, options);
        state
    }

    #[test]
    fn actual_empty_literal_pool_is_effect_only_and_expires_with_original_capture() {
        let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile();
        let (mut vm, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
        let state = native_pool_state(&entry);
        let pool = state
            .ordinary_literal_pool
            .as_ref()
            .expect("same live empty world");
        assert!(pool.effects_current());
        assert!(!pool.authored_objects());
        assert!(!pool.initial_numeric_representations());
        assert!(pool.integer_contents.is_none());
        let literal = crate::ir::WordExpr::Literal {
            text: "1".into(),
            source: crate::ir::SourceSite::source(tcl_lexer::Span::new(0, 1)),
        };
        assert!(SourceOrdinaryLiteralObject::capture_word(&literal, &state).is_none());
        let mut missing = entry.clone();
        missing.empty_literal_world = None;
        assert!(native_pool_state(&missing).ordinary_literal_pool.is_none());
        let mut foreign = entry.clone();
        foreign.interpreter.owner += 1;
        assert!(native_pool_state(&foreign).ordinary_literal_pool.is_none());
        let mut later = entry.clone();
        later.epoch += 1;
        assert!(native_pool_state(&later).ordinary_literal_pool.is_none());
        assert!(vm.try_eval_source("set second_capture 1").is_err());
        assert!(!pool.effects_current());
        assert!(pool.joined(pool).is_none());
        assert!(native_pool_state(&entry).ordinary_literal_pool.is_none());
        drop(vm);
        assert!(!entry.empty_literal_world.unwrap().is_current());
    }

    #[test]
    fn actual_empty_pool_preserves_independent_original_class_and_proc_publications() {
        let profile = tcl_registry::model::ingress::resolve_environment("tcl8.6").unit_profile();
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let (_vm, entry) = crate::environment_ingress::captured_native_entry_with_owner(profile);
        let source = "oo::class create C {method m {} {return method}}; namespace eval ::C {}; proc ::C::p {} {return proc}; [C new] m; ::C::p";
        let bindings = SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::for_profile(Some(profile)),
            registry,
            SourceAnalysisOptions {
                native_entry: Some(&entry),
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                native_compilation: crate::environment_ingress::authoring_native_compilation(),
                ..Default::default()
            },
        );
        let call = u32::try_from(source.rfind("::C::p").unwrap()).unwrap();
        assert!(
            bindings
                .invocation_at_source("::C::p", call)
                .proved_target()
                .is_some(),
            "fresh exact proc publication preserves the independently retained class dispatcher"
        );
        let mut unknown = bindings.final_state.as_ref().clone();
        unknown.mark_opaque_binding_mutation();
        assert!(unknown.ordinary_literal_pool.is_none());
    }
}
