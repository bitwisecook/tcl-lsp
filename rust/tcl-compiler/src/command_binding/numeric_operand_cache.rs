// tcl-lsp — a language server and toolchain for Tcl
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Current caches from reached original-object numeric getters. Contents,
//! effects, completion and frozen numeric operands remain independent proofs.

use super::{Arc, ModuleCommandBindings, SourceExecutionContext, SourceOutcomes};
use crate::{native_numeric::SourceNativeNumericShape, place::Place, var_resolve::ResolveContext};
use tcl_registry::native_numeric_conversion::NativeOperandNumericCacheProduction;

pub(super) struct CapturedNumericOperandCache {
    place: Place,
    origin: crate::var_resolve::ContentsOrigin,
    source: Option<Arc<super::SourceOriginId>>,
    shape: SourceNativeNumericShape,
}

impl CapturedNumericOperandCache {
    fn capture(
        place: Place,
        variables: &ResolveContext,
        production: NativeOperandNumericCacheProduction,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Self> {
        let dialect = variables.invocation_dialect?;
        if dialect.tcl_version != Some(production.tcl_version())
            || !variables.contents_native_string_access_closed_at(&place, registry)
            || !variables.contents_integer_number_branch_at(&place, registry)
            || ((production.requires_non_list_input()
                || production.requires_original_integer_cache())
                && variables.contents_native_numeric_category_at(&place, registry)
                    != Some(tcl_registry::TclType::Int))
        {
            return None;
        }
        let production =
            production.with_original_cache_class(original_cache_class(&place, variables, registry));
        let integer_contents = variables.contents_integer_increment_conversion_at(&place, registry);
        Some(Self {
            origin: variables.contents_origin(&place),
            source: variables.contents_source(&place).cloned(),
            place,
            shape: SourceNativeNumericShape::from_conversion(dialect, production)
                .with_integer_contents(integer_contents),
        })
    }

    fn finish(self, outcomes: &mut SourceOutcomes, registry: &tcl_registry::CommandRegistry) {
        let Some(normal) = &mut outcomes.normal else {
            return;
        };
        let variables = &normal.source_variables;
        // A different object stored into the same spelling after the original
        // read cannot inherit its cache. Unknown callback worlds fail the read.
        if !variables.read_produces_value(&self.place, registry)
            || variables.contents_origin(&self.place) != self.origin
            || variables.contents_source(&self.place) != self.source.as_ref()
        {
            return;
        }
        if let Some(key) = crate::var_resolve::canonical_binding_value_key(&self.place) {
            Arc::make_mut(&mut normal.source_variables)
                .value_representations
                .insert(
                    key,
                    crate::native_numeric::StoredNativeRepresentation::NumericShape(self.shape),
                );
        }
    }
}

fn original_cache_class(
    place: &Place,
    variables: &ResolveContext,
    registry: &tcl_registry::CommandRegistry,
) -> tcl_registry::native_numeric_conversion::NativeNumericOperandClass {
    use tcl_registry::{TclType, native_numeric_conversion::NativeNumericOperandClass as Class};
    match variables.contents_native_numeric_category_at(place, registry) {
        Some(TclType::Int) => return Class::Integer,
        Some(TclType::Double) => return Class::Double,
        Some(TclType::Numeric) => return Class::Numeric,
        _ => {}
    }
    match variables.contents_representation_at(place) {
        tcl_syntax::value::ValueRepresentation::String => Class::String,
        tcl_syntax::value::ValueRepresentation::List => Class::List,
        tcl_syntax::value::ValueRepresentation::Dict => Class::Dict,
        tcl_syntax::value::ValueRepresentation::Unknown => Class::Unknown,
    }
}

pub(super) fn capture_indices(
    native: super::SourceNativeInvocation<'_>,
    facts: &tcl_registry::InvocationFacts,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'_>,
) -> Vec<CapturedNumericOperandCache> {
    let arguments = native.invocation.arguments();
    let Some(count) = arguments.exact_argv_len() else {
        return Vec::new();
    };
    (0..count)
        .filter_map(|index| {
            let word = native.script_operands().written_word(index)?;
            let production = facts.integer_index_operand_conversion(
                arguments,
                *native.compilation_selection,
                index,
                crate::registry_invocation::native_compilation_word_shape(word),
            )?;
            let place =
                super::argument_reads::current_argument_place(native, index, state, context)?
                    .clone();
            CapturedNumericOperandCache::capture(
                place,
                &state.source_variables,
                production,
                context.registry,
            )
        })
        .collect()
}

pub(super) fn finish_indices(
    outcomes: &mut SourceOutcomes,
    captured: Vec<CapturedNumericOperandCache>,
    registry: &tcl_registry::CommandRegistry,
) {
    for cache in captured {
        cache.finish(outcomes, registry);
    }
}

fn expression_place(
    node: &crate::expr_ast::ExprNode,
    preparation: &super::SourceExpressionPreparation,
    reads: &[super::SourceVariableAccess],
    variables: &ResolveContext,
    registry: &tcl_registry::CommandRegistry,
) -> Option<Place> {
    let (_, site) = preparation.original_variable_source(node)?;
    let mut matching = reads.iter().filter(|read| read.source == site
        && matches!(&read.owner, super::SourceVariableEvaluationOwner::NativeExpression { invocation, .. }
            if invocation == &preparation.invocation));
    let read = matching.next()?;
    if matching.next().is_some()
        || read.context_residual() != super::SourceVariableReadResidual::Closed
        || !read
            .context_alternatives()
            .iter()
            .any(|context| context.as_ref() == variables)
    {
        return None;
    }
    Some(read.place_in_context(variables, registry))
}

/// Both original comparison inputs must select the integer numeric branch;
/// otherwise successful `<` can use strings without reaching `GetNumber`.
#[inline(never)]
pub(super) fn finish_expression(
    outcomes: &mut SourceOutcomes,
    preparation: &super::SourceExpressionPreparation,
    state: &ModuleCommandBindings,
    original_variables: &ResolveContext,
    reads: &[super::SourceVariableAccess],
    context: SourceExecutionContext<'_>,
) {
    use crate::expr_ast::ExprNode;
    let ExprNode::Binary {
        op: tcl_syntax::expr::BinOp::Lt,
        left,
        right,
    } = preparation.witness.tree()
    else {
        return;
    };
    let Some(dialect) = state.baseline.dialect else {
        return;
    };
    let mut caches = Vec::new();
    for node in [left.as_ref(), right.as_ref()] {
        match node {
            ExprNode::Literal { text, .. } => {
                if super::literal_object_pool::SourceOrdinaryLiteralObject::capture_expression_literal(node, state).is_none()
                    || !matches!(tcl_syntax::number::parse_whole_with(text,
                        tcl_syntax::number::ParseFlags::for_syntax(dialect.numbers)),
                        Some(tcl_syntax::number::Number::Int(_) | tcl_syntax::number::Number::Big { .. }))
                { return; }
            }
            ExprNode::Var { .. } => {
                let Some(production) = preparation.witness.integer_relational_operand_conversion(node) else { return };
                let Some(place) = expression_place(node, preparation, reads, original_variables, context.registry) else { return };
                let Some(cache) = CapturedNumericOperandCache::capture(place, original_variables, production, context.registry) else { return };
                caches.push(cache);
            }
            _ => return,
        }
    }
    finish_indices(outcomes, caches, context.registry);
}

#[cfg(test)]
mod tests {
    #[test]
    fn reached_integer_comparison_populates_only_the_current_operand_cache() {
        for (profile, expected) in [
            ("tcl8.4", None),
            ("tcl8.5", Some(tcl_registry::TclType::Numeric)),
            ("tcl8.6", Some(tcl_registry::TclType::Numeric)),
            ("tcl9.0", Some(tcl_registry::TclType::Numeric)),
            ("tcl9.1", Some(tcl_registry::TclType::Numeric)),
            ("jim", None),
        ] {
            let registry = tcl_registry::model::ingress::static_context_for(profile).commands();
            let source = "proc f {} {set x 0; expr {$x < 3}; set view $x}";
            let bindings = super::super::SourceCommandBindings::analyse_with_options(
                source,
                tcl_lexer::LexerConfig::for_profile(registry.profile()),
                registry,
                super::super::SourceAnalysisOptions {
                    invocation_dialect: registry
                        .profile()
                        .map(tcl_registry::InvocationDialect::of_profile),
                    native_compilation: crate::environment_ingress::authoring_native_compilation(),
                    ..Default::default()
                },
            );
            let offset = u32::try_from(source.rfind("$x").unwrap()).unwrap();
            let reads =
                bindings.variable_accesses_in_span(tcl_lexer::Span::new(offset, offset + 2));
            assert_eq!(reads.len(), 1, "{profile}");
            assert!(!reads[0].context_alternatives().is_empty(), "{profile}");
            for context in reads[0].context_alternatives() {
                let place = reads[0].place_in_context(context, registry);
                assert_eq!(
                    context.contents_native_numeric_category_at(&place, registry),
                    expected,
                    "{profile}: {:?}",
                    context.value_representations
                );
                assert!(
                    context
                        .contents_native_numeric_at(&place, registry)
                        .is_none()
                );
            }
        }
    }

