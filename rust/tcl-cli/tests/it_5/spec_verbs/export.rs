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

//! `tcl spec export`: a programmed pack expanded into canonical source.

use super::*;

// `tcl spec export` — the canonical renderer.

/// A templated pack — the shape `spec export` exists for.
const PROGRAM: &str = "speclib fleet 2.0 {
    proc fleet-command {name arity} {
        command math::fleet::$name {
            arity $arity
            traits {PURE}
            option -verbose -detail {Report each step as it runs.}

            subcommand probe {
                arity 0
                detail {Probe one input.}
            }
        }
    }

    foreach {name arity} {alpha 2 beta 1 gamma 3} {
        fleet-command $name $arity
    }
}
";

#[test]
fn spec_export_expands_a_programmed_pack_into_canonical_source() {
    let tree = Tree::new("export");
    let pack = tree.path().join("fleet.tclspec");
    std::fs::write(&pack, PROGRAM).expect("write pack");
    let path = pack.to_string_lossy().into_owned();

    let (stdout, stderr, code) = run(&["spec", "export", &path]);
    assert_eq!(code, 0, "stderr: {stderr}");

    // One literal declaration per iteration, and none of the program.
    for name in ["alpha", "beta", "gamma"] {
        assert!(
            stdout.contains(&format!("command math::fleet::{name} {{")),
            "{stdout}"
        );
    }
    for word in ["proc ", "foreach ", "$name", "$arity"] {
        assert!(!stdout.contains(word), "{stdout}");
    }
    // The loop's data is in place, per iteration.
    assert!(
        stdout.contains("arity 2") && stdout.contains("arity 3"),
        "{stdout}"
    );
    // The pack's own vocabulary word survives: raising it is `spec upgrade`.
    assert!(stdout.contains("speclib fleet 2.0 {"), "{stdout}");

    // And the expansion is a pack: it reloads to the same snapshot with no
    // templating left to run.
    let reloaded = tcl_spectcl::evaluate_pack(&stdout);
    assert!(reloaded.load_error.is_none(), "{:#?}", reloaded.notices);
    let names: Vec<&str> = reloaded.commands.iter().map(|c| c.spec.name).collect();
    assert_eq!(
        names,
        vec![
            "math::fleet::alpha",
            "math::fleet::beta",
            "math::fleet::gamma"
        ]
    );
}

#[test]
fn spec_export_round_trips_a_canonical_pack_through_the_shared_formatter() {
    // Every shipped example pack: export, reload, and compare the snapshot
    // the registry would see. This is the CLI-side half of the E-R11 gate —
    // the part that also passes the text through `format_pack`.
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/design/spec-dsl-examples");
    let mut packs = 0;
    for entry in std::fs::read_dir(&dir).expect("the spec-dsl-examples directory") {
        let path = entry.expect("a directory entry").path();
        if path.extension().is_none_or(|ext| ext != "tclspec") {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("readable pack");
        let (stdout, stderr, code) = run(&["spec", "export", &path.to_string_lossy()]);
        assert_eq!(code, 0, "{}: {stderr}", path.display());

        let before = tcl_spectcl::evaluate_pack(&source);
        let after = tcl_spectcl::evaluate_pack(&stdout);
        let render = |pack: &tcl_spectcl::Pack| {
            pack.commands
                .iter()
                .map(|c| format!("{:?}", c.spec))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            render(&before),
            render(&after),
            "{}: the formatted export is not the same snapshot",
            path.display()
        );
        packs += 1;
    }
    assert!(packs >= 8, "only {packs} example packs exported");
}

#[test]
fn spec_export_json_reports_the_expansion_and_its_notices() {
    let tree = Tree::new("export-json");
    let pack = tree.path().join("fleet.tclspec");
    std::fs::write(&pack, PROGRAM).expect("write pack");

    let (stdout, stderr, code) = run(&["spec", "export", &pack.to_string_lossy(), "--json"]);
    assert_eq!(code, 0, "stderr: {stderr}");
    let value: serde_json::Value = serde_json::from_str(&stdout).expect("JSON");
    assert_eq!(value["pack"], serde_json::json!("fleet"));
    assert_eq!(value["commands"], serde_json::json!(3));
    assert_eq!(value["target_dependent"], serde_json::json!(false));
    assert!(
        value["canonical_source"]
            .as_str()
            .expect("canonical source")
            .contains("command math::fleet::alpha {"),
        "{stdout}"
    );
    assert!(value["notices"].is_array(), "{stdout}");
}

#[test]
fn spec_export_reports_a_pack_whose_evaluation_failed() {
    let tree = Tree::new("export-denied");
    let pack = tree.path().join("clocky.tclspec");
    std::fs::write(
        &pack,
        "speclib clocky 2.0 {\n    set now [clock seconds]\n    command demo { arity 1 }\n}\n",
    )
    .expect("write pack");

    let (_stdout, stderr, code) = run(&["spec", "export", &pack.to_string_lossy()]);
    assert_eq!(code, 1, "a failed evaluation exits non-zero: {stderr}");
    assert!(stderr.contains("determinism axis"), "{stderr}");
}
