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

//! iRules embedded in a BIG-IP configuration: list, extract and write back.

use super::{
    Backend, DocumentChanges, DocumentState, FromStr, HashMap, OneOf,
    OptionalVersionedTextDocumentIdentifier, Position, Range, TextDocumentEdit, TextEdit, Uri,
    WorkspaceEdit, jsonrpc,
};

impl Backend {
    /// One `ltm rule` / `gtm rule` stanza, as the editors model it.
    ///
    /// The offsets are byte offsets into the configuration document as the
    /// server read it at `documentVersion` (null when the server holds no
    /// client version). The whole object is the receipt a client returns to
    /// `tcl-lsp.writeRuleBack`: the server rejoins the stanza by its header,
    /// path and original body there, so the client never does offset
    /// arithmetic of its own.
    fn rule_info(
        rule: &tcl_bigip::rule_extract::EmbeddedRule,
        uri: &str,
        version: Option<i32>,
    ) -> serde_json::Value {
        serde_json::json!({
            "name": rule.name,
            "fullPath": rule.full_path,
            "header": rule.header,
            "body": rule.body,
            "bodyStartOffset": rule.body_start_offset,
            "bodyEndOffset": rule.body_end_offset,
            "uri": uri,
            "documentVersion": version,
            "blockStartLine": rule.range.start.line,
        })
    }

