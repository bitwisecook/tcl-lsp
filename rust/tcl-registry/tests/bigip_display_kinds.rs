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

//! Differential tests for `candidate_registry_kinds_for_display` (display
//! `Reference.target_kind` → registry kinds) against the captured golden
//! registry.
//!
//! Both exact `(module, object_type)` matches (`"ltm rule"`, `"net vlan"`, …)
//! and family fan-outs (`"ltm monitor"`, `"ltm profile"`, …) must match the
//! golden exactly — the registry-data regeneration brought the
//! fan-out kind sets fully in sync.

use tcl_registry::bigip::default_registry;

#[test]
fn display_kinds_match_golden() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/display_kinds.golden.tsv"
    );
    let golden = std::fs::read_to_string(path).expect("read golden");
    let reg = default_registry();

    for line in golden.lines() {
        let mut it = line.splitn(2, '\t');
        let display = it.next().unwrap();
        let want_str = it.next().unwrap_or("");
        let want: Vec<&str> = if want_str.is_empty() {
            Vec::new()
        } else {
            want_str.split(',').collect()
        };
        let got = reg.candidate_registry_kinds_for_display(display);
        assert_eq!(got, want, "display kind {display:?}");
    }
}
