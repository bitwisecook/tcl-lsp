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

//! The command realm a document is analysed in: its build inputs, the prebuilt realm and compilation unit a caller may supply, and the read-only realms.

use super::Analyser;
use std::sync::Arc;

/// Everything [`Analyser::document_command_realm`] reads besides the source.
/// Two analysers with equal inputs build equal realms for equal source.
#[derive(Debug, PartialEq)]
pub(in super::super) struct RealmBuildInputs {
    input: crate::analyser::ResolvedAnalysisInput,
    entry: Option<Arc<crate::command_binding::SourceAnalysisEntry>>,
    pub(super) file_path: Option<String>,
    settings: super::super::source_settings::OriginalAnalyserSourceSettings,
}

/// A realm already built for `source` under `inputs`, offered to the next walk.
#[derive(Debug)]
pub(in super::super) struct PrebuiltRealm {
    pub(super) source: String,
    pub(super) inputs: RealmBuildInputs,
    pub(super) realm: crate::realm::CommandBindingRealm,
    /// The whole-file command-trust scan, when the shell already lowered the
    /// document for it; it reads the same source, store, grammar and entry.
    pub(super) command_trust: Option<Arc<crate::command_binding::ModuleCommandMutations>>,
}

/// Ownership of the pre-built unit consumed by one diagnostic tail.
#[derive(Debug)]
pub(in super::super) enum CompilationUnitOverride {
    Shared(std::sync::Arc<crate::compilation_unit::CompilationUnit>),
    Owned(Box<crate::compilation_unit::CompilationUnit>),
}

impl CompilationUnitOverride {
    pub(in super::super) fn as_unit(&self) -> &crate::compilation_unit::CompilationUnit {
        match self {
            Self::Shared(unit) => unit,
            Self::Owned(unit) => unit,
        }
    }
}

impl Analyser {
    /// Resolve the driver's complete editing input without walking source.
    /// An unavailable overlay is returned unchanged, without a donor generation.
    ///
    /// # Errors
    /// Returns the selected overlay miss when its command store is unavailable.
    pub fn prepare_analysis_input(
        &mut self,
        dialect: &str,
    ) -> Result<super::super::input::ResolvedAnalysisInput, tcl_registry::model::OverlayMiss> {
        self.resolve_walk_environment(dialect);
        self.try_analysis_context()?;
        Ok(self.resolved_analysis_input())
    }

    /// Retain the current whole-source editing generation and configuration.
    /// Supplied inputs stay exact; default ingress records its file boundary.
    /// Extracted bodies explicitly use the retained input's nested projection.
    /// This does not retain the document's temporal execution world.
    #[must_use]
    pub fn resolved_analysis_input(&self) -> super::super::input::ResolvedAnalysisInput {
        let input = super::super::input::ResolvedAnalysisInput::new(
            self.profile,
            self.unit_profile.unwrap_or(self.profile),
            self.analysis_context(),
            self.file_lexer_config(),
        )
        .with_workspace_source_modules(self.workspace_source_modules.clone())
        .with_script_encoding(self.script_encoding);
        // A supplied input keeps the policy its supplier gave it, including
        // none; only the default ingress derives one from the environment.
        let policy = match &self.resolved_input {
            Some(supplied) => supplied.vendor_source_policy(),
            None => self.vendor_source_name_policy(),
        };
        match policy {
            Some(policy) => input.with_vendor_source_policy(policy),
            None => input,
        }
    }

    /// Select the same source interpretation without walking diagnostics or
    /// constructing an analysis projection. Its editing input is sealed once.
    pub(crate) fn readonly_source_registry_realm(
        source: &str,
        input: super::super::input::ResolvedAnalysisInput,
    ) -> (
        crate::realm::CommandBindingRealm,
        super::super::input::ResolvedAnalysisInput,
    ) {
        Self::readonly_realm(input, false, |analyser| {
            analyser.document_command_realm(source)
        })
    }

    /// The fixed package-index survey's readonly realm, with its scoped
    /// conditional source control. Any input the survey realm declines keeps
    /// the ordinary readonly source realm.
    pub(crate) fn readonly_package_index_survey_realm(
        source: &str,
        input: super::super::input::ResolvedAnalysisInput,
    ) -> (
        crate::realm::CommandBindingRealm,
        super::super::input::ResolvedAnalysisInput,
    ) {
        Self::readonly_realm(input, false, |analyser| {
            analyser
                .package_index_survey_realm(source)
                .unwrap_or_else(|| analyser.document_command_realm(source))
        })
    }

    /// Select exactly the standalone structural consumer's pre-walk realm and
    /// settings. No source walk, diagnostics or analysis projection is made.
    pub(crate) fn readonly_standalone_structure_realm(
        source: &str,
        input: super::super::input::ResolvedAnalysisInput,
    ) -> (
        crate::realm::CommandBindingRealm,
        super::super::input::ResolvedAnalysisInput,
    ) {
        Self::readonly_realm(input, true, |analyser| {
            analyser.document_command_realm(source)
        })
    }

