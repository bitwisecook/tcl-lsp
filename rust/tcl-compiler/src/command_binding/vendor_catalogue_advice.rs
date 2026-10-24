// SPDX-License-Identifier: AGPL-3.0-or-later
//! Conditional hosted catalogue coordinates use their own source policy.

use super::{SourceCommandKey, SourceInvocationBinding, SourceNamespaceKey};
use crate::ir::CommandTokens;
use crate::signature_scan::vendor_name::VendorSourceNameInput;
use tcl_core_types::NameBytes;
use tcl_lexer::{LexerConfig, NativeWord, SourceImage};
use tcl_registry::{CommandRegistry, RegistrySemanticKey};
use tcl_syntax::naming::VendorSourceNamePurpose;

/// Unclosed applicability of hosted source-only Registry metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VendorCatalogueSourceObligation {
    /// These coordinates are authored source syntax, not native table geometry.
    AuthoredSourceCoordinates,
    /// Stored script syntax supplies no proof that its parent body is entered.
    StoredScriptBodyApplicability,
    /// The current modeled command table has unknown alternatives.
    UnknownCurrentLookup,
    /// The source namespace can have unknown lookup alternatives.
    UnknownSourceNamespace,
    /// An earlier source cell may stop the nominal metadata lookup.
    EarlierBindingAlternative(NameBytes),
    /// The selected source cell may have a non-Registry implementation.
    SelectedBindingAlternative(NameBytes),
    /// Independent original command-slot geometry retains its own residuals.
    OriginalCommandSlots(Vec<super::OriginalCatalogueSourceObligation>),
}

/// Genuine hosted input and conditional source coordinate evidence.
/// Vendor policy stays separate from retained original slot geometry; no
/// current Native holder or executable target is supplied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VendorCatalogueSourceCandidate {
    head: VendorSourceNameInput,
    original: Vec<NativeWord>,
    command: String,
    paths: Vec<Vec<NameBytes>>,
    obligations: Vec<VendorCatalogueSourceObligation>,
    site: super::CommandAllocationSite,
    snapshot: Option<std::sync::Arc<super::SourceLookupSnapshot>>,
    body_origin: Option<std::sync::Arc<crate::registry_invocation::OriginalSourceScriptBodyOrigin>>,
    registry: RegistrySemanticKey,
    original_slots: Option<std::sync::Arc<OriginalVendorSourceSlots>>,
}

/// A same-point source table projection. Vendor coordinates remain separate;
/// original naming geometry supplies no current handler or Native execution.
#[derive(Debug, Clone, PartialEq, Eq)]
struct OriginalVendorSourceSlots {
    snapshot: std::sync::Arc<super::SourceLookupSnapshot>,
    analysis: std::sync::Arc<super::OriginalSourceAnalysisOwner>,
    namespace: SourceNamespaceKey,
    head: crate::signature_scan::original_name::SourceOriginalNameOccurrence,
    candidate: super::DeclaredCommandCandidate,
    paths: Vec<Vec<tcl_core_types::ByteCommandSlot>>,
    obligations: Vec<super::OriginalCatalogueSourceObligation>,
}

impl OriginalVendorSourceSlots {
    fn matches_invocation(&self, binding: &SourceInvocationBinding) -> bool {
        binding
            .lookup_state
            .as_ref()
            .is_some_and(|snapshot| std::sync::Arc::ptr_eq(snapshot, &self.snapshot))
            && binding
                .original_analysis_attachment()
                .is_some_and(|owner| std::sync::Arc::ptr_eq(owner, &self.analysis))
            && binding.lookup_namespace_key == self.namespace
            && binding.invocation_site() == Some(self.head.site())
    }

    fn matches_state(
        &self,
        state: &super::ModuleCommandBindings,
        current: &SourceNamespaceKey,
    ) -> bool {
        std::ptr::eq(state, &self.snapshot.state)
            && current == &self.namespace
            && !state.has_opaque_domain()
            && !state.baseline.unknown_entry
            && !self.paths.is_empty()
    }
}

