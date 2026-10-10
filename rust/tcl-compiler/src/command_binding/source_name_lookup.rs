// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original command-name geometry at an independently retained source point.

use super::{SourceInvocationBinding, SourceNamespaceKey};
use crate::signature_scan::scope::{
    SignatureNamespaceScope, SignatureSourceLookup, SignatureSourceNameInput,
    SignatureSourceNameKey,
};
use tcl_core_types::{ByteCommandSlot, NameBytes};

/// Original lookup geometry sealed at one authentic source invocation.
/// Complete word and readonly value producers remain distinct; candidates
/// supply no command existence, implementation, Normal result or edit grant.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OriginalCommandLookup {
    input: SignatureSourceNameInput,
    site: super::CommandAllocationSite,
    candidates: Vec<Vec<ByteCommandSlot>>,
    naming_scope: Option<SignatureNamespaceScope>,
    observations: Vec<super::declaration_layout::DeclarationLayoutObservation>,
    callback_scope: Option<tcl_registry::ScriptLookupScope>,
}

impl OriginalCommandLookup {
    /// Retained original producer, including readonly value lineage.
    #[must_use]
    pub const fn name_input(&self) -> &SignatureSourceNameInput {
        &self.input
    }
    /// Exact consuming source instance and command position.
    #[must_use]
    pub const fn site(&self) -> &super::CommandAllocationSite {
        &self.site
    }
    /// Independently retained native name policy.
    #[must_use]
    pub fn policy(&self) -> tcl_syntax::naming::NamePolicyProtocol {
        self.input.policy()
    }
    /// Ordered candidates for each authentic alternative namespace path.
    #[must_use]
    pub fn candidates(&self) -> &[Vec<ByteCommandSlot>] {
        &self.candidates
    }

    /// Caller naming geometry retained separately from the selected slot or
    /// namespace-path fallback. This grants no namespace existence or entered
    /// frame. Absolute lookup can remain valid when caller geometry is absent.
    #[must_use]
    pub fn original_naming_scope(&self) -> Option<&SignatureNamespaceScope> {
        self.naming_scope.as_ref()
    }

    /// Independently selected callback frame coordinates. These establish
    /// neither a future command table nor an entered callback activation.
    #[must_use]
    pub const fn callback_lookup_scope(&self) -> Option<tcl_registry::ScriptLookupScope> {
        self.callback_scope
    }

    pub(super) fn source_observations(
        &self,
    ) -> &[super::declaration_layout::DeclarationLayoutObservation] {
        &self.observations
    }

    /// First matching typed publications in the genuine ordered lookup paths.
    /// Every retained alternative must select the same metadata set; neither
    /// this matcher nor an empty match supplies command existence or liveness.
    #[must_use]
    pub fn matching_publications<'a, T: Clone + Eq>(
        &self,
        declarations: impl IntoIterator<
            Item = (&'a crate::signature_scan::scope::SignatureSourceCommand, T),
        >,
    ) -> Option<Vec<T>> {
        self.matching_slot_publications(
            declarations
                .into_iter()
                .map(|(name, metadata)| (name.slot(), name.policy(), metadata)),
        )
    }

    /// First matching exact slots under their independently selected policies.
    /// Every original lookup alternative must select the same metadata set.
    /// Publications can belong to another source; the caller owns that join.
    #[must_use]
    pub fn matching_slot_publications<'a, T: Clone + Eq>(
        &self,
        declarations: impl IntoIterator<
            Item = (
                &'a ByteCommandSlot,
                tcl_syntax::naming::NamePolicyProtocol,
                T,
            ),
        >,
    ) -> Option<Vec<T>> {
        let declarations = declarations.into_iter().collect::<Vec<_>>();
        let mut unanimous = None;
        for candidates in &self.candidates {
            let selected = crate::signature_scan::scope::first_matching_byte_slots(
                self.policy(),
                candidates,
                declarations
                    .iter()
                    .map(|(slot, policy, metadata)| (*slot, *policy, metadata.clone())),
            );
            if unanimous
                .as_ref()
                .is_some_and(|previous| previous != &selected)
            {
                return None;
            }
            unanimous = Some(selected);
        }
        unanimous
    }
}

