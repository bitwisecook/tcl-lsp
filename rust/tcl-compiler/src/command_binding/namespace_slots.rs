// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source table operations over retained namespace geometry and incarnations.

use super::{
    AllocationIncarnation, Arc, BTreeSet, CommandAllocationSite, ModuleCommandBindings,
    NamespaceKeyQuery, SourceCommandKey, SourceNamespaceKey,
};
use crate::signature_scan::scope::{SignatureNamespaceScope, SignatureSourceNameInput};
use tcl_core_types::ByteNamespacePath;
use tcl_registry::NamespaceTransitionTarget;
use tcl_syntax::naming::NativeNameContext;

#[derive(Clone, Copy)]
pub(super) enum PublicationPurpose {
    Procedure,
    Rename,
    Alias,
    Command,
}

/// A unique namespace geometry may prove absence; ambiguous geometry is unavailable.
pub(super) enum OriginalNamespaceKeyPresence {
    Unavailable,
    Available(Option<SourceNamespaceKey>),
}

impl ModuleCommandBindings {
    /// Select an original declared namespace operand from the actual root.
    /// This never interprets a displayed namespace label as written input.
    pub(super) fn namespace_for_rooted_operand(&self, written: &str) -> Option<SourceNamespaceKey> {
        let root = self.source_root_namespace_key()?;
        self.namespace_target_key_at(
            &NamespaceTransitionTarget::Named(tcl_registry::TransitionSubject::Literal(
                written.to_owned(),
            )),
            &root,
        )
    }

    /// The complete original lambda list supplies either an exact namespace
    /// child or independently proved omission. Display strings supply neither.
    pub(super) fn namespace_for_lambda_operand(
        &self,
        lambda: &SignatureSourceNameInput,
        config: tcl_lexer::LexerConfig,
    ) -> Option<SourceNamespaceKey> {
        let recipe = self.baseline.dialect?.native_name_protocol()?;
        let policy = self.baseline.execution_name_policy?.native_recipe()?;
        if lambda.policy() != policy
            || recipe != policy.recipe()
            || !lambda.is_current(&self.source_variables)
        {
            return None;
        }
        if let Some(container) = lambda.original_static_list_container() {
            let word = container.parent_word();
            if self.current_source_origin.as_ref()?.source_image() != word.image()
                || config != word.config()
            {
                return None;
            }
        }
        if !recipe.is_jim084() {
            recipe.lambda_namespace_input(b"").ok()?;
        }
        let children = lambda.original_list_elements()?;
        let key = match children.as_slice() {
            [_, _] => self.source_root_namespace_key()?,
            [_, _, namespace] if recipe == tcl_syntax::naming::NativeNameProtocol::Jim084 => {
                // Jim retains its independent source namespace recipe; no C
                // prefix or namespace-tree authority is transferred to it.
                self.namespace_for_rooted_operand(std::str::from_utf8(namespace.bytes()).ok()?)?
            }
            [_, _, namespace] => {
                let path = recipe.lambda_namespace_path(namespace.bytes()).ok()?;
                match self.original_namespace_key_for_path(&path, policy) {
                    OriginalNamespaceKeyPresence::Available(Some(key)) => key,
                    OriginalNamespaceKeyPresence::Available(None)
                    | OriginalNamespaceKeyPresence::Unavailable => return None,
                }
            }
            _ => return None,
        };
        (!self.unknown_lookup_namespaces.contains(&key)).then_some(key)
    }

    /// Exact current variable-frame namespace; symbolic compatibility is
    /// available only without a supplied native world.
    pub(super) fn variable_frame_namespace_key(&self) -> Option<SourceNamespaceKey> {
        if !self.source_variables.namespace_known {
            return None;
        }
        self.source_variables
            .namespace_identity
            .clone()
            .or_else(|| {
                (self.baseline.native_entry.is_none() && self.source_variables.namespace_known)
                    .then(|| SourceNamespaceKey::authored(&self.source_variables.namespace))
            })
    }

