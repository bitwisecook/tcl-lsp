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

//! The iRulesLX method relation: definition and references across the workspace and its plugins.

use super::*;

/// A snapshot of what the editor holds for every open document, by path.
///
/// Built by [`Backend::open_document_texts`] and handed to the iRulesLX
/// cross-file walk, which reads other files itself and must see the same bytes
/// [`Backend::read_document`] would — the open buffer first, disk only for what
/// is not open.
///
/// It also carries per-document input for the relation's sibling rule files:
/// each open iRules document's own analysis, and — added per request by
/// [`Backend::supply_sibling_rule_inputs`] — the text and own analysis of
/// every other sibling rule file the request reads, open or closed.
struct OpenDocumentTexts {
    texts: HashMap<PathBuf, Arc<str>>,
    pub(in super::super) analyses: HashMap<PathBuf, Arc<AnalysisResult>>,
}

impl tcl_lsp_core::ilx_navigation::OpenDocuments for OpenDocumentTexts {
    fn text(&self, path: &Path) -> Option<Arc<str>> {
        self.texts.get(path).map(Arc::clone)
    }

    fn analysis(&self, path: &Path) -> Option<Arc<AnalysisResult>> {
        self.analyses.get(path).map(Arc::clone)
    }
}

impl Backend {
    /// The iRulesLX method word under `pos`, with what it resolves to.
    ///
    /// The dialect gate is the registry itself: `ILX::call` / `ILX::notify` are
    /// iRules-surface specs carrying the remote-method descriptors, so a
    /// document analysed against any other dialect's registry has no such
    /// command, finds no site, and answers `None` here. An ordinary Tcl file
    /// that defines its own `ILX::call` is therefore untouched.
    pub(super) async fn ilx_method_at(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        path: &Path,
        pos: Position,
    ) -> DocumentSourceResult<Option<(core_ilx::MethodCall, core_ilx::IlxTarget)>> {
        let analysis = self
            .analysis_for(uri, doc.text.clone(), doc.dialect.clone())
            .await?;
        let registry = (match analysis.resolved_registry() {
            Some(__source_value) => __source_value,
            None => return Ok(None),
        })
        .snapshot()
        .shared_registry();
        let open = self.open_document_texts().await;
        let plugins = self.ilx_plugin_roots(uri).await;
        let document = core_ilx::IlxDocument {
            path,
            text: &doc.text,
        };
        let ctx = core_ilx::IlxContext::new(&registry, self.store.as_ref())
            .with_analysis(&analysis)
            .with_open_documents(&open)
            .with_plugins(&plugins);
        let Some(call) = core_ilx::method_call_at(document, ctx, pos.line, pos.character) else {
            return Ok(None);
        };
        let target = core_ilx::definition(document, ctx, &call);
        Ok(Some((call, target)))
    }

    fn finish_ilx_reference_locations(
        found: &mut Vec<core_ilx::IlxLocation>,
        include_decl: bool,
    ) -> Vec<Location> {
        found.sort_by(|left, right| {
            (
                &left.path,
                left.range.start_line,
                left.range.start_character,
            )
                .cmp(&(
                    &right.path,
                    right.range.start_line,
                    right.range.start_character,
                ))
        });
        found.dedup();
        found
            .iter()
            .filter(|location| include_decl || location.kind != core_ilx::IlxSite::Registration)
            .filter_map(Self::ilx_location)
            .collect()
    }

