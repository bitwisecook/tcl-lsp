// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Actual selected substitution addresses frozen with original argv words.

use super::{Arc, ModuleCommandBindings, SourceCommandBindings, SourceExecutionContext};
use crate::{place::Place, var_resolve::ResolveContext};

pub(super) struct FrozenSourceArgumentRead {
    place: Place,
    context: Arc<ResolveContext>,
    source: crate::ir::SourceSite,
    spelling: String,
    origin: Arc<super::SourceOriginId>,
    pub(super) expression: Option<Arc<super::read_store_schedule::CapturedExpressionRead>>,
}

impl FrozenSourceArgumentRead {
    pub(super) fn from_expression(
        read: Option<Arc<super::read_store_schedule::CapturedExpressionRead>>,
        state: &ModuleCommandBindings,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Arc<Self>> {
        let read = read?;
        if !read.remains_current(state, registry) {
            return None;
        }
        Some(Arc::new(Self {
            place: read.place.clone(),
            context: Arc::clone(&read.before),
            source: read.operand.clone(),
            spelling: String::new(),
            origin: Arc::clone(&read.expression.source),
            expression: Some(read),
        }))
    }

    pub(super) fn current_place(
        &self,
        variables: &ResolveContext,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<&Place> {
        if !variables.read_produces_value(&self.place, registry)
            || variables.contents_origin(&self.place) != self.context.contents_origin(&self.place)
            || variables.contents_source(&self.place) != self.context.contents_source(&self.place)
        {
            return None;
        }
        Some(&self.place)
    }
}

impl SourceCommandBindings {
    pub(super) fn capture_argument_read(
        &self,
        word: &crate::ir::WordExpr,
        state: &ModuleCommandBindings,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Arc<FrozenSourceArgumentRead>> {
        let (spelling, source) = word.sole_variable_substitution()?;
        let origin = state.current_source_origin.as_ref()?;
        let accesses = if self.root_origin.as_ref() == Some(origin) {
            self.variable_accesses.get(&source.span.start())?
        } else {
            self.origin_variable_accesses
                .get(origin)?
                .get(&source.span.start())?
        };
        let access = super::SourceVariableAccess::find_at_source(accesses, source, spelling)?;
        if access.context_residual() != super::SourceVariableReadResidual::Closed
            || !access
                .context_alternatives()
                .iter()
                .any(|context| context == &state.source_variables)
        {
            return None;
        }
        let place = access.place_in_context(&state.source_variables, registry);
        state
            .source_variables
            .read_produces_value(&place, registry)
            .then(|| {
                Arc::new(FrozenSourceArgumentRead {
                    place,
                    context: Arc::clone(&state.source_variables),
                    source: source.clone(),
                    spelling: spelling.to_owned(),
                    origin: Arc::clone(origin),
                    expression: None,
                })
            })
    }
}

pub(super) fn current_argument_place<'a>(
    native: super::SourceNativeInvocation<'_>,
    argument: usize,
    state: &ModuleCommandBindings,
    context: SourceExecutionContext<'a>,
) -> Option<&'a Place> {
    // The original-word mapping excludes expansion and captured alias prefixes.
    if !super::object_callbacks::original_operand_is_current(native, argument, context) {
        return None;
    }
    let operands = native.script_operands();
    let crate::registry_invocation::InvocationWordOrigin::Written(written) =
        operands.written_origin(argument)?
    else {
        return None;
    };
    let (spelling, source) = operands
        .written_word(argument)?
        .sole_variable_substitution()?;
    context.written_representations?;
    let read = context.written_variable_reads?.get(written)?.as_ref()?;
    if read.source != *source
        || read.spelling != spelling
        || state.current_source_origin.as_ref() != Some(&read.origin)
    {
        return None;
    }
    read.current_place(&state.source_variables, context.registry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_syntax::value::ValueRepresentation;

    #[test]
    fn native_container_reads_use_the_evaluated_argument_address() {
        for profile in ["tcl8.6", "tcl9.1"] {
            let environment = tcl_registry::model::ingress::static_context_for(profile);
            let registry = environment.commands();
            for (source, observed) in [
                (
                    "set k old; set a(oldnew) [dict create key value]; llength $a($k[set k new]); set view $a(oldnew)",
                    false,
                ),
                (
                    "set k old; set a(oldnew) [dict create key value]; proc observe args {}; trace add variable a(oldnew) read observe; llength $a($k[set k new]); set view $a(oldnew)",
                    true,
                ),
            ] {
                let bindings = SourceCommandBindings::analyse_with_options(
                    source,
                    tcl_lexer::LexerConfig::for_profile(registry.profile()),
                    registry,
                    super::super::SourceAnalysisOptions {
                        invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(
                            registry.profile().unwrap(),
                        )),
                        native_compilation:
                            crate::environment_ingress::authoring_native_compilation(),
                        ..super::super::SourceAnalysisOptions::default()
                    },
                );
                let offset = u32::try_from(source.rfind("$a(oldnew)").unwrap()).unwrap();
                let reads =
                    bindings.variable_accesses_in_span(tcl_lexer::Span::new(offset, offset + 10));
                assert_eq!(reads.len(), 1, "{profile}: {source}");
                for context in reads[0].context_alternatives() {
                    let place = reads[0].place_in_context(context, registry);
                    if observed {
                        assert!(
                            !context.read_produces_value(&place, registry),
                            "{profile}: {source}"
                        );
                    } else {
                        assert_eq!(
                            context.contents_representation_at(&place),
                            ValueRepresentation::List,
                            "{profile}: {source}"
                        );
                    }
                }
            }
        }
    }
}
