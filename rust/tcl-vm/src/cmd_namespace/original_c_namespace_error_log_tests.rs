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

#[test]
fn public_namespace_error_logging_reaches_later_original_words_after_handler_refusal() {
    // Tcl_EvalObjv, tclBasic.c:3420 (8.4.20), generates the complete argv
    // only after TclEvalObjvInternal reports Error; the handler has returned.
    for release in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        for selector in ["export", "import"] {
            let mut vm =
                crate::native_fixture::core(crate::environment::profile_for_dialect(release));
            let later = Value::int(42);
            assert!(later.resident_string_bytes().is_none());
            let invalid = if selector == "export" {
                "::invalid"
            } else {
                "unqualified"
            };
            let arguments = [
                Value::string(selector),
                Value::string(invalid),
                later.clone(),
            ];
            let result =
                vm.invoke_host_original_object_vector(&Value::string("namespace"), &arguments);
            assert_eq!(result.code, Code::Error, "{release}/{selector}");
            assert_eq!(later.resident_string_bytes().unwrap().as_ref(), b"42");
            let info = vm.error_info_value().unwrap();
            assert!(info.windows(2).any(|part| part == b"42"));
            assert!(vm.execution_refusal.is_none());
        }
    }
}
