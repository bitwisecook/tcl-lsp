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

//! Original child-alias public windows and independently authored issuer controls.

fn original_result(rows: &str) -> (i64, Vec<u8>) {
    let fields: Vec<_> = rows
        .lines()
        .find(|line| line.starts_with("ORIGINAL|"))
        .unwrap()
        .split('|')
        .collect();
    let hex = fields[2].as_bytes();
    assert_eq!(hex.len() % 2, 0);
    (
        fields[1].parse().unwrap(),
        hex.chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect(),
    )
}

type OriginalControl = (&'static str, &'static [u8], [&'static str; 6]);
const CONTROLS: &[OriginalControl] = &[
    ("original-rooted-child-alias", include_bytes!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/original-rooted-child-alias.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.4.20/original-rooted-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.5.19/original-rooted-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.6.18/original-rooted-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/9.0.4/original-rooted-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/9.1.0/original-rooted-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/jim/original-rooted-child-alias/stdout"),
    ]),
    ("original-extra-root-child-alias", include_bytes!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/original-extra-root-child-alias.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.4.20/original-extra-root-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.5.19/original-extra-root-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.6.18/original-extra-root-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/9.0.4/original-extra-root-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/9.1.0/original-extra-root-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/jim/original-extra-root-child-alias/stdout"),
    ]),
    ("original-nul-before-suffix-child-alias", include_bytes!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/original-nul-before-suffix-child-alias.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.4.20/original-nul-before-suffix-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.5.19/original-nul-before-suffix-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.6.18/original-nul-before-suffix-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/9.0.4/original-nul-before-suffix-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/9.1.0/original-nul-before-suffix-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/jim/original-nul-before-suffix-child-alias/stdout"),
    ]),
    ("original-nul-after-suffix-child-alias", include_bytes!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/original-nul-after-suffix-child-alias.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.4.20/original-nul-after-suffix-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.5.19/original-nul-after-suffix-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.6.18/original-nul-after-suffix-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/9.0.4/original-nul-after-suffix-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/9.1.0/original-nul-after-suffix-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/jim/original-nul-after-suffix-child-alias/stdout"),
    ]),
    ("original-unicode-child-alias", include_bytes!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/original-unicode-child-alias.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.4.20/original-unicode-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.5.19/original-unicode-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.6.18/original-unicode-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/9.0.4/original-unicode-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/9.1.0/original-unicode-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/jim/original-unicode-child-alias/stdout"),
    ]),
    ("original-ff-child-alias", include_bytes!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/original-ff-child-alias.tcl").as_slice(), [
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.4.20/original-ff-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.5.19/original-ff-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/8.6.18/original-ff-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/9.0.4/original-ff-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/9.1.0/original-ff-child-alias/stdout"),
        include_str!("../../../../rust/tcl-registry/tests/data/native_child_alias_inventory298/jim/original-ff-child-alias/stdout"),
    ]),
];

#[test]
fn original_child_alias_publication_matches_all_36_native_public_windows() {
    // naming.alias.original-child-publication-and-inventory
    // docs/design/analysis/name-resolution-proofs/alias-original-child-publication-and-inventory.md
    // Replay each complete original source without altering counted input.
    // Literal U+00E9 is UTF-8 source; NUL and FF are produced by binary format.
    // NAME_BEFORE, definition, invocation, command inventory, actual alias API,
    // and the caught opposite alias API remain independently captured fields
    // within the whole result. Jim child forwarding commands are not core
    // aliases. C inventory's encoding-convertto channel differs from Jim raw
    // bytes. Every inventory has one matching alias; no hash-order claim follows.
    // These observations confer no original header, parent lifetime, physical
    // key identity, cross-provider string encoding or Rust execution authority.
    let providers = ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"];
    let mut comparisons = 0;
    for &(case, source, columns) in CONTROLS {
        for (engine, column) in providers.iter().zip(columns) {
            let (expected_code, expected_result) = original_result(column);
            let mut interp = super::Interp::with_native_core(
                super::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            // The original byte-producing dependency is explicitly installed;
            // it grants no complete Jim distribution or core alias membership.
            if *engine == "jim" {
                crate::cmd_binary::install(&mut interp);
            }
            let code = interp.eval_str(source);
            assert_eq!(
                code.as_int(),
                expected_code,
                "{engine}/{case}: {:?}; host={:?}",
                interp.result_bytes(),
                interp.native_access_refusal()
            );
            assert!(!interp.host_refusal_pending(), "{engine}/{case}");
            assert_eq!(interp.result_bytes(), expected_result, "{engine}/{case}");
            comparisons += 1;
        }
    }
    assert_eq!(comparisons, 36);
}

#[test]
fn child_alias_publication_refuses_missing_or_foreign_issuer_before_binding() {
    // Authored host-policy mutation only: Native298 does not measure these
    // unavailable/foreign host states or supply a child object lifetime grant.
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
        for foreign in [false, true] {
            let mut interp = super::Interp::with_native_core(
                super::default_host(),
                crate::environment::profile_for_dialect(engine),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            interp.create_child(Some(b"child".to_vec()));
            assert!(interp.child_exists(b"child"));
            let child_before = interp
                .with_child(b"child", |child| {
                    let selected = if foreign {
                        if engine == "jim" { "tcl8.6" } else { "jim" }
                    } else {
                        "irules"
                    };
                    child.set_dialect_profile(crate::environment::profile_for_dialect(selected));
                    assert!(
                        child
                            .namespaces()
                            .command_in(crate::namespace::GLOBAL, b"r2286_refused")
                            .is_none()
                    );
                    // Changing a profile may already retain a host-only cause
                    // while rebuilding stock wrappers. The refused installation
                    // must leave that actual child state untouched.
                    let result = child.result_obj();
                    let bytes = child.result_bytes().to_vec();
                    (
                        child.native_compilation_admission_error(),
                        child.native_access_refusal(),
                        result,
                        bytes,
                    )
                })
                .unwrap();
            interp.set_result_bytes(b"PARENT RESULT");
            let original = interp.result_obj();
            assert!(!interp.install_parent_alias(
                b"child",
                b"::r2286_refused",
                b"list".to_vec(),
                vec![b"VALUE".to_vec()]
            ));
            assert!(interp.host_refusal_pending());
            assert_eq!(interp.result_obj(), original);
            assert_eq!(interp.result_bytes(), b"PARENT RESULT");
            interp
                .with_child(b"child", |child| {
                    assert!(
                        child
                            .namespaces()
                            .command_in(crate::namespace::GLOBAL, b"r2286_refused")
                            .is_none()
                    );
                    assert_eq!(child.native_compilation_admission_error(), child_before.0);
                    assert_eq!(child.native_access_refusal(), child_before.1);
                    assert_eq!(child.result_obj(), child_before.2);
                    assert_eq!(child.result_bytes(), child_before.3);
                })
                .unwrap();
        }
    }
}