    /// Find-references for the iRulesLX method relation, or `None` when the
    /// cursor is not on one of its two ends.
    ///
    /// The JavaScript end is gated on the document actually being an ILX
    /// extension entry point — `…/extensions/<name>/index.js`, or whatever its
    /// `package.json` names as `main` — so an ordinary `.js` file anywhere
    /// else in a project is never even scanned.
    ///
    /// `include_decl` is the request's own `includeDeclaration`: the
    /// `addMethod` registration is this symbol's declaration, so a client
    /// asking for uses only does not get it — including when the request came
    /// *from* the registration.
    pub(super) async fn ilx_references(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        pos: Position,
        include_decl: bool,
    ) -> DocumentSourceResult<Option<Vec<Location>>> {
        let path = (match uri.document_path() {
            Some(__source_value) => __source_value,
            None => return Ok(None),
        })
        .into_owned();
        let document = core_ilx::IlxDocument {
            path: &path,
            text: &doc.text,
        };
        let store = self.store.as_ref();
        let mut open = self.open_document_texts().await;
        // Which end of the relation the cursor is on decides which dialect's
        // registry reads the *rule* files — and which declarations apply — so
        // the file gate is asked first; it needs no registry at all.
        let js_end = core_ilx::is_extension_entry(
            &path,
            core_ilx::IlxFiles::new(store).with_open_documents(&open),
        );
        // A Tcl document belongs to a workspace folder, so its own folder's
        // declarations are the ones that describe it. A JavaScript entry point
        // is matched by *workspace path* instead, so which folder declared it
        // is irrelevant — see `all_ilx_plugin_roots`.
        let plugins = if js_end {
            self.all_ilx_plugin_roots().await
        } else {
            self.ilx_plugin_roots(uri).await
        };
        let analysis = if js_end {
            None
        } else {
            Some(
                self.analysis_for(uri, doc.text.clone(), doc.dialect.clone())
                    .await?,
            )
        };
        let registry = if let Some(analysis) = &analysis {
            (match analysis.resolved_registry() {
                Some(__source_value) => __source_value,
                None => return Ok(None),
            })
            .snapshot()
            .shared_registry()
        } else {
            // This explicit JavaScript/source-only route has no Tcl document
            // input or evaluated handle; identity references do not infer one.
            self.registry_for_dialect(core_ilx::EXTENSION_RULE_DIALECT)
                .await
        };
        // First the anchor under the cursor and the rule files its relation
        // reads; then each of those files' own input, so a sibling is scanned
        // under its own analysis rather than reported as having no sites.
        let (registration, call, siblings) = {
            let ctx = core_ilx::IlxContext {
                registry: &registry,
                analysis: analysis.as_deref(),
                files: core_ilx::IlxFiles::new(store).with_open_documents(&open),
                plugins: &plugins,
            };
            if js_end {
                let Some(registration) =
                    core_ilx::registration_at(document, pos.line, pos.character)
                else {
                    return Ok(None);
                };
                let siblings = core_ilx::sibling_rule_files_from_registration(&path, ctx);
                (Some(registration), None, siblings)
            } else {
                let Some(call) = core_ilx::method_call_at(document, ctx, pos.line, pos.character)
                else {
                    return Ok(None);
                };
                let siblings = core_ilx::sibling_rule_files(document, ctx, &call);
                (None, Some(call), siblings)
            }
        };
        self.supply_sibling_rule_inputs(&mut open, siblings).await;
        let ctx = core_ilx::IlxContext {
            registry: &registry,
            analysis: analysis.as_deref(),
            files: core_ilx::IlxFiles::new(store).with_open_documents(&open),
            plugins: &plugins,
        };
        let mut found = match (registration, call) {
            (Some(registration), _) => {
                let mut locations =
                    core_ilx::references_from_registration(document, ctx, &registration);
                locations.extend(
                    core_ilx::source_candidates_from_registration(document, ctx, &registration)
                        .into_iter()
                        .map(|candidate| candidate.location),
                );
                locations
            }
            (None, Some(call)) => {
                let mut locations = core_ilx::references(document, ctx, &call);
                locations.extend(
                    core_ilx::source_candidates(document, ctx, &call)
                        .into_iter()
                        .map(|candidate| candidate.location),
                );
                locations
            }
            (None, None) => return Ok(None),
        };
        // LSP locations are readonly source navigation; candidate joins never
        // populate an evaluated target or authorise an edit.
        Ok(Some(Self::finish_ilx_reference_locations(
            &mut found,
            include_decl,
        )))
    }