impl VendorCatalogueSourceCandidate {
    /// Complete original hosted command head and its separate policy.
    #[must_use]
    pub const fn original_head(&self) -> &VendorSourceNameInput {
        &self.head
    }
    /// Genuine complete original source vector.
    #[must_use]
    pub fn original_words(&self) -> &[NativeWord] {
        &self.original
    }
    /// Nominal Registry descriptor, independently of executable availability.
    #[must_use]
    pub fn command(&self) -> &str {
        &self.command
    }
    /// Authored source coordinates, never Native `ByteCommandSlot` values.
    #[must_use]
    pub fn paths(&self) -> &[Vec<NameBytes>] {
        &self.paths
    }
    /// Authored source lookup prefixes through the actual nominal descriptor.
    /// This projection supplies no native holder or runtime target.
    pub(crate) fn selected_source_prefixes(&self) -> Option<Vec<Vec<NameBytes>>> {
        self.paths
            .iter()
            .map(|path| {
                let mut prefix = Vec::new();
                for coordinate in path {
                    prefix.push(coordinate.clone());
                    let key = std::str::from_utf8(coordinate.as_bytes()).ok()?;
                    let selected = self.snapshot.as_ref().map_or_else(
                        || {
                            tcl_syntax::naming::qualify_bytes(b"::", self.command.as_bytes())
                                .as_slice()
                                == coordinate.as_bytes()
                        },
                        |snapshot| {
                            snapshot.state.baseline.catalogue_commands.get(key)
                                == Some(&self.command)
                        },
                    );
                    if selected {
                        return Some(prefix);
                    }
                }
                None
            })
            .collect()
    }

    pub(crate) fn retain_script_body_origin(
        &mut self,
        body: &crate::registry_invocation::OriginalSourceScriptBodyOrigin,
    ) {
        self.body_origin = Some(std::sync::Arc::new(body.clone()));
        for obligation in body.obligations().iter().cloned().chain(std::iter::once(
            VendorCatalogueSourceObligation::StoredScriptBodyApplicability,
        )) {
            if !self.obligations.contains(&obligation) {
                self.obligations.push(obligation);
            }
        }
    }

    pub(crate) fn script_body_origin(
        &self,
    ) -> Option<&crate::registry_invocation::OriginalSourceScriptBodyOrigin> {
        self.body_origin.as_deref()
    }

    /// Every retained applicability limitation.
    #[must_use]
    pub fn obligations(&self) -> &[VendorCatalogueSourceObligation] {
        &self.obligations
    }
    /// Exact whole source, input channel and every grammar axis.
    #[must_use]
    pub fn matches_source(&self, image: &SourceImage, config: LexerConfig) -> bool {
        self.head.matches_source(image, config)
            && self
                .original
                .iter()
                .all(|word| word.image() == image && word.config() == config)
    }
    /// Same authentic child source carrier. Missing child lookup is retained
    /// only with a separately issued script-body origin, never a parent table.
    pub(crate) fn matches_invocation(&self, tokens: &CommandTokens) -> bool {
        if tokens.synthetic.is_some() {
            return false;
        }
        let Some(binding) = tokens.source_binding.as_ref() else {
            return false;
        };
        let same_lookup = match (&self.snapshot, &binding.lookup_state) {
            (Some(expected), Some(actual)) => std::sync::Arc::ptr_eq(actual, expected),
            (None, None) => self.body_origin.is_some(),
            _ => false,
        };
        if !same_lookup {
            return false;
        }
        if self.original_slots.as_ref().is_some_and(|slots| {
            !slots.matches_invocation(binding)
                || crate::registry_invocation::original_native_compiler_words(
                    self.head.source_image(),
                    tokens.words(),
                    self.site.offset,
                    self.head.lexer_config(),
                )
                .as_deref()
                    != Some(self.original.as_slice())
        }) {
            return false;
        }
        if self.snapshot.is_some() {
            return binding.original_lexer_config_for_tokens(tokens)
                == Some(self.head.lexer_config())
                && binding.invocation_site() == Some(&self.site);
        }
        // Missing runtime/compiler attachments stay missing. The sealed body
        // occurrence owns source geometry independently of an entered point.
        binding
            .invocation_site()
            .is_none_or(|site| site == &self.site)
            && binding
                .original_lexer_config_for_tokens(tokens)
                .is_none_or(|config| config == self.head.lexer_config())
            && crate::registry_invocation::original_native_compiler_words(
                self.head.source_image(),
                tokens.words(),
                self.site.offset,
                self.head.lexer_config(),
            )
            .is_some_and(|original| original == self.original)
    }

