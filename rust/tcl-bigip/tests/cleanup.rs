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

//! Byte-stable fixtures for `compute_cleanup` (tmsh script + JSON), checked
//! against captured expected output across keep-filter variants. Self-contained.

use std::collections::HashSet;

use tcl_bigip::cleanup::{compute_cleanup, report_to_json};
use tcl_bigip::graph::{GraphContext, build_bigip_object_graph};
use tcl_bigip::parser::parse_bigip_conf;

fn build() -> (GraphContext, tcl_bigip::parser::BigipConfig, String) {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let source = std::fs::read_to_string(format!("{dir}/bigip.conf")).expect("read config");
    let cfg = parse_bigip_conf(&source, "Common");
    (GraphContext::new(), cfg, source)
}

fn check(name: &str, keep_paths: &[&str], keep_partitions: &[&str]) {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures");
    let (ctx, cfg, source) = build();
    let uri = "test://config".to_owned();
    let sources = [(uri.clone(), source)];
    let configs = [(uri.clone(), &cfg)];
    let graph = build_bigip_object_graph(&sources, &configs, &ctx);

    let kp: HashSet<String> = keep_paths.iter().map(|s| (*s).to_owned()).collect();
    let parts: Vec<String> = keep_partitions.iter().map(|s| (*s).to_owned()).collect();
    let report = compute_cleanup(&graph, &[uri], &kp, &parts);

    let mut tmsh = report.tmsh_script.clone();
    if !tmsh.ends_with('\n') {
        tmsh.push('\n');
    }
    let want_tmsh = std::fs::read_to_string(format!("{dir}/cleanup_{name}.tmsh.golden"))
        .expect("read tmsh golden");
    assert_eq!(
        tmsh, want_tmsh,
        "cleanup tmsh ({name}) differs from the expected fixture"
    );

    let json = report_to_json(&report);
    let want_json = std::fs::read_to_string(format!("{dir}/cleanup_{name}.json.golden"))
        .expect("read json golden");
    assert_eq!(
        json, want_json,
        "cleanup JSON ({name}) differs from the expected fixture"
    );
}

#[test]
fn cleanup() {
    check("default", &[], &["/Common/"]);
    check("nokeepcommon", &[], &[]);
    check("keeppath", &["/Common/unused_pool"], &[]);
}
