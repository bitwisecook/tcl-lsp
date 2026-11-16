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

use super::*;
use tcl_compiler::analyser::Analyser;

#[test]
// Implementation contract: naming.core.readonly-member-source-candidates
// docs/design/analysis/name-resolution-proofs/readonly-member-source-candidates.md
fn original_readonly_worker_value_does_not_retag_variable_syntax_as_member_declaration() {
    let source = "oo::class create C {foreach item {left right} {method $item {} {return body}}}";
    let analysis = Analyser::new().analyse(source, "tcl8.6");
    let methods = analysis
        .original_class_declarations()
        .next()
        .unwrap()
        .metadata()
        .original_members
        .declarations()
        .collect::<Vec<_>>();
    assert_eq!(methods.len(), 2);
    let roles = OriginalTokenRoles::capture(source, Some(&analysis));
    for method in methods {
        assert!(method.declaration().static_occurrence().is_none());
        assert!(
            !roles
                .names
                .contains_key(&method.declaration().original_word().span().start())
        );
    }
}