    /// Same command-store semantic generation.
    #[must_use]
    pub fn matches_registry(&self, registry: &CommandRegistry) -> bool {
        self.registry == registry.snapshot().semantic_key()
    }
}
#[cfg(test)]
fn vendor_candidate_option<T>(stage: &str, value: Option<T>) -> Option<T> {
    if value.is_none() && std::env::var_os("TCL_LSP_TRACE_REMOTE_FIRST_OPERAND").is_some() {
        eprintln!("VENDOR_SOURCE_CANDIDATE_REFUSAL stage={stage}");
    }
    value
}

#[cfg(not(test))]
fn vendor_candidate_option<T>(_: &str, value: Option<T>) -> Option<T> {
    value
}

#[cfg(test)]
fn vendor_candidate_rejection(stage: &str, value: bool) -> bool {
    if value && std::env::var_os("TCL_LSP_TRACE_REMOTE_FIRST_OPERAND").is_some() {
        eprintln!("VENDOR_SOURCE_CANDIDATE_REFUSAL stage={stage}");
    }
    value
}

#[cfg(not(test))]
fn vendor_candidate_rejection(_: &str, value: bool) -> bool {
    value
}

impl SourceInvocationBinding {
    pub(crate) fn original_vendor_catalogue_source_candidate(
        &self,
        tokens: &CommandTokens,
        head: &VendorSourceNameInput,
        original: &[NativeWord],
        registry: &CommandRegistry,
    ) -> Option<VendorCatalogueSourceCandidate> {
        let site = vendor_candidate_option("invocation_site", self.invocation_site())?;
        if vendor_candidate_rejection(
            "foreign_token_binding",
            tokens.source_binding.as_ref() != Some(self),
        ) || vendor_candidate_rejection(
            "different_lexer_config",
            vendor_candidate_option(
                "lexer_config",
                self.original_lexer_config_for_tokens(tokens),
            )? != head.lexer_config(),
        ) || vendor_candidate_rejection(
            "different_source_image",
            site.source.source_image() != head.source_image(),
        ) || vendor_candidate_rejection(
            "different_original_head",
            original.first() != Some(head.original_word()),
        ) || vendor_candidate_rejection(
            "expanded_original_word",
            original.iter().any(|word| word.group().expand),
        ) || vendor_candidate_rejection(
            "different_original_vector",
            vendor_candidate_option(
                "original_word_vector",
                crate::registry_invocation::original_native_compiler_words(
                    head.source_image(),
                    tokens.words(),
                    site.offset,
                    head.lexer_config(),
                ),
            )?
            .as_slice()
                != original,
        ) {
            return None;
        }
        self.vendor_catalogue_at_original(head, original, registry, site)
    }

