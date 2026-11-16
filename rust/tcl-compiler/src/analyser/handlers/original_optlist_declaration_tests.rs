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

use crate::analyser::Analyser;

#[test]
fn original_optlist_procedure_retains_its_name_without_fabricating_native_formals() {
    let source = r"tcl::OptProc p\uD800 {{-option DEFAULT}} {return $args}";
    let mut result = Analyser::new().analyse(source, "tcl8.6");
    let record = result.original_procedure_declarations().next().unwrap();
    assert_eq!(record.name_input().bytes(), b"p\xed\xa0\x80");
    assert_eq!(record.metadata().params.len(), 1);
    assert_eq!(record.metadata().params[0].name, "args");
    assert!(!record.metadata().params[0].has_default);
    let input = crate::signature_scan::scope::SignatureSourceNameInput::OriginalWord(
        record.name_input().clone(),
    );
    let namespace = result.original_namespace_scope_at(0).unwrap().clone();
    result.all_procs.clear();
    result.global_scope.procs.clear();
    assert_eq!(
        result
            .procedures_for_original_name(&input, &namespace)
            .len(),
        1
    );
}