    /// `tcl-lsp.ilxReferences` — find-references from a point inside an ILX
    /// extension's **JavaScript** entry point.
    ///
    /// A command rather than a `textDocument/references` route because a `.js`
    /// file is not a Tcl document and must not become one: routing it to this
    /// server would put JavaScript through the Tcl analyser, the workspace
    /// index and the diagnostics pipeline, none of which have anything true to
    /// say about it. The `VS Code` client instead registers a *second*
    /// `ReferenceProvider` for `javascript` that calls this and contributes its
    /// answers alongside the JavaScript language service's own — so the editor
    /// shows the Tcl call sites without either provider displacing the other.
    ///
    /// The caller passes the buffer's current `text`, since this server holds
    /// no copy of a document it does not own; an unsaved `addMethod` is
    /// therefore still what navigation sees, exactly as on the Tcl side.
    ///
    /// The extension-entry gate is applied here as well as in the client, so a
    /// request naming an ordinary `.js` file answers nothing rather than
    /// scanning it.
    ///
    /// `{ uri, text, line, character, includeDeclaration? }` →
    /// `Location[]`, or `null` when the position is not on a registration.
    pub(in super::super) async fn ilx_references_command(
        &self,
        args: &[serde_json::Value],
    ) -> Option<serde_json::Value> {
        let request = args.first().and_then(serde_json::Value::as_object)?;
        let uri = Uri::from_str(request.get("uri").and_then(serde_json::Value::as_str)?).ok()?;
        let path = uri.document_path()?.into_owned();
        let text = request.get("text").and_then(serde_json::Value::as_str)?;
        let line = u32::try_from(request.get("line").and_then(serde_json::Value::as_u64)?).ok()?;
        let character = u32::try_from(
            request
                .get("character")
                .and_then(serde_json::Value::as_u64)?,
        )
        .ok()?;
        let include_decl = request
            .get("includeDeclaration")
            .and_then(serde_json::Value::as_bool)
            .unwrap_or(true);

        let mut open = self.open_document_texts().await;
        if !core_ilx::is_extension_entry(
            &path,
            core_ilx::IlxFiles::new(self.store.as_ref()).with_open_documents(&open),
        ) {
            return None;
        }
        // Every declaration in the session, not just the ones the folder
        // holding this `.js` made — see `all_ilx_plugin_roots`.
        let plugins = self.all_ilx_plugin_roots().await;
        // JavaScript has no Tcl dialect of its own, so the rule files it is
        // matched against are read with the dialect that owns the ILX surface.
        let registry = self
            .registry_for_dialect(core_ilx::EXTENSION_RULE_DIALECT)
            .await;
        let document = core_ilx::IlxDocument { path: &path, text };
        let registration = core_ilx::registration_at(document, line, character)?;
        let siblings = core_ilx::sibling_rule_files_from_registration(
            &path,
            core_ilx::IlxContext {
                registry: &registry,
                analysis: None,
                files: core_ilx::IlxFiles::new(self.store.as_ref()).with_open_documents(&open),
                plugins: &plugins,
            },
        );
        self.supply_sibling_rule_inputs(&mut open, siblings).await;
        let ctx = core_ilx::IlxContext {
            registry: &registry,
            analysis: None,
            files: core_ilx::IlxFiles::new(self.store.as_ref()).with_open_documents(&open),
            plugins: &plugins,
        };
        let mut found = core_ilx::references_from_registration(document, ctx, &registration);
        found.extend(
            core_ilx::source_candidates_from_registration(document, ctx, &registration)
                .into_iter()
                .map(|candidate| candidate.location),
        );
        found.sort_by(|left, right| {
            (
                &left.path,
                left.range.start_line,
                left.range.start_character,
            )
                .cmp(&(
                    &right.path,
                    right.range.start_line,
                    right.range.start_character,
                ))
        });
        found.dedup();
        let locations: Vec<Location> = found
            .iter()
            .filter(|location| include_decl || location.kind != core_ilx::IlxSite::Registration)
            .filter_map(Self::ilx_location)
            .collect();
        serde_json::to_value(locations).ok()
    }