    #[test]
    fn string_comparison_and_unproved_inputs_cannot_mint_a_numeric_cache() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for source in [
            "proc f {} {set x hello; expr {$x < \"world\"}; set view $x}",
            "proc f {x} {expr {$x < 3}; set view $x}",
            "proc observe args {}; proc f {} {set x 0; trace add variable x read observe; expr {$x < 3}; set view $x}",
        ] {
            let bindings = super::super::SourceCommandBindings::analyse_with_options(
                source,
                tcl_lexer::LexerConfig::for_profile(registry.profile()),
                registry,
                super::super::SourceAnalysisOptions {
                    invocation_dialect: registry
                        .profile()
                        .map(tcl_registry::InvocationDialect::of_profile),
                    native_compilation: crate::environment_ingress::authoring_native_compilation(),
                    ..Default::default()
                },
            );
            let offset = u32::try_from(source.rfind("$x").unwrap()).unwrap();
            let reads =
                bindings.variable_accesses_in_span(tcl_lexer::Span::new(offset, offset + 2));
            assert_eq!(reads.len(), 1, "{source}");
            assert!(!reads[0].context_alternatives().is_empty(), "{source}");
            for context in reads[0].context_alternatives() {
                let place = reads[0].place_in_context(context, registry);
                assert!(
                    !context.contents_already_native_numeric_at(&place, registry),
                    "{source}"
                );
            }
        }
    }

