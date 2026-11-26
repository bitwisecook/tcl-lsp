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

mod tests {
    #[test]
    fn original_match_source_visits_pattern_then_subject_under_authentic_compiler() {
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(engine).unwrap();
            let mut vm = crate::fixture::interpreter(profile);
            let completion = vm.eval_source("set order {}; proc pattern {} {global order; set order pattern; return *}; proc subject {} {global order; set order [list $order subject]; return A}; proc match {} {string match [pattern] [subject]}; list [match] $order").unwrap();
            assert_eq!(
                completion.code,
                tcl_vm::Code::Ok,
                "{engine}: {}",
                completion.result.to_str()
            );
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                b"1 {pattern subject}",
                "{engine}"
            );
        }
    }
}
