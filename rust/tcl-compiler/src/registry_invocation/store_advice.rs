//! Diagnostic-only ordering evidence for an overwritten original local store.
//!
//! This projection leaves command dispatch, observer schedules and executable
//! stores unchanged. It is not a dead-store elimination or replay contract.

use super::InvocationMetadataContext;
use crate::command_binding::CommandAllocationSite;
use crate::ir::{CommandTokens, WordExpr};
use crate::place::{CellGeneration, CellOwner, Place, PlaceKind};
use crate::var_resolve::ContentsOrigin;
use tcl_registry::CommandRegistry;

/// Original declared setter ordering, independent of physical store entry.
/// No cell, contents version, successful completion or edit escapes this type.
pub(crate) struct ConditionalDeclaredLocalOverwriteAdvice {
    first: CommandAllocationSite,
    next: CommandAllocationSite,
    name: String,
    target: tcl_lexer::Span,
}

impl ConditionalDeclaredLocalOverwriteAdvice {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn target(&self) -> tcl_lexer::Span {
        self.target
    }

    pub(crate) fn owns_source(&self, source: &tcl_lexer::SourceImage) -> bool {
        self.first.source == self.next.source
            && self.first.offset < self.next.offset
            && matches!(self.first.source.kind(), crate::command_binding::SourceOriginKind::Authored(image) if image == source)
    }
}

/// The declaration graph owns adjacency, native setter operands and prefix
/// availability. This adapter exposes only explicitly conditional warnings.
pub(crate) fn conditional_declared_overwrite_advice(
    report: &crate::command_binding::DeclarationFlowReport,
) -> impl Iterator<Item = ConditionalDeclaredLocalOverwriteAdvice> + '_ {
    report
        .conditional_overwrite_occurrences()
        .map(
            |(first, next, name, target)| ConditionalDeclaredLocalOverwriteAdvice {
                first: first.clone(),
                next: next.clone(),
                name: name.to_owned(),
                target,
            },
        )
}

/// An original literal local setter with no read in its complete conditional
/// native declaration layout. Runtime callback and observation alternatives
/// remain present, so this receipt permits only a conditional warning.
pub(crate) struct ConditionalUnreadLocalStoreAdvice {
    site: CommandAllocationSite,
    name: String,
}

impl ConditionalUnreadLocalStoreAdvice {
    pub(crate) fn name(&self) -> &str {
        &self.name
    }

    pub(crate) fn owns(&self, tokens: &CommandTokens) -> bool {
        tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.invocation_site())
            == Some(&self.site)
    }
}

/// Require an unobserved original setter and a complete declaration graph.
/// Explicit reads, aliases, unknown command/script paths and dynamic read
/// operands withdraw this diagnostic projection. It changes no liveness or
/// dead-store removal permission.
pub(crate) fn conditional_unread_local_store_advice(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
    report: &crate::command_binding::DeclarationFlowReport,
) -> Option<ConditionalUnreadLocalStoreAdvice> {
    if tokens.synthetic.is_some() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    if !binding.unobserved_native_dispatch() {
        return None;
    }
    let place = original_literal_store(registry, context, tokens)?;
    let site = binding.invocation_site()?;
    report
        .local_store_may_be_unread(site, &place.name)
        .then(|| ConditionalUnreadLocalStoreAdvice {
            site: site.clone(),
            name: place.name,
        })
}

/// Two unobserved original setters on one physical local cell. Private
/// construction retains the exact source-instance order and intervening
/// contents origin; a later read of the second value cannot read the first.
pub(crate) struct OverwrittenLocalStoreAdvice {
    first: CommandAllocationSite,
    next: CommandAllocationSite,
    place: Place,
}

impl OverwrittenLocalStoreAdvice {
    /// Diagnostic target. This grants no executable removal of either setter.
    pub(crate) fn name(&self) -> &str {
        &self.place.name
    }

    pub(crate) fn owns(&self, first: &CommandTokens, next: &CommandTokens) -> bool {
        first
            .source_binding
            .as_ref()
            .and_then(|b| b.invocation_site())
            == Some(&self.first)
            && next
                .source_binding
                .as_ref()
                .and_then(|b| b.invocation_site())
                == Some(&self.next)
    }
}