    /// Revalidate an already captured receiver/implementation by its retained
    /// command object. No displayed command name is converted into an address.
    pub(super) fn retained_target_is_current(&self, target: &super::SourceCommandTarget) -> bool {
        let Some(identity) = target.identity.as_ref() else {
            return false;
        };
        let matches = |binding: &super::MayBinding| {
            matches!(binding,
                super::MayBinding::Target(current) if current.terminal
                    && current.token.as_ref() == Some(identity)
                    && current.registry_backed == target.registry_backed
                    && current.kind == target.kind
                    && current.implementation_generation == target.implementation_generation
                    && current.implementation_allocation == target.implementation_allocation
                    && current.prepended == target.prepended
            )
        };
        if self.bindings.values().any(|bindings| {
            bindings.len() == 1
                && bindings.iter().any(|binding| {
                    matches(binding)
                        || match binding {
                            super::MayBinding::Imported(imported)
                                if &imported.origin == identity =>
                            {
                                self.objects.get(identity).is_some_and(|objects| {
                                    objects.len() == 1 && objects.iter().any(matches)
                                })
                            }
                            _ => false,
                        }
                })
        }) {
            return true;
        }
        // An unmodified authored row may remain implicit in its registry base.
        self.baseline.native_entry.is_none()
            && super::source_binding(self, &target.command, "::").proved_target() == Some(target)
    }

    /// Diagnostic/source inventory presentation; never re-entered as native lookup input.
    pub(super) fn command_key_label(key: &SourceCommandKey) -> Option<String> {
        match key {
            SourceCommandKey::Authored(label) => Some(label.clone()),
            SourceCommandKey::Slot { namespace, simple } => {
                let holder = namespace.display()?;
                let simple = simple.try_utf8().ok()?;
                Some(if holder == "::" {
                    format!("::{simple}")
                } else {
                    format!("{holder}::{simple}")
                })
            }
        }
    }

    pub(super) fn all_command_keys(&self) -> BTreeSet<SourceCommandKey> {
        self.bindings
            .keys()
            .cloned()
            .chain(
                self.baseline
                    .semantics
                    .binding_names()
                    .iter()
                    .cloned()
                    .map(SourceCommandKey::authored),
            )
            .collect()
    }

    pub(super) fn namespace_key_for_path(
        &self,
        path: &ByteNamespacePath,
    ) -> Option<SourceNamespaceKey> {
        let mut keys = self
            .namespaces
            .iter()
            .filter(|key| key.exact_native_path() == Some(path));
        let key = keys.next()?.clone();
        keys.next().is_none().then_some(key)
    }

    /// Select only a namespace already reached through the original operand's
    /// counted geometry. A reporting name cannot supply this path or lifetime.
    pub(super) fn original_namespace_key_for_input(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
    ) -> Option<SourceNamespaceKey> {
        let path = self.original_namespace_path_for_input(current, input)?;
        match self.original_namespace_key_for_path(&path, input.policy()) {
            OriginalNamespaceKeyPresence::Available(key) => key,
            OriginalNamespaceKeyPresence::Unavailable => None,
        }
    }

    fn original_namespace_path_for_input(
        &self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
    ) -> Option<ByteNamespacePath> {
        let word = input.original_word_key()?;
        let policy = input.policy();
        if !matches!(
            policy.recipe(),
            tcl_syntax::naming::NativeNameProtocol::C(_)
        ) || self.baseline.execution_name_policy?.native_recipe()? != policy
            || self.current_source_origin.as_ref()?.source_image() != word.source_image()
            || !input.is_current(&self.source_variables)
            || self.unknown_lookup_namespaces.contains(current)
        {
            return None;
        }
        let scope = self.original_namespace_geometry(current, policy)?;
        policy
            .recipe()
            .namespace_address_path(scope.context()?, input.bytes())
            .ok()
    }

    pub(super) fn original_namespace_key_for_path(
        &self,
        path: &ByteNamespacePath,
        policy: tcl_syntax::naming::NamePolicyProtocol,
    ) -> OriginalNamespaceKeyPresence {
        let scope = SignatureNamespaceScope::C(path.clone());
        let mut keys = self.namespaces.iter().filter(|key| {
            key.exact_native_path() == Some(path)
                || self.original_namespace_geometry(key, policy).as_ref() == Some(&scope)
        });
        let key = keys.next().cloned();
        if keys.next().is_none() {
            OriginalNamespaceKeyPresence::Available(key)
        } else {
            OriginalNamespaceKeyPresence::Unavailable
        }
    }

