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

//! A precompiled literal command reference keeps a descendant command token
//! across an activation-delayed parent namespace deletion (#1755).

use tcl_dialect::TclVersion;
use tcl_runtime::interp::{Code, Interp};

const SOURCE: &str = r"set answer [namespace eval ::N {
    namespace eval C {proc q {} {return OLDQ}}
    proc p {} {
        namespace delete ::N
        namespace eval ::N::C {proc q {} {return NEWQ}}
        list [C::q] [::N::C::q]
    }
    p
}]
list $answer [::N::C::q]";

/// Measured on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1.0: from 8.5 both
/// references keep the retained old child token while the deleting activation
/// is live; 8.4 re-resolves the absolute spelling through the recreation.
#[test]
fn compiled_descendant_command_survives_activation_delayed_parent_deletion() {
    for (version, expected) in [
        (TclVersion::V8_4, "{OLDQ NEWQ} NEWQ"),
        (TclVersion::V8_5, "{OLDQ OLDQ} NEWQ"),
        (TclVersion::V8_6, "{OLDQ OLDQ} NEWQ"),
        (TclVersion::V9_0, "{OLDQ OLDQ} NEWQ"),
        (TclVersion::V9_1, "{OLDQ OLDQ} NEWQ"),
    ] {
        let mut interp = Interp::new();
        interp.set_runtime_version(version);
        let code = interp.eval_str(SOURCE.as_bytes());
        let result = String::from_utf8_lossy(&interp.result_bytes()).into_owned();
        assert_eq!((code, result.as_str()), (Code::Ok, expected), "{version:?}");
    }
}
