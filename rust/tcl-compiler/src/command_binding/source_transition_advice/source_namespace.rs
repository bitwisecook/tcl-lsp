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

//! Policy-projected namespace advice shares source origins and mutations.

use super::{AdviceCell, AdviceGraph, AdviceNamingPolicy, OriginalSourceCommandTransition};
use std::sync::Arc;
use tcl_core_types::{ByteCommandSlot, ByteNamespacePath};
use tcl_registry::{NamespaceTransition, NamespaceTransitionTarget};
use tcl_syntax::naming::NativeNamePurpose;
use tcl_syntax::native_glob::NativeNameGlobPurpose;

impl AdviceGraph {
    pub(super) fn withdraw_namespace_advice(&mut self) {
        self.unknown_exports.extend(self.namespaces.iter().cloned());
        self.unknown_paths.extend(self.namespaces.iter().cloned());
        self.namespace_paths.clear();
    }

    pub(super) fn remove_namespace_import_origin(&mut self, origin: &ByteCommandSlot) {
        let linked = self
            .namespace_imports
            .iter()
            .filter_map(|(slot, source)| (source == origin).then_some(slot.clone()))
            .collect::<Vec<_>>();
        for slot in linked {
            self.namespace_imports.remove(&slot);
            self.cells.insert(slot, AdviceCell::Deleted);
        }
    }

    pub(super) fn move_namespace_import_origin(
        &mut self,
        from: &ByteCommandSlot,
        to: &ByteCommandSlot,
        receipt: &Arc<OriginalSourceCommandTransition>,
    ) {
        for (slot, source) in &mut self.namespace_imports {
            if source == from {
                *source = to.clone();
                if let Some(AdviceCell::Descriptor(_, lineage)) = self.cells.get_mut(slot) {
                    lineage.push(Arc::clone(receipt));
                }
            }
        }
    }

    pub(super) fn logical_transition_namespace(
        &self,
        receipt: &OriginalSourceCommandTransition,
        target: &NamespaceTransitionTarget,
    ) -> Option<ByteNamespacePath> {
        let current = self.logical_namespace.as_ref()?;
        match target {
            NamespaceTransitionTarget::Current => Some(current.clone()),
            NamespaceTransitionTarget::Named(subject) => {
                let input = receipt.input(subject)?.logical_input()?;
                tcl_syntax::naming::authored_source_namespace_path(current, input.bytes())
            }
        }
    }

    fn apply_source_namespace_export(
        &mut self,
        receipt: &Arc<OriginalSourceCommandTransition>,
        namespace: &NamespaceTransitionTarget,
        patterns: &[tcl_registry::TransitionSubject],
    ) -> Option<()> {
        let namespace = self.source_transition_namespace(receipt, namespace)?;
        let clear = match &self.policy {
            AdviceNamingPolicy::Logical(_) => {
                NamespaceTransition::export_pattern_operands(patterns)?.0
            }
            AdviceNamingPolicy::Native(policy) => {
                let inputs = patterns
                    .iter()
                    .map(|subject| {
                        receipt
                            .input(subject)?
                            .native_input()
                            .filter(|input| input.policy() == *policy)
                            .map(crate::signature_scan::scope::SignatureSourceNameInput::bytes)
                    })
                    .collect::<Option<Vec<_>>>()?;
                NamespaceTransition::export_pattern_byte_operands(&inputs, *policy)?.0
            }
        };
        let patterns = patterns[usize::from(clear)..]
            .iter()
            .map(|subject| {
                let (source, pattern) = self.source_namespace_pattern(
                    &namespace,
                    receipt.input(subject)?,
                    NativeNamePurpose::NamespaceExportPattern,
                )?;
                source.is_none().then_some(pattern)
            })
            .collect::<Option<Vec<_>>>()?;
        if clear {
            self.namespace_exports.insert(namespace.clone(), patterns);
            self.unknown_exports.remove(&namespace);
        } else {
            self.namespace_exports
                .entry(namespace)
                .or_default()
                .extend(patterns);
        }
        Some(())
    }

