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

use super::{TCLOO_GRAMMAR, TclVersion};

#[test]
fn original_forward_layout_requires_selected_oo_and_nonempty_prefix() {
    // naming.tcloo.original-forward-registration-prefix
    // docs/design/analysis/name-resolution-proofs/tcloo-original-forward-registration-prefix.md
    for version in [
        TclVersion::V8_4,
        TclVersion::V8_5,
        TclVersion::V8_6,
        TclVersion::V9_0,
        TclVersion::V9_1,
    ] {
        let dialect = crate::InvocationDialect::for_version(version);
        let forward = TCLOO_GRAMMAR.member("forward").unwrap();
        for count in [0, 1, 2, 8] {
            assert_eq!(
                TCLOO_GRAMMAR
                    .native_deferred_forward_setter(forward, dialect, count)
                    .is_some(),
                version >= TclVersion::V8_6 && count >= 2
            );
        }
        assert!(
            TCLOO_GRAMMAR
                .native_deferred_forward_setter(TCLOO_GRAMMAR.member("method").unwrap(), dialect, 3)
                .is_none()
        );
    }
    let jim = crate::InvocationDialect::of_profile(crate::native_test_provider::profile("jim"));
    assert!(
        TCLOO_GRAMMAR
            .native_deferred_forward_setter(TCLOO_GRAMMAR.member("forward").unwrap(), jim, 2)
            .is_none()
    );
}
#[test]
fn original_forward_layout_matches_captured_registration_boundaries() {
    // naming.tcloo.native-forward-registration-prefix-boundaries
    // docs/design/analysis/name-resolution-proofs/tcloo-native-forward-registration-prefix-boundaries.md
    let captures = [
        (
            "tcl8.4",
            include_str!("../../tests/data/native_tcloo_forward_registration/tcl8.4/stdout.tsv"),
        ),
        (
            "tcl8.5",
            include_str!("../../tests/data/native_tcloo_forward_registration/tcl8.5/stdout.tsv"),
        ),
        (
            "tcl8.6",
            include_str!("../../tests/data/native_tcloo_forward_registration/tcl8.6/stdout.tsv"),
        ),
        (
            "tcl9.0",
            include_str!("../../tests/data/native_tcloo_forward_registration/tcl9.0/stdout.tsv"),
        ),
        (
            "tcl9.1",
            include_str!("../../tests/data/native_tcloo_forward_registration/tcl9.1/stdout.tsv"),
        ),
        (
            "jim",
            include_str!("../../tests/data/native_tcloo_forward_registration/jim/stdout.tsv"),
        ),
    ];
    for (name, rows) in captures {
        let profile = crate::native_test_provider::profile(name);
        let dialect = crate::InvocationDialect::of_profile(profile);
        let available = !captured_forward_row(rows, "AVAILABILITY").1.is_empty();
        let member = TCLOO_GRAMMAR.member("forward").unwrap();
        assert_eq!(
            TCLOO_GRAMMAR
                .native_deferred_forward_setter(member, dialect, 2)
                .is_some(),
            available,
            "{name}"
        );
        if !available {
            continue;
        }
        for label in [
            "MISSING_REGISTER",
            "NO_TARGET_CALL",
            "SCRIPT_REPLACED",
            "FORWARD_REPLACED",
            "EMPTY_TARGET_REGISTER",
            "SUPER_FORWARD",
        ] {
            assert_eq!(captured_forward_row(rows, label).0, "0", "{name}: {label}");
        }
        assert_eq!(captured_forward_row(rows, "NO_TARGET_CALL").1, "30");
        assert_eq!(captured_forward_row(rows, "MISSING_INVOKE").0, "1");
        assert_eq!(captured_forward_row(rows, "PREFIX_REQUIRED").0, "1");
        assert!(
            TCLOO_GRAMMAR
                .native_deferred_forward_setter(member, dialect, 1)
                .is_none()
        );
        // This complete dynamic script observes a definition-scope difference;
        // the bounded source transfer admits only static prefix words.
        assert_eq!(
            captured_forward_row(rows, "PREFIX_CAPTURE").0,
            if name == "tcl8.6" { "0" } else { "1" }
        );
    }
}

fn captured_forward_row<'a>(rows: &'a str, label: &str) -> (&'a str, &'a str) {
    let line = rows
        .lines()
        .find(|line| line.split('|').next() == Some(label))
        .unwrap();
    let fields = line.split('|').collect::<Vec<_>>();
    assert_eq!(fields.len(), 4);
    (fields[1], fields[3])
}