    /// Apply the reached Ensure operation using a genuine static original
    /// operand. Allocated keys retain the real source site and bounded lifetime;
    /// this conditional source transfer creates no native namespace token.
    // Implementation contract: naming.namespace.original-counted-namespace-allocation
    // docs/design/analysis/name-resolution-proofs/namespace-original-counted-allocation.md
    pub(super) fn ensure_original_namespace_key_at(
        &mut self,
        current: &SourceNamespaceKey,
        input: &SignatureSourceNameInput,
        offset: u32,
    ) -> Option<SourceNamespaceKey> {
        let path = self.original_namespace_path_for_input(current, input)?;
        let policy = input.policy();
        let site = CommandAllocationSite {
            source: Arc::clone(self.current_source_origin.as_ref()?),
            offset,
        };
        let mut result = self.source_root_namespace_key()?;
        let mut parent = ByteNamespacePath::root();
        for component in path.as_segments() {
            parent.push(component.clone());
            match self.original_namespace_key_for_path(&parent, policy) {
                OriginalNamespaceKeyPresence::Available(Some(key)) => {
                    result = key;
                    continue;
                }
                OriginalNamespaceKeyPresence::Available(None) => {}
                OriginalNamespaceKeyPresence::Unavailable => return None,
            }
            let count = Arc::make_mut(&mut self.namespace_allocation_counts)
                .entry((site.clone(), parent.clone()))
                .or_default();
            *count = count.saturating_add(1).min(3);
            result = SourceNamespaceKey::Allocated {
                site: site.clone(),
                incarnation: match *count {
                    1 => AllocationIncarnation::First,
                    2 => AllocationIncarnation::Second,
                    _ => AllocationIncarnation::RepeatedFresh,
                },
                path: parent.clone(),
            };
            Arc::make_mut(&mut self.namespaces).insert(result.clone());
        }
        Some(result)
    }

    pub(super) fn namespace_target_key_at(
        &self,
        target: &NamespaceTransitionTarget,
        current: &(impl NamespaceKeyQuery + ?Sized),
    ) -> Option<SourceNamespaceKey> {
        let current = current.namespace_key();
        let NamespaceTransitionTarget::Named(subject) = target else {
            return Some(current.into_owned());
        };
        let name = subject.literal()?;
        if let SourceNamespaceKey::Authored(current) = current.as_ref() {
            return Some(SourceNamespaceKey::authored(tcl_syntax::naming::qualify(
                current, name,
            )));
        }
        let entry = self.baseline.native_entry.as_ref()?;
        let protocol = entry.command_name_policy()?.recipe();
        let path = current.exact_native_path()?;
        if protocol.is_jim084() {
            let context = current.native_context()?;
            let row = entry.namespace_context(context.token).ok()?;
            let object = row.jim_namespace_object.as_ref()?;
            let selected = protocol
                .namespace_address_input(
                    NativeNameContext::with_jim_namespace(path, object.as_bytes()),
                    name.as_bytes(),
                )
                .ok()?;
            let mut rows = entry.namespaces.iter().filter(|row| {
                row.visible
                    && row
                        .jim_namespace_object
                        .as_ref()
                        .is_some_and(|object| object.as_bytes() == selected.selected())
            });
            let row = rows.next()?;
            if rows.next().is_some() {
                return None;
            }
            return super::runtime_entry::native_namespace_key(entry, row.token);
        }
        let input = protocol
            .namespace_address_input(NativeNameContext::new(path), name.as_bytes())
            .ok()?;
        if input.qualification() != tcl_syntax::naming::NativeNameQualification::Absolute
            && !self.namespaces.contains(current.as_ref())
        {
            return None;
        }
        let selected = protocol
            .namespace_address_path(NativeNameContext::new(path), name.as_bytes())
            .ok()?;
        self.namespace_key_for_path(&selected)
    }