    fn apply_source_namespace_import(
        &mut self,
        receipt: &Arc<OriginalSourceCommandTransition>,
        namespace: &NamespaceTransitionTarget,
        force: Option<bool>,
        patterns: &[tcl_registry::TransitionSubject],
    ) -> Option<()> {
        let namespace = self.source_transition_namespace(receipt, namespace)?;
        let force = force?;
        for pattern in patterns {
            let (source, pattern) = self.source_namespace_pattern(
                &namespace,
                receipt.input(pattern)?,
                NativeNamePurpose::NamespaceImportPattern,
            )?;
            let source = source?;
            if source == namespace || !self.namespaces.contains(&source) {
                return None;
            }
            let exports = self
                .namespace_exports
                .get(&source)
                .map_or(&[][..], Vec::as_slice);
            let mut matches = Vec::new();
            for (slot, cell) in &self.cells {
                if slot.namespace != source
                    || matches!(cell, AdviceCell::Deleted)
                    || !self.source_namespace_match(
                        NativeNameGlobPurpose::ImportSearch,
                        pattern.as_bytes(),
                        slot.simple.as_bytes(),
                    )?
                {
                    continue;
                }
                let mut exported = false;
                for pattern in exports {
                    exported |= self.source_namespace_match(
                        NativeNameGlobPurpose::ExportFilter,
                        pattern.as_bytes(),
                        slot.simple.as_bytes(),
                    )?;
                }
                if exported {
                    matches.push((slot.clone(), cell.clone()));
                }
            }
            for (source, cell) in matches {
                let destination = ByteCommandSlot::new(namespace.clone(), source.simple.clone());
                let origin = self
                    .namespace_imports
                    .get(&source)
                    .unwrap_or(&source)
                    .clone();
                if origin == destination {
                    return None;
                }
                if self.namespace_imports.get(&destination) == Some(&origin) {
                    continue;
                }
                if !force
                    && self
                        .cells
                        .get(&destination)
                        .is_some_and(|cell| !matches!(cell, AdviceCell::Deleted))
                {
                    return None;
                }
                self.remove_namespace_import_origin(&destination);
                let imported = match cell {
                    AdviceCell::Descriptor(command, mut lineage) => {
                        lineage.push(Arc::clone(receipt));
                        AdviceCell::Descriptor(command, lineage)
                    }
                    _ => AdviceCell::Shadowed,
                };
                self.namespace_imports.insert(destination.clone(), origin);
                self.cells.insert(destination, imported);
            }
        }
        Some(())
    }

    fn apply_source_namespace_forget(
        &mut self,
        receipt: &Arc<OriginalSourceCommandTransition>,
        namespace: &NamespaceTransitionTarget,
        patterns: &[tcl_registry::TransitionSubject],
    ) -> Option<()> {
        let namespace = self.source_transition_namespace(receipt, namespace)?;
        for pattern in patterns {
            let (holder, pattern) = self.source_namespace_pattern(
                &namespace,
                receipt.input(pattern)?,
                NativeNamePurpose::NamespaceForgetPattern,
            )?;
            let mut removed = Vec::new();
            for (slot, source) in &self.namespace_imports {
                if slot.namespace != namespace {
                    continue;
                }
                let matched = if let Some(holder) = &holder {
                    source.namespace == *holder
                        && self.source_namespace_match(
                            NativeNameGlobPurpose::ForgetOriginFilter,
                            pattern.as_bytes(),
                            source.simple.as_bytes(),
                        )?
                } else {
                    self.source_namespace_match(
                        NativeNameGlobPurpose::ForgetOwnSearch,
                        pattern.as_bytes(),
                        slot.simple.as_bytes(),
                    )?
                };
                if matched {
                    removed.push(slot.clone());
                }
            }
            for slot in removed {
                self.namespace_imports.remove(&slot);
                self.cells.insert(slot, AdviceCell::Deleted);
            }
        }
        Some(())
    }

