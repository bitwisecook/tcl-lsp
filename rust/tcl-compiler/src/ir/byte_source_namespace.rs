// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Explicit byte compilation target geometry, independently of native admission.

use super::{Module, TopLevelKind};
use crate::analyser::ResolvedAnalysisInput;
use crate::command_binding::{SourceAnalysisEntry, SourceNamespaceKey};
use tcl_core_types::{ByteNamespacePath, ProcedureModuleId};
use tcl_lexer::{LexerConfig, SourceImage};
use tcl_registry::{CommandRegistry, RegistrySemanticKey};

/// Original opaque-byte target namespace. Its private constructor belongs to
/// the actual byte-source lowering driver, without a rendered namespace key.
#[derive(Debug, Clone, PartialEq)]
pub struct OriginalByteSourceNamespace {
    module: ProcedureModuleId,
    image: SourceImage,
    namespace: ByteNamespacePath,
    context: Option<SourceNamespaceKey>,
    entry: SourceAnalysisEntry,
    input: Option<ResolvedAnalysisInput>,
    config: LexerConfig,
    registry: RegistrySemanticKey,
    profile: Option<tcl_dialect::DialectProfileKey>,
    kind: TopLevelKind,
}

impl Module {
    pub(crate) fn retain_original_byte_source_namespace(
        &mut self,
        namespace: &ByteNamespacePath,
        registry: &CommandRegistry,
    ) {
        self.original_byte_source_namespace = None;
        if self.native_namespace.as_ref() != Some(namespace)
            || self
                .registry_snapshot
                .as_ref()
                .is_none_or(|held| held.semantic_key() != registry.snapshot().semantic_key())
            || self
                .source_entry
                .native_entry
                .as_deref()
                .is_some_and(|entry| {
                    SourceNamespaceKey::from_native_entry(entry)
                        .ok()
                        .and_then(|key| key.source_artifact_path())
                        .as_ref()
                        != Some(namespace)
                })
        {
            return;
        }
        self.original_byte_source_namespace =
            Some(std::sync::Arc::new(OriginalByteSourceNamespace {
                module: self.procedure_ids.module_id(),
                image: self.source.clone(),
                namespace: namespace.clone(),
                context: self.top_level_namespace_context.clone(),
                entry: self.source_entry.clone(),
                input: self.source_metadata_input.clone(),
                config: self.native_lexer_config(),
                registry: registry.snapshot().semantic_key(),
                profile: self
                    .dialect_profile
                    .map(tcl_dialect::DialectProfile::cache_key),
                kind: self.top_level_kind,
            }));
    }

    pub(crate) fn original_byte_source_namespace_for_codegen(
        &self,
        registry: &CommandRegistry,
    ) -> Option<&ByteNamespacePath> {
        let owner = self.original_byte_source_namespace.as_ref()?;
        (owner.module == self.procedure_ids.module_id()
            && owner.image == self.own_source()
            && self.native_namespace.as_ref() == Some(&owner.namespace)
            && owner.context == self.top_level_namespace_context
            && owner.entry == self.source_entry
            && owner.input == self.source_metadata_input
            && owner.config == self.native_lexer_config()
            && owner.registry == registry.snapshot().semantic_key()
            && self.registry_snapshot.as_ref()?.semantic_key() == owner.registry
            && owner.profile
                == self
                    .dialect_profile
                    .map(tcl_dialect::DialectProfile::cache_key)
            && owner.kind == self.top_level_kind)
            .then_some(&owner.namespace)
    }
}