    #[test]
    fn scalar_index_cache_requires_the_reached_original_integer_branch() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (source, expected) in [
            (
                "proc f {} {set x 0; lrange {a b} $x end; set view $x}",
                Some(tcl_registry::TclType::Int),
            ),
            (
                "proc f {} {set x 0; incr x; lindex {a b} $x; set view $x}",
                Some(tcl_registry::TclType::Int),
            ),
            ("proc f {} {set x 0; lindex {a b} $x; set view $x}", None),
            (
                "proc f {} {set x end-1; lrange {a b} $x end; set view $x}",
                None,
            ),
        ] {
            let bindings = super::super::SourceCommandBindings::analyse_with_options(
                source,
                tcl_lexer::LexerConfig::for_profile(registry.profile()),
                registry,
                super::super::SourceAnalysisOptions {
                    invocation_dialect: registry
                        .profile()
                        .map(tcl_registry::InvocationDialect::of_profile),
                    native_compilation: crate::environment_ingress::authoring_native_compilation(),
                    ..Default::default()
                },
            );
            let offset = u32::try_from(source.rfind("$x").unwrap()).unwrap();
            let reads =
                bindings.variable_accesses_in_span(tcl_lexer::Span::new(offset, offset + 2));
            assert_eq!(reads.len(), 1, "{source}");
            assert!(!reads[0].context_alternatives().is_empty(), "{source}");
            for context in reads[0].context_alternatives() {
                let place = reads[0].place_in_context(context, registry);
                assert_eq!(
                    context.contents_native_numeric_category_at(&place, registry),
                    expected,
                    "{source}: {:?}",
                    context.value_representations
                );
                assert!(
                    context
                        .contents_native_numeric_at(&place, registry)
                        .is_none(),
                    "{source}"
                );
            }
        }
    }
}