/// Project adjacent original setter observations. Missing dispatch, source,
/// physical origin or object-effect evidence declines rather than inventing
/// an observer-free interval. Callers must additionally prove adjacency in
/// their original statement sequence, without crossing a branch or callback.
pub(crate) fn overwritten_local_store_advice(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    first: &CommandTokens,
    next: &CommandTokens,
) -> Option<OverwrittenLocalStoreAdvice> {
    overwrite_interval(registry, context, first, next, true)
}

/// A local overwrite if native object conversion and release hooks do not
/// observe the intermediate value. Runtime callback uncertainty stays open;
/// only a diagnostic phrased as a possible overwrite may consume this type.
pub(crate) struct ConditionalOverwrittenLocalStoreAdvice(OverwrittenLocalStoreAdvice);

impl ConditionalOverwrittenLocalStoreAdvice {
    pub(crate) fn name(&self) -> &str {
        self.0.name()
    }

    pub(crate) fn owns(&self, first: &CommandTokens, next: &CommandTokens) -> bool {
        self.0.owns(first, next)
    }
}

/// Preserve original source, dispatch, physical cell and contents-order
/// requirements while retaining the explicit object-callback condition.
pub(crate) fn conditional_overwritten_local_store_advice(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    first: &CommandTokens,
    next: &CommandTokens,
) -> Option<ConditionalOverwrittenLocalStoreAdvice> {
    overwrite_interval(registry, context, first, next, false)
        .map(ConditionalOverwrittenLocalStoreAdvice)
}

fn overwrite_interval(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    first: &CommandTokens,
    next: &CommandTokens,
    closed_object_effects: bool,
) -> Option<OverwrittenLocalStoreAdvice> {
    if first.synthetic.is_some() || next.synthetic.is_some() {
        return None;
    }
    let first_binding = first.source_binding.as_ref()?;
    let next_binding = next.source_binding.as_ref()?;
    let first_site = first_binding.invocation_site()?;
    let next_site = next_binding.invocation_site()?;
    if first_site.source != next_site.source
        || first_site.offset >= next_site.offset
        || first_binding.variable_frame != next_binding.variable_frame
        || !original_gap_is_empty(first, next)
        || !first_binding.unobserved_native_dispatch()
        || !next_binding.unobserved_native_dispatch()
        || (closed_object_effects
            && (!first_binding.object_callback_effects_closed()
                || !next_binding.object_callback_effects_closed()))
    {
        return None;
    }
    let first_place = original_literal_store(registry, context, first)?;
    let next_place = original_literal_store(registry, context, next)?;
    let mut published = first_place.cell.clone()?;
    let next_cell = next_place.cell.as_ref()?;
    if published.generation == CellGeneration::Incoming
        && next_cell.generation == CellGeneration::After(first_site.offset)
    {
        published.generation = next_cell.generation;
    }
    if &published != next_cell
        || first_place.name != next_place.name
        || next_binding.variable_context.contents_origin(&next_place)
            != ContentsOrigin::WrittenAt(first_site.offset)
        || next_binding
            .variable_context
            .contents_stock_literal_object_at(&next_place, registry)
            .is_none()
    {
        return None;
    }
    Some(OverwrittenLocalStoreAdvice {
        first: first_site.clone(),
        next: next_site.clone(),
        place: first_place,
    })
}