    pub(super) fn ensure_namespace_key_at(
        &mut self,
        target: &NamespaceTransitionTarget,
        current: &SourceNamespaceKey,
        offset: u32,
    ) -> Option<SourceNamespaceKey> {
        if let Some(key) = self.namespace_target_key_at(target, current) {
            return Some(key);
        }
        let NamespaceTransitionTarget::Named(subject) = target else {
            return None;
        };
        let entry = self.baseline.native_entry.as_ref()?;
        let protocol = entry.command_name_policy()?.recipe();
        let input = protocol
            .namespace_address_input(
                NativeNameContext::new(current.exact_native_path()?),
                subject.literal()?.as_bytes(),
            )
            .ok()?;
        if input.qualification() != tcl_syntax::naming::NativeNameQualification::Absolute
            && !self.namespaces.contains(current)
        {
            return None;
        }
        let path = protocol
            .namespace_address_path(
                NativeNameContext::new(current.exact_native_path()?),
                subject.literal()?.as_bytes(),
            )
            .ok()?;
        let source = Arc::clone(self.current_source_origin.as_ref()?);
        let site = CommandAllocationSite { source, offset };
        let mut parent = ByteNamespacePath::root();
        let mut result = self.native_root_namespace_key()?;
        for component in path.as_segments() {
            parent.push(component.clone());
            if let Some(key) = self.namespace_key_for_path(&parent) {
                result = key;
                continue;
            }
            let count = Arc::make_mut(&mut self.namespace_allocation_counts)
                .entry((site.clone(), parent.clone()))
                .or_default();
            *count = count.saturating_add(1).min(3);
            result = SourceNamespaceKey::Allocated {
                site: site.clone(),
                incarnation: match *count {
                    1 => AllocationIncarnation::First,
                    2 => AllocationIncarnation::Second,
                    _ => AllocationIncarnation::RepeatedFresh,
                },
                path: parent.clone(),
            };
            Arc::make_mut(&mut self.namespaces).insert(result.clone());
        }
        Some(result)
    }

    /// Resolve mutable source-time C paths while retaining every reached base's
    /// incarnation. Later path geometry is examined only after an earlier miss.
    pub(super) fn native_source_lookup_paths(
        &self,
        name: &str,
        current: &SourceNamespaceKey,
    ) -> Result<
        Vec<Vec<SourceCommandKey>>,
        tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable,
    > {
        self.native_source_lookup_paths_from_input(name.as_bytes(), current, false)
    }

    pub(super) fn native_source_lookup_paths_bytes(
        &self,
        name: &[u8],
        current: &SourceNamespaceKey,
    ) -> Result<
        Vec<Vec<SourceCommandKey>>,
        tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable,
    > {
        self.native_source_lookup_paths_from_input(name, current, true)
    }

    fn native_source_lookup_paths_from_input(
        &self,
        name: &[u8],
        current: &SourceNamespaceKey,
        retain_missing: bool,
    ) -> Result<
        Vec<Vec<SourceCommandKey>>,
        tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable,
    > {
        use tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable as Unavailable;
        let entry = self
            .baseline
            .native_entry
            .as_ref()
            .ok_or(Unavailable::Namespace)?;
        if !entry.closed {
            return Err(Unavailable::OpenTable);
        }
        let protocol = entry
            .command_name_policy()
            .ok_or(Unavailable::NamePolicy)?
            .recipe();
        if let SourceNamespaceKey::Native(context) = current
            && (context.interpreter != entry.interpreter
                || entry.retained_namespace_context(context.token)? != *context)
        {
            return Err(Unavailable::Namespace);
        }
        if protocol.is_jim084() {
            self.native_jim_source_lookup_paths(entry, name, current, retain_missing)
        } else {
            self.native_c_source_lookup_paths(protocol, name, current, retain_missing)
        }
    }

    fn native_jim_source_lookup_paths(
        &self,
        entry: &tcl_runtime_api::native_compilation::NativeCompilationEntry,
        name: &[u8],
        current: &SourceNamespaceKey,
        retain_missing: bool,
    ) -> Result<
        Vec<Vec<SourceCommandKey>>,
        tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable,
    > {
        use tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable as Unavailable;
        let context = current.native_context().ok_or(Unavailable::Namespace)?;
        let mut cursor = entry.command_lookup_cursor(context.token, name)?;
        let mut result = Vec::new();
        let mut absent = None;
        while let Some(candidate) = cursor.next_candidate()? {
            let namespace =
                super::runtime_entry::native_namespace_key(entry, candidate.namespace_token)
                    .ok_or(Unavailable::Namespace)?;
            let key = SourceCommandKey::slot(namespace, candidate.slot.simple);
            if self.retain_source_lookup_candidate(key, &mut result, &mut absent, retain_missing) {
                return Ok(vec![result]);
            }
        }
        if let Some(key) = absent
            && !result.contains(&key)
        {
            result.push(key);
        }
        Ok(vec![result])
    }