    /// Go-to-definition for an iRulesLX method word, or `None` when the cursor
    /// is not on one.
    ///
    /// `Some(vec![])` — an ILX method word that resolved to nothing — is a
    /// *definitive* answer, not a fall-through. The cursor sits on an argument
    /// of `ILX::call` / `ILX::notify`, which no other tier has a real answer
    /// for, so letting an unresolved one continue would turn a same-named
    /// sibling proc into a false-positive jump: the opposite of the abstention
    /// this relation is built on.
    pub(super) async fn ilx_definition(
        &self,
        uri: &Uri,
        doc: &DocumentState,
        pos: Position,
    ) -> DocumentSourceResult<Option<Vec<Location>>> {
        let path = (match uri.document_path() {
            Some(__source_value) => __source_value,
            None => return Ok(None),
        })
        .into_owned();
        let Some((_, target)) = self.ilx_method_at(uri, doc, &path, pos).await? else {
            return Ok(None);
        };
        Ok(Some(match target {
            core_ilx::IlxTarget::Resolved(location) => {
                Self::ilx_location(&location).into_iter().collect()
            }
            core_ilx::IlxTarget::SourceCandidate(candidate) => {
                Self::ilx_location(&candidate.location)
                    .into_iter()
                    .collect()
            }
            core_ilx::IlxTarget::Ambiguous { .. } | core_ilx::IlxTarget::Unresolved(_) => {
                Vec::new()
            }
        }))
    }

    /// The text the editor currently holds for every open document, by path.
    ///
    /// The cross-file half of the ILX relation reads *other* files — sibling
    /// rules, and the extension's JavaScript — and must see the same bytes
    /// [`Backend::read_document`] would: the open buffer when there is one,
    /// the file on disk otherwise. Without this, find-references over a rule
    /// with unsaved edits would report the text last written to disk.
    ///
    /// A snapshot rather than a per-file `read_document` round trip because
    /// the core walk is synchronous and discovers the files itself; the map is
    /// one `Arc` clone per open document, and the normalisation is the same
    /// `normalise_lone_cr` pass `read_document` applies (skipped outright for
    /// the documents that need none, which is all of them on LF and CRLF).
    /// Every open document's text, and — for each open iRules document — its
    /// own analysis through [`Self::analysis_for`], so an ILX sibling rule is
    /// scanned under its own source, input and configuration rather than the
    /// requesting document's. An analysis that cannot be had is left out; the
    /// scanner then has no facts for that sibling.
    async fn open_document_texts(&self) -> OpenDocumentTexts {
        let mut rules = Vec::new();
        let mut texts = HashMap::new();
        {
            let docs = self.documents.lock("open_document_texts").await;
            texts.reserve(docs.len());
            for (uri, doc) in docs.iter() {
                let Some(path) = uri.document_path() else {
                    continue;
                };
                let path = path.into_owned();
                let text: Arc<str> = if doc.has_bare_cr {
                    tcl_lexer::normalise_lone_cr(&doc.text).into_owned().into()
                } else {
                    Arc::clone(&doc.text)
                };
                if tcl_lsp_core::profile_for_dialect(&doc.dialect).is_irules() {
                    rules.push((
                        uri.clone(),
                        path.clone(),
                        Arc::clone(&text),
                        doc.dialect.clone(),
                    ));
                }
                texts.insert(path, text);
            }
        }
        let mut analyses = HashMap::with_capacity(rules.len());
        for (uri, path, text, dialect) in rules {
            if let Ok(analysis) = self.analysis_for(&uri, text, dialect).await {
                analyses.insert(path, analysis);
            }
        }
        OpenDocumentTexts { texts, analyses }
    }

    /// Give each ILX sibling rule file in `paths` that has no analysis in
    /// `open` its own: the document read through [`Self::read_document`] (the
    /// open buffer, or the closed file from disk with its closed-file dialect)
    /// and analysed through [`Self::analysis_for`] under that file's own
    /// input and configuration. The text recorded is exactly the text
    /// analysed, so the scanner reads the bytes its analysis describes. A file
    /// that cannot be read or analysed is left as it was — the core then has
    /// no facts for it and borrows none.
    async fn supply_sibling_rule_inputs(&self, open: &mut OpenDocumentTexts, paths: Vec<PathBuf>) {
        for path in paths {
            if open.analyses.contains_key(&path) {
                continue;
            }
            let Some(uri) = canonical_file_uri(&path) else {
                continue;
            };
            let Ok(Some(doc)) = self.read_document(&uri).await else {
                continue;
            };
            let text = Arc::clone(&doc.text);
            if let Ok(analysis) = self
                .analysis_for(&uri, Arc::clone(&text), doc.dialect.clone())
                .await
            {
                open.texts.insert(path.clone(), text);
                open.analyses.insert(path, analysis);
            }
        }
    }

