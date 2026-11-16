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

//! BIG-IP object graphs: linked objects, cleanup and partition rename.

use super::{
    Backend, DocumentChanges, DocumentState, FromStr, HashMap, HashSet, OneOf,
    OptionalVersionedTextDocumentIdentifier, Position, Range, TextDocumentEdit, TextEdit, Uri,
    WorkspaceEdit, jsonrpc,
};

impl Backend {
    /// Handle `tcl-lsp.extractLinkedObjects`: the BIG-IP object at the cursor
    /// and everything it references, out to a depth.
    ///
    /// Arguments are `[uri, cursor, maxDepth, maxNodes, extraCursors?]`, where
    /// `extraCursors` is a list of `[uri, cursor]` pairs for a multi-cursor
    /// selection and each cursor is a legacy byte offset or a versioned LSP
    /// position ([`Self::source_position_offset`]). The walk starts at every
    /// root the cursors land on, so the result answers "these objects and
    /// what they depend on" rather than being run once per cursor.
    ///
    /// `sources` in the reply names each document and the version the graph
    /// was read from, so a client navigating to a node can tell whether its
    /// range still describes the buffer. A positioned cursor that no longer
    /// matches its document answers `{unavailable: reason}`.
    ///
    /// Breadth-first, so a node is reported at its shortest distance from a
    /// root. `maxNodes` stops the walk rather than truncating the answer
    /// afterwards, which keeps the edges consistent with the nodes: an edge is
    /// only emitted when both of its ends are in the result.
    ///
    pub(in super::super) async fn extract_linked_objects_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(primary_uri) = args.first().and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let Some(primary_cursor) = args
            .get(1)
            .filter(|cursor| cursor.is_u64() || cursor.is_object())
        else {
            return Ok(None);
        };
        let max_depth = args
            .get(2)
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(5)
            .min(64);
        let max_nodes = usize::try_from(
            args.get(3)
                .and_then(serde_json::Value::as_u64)
                .unwrap_or(400),
        )
        .unwrap_or(400)
        .max(1);

        // Every (uri, cursor) the client asked about: the primary cursor plus
        // any additional ones.
        let mut requested: Vec<(String, &serde_json::Value)> =
            vec![(primary_uri.to_owned(), primary_cursor)];
        if let Some(extra) = args.get(4).and_then(serde_json::Value::as_array) {
            for pair in extra {
                let (Some(u), Some(cursor)) = (
                    pair.get(0).and_then(serde_json::Value::as_str),
                    pair.get(1)
                        .filter(|cursor| cursor.is_u64() || cursor.is_object()),
                ) else {
                    continue;
                };
                requested.push((u.to_owned(), cursor));
            }
        }

        // Read each distinct document once, and resolve each cursor against
        // the document it names.
        let mut documents: Vec<(String, DocumentState)> = Vec::new();
        for (uri_str, _) in &requested {
            if documents.iter().any(|(u, _)| u == uri_str) {
                continue;
            }
            let Ok(uri) = Uri::from_str(uri_str) else {
                continue;
            };
            if let Some(doc) = self.read_local_document(&uri).await? {
                documents.push((uri_str.clone(), doc));
            }
        }
        if documents.is_empty() {
            return Ok(None);
        }
        let mut cursors: Vec<(String, usize)> = Vec::new();
        for (uri_str, cursor) in &requested {
            let Some((_, doc)) = documents.iter().find(|(u, _)| u == uri_str) else {
                continue;
            };
            match Self::source_position_offset(doc, cursor) {
                Ok(offset) => cursors.push((uri_str.clone(), offset)),
                Err(reason) => return Ok(Some(serde_json::json!({ "unavailable": reason }))),
            }
        }
        let read: Vec<serde_json::Value> = documents
            .iter()
            .map(|(uri, doc)| serde_json::json!({ "uri": uri, "version": doc.version }))
            .collect();
        let sources: Vec<(String, String)> = documents
            .into_iter()
            .map(|(uri, doc)| (uri, doc.text.to_string()))
            .collect();