    fn native_c_source_lookup_paths(
        &self,
        protocol: tcl_syntax::naming::NativeNameProtocol,
        name: &[u8],
        current: &SourceNamespaceKey,
        retain_missing: bool,
    ) -> Result<
        Vec<Vec<SourceCommandKey>>,
        tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable,
    > {
        use tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable as Unavailable;
        use tcl_syntax::naming::{NativeNameProtocol, NativeNameQualification};
        let current_path = current.exact_native_path().ok_or(Unavailable::Namespace)?;
        let projection = protocol
            .command_lookup_input(NativeNameContext::new(current_path), name)
            .map_err(|_| Unavailable::NamePolicy)?;
        let absolute = projection.qualification() == NativeNameQualification::Absolute;
        let empty = BTreeSet::from([Vec::new()]);
        let paths = self.namespace_paths.get(current).unwrap_or(&empty);
        let mut results = Vec::new();
        for path in paths {
            let mut result = Vec::new();
            let mut absent = None;
            let mut stopped = false;
            if !absolute {
                if let Some(key) = self.source_candidate_at_bytes(protocol, current, name)? {
                    stopped = self.retain_source_lookup_candidate(
                        key,
                        &mut result,
                        &mut absent,
                        retain_missing,
                    );
                }
                if !stopped
                    && matches!(protocol, NativeNameProtocol::C(version) if version.has_namespace_path())
                    && self.unknown_namespace_paths.contains(current)
                {
                    return Err(Unavailable::Namespace);
                }
                if !stopped
                    && matches!(protocol, NativeNameProtocol::C(version) if version.has_namespace_path())
                {
                    for base in path {
                        // A deleted path entry is ignored by native lookup. Its
                        // replacement at the same display path is a different base.
                        if !self.namespaces.contains(base) {
                            continue;
                        }
                        if let Some(key) = self.source_candidate_at_bytes(protocol, base, name)? {
                            stopped = self.retain_source_lookup_candidate(
                                key,
                                &mut result,
                                &mut absent,
                                retain_missing,
                            );
                            if stopped {
                                break;
                            }
                        }
                    }
                }
            }
            if !stopped {
                let root = self
                    .native_root_namespace_key()
                    .ok_or(Unavailable::Namespace)?;
                if let Some(key) = self.source_candidate_at_bytes(protocol, &root, name)? {
                    stopped = self.retain_source_lookup_candidate(
                        key,
                        &mut result,
                        &mut absent,
                        retain_missing,
                    );
                }
            }
            if !stopped
                && let Some(key) = absent
                && !result.contains(&key)
            {
                result.push(key);
            }
            if !results.contains(&result) {
                results.push(result);
            }
        }
        Ok(results)
    }

    fn retain_source_lookup_candidate(
        &self,
        key: SourceCommandKey,
        result: &mut Vec<SourceCommandKey>,
        absent: &mut Option<SourceCommandKey>,
        retain_missing: bool,
    ) -> bool {
        let stopped = self.retain_lookup_candidate(key.clone(), result, absent);
        if retain_missing && !result.contains(&key) {
            result.push(key);
        }
        stopped
    }

    fn retain_lookup_candidate(
        &self,
        key: SourceCommandKey,
        result: &mut Vec<SourceCommandKey>,
        absent: &mut Option<SourceCommandKey>,
    ) -> bool {
        let bindings = self.bindings.get(&key);
        let missing =
            bindings.is_none_or(|bindings| bindings.contains(&super::MayBinding::Missing));
        if missing {
            *absent = Some(key.clone());
        }
        if (!missing || bindings.is_some_and(|bindings| bindings.len() > 1))
            && !result.contains(&key)
        {
            result.push(key);
        }
        !missing
    }

