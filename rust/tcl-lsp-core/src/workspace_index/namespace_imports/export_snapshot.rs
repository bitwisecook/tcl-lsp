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

//! The whole-program `namespace export` oracle the single-document tier consults.

use super::super::WorkspaceIndex;
use super::super::records::WorkspaceNamespaceExport;
use super::{exported_at_site, ns_eq};
use crate::namespace_import::ExportVerdict;
use crate::source_graph::RunPoint;
use std::collections::HashSet;
use std::sync::Arc;
use tcl_syntax::naming::unroot_rooted_key;

impl WorkspaceIndex {
    /// The whole-program export oracle the *single-document* tier consults
    /// when deciding whether a `namespace import -force` really deleted the
    /// importing namespace's own command.
    ///
    /// One document cannot answer that on its own: the `namespace export` that
    /// decides it may live in another file, and two programs whose single
    /// document is byte-identical then disagree — the transcript is on
    /// [`crate::namespace_import::NamespaceExportOracle`]. This is the
    /// whole-program evidence that closes the gap, and nothing more: it
    /// answers one question and may answer
    /// [`ExportVerdict::Unknown`].
    ///
    /// A [`Derived`] view, so it is built at most once per
    /// [`Self::generation`]; the returned `Arc` is owned, so a caller may hold
    /// it after releasing the index lock (which is how the server hands it to
    /// its blocking providers).
    #[must_use]
    pub fn export_snapshot(&self) -> Arc<NamespaceExportSnapshot> {
        self.export_snapshot.get_or_build(|| {
            let mut exports_by_ns: std::collections::HashMap<
                String,
                Vec<WorkspaceNamespaceExport>,
            > = std::collections::HashMap::new();
            for exp in self.namespace_exports() {
                exports_by_ns
                    .entry(unroot_rooted_key(&exp.ns).unwrap_or(&exp.ns).to_owned())
                    .or_default()
                    .push(exp.clone());
            }
            NamespaceExportSnapshot {
                original_exports: self.namespace_exports().cloned().collect(),
                original_declarations: self
                    .docs
                    .iter()
                    .flat_map(|doc| {
                        doc.original_procs
                            .iter()
                            .map(|record| record.name().clone())
                            .chain(
                                doc.original_classes
                                    .iter()
                                    .map(|record| record.name().clone()),
                            )
                    })
                    .collect(),
                exports_by_ns,
                observable: self
                    .observable_namespaces()
                    .into_iter()
                    .map(str::to_owned)
                    .collect(),
                order: self.run_order(),
            }
        })
    }
}

/// The workspace's `namespace export` records, plus the run order and the
/// observable-namespace set needed to read them — an **owned**, self-contained
/// [`crate::namespace_import::NamespaceExportOracle`] the single-document tier
/// can borrow.
///
/// Owned rather than a view borrowing [`WorkspaceIndex`] because the server's
/// pure-CPU providers run on a blocking worker with the index lock released:
/// the snapshot is taken under the lock and moved into the worker. It is a
/// [`Derived`] view, so a whole generation's requests share one build.
///
/// It holds *only* what the export question needs. Everything else about an
/// import — the conflict rule, the alias lifecycle, chain following — stays
/// where it already is; this is not a second cross-document resolver.
#[derive(Debug, Default)]
pub struct NamespaceExportSnapshot {
    original_exports: Vec<WorkspaceNamespaceExport>,
    original_declarations: Vec<tcl_compiler::signature_scan::scope::SignatureSourceCommand>,
    /// Export rows by exporting namespace, `::`-stripped so the two spellings
    /// an import pattern and an export record may carry still meet.
    pub(super) exports_by_ns: std::collections::HashMap<String, Vec<WorkspaceNamespaceExport>>,
    /// The `::`-stripped namespaces the workspace can say anything about — the
    /// discriminator between [`ExportVerdict::NotExported`] and
    /// [`ExportVerdict::Unknown`], and the same set
    /// [`WorkspaceIndex::live_command_links`] gates its own abstention on.
    pub(super) observable: HashSet<String>,
    /// The `source`-graph load order, so an export the graph proves runs
    /// *after* the import is not retroactive.
    pub(super) order: Arc<crate::source_graph::RunOrder>,
}

impl crate::namespace_import::NamespaceExportOracle for NamespaceExportSnapshot {
    fn exported_at(&self, source_ns: &str, name: &str, import_site: RunPoint<'_>) -> ExportVerdict {
        let mut subject = None;
        for declaration in &self.original_declarations {
            let Ok(tail) = declaration.slot().simple.try_utf8() else {
                continue;
            };
            let scope = tcl_names::namespace::SignatureNamespaceScope::C(
                declaration.slot().namespace.clone(),
            );
            if tail != name
                || !scope
                    .display()
                    .is_some_and(|label| ns_eq(&label, source_ns))
            {
                continue;
            }
            if subject.is_some_and(
                |previous: &tcl_compiler::signature_scan::scope::SignatureSourceCommand| {
                    previous.slot() != declaration.slot()
                        || previous.policy() != declaration.policy()
                },
            ) {
                return ExportVerdict::Unknown;
            }
            subject = Some(declaration);
        }
        if let Some(subject) = subject {
            return self.exported_at_original(subject.slot(), subject.policy(), import_site);
        }
        // Explicit lexical-only authored inventories remain advice. Native
        // source records without an original publication do not enter this path.
        let ns = unroot_rooted_key(source_ns).unwrap_or(source_ns);
        if !self.observable.contains(ns) {
            return ExportVerdict::Unknown;
        }
        let exports = self
            .exports_by_ns
            .get(ns)
            .into_iter()
            .flatten()
            .filter(|event| event.original.is_none() && !event.unknown);
        if exported_at_site(exports, name, &self.order, import_site) {
            ExportVerdict::Exported
        } else {
            ExportVerdict::Unknown
        }
    }

    fn exported_at_original(
        &self,
        slot: &tcl_core_types::ByteCommandSlot,
        policy: tcl_syntax::naming::NamePolicyProtocol,
        import_site: RunPoint<'_>,
    ) -> ExportVerdict {
        if tcl_syntax::native_glob::NativeGlobProtocol::from_name_policy(policy)
            .export_filter_applies()
            && self.original_exports.iter().any(|event| {
                event.unknown
                    && self
                        .order
                        .navigation_precedence(
                            RunPoint {
                                uri: &event.uri,
                                at: event.at,
                                enclosing_body: event.enclosing_body,
                            },
                            import_site,
                        )
                        .and_then(
                            crate::source_graph::SourceNavigationOrder::modeled_source_exclusion,
                        )
                        .unwrap_or(true)
            })
        {
            return ExportVerdict::Unknown;
        }
        crate::namespace_import::exported_original_at_import_site(
            self.original_exports.iter().filter_map(|event| {
                Some((
                    event.original.as_ref()?,
                    RunPoint {
                        uri: &event.uri,
                        at: event.at,
                        enclosing_body: event.enclosing_body,
                    },
                ))
            }),
            slot,
            policy,
            &self.order,
            import_site,
        )
    }
}