    /// The byte offset a client's cursor names in `doc`.
    ///
    /// A bare number is the legacy byte offset, kept for existing callers. An
    /// object `{position: {line, character}, version}` is an LSP position —
    /// UTF-16 columns, as an editor counts them — valid only against the
    /// document version it was read from; `Err` names why it is not.
    pub(super) fn source_position_offset(
        doc: &DocumentState,
        value: &serde_json::Value,
    ) -> Result<usize, &'static str> {
        if let Some(offset) = value.as_u64() {
            return Ok(usize::try_from(offset).unwrap_or(usize::MAX));
        }
        let position = value.get("position").ok_or("request")?;
        let (Some(line), Some(character), Some(version)) = (
            position
                .get("line")
                .and_then(serde_json::Value::as_u64)
                .and_then(|line| u32::try_from(line).ok()),
            position
                .get("character")
                .and_then(serde_json::Value::as_u64)
                .and_then(|character| u32::try_from(character).ok()),
            value
                .get("version")
                .and_then(serde_json::Value::as_i64)
                .and_then(|version| i32::try_from(version).ok()),
        ) else {
            return Err("request");
        };
        if doc.version != Some(version) {
            return Err("stale");
        }
        if usize::try_from(line).unwrap_or(usize::MAX) >= doc.line_index.line_count() {
            return Err("request");
        }
        let offset =
            doc.line_index
                .offset_at_utf16(line, tcl_lexer::Utf16Col::new(character), &doc.text);
        Ok(usize::try_from(offset).unwrap_or(usize::MAX))
    }

    /// Handle `tcl-lsp.listRules`: every iRule embedded in a BIG-IP
    /// configuration document.
    ///
    /// Arguments are `[uri]`. A document with no `ltm rule` or `gtm rule`
    /// stanza yields an empty list rather than null: the clients tell those
    /// apart, showing "no rules in this file" for the first and a warning for
    /// a document they could not read.
    pub(in super::super) async fn list_rules_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(uri_str) = args.first().and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let Ok(uri) = Uri::from_str(uri_str) else {
            return Ok(None);
        };
        let Some(doc) = self.read_local_document(&uri).await? else {
            return Ok(None);
        };
        let rules = tcl_bigip::rule_extract::find_embedded_rules(&doc.text);
        let uri_owned = uri_str.to_owned();
        Ok(Some(serde_json::Value::Array(
            rules
                .iter()
                .map(|r| Self::rule_info(r, &uri_owned, doc.version))
                .collect(),
        )))
    }

    /// Handle `tcl-lsp.extractRule`: the iRule containing a cursor.
    ///
    /// Arguments are `[uri, cursor]`, where `cursor` is a legacy byte offset
    /// or a versioned LSP position ([`Self::source_position_offset`]). Null
    /// when the cursor is outside every rule stanza, which is what the clients
    /// render as "cursor is not inside an ltm rule or gtm rule block";
    /// `{unavailable: reason}` when a position cannot be read against the
    /// current document.
    pub(in super::super) async fn extract_rule_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let Some(uri_str) = args.first().and_then(serde_json::Value::as_str) else {
            return Ok(None);
        };
        let Ok(uri) = Uri::from_str(uri_str) else {
            return Ok(None);
        };
        let Some(cursor) = args.get(1) else {
            return Ok(None);
        };
        let Some(doc) = self.read_local_document(&uri).await? else {
            return Ok(None);
        };
        let offset = match Self::source_position_offset(&doc, cursor) {
            Ok(offset) => offset,
            Err(reason) => return Ok(Some(serde_json::json!({ "unavailable": reason }))),
        };
        Ok(
            tcl_bigip::rule_extract::find_rule_at_offset(&doc.text, offset)
                .map(|rule| Self::rule_info(&rule, uri_str, doc.version)),
        )
    }

    /// Handle `tcl-lsp.writeRuleBack`: put an edited rule body back into its
    /// configuration document.
    ///
    /// Arguments are `[rule, newBody]`, where `rule` is the object
    /// `tcl-lsp.extractRule` / `tcl-lsp.listRules` returned. The stanza is
    /// rejoined in the current document by its header, path and original
    /// body — wherever earlier edits have moved it — so a concurrent change
    /// to the rule refuses rather than overwriting it; the replacement must
    /// parse back as the same rule; and the edit names the document version
    /// it was computed against, so the client rejects it once that is stale.
    /// The reply is `{applied, rule?, reason?}`: on success `rule` is the
    /// stanza as it stands after the edit, the receipt for the next write.
    ///
    /// The legacy `[uri, bodyStart, bodyEnd, newBody]` form, whose span is
    /// the client's own, still answers the bare boolean it always did.
    ///
    /// The edit is applied through `workspace/applyEdit` rather than returned,
    /// because the caller is a save handler with nothing to apply an edit to:
    /// the document being changed is the configuration file, not the scratch
    /// buffer that was saved.
    pub(in super::super) async fn write_rule_back_command(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        if args.first().is_some_and(serde_json::Value::is_object) {
            return self.write_rule_back_receipt(args).await;
        }
        let Some(uri_str) = args.first().and_then(serde_json::Value::as_str) else {
            return Ok(Some(serde_json::json!(false)));
        };
        let Ok(uri) = Uri::from_str(uri_str) else {
            return Ok(Some(serde_json::json!(false)));
        };
        let (Some(start), Some(end), Some(body)) = (
            args.get(1).and_then(serde_json::Value::as_u64),
            args.get(2).and_then(serde_json::Value::as_u64),
            args.get(3).and_then(serde_json::Value::as_str),
        ) else {
            return Ok(Some(serde_json::json!(false)));
        };
        let Some(doc) = self.read_local_document(&uri).await? else {
            return Ok(Some(serde_json::json!(false)));
        };

        let (start, end) = (
            usize::try_from(start).unwrap_or(usize::MAX),
            usize::try_from(end).unwrap_or(usize::MAX),
        );
        // A span the document cannot honour is refused rather than clamped:
        // writing a rule body into the wrong range corrupts the file, and the
        // client's warning is the honest outcome.
        if start > end
            || end > doc.text.len()
            || !doc.text.is_char_boundary(start)
            || !doc.text.is_char_boundary(end)
        {
            return Ok(Some(serde_json::json!(false)));
        }
        let applied = self
            .apply_rule_body_edit(uri, &doc, start, end, body, None)
            .await;
        Ok(Some(serde_json::json!(applied)))
    }

    /// The receipt form of [`Self::write_rule_back_command`].
    async fn write_rule_back_receipt(
        &self,
        args: &[serde_json::Value],
    ) -> jsonrpc::Result<Option<serde_json::Value>> {
        let refuse = |reason: &str| {
            Ok(Some(
                serde_json::json!({ "applied": false, "reason": reason }),
            ))
        };
        let receipt = &args[0];
        let field = |name: &str| receipt.get(name).and_then(serde_json::Value::as_str);
        let (Some(uri_str), Some(header), Some(full_path), Some(original), Some(new_body)) = (
            field("uri"),
            field("header"),
            field("fullPath"),
            field("body"),
            args.get(1).and_then(serde_json::Value::as_str),
        ) else {
            return refuse("request");
        };
        let Ok(uri) = Uri::from_str(uri_str) else {
            return refuse("request");
        };
        let Some(doc) = self.read_local_document(&uri).await? else {
            return refuse("notOpen");
        };
        let Some(rule) = Self::rejoin_rule(&doc.text, header, full_path, original) else {
            return refuse("stale");
        };
        let updated = tcl_bigip::rule_extract::replace_rule_body(&doc.text, &rule, new_body);
        let Some(after) = Self::rejoin_rule(&updated, header, full_path, new_body)
            .filter(|after| after.body_start_offset == rule.body_start_offset)
        else {
            return refuse("body");
        };
        let version = doc.version;
        if !self
            .apply_rule_body_edit(
                uri,
                &doc,
                rule.body_start_offset,
                rule.body_end_offset,
                new_body,
                version,
            )
            .await
        {
            return refuse("rejected");
        }
        Ok(Some(serde_json::json!({
            "applied": true,
            "rule": Self::rule_info(&after, uri_str, None),
        })))
    }

    /// The one rule stanza in `source` with this header and path whose body
    /// is still `body`; `None` when there is not exactly one.
    fn rejoin_rule(
        source: &str,
        header: &str,
        full_path: &str,
        body: &str,
    ) -> Option<tcl_bigip::rule_extract::EmbeddedRule> {
        let mut matching = tcl_bigip::rule_extract::find_embedded_rules(source)
            .into_iter()
            .filter(|rule| rule.header == header && rule.full_path == full_path);
        let rule = matching.next()?;
        (matching.next().is_none() && rule.body == body).then_some(rule)
    }

    /// Ask the client to replace `[start, end)` of `doc` with `body`. With a
    /// `version`, the edit is a versioned document change the client applies
    /// only to that version.
    async fn apply_rule_body_edit(
        &self,
        uri: Uri,
        doc: &DocumentState,
        start: usize,
        end: usize,
        body: &str,
        version: Option<i32>,
    ) -> bool {
        // The rule offsets count bytes; LSP ranges count UTF-16 code units,
        // so the span is converted rather than passed through.
        let index = tcl_lexer::LineIndex::new_lsp(&doc.text);
        let to_position = |offset: usize| {
            let p = index.position_at_utf16(u32::try_from(offset).unwrap_or(u32::MAX), &doc.text);
            Position {
                line: p.line,
                character: p.character.get(),
            }
        };
        let edit = TextEdit {
            range: Range {
                start: to_position(start),
                end: to_position(end),
            },
            new_text: body.to_owned(),
        };
        let workspace_edit = if version.is_some() {
            WorkspaceEdit {
                document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
                    text_document: OptionalVersionedTextDocumentIdentifier { uri, version },
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
        self.client
            .apply_edit(workspace_edit)
            .await
            .is_ok_and(|response| response.applied)
    }
}