    /// Lift an ILX location — a path plus a range — onto the wire.
    fn ilx_location(location: &core_ilx::IlxLocation) -> Option<Location> {
        Some(Location {
            uri: canonical_file_uri(&location.path)?,
            range: lift_lsp_range(location.range),
        })
    }

    /// The iRulesLX plugin associations declared for `uri`'s workspace folder,
    /// with every path resolved against that folder's root.
    ///
    /// Folder-scoped for the same reason `entry_points` is: a relative path
    /// here means "under this folder", and resolving it against every root in
    /// a multi-root session would invent workspaces the user never declared.
    /// A no-folder session — a single loose file — has none, and the
    /// directory-name convention is then the whole story.
    /// Every folder's declared iRulesLX plugin associations, resolved.
    ///
    /// The session-wide view `tcl-lsp.getEffectiveConfig` reports, so a client
    /// (and the e2e config barrier) can see *that* a declaration has been
    /// applied and *where* it resolved to, without having to name a document
    /// inside the folder that carries it.
    ///
    /// It is also what the **JavaScript** end of the ILX relation resolves
    /// against, rather than [`Self::ilx_plugin_roots`]. A declaration may point
    /// outside the folder that made it (`prod = ../shared/ws`, or an absolute
    /// path), and then the entry point is not under that folder at all — so a
    /// folder-scoped lookup keyed on the `.js` URI would find nothing and fall
    /// back to the workspace's *directory name*, which is exactly the name the
    /// declaration exists to correct. Reverse references would then find no
    /// callers even though Tcl-to-JavaScript navigation through the same
    /// mapping works.
    ///
    /// Widening cannot mismatch here, because the JavaScript end selects by
    /// resolved **workspace path**: a declaration only applies when its
    /// workspace *is* this entry point's. The Tcl end keeps the folder-scoped
    /// lookup, where a session-wide one would make two folders that each
    /// declare the same plugin name for their own workspace ambiguous for
    /// both.
    /// [`Self::all_ilx_plugin_roots`] as the JSON `tcl-lsp.getEffectiveConfig`
    /// reports: `[{plugin, workspace, rules}]`, paths absolute.
    pub(in super::super) async fn reported_ilx_plugin_roots(&self) -> Vec<serde_json::Value> {
        self.all_ilx_plugin_roots()
            .await
            .into_iter()
            .map(|root| {
                serde_json::json!({
                    "plugin": root.plugin,
                    "workspace": root.workspace.to_string_lossy(),
                    "rules": root
                        .extra_rule_dirs
                        .iter()
                        .map(|dir| dir.to_string_lossy().into_owned())
                        .collect::<Vec<String>>(),
                })
            })
            .collect()
    }

    async fn all_ilx_plugin_roots(&self) -> Vec<core_ilx::IlxPluginRoot> {
        let configs = self.folder_configs.lock().await;
        let mut out: Vec<core_ilx::IlxPluginRoot> = Vec::new();
        for (folder, fc) in configs.iter() {
            let Some(root) = folder.document_path().map(std::borrow::Cow::into_owned) else {
                continue;
            };
            out.extend(resolve_ilx_specs(&fc.iruleslx, &root));
        }
        out.sort_by(|a, b| (&a.plugin, &a.workspace).cmp(&(&b.plugin, &b.workspace)));
        out.dedup();
        out
    }

    pub(in super::super) async fn ilx_plugin_roots(
        &self,
        uri: &Uri,
    ) -> Vec<core_ilx::IlxPluginRoot> {
        let configs = self.folder_configs.lock().await;
        let mut best: Option<&(Uri, FolderConfig)> = None;
        for entry in configs.iter() {
            if uri_under_folder(uri.as_str(), entry.0.as_str())
                && best.is_none_or(|b| entry.0.as_str().len() > b.0.as_str().len())
            {
                best = Some(entry);
            }
        }
        let Some((folder, fc)) = best else {
            return Vec::new();
        };
        if fc.iruleslx.is_empty() {
            return Vec::new();
        }
        let Some(root) = folder.document_path().map(std::borrow::Cow::into_owned) else {
            return Vec::new();
        };
        resolve_ilx_specs(&fc.iruleslx, &root)
    }
}
