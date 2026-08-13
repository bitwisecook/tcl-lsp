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

//! Byte-stable fixtures for `compute_stats` (text report + JSON), checked
//! against captured expected output on a realistic `bigip.conf`. Self-contained.

use tcl_bigip::graph::{GraphContext, build_bigip_object_graph};
use tcl_bigip::parser::parse_bigip_conf;
use tcl_bigip::stats::{compute_stats, report_to_json};

#[test]
fn stats() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let source = std::fs::read_to_string(format!("{dir}/bigip.conf")).expect("read config");
    let cfg = parse_bigip_conf(&source, "Common");
    let uri = "test://config".to_owned();
    let sources = [(uri.clone(), source)];
    let configs = [(uri, &cfg)];

    let ctx = GraphContext::new();
    let graph = build_bigip_object_graph(&sources, &configs, &ctx);

    for top in [10usize, 3] {
        let report = compute_stats(&graph, &configs, top);

        let mut text = report.text_report.clone();
        if !text.ends_with('\n') {
            text.push('\n');
        }
        let want_text = std::fs::read_to_string(format!("{dir}/stats_top{top}.txt.golden"))
            .expect("read text golden");
        assert_eq!(
            text, want_text,
            "stats text (top {top}) differs from the expected fixture"
        );

        let json = report_to_json(&report) + "\n";
        let want_json = std::fs::read_to_string(format!("{dir}/stats_top{top}.json.golden"))
            .expect("read json golden");
        assert_eq!(
            json, want_json,
            "stats JSON (top {top}) differs from the expected fixture"
        );
    }
}