    fn vendor_catalogue_at_original(
        &self,
        head: &VendorSourceNameInput,
        original: &[NativeWord],
        registry: &CommandRegistry,
        site: &super::CommandAllocationSite,
    ) -> Option<VendorCatalogueSourceCandidate> {
        let command = std::str::from_utf8(vendor_candidate_option(
            "literal_command_head",
            head.literal_units(VendorSourceNamePurpose::CommandHead),
        )?)
        .ok()?;
        let snapshot = vendor_candidate_option("lookup_state", self.lookup_state.as_ref())?;
        let state = &snapshot.state;
        let registry_key = registry.snapshot().semantic_key();
        if vendor_candidate_rejection(
            "different_source_origin",
            state.current_source_origin.as_ref() != Some(&site.source),
        ) || vendor_candidate_rejection(
            "different_registry_generation",
            state.baseline.registry_snapshot.as_ref() != Some(&registry_key),
        ) || vendor_candidate_rejection(
            "native_world_without_hosted_owner",
            state
                .baseline
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                .is_some()
                && !state.hosted_rule_loader_world(),
        ) {
            return None;
        }
        let original_slots = if state.has_vendor_original_slots() {
            Some(std::sync::Arc::new(
                self.vendor_original_slots(original, snapshot)?,
            ))
        } else {
            None
        };
        let (command, paths, mut obligations) = vendor_candidate_option(
            "table_classifier",
            original_slots.as_deref().map_or_else(
                || state.vendor_catalogue_source_candidate(&self.lookup_namespace_key, command),
                |slots| {
                    state.vendor_catalogue_source_candidate_with_slots(
                        &self.lookup_namespace_key,
                        command,
                        Some(slots),
                    )
                },
            ),
        )?;
        if let Some(slots) = &original_slots {
            if slots.candidate.name != command {
                return None;
            }
            obligations.push(VendorCatalogueSourceObligation::OriginalCommandSlots(
                slots.obligations.clone(),
            ));
        }
        Some(VendorCatalogueSourceCandidate {
            head: head.clone(),
            original: original.to_vec(),
            command,
            paths,
            obligations,
            site: site.clone(),
            snapshot: Some(std::sync::Arc::clone(snapshot)),
            body_origin: None,
            registry: registry_key,
            original_slots,
        })
    }
}
impl SourceInvocationBinding {
    fn vendor_original_slots(
        &self,
        original: &[NativeWord],
        snapshot: &std::sync::Arc<super::SourceLookupSnapshot>,
    ) -> Option<OriginalVendorSourceSlots> {
        use crate::signature_scan::scope::{SignatureSourceNameInput, SignatureSourceNameKey};
        let state = &snapshot.state;
        if state.has_opaque_domain() || state.baseline.unknown_entry {
            return None;
        }
        let analysis = self.original_analysis_attachment()?;
        let policy = state.baseline.execution_name_policy?.native_recipe()?;
        let word = original.first()?;
        let input = SignatureSourceNameKey::from_original_native_word(
            word,
            tcl_syntax::word_rules::WordValueRules::from_config(&word.config()),
            policy,
        )?;
        let head = crate::signature_scan::original_name::SourceOriginalNameOccurrence::new(
            self.invocation_site()?,
            input.clone(),
        )?;
        let namespace = state.original_namespace_geometry(&self.lookup_namespace_key, policy)?;
        let (candidate, paths, obligations) = state.original_catalogue_source_candidate(
            &self.lookup_namespace_key,
            &namespace,
            &SignatureSourceNameInput::OriginalWord(input),
        )?;
        Some(OriginalVendorSourceSlots {
            snapshot: std::sync::Arc::clone(snapshot),
            analysis: std::sync::Arc::clone(analysis),
            namespace: self.lookup_namespace_key.clone(),
            head,
            candidate,
            paths,
            obligations,
        })
    }

    pub(crate) fn original_vendor_script_body_source_candidate(
        &self,
        context: &tcl_registry::model::ContextRegistry,
        tokens: &CommandTokens,
        shape: &crate::registry_invocation::VendorRegistryInvocationShape,
        site: &super::CommandAllocationSite,
        body: &crate::registry_invocation::OriginalSourceScriptBodyOrigin,
    ) -> Option<VendorCatalogueSourceCandidate> {
        // A present source lookup must use its own classifier. Neither a
        // blocked table nor a stale carrier can fall through to body syntax.
        if tokens.synthetic.is_some()
            || self.lookup_state.is_some()
            || shape
                .original_words()
                .iter()
                .any(|word| word.group().expand)
            || !body.matches_child(context, shape.original_words())
        {
            return None;
        }
        let head = shape.original_head();
        if tokens.source_binding.as_ref() != Some(self)
            || self.invocation_site().is_some_and(|actual| actual != site)
            || self
                .original_lexer_config_for_tokens(tokens)
                .is_some_and(|config| config != head.lexer_config())
            || site.source.source_image() != head.source_image()
            || site.offset != head.original_word().span().start()
            || crate::registry_invocation::original_native_compiler_words(
                head.source_image(),
                tokens.words(),
                site.offset,
                head.lexer_config(),
            )?
            .as_slice()
                != shape.original_words()
        {
            return None;
        }
        let written =
            std::str::from_utf8(head.literal_units(VendorSourceNamePurpose::CommandHead)?).ok()?;
        let paths = tcl_syntax::naming::command_resolution_candidates_from_namespace_keys(
            "::",
            &[] as &[&str],
            written,
        )
        .into_iter()
        .map(|name| NameBytes::from(name.as_bytes()))
        .collect::<Vec<_>>();
        let mut obligations = body.obligations().to_vec();
        for obligation in [
            VendorCatalogueSourceObligation::AuthoredSourceCoordinates,
            VendorCatalogueSourceObligation::UnknownCurrentLookup,
            VendorCatalogueSourceObligation::UnknownSourceNamespace,
        ] {
            if !obligations.contains(&obligation) {
                obligations.push(obligation);
            }
        }
        Some(VendorCatalogueSourceCandidate {
            head: head.clone(),
            original: shape.original_words().to_vec(),
            command: shape.command().to_owned(),
            paths: vec![paths],
            obligations,
            site: site.clone(),
            snapshot: None,
            body_origin: Some(std::sync::Arc::new(body.clone())),
            registry: context.commands().snapshot().semantic_key(),
            original_slots: None,
        })
    }
}

