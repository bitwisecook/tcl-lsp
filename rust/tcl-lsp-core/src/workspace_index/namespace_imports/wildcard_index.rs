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

//! Building the per-namespace import index and answering it: export gates, forced shadows, removal events and alias liveness.

use super::super::WorkspaceIndex;
use super::super::records::{
    WorkspaceCommandDeletion, WorkspaceImportGate, WorkspaceNamespaceExport,
    WorkspaceNamespaceForget,
};
use super::{
    CallSite, GlobImportRow, ImportSite, NamespaceImports, WildcardImportIndex, export_events,
    exported_at_site, importing_namespace_of, ns_eq,
};
use crate::namespace_import::ExportVerdict;
use crate::source_graph::RunPoint;
use std::collections::HashSet;
use tcl_syntax::naming::unroot_rooted_key;

/// The names a source namespace had exported by the time one recorded import
/// ran — the word-independent half of
/// [`crate::namespace_import::exported_at_import_site`], decided once per
/// import at index-build time.
///
/// An import's position is fixed by the source text, so which export rows are
/// visible from it, and which of those a `-clear` tombstone revokes, cannot
/// depend on the call being resolved. Only the final glob match can. Asking
/// the whole question per call would run the [`crate::source_graph::RunOrder`]
/// walk once per (invocation × in-scope import × export row); with the
/// timeline half hoisted here it runs once per import, and the per-call cost is
/// a hash probe plus a glob match against however few non-literal patterns
/// remain.
///
/// Splitting literal patterns out is the same reasoning one level down:
/// `namespace export Resistor R Capacitor C …` is the common shape, and a
/// pattern with no metacharacter matches exactly its own text
/// ([`tcl_syntax::glob::is_literal`]), so it belongs in a set rather than in a
/// linear glob scan.
#[derive(Debug, Default)]
pub(in super::super) struct ExportGate<'a> {
    /// Surviving patterns with no glob metacharacter: an exact-name set.
    literal: HashSet<&'a str>,
    /// Original import and export purposes were already checked against the
    /// retained native declaration slot; display membership needs no parser.
    original_tail_matched: bool,
    /// Surviving patterns that need `Tcl_StringMatch` semantics (`*`, `b*`,
    /// `{p[ab]}`, …), in recorded order.
    globs: Vec<&'a str>,
}

#[derive(Clone, Copy)]
pub(in super::super) struct ExportQuery<'p, 's, 'n> {
    pub(in super::super) pattern:
        Option<&'p tcl_compiler::signature_scan::original_name::SourceNamespacePattern>,
    pub(super) subjects: &'s [&'n tcl_compiler::signature_scan::scope::SignatureSourceCommand],
    source_known: bool,
}

impl<'a> ExportGate<'a> {
    /// The gate for `source_ns` as seen from the import at `site`, or an empty
    /// gate when the namespace has no export rows at all.
    pub(in super::super) fn decide(
        exports_by_ns: &std::collections::HashMap<&'a str, Vec<&'a WorkspaceNamespaceExport>>,
        order: &crate::source_graph::RunOrder,
        source_ns: &str,
        site: ImportSite<'_>,
        query: ExportQuery<'_, '_, 'a>,
        purpose: crate::namespace_import::NamespaceStatementPurpose,
    ) -> Self {
        let ExportQuery {
            pattern: original,
            subjects,
            source_known: native_source_known,
        } = query;
        if let Some(original) = original {
            if tcl_syntax::native_glob::NativeGlobProtocol::from_name_policy(
                original.name_input().policy(),
            )
            .export_filter_applies()
                && exports_by_ns.values().flatten().any(|event| {
                    event.unknown
                        && purpose
                            .precedence(
                                order,
                                RunPoint {
                                    uri: &event.uri,
                                    at: event.at,
                                    enclosing_body: event.enclosing_body,
                                },
                                site.point(),
                            )
                            .unwrap_or(true)
                })
            {
                return Self::default();
            }
            let literal = subjects
                .iter()
                .copied()
                .filter(|name| {
                    original.matches_imported_command(name.slot(), name.policy()) == Some(true)
                        && crate::namespace_import::exported_original_for_purpose(
                            exports_by_ns.values().flatten().filter_map(|event| {
                                Some((
                                    event.original.as_ref()?,
                                    RunPoint {
                                        uri: &event.uri,
                                        at: event.at,
                                        enclosing_body: event.enclosing_body,
                                    },
                                ))
                            }),
                            name.slot(),
                            name.policy(),
                            order,
                            site.point(),
                            purpose,
                        ) == ExportVerdict::Exported
                })
                .filter_map(|name| {
                    std::str::from_utf8(original.matched_imported_command_tail(name)?).ok()
                })
                .collect();
            return Self {
                literal,
                globs: Vec::new(),
                original_tail_matched: true,
            };
        }
        if native_source_known {
            return Self::default();
        }
        let Some(exports) = exports_by_ns.get(source_ns) else {
            return Self::default();
        };
        let mut events = export_events(
            exports
                .iter()
                .copied()
                .filter(|event| event.original.is_none() && !event.unknown),
        );
        let surviving = crate::namespace_import::export_patterns_for_purpose(
            &mut events,
            order,
            site.point(),
            purpose,
        );
        let (literal, globs) = surviving
            .into_iter()
            .partition::<Vec<_>, _>(|p| tcl_syntax::glob::is_literal(p));
        Self {
            literal: literal.into_iter().collect(),
            original_tail_matched: false,
            globs,
        }
    }