fn original_literal_store(
    registry: &CommandRegistry,
    context: Option<InvocationMetadataContext<'_>>,
    tokens: &CommandTokens,
) -> Option<Place> {
    let context = context?;
    let binding = tokens.source_binding.as_ref()?;
    if let Some(input) = context.source_analysis_input()
        && binding
            .original_lexer_config_for_tokens(tokens)?
            .normalized()
            != input.lexer_config().normalized()
    {
        return None;
    }
    let normal =
        super::normal_transfer_invocation_with_metadata_context(registry, Some(context), tokens)?;
    if normal.stored_value_argument()? != 1 || normal.written_argument(1)? != 1 {
        return None;
    }
    let word = normal.stored_value_word(&binding.variable_context, registry)?;
    if !matches!(
        word,
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. }
    ) {
        return None;
    }
    normal.stored_value_literal(&binding.variable_context, registry)?;
    let places = normal.mutation_places(&binding.variable_context, registry);
    let [place] = places.as_slice() else {
        return None;
    };
    let cell = place.cell.as_ref()?;
    if place.kind != PlaceKind::Scalar
        || place.is_global()
        || place.dynamic
        || place.observed
        || !matches!(cell.owner, CellOwner::Activation(_))
        || cell.generation == CellGeneration::Unknown
    {
        return None;
    }
    Some(place.clone())
}

fn original_gap_is_empty(first: &CommandTokens, next: &CommandTokens) -> bool {
    let Some(binding) = first.source_binding.as_ref() else {
        return false;
    };
    let Some(site) = binding.invocation_site() else {
        return false;
    };
    let Some(next_site) = next
        .source_binding
        .as_ref()
        .and_then(|b| b.invocation_site())
    else {
        return false;
    };
    let Some(end) = first.words().last().map(|word| word.source().span.end()) else {
        return false;
    };
    let image = site.source.source_image();
    let Some(gap) = image.bytes().get(end as usize..next_site.offset as usize) else {
        return false;
    };
    let Some(dialect) = binding.variable_context.invocation_dialect else {
        return false;
    };
    let gap = tcl_lexer::SourceImage::from_bytes(gap, image.channel());
    tcl_lexer::Lexer::with_source_map(
        gap.source_map(),
        tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
    )
    .tokenise_all()
    .is_ok_and(|tokens| {
        tokens.iter().all(|token| {
            matches!(
                token.kind,
                tcl_lexer::TokenType::Sep
                    | tcl_lexer::TokenType::Eol
                    | tcl_lexer::TokenType::Comment,
            )
        })
    })
}