impl SourceInvocationBinding {
    /// Original alias recipe at this call's actual post-argument lookup point.
    /// The called implementation allocation, original slot and policy must
    /// agree with the retained publication. A pre-argv lookup, a final-world
    /// publication or a surviving command token cannot replace that agreement.
    #[must_use]
    pub fn original_alias_target_for_reference(
        &self,
        reference: &super::SourceCommandReference,
    ) -> Option<super::source_command_world::OriginalAliasTarget> {
        use super::SourceCommandReferenceBinding;
        let site = self.invocation_site()?;
        if !reference.issued_at(site)
            || !matches!(
                reference.binding(),
                SourceCommandReferenceBinding::Direct {
                    kind: super::BindingKind::Alias,
                    ..
                } | SourceCommandReferenceBinding::Imported {
                    kind: super::BindingKind::Alias,
                    ..
                }
            )
        {
            return None;
        }
        let state = &self.lookup_state.as_ref()?.state;
        if state.has_opaque_domain() || state.source_step_observed() {
            return None;
        }
        let slot = reference.original_slot()?;
        let policy = reference.original_name_policy()?;
        let publication = state.original_publication_at(slot, policy)?;
        if !publication.has_implementation_allocation(reference.called_implementation_allocation()?)
        {
            return None;
        }
        publication.alias_target().cloned()
    }

    fn original_name_rows(
        &self,
        key: &SignatureSourceNameKey,
    ) -> Option<Vec<&super::declaration_layout::DeclarationLayoutObservation>> {
        let site = self.invocation_site()?;
        if site.source.source_image() != key.source_image()
            || site.offset != key.original_word().group().span.start()
        {
            return None;
        }
        let rows = super::declaration_layout::original_declaration_layouts(
            self.declaration_layout_observations.as_deref()?,
        )?;
        let first = rows.clone().next()?;
        if rows.clone().any(|row| {
            row.config != key.lexer_config()
                || row.namespace != first.namespace
                || row
                    .snapshot
                    .state
                    .baseline
                    .execution_name_policy
                    .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                    != Some(key.policy())
        }) {
            return None;
        }
        Some(rows.collect())
    }

    pub(super) fn original_operand_naming_scope(
        &self,
        input: &SignatureSourceNameInput,
    ) -> Option<SignatureNamespaceScope> {
        input.is_current(&self.variable_context).then_some(())?;
        let Some(observations) = self.declaration_layout_observations.as_deref() else {
            let snapshot = self.lookup_state.as_ref()?;
            if snapshot.state.current_source_origin.as_deref()
                != Some(self.invocation_site()?.source.as_ref())
                || snapshot
                    .state
                    .baseline
                    .execution_name_policy
                    .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                    != Some(input.policy())
                || !self.variable_context.namespace_known
            {
                return None;
            }
            return snapshot
                .state
                .original_namespace_geometry(&self.lookup_namespace_key, input.policy());
        };
        let rows = super::declaration_layout::original_declaration_layouts(observations)?;
        let mut found = None;
        for row in rows {
            if row
                .snapshot
                .state
                .baseline
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                != Some(input.policy())
            {
                return None;
            }
            let scope = if matches!(row.namespace, SourceNamespaceKey::Authored(_)) {
                self.variable_context.namespace_known.then_some(())?;
                row.snapshot
                    .state
                    .original_command_world
                    .conditional_scope(&row.namespace, input.policy())?
            } else {
                original_home_scope(self, row, input.policy())?
            };
            if found.as_ref().is_some_and(|previous| previous != &scope) {
                return None;
            }
            found = Some(scope);
        }
        found
    }

    fn original_name_scope(&self, key: &SignatureSourceNameKey) -> Option<SignatureNamespaceScope> {
        let rows = self.original_name_rows(key)?;
        let mut scope = None;
        for row in rows {
            let selected = original_input_scope(
                self,
                row,
                &SignatureSourceNameInput::OriginalWord(key.clone()),
            )?;
            if scope.as_ref().is_some_and(|previous| previous != &selected) {
                return None;
            }
            scope = Some(selected);
        }
        scope
    }

