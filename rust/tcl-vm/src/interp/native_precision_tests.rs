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

//! Public precision lifecycle controls from six original native providers.

#[test]
fn original_precision_unset_matches_all_six_native_sources() {
    // naming.variable.original-precision-unset-publication
    // docs/design/analysis/name-resolution-proofs/variable-original-precision-unset-publication.md
    // Exact public results only; these rows do not observe hidden cell
    // identities or establish a pointer-equivalence claim.
    macro_rules! provider {
        ($engine:literal, $version:literal) => {
            (
                $engine,
                [
                    include_str!(concat!(
                        "../../../tcl-registry/tests/data/native_precision_unset319/",
                        $version,
                        "/root/stdout"
                    )),
                    include_str!(concat!(
                        "../../../tcl-registry/tests/data/native_precision_unset319/",
                        $version,
                        "/linked/stdout"
                    )),
                    include_str!(concat!(
                        "../../../tcl-registry/tests/data/native_precision_unset319/",
                        $version,
                        "/unrelated/stdout"
                    )),
                ],
            )
        };
    }
    let sources = [
        include_str!("../../../tcl-registry/tests/data/native_precision_unset319/root.tcl"),
        include_str!("../../../tcl-registry/tests/data/native_precision_unset319/linked.tcl"),
        include_str!("../../../tcl-registry/tests/data/native_precision_unset319/unrelated.tcl"),
    ];
    for (engine, outputs) in [
        provider!("tcl8.4", "8.4.20"),
        provider!("tcl8.5", "8.5.19"),
        provider!("tcl8.6", "8.6.18"),
        provider!("tcl9.0", "9.0.4"),
        provider!("tcl9.1", "9.1.0"),
        provider!("jim", "jim"),
    ] {
        for (source, stdout) in sources.iter().zip(outputs) {
            let original = stdout
                .lines()
                .find_map(|line| line.strip_prefix("ORIGINAL|0|"))
                .unwrap();
            let expected: Vec<u8> = original
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect();
            let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
            // The unchanged original is evaluated as source; the core issuer
            // and actual bytecode compile service remain separate capabilities.
            let mut vm = crate::native_fixture::interpreter(profile);
            let completion = vm.eval_source(source).unwrap();
            assert_eq!(
                completion.code,
                crate::Code::Ok,
                "{engine}: {:?}",
                completion.result.string_bytes()
            );
            assert!(vm.refused_completion().is_none(), "{engine}");
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                expected.as_slice(),
                "{engine}"
            );
        }
    }
}
