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

#[test]
fn original_document_member_grammar_keeps_source_vocabulary_separate_from_execution() {
    // Implementation contract: naming.consumer.typed-spec-pack-notice-actions
    // docs/design/analysis/name-resolution-proofs/typed-spec-pack-notice-actions.md
    let context = crate::model::ingress::static_context_for("spectcl");
    let registry = context.commands();
    let document = registry.document_grammar().unwrap();
    let pack = registry
        .authored_document_member_grammar(document, "speclib")
        .unwrap();
    let command = registry
        .authored_document_member_grammar(pack, "command")
        .unwrap();
    assert!(
        command
            .members
            .iter()
            .any(|member| member.keyword == "arity")
    );
    assert!(
        registry
            .authored_document_member_grammar(document, "command")
            .is_none()
    );
    assert!(
        registry
            .authored_document_member_grammar(command, "pool")
            .is_none()
    );
    let native = crate::model::ingress::static_context_for("tcl8.6").commands();
    assert!(
        native
            .authored_document_member_grammar(document, "speclib")
            .is_none()
    );
    let other = crate::model::ingress::static_context_for("sslictcl").commands();
    assert!(
        other
            .authored_document_member_grammar(document, "speclib")
            .is_none()
    );
}