    /// Whether the gate lets `name` through — the answer
    /// [`crate::namespace_import::exported_at_import_site`] would give for the
    /// same name at the same site.
    pub(in super::super) fn covers(&self, name: &str) -> bool {
        self.literal.contains(name)
            || self
                .globs
                .iter()
                .any(|pattern| tcl_syntax::glob::string_match(pattern, name))
    }
}

impl<'a> NamespaceImports<'a> {
    /// Index `rows` by the names they admit. Called once per namespace per
    /// index build.
    pub(in super::super) fn index(rows: Vec<GlobImportRow<'a>>) -> Self {
        let mut by_name: rustc_hash::FxHashMap<&'a str, Vec<u32>> =
            rustc_hash::FxHashMap::default();
        let mut glob_gated = Vec::new();
        let mut forced = Vec::new();
        for (i, row) in rows.iter().enumerate() {
            let idx = u32::try_from(i).unwrap_or(u32::MAX);
            if row.imp.forced {
                forced.push(idx);
            }
            if !row.exported.globs.is_empty() {
                glob_gated.push(idx);
            }
            for name in &row.exported.literal {
                // The tail pattern is the other half of the admission test,
                // and it is decidable here because the name is known.
                if row.exported.original_tail_matched
                    || tcl_syntax::glob::string_match(&row.imp.tail_pattern, name)
                {
                    by_name.entry(name).or_default().push(idx);
                }
            }
        }
        Self {
            rows,
            by_name,
            glob_gated,
            forced,
        }
    }

    /// The rows whose tail pattern **and** export gate both admit `word`, in
    /// recorded order — the exact subsequence
    /// `rows.iter().filter(tail matches).filter(gate covers)` would yield.
    ///
    /// Order matters: `import_hop` breaks ties on the position within this
    /// sequence, so producing it out of order would change which import wins.
    pub(in super::super) fn admitting(
        &self,
        word: &str,
    ) -> impl Iterator<Item = &GlobImportRow<'a>> {
        let mut idx: Vec<u32> = self
            .by_name
            .get(word)
            .map_or_default(Vec::as_slice)
            .to_vec();
        idx.extend(self.glob_gated.iter().copied().filter(|&i| {
            let row = &self.rows[i as usize];
            tcl_syntax::glob::string_match(&row.imp.tail_pattern, word) && row.exported.covers(word)
        }));
        // A row carrying both literal and glob patterns can arrive twice.
        idx.sort_unstable();
        idx.dedup();
        idx.into_iter().map(|i| &self.rows[i as usize])
    }

    /// The `-force` rows, in recorded order.
    fn forced_rows(&self) -> impl Iterator<Item = &GlobImportRow<'a>> {
        self.forced.iter().map(|&i| &self.rows[i as usize])
    }
}