    fn source_candidate_at_bytes(
        &self,
        protocol: tcl_syntax::naming::NativeNameProtocol,
        base: &SourceNamespaceKey,
        name: &[u8],
    ) -> Result<
        Option<SourceCommandKey>,
        tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable,
    > {
        use tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable as Unavailable;
        let path = base.exact_native_path().ok_or(Unavailable::Namespace)?;
        let slot = protocol
            .command_lookup_slot(NativeNameContext::new(path), name)
            .map_err(|_| Unavailable::NamePolicy)?;
        let namespace = if &slot.namespace == path {
            base.clone()
        } else {
            // The same path spelling cannot revive a retired incarnation's
            // descendants. Absolute input remains rooted independently.
            let absolute = protocol
                .command_lookup_input(NativeNameContext::new(path), name)
                .map_err(|_| Unavailable::NamePolicy)?
                .qualification()
                == tcl_syntax::naming::NativeNameQualification::Absolute;
            if !absolute && !self.namespaces.contains(base) {
                return Err(Unavailable::RetainedDescendant);
            }
            let Some(namespace) = self.namespace_key_for_path(&slot.namespace) else {
                return Ok(None);
            };
            namespace
        };
        Ok(Some(SourceCommandKey::slot(namespace, slot.simple)))
    }