    pub(super) fn apply_source_namespace(
        &mut self,
        receipt: &Arc<OriginalSourceCommandTransition>,
        transition: &NamespaceTransition,
    ) -> Option<()> {
        if self.dialect.family() != Some(tcl_dialect::model::Family::Tcl) {
            return None;
        }
        match transition {
            NamespaceTransition::Export {
                namespace,
                patterns,
            } => {
                self.apply_source_namespace_export(receipt, namespace, patterns)?;
            }
            NamespaceTransition::Import {
                namespace,
                force,
                patterns,
            } => {
                self.apply_source_namespace_import(receipt, namespace, *force, patterns)?;
            }
            NamespaceTransition::Forget {
                namespace,
                patterns,
            } => {
                self.apply_source_namespace_forget(receipt, namespace, patterns)?;
            }
            NamespaceTransition::SetPath { namespace, path } => {
                if self
                    .dialect
                    .tcl_version
                    .is_some_and(|version| version < tcl_dialect::TclVersion::V8_5)
                {
                    return None;
                }
                let namespace = self.source_transition_namespace(receipt, namespace)?;
                let path = self.source_namespace_path_list(&namespace, receipt.input(path)?)?;
                self.unknown_paths.remove(&namespace);
                self.namespace_paths.insert(namespace, path);
            }
            NamespaceTransition::Delete { namespace } => {
                let namespace = self.source_transition_namespace(receipt, namespace)?;
                if namespace.is_root() {
                    return None;
                }
                let removed = self
                    .cells
                    .keys()
                    .filter(|slot| {
                        slot.namespace
                            .as_segments()
                            .starts_with(namespace.as_segments())
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                for slot in removed {
                    self.remove_namespace_import_origin(&slot);
                    self.namespace_imports.remove(&slot);
                    self.cells.insert(slot, AdviceCell::Deleted);
                }
                self.namespaces
                    .retain(|path| !path.as_segments().starts_with(namespace.as_segments()));
            }
            _ => return None,
        }
        Some(())
    }
}

#[cfg(test)]
mod tests {
    use crate::analyser::Analyser;
    use crate::command_binding::source_transition_advice::OriginalSourceTransitionAdviceLookup;
    use crate::command_binding::{
        OriginalSourceCommandTransitionAdvice, SourceCommandTransitionObligation,
    };

    fn query(source: &str, head: &str) -> Option<OriginalSourceCommandTransitionAdvice> {
        let analysis = Analyser::new().analyse(source, "tcl");
        let context = analysis.resolved_input.as_ref().unwrap().context_registry();
        let tape = analysis
            .retained_command_realm()
            .unwrap()
            .source_bindings_ref()
            .original_source_transition_advice_tape(&context)
            .unwrap();
        let offset = u32::try_from(source.rfind(head).unwrap()).unwrap();
        match tape.lookup(offset) {
            OriginalSourceTransitionAdviceLookup::Advice(advice) => Some(advice.clone()),
            OriginalSourceTransitionAdviceLookup::Refused
            | OriginalSourceTransitionAdviceLookup::Unrepresented => None,
        }
    }

    #[test]
    fn logical_namespace_imports_retain_conditional_source_roles_and_origins() {
        for prefix in [
            "namespace import ::tcltest::*; ",
            "namespace import ::tcltest::*; rename ::tcltest::test ::tcltest::moved; ",
            "proc test args {}; namespace import -force ::tcltest::*; ",
            "namespace import ::tcltest::*; opaque; ",
        ] {
            let source = format!("{prefix}test naming-1 description -body {{set x 1}} -result 1");
            let advice = query(&source, "test naming-1").unwrap_or_else(|| panic!("{source}"));
            assert_eq!(advice.command(), "tcltest::test", "{source}");
            assert!(
                advice
                    .obligations()
                    .contains(&SourceCommandTransitionObligation::UnavailableActualLookup)
            );
            assert!(
                advice
                    .obligations()
                    .contains(&SourceCommandTransitionObligation::WrittenTransitionApplicability)
            );
            assert!(
                advice
                    .arguments()
                    .iter()
                    .any(|argument| argument.value.as_deref() == Some(b"-body")),
                "{source}"
            );
            if prefix.contains("opaque") {
                assert!(
                    advice
                        .obligations()
                        .contains(&SourceCommandTransitionObligation::UnknownEarlierMutation)
                );
            }
        }
    }

    #[test]
    fn logical_namespace_imports_refuse_known_shadow_forget_and_origin_deletion() {
        for prefix in [
            "proc test args {}; namespace import ::tcltest::*; ",
            "namespace import ::tcltest::*; proc test args {}; ",
            "namespace import ::tcltest::*; namespace forget ::tcltest::*; ",
            "namespace import ::tcltest::*; namespace forget test; ",
            "namespace import ::tcltest::*; rename ::tcltest::test {}; ",
            "namespace import ::tcltest::*; proc ::tcltest::test args {}; ",
            "namespace import ::tcltest::*; namespace delete ::tcltest; ",
        ] {
            let source = format!("{prefix}test naming-1 description -body {{set x 1}} -result 1");
            assert!(query(&source, "test naming-1").is_none(), "{source}");
        }
    }

    #[test]
    fn logical_namespace_path_uses_generic_registry_descriptors_and_clear_barriers() {
        let prefix = "namespace eval ::provider {}; rename if ::provider::choose; ";
        let source = format!("{prefix}namespace path ::provider; choose 1 {{puts ok}}");
        let advice = query(&source, "choose 1").expect("path candidate");
        assert_eq!(advice.command(), "if");
        for mutation in [
            "namespace path {}; ",
            "namespace path $dynamic; ",
            "namespace path {::provider ::missing}; ",
            "proc choose args {}; ",
            "rename ::provider::choose {}; ",
        ] {
            let source =
                format!("{prefix}namespace path ::provider; {mutation}choose 1 {{puts ok}}");
            assert!(query(&source, "choose 1").is_none(), "{source}");
        }
    }

    #[test]
    fn logical_namespace_nested_import_and_export_clear_keep_own_scope() {
        let source = concat!(
            "package require tcltest; ",
            "namespace eval ::client {",
            "namespace import ::tcltest::*; ",
            "test naming-1 description -body {set x 1} -result 1}",
        );
        assert_eq!(
            query(source, "test naming-1")
                .expect("nested import")
                .command(),
            "tcltest::test"
        );
        let source = concat!(
            "namespace eval ::tcltest {namespace export -clear; ",
            "namespace eval ::client {namespace import ::tcltest::*; ",
            "test naming-1 description -body {set x 1} -result 1}}",
        );
        assert!(query(source, "test naming-1").is_none());
    }
}
