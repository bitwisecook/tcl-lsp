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

//! Behavioural coverage for the iRules walker (`extract_irules_object_references`),
//! exercising literal refs, `set`-binding copy-propagation, widening, nested
//! bodies, `if` conditions, and GTM pool fan-out against fixed expected outputs.
//! Self-contained — runs entirely in-process.

use tcl_dialect::model::{Family, SurfaceLayer};
use tcl_irules::extract_irules_object_references;
use tcl_registry::CommandRegistry;

#[test]
fn walker_matches_golden() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let cases: Vec<(String, String)> =
        serde_json::from_str(&std::fs::read_to_string(format!("{dir}/irule_cases.json")).unwrap())
            .expect("parse cases");
    let golden = std::fs::read_to_string(format!("{dir}/irule_refs.golden.tsv")).expect("golden");

    let mut reg = CommandRegistry::build_default();
    reg.load_surface(SurfaceLayer::Core(Family::F5Irules, ""));

    for (i, (line, (source, rule_module))) in golden.lines().zip(&cases).enumerate() {
        let want = line.split('\t').nth(1).unwrap_or("-");
        let refs = extract_irules_object_references(source, Some(rule_module), &reg);
        let got = if refs.is_empty() {
            "-".to_owned()
        } else {
            refs.iter()
                .map(|r| {
                    format!(
                        "{}|{}|{}|{}",
                        r.name,
                        r.command,
                        r.argument_index,
                        r.kinds.join(",")
                    )
                })
                .collect::<Vec<_>>()
                .join("  ;;  ")
        };
        assert_eq!(got, want, "case {i}: {source:?}");
    }
}