    pub(super) fn publication_key_at(
        &self,
        current: &(impl NamespaceKeyQuery + ?Sized),
        name: &str,
        purpose: PublicationPurpose,
    ) -> Option<SourceCommandKey> {
        let current = current.namespace_key();
        if let SourceNamespaceKey::Authored(current) = current.as_ref() {
            let key = tcl_syntax::naming::qualify(current, name);
            let key = if matches!(purpose, PublicationPurpose::Procedure) {
                tcl_registry::native_procedure::published_procedure_key(key, self.baseline.dialect)?
            } else {
                key
            };
            return Some(SourceCommandKey::authored(key));
        }
        let entry = self.baseline.native_entry.as_ref()?;
        let protocol = entry.command_name_policy()?.recipe();
        let path = current.exact_native_path()?;
        let jim_object = if protocol.is_jim084() {
            Some(
                entry
                    .namespace_context(current.native_context()?.token)
                    .ok()?
                    .jim_namespace_object
                    .as_ref()?
                    .as_bytes(),
            )
        } else {
            None
        };
        let context = NativeNameContext {
            namespace: path,
            jim_namespace_object: jim_object,
        };
        let slot = match purpose {
            PublicationPurpose::Procedure => {
                protocol.command_publication_slot(context, name.as_bytes())
            }
            PublicationPurpose::Rename => {
                protocol.rename_destination_slot(context, name.as_bytes())
            }
            PublicationPurpose::Alias => protocol.alias_publication_slot(context, name.as_bytes()),
            PublicationPurpose::Command => {
                protocol.command_c_api_publication_slot(context, name.as_bytes())
            }
        }
        .ok()?;
        let namespace = self.namespace_key_for_path(&slot.namespace)?;
        Some(SourceCommandKey::slot(namespace, slot.simple))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn original_namespace_operand(
        source: &str,
        version: tcl_dialect::TclVersion,
    ) -> (ModuleCommandBindings, SignatureSourceNameInput) {
        let registry =
            tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let bindings = super::super::SourceCommandBindings::analyse_with_options(
            source,
            tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let point = bindings.invocation_at_source("namespace", 0);
        let input = point
            .original_retained_written_name_input(2)
            .unwrap_or_else(|| panic!("{}", version.dialect_name()));
        let state = point
            .lookup_state
            .as_ref()
            .unwrap_or_else(|| panic!("{}", version.dialect_name()))
            .state
            .clone();
        (state, input)
    }

    fn original_lambda_operand(
        source: &str,
        version: tcl_dialect::TclVersion,
    ) -> (
        ModuleCommandBindings,
        SignatureSourceNameInput,
        tcl_lexer::LexerConfig,
    ) {
        let registry =
            tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
        let dialect = tcl_registry::InvocationDialect::for_version(version);
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        let bindings = super::super::SourceCommandBindings::analyse_with_options(
            source,
            config,
            registry,
            super::super::SourceAnalysisOptions {
                invocation_dialect: Some(dialect),
                native_compilation: tcl_registry::native_compilation::NativeCompilationContext {
                    mode: tcl_registry::native_compilation::NativeCompilationMode::Direct,
                    ..Default::default()
                },
                ..Default::default()
            },
        );
        let offset = u32::try_from(source.rfind("apply ").unwrap()).unwrap();
        let point = bindings.invocation_at_source("apply", offset);
        let input = point.original_retained_written_name_input(1).unwrap();
        let state = point.lookup_state.as_ref().unwrap().state.clone();
        (state, input, config)
    }

    #[test]
    fn original_lambda_namespace_uses_native_list_children_and_proved_omission() {
        // naming.lambda.original-namespace-constructor-and-getter
        // docs/design/analysis/name-resolution-proofs/lambda-original-namespace-constructor-and-getter.md
        // This tests a genuine source-list projection and retained namespace
        // inventory, independently of the SDK's actual Apply worker.
        for version in tcl_dialect::TclVersion::ALL
            .into_iter()
            .filter(|version| *version != tcl_dialect::TclVersion::V8_4)
        {
            let (state, input, config) = original_lambda_operand(
                "namespace eval ns {}; namespace eval :ns {}; apply {{} {} :ns}",
                version,
            );
            let key = state.namespace_for_lambda_operand(&input, config).unwrap();
            assert_eq!(
                state.original_namespace_geometry(&key, input.policy()),
                Some(SignatureNamespaceScope::C(
                    ByteNamespacePath::from_segments([b"ns".as_slice()])
                ))
            );
            let (state, omitted, config) = original_lambda_operand("apply {{} {}}", version);
            assert_eq!(
                state.namespace_for_lambda_operand(&omitted, config),
                state.source_root_namespace_key()
            );
            let (state, opaque, config) = original_lambda_operand(
                r"namespace eval N\uD800 {}; apply {{} {} N\uD800}",
                version,
            );
            let key = state.namespace_for_lambda_operand(&opaque, config).unwrap();
            assert_eq!(
                state.original_namespace_geometry(&key, opaque.policy()),
                Some(SignatureNamespaceScope::C(
                    ByteNamespacePath::from_segments([b"N\xed\xa0\x80".as_slice()])
                ))
            );
        }
    }

    #[test]
    fn original_lambda_namespace_refuses_foreign_source_and_configuration() {
        // naming.lambda.original-namespace-constructor-and-getter
        // docs/design/analysis/name-resolution-proofs/lambda-original-namespace-constructor-and-getter.md
        let version = tcl_dialect::TclVersion::V8_6;
        let source = "namespace eval ns {}; apply {{} {} ns}";
        let (state, input, config) = original_lambda_operand(source, version);
        let (_, foreign, _) = original_lambda_operand(
            "namespace eval ns {}; apply {{} {} ns}; list foreign",
            version,
        );
        assert!(
            state
                .namespace_for_lambda_operand(&foreign, config)
                .is_none()
        );
        let mut changed = config;
        changed.leading_bom = tcl_lexer::LeadingBom::Skip;
        if changed == config {
            changed.leading_bom = tcl_lexer::LeadingBom::Content;
        }
        assert!(
            state
                .namespace_for_lambda_operand(&input, changed)
                .is_none()
        );
        let mut unknown = state;
        let selected = unknown
            .namespace_for_lambda_operand(&input, config)
            .unwrap();
        Arc::make_mut(&mut unknown.unknown_lookup_namespaces).insert(selected);
        assert!(
            unknown
                .namespace_for_lambda_operand(&input, config)
                .is_none()
        );
    }

    #[test]
    fn original_counted_namespace_allocation_retains_components_and_lifetimes() {
        // Implementation contract: naming.namespace.original-counted-namespace-allocation
        // docs/design/analysis/name-resolution-proofs/namespace-original-counted-allocation.md
        for version in tcl_dialect::TclVersion::ALL {
            let (mut state, input) =
                original_namespace_operand(r"namespace eval N\uD800::child {}", version);
            let root = state.source_root_namespace_key().unwrap();
            let namespace = state
                .ensure_original_namespace_key_at(&root, &input, 0)
                .unwrap();
            let SourceNamespaceKey::Allocated {
                site,
                incarnation,
                path,
            } = &namespace
            else {
                panic!("counted source allocation")
            };
            assert_eq!(site.offset, 0);
            assert_eq!(*incarnation, AllocationIncarnation::First);
            assert_eq!(
                path.as_segments()
                    .iter()
                    .map(tcl_core_types::NameBytes::as_bytes)
                    .collect::<Vec<_>>(),
                [b"N\xed\xa0\x80".as_slice(), b"child".as_slice()],
                "{version:?}"
            );
            assert_eq!(
                state.ensure_original_namespace_key_at(&root, &input, 0),
                Some(namespace.clone())
            );
            assert_eq!(
                state.original_namespace_key_for_input(&root, &input),
                Some(namespace.clone())
            );
            Arc::make_mut(&mut state.namespaces).remove(&namespace);
            let successor = state
                .ensure_original_namespace_key_at(&root, &input, 0)
                .unwrap();
            assert!(matches!(
                successor,
                SourceNamespaceKey::Allocated {
                    incarnation: AllocationIncarnation::Second,
                    ..
                }
            ));
            assert_ne!(successor, namespace);
        }
    }

    #[test]
    fn original_counted_namespace_frame_uses_relative_original_children() {
        // Implementation contract: naming.namespace.original-counted-namespace-allocation
        // docs/design/analysis/name-resolution-proofs/namespace-original-counted-allocation.md
        for version in tcl_dialect::TclVersion::ALL {
            let source = r"namespace eval N\uD800 {namespace eval child {proc p {} {}; namespace export p}; set insideCheckpoint READY}; set checkpoint READY";
            let registry =
                tcl_registry::model::ingress::static_context_for(version.dialect_name()).commands();
            let dialect = tcl_registry::InvocationDialect::for_version(version);
            let bindings = super::super::SourceCommandBindings::analyse_with_options(
                source,
                tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar),
                registry,
                super::super::SourceAnalysisOptions {
                    invocation_dialect: Some(dialect),
                    ..Default::default()
                },
            );
            let offset = u32::try_from(source.find("set insideCheckpoint").unwrap()).unwrap();
            let point = bindings.invocation_at_source("set", offset);
            let state = &point
                .lookup_state
                .as_ref()
                .unwrap_or_else(|| panic!("{}", version.dialect_name()))
                .state;
            let namespace = state
                .variable_frame_namespace_key()
                .unwrap_or_else(|| panic!("{}", version.dialect_name()));
            let SourceNamespaceKey::Allocated { path, .. } = &namespace else {
                panic!("original allocated parent frame")
            };
            assert_eq!(
                path.as_segments()
                    .iter()
                    .map(tcl_core_types::NameBytes::as_bytes)
                    .collect::<Vec<_>>(),
                [b"N\xed\xa0\x80".as_slice()],
                "{version:?}"
            );
            assert!(
                state
                    .namespaces
                    .iter()
                    .any(|key| key.exact_native_path().is_some_and(|path| path
                        .as_segments()
                        .iter()
                        .map(tcl_core_types::NameBytes::as_bytes)
                        .collect::<Vec<_>>()
                        == [b"N\xed\xa0\x80".as_slice(), b"child".as_slice()])),
                "{version:?}"
            );
            assert!(
                !state
                    .namespaces
                    .contains(&SourceNamespaceKey::authored("::child"))
            );
        }
    }

    #[test]
    fn original_counted_namespace_allocation_refuses_foreign_unknown_and_duplicate_geometry() {
        // Implementation contract: naming.namespace.original-counted-namespace-allocation
        // docs/design/analysis/name-resolution-proofs/namespace-original-counted-allocation.md
        for version in tcl_dialect::TclVersion::ALL {
            let (mut state, input) =
                original_namespace_operand(r"namespace eval N\uD800 {}", version);
            let root = state.source_root_namespace_key().unwrap();
            let (_, foreign) =
                original_namespace_operand(r"namespace eval N\uD800 {}; set foreign 1", version);
            assert!(
                state
                    .ensure_original_namespace_key_at(&root, &foreign, 0)
                    .is_none()
            );
            let baseline = state.clone();
            Arc::make_mut(&mut state.unknown_lookup_namespaces).insert(root.clone());
            assert!(
                state
                    .ensure_original_namespace_key_at(&root, &input, 0)
                    .is_none()
            );
            state = baseline;
            let first = state
                .ensure_original_namespace_key_at(&root, &input, 0)
                .unwrap();
            let SourceNamespaceKey::Allocated { site, path, .. } = first else {
                unreachable!()
            };
            Arc::make_mut(&mut state.namespaces).insert(SourceNamespaceKey::Allocated {
                site,
                incarnation: AllocationIncarnation::Second,
                path,
            });
            assert!(
                state
                    .ensure_original_namespace_key_at(&root, &input, 0)
                    .is_none()
            );
        }
    }
}