    /// Actual selected Registry implementation for this original source head.
    /// Alias prefixes, uncertain targets and missing original observations
    /// cannot donate a static index declaration grammar or normal completion.
    #[must_use]
    pub fn original_registry_target_for_name(
        &self,
        key: &SignatureSourceNameKey,
    ) -> Option<super::SourceCommandTarget> {
        let rows = self.original_name_rows(key);
        #[cfg(test)]
        if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_PACKAGE_LOOKUP").is_some() {
            eprintln!(
                "ORIGINAL_PACKAGE_LOOKUP offset={} stage=rows rows={} site={} input_bytes={:?}",
                key.original_word().group().span.start(),
                rows.as_ref().map_or(0, Vec::len),
                self.invocation_site().is_some(),
                key.bytes()
            );
        }
        let rows = rows?;
        let mut unanimous = None;
        for row in rows {
            #[cfg(test)]
            if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_PACKAGE_LOOKUP").is_some() {
                eprintln!(
                    "ORIGINAL_PACKAGE_LOOKUP offset={} stage=snapshot opaque={} step={} execution={} namespace_known={} policy_agrees={}",
                    key.original_word().group().span.start(),
                    row.snapshot.state.has_opaque_domain(),
                    row.snapshot.state.source_step_observed(),
                    row.snapshot.state.source_execution_observed(None),
                    row.snapshot.state.namespaces.contains(&row.namespace),
                    row.snapshot
                        .state
                        .baseline
                        .execution_name_policy
                        .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                        == Some(key.policy())
                );
            }
            let advice = self.original_registry_layout_for_name_row(key, row);
            #[cfg(test)]
            if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_PACKAGE_LOOKUP").is_some() {
                eprintln!(
                    "ORIGINAL_PACKAGE_LOOKUP offset={} stage=advice present={} closed={} opaque={} targets={}",
                    key.original_word().group().span.start(),
                    advice.is_some(),
                    advice
                        .as_ref()
                        .is_some_and(super::OriginalCompilationLookupAdvice::closed_lookup),
                    advice.as_ref().is_some_and(
                        super::OriginalCompilationLookupAdvice::has_opaque_handler_alternatives
                    ),
                    advice.as_ref().map_or(0, |advice| advice.targets().len())
                );
            }
            let advice = advice?;
            if !advice.closed_lookup() || advice.has_opaque_handler_alternatives() {
                return None;
            }
            let target = advice.targets().first()?;
            if !target.prepended.is_empty()
                || !target.registry_backed
                || advice
                    .targets()
                    .iter()
                    .any(|alternative| alternative != target)
                || unanimous
                    .as_ref()
                    .is_some_and(|previous| previous != target)
            {
                return None;
            }
            unanimous = Some(target.clone());
        }
        unanimous
    }

    fn original_registry_layout_for_name_row(
        &self,
        key: &SignatureSourceNameKey,
        row: &super::declaration_layout::DeclarationLayoutObservation,
    ) -> Option<super::OriginalCompilationLookupAdvice> {
        // Reuse the complete original vector owner, then project its actual
        // segmented command. No argument evaluation or execution binding
        // is required to ask the retained pre-argv handler for its grammar.
        let image = key.source_image();
        let native = crate::registry_invocation::original_native_compiler_words(
            image,
            &row.words,
            self.invocation_site()?.offset,
            row.config,
        )?;
        let region =
            tcl_lexer::Span::new(native.first()?.span().start(), native.last()?.span().end());
        let selected = tcl_lexer::SourceImage::from_bytes(
            image.bytes().get(region.as_range())?,
            image.channel(),
        );
        let commands = crate::segmenter::segment_commands_image_with_offset_and_config(
            &selected,
            region.start(),
            row.config,
        )?;
        let [command] = commands.as_slice() else {
            return None;
        };
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::from_image(image),
            row.config,
            command,
        );
        if tokens.words() != row.words.as_ref() {
            return None;
        }
        super::original_site_operand_layout_advice(
            self.invocation_site()?,
            &tokens,
            &row.snapshot,
            &row.namespace,
            row.config,
        )
    }

    /// Exact original command lookup candidates in the retained caller scope.
    /// Missing source/configuration/scope ownership cannot borrow a displayed
    /// namespace. Candidate geometry grants no command existence or dispatch.
    #[must_use]
    pub fn original_command_name_lookup(
        &self,
        key: &SignatureSourceNameKey,
    ) -> Option<SignatureSourceLookup> {
        SignatureSourceLookup::from_key(self.original_name_scope(key)?, key)
    }

    /// Original lookup geometry at the genuine consuming command owner.
    /// A readonly value must be the independently frozen actual head input;
    /// source coordinates and equal bytes cannot manufacture that association.
    #[must_use]
    pub fn original_command_lookup(
        &self,
        tokens: &crate::ir::CommandTokens,
        input: &SignatureSourceNameInput,
    ) -> Option<OriginalCommandLookup> {
        let rows = match input {
            SignatureSourceNameInput::OriginalWord(key) => self.original_name_rows(key)?,
            SignatureSourceNameInput::OriginalValue(_) => {
                if self.original_head_name_input(tokens).as_ref() != Some(input)
                    || !input.is_current(&self.variable_context)
                {
                    return None;
                }
                let rows = super::declaration_layout::original_declaration_layouts(
                    self.declaration_layout_observations.as_deref()?,
                )?;
                rows.collect()
            }
            SignatureSourceNameInput::OriginalVariableRoot(_) => return None,
        };
        self.original_lookup_from_rows(input, rows)
    }

    /// Complete written static head under its genuine declaration observations.
    /// This supplies source lookup geometry, never entered dispatch or presence.
    pub(crate) fn original_static_head_lookup_for_tokens(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<OriginalCommandLookup> {
        if tokens.synthetic.is_some() || tokens.source_binding.as_ref() != Some(self) {
            return None;
        }
        let rows = super::declaration_layout::original_declaration_layouts(
            self.declaration_layout_observations.as_deref()?,
        )?;
        let first = rows.clone().next()?;
        let policy = first
            .snapshot
            .state
            .baseline
            .execution_name_policy?
            .native_recipe()?;
        let site = self.invocation_site()?;
        let original = crate::registry_invocation::original_native_compiler_words(
            site.source.source_image(),
            tokens.words(),
            site.offset,
            first.config,
        )?;
        if original.iter().any(|word| word.group().expand) {
            return None;
        }
        let key = SignatureSourceNameKey::from_original_native_word(
            original.first()?,
            tcl_syntax::word_rules::WordValueRules::from_config(&first.config),
            policy,
        )?;
        self.original_command_lookup(tokens, &SignatureSourceNameInput::OriginalWord(key))
    }

    /// Original static head lookup without manufacturing a command vector.
    /// The complete retained word, source instance, configuration and actual
    /// invocation site must agree with every original observation.
    #[must_use]
    pub fn original_static_command_lookup(
        &self,
        key: &SignatureSourceNameKey,
    ) -> Option<OriginalCommandLookup> {
        self.original_lookup_from_rows(
            &SignatureSourceNameInput::OriginalWord(key.clone()),
            self.original_name_rows(key)?,
        )
    }

    pub(super) fn original_lookup_from_rows(
        &self,
        input: &SignatureSourceNameInput,
        rows: Vec<&super::declaration_layout::DeclarationLayoutObservation>,
    ) -> Option<OriginalCommandLookup> {
        let mut unanimous = None;
        let mut naming_scope = None;
        let mut naming_scope_available = true;
        for row in &rows {
            if row
                .snapshot
                .state
                .baseline
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                != Some(input.policy())
            {
                return None;
            }
            let scope = original_input_scope(self, row, input)?;
            let home = original_home_scope(self, row, input.policy());
            if let Some(home) = home {
                if naming_scope
                    .as_ref()
                    .is_some_and(|previous| previous != &home)
                {
                    naming_scope_available = false;
                }
                naming_scope = Some(home);
            } else {
                naming_scope_available = false;
            }
            let candidates = if matches!(row.namespace, SourceNamespaceKey::Authored(_)) {
                if row
                    .snapshot
                    .state
                    .unknown_lookup_namespaces
                    .contains(&row.namespace)
                    && !input.bytes().starts_with(b"::")
                {
                    return None;
                }
                original_authored_candidates(row, input, &scope)?
            } else {
                let current = if input.bytes().starts_with(b"::") {
                    row.snapshot.state.native_root_namespace_key()?
                } else {
                    row.namespace.clone()
                };
                row.snapshot
                    .state
                    .native_source_lookup_paths_bytes(input.bytes(), &current)
                    .ok()?
                    .into_iter()
                    .map(|path| {
                        path.into_iter()
                            .map(|candidate| {
                                let super::SourceCommandKey::Slot { namespace, simple } = candidate
                                else {
                                    return None;
                                };
                                let namespace = match input.policy().recipe() {
                                    tcl_syntax::naming::NativeNameProtocol::C(_) => {
                                        namespace.exact_native_path()?.clone()
                                    }
                                    tcl_syntax::naming::NativeNameProtocol::Jim084 => {
                                        tcl_core_types::ByteNamespacePath::root()
                                    }
                                };
                                Some(ByteCommandSlot { namespace, simple })
                            })
                            .collect::<Option<Vec<_>>>()
                    })
                    .collect::<Option<Vec<_>>>()?
            };
            if unanimous
                .as_ref()
                .is_some_and(|previous| previous != &candidates)
            {
                return None;
            }
            unanimous = Some(candidates);
        }
        Some(OriginalCommandLookup {
            input: input.clone(),
            site: self.invocation_site()?.clone(),
            candidates: unanimous?,
            naming_scope: naming_scope_available.then_some(naming_scope).flatten(),
            observations: rows.into_iter().cloned().collect(),
            callback_scope: None,
        })
    }

    pub(super) fn original_callback_lookup_from_rows(
        &self,
        input: &SignatureSourceNameInput,
        rows: Vec<&super::declaration_layout::DeclarationLayoutObservation>,
        scope: tcl_registry::ScriptLookupScope,
    ) -> Option<OriginalCommandLookup> {
        use tcl_registry::ScriptLookupScope;
        let mut lookup = match scope {
            ScriptLookupScope::GlobalFrame | ScriptLookupScope::TriggerFrame => {
                if rows.is_empty()
                    || rows.iter().any(|row| {
                        row.snapshot
                            .state
                            .baseline
                            .execution_name_policy
                            .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                            != Some(input.policy())
                    })
                {
                    return None;
                }
                let root = SignatureNamespaceScope::root(Some(input.policy()));
                if scope == ScriptLookupScope::TriggerFrame {
                    // The original native naming owner, including counted/CString
                    // boundaries, decides qualification. Absolute target slots
                    // need no unknown triggering namespace; relative names do.
                    let recipe = input.policy().recipe();
                    if !matches!(recipe, tcl_syntax::naming::NativeNameProtocol::C(_))
                        || recipe
                            .command_lookup_input(root.context()?, input.bytes())
                            .ok()?
                            .qualification()
                            != tcl_syntax::naming::NativeNameQualification::Absolute
                    {
                        return None;
                    }
                }
                let candidates =
                    SignatureSourceLookup::from_input(root.clone(), input)?.candidates()?;
                OriginalCommandLookup {
                    input: input.clone(),
                    site: self.invocation_site()?.clone(),
                    candidates: vec![candidates],
                    naming_scope: (scope == ScriptLookupScope::GlobalFrame).then_some(root),
                    observations: rows.into_iter().cloned().collect(),
                    callback_scope: None,
                }
            }
            ScriptLookupScope::InvokingFrame => {
                if self.original_argument_completion.is_normal() {
                    let snapshot = self.lookup_state.as_ref()?;
                    let actual = rows
                        .iter()
                        .map(|row| {
                            if snapshot.state.variable_frame != *row.entry.frame() {
                                return None;
                            }
                            let mut row = (**row).clone();
                            row.snapshot = std::sync::Arc::clone(snapshot);
                            Some(row)
                        })
                        .collect::<Option<Vec<_>>>()?;
                    self.original_lookup_from_rows(input, actual.iter().collect())?
                } else {
                    self.original_lookup_from_rows(input, rows)?
                }
            }
        };
        lookup.callback_scope = Some(scope);
        Some(lookup)
    }

    /// Ordered original candidates, retaining namespace paths and incarnations.
    /// Missing paths or conflicting observations cannot supply root fallback.
    #[must_use]
    pub fn original_command_lookup_candidates(
        &self,
        key: &SignatureSourceNameKey,
    ) -> Option<Vec<Vec<ByteCommandSlot>>> {
        let rows = self.original_name_rows(key)?;
        Some(
            self.original_lookup_from_rows(
                &SignatureSourceNameInput::OriginalWord(key.clone()),
                rows,
            )?
            .candidates,
        )
    }

    /// C library `auto_qualify` candidates from the original name producer and
    /// independently retained caller namespace. Jim and unavailable scopes
    /// supply no C loader recipe or implicit global fallback.
    #[must_use]
    pub fn original_autoload_command_candidates(
        &self,
        key: &SignatureSourceNameKey,
    ) -> Option<Vec<NameBytes>> {
        let SignatureNamespaceScope::C(path) = self.original_name_scope(key)? else {
            return None;
        };
        tcl_syntax::naming::native_autoload_command_candidates(
            key.policy().string_protocol(),
            key.bytes(),
            &tcl_syntax::naming::native_namespace_full_name_bytes(&path),
        )
        .map(|values| values.into_iter().map(NameBytes::from).collect())
    }
}