#[cfg(test)]
mod tests {
    fn standalone_context() -> Option<super::InvocationMetadataContext<'static>> {
        Some(tcl_registry::model::ingress::static_context_for("tcl8.6").into())
    }

    fn setters(source: &str) -> Vec<crate::ir::CommandTokens> {
        setters_in(
            source,
            tcl_registry::model::ingress::static_context_for("tcl8.6").commands(),
        )
    }

    fn setters_in(
        source: &str,
        registry: &tcl_registry::CommandRegistry,
    ) -> Vec<crate::ir::CommandTokens> {
        let unit = crate::compilation_unit::CompilationUnit::build_for_profile(
            source,
            registry,
            false,
            registry.profile().unwrap(),
        );
        let procedure = &unit.ir_module.procedures["::p"];
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let bindings =
            crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
        crate::segmenter::segment_commands_with_offset_and_config(
            procedure
                .body_source
                .as_deref()
                .expect("original declaration body"),
            procedure.body_offset,
            config,
        )
        .into_iter()
        .filter(|segment| segment.texts.first().is_some_and(|head| head == "set"))
        .map(|segment| {
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceImage::document(source).source_map(),
                config,
                &segment,
            );
            bindings.stamp_original_tokens(&mut tokens);
            tokens
        })
        .collect()
    }

    fn report_overwrite_gates(
        registry: &tcl_registry::CommandRegistry,
        first: &crate::ir::CommandTokens,
        next: &crate::ir::CommandTokens,
    ) {
        eprintln!(
            "overwrite gap_closed={}",
            super::original_gap_is_empty(first, next)
        );
        for (label, tokens) in [("first", first), ("next", next)] {
            let Some(binding) = &tokens.source_binding else {
                eprintln!("overwrite {label}: no original binding");
                continue;
            };
            let place = super::original_literal_store(registry, standalone_context(), tokens);
            let normal =
                crate::registry_invocation::normal_transfer_invocation(registry, None, tokens);
            eprintln!(
                "overwrite {label}: offset={:?} frame={:?} unobserved={} callbacks_closed={} normal={} literal={:?} place={:?} origin={:?} stock={}",
                binding.invocation_site().map(|site| site.offset),
                binding.variable_frame,
                binding.unobserved_native_dispatch(),
                binding.object_callback_effects_closed(),
                normal.is_some(),
                normal.as_ref().and_then(
                    |normal| normal.stored_value_literal(&binding.variable_context, registry)
                ),
                place,
                place
                    .as_ref()
                    .map(|place| binding.variable_context.contents_origin(place)),
                place.as_ref().is_some_and(|place| binding
                    .variable_context
                    .contents_stock_literal_object_at(place, registry)
                    .is_some()),
            );
        }
    }

    #[test]
    fn original_store_advice_keeps_actual_availability_and_literal_cell_names() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Availability describes the selected setter; original local cells,
        // contents order and observer requirements remain independently needed.
        let profile = tcl_dialect::DialectProfile::find("tcl8.6").unwrap();
        let mut registry =
            tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let baseline = tcl_registry::model::ingress::static_context_for("tcl8.6");
        let mut setter = registry.get("set").unwrap().clone();
        setter.surface = Some(tcl_dialect::model::SpecSurface::TCL86_PLUS);
        registry.insert(setter);
        let current =
            std::sync::Arc::new(baseline.with_command_store(std::sync::Arc::new(registry)));
        let older = tcl_registry::model::ingress::static_context_for("tcl8.4")
            .with_command_store(std::sync::Arc::clone(current.commands()));
        assert!(std::sync::Arc::ptr_eq(current.commands(), older.commands()));
        let registry = current.commands();
        let source = "proc p {} {set {$literal} FIRST; set {$literal} SECOND; puts ${$literal}}";
        let tokens = setters_in(source, registry);
        assert_eq!(tokens.len(), 2);
        let advice = super::conditional_overwritten_local_store_advice(
            registry,
            Some(current.as_ref().into()),
            &tokens[0],
            &tokens[1],
        )
        .expect("same original literal cell under selected current availability");
        assert_eq!(advice.name(), "$literal");
        assert!(advice.owns(&tokens[0], &tokens[1]));
        assert!(
            super::conditional_overwritten_local_store_advice(
                registry,
                Some((&older).into()),
                &tokens[0],
                &tokens[1],
            )
            .is_none()
        );
        assert!(
            super::conditional_overwritten_local_store_advice(
                registry, None, &tokens[0], &tokens[1],
            )
            .is_none()
        );
        assert!(
            super::conditional_overwritten_local_store_advice(
                registry,
                Some(tcl_registry::model::ingress::static_context_for("tcl9.1").into()),
                &tokens[0],
                &tokens[1],
            )
            .is_none()
        );
        let profile = registry.profile().unwrap();
        let mut config = tcl_lexer::LexerConfig::for_profile(Some(profile));
        config.strict_quoting = !config.strict_quoting;
        let input = crate::analyser::ResolvedAnalysisInput::new(
            profile,
            profile,
            std::sync::Arc::clone(&current),
            config,
        );
        let changed = super::InvocationMetadataContext::for_analysis_input(registry, &input);
        assert!(
            super::conditional_overwritten_local_store_advice(
                registry, changed, &tokens[0], &tokens[1],
            )
            .is_none()
        );
    }

    #[test]
    fn conditional_unread_store_requires_the_complete_original_layout() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        for (source, expected) in [
            (
                "proc p {d} {dict for {k v} $d {set unusedvar 1; puts $k}}",
                true,
            ),
            (
                "proc p {d} {dict for {k v} $d {set unusedvar 1; puts $unusedvar}}",
                false,
            ),
            (
                "proc p {d} {dict for {k v} $d {set unusedvar 1; set unusedvar}}",
                false,
            ),
            (
                "proc p {d} {dict for {k v} $d {set unusedvar 1; subst {$unusedvar}}}",
                false,
            ),
            (
                "proc p {d} {dict for {k v} $d {set unusedvar 1; operation}}",
                false,
            ),
            (
                "proc p {d} {dict for {k v} $d {set unusedvar 1; set other [operation]}}",
                false,
            ),
            (
                "proc p {d} {dict for {k v} $d {set unusedvar 1; upvar 0 observed unusedvar}}",
                false,
            ),
        ] {
            let bindings =
                crate::command_binding::SourceCommandBindings::analyse(source, config, registry);
            let command = "set unusedvar 1";
            let offset =
                u32::try_from(source.find(command).unwrap()).expect("fixture source offset");
            let segment =
                crate::segmenter::segment_commands_with_offset_and_config(command, offset, config)
                    .remove(0);
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceImage::document(source).source_map(),
                config,
                &segment,
            );
            bindings.stamp_original_tokens(&mut tokens);
            let report = tokens
                .source_binding
                .as_ref()
                .and_then(|binding| binding.declaration_flow_report(registry));
            let advice = report.as_ref().and_then(|report| {
                super::conditional_unread_local_store_advice(
                    registry,
                    standalone_context(),
                    &tokens,
                    report,
                )
            });
            if expected
                && advice.is_none()
                && let Some(binding) = &tokens.source_binding
            {
                eprintln!(
                    "conditional unread setter: unobserved={} local_literal={}",
                    binding.unobserved_native_dispatch(),
                    super::original_literal_store(registry, standalone_context(), &tokens)
                        .is_some()
                );
                if let Some(site) = binding.invocation_site()
                    && let Some(report) = &report
                {
                    eprintln!(
                        "conditional unread graph: {}",
                        report.unread_gate_summary(site, "unusedvar")
                    );
                }
            }
            assert_eq!(advice.is_some(), expected, "{source}");
            if let Some(advice) = advice {
                assert_eq!(advice.name(), "unusedvar");
                assert!(advice.owns(&tokens));
            }
        }
    }

    #[test]
    fn numeric_original_stores_keep_conditional_overwrite_advice() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let tokens = setters("proc p {} {set a 1; set x 1; set x 2; return [subst {$a$x}]}");
        assert_eq!(tokens.len(), 3);
        let advice = super::conditional_overwritten_local_store_advice(
            registry,
            standalone_context(),
            &tokens[1],
            &tokens[2],
        );
        if advice.is_none() {
            report_overwrite_gates(registry, &tokens[1], &tokens[2]);
        }
        assert!(
            advice.is_some(),
            "original numeric literal overwrite remains diagnostic advice"
        );
    }

    #[test]
    fn exact_overwrite_interval_rejects_reads_callbacks_and_substituted_values() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let source = "proc p {} {set x FIRST; set x SECOND; puts $x}";
        let tokens = setters(source);
        assert_eq!(tokens.len(), 2);
        assert!(
            super::overwritten_local_store_advice(
                registry,
                standalone_context(),
                &tokens[0],
                &tokens[1]
            )
            .is_none(),
            "an unentered declaration does not close object callbacks"
        );
        let advice = super::conditional_overwritten_local_store_advice(
            registry,
            standalone_context(),
            &tokens[0],
            &tokens[1],
        );
        if advice.is_none() {
            report_overwrite_gates(registry, &tokens[0], &tokens[1]);
        }
        let advice = advice.expect("conditional original local overwrite interval");
        assert_eq!(advice.name(), "x");
        assert!(advice.owns(&tokens[0], &tokens[1]));
        for source in [
            "proc p {} {set x FIRST; puts $x; set x SECOND}",
            "proc p {} {set x FIRST; set x [list SECOND]}",
            "proc watch args {}; trace add execution set enter watch; proc p {} {set x FIRST; set x SECOND}",
            "proc set args {}; proc p {} {set x FIRST; set x SECOND}",
        ] {
            let tokens = setters(source);
            assert_eq!(tokens.len(), 2, "original setters: {source}");
            assert!(
                super::conditional_overwritten_local_store_advice(
                    registry,
                    standalone_context(),
                    &tokens[0],
                    &tokens[1]
                )
                .is_none(),
                "no conditional original overwrite: {source}"
            );
        }
    }
}
