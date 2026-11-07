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

//! Actual naming/recovery settings, independently of physical document policy.

use super::Analyser;
use std::collections::HashSet;
use std::hash::{Hash, Hasher};
use std::sync::Arc;

/// Opaque actual naming and recovery settings retained by source producers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OriginalAnalyserSourceSettings {
    extra_commands: Arc<HashSet<String>>,
    pack_overlay: u64,
    library_versions: tcl_dialect::LibraryVersionOverrides,
    declared_targets: Vec<(String, String)>,
    package_provides: Vec<(String, Vec<String>)>,
    deep_param_traits: bool,
    structure_only: bool,
    defer_proc_bodies: bool,
    workspace_classes: HashSet<String>,
    workspace_bare_word_classes: HashSet<String>,
    workspace_class_factories: Option<Arc<super::types::ClassFactoryIndex>>,
    workspace_subclass_methods: Option<Arc<super::types::SubclassProvidedMethods>>,
    workspace_declared_commands: Option<Arc<tcl_registry::model::DeclaredSurface>>,
    workspace_source_modules: Option<Arc<Vec<crate::command_binding::TrustedSourceModuleLoader>>>,
}
impl Hash for OriginalAnalyserSourceSettings {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // Bucket selection only; complete Eq above retains every typed fact.
        self.pack_overlay.hash(state);
        self.declared_targets.hash(state);
        self.package_provides.hash(state);
        self.deep_param_traits.hash(state);
        self.structure_only.hash(state);
        self.defer_proc_bodies.hash(state);
    }
}
impl OriginalAnalyserSourceSettings {
    pub(crate) fn capture(analyser: &Analyser) -> Self {
        Self {
            extra_commands: Arc::clone(&analyser.extra_commands),
            pack_overlay: analyser.pack_overlay,
            library_versions: analyser.library_versions.clone(),
            declared_targets: analyser.declared_targets.clone(),
            package_provides: analyser.package_provides.clone(),
            deep_param_traits: analyser.deep_param_traits,
            structure_only: analyser.structure_only,
            defer_proc_bodies: analyser.defer_proc_bodies,
            workspace_classes: analyser.workspace_classes.clone(),
            workspace_bare_word_classes: analyser.workspace_bare_word_classes.clone(),
            workspace_class_factories: analyser.workspace_class_factories.clone(),
            workspace_subclass_methods: analyser.workspace_subclass_methods.clone(),
            workspace_declared_commands: analyser.workspace_declared_commands.clone(),
            workspace_source_modules: analyser.workspace_source_modules.clone(),
        }
    }
    pub(crate) fn apply(&self, analyser: &mut Analyser) {
        analyser.extra_commands = Arc::clone(&self.extra_commands);
        analyser.pack_overlay = self.pack_overlay;
        analyser.library_versions.clone_from(&self.library_versions);
        analyser.declared_targets.clone_from(&self.declared_targets);
        analyser.package_provides.clone_from(&self.package_provides);
        analyser.deep_param_traits = self.deep_param_traits;
        analyser.structure_only = self.structure_only;
        analyser.defer_proc_bodies = self.defer_proc_bodies;
        analyser
            .workspace_classes
            .clone_from(&self.workspace_classes);
        analyser
            .workspace_bare_word_classes
            .clone_from(&self.workspace_bare_word_classes);
        analyser
            .workspace_class_factories
            .clone_from(&self.workspace_class_factories);
        analyser
            .workspace_subclass_methods
            .clone_from(&self.workspace_subclass_methods);
        analyser
            .workspace_declared_commands
            .clone_from(&self.workspace_declared_commands);
        analyser
            .workspace_source_modules
            .clone_from(&self.workspace_source_modules);
    }
}

impl Analyser {
    /// Capture the actual naming/recovery settings before source compilation.
    /// This supplies no runtime entry, handler selection or rewrite authority.
    #[must_use]
    pub fn source_compilation_settings(&self) -> Arc<OriginalAnalyserSourceSettings> {
        Arc::new(OriginalAnalyserSourceSettings::capture(self))
    }
}

impl super::AnalysisResult {
    fn current_source_settings(
        &self,
        source: &str,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<&OriginalAnalyserSourceSettings> {
        crate::source_graph::current_analysis(source, self)?;
        let input = self.resolved_input.as_ref()?;
        let realm = self.retained_command_realm()?;
        if !realm.matches_resolved_analysis_input(input)
            || registry.snapshot() != input.borrowed_context_registry().commands().snapshot()
        {
            return None;
        }
        realm
            .source_bindings_ref()
            .retained_analyser_source_settings()
    }

    /// Restore the same actual naming/recovery settings without reconstructing
    /// them from a profile, resource folder or report labels. No entry is minted.
    pub fn configure_source_analyser(
        &self,
        source: &str,
        registry: &tcl_registry::CommandRegistry,
        analyser: &mut Analyser,
    ) -> bool {
        let Some(settings) = self.current_source_settings(source, registry) else {
            return false;
        };
        settings.apply(analyser);
        true
    }

    /// Compare every retained naming/recovery option after a proposed secondary
    /// configuration. Resolved input, entry and host currency need their own joins.
    #[must_use]
    pub fn accepts_source_analyser_settings(
        &self,
        source: &str,
        registry: &tcl_registry::CommandRegistry,
        analyser: &Analyser,
    ) -> bool {
        self.current_source_settings(source, registry)
            .is_some_and(|settings| settings == &OriginalAnalyserSourceSettings::capture(analyser))
    }
}