impl<'a> GlobImportRow<'a> {
    pub(in super::super) fn site(&self) -> ImportSite<'a> {
        self.imp.site()
    }

    /// Whether a fresh interpreter of this import's dialect already holds a
    /// command at `qualified_name` — the registry half of the non-`-force`
    /// "already exists" conflict.
    ///
    /// The workspace half ([`WorkspaceIndex::defines_command`]) sees only
    /// procs and classes the workspace itself declares, so without this an
    /// unforced `namespace import` of a namespace exporting `set` reads as
    /// installed and every bare `set` in the workspace is filed as a
    /// reference to it — where real Tcl raises `can't import command "set":
    /// already exists` and installs nothing (oracle 9.0.4 / 8.6.14). The
    /// single-document tier applies the same gate
    /// (`definition::resolve_called_proc`'s `has_builtin`), from the same
    /// source of truth.
    ///
    /// Abstains toward *not* conflicting when the dialect is unknown:
    /// inventing a conflict drops an alias the program really has.
    pub(in super::super) fn declares_builtin_at(&self, qualified_name: &str) -> bool {
        self.registry
            .is_some_and(|r| r.declares_command_at_rooted_key(qualified_name))
    }
}

/// One recorded **exact** `namespace import ::src::p` link, with its export
/// gate already decided — the exact-pattern twin of [`GlobImportRow`].
#[derive(Debug)]
pub(in super::super) struct ExactImportRow<'a> {
    /// The document the link's import is written in — [`WorkspaceImportGate`]
    /// does not duplicate it.
    pub(in super::super) uri: &'a str,
    gate: &'a WorkspaceImportGate,
    pub(in super::super) exported: ExportGate<'a>,
}

impl<'a> ExactImportRow<'a> {
    pub(in super::super) fn site(&self) -> ImportSite<'a> {
        self.gate.site(self.uri)
    }
}

pub(in super::super) type CommandDeletionsByName<'a> =
    std::collections::HashMap<&'a str, Vec<&'a WorkspaceCommandDeletion>>;