        // Parsing and the graph walk are pure CPU over the whole config, so
        // they run off the LSP event loop.
        let value = crate::rt::spawn_blocking(move || {
            Self::linked_objects_payload(&sources, &cursors, max_depth, max_nodes)
        })
        .await
        .unwrap_or(None)
        .map(|mut payload| {
            if let Some(object) = payload.as_object_mut() {
                object.insert("sources".to_owned(), serde_json::Value::Array(read));
            }
            payload
        });
        Ok(value)
    }

    /// One object as the clients model it, at the depth it was reached.
    ///
    /// `sourceOrigin` is always null: the clients read the field, but the
    /// graph does not track how a stanza reached the file, and inventing a
    /// value would be worse than saying nothing.
    fn linked_object_node(node: &tcl_bigip::graph::ObjectNode, depth: u64) -> serde_json::Value {
        serde_json::json!({
            "id": node.node_id,
            "uri": node.uri,
            "module": node.module,
            "objectType": node.object_type,
            "identifier": node.identifier,
            "kind": node.kind,
            "header": node.header,
            "depth": depth,
            "sourceOrigin": serde_json::Value::Null,
            "range": {
                "start": { "line": node.range.start.line, "character": node.range.start.character },
                "end": { "line": node.range.end.line, "character": node.range.end.character },
            },
        })
    }

    /// Breadth-first from every root, out to `max_depth` hops and at most
    /// `max_nodes` objects.
    ///
    /// Returns the nodes in the order they were reached and the depth each was
    /// first reached at. Stopping the walk on `max_nodes`, rather than
    /// truncating afterwards, is what lets the caller emit only edges whose
    /// both ends are present.
    fn walk_linked_objects<'a>(
        root_ids: &[&'a str],
        outgoing: &HashMap<&'a str, Vec<&'a tcl_bigip::graph::ObjectEdge>>,
        by_id: &HashMap<&'a str, &'a tcl_bigip::graph::ObjectNode>,
        max_depth: u64,
        max_nodes: usize,
    ) -> (Vec<&'a str>, HashMap<&'a str, u64>) {
        use std::collections::VecDeque;

        let mut depth_of: HashMap<&str, u64> = HashMap::new();
        let mut order: Vec<&str> = Vec::new();
        let mut queue: VecDeque<(&str, u64)> = VecDeque::new();
        for id in root_ids {
            if depth_of.contains_key(id) {
                continue;
            }
            depth_of.insert(id, 0);
            order.push(id);
            queue.push_back((id, 0));
        }
        while let Some((id, depth)) = queue.pop_front() {
            if depth >= max_depth || order.len() >= max_nodes {
                continue;
            }
            for edge in outgoing.get(id).into_iter().flatten() {
                let target = edge.target_id.as_str();
                if depth_of.contains_key(target) || !by_id.contains_key(target) {
                    continue;
                }
                if order.len() >= max_nodes {
                    break;
                }
                depth_of.insert(target, depth + 1);
                order.push(target);
                queue.push_back((target, depth + 1));
            }
        }
        (order, depth_of)
    }

    /// The graph walk behind `tcl-lsp.extractLinkedObjects`.
    fn linked_objects_payload(
        sources: &[(String, String)],
        cursors: &[(String, usize)],
        max_depth: u64,
        max_nodes: usize,
    ) -> Option<serde_json::Value> {
        let ctx = tcl_bigip::graph::GraphContext::new();
        let parsed: Vec<(String, tcl_bigip::parser::driver::BigipConfig)> = sources
            .iter()
            .map(|(uri, text)| {
                (
                    uri.clone(),
                    tcl_bigip::parser::driver::parse_bigip_conf(text, "Common"),
                )
            })
            .collect();
        let configs: Vec<(String, &tcl_bigip::parser::driver::BigipConfig)> =
            parsed.iter().map(|(u, c)| (u.clone(), c)).collect();
        let graph = tcl_bigip::graph::build_bigip_object_graph(sources, &configs, &ctx);

        let nodes: Vec<&tcl_bigip::graph::ObjectNode> = graph
            .nodes_by_uri
            .iter()
            .flat_map(|(_uri, ns)| ns.iter())
            .collect();

        // The stanza a cursor sits in. The innermost match wins, so a cursor
        // inside a nested block still names the object that contains it.
        let root_ids: Vec<&str> = cursors
            .iter()
            .filter_map(|(uri, offset)| {
                nodes
                    .iter()
                    .filter(|n| {
                        &n.uri == uri && *offset >= n.header_start_offset && *offset < n.end_offset
                    })
                    .min_by_key(|n| n.end_offset - n.header_start_offset)
                    .map(|n| n.node_id.as_str())
            })
            .collect();
        let first_root = *root_ids.first()?;

        let mut outgoing: HashMap<&str, Vec<&tcl_bigip::graph::ObjectEdge>> = HashMap::new();
        for edge in &graph.edges {
            outgoing
                .entry(edge.source_id.as_str())
                .or_default()
                .push(edge);
        }
        let by_id: HashMap<&str, &tcl_bigip::graph::ObjectNode> =
            nodes.iter().map(|n| (n.node_id.as_str(), *n)).collect();

        let (order, depth_of) =
            Self::walk_linked_objects(&root_ids, &outgoing, &by_id, max_depth, max_nodes);

        let node_values: Vec<serde_json::Value> = order
            .iter()
            .filter_map(|id| {
                by_id
                    .get(id)
                    .map(|n| Self::linked_object_node(n, depth_of.get(id).copied().unwrap_or(0)))
            })
            .collect();

        // Only edges whose both ends survived the walk, so the client never
        // draws an arrow to a node it was not given.
        let edge_values: Vec<serde_json::Value> = graph
            .edges
            .iter()
            .filter(|e| {
                depth_of.contains_key(e.source_id.as_str())
                    && depth_of.contains_key(e.target_id.as_str())
            })
            .map(|e| {
                serde_json::json!({
                    "source": e.source_id,
                    "target": e.target_id,
                    "viaProperty": e.via_property,
                    "viaKind": e.via_kind,
                })
            })
            .collect();

        let root = by_id.get(first_root)?;
        let roots: Vec<serde_json::Value> = root_ids
            .iter()
            .filter_map(|id| by_id.get(id))
            .map(|n| {
                serde_json::json!({
                    "id": n.node_id,
                    "uri": n.uri,
                    "header": n.header,
                })
            })
            .collect();

        Some(serde_json::json!({
            "root": root.node_id,
            "rootUri": root.uri,
            "rootHeader": root.header,
            "roots": roots,
            "maxDepth": max_depth,
            "maxNodes": max_nodes,
            "nodes": node_values,
            "edges": edge_values,
        }))
    }

    /// Handle `tcl-lsp.bigipCleanup`: the objects no virtual server or wide-IP
    /// reaches, and a tmsh script that deletes them.
    ///
    /// Arguments are `[uris, keepPaths?]`. `keepPaths` spares objects by
    /// full path even when nothing references them. Both editor hosts send a
    /// third argument, always `false`, which neither gives a meaning; it is
    /// accepted and ignored rather than guessed at.
    ///
    /// The candidates come back in delete order — a referenced object after
    /// the thing referencing it — so the script runs top to bottom without
    /// tripping over tmsh's own dependency checks.
    pub(in super::super) async fn bigip_cleanup_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(uris) = args.first().and_then(serde_json::Value::as_array) else {
            return Ok(None);
        };
        let keep_paths: HashSet<String> = args
            .get(1)
            .and_then(serde_json::Value::as_array)
            .map_or_default(|a| {
                a.iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                    .collect()
            });

        let mut sources: Vec<(String, String)> = Vec::new();
        for value in uris {
            let Some(uri_str) = value.as_str() else {
                continue;
            };
            let Ok(uri) = Uri::from_str(uri_str) else {
                continue;
            };
            if let Some(doc) = self.read_local_document(&uri).await? {
                sources.push((uri_str.to_owned(), doc.text.to_string()));
            }
        }
        if sources.is_empty() {
            return Ok(None);
        }

        let value = crate::rt::spawn_blocking(move || {
            let ctx = tcl_bigip::graph::GraphContext::new();
            let parsed: Vec<(String, tcl_bigip::parser::driver::BigipConfig)> = sources
                .iter()
                .map(|(uri, text)| {
                    (
                        uri.clone(),
                        tcl_bigip::parser::driver::parse_bigip_conf(text, "Common"),
                    )
                })
                .collect();
            let configs: Vec<(String, &tcl_bigip::parser::driver::BigipConfig)> =
                parsed.iter().map(|(u, c)| (u.clone(), c)).collect();
            let graph = tcl_bigip::graph::build_bigip_object_graph(&sources, &configs, &ctx);

            let mut uri_list: Vec<String> = sources.iter().map(|(u, _)| u.clone()).collect();
            uri_list.sort();
            let report = tcl_bigip::cleanup::compute_cleanup(&graph, &uri_list, &keep_paths, &[]);
            // The crate already renders this report in the shape the clients
            // read, so it is parsed rather than rebuilt field by field.
            serde_json::from_str::<serde_json::Value>(&tcl_bigip::cleanup::report_to_json(&report))
                .ok()
        })
        .await
        .unwrap_or(None);
        Ok(value)
    }

    /// Handle `tcl-lsp.renamePartition`: rename a BIG-IP partition and every
    /// path that names it.
    ///
    /// Arguments are `[uri, currentName, newName, version?]`, bare partition
    /// names rather than paths. The reply is `{success, edit?, error?}`: the
    /// client applies the edit itself, so it can undo the rename as one step.
    /// With `version` — the document version the client read the stanza
    /// from — a different current version is refused, and the edit is a
    /// versioned document change the client applies only to that version.
    ///
    /// The rewrite is the query engine's `rename_partition`, which owns the
    /// cascade — the `auth partition` stanza and every `/<old>/` prefix
    /// through the whole file. Doing it here with a search and replace would
    /// be a second implementation of that cascade, and the wrong one.
    ///
    /// The result is re-parsed before it is offered. A rename that produced a
    /// file the parser no longer reads is refused rather than handed to the
    /// editor, because the client applies it without review.
    pub(in super::super) async fn rename_partition_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let (Some(uri_str), Some(current), Some(new_name)) = (
            args.first().and_then(serde_json::Value::as_str),
            args.get(1).and_then(serde_json::Value::as_str),
            args.get(2).and_then(serde_json::Value::as_str),
        ) else {
            return Ok(Some(
                serde_json::json!({ "success": false, "error": "expected [uri, currentName, newName]" }),
            ));
        };
        let Ok(uri) = Uri::from_str(uri_str) else {
            return Ok(Some(
                serde_json::json!({ "success": false, "error": "unreadable document URI" }),
            ));
        };
        let Some(doc) = self.read_local_document(&uri).await? else {
            return Ok(Some(
                serde_json::json!({ "success": false, "error": "document is not open" }),
            ));
        };
        let expected_version = match args.get(3) {
            None | Some(serde_json::Value::Null) => None,
            Some(value) => {
                let Some(version) = value.as_i64().and_then(|v| i32::try_from(v).ok()) else {
                    return Ok(Some(
                        serde_json::json!({ "success": false, "error": "unreadable document version" }),
                    ));
                };
                if doc.version != Some(version) {
                    return Ok(Some(serde_json::json!({
                        "success": false,
                        "error": "the document changed since the partition was read; try again",
                    })));
                }
                Some(version)
            }
        };

        let text = doc.text.to_string();
        let uri_owned = uri_str.to_owned();
        let (current, new_name) = (current.to_owned(), new_name.to_owned());
        let outcome = crate::rt::spawn_blocking(move || {
            Self::rename_partition_outcome(&uri_owned, &text, &current, &new_name)
        })
        .await
        .unwrap_or_else(|_| Err("the rename did not complete".to_owned()));

        Ok(Some(match outcome {
            Ok(new_source) => {
                let index = tcl_lexer::LineIndex::new_lsp(&doc.text);
                let last = index.position_at_utf16(
                    u32::try_from(doc.text.len()).unwrap_or(u32::MAX),
                    &doc.text,
                );
                let edit = TextEdit {
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: last.line,
                            character: last.character.get(),
                        },
                    },
                    new_text: new_source,
                };
                let workspace_edit = if expected_version.is_some() {
                    WorkspaceEdit {
                        document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
                            text_document: OptionalVersionedTextDocumentIdentifier {
                                uri,
                                version: expected_version,
                            },
                            edits: vec![OneOf::Left(edit)],
                        }])),
                        ..WorkspaceEdit::default()
                    }
                } else {
                    let mut changes = HashMap::new();
                    changes.insert(uri, vec![edit]);
                    WorkspaceEdit {
                        changes: Some(changes),
                        ..WorkspaceEdit::default()
                    }
                };
                serde_json::json!({
                    "success": true,
                    "edit": serde_json::to_value(workspace_edit).unwrap_or(serde_json::Value::Null),
                })
            }
            Err(message) => serde_json::json!({ "success": false, "error": message }),
        }))
    }

    /// Run the partition rename and return the rewritten source.
    ///
    /// The name rules are the engine's: it refuses an empty name, a path
    /// rather than a bare name, and `/Common` in either direction. Repeating
    /// those checks here would be a second rulebook to keep in step, so the
    /// engine's error is passed through as the message the client shows.
    fn rename_partition_outcome(
        uri: &str,
        source: &str,
        current: &str,
        new_name: &str,
    ) -> Result<String, String> {
        // The engine quotes its own arguments; the names still must not carry
        // a quote that would end the string early.
        if current.contains('"') || new_name.contains('"') {
            return Err("partition names must not contain quotes".to_owned());
        }
        let query = format!(r#"rename_partition("{current}", "{new_name}")"#);
        let sources = vec![(uri.to_owned(), source.to_owned())];
        let options = tcl_bigip_query::runner::QueryOptions::default();

        let result = tcl_bigip_query::runner::run_query(&query, &sources, &options)
            .map_err(|e| e.to_string())?;
        let Some((_, applied)) = result.edits_per_file.into_iter().next() else {
            return Err(format!("no partition named '{current}' in this file"));
        };
        if applied.new_source == source {
            return Err(format!("no partition named '{current}' in this file"));
        }

        // A rewrite the parser can no longer read is a corrupt file, and the
        // client applies this edit without review.
        let reparsed = tcl_bigip::parser::driver::parse_bigip_conf(&applied.new_source, "Common");
        let original = tcl_bigip::parser::driver::parse_bigip_conf(source, "Common");
        if reparsed.objects.len() != original.objects.len() {
            return Err("the rename changed how the file parses and was not applied".to_owned());
        }
        Ok(applied.new_source)
    }
}
