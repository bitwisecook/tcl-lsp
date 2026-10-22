/// One callee's invocation-independent caller-alias survey.
#[derive(Debug)]
pub(crate) struct DeclarationCallerAliasSurvey {
    callee: std::sync::Arc<SourceConditionalBodyEntry>,
    parameters: Vec<usize>,
}

type DeclarationCallerAliasKey = (
    super::CommandAllocation,
    tcl_registry::RegistrySemanticKey,
    crate::registry_invocation::OwnedInvocationMetadataContext,
);

/// Realm-owned memo of [`DeclarationCallerAliasSurvey`] per callee allocation,
/// registry semantics and metadata owner. Clones of one immutable realm share
/// it; a new realm starts empty. A missing callee is retained as `None`.
#[derive(Debug, Clone, Default)]
pub(crate) struct DeclarationCallerAliasMemo {
    entries: std::sync::Arc<
        std::sync::Mutex<
            std::collections::HashMap<
                DeclarationCallerAliasKey,
                Option<std::sync::Arc<DeclarationCallerAliasSurvey>>,
            >,
        >,
    >,
}

impl DeclarationCallerAliasMemo {
    fn get_or_survey(
        &self,
        allocation: &super::CommandAllocation,
        registry: &tcl_registry::CommandRegistry,
        owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
        survey: impl FnOnce() -> Option<DeclarationCallerAliasSurvey>,
    ) -> Option<std::sync::Arc<DeclarationCallerAliasSurvey>> {
        let key = (
            allocation.clone(),
            registry.snapshot().semantic_key(),
            owner.clone(),
        );
        if let Some(retained) = self
            .entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .get(&key)
        {
            return retained.clone();
        }
        let surveyed = survey().map(std::sync::Arc::new);
        self.entries
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .entry(key)
            .or_insert(surveyed)
            .clone()
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
        owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
    ) -> Option<Vec<String>> {
        self.declaration_caller_alias_arguments_with(invocation, registry, owner, None)
    }

    /// As [`Self::declaration_caller_alias_arguments`], sharing the
    /// invocation-independent callee survey through `memo`. The memo belongs
    /// to one immutable realm, so its inventories cannot change underneath it.
    pub(crate) fn declaration_caller_alias_arguments_with(
        &self,
        invocation: &crate::ir::CommandTokens,
        registry: &tcl_registry::CommandRegistry,
        owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
        memo: Option<&DeclarationCallerAliasMemo>,
    ) -> Option<Vec<String>> {
        invocation
            .source_binding
            .as_ref()?
            .original_invocation_metadata_for_owner(invocation, owner, registry)
            .ok()?;
        let advice = self.declaration_call_layout_advice(invocation)?;
        let mut agreed = None;
        for target in advice.targets() {
            if target.registry_backed || target.kind != BindingKind::Proc {
                return None;
            }
            let allocation = target.implementation_allocation.as_ref()?;
            let survey = match memo {
                Some(memo) => memo.get_or_survey(allocation, registry, owner, || {
                    self.declaration_caller_alias_survey(allocation, registry, owner)
                })?,
                None => std::sync::Arc::new(self.declaration_caller_alias_survey(
                    allocation, registry, owner,
                )?),
            };
            let mut values = Vec::new();
            for &parameter in &survey.parameters {
                if let Some((_, value)) =
                    original_formal_argument(self, invocation, &survey.callee, parameter)
                    && !values.contains(&value)
                {
                    values.push(value);
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

    /// The callee and, in declaration-site order, every formal its original
    /// body passes to a caller-alias operand. Independent of the invocation.
    fn declaration_caller_alias_survey(
        &self,
        allocation: &super::CommandAllocation,
        registry: &tcl_registry::CommandRegistry,
        owner: &crate::registry_invocation::OwnedInvocationMetadataContext,
    ) -> Option<DeclarationCallerAliasSurvey> {
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
        let mut parameters = Vec::new();
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
                    (registry, owner),
                ) else {
                    continue;
                };
                if let Some(parameter) = callee
                    .parameters()
                    .iter()
                    .position(|parameter| parameter.name == formal)
                {
                    parameters.push(parameter);
                }
            }
        }
        Some(DeclarationCallerAliasSurvey { callee, parameters })
    }