fn original_input_scope(
    binding: &SourceInvocationBinding,
    row: &super::declaration_layout::DeclarationLayoutObservation,
    input: &SignatureSourceNameInput,
) -> Option<SignatureNamespaceScope> {
    if input.bytes().starts_with(b"::") {
        return Some(SignatureNamespaceScope::root(Some(input.policy())));
    }
    original_home_scope(binding, row, input.policy())
}

fn original_home_scope(
    binding: &SourceInvocationBinding,
    row: &super::declaration_layout::DeclarationLayoutObservation,
    policy: tcl_syntax::naming::NamePolicyProtocol,
) -> Option<SignatureNamespaceScope> {
    match &row.namespace {
        SourceNamespaceKey::Native(context) => {
            let entry = row.snapshot.state.baseline.native_entry.as_ref()?;
            if entry.retained_namespace_context(context.token).ok()? != *context
                || entry.command_name_policy()? != policy
            {
                return None;
            }
            match policy.recipe() {
                tcl_syntax::naming::NativeNameProtocol::C(_) => {
                    Some(SignatureNamespaceScope::C(context.path.clone()))
                }
                tcl_syntax::naming::NativeNameProtocol::Jim084 => {
                    Some(SignatureNamespaceScope::Jim(
                        entry
                            .namespace_context(context.token)
                            .ok()?
                            .jim_namespace_object
                            .as_ref()?
                            .clone(),
                    ))
                }
            }
        }
        SourceNamespaceKey::Allocated { path, .. }
            if matches!(
                policy.recipe(),
                tcl_syntax::naming::NativeNameProtocol::C(_)
            ) =>
        {
            Some(SignatureNamespaceScope::C(path.clone()))
        }
        SourceNamespaceKey::Authored(namespace) if namespace == "::" => {
            (binding.variable_context.namespace_known
                && row.snapshot.state.baseline.native_entry.is_none())
            .then(|| SignatureNamespaceScope::root(Some(policy)))
        }
        SourceNamespaceKey::Authored(_) => row
            .snapshot
            .state
            .original_command_world
            .scope(&row.namespace, policy),
        SourceNamespaceKey::Allocated { .. } => None,
    }
}

