// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original source lookup applicability for excluded Registry descriptors.

use super::{
    OriginalCatalogueSourceObligation, OriginalCommandLookup, SourceCommandKey,
    SourceInvocationBinding,
};
use crate::ir::CommandTokens;
use tcl_core_types::{ByteCommandSlot, NameBytes};
use tcl_dialect::model::InvocationRealm;
use tcl_registry::model::{CommandSourceUnavailability, ContextRegistry};

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OriginalNativeUnavailableCommand {
    pub(crate) lookup: OriginalCommandLookup,
    pub(crate) descriptor: CommandSourceUnavailability,
    pub(crate) obligations: Vec<OriginalCatalogueSourceObligation>,
}

impl SourceInvocationBinding {
    pub(crate) const fn has_original_lookup_snapshot(&self) -> bool {
        self.lookup_state.is_some()
    }

    pub(crate) fn original_native_unavailable_command(
        &self,
        tokens: &CommandTokens,
        context: &ContextRegistry,
        realm: InvocationRealm,
    ) -> Option<OriginalNativeUnavailableCommand> {
        // naming.diagnostic.original-command-source-unavailability
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
        let lookup = self.original_static_head_lookup_for_tokens(tokens)?;
        let registry = context.commands().snapshot().semantic_key();
        let mut selected = None;
        let mut obligations = Vec::new();
        for row in lookup.source_observations() {
            if row.words.as_ref() != tokens.words()
                || row.config != lookup.name_input().original_word_key()?.lexer_config()
                || row.entry.source().origin.as_ref() != lookup.site().source.as_ref()
                || row.snapshot.state.baseline.registry_snapshot.as_ref() != Some(&registry)
            {
                return None;
            }
            let (descriptor, residual) =
                row.snapshot.state.original_unavailable_command_from_paths(
                    lookup.candidates(),
                    lookup.policy(),
                    context,
                    realm,
                )?;
            if selected
                .as_ref()
                .is_some_and(|previous| previous != &descriptor)
            {
                return None;
            }
            selected = Some(descriptor);
            for obligation in residual {
                if !obligations.contains(&obligation) {
                    obligations.push(obligation);
                }
            }
        }
        Some(OriginalNativeUnavailableCommand {
            lookup,
            descriptor: selected?,
            obligations,
        })
    }

    pub(crate) fn original_authored_unavailable_command(
        &self,
        tokens: &CommandTokens,
        original: &tcl_lexer::NativeWord,
        context: &ContextRegistry,
        realm: InvocationRealm,
        written: &str,
    ) -> Option<(CommandSourceUnavailability, Vec<Vec<NameBytes>>)> {
        // naming.diagnostic.original-command-source-unavailability
        // docs/design/analysis/name-resolution-proofs/diagnostic-original-command-source-unavailability.md
        let snapshot = self.lookup_state.as_ref()?;
        let state = &snapshot.state;
        let site = self.invocation_site()?;
        if tokens.source_binding.as_ref() != Some(self)
            || self.original_lexer_config_for_tokens(tokens)? != original.config()
            || site.source.source_image() != original.image()
            || state.current_source_origin.as_ref() != Some(&site.source)
            || state.baseline.registry_snapshot.as_ref()
                != Some(&context.commands().snapshot().semantic_key())
            || state
                .baseline
                .execution_name_policy
                .and_then(tcl_syntax::naming::ExecutionNamePolicy::native_recipe)
                .is_some()
            || state
                .unknown_namespace_paths
                .contains(&self.lookup_namespace_key)
        {
            return None;
        }
        let paths = state.source_lookup_paths(written, &self.lookup_namespace_key);
        let mut selected = None;
        for path in &paths {
            let mut found = None;
            for key in path {
                let coordinate = key.authored_spelling()?;
                let bindings = state
                    .bindings
                    .get(key)
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>();
                let descriptor = context
                    .commands()
                    .command_names_in_any_dialect()
                    .filter(|name| {
                        SourceCommandKey::authored(tcl_syntax::naming::qualify("::", name)) == *key
                    })
                    .filter_map(|name| context.command_source_unavailability(name, realm))
                    .collect::<Vec<_>>();
                let candidate = match descriptor.as_slice() {
                    [descriptor] => Some(descriptor.command()),
                    [] => None,
                    _ => return None,
                };
                if super::original_command_table::classify_catalogue_source_cell(
                    &bindings, candidate,
                ) == super::original_command_table::CatalogueSourceCell::Barrier
                    || state.baseline.declared_commands.contains_key(coordinate)
                {
                    return None;
                }
                match descriptor.as_slice() {
                    [descriptor] => {
                        found = Some(descriptor.clone());
                        break;
                    }
                    [] => {}
                    _ => return None,
                }
            }
            let found = found?;
            if selected.as_ref().is_some_and(|previous| previous != &found) {
                return None;
            }
            selected = Some(found);
        }
        let paths = paths
            .into_iter()
            .map(|path| {
                path.into_iter()
                    .map(|key| Some(NameBytes::from(key.authored_spelling()?.as_bytes())))
                    .collect::<Option<Vec<_>>>()
            })
            .collect::<Option<Vec<_>>>()?;
        Some((selected?, paths))
    }
}

impl super::ModuleCommandBindings {
    fn original_unavailable_command_from_paths(
        &self,
        paths: &[Vec<ByteCommandSlot>],
        policy: tcl_syntax::naming::NamePolicyProtocol,
        context: &ContextRegistry,
        realm: InvocationRealm,
    ) -> Option<(
        CommandSourceUnavailability,
        Vec<OriginalCatalogueSourceObligation>,
    )> {
        use super::original_command_table::{
            CatalogueSourceCell, classify_catalogue_source_cell, original_registry_metadata_slot,
        };
        use OriginalCatalogueSourceObligation as Obligation;
        let mut obligations = Vec::new();
        if self.has_opaque_domain() || self.baseline.unknown_entry {
            obligations.push(Obligation::UnknownCurrentLookup);
        }
        let mut selected = None;
        for path in paths {
            let mut found = None;
            for slot in path {
                let bindings = self.original_catalogue_bindings_for_slot(slot, policy);
                if self.baseline.declared_commands.keys().any(|name| {
                    original_registry_metadata_slot(policy, name).as_ref() == Some(slot)
                }) {
                    return None;
                }
                let descriptors = context
                    .commands()
                    .command_names_in_any_dialect()
                    .filter(|name| {
                        original_registry_metadata_slot(policy, name).as_ref() == Some(slot)
                    })
                    .filter_map(|name| context.command_source_unavailability(name, realm))
                    .collect::<Vec<_>>();
                let candidate = match descriptors.as_slice() {
                    [descriptor] => Some(descriptor.command()),
                    [] => None,
                    _ => return None,
                };
                match classify_catalogue_source_cell(&bindings, candidate) {
                    CatalogueSourceCell::Barrier => return None,
                    CatalogueSourceCell::Alternative => {
                        obligations.push(Obligation::EarlierBindingAlternative(slot.clone()));
                    }
                    CatalogueSourceCell::Pass => {}
                }
                match descriptors.as_slice() {
                    [descriptor] => {
                        found = Some(descriptor.clone());
                        break;
                    }
                    [] => {}
                    _ => return None,
                }
            }
            let found = found?;
            if selected.as_ref().is_some_and(|previous| previous != &found) {
                return None;
            }
            selected = Some(found);
        }
        Some((selected?, obligations))
    }
}