type DeclarationSitesByName<'a> = std::collections::HashMap<&'a str, Vec<(&'a str, u32)>>;

impl<'a> WildcardImportIndex<'a> {
    pub(in super::super) fn build(index: &'a WorkspaceIndex) -> Self {
        Self::build_for_purpose(
            index,
            crate::namespace_import::NamespaceStatementPurpose::Modeled,
        )
    }

    pub(super) fn build_for_purpose(
        index: &'a WorkspaceIndex,
        purpose: crate::namespace_import::NamespaceStatementPurpose,
    ) -> Self {
        // Exports first: every import row's gate is decided against them here,
        // once, instead of once per call site.
        let mut exports_by_ns: std::collections::HashMap<&str, Vec<&WorkspaceNamespaceExport>> =
            std::collections::HashMap::new();
        for exp in index.namespace_exports() {
            exports_by_ns.entry(exp.ns.as_str()).or_default().push(exp);
        }
        let order = index.run_order();
        let original_subjects = index
            .docs
            .iter()
            .flat_map(|doc| {
                doc.original_procs
                    .iter()
                    .map(tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata::name)
                    .chain(doc.original_classes.iter().map(tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata::name))
            })
            .collect::<Vec<_>>();
        let mut rows_by_ns: std::collections::HashMap<&str, Vec<GlobImportRow<'a>>> =
            std::collections::HashMap::new();
        for imp in index.glob_imports() {
            rows_by_ns
                .entry(imp.ns.as_str())
                .or_default()
                .push(GlobImportRow {
                    imp,
                    exported: ExportGate::decide(
                        &exports_by_ns,
                        &order,
                        &imp.source_ns,
                        imp.site(),
                        ExportQuery {
                            pattern: imp.original_pattern.as_ref(),
                            subjects: &original_subjects,
                            source_known: imp.native_source.is_some(),
                        },
                        purpose,
                    ),
                    registry: index.registry_for(&imp.uri),
                });
        }
        let mut exact_by_ns: std::collections::HashMap<&str, Vec<ExactImportRow<'a>>> =
            std::collections::HashMap::new();
        for link in index.command_links() {
            if let Some(gate) = link.import_gate.as_ref() {
                let uri = link.uri.as_str();
                exact_by_ns
                    .entry(importing_namespace_of(&link.linked_qname))
                    .or_default()
                    .push(ExactImportRow {
                        uri,
                        gate,
                        exported: ExportGate::decide(
                            &exports_by_ns,
                            &order,
                            &gate.source_ns,
                            gate.site(uri),
                            ExportQuery {
                                pattern: gate.original_pattern.as_ref(),
                                subjects: &original_subjects,
                                source_known: gate.native_source.is_some(),
                            },
                            purpose,
                        ),
                    });
            }
        }
        let mut forgets_by_ns: std::collections::HashMap<&str, Vec<&WorkspaceNamespaceForget>> =
            std::collections::HashMap::new();
        for fgt in index.namespace_forgets() {
            forgets_by_ns.entry(fgt.ns.as_str()).or_default().push(fgt);
        }
        let deletions_by_name = Self::deletions_by_name(index);
        let declarations_by_qname = Self::declaration_sites_by_name(index);
        let imports_by_ns = rows_by_ns
            .into_iter()
            .map(|(ns, rows)| (ns, NamespaceImports::index(rows)))
            .collect();
        Self {
            source_patterns: index
                .docs
                .iter()
                .flat_map(|document| {
                    document
                        .original_namespace_patterns
                        .iter()
                        .map(|pattern| (document.uri.as_str(), pattern))
                })
                .collect(),
            purpose,
            original_subjects,
            imports_by_ns,
            exact_by_ns,
            exports_by_ns,
            forgets_by_ns,
            deletions_by_name,
            declarations_by_qname,
            order,
            observable: index.observable_namespaces(),
        }
    }

    pub(in super::super) fn deletions_by_name(
        index: &'a WorkspaceIndex,
    ) -> CommandDeletionsByName<'a> {
        let mut deletions_by_name: std::collections::HashMap<&str, Vec<&WorkspaceCommandDeletion>> =
            std::collections::HashMap::new();
        for del in index.command_deletions() {
            deletions_by_name
                .entry(del.qualified_name.as_str())
                .or_default()
                .push(del);
        }
        deletions_by_name
    }

    fn declaration_sites_by_name(index: &'a WorkspaceIndex) -> DeclarationSitesByName<'a> {
        let mut declarations_by_qname: std::collections::HashMap<&str, Vec<(&str, u32)>> =
            std::collections::HashMap::new();
        for (qname, uri, at) in index
            .procs()
            .map(|p| {
                (
                    p.qualified_name.as_str(),
                    p.uri.as_str(),
                    p.name_span.start(),
                )
            })
            .chain(index.classes().map(|c| {
                (
                    c.qualified_name.as_str(),
                    c.uri.as_str(),
                    c.name_span.start(),
                )
            }))
        {
            declarations_by_qname
                .entry(qname)
                .or_default()
                .push((uri, at));
        }
        declarations_by_qname
    }

    /// Whether a live `namespace import -force` in `ns` has taken `name` away
    /// from `ns`'s own command table by the time the call at `call` runs — the
    /// cross-document twin of `definition::forced_import_shadows`.
    ///
    /// `-force` is the one import that outranks a command the importing
    /// namespace already holds, so it is the one case where a *defined*
    /// candidate must not settle a call ([`WorkspaceIndex::settle_invocation`]).
    /// No conflict check: a forced import never conflicts, which is what
    /// `-force` means.
    ///
    /// The export gate abstains toward the shadow for a source namespace no
    /// indexed file declares — the same direction the single-document tier
    /// takes and the same one [`WorkspaceIndex::live_command_links`] takes for
    /// exact imports. Answering with a command the import may have deleted is
    /// worse than answering nothing.
    pub(in super::super) fn forced_shadow_at(
        &self,
        ns: &str,
        name: &str,
        call: CallSite<'_>,
    ) -> bool {
        self.imports_by_ns.get(ns).is_some_and(|imports| {
            // Only the `-force` rows: this is the one path whose export gate
            // can be bypassed (an unobservable source namespace), so it cannot
            // read `NamespaceImports::by_name`.
            imports.forced_rows().any(|row| {
                tcl_syntax::glob::string_match(&row.imp.tail_pattern, name)
                    && (!self.observable.contains(
                        unroot_rooted_key(&row.imp.source_ns).unwrap_or(&row.imp.source_ns),
                    ) || row.exported.covers(name))
                    && self.alias_live_at(ns, &row.imp.source_ns, name, row.site(), call)
            })
        })
    }

    /// [`Self::forced_shadow_at`] over a call's own candidate list, in Tcl's
    /// resolution order — the shape [`WorkspaceIndex::settle_invocation`] has
    /// in hand.
    pub(in super::super) fn forced_shadow_over_candidates(
        &self,
        name: &str,
        resolution_candidates: &[String],
        call: CallSite<'_>,
    ) -> bool {
        if tcl_syntax::naming::is_qualified(name.as_bytes()) {
            return false;
        }
        resolution_candidates.iter().any(|cand| {
            let (prefix, tail) = tcl_syntax::naming::key_holder_and_tail(cand);
            tail == name
                && self.forced_shadow_at(if prefix.is_empty() { "::" } else { prefix }, name, call)
        })
    }

    /// Every removal event bearing on the alias `importing_ns` took from
    /// `source_ns` for `name`, as seen from `query` — the cross-document
    /// half of [`crate::namespace_import::alias_live_at`]'s event log.
    ///
    /// Three kinds, and the ordering rule differs per kind because the
    /// underlying facts differ:
    ///
    /// - **`namespace forget`** and **a redefinition of the imported name**
    ///   are events on *this namespace's* slot. They are ordered only inside
    ///   one document: which of two files loads first is not a static fact
    ///   so an event in another document is passed unordered
    ///   and revokes nothing.
    /// - **Destroying the source command** (`rename ::src::p {}`) is not a
    ///   slot event at all — the command *object* the alias holds is gone,
    ///   workspace-wide, and no load order brings it back. When there is a
    ///   call site to order against it is ordered like the rest; with none
    ///   ([`Self::link_alias_live`]) it is passed as having already happened.
    fn removal_events<'e>(
        &'e self,
        importing_ns: &'e str,
        source_ns: &'e str,
        name: &'e str,
        install_at: ImportSite<'e>,
        query: Option<RunPoint<'e>>,
    ) -> impl Iterator<Item = crate::namespace_import::AliasEvent<'e>> + 'e {
        use crate::namespace_import::{AliasEvent, AliasEventKind};
        let forgets = self
            .forgets_by_ns
            .get(importing_ns)
            .map_or_default(Vec::as_slice)
            .iter()
            .filter(move |f| {
                if self.purpose == crate::namespace_import::NamespaceStatementPurpose::SourceHorizon
                {
                    return self.source_patterns.iter().any(|(uri, forget)| {
                        *uri == f.uri
                            && forget.span().start() == f.at
                            && forget.purpose()
                                == tcl_syntax::naming::NativeNamePurpose::NamespaceForgetPattern
                            && self.source_patterns.iter().any(|(uri, import)| {
                                *uri == install_at.uri
                                    && import.span().start() == install_at.at
                                    && self.original_subjects.iter().any(|command| {
                                        command.slot().simple.try_utf8().ok() == Some(name)
                                            && forget.forgets_source_import(import, command)
                                                == Some(true)
                                    })
                            })
                    });
                }
                f.source_ns.as_deref().is_none_or(|src| {
                    unroot_rooted_key(src).unwrap_or(src)
                        == unroot_rooted_key(source_ns).unwrap_or(source_ns)
                }) && tcl_syntax::glob::string_match(&f.pattern, name)
            })
            .map(move |f| AliasEvent {
                kind: AliasEventKind::Remove,
                at: RunPoint {
                    uri: f.uri.as_str(),
                    at: f.at,
                    enclosing_body: None,
                },
            });
        // A `proc` / class declaration of the *imported* name recreates it as
        // an ordinary command and ends the alias (oracle on
        // `definition::live_import_at` case 5).
        let redefinitions = self
            .declarations_by_qname
            .get(tcl_syntax::naming::qualify(importing_ns, name).as_str())
            .map_or_default(Vec::as_slice)
            .iter()
            .filter(move |_| {
                self.purpose == crate::namespace_import::NamespaceStatementPurpose::Modeled
            })
            .map(move |&(uri, at)| AliasEvent {
                kind: AliasEventKind::Remove,
                at: RunPoint {
                    uri,
                    at,
                    enclosing_body: None,
                },
            });
        let qualified = tcl_syntax::naming::qualify(source_ns, name);
        let deletions = self
            .deletions_by_name
            .get(qualified.as_str())
            .map_or_default(Vec::as_slice)
            .iter()
            .map(|d| RunPoint {
                uri: d.uri.as_str(),
                at: d.at,
                enclosing_body: None,
            })
            .filter(move |&at| match self.purpose {
                crate::namespace_import::NamespaceStatementPurpose::Modeled => true,
                // Source advice drops a destruction unless the call's own
                // document runs it, in straight-line order, before the call.
                crate::namespace_import::NamespaceStatementPurpose::SourceHorizon => query
                    .is_some_and(|query| {
                        query.uri == at.uri
                            && self.order.source_advice_statement_precedence(at, query)
                                == Some(true)
                    }),
            })
            .map(|at| AliasEvent {
                kind: AliasEventKind::Remove,
                at,
            });
        forgets.chain(redefinitions).chain(deletions)
    }

    /// Whether the alias `importing_ns` took from `source_ns` for `name` is
    /// still there when the call at `call` runs — the cross-document binding
    /// of the shared lifecycle decision
    /// [`crate::namespace_import::alias_live_at`].
    ///
    /// `install_at` is the import's own site. It is ordered against the call
    /// **only when the two share a document**: a byte offset in the importing
    /// file and a byte offset in the calling file are unrelated numbers, and
    /// comparing them lets a `namespace forget` in the caller revoke a
    /// cross-file import purely because its local offset happens to be the
    /// larger one. Unordered, the shared function keeps the
    /// alias — the same direction every other cross-file event takes here.
    ///
    /// Within one document the comparison is
    /// [`tcl_compiler::analyser::indirection::in_effect_within`] — the
    /// *identical* rule the same-document tier applies — stated over the
    /// call's own offset and enclosing body span ([`CallSite`]). A plain
    /// offset test is not good enough in either direction: it would leave a
    /// body-local call resolving through a top-level `namespace forget`
    /// written before it (the lenient direction), and, since installs are
    /// order-gated too, it would drop the alias of every proc body calling a
    /// name its own file imports further down (the *un*safe direction, which
    /// is why the span is a required [`CallSite`] field rather than an
    /// optional refinement).
    pub(in super::super) fn alias_live_at(
        &self,
        importing_ns: &str,
        source_ns: &str,
        name: &str,
        install_at: ImportSite<'_>,
        call: CallSite<'_>,
    ) -> bool {
        use crate::namespace_import::{AliasEvent, AliasEventKind};
        let install = std::iter::once(AliasEvent {
            kind: AliasEventKind::Install,
            at: install_at.point(),
        });
        let mut events = install.chain(self.removal_events(
            importing_ns,
            source_ns,
            name,
            install_at,
            Some(call.point()),
        ));
        crate::namespace_import::alias_for_purpose(
            &mut events,
            &self.order,
            Some(call.point()),
            self.purpose,
        )
    }

    /// Whether `ns` already holds a live alias for `name` from a namespace
    /// other than `source_ns` when the import at `site` runs — the conflict a
    /// non-`-force` `namespace import` aborts on.
    ///
    /// Oracle (9.0.4 / 8.6.14): with `::dst` already importing `p` from `::A`,
    /// a later unforced import of `p` from `::B` raises `can't import command
    /// "p": already exists` and leaves `namespace origin ::dst::p` → `::A::p`.
    ///
    /// **Both spellings of an import count on both sides**. Tcl installs one
    /// alias per name; whether the import that installed
    /// it was written as a glob or as an exact pattern is a fact about the
    /// *source text*, not about the command table. The index splits them —
    /// a glob pattern names no single command, so it becomes a
    /// [`WorkspaceGlobImport`] consulted per call, while an exact one becomes
    /// a fixed [`WorkspaceCommandLink`] — and asking each side only about its
    /// own kind would make the conflict rule directional: a glob import that
    /// had already bound the name would not conflict with a later exact
    /// import of it. One function, both tables, so neither caller can drift.
    ///
    /// Ordered within the import's own document only: two imports in
    /// different files have no static load order, and conflicting on a guess
    /// would drop an alias Tcl really installed. A **same-source** re-import
    /// is a silent no-op (oracle), never a conflict — which is also what stops
    /// an import from conflicting with itself.
    ///
    /// Within that document the order is
    /// [`crate::namespace_import::load_order`], shared with the same-document
    /// resolver's slot-log fold. A raw `other.at < site.at` is wrong for
    /// exactly the shape this whole family exists to model: a **body-local**
    /// import is not ordered against a top-level import of its own file by
    /// offset at all, because the file loads — running every top-level
    /// statement, imports included — before any body runs, so the top-level
    /// one owns the name however far below it is written (oracle transcript on
    /// `load_order`). An offset comparison sees `::A` written *later* and lets
    /// the body-local `::B` import install, so navigation answers a source
    /// the program never reaches.
    ///
    /// It is deliberately **not**
    /// [`tcl_compiler::analyser::indirection::in_effect_within`], the
    /// primitive the lifecycle check below runs on: that one is lenient about
    /// events in bodies that may never run, which is the safe direction for a
    /// *removal* and the unsafe one for a conflict — applied here it makes the
    /// two imports above cancel each other and the name resolve nowhere.
    /// [`crate::namespace_import::load_order`] carries the reasoning.
    ///
    /// Self-exclusion comes for free: an import's own key is never less than
    /// itself, so it cannot conflict with itself.
    pub(in super::super) fn conflicting_alias_at(
        &self,
        ns: &str,
        source_ns: &str,
        name: &str,
        site: ImportSite<'_>,
    ) -> bool {
        let call = CallSite {
            uri: site.uri,
            at: site.at,
            enclosing_body: site.enclosing_body,
        };
        // `admitting` applies exactly the two tests this arm needs — the tail
        // pattern covers `name`, and the export gate admits it — as a probe
        // rather than a scan of the namespace. A scan here would multiply with
        // `import_hop`'s own admission walk, since this runs once per row
        // `import_hop` admits.
        //
        // `other_exported.covers(name)` stays in the conjunction below: it is
        // redundant for the glob arm, but the exact arm chained onto it is
        // *not* filtered by the gate, so the test still has work to do there.
        let mut earlier = self
            .imports_by_ns
            .get(ns)
            .map_or_default(|imports| imports.admitting(name).collect::<Vec<_>>())
            .into_iter()
            .map(|other| (other.imp.source_ns.as_str(), other.site(), &other.exported))
            .chain(
                self.exact_by_ns
                    .get(ns)
                    .map_or_default(Vec::as_slice)
                    .iter()
                    .filter(|other| other.gate.name == name)
                    .map(|other| (other.gate.source_ns.as_str(), other.site(), &other.exported)),
            );
        earlier.any(|(other_source, other_site, other_exported)| {
            self.purpose
                .comparison(&self.order, other_site.point(), site.point())
                == Some(std::cmp::Ordering::Less)
                && !ns_eq(other_source, source_ns)
                && other_exported.covers(name)
                && self.alias_certainly_live_at(ns, other_source, name, other_site, call)
        })
    }

    /// [`Self::alias_live_at`] for the conflict premise: the earlier alias
    /// must be certainly live at the later import, so a removal whose position
    /// is unknown (another document, a deferred body, a conditional branch)
    /// leaves it possibly dead and the later install stands
    /// ([`crate::namespace_import::alias_certainly_live`]).
    fn alias_certainly_live_at(
        &self,
        importing_ns: &str,
        source_ns: &str,
        name: &str,
        install_at: ImportSite<'_>,
        call: CallSite<'_>,
    ) -> bool {
        use crate::namespace_import::{AliasEvent, AliasEventKind};
        let install = std::iter::once(AliasEvent {
            kind: AliasEventKind::Install,
            at: install_at.point(),
        });
        let mut events = install.chain(self.removal_events(
            importing_ns,
            source_ns,
            name,
            install_at,
            Some(call.point()),
        ));
        crate::namespace_import::alias_certainly_live(
            &mut events,
            &self.order,
            call.point(),
            self.purpose,
        )
    }

    /// Whether an **exact** import link is still installed, with no call site
    /// to order against — the gate [`WorkspaceIndex::live_command_links`]
    /// applies.
    ///
    /// `namespace import ::src::p` produces a fixed `WorkspaceCommandLink`
    /// rather than a per-call glob lookup, so without this gate the export
    /// snapshot and the `-force` conflict would be the only things gating it:
    /// after `namespace forget ::src::p` — or `rename ::src::p {}` — the link
    /// would stay live and cross-document definition / references would still
    /// resolve `::dst::p`.
    ///
    /// The question a link answers is "does this alias exist for navigation",
    /// which has no query point of its own, so it reads the state after the
    /// document completes normally, as tclsh leaves it after `source`.
    ///
    /// - On workspace navigation a forget or redefinition applies only when
    ///   its document's end is admitted and it is a sequence position before
    ///   that end; it then revokes the link when written *after* the import
    ///   in the same document. Where the end is withheld (a possible early
    ///   exit, an unretained script, a function call, unresolved words), or
    ///   the removal sits in a deferred body or a branch, the order stays
    ///   conditional and the link stays live.
    /// - An explicitly constructed order treats every recorded removal as
    ///   having run.
    /// - Either way the removal is ordered against the **import**: one
    ///   written before it is undone by the import itself (a re-import after
    ///   a forget reinstalls — oracle), and one in another document revokes
    ///   nothing, exactly as in [`Self::alias_live_at`].
    /// - A destroyed source command revokes regardless, and is **not** put on
    ///   the timeline at all: the command object is gone workspace-wide
    ///   (oracle: `rename ::src::p {}` makes `::dst::p` an `invalid command
    ///   name` and empties `info commands ::dst::*`) and no load order brings
    ///   it back, so with no query point to order anything against it is
    ///   decided before the fold runs.  Encoding it as an event at `u32::MAX`
    ///   instead would be wrong for a **body-local** import gate: the
    ///   load-order rule reads a removal written outside the import's own body
    ///   as having run *before* it — right for a `namespace forget`, which the
    ///   import then undoes, and exactly backwards for a destruction the
    ///   import cannot undo.
    pub(in super::super) fn link_alias_live(
        &self,
        importing_ns: &str,
        gate: &WorkspaceImportGate,
        uri: &str,
    ) -> bool {
        use crate::namespace_import::{AliasEvent, AliasEventKind};
        if self
            .deletions_by_name
            .contains_key(tcl_syntax::naming::qualify(&gate.source_ns, &gate.name).as_str())
        {
            return false;
        }
        let install = std::iter::once(AliasEvent {
            kind: AliasEventKind::Install,
            at: gate.site(uri).point(),
        });
        let mut events = install.chain(self.removal_events(
            importing_ns,
            &gate.source_ns,
            &gate.name,
            gate.site(uri),
            None,
        ));
        crate::namespace_import::alias_live_at(&mut events, &self.order, None)
    }

    /// Whether `ns` (`::`-rooted) had exported the unqualified `name` **as of
    /// the import site** at `import_at` in `import_uri` — the cross-document
    /// half of the wildcard-import gate (real Tcl only imports names a source
    /// namespace has actually exported — `Tcl_Export`, `tclNamesp.c`), taken
    /// per import site rather than against the workspace's final export state.
    ///
    /// Delegates to [`crate::namespace_import::exported_at_import_site`] — the
    /// same function the same-document resolver
    /// (`definition::exported_original_slot_at_import_in`) calls, so the two tiers cannot
    /// disagree about what an import site sees. The only tier-specific part
    /// is which events are *ordered* against the import: those in the
    /// import's own document, compared by offset. An export in another
    /// document has no static order relative to this import (nothing fixes
    /// which file loads first), so it is passed unordered and the shared
    /// function abstains toward continuing to resolve.
    pub(in super::super) fn exports_name_at(
        &self,
        ns: &str,
        name: &str,
        site: ImportSite<'_>,
    ) -> bool {
        let mut found = None;
        for subject in &self.original_subjects {
            let scope = tcl_compiler::signature_scan::scope::SignatureNamespaceScope::C(
                subject.slot().namespace.clone(),
            );
            if subject.slot().simple.try_utf8().ok() != Some(name)
                || !scope.display().is_some_and(|label| ns_eq(&label, ns))
            {
                continue;
            }
            if found.is_some_and(
                |previous: &tcl_compiler::signature_scan::scope::SignatureSourceCommand| {
                    previous.slot() != subject.slot() || previous.policy() != subject.policy()
                },
            ) {
                return false;
            }
            found = Some(*subject);
        }
        if let Some(subject) = found {
            if self.exports_by_ns.values().flatten().any(|event| {
                event.unknown
                    && self
                        .order
                        .navigation_precedence(
                            RunPoint {
                                uri: &event.uri,
                                at: event.at,
                                enclosing_body: event.enclosing_body,
                            },
                            site.point(),
                        )
                        .and_then(
                            crate::source_graph::SourceNavigationOrder::modeled_source_exclusion,
                        )
                        .unwrap_or(true)
            }) {
                return false;
            }
            return crate::namespace_import::exported_original_at_import_site(
                self.exports_by_ns.values().flatten().filter_map(|event| {
                    Some((
                        event.original.as_ref()?,
                        RunPoint {
                            uri: &event.uri,
                            at: event.at,
                            enclosing_body: event.enclosing_body,
                        },
                    ))
                }),
                subject.slot(),
                subject.policy(),
                &self.order,
                site.point(),
            ) == ExportVerdict::Exported;
        }
        self.exports_by_ns.get(ns).is_some_and(|exports| {
            exported_at_site(
                exports
                    .iter()
                    .copied()
                    .filter(|event| event.original.is_none() && !event.unknown),
                name,
                &self.order,
                site.point(),
            )
        })
    }
}
