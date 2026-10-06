//! Original caller operands for a source-owned callee formal occurrence.
//!
//! Call allocation, native formal layout and symbolic incoming-value provenance
//! remain separate from an actual activation or a closed caller set.

use super::{
    BindingKind, CommandAllocationSite, SourceCommandBindings, SourceConditionalBodyEntry,
};
use crate::registry_invocation::{EffectiveInvocationWord, InvocationWordOrigin};

/// A literal candidate delivered by one original declaration call. Missing or
/// external callers remain possible; the receipt is neither a parameter value
/// nor a selected runtime command target.
pub(crate) struct DeclarationFormalCallValue {
    call: CommandAllocationSite,
    origin: InvocationWordOrigin,
    value: String,
}

impl DeclarationFormalCallValue {
    pub(crate) fn value_in_source(
        &self,
        source: &std::sync::Arc<super::SourceOriginId>,
    ) -> Option<&str> {
        if &self.call.source != source {
            return None;
        }
        match self.origin {
            InvocationWordOrigin::Written(index) if index > 0 => Some(&self.value),
            InvocationWordOrigin::BindingPrefix(_) => Some(&self.value),
            _ => None,
        }
    }
}

impl SourceCommandBindings {
    /// Original callee formal operands used by registry-owned caller links.
    /// The returned names suppress caller diagnostics only. Unknown dispatch,
    /// alias installation, physical effects and a closed caller set stay open.
    pub(crate) fn declaration_caller_alias_arguments(
        &self,
        invocation: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Vec<String>> {
        let advice = self.declaration_call_layout_advice(invocation)?;
        let mut agreed = None;
        for target in advice.targets() {
            if target.registry_backed || target.kind != BindingKind::Proc {
                return None;
            }
            let allocation = target.implementation_allocation.as_ref()?;
            let callee = self
                .declaration_layouts
                .values()
                .filter_map(|observations| {
                    super::declaration_layout::original_declaration_layouts(observations)
                })
                .flatten()
                .find_map(|observation| {
                    let super::declaration_layout::OriginalDiagnosticFrameEntry::Body(body) =
                        observation.entry.as_ref()
                    else {
                        return None;
                    };
                    (body.allocation() == allocation).then(|| body.clone())
                })?;
            let names = callee
                .parameters()
                .iter()
                .map(|parameter| parameter.name.as_str())
                .collect::<Vec<_>>();
            let mut values = Vec::new();
            for (site, observations) in &self.declaration_layouts {
                let Some(observations) =
                    super::declaration_layout::original_declaration_layouts(observations)
                else {
                    continue;
                };
                if !callee.owns_source(&site.source, site.offset)
                    || observations.clone().any(|observation|
                        !matches!(observation.entry.as_ref(), super::declaration_layout::OriginalDiagnosticFrameEntry::Body(body) if body == &callee))
                { continue; }
                let Some(tokens) = self.declaration_original_tokens_at(&callee, site.offset) else {
                    continue;
                };
                let Some(layout) = self.declaration_operand_layout_advice(&tokens) else {
                    continue;
                };
                let Some(flow) = crate::registry_invocation::declaration_invocation_flow(
                    registry, &tokens, &layout,
                ) else {
                    continue;
                };
                for operand in flow.caller_alias_operands {
                    let crate::ir::WordExpr::Variable { spelling, source } = operand else {
                        continue;
                    };
                    let Some(formal) = self.symbolic_declaration_formal_value_at(
                        &site.source,
                        &source,
                        &spelling,
                        &names,
                        registry,
                    ) else {
                        continue;
                    };
                    let Some(parameter) = callee
                        .parameters()
                        .iter()
                        .position(|parameter| parameter.name == formal)
                    else {
                        continue;
                    };
                    if let Some((_, value)) =
                        original_formal_argument(self, invocation, &callee, parameter)
                        && !values.contains(&value)
                    {
                        values.push(value);
                    }
                }
            }
            values.sort();
            if agreed.as_ref().is_some_and(|previous| previous != &values) {
                return None;
            }
            agreed = Some(values);
        }
        agreed
    }

    /// Query a command-head formal through its original incoming/copy receipt,
    /// then independently pair original callers with the same callee allocation.
    pub(crate) fn declaration_formal_call_values(
        &self,
        offset: u32,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Vec<DeclarationFormalCallValue>> {
        let origin = self.root_origin.as_ref()?;
        let entry = self.conditional_body_entry_at(origin, offset)?;
        let tokens = self.declaration_original_tokens_at(&entry, offset)?;
        let head = tokens.words().first()?;
        let crate::ir::WordExpr::Variable {
            spelling, source, ..
        } = head
        else {
            return None;
        };
        if source.span.start() != offset {
            return None;
        }
        let parameters = entry
            .parameters()
            .iter()
            .map(|parameter| parameter.name.as_str())
            .collect::<Vec<_>>();
        let formal = self.symbolic_declaration_formal_value_at(
            origin,
            source,
            spelling,
            &parameters,
            registry,
        )?;
        let parameter = entry
            .parameters()
            .iter()
            .position(|parameter| parameter.name == formal)?;
        let mut candidates = Vec::new();
        for (site, observations) in &self.declaration_layouts {
            let Some(observations) =
                super::declaration_layout::original_declaration_layouts(observations)
            else {
                continue;
            };
            let Some(first) = observations.clone().next() else {
                continue;
            };
            if observations
                .clone()
                .any(|observation| observation.entry != first.entry)
                || site.source != *origin
            {
                continue;
            }
            let Some(caller) = self.diagnostic_original_tokens_at(&first.entry, site.offset) else {
                continue;
            };
            let Some(binding) = caller.source_binding.as_ref() else {
                continue;
            };
            if binding
                .declaration_flow_report(registry)
                .and_then(|report| report.invocation_may_be_reached(site))
                != Some(true)
            {
                continue;
            }
            let Some(value) = original_formal_argument(self, &caller, &entry, parameter) else {
                continue;
            };
            candidates.push(DeclarationFormalCallValue {
                call: site.clone(),
                origin: value.0,
                value: value.1,
            });
        }
        Some(candidates)
    }
}

fn original_formal_argument(
    bindings: &SourceCommandBindings,
    invocation: &crate::ir::CommandTokens,
    callee: &SourceConditionalBodyEntry,
    parameter: usize,
) -> Option<(InvocationWordOrigin, String)> {
    use tcl_syntax::formal_params::FormalArgumentBinding;
    let advice = bindings.declaration_call_layout_advice(invocation)?;
    let dialect = advice.dialect();
    let mut agreed = None;
    for target in advice.targets() {
        if target.registry_backed
            || target.kind != BindingKind::Proc
            || target.implementation_allocation.as_ref() != Some(callee.allocation())
        {
            return None;
        }
        let effective = crate::registry_invocation::effective_words_for_target(invocation, target)?;
        let plan = tcl_syntax::formal_params::bind_formal_arguments(
            callee.parameters(),
            effective.words.len().checked_sub(1)?,
            dialect.parameter_grammar()?,
        )
        .ok()?;
        let argument = plan.into_iter().find_map(|binding| match binding {
            FormalArgumentBinding::Value {
                parameter: selected,
                argument,
            } if selected == parameter => Some(argument),
            _ => None,
        })?;
        let origin = *effective.origins.get(argument + 1)?;
        let values =
            crate::registry_invocation::declared_argument_words(invocation, &effective, dialect);
        let EffectiveInvocationWord::Literal(value) = values.get(argument)? else {
            return None;
        };
        let candidate = (origin, value.clone());
        if agreed
            .as_ref()
            .is_some_and(|previous| previous != &candidate)
        {
            return None;
        }
        agreed = Some(candidate);
    }
    agreed
}

#[cfg(test)]
mod tests {
    fn candidates(source: &str, dispatch: &str) -> Vec<String> {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let bindings = super::SourceCommandBindings::analyse(source, config, registry);
        let offset = u32::try_from(source.find(dispatch).unwrap()).unwrap();
        let origin = bindings.root_origin.as_ref().unwrap();
        bindings
            .declaration_formal_call_values(offset, registry)
            .unwrap_or_default()
            .iter()
            .filter_map(|candidate| candidate.value_in_source(origin).map(str::to_owned))
            .collect()
    }

    #[test]
    fn original_caller_alias_advice_uses_selected_formals_and_registry_links() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = tcl_lexer::LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        for (source, call, expected) in [
            (
                "proc maybe {target} {upvar 1 $target v; scan 42 %d v}; proc caller {} {set x 1; maybe x}",
                "maybe x",
                vec!["x"],
            ),
            (
                "proc maybe {target} {upvar 1 $target v}; interp alias {} link {} maybe; proc caller {} {set x 1; link x}",
                "link x",
                vec!["x"],
            ),
            (
                "proc maybe {target} {return $target}; namespace eval N {proc maybe {target} {upvar 1 $target v}; proc caller {} {set x 1; maybe x}}",
                "maybe x",
                vec!["x"],
            ),
            (
                "proc maybe {target} {upvar 1 $target v}; namespace eval N {proc maybe {target} {return $target}; proc caller {} {set x 1; maybe x}}",
                "maybe x",
                vec![],
            ),
            (
                "proc maybe {target} {upvar #0 $target v}; proc caller {} {set x 1; maybe x}",
                "maybe x",
                vec![],
            ),
        ] {
            let bindings = super::SourceCommandBindings::analyse(source, config, registry);
            let offset = source.rfind(call).unwrap() as u32;
            let segment =
                crate::segmenter::segment_commands_with_offset_and_config(call, offset, config)
                    .remove(0);
            let mut tokens = crate::ir::CommandTokens::from_segmented(
                &tcl_lexer::SourceImage::document(source).source_map(),
                config,
                &segment,
            );
            bindings.stamp_original_tokens(&mut tokens);
            let actual = bindings
                .declaration_caller_alias_arguments(&tokens, registry)
                .unwrap_or_default();
            assert_eq!(actual, expected, "{source}");
        }
    }

    #[test]
    fn original_formal_call_advice_keeps_allocation_binding_and_prefix_flow() {
        assert_eq!(
            candidates(
                "proc run {cmd} {$cmd 5}; proc d {} {run notacommand}",
                "$cmd 5"
            ),
            ["notacommand"]
        );
        assert_eq!(
            candidates("proc run {cmd} {$cmd 5}; proc d {} {run puts}", "$cmd 5"),
            ["puts"]
        );
        assert_eq!(
            candidates(
                "proc run {cmd} {$cmd 5}; interp alias {} go {} run notacommand; proc d {} {go}",
                "$cmd 5"
            ),
            ["notacommand"]
        );
        assert_eq!(
            candidates(
                "proc run {cmd} {$cmd 5}; proc d {} {run notacommand}; d",
                "$cmd 5"
            ),
            ["notacommand"]
        );
        for source in [
            "proc run {cmd} {set cmd puts; $cmd 5}; proc d {} {run notacommand}",
            "proc run {cmd} {$cmd 5}; proc d {} {return; run notacommand}",
            "proc run {cmd} {$cmd 5}; proc d {} {unknown_child; run notacommand}",
            "proc run {cmd} {$cmd 5}; proc run {cmd} {return DONE}; proc d {} {run notacommand}",
            "proc run {cmd} {$cmd 5}; proc d {} {run $name}",
        ] {
            assert!(candidates(source, "$cmd 5").is_empty(), "{source}");
        }
    }
}