impl super::ModuleCommandBindings {
    fn vendor_catalogue_source_candidate(
        &self,
        current: &SourceNamespaceKey,
        command: &str,
    ) -> Option<(
        String,
        Vec<Vec<NameBytes>>,
        Vec<VendorCatalogueSourceObligation>,
    )> {
        self.vendor_catalogue_source_candidate_with_slots(current, command, None)
    }

    fn has_vendor_original_slots(&self) -> bool {
        self.hosted_rule_loader_world()
            && self
                .bindings
                .keys()
                .any(|key| matches!(key, SourceCommandKey::Slot { .. }))
    }

    fn vendor_catalogue_source_candidate_with_slots(
        &self,
        current: &SourceNamespaceKey,
        command: &str,
        slots: Option<&OriginalVendorSourceSlots>,
    ) -> Option<(
        String,
        Vec<Vec<NameBytes>>,
        Vec<VendorCatalogueSourceObligation>,
    )> {
        use super::original_command_table::{CatalogueSourceCell, classify_catalogue_source_cell};
        use VendorCatalogueSourceObligation as Obligation;
        let SourceNamespaceKey::Authored(namespace) = current else {
            return vendor_candidate_option("namespace_not_authored", None);
        };
        if !command.is_ascii()
            || command.as_bytes().contains(&0)
            || self.unknown_namespace_paths.contains(current)
        {
            return None;
        }
        // A document publication must be read through its original typed
        // table at this same point; authored coordinates cannot bypass it.
        if self.has_vendor_original_slots()
            && slots.is_none_or(|slots| !slots.matches_state(self, current))
        {
            return vendor_candidate_option("table_has_slot", None);
        }
        let mut obligations = vec![Obligation::AuthoredSourceCoordinates];
        if self.has_opaque_domain() || self.baseline.unknown_entry {
            obligations.push(Obligation::UnknownCurrentLookup);
        }
        if self.unknown_lookup_namespaces.contains(current) {
            obligations.push(Obligation::UnknownSourceNamespace);
        }
        let default = std::collections::BTreeSet::from([Vec::new()]);
        let paths = self
            .namespace_paths
            .get(current)
            .unwrap_or(&default)
            .iter()
            .map(|path| {
                let contexts = path
                    .iter()
                    .map(|key| match key {
                        SourceNamespaceKey::Authored(text) => Some(text.as_str()),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()?;
                Some(
                    tcl_syntax::naming::command_resolution_candidates_from_namespace_keys(
                        namespace, &contexts, command,
                    ),
                )
            })
            .collect::<Option<Vec<_>>>()?;
        let mut unanimous = None;
        for path in &paths {
            let mut selected = None;
            for coordinate in path {
                let candidate = self.baseline.catalogue_commands.get(coordinate);
                let key = SourceCommandKey::authored(coordinate);
                let bindings = self
                    .bindings
                    .get(&key)
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();
                match classify_catalogue_source_cell(&bindings, candidate.map(String::as_str)) {
                    CatalogueSourceCell::Barrier => return None,
                    CatalogueSourceCell::Alternative if candidate.is_some() => {
                        obligations.push(Obligation::SelectedBindingAlternative(NameBytes::from(
                            coordinate.as_bytes(),
                        )));
                    }
                    CatalogueSourceCell::Alternative => {
                        obligations.push(Obligation::EarlierBindingAlternative(NameBytes::from(
                            coordinate.as_bytes(),
                        )));
                    }
                    CatalogueSourceCell::Pass => {}
                }
                if self.baseline.declared_commands.contains_key(coordinate) {
                    return None;
                }
                if let Some(candidate) = candidate {
                    selected = Some(candidate.clone());
                    break;
                }
            }
            let selected = selected?;
            if unanimous.as_ref().is_some_and(|prior| prior != &selected) {
                return None;
            }
            unanimous = Some(selected);
        }
        Some((
            unanimous?,
            paths
                .into_iter()
                .map(|path| {
                    path.into_iter()
                        .map(|key| NameBytes::from(key.as_bytes()))
                        .collect()
                })
                .collect(),
            obligations,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_vendor_slot_projection_retains_before_opaque_remote_source() {
        let source = "proc invoke {} {set h [ILX::init p e]; ILX::call $h m}";
        let analysis = crate::analyser::Analyser::new().analyse(source, "f5-irules");
        let calls =
            crate::registry_invocation::source_remote_first_operand_associations(source, &analysis)
                .unwrap();
        assert_eq!(calls.len(), 1);
        assert!(calls[0].matches_source_analysis(source, &analysis));
        let blocked = "proc invoke {} {set h [ILX::init p e]; unknown_command; ILX::call $h m}";
        let analysis = crate::analyser::Analyser::new().analyse(blocked, "f5-irules");
        assert!(
            crate::registry_invocation::source_remote_first_operand_associations(
                blocked, &analysis
            )
            .unwrap()
            .is_empty()
        );
    }

    #[test]
    fn original_conditional_vendor_catalogue_keeps_policy_and_cell_barriers_separate() {
        // Implementation contract dependency: naming.consumer.hosted-source-taint-descriptor
        // docs/design/analysis/name-resolution-proofs/hosted-source-taint-descriptor.md
        // Implementation contract: naming.compiler.conditional-registry-source-metadata
        // docs/design/analysis/name-resolution-proofs/conditional-registry-source-metadata.md
        let generation = tcl_registry::model::ingress::static_context_for("f5-irules");
        let registry = generation.commands();
        let config = tcl_lexer::LexerConfig::for_profile(registry.profile());
        let state = super::super::ModuleCommandBindings::initial_with_options(
            registry,
            super::super::SourceAnalysisOptions::default(),
            Some(config),
        );
        assert!(
            state
                .baseline
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                .is_none()
        );
        let root = state.source_root_namespace_key().unwrap();
        let (candidate, _, obligations) = state
            .vendor_catalogue_source_candidate(&root, "HTTP::respond")
            .unwrap();
        assert_eq!(candidate, "HTTP::respond");
        assert!(obligations.contains(&VendorCatalogueSourceObligation::AuthoredSourceCoordinates));
        let key = SourceCommandKey::authored("::HTTP::respond");
        let original = state
            .targets("HTTP::respond", &root)
            .into_iter()
            .next()
            .unwrap();
        let mut custom = original.clone();
        custom.registry_backed = false;
        custom.kind = super::super::BindingKind::Proc;
        for replacement in [
            super::super::MayBinding::Missing,
            super::super::MayBinding::Target(custom),
        ] {
            let mut blocked = state.clone();
            blocked.replace(key.clone(), std::collections::BTreeSet::from([replacement]));
            assert!(
                blocked
                    .vendor_catalogue_source_candidate(&root, "HTTP::respond")
                    .is_none()
            );
        }
        // Moving preserves the Registry implementation at a different source
        // coordinate, but cannot revive the old nominal catalogue cell or
        // donate its descriptor merely from the moved command's label.
        let mut moved = state.clone();
        moved.replace(
            key.clone(),
            std::collections::BTreeSet::from([super::super::MayBinding::Missing]),
        );
        moved.replace(
            SourceCommandKey::authored("::renamed_response"),
            std::collections::BTreeSet::from([super::super::MayBinding::Target(original)]),
        );
        assert!(
            moved
                .vendor_catalogue_source_candidate(&root, "HTTP::respond")
                .is_none()
        );
        assert!(
            moved
                .vendor_catalogue_source_candidate(&root, "renamed_response")
                .is_none()
        );
        let mut unknown = state.clone();
        unknown.opaque_domain = true;
        unknown.replace(
            key,
            std::collections::BTreeSet::from([super::super::MayBinding::Unknown]),
        );
        let (_, _, obligations) = unknown
            .vendor_catalogue_source_candidate(&root, "HTTP::respond")
            .unwrap();
        assert!(obligations.contains(&VendorCatalogueSourceObligation::UnknownCurrentLookup));
        assert!(obligations.iter().any(|item| matches!(
            item,
            VendorCatalogueSourceObligation::SelectedBindingAlternative(_)
        )));
        assert!(
            state
                .vendor_catalogue_source_candidate(&root, "HTTP::respond\\uD800")
                .is_none()
        );
    }
}
