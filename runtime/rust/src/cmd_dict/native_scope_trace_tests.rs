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

//! Measured original variable reads at dictionary scope boundaries.

use crate::interp::{default_host, Code, Interp};
use crate::obj::{self, Owned};

fn interpreter(profile: &str) -> Interp {
    Interp::with_native_core(
        default_host(),
        crate::environment::profile_for_dialect(profile),
        tcl_registry::special_vars::NativeBootstrapInputs {
            package_path: Vec::new(),
            default_library: None,
        },
    )
    .expect("original native core constructor")
}

fn decode(hex: &str) -> Vec<u8> {
    hex.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

// Native proof: naming.variable.dictionary-scope-trace-phase-order
// docs/design/analysis/name-resolution-proofs/variable.dictionary-scope-trace-phase-order.md
#[test]
fn dictionary_scope_reads_match_64_native_callback_windows() {
    let mut compared = 0;
    for row in include_str!("../../tests/data/native_dictionary_scope_traces/controls.tsv").lines()
    {
        let fields = row.split('\t').collect::<Vec<_>>();
        assert_eq!(fields.len(), 5);
        let profile = match fields[0] {
            "8.5.19" => "tcl8.5",
            "8.6.18" => "tcl8.6",
            "9.0.4" => "tcl9.0",
            "9.1.0" => "tcl9.1",
            "8.4.20" | "jim" => continue,
            _ => panic!("unmeasured native version"),
        };
        let mut interp = interpreter(profile);
        let source = decode(fields[2]);
        let originals = [b"eval".as_slice(), source.as_slice()]
            .map(|bytes| Owned::fresh(obj::new_string_bytes(bytes)));
        let arguments = originals.iter().map(Owned::as_ptr).collect::<Vec<_>>();
        let code = interp.eval_original_object_vector(&arguments);
        assert_eq!(
            code,
            Code::from_int(fields[3].parse().unwrap()),
            "{}/{}: {:?}",
            fields[0],
            fields[1],
            interp.native_access_refusal(),
        );
        assert_eq!(
            interp
                .native_object_string_bytes(interp.result_obj())
                .unwrap()
                .as_ref(),
            decode(fields[4]),
            "{}/{}",
            fields[0],
            fields[1],
        );
        compared += 1;
    }
    assert_eq!(compared, 64);
}

#[test]
fn dictionary_scope_read_returns_the_same_original_dictionary_header() {
    for profile in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut interp = interpreter(profile);
        let name = Owned::fresh(obj::new_string_bytes(b"d"));
        let original = Owned::fresh(obj::new_string_bytes(b"k ORIGINAL"));
        interp
            .assign_original_named_variable(name.as_ptr(), original.as_ptr())
            .unwrap();
        assert_eq!(interp.eval_str(b"set events {}; proc watch {a b op} {lappend ::events $op}; trace add variable d read watch"), Code::Ok);
        for phase in [
            crate::interp::DictionaryScopeRead::Initial,
            crate::interp::DictionaryScopeRead::UpdateWriteback,
            crate::interp::DictionaryScopeRead::WithWriteback,
        ] {
            assert_eq!(
                interp
                    .dictionary_scope_variable_read(name.as_ptr(), phase)
                    .unwrap(),
                Some(original.as_ptr()),
                "{profile}: same original root after read callback",
            );
        }
        assert_eq!(
            interp.eval_str(b"trace remove variable d read watch; set events"),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"read read read");
    }
}
