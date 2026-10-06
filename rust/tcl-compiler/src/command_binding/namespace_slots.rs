// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source table operations over retained namespace geometry and incarnations.

use super::{
    AllocationIncarnation, Arc, BTreeSet, CommandAllocationSite, ModuleCommandBindings,
    NamespaceKeyQuery, SourceCommandKey, SourceNamespaceKey,
};
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
        use tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable as Unavailable;
        use tcl_syntax::naming::{NativeNameProtocol, NativeNameQualification};
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
            let context = current.native_context().ok_or(Unavailable::Namespace)?;
            let mut cursor = entry.command_lookup_cursor(context.token, name.as_bytes())?;
            let mut result = Vec::new();
            let mut absent = None;
            while let Some(candidate) = cursor.next_candidate()? {
                let namespace =
                    super::runtime_entry::native_namespace_key(entry, candidate.namespace_token)
                        .ok_or(Unavailable::Namespace)?;
                let key = SourceCommandKey::slot(namespace, candidate.slot.simple);
                if self.retain_lookup_candidate(key, &mut result, &mut absent) {
                    return Ok(vec![result]);
                }
            }
            if let Some(key) = absent
                && !result.contains(&key)
            {
                result.push(key);
            }
            return Ok(vec![result]);
        }
        let current_path = current.exact_native_path().ok_or(Unavailable::Namespace)?;
        let projection = protocol
            .command_lookup_input(NativeNameContext::new(current_path), name.as_bytes())
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
                if let Some(key) = self.source_candidate_at(protocol, current, name)? {
                    stopped = self.retain_lookup_candidate(key, &mut result, &mut absent);
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
                        if let Some(key) = self.source_candidate_at(protocol, base, name)? {
                            stopped = self.retain_lookup_candidate(key, &mut result, &mut absent);
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
                if let Some(key) = self.source_candidate_at(protocol, &root, name)? {
                    stopped = self.retain_lookup_candidate(key, &mut result, &mut absent);
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

    fn source_candidate_at(
        &self,
        protocol: tcl_syntax::naming::NativeNameProtocol,
        base: &SourceNamespaceKey,
        name: &str,
    ) -> Result<
        Option<SourceCommandKey>,
        tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable,
    > {
        use tcl_runtime_api::native_compilation::NativeCommandLookupUnavailable as Unavailable;
        let path = base.exact_native_path().ok_or(Unavailable::Namespace)?;
        let slot = protocol
            .command_lookup_slot(NativeNameContext::new(path), name.as_bytes())
            .map_err(|_| Unavailable::NamePolicy)?;
        let namespace = if &slot.namespace == path {
            base.clone()
        } else {
            // The same path spelling cannot revive a retired incarnation's
            // descendants. Absolute input remains rooted independently.
            let absolute = protocol
                .command_lookup_input(NativeNameContext::new(path), name.as_bytes())
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