    fn readonly_realm(
        input: super::super::input::ResolvedAnalysisInput,
        structure_only: bool,
        realm: impl FnOnce(&Self) -> crate::realm::CommandBindingRealm,
    ) -> (
        crate::realm::CommandBindingRealm,
        super::super::input::ResolvedAnalysisInput,
    ) {
        let profile = input.analyser_profile();
        let mut analyser = Self::new().with_resolved_input(input);
        analyser.structure_only = structure_only;
        analyser.resolve_walk_environment(profile.name);
        analyser.registry = Some(analyser.profile_registry());
        let realm = realm(&analyser);
        (realm, analyser.resolved_analysis_input())
    }

    fn package_index_survey_realm(
        &self,
        source: &str,
    ) -> Option<crate::realm::CommandBindingRealm> {
        if self.source_analysis_entry.is_some() {
            return None;
        }
        let input = self.resolved_analysis_input();
        let declarations = crate::command_binding::SourceDeclaredCommandContracts::for_document(
            source,
            self.file_path.as_deref(),
            &input,
            None,
            self.workspace_declared_commands.as_deref(),
            self.workspace_source_modules
                .as_deref()
                .map_or(&[][..], Vec::as_slice),
        );
        crate::realm::document_package_index_survey_realm_with_resolved_input(
            source,
            &input,
            &declarations,
        )
    }

    pub(in super::super) fn realm_build_inputs(&self) -> RealmBuildInputs {
        RealmBuildInputs {
            input: self.resolved_analysis_input(),
            entry: self.source_analysis_entry.clone(),
            file_path: self.file_path.clone(),
            settings: super::super::source_settings::OriginalAnalyserSourceSettings::capture(self),
        }
    }

    /// The realm for `source`: the one the per-item shell already built when it
    /// was built for the same source under equal inputs, otherwise a new one.
    pub(super) fn document_command_realm_reusing(
        &mut self,
        source: &str,
    ) -> crate::realm::CommandBindingRealm {
        if self.source_shell {
            return crate::realm::document_shell_realm_with_resolved_input(
                source,
                &self.resolved_analysis_input(),
            )
            .with_analyser_source_settings(Arc::new(
                super::super::source_settings::OriginalAnalyserSourceSettings::capture(self),
            ));
        }
        if let Some(prebuilt) = self.prebuilt_realm.take()
            && self.source_region_override.is_none()
            && prebuilt.source == source
            && prebuilt.inputs == self.realm_build_inputs()
        {
            self.command_trust = prebuilt.command_trust;
            return prebuilt.realm;
        }
        self.document_command_realm(source)
    }

    pub(in super::super) fn document_command_realm(
        &self,
        source: &str,
    ) -> crate::realm::CommandBindingRealm {
        if let Some(region) = &self.source_region_override {
            // Only the private region entry installs this override after exact
            // input/settings checks. It reuses the actual kernel attachment.
            return crate::realm::conditional_region_realm(region);
        }

        // naming.minifier.logical-source-header
        // docs/design/analysis/name-resolution-proofs/minifier-logical-source-header.md
        // naming.source.original-declared-command-word-contract
        // docs/design/analysis/name-resolution-proofs/source-original-declared-command-word-contract.md
        let input = self.resolved_analysis_input();
        let unavailable_entry = self.source_analysis_entry.as_deref().and_then(|entry| {
            entry
                .source_point_entry
                .as_ref()
                .filter(|point| !point.accepts_analyser_source_settings(self))
                .map(|_| crate::command_binding::SourceAnalysisEntry {
                    metadata_context:
                        crate::registry_invocation::OwnedInvocationMetadataContext::Unavailable,
                    ..entry.clone()
                })
        });
        let entry = unavailable_entry
            .as_ref()
            .or(self.source_analysis_entry.as_deref());
        let declarations = crate::command_binding::SourceDeclaredCommandContracts::for_document(
            source,
            self.file_path.as_deref(),
            &input,
            entry,
            self.workspace_declared_commands.as_deref(),
            self.workspace_source_modules
                .as_deref()
                .map_or(&[][..], Vec::as_slice),
        );
        crate::realm::document_realm_with_declared_contracts(source, &input, entry, &declarations)
            .with_analyser_source_settings(Arc::new(
                super::super::source_settings::OriginalAnalyserSourceSettings::capture(self),
            ))
    }

    /// Supply a pre-built [`crate::compilation_unit::CompilationUnit`] for the
    /// CFG/SSA diagnostic tail, so [`Self::emit_cfg_ssa_diagnostics`] consumes
    /// it (once) instead of rebuilding the whole-file unit.  The supplied unit
    /// must be equal to what the tail would build for this document's source —
    /// the incremental path builds it from memoised per-function lattices, so
    /// only the unchanged-body lattice recompute is skipped.  Reset after the
    /// next `analyse`.
    pub fn set_cu_override(
        &mut self,
        cu: std::sync::Arc<crate::compilation_unit::CompilationUnit>,
    ) {
        self.cu_override = Some(CompilationUnitOverride::Shared(cu));
    }

    /// Supply an owned unit. The same source-equivalence
    /// and consume-once contract as [`Self::set_cu_override`] applies.
    pub fn set_owned_cu_override(&mut self, cu: crate::compilation_unit::CompilationUnit) {
        self.cu_override = Some(CompilationUnitOverride::Owned(Box::new(cu)));
    }
}