fn original_authored_candidates(
    row: &super::declaration_layout::DeclarationLayoutObservation,
    input: &SignatureSourceNameInput,
    scope: &SignatureNamespaceScope,
) -> Option<Vec<Vec<ByteCommandSlot>>> {
    use tcl_syntax::naming::{NativeNameProtocol, NativeNameQualification};
    let policy = input.policy();
    let recipe = policy.recipe();
    if recipe == NativeNameProtocol::Jim084 {
        // Jim's flat current/global candidates are a distinct lookup purpose;
        // even an absolute name has no C namespace-table slot projection.
        return Some(vec![
            SignatureSourceLookup::from_input(scope.clone(), input)?.candidates()?,
        ]);
    }
    let selected = recipe
        .command_lookup_input(scope.context()?, input.bytes())
        .ok()?;
    if selected.qualification() == NativeNameQualification::Absolute {
        return Some(vec![vec![
            recipe
                .command_lookup_slot(scope.context()?, input.bytes())
                .ok()?,
        ]]);
    }
    let state = &row.snapshot.state;
    let uses_paths =
        matches!(recipe, NativeNameProtocol::C(version) if version.has_namespace_path());
    if uses_paths && state.unknown_namespace_paths.contains(&row.namespace) {
        return None;
    }
    let mut result = Vec::new();
    let paths = state.namespace_paths.get(&row.namespace);
    for path in paths.into_iter().flatten() {
        let mut candidates = vec![
            recipe
                .command_lookup_slot(scope.context()?, input.bytes())
                .ok()?,
        ];
        if uses_paths {
            for namespace in path {
                if !state.namespaces.contains(namespace) {
                    continue;
                }
                let path_scope = state.original_command_world.scope(namespace, policy)?;
                let candidate = recipe
                    .command_lookup_slot(path_scope.context()?, input.bytes())
                    .ok()?;
                if !candidates.contains(&candidate) {
                    candidates.push(candidate);
                }
            }
        }
        let root = SignatureNamespaceScope::root(Some(policy));
        let global = recipe
            .command_lookup_slot(root.context()?, input.bytes())
            .ok()?;
        if !candidates.contains(&global) {
            candidates.push(global);
        }
        result.push(candidates);
    }
    if result.is_empty() {
        result.push(SignatureSourceLookup::from_input(scope.clone(), input)?.candidates()?);
    }
    Some(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_lexer::{LexerConfig, SourceImage, Span};
    use tcl_syntax::word_rules::WordValueRules;

    fn original(
        source: &str,
        dialect: &str,
        offset: usize,
    ) -> (SignatureSourceNameKey, SourceInvocationBinding) {
        let registry = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let point = tcl_dialect::model::DialectPoint::of_dialect_name(Some(dialect)).unwrap();
        let dialect = tcl_registry::InvocationDialect::of_point(point);
        let config = LexerConfig::from_grammar(dialect.lexer_grammar);
        let image = SourceImage::document(source);
        let plan = tcl_lexer::native_script_words_in(
            image,
            Span::new(0, u32::try_from(source.len()).unwrap()),
            config,
        )
        .unwrap();
        let word = plan
            .commands
            .iter()
            .flat_map(|command| &command.words)
            .find(|word| word.span().start() == u32::try_from(offset).unwrap())
            .unwrap();
        let policy = dialect.authored_name_policy().unwrap();
        let key = SignatureSourceNameKey::from_original_native_word(
            word,
            WordValueRules::from_config(&config),
            policy,
        )
        .unwrap();
        let bindings = super::super::SourceCommandBindings::analyse_with_options(
            source,
            config,
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                ..super::super::SourceAnalysisOptions::default()
            },
        );
        let binding =
            bindings.invocation_at_source(word.try_text().unwrap(), u32::try_from(offset).unwrap());
        (key, binding)
    }

    #[test]
    fn original_autoload_uses_exact_name_and_caller_receipt() {
        for dialect in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let (key, binding) = original(r"missing\u0000tail", dialect, 0);
            assert_eq!(
                binding.original_autoload_command_candidates(&key).unwrap(),
                vec![NameBytes::from(b"missing\xc0\x80tail".as_slice())]
            );
            let (foreign, _) = original("different", dialect, 0);
            assert!(
                binding
                    .original_autoload_command_candidates(&foreign)
                    .is_none()
            );
            for source in ["{missing}", "\"missing\"", "::missing"] {
                let (key, binding) = original(source, dialect, 0);
                assert_eq!(
                    binding
                        .original_autoload_command_candidates(&key)
                        .unwrap()
                        .first()
                        .unwrap()
                        .as_bytes(),
                    b"missing"
                );
            }
        }
        let (key, binding) = original("missing", "jimtcl", 0);
        assert!(binding.original_autoload_command_candidates(&key).is_none());
    }

    #[test]
    fn original_package_index_handlers_retain_each_genuine_registration_point() {
        // Implementation contract: naming.source.package-index-entry-values
        // docs/design/analysis/name-resolution-proofs/package-index-entry-values.md
        let cases = [
            (
                "package ifneeded p 1.0 {source p.tcl}\npackage ifneeded q 2.0 {source q.tcl}",
                "package",
                2,
                0,
            ),
            (
                "if {1} {package ifneeded p\\uD800 1.0 {source p.tcl}}\npackage ifneeded p\\uD801 2.0 {source q.tcl}",
                "package",
                2,
                0,
            ),
            (
                "set dir /package\nset {auto_index(p\\u0000tail)} [list source [file join $dir p.tcl]]\nset {auto_index( spaced )} [list source [file join $dir q.tcl]]",
                "set",
                2,
                "set dir /package\n".len(),
            ),
        ];
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            let config = LexerConfig::from_grammar(dialect.lexer_grammar);
            let registry =
                tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
            let policy = dialect.authored_name_policy().unwrap();
            for (source, head, expected, registration_from) in cases {
                let image = SourceImage::document(source);
                let bindings = super::super::SourceCommandBindings::analyse_with_options(
                    source,
                    config,
                    registry,
                    super::super::SourceAnalysisOptions {
                        invocation_dialect: Some(dialect),
                        native_compilation:
                            crate::environment_ingress::authoring_native_compilation(),
                        ..Default::default()
                    },
                );
                let mut selected = 0;
                for (offset, _) in source
                    .match_indices(head)
                    .filter(|(offset, _)| *offset >= registration_from)
                {
                    // Select only an independently parsed complete original
                    // head word; the consumer cannot author a token vector.
                    let start = u32::try_from(offset).unwrap();
                    let end = start + u32::try_from(head.len()).unwrap();
                    let plan = tcl_lexer::native_script_words_in(
                        image.clone(),
                        Span::new(start, end),
                        config,
                    )
                    .unwrap();
                    let [command] = plan.commands.as_slice() else {
                        panic!("one original head extent")
                    };
                    let [word] = command.words.as_slice() else {
                        panic!("one complete original head word")
                    };
                    let key = SignatureSourceNameKey::from_original_native_word(
                        word,
                        WordValueRules::from_config(&config),
                        policy,
                    )
                    .unwrap();
                    let binding = bindings.invocation_at_source(head, start);
                    if let Some(target) = binding.original_registry_target_for_name(&key) {
                        assert_eq!(
                            registry
                                .get(target.registry_identity().unwrap())
                                .unwrap()
                                .name,
                            head
                        );
                        selected += 1;
                    }
                }
                assert_eq!(selected, expected, "{version:?}: {source}");
            }
        }
    }

    #[test]
    fn original_package_index_unknown_entry_value_cannot_close_registration_argv() {
        // Implementation contract: naming.source.package-index-entry-values
        // docs/design/analysis/name-resolution-proofs/package-index-entry-values.md
        let source = "set {auto_index(p\\u0000tail)} [list source [file join $dir p.tcl]]";
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let config = LexerConfig::from_grammar(registry.profile().unwrap().grammar);
        let bindings = super::super::SourceCommandBindings::analyse(source, config, registry);
        assert!(bindings.original_completed_command_world().is_none());
        let segment =
            crate::segmenter::segment_commands_with_offset_and_config(source, 0, config).remove(0);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &SourceImage::document(source).source_map(),
            config,
            &segment,
        );
        bindings.stamp_original_tokens(&mut tokens);
        assert!(
            crate::registry_invocation::normal_transfer_invocation(registry, None, &tokens)
                .is_none()
        );
        // Braced source retains the ASCII escape spelling in its array index;
        // it is a different producer from an evaluated counted-NUL name.
        let native = crate::registry_invocation::original_native_compiler_words(
            &SourceImage::document(source),
            tokens.words(),
            0,
            config,
        )
        .unwrap();
        let name = SignatureSourceNameKey::from_original_native_word(
            &native[1],
            WordValueRules::from_config(&config),
            tcl_registry::InvocationDialect::of_profile(registry.profile().unwrap())
                .authored_name_policy()
                .unwrap(),
        )
        .unwrap();
        assert_eq!(name.bytes(), b"auto_index(p\\u0000tail)");
    }
}
