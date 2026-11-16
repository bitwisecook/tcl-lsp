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

//! The design's tenant pack example: its completion test on the CLI and the pack surfaces.

use super::*;

/// The value-transfer design's executable example, whose one home is
/// the compiler's test fixtures.
const TENANT_PACK: &str =
    include_str!("../../../tcl-compiler/tests/fixtures/value_transfers/tenant.tclspec");

/// The example's three spellings: its own name, the rename, and the
/// subcommand form whose operand sits one word later.
const TENANT_SPELLINGS: [&str; 3] = ["tenant::label", "tenant::tag", "tenant label"];

/// A scratch workspace named `name` whose `.tcl-lsp/` directory holds
/// `pack`, the one pack the CLI then discovers from its working directory,
/// or nothing.
fn scratch_workspace(name: &str, pack: Option<&str>) -> PathBuf {
    let root =
        std::env::temp_dir().join(format!("value-transfers-cli-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".tcl-lsp")).expect("a scratch workspace");
    if let Some(pack) = pack {
        std::fs::write(root.join(".tcl-lsp").join("tenant.tclspec"), pack).expect("the pack");
    }
    root
}

/// [`run_tcl`] with `workspace` as the working directory, so the pack in
/// its `.tcl-lsp/` is the one discovered.
fn run_tcl_in(workspace: &Path, args: &[&str]) -> String {
    let output = tcl()
        .current_dir(workspace)
        .args(args)
        .reused_output()
        .expect("failed to spawn tcl binary");
    assert!(
        output.status.success(),
        "tcl {args:?} exited {:?}\nstderr: {}",
        output.status.code(),
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf-8 stdout")
}

/// The three spellings, one statement each: `set a [tenant::label acme]`,
/// `set b [tenant::tag acme]`, `set c [tenant label acme]`.
fn one_statement_per_spelling(statement: impl Fn(char, &str) -> String) -> String {
    ['a', 'b', 'c']
        .into_iter()
        .zip(TENANT_SPELLINGS)
        .map(|(variable, spelling)| statement(variable, spelling))
        .collect::<Vec<_>>()
        .join("\n")
}

/// `tcl explore --show sccp --text` over one assignment per spelling in
/// `workspace`: each variable holds `tenant:acme`, from the implementation
/// route, and every answer is evaluated.
fn assert_the_explorer_folds(workspace: &Path) {
    let source = one_statement_per_spelling(|variable, spelling| {
        format!("set {variable} [{spelling} acme]")
    });
    let explored = run_tcl_in(
        workspace,
        &[
            "explore",
            "--source",
            &source,
            "--show",
            "sccp",
            "--text",
            "--no-colour",
        ],
    );
    for (variable, spelling) in ['a', 'b', 'c'].into_iter().zip(TENANT_SPELLINGS) {
        let head = spelling.split(' ').next().expect("a head");
        assert!(
            explored.contains(&format!("{variable}#1 = const('tenant:acme')")),
            "{spelling}:\n{explored}"
        );
        assert!(
            explored.contains(&format!("route {head}: implementation tenant.label.v1")),
            "{spelling}:\n{explored}"
        );
    }
    assert_eq!(
        explored.matches("· answer: evaluated").count(),
        3,
        "{explored}"
    );
    assert!(explored.contains("implementation 3"), "{explored}");
}

/// The completion test on the CLI and the pack surfaces
/// (`docs/design/compiler/value-transfers.md` § *The completion test*),
/// beside `the_completion_test_needs_no_consumer_edit` in the compiler
/// witnesses: the executable example's three spellings, from a workspace
/// pack alone, fold `acme` to `tenant:acme` in `tcl explore`, make the
/// condition over it constant in `tcl diag` (I230) and forward the constant
/// in `tcl opt` (O100). `tcl spec export`'s canonical pack and a studio
/// form edit of each declaration keep all three evaluators — the renderer
/// writes a `-native` placeholder for the body it cannot draw, and the
/// studio carries the author's own bytes forward over it — and each folds
/// again from its own workspace. A workspace without the pack folds
/// nothing.
#[test]
fn the_completion_test_reaches_every_surface() {
    let workspace = scratch_workspace("fixture", Some(TENANT_PACK));
    assert_the_explorer_folds(&workspace);
    let conditions = one_statement_per_spelling(|_, spelling| {
        format!("if {{[{spelling} acme] eq \"tenant:acme\"}} {{puts yes}} else {{puts no}}")
    });
    let diagnosed = run_tcl_in(&workspace, &["diag", "--source", &conditions]);
    assert_eq!(
        diagnosed.matches("I230").count(),
        3,
        "every condition is constant:\n{diagnosed}"
    );
    let forwarded = one_statement_per_spelling(|variable, spelling| {
        format!("set {variable} [{spelling} acme]; puts ${variable}")
    });
    let optimised = run_tcl_in(
        &workspace,
        &["opt", "--source", &forwarded, "--profile", "full"],
    );
    assert_eq!(
        optimised.matches("puts tenant:acme").count(),
        3,
        "{optimised}"
    );
    assert!(optimised.contains("O100"), "{optimised}");

    let bare = scratch_workspace("bare", None);
    let explored = run_tcl_in(
        &bare,
        &[
            "explore",
            "--source",
            "set r [tenant::label acme]",
            "--show",
            "sccp",
            "--text",
            "--no-colour",
        ],
    );
    assert!(explored.contains("r#1 = overdefined"), "{explored}");
    assert!(
        explored.contains("implementation 0") && !explored.contains("tenant:acme"),
        "{explored}"
    );

    let exported = run_tcl_in(&workspace, &["spec", "export", ".tcl-lsp/tenant.tclspec"]);
    assert_eq!(
        exported
            .matches("evaluate -implementation tenant.label.v1")
            .count(),
        3,
        "{exported}"
    );
    let from_export = scratch_workspace("exported", Some(&exported));
    assert_the_explorer_folds(&from_export);

    let mut store = tcl_spec_studio::store::PackStore::from_source(TENANT_PACK);
    // A declared implementation's body is not on the spec, so the draft names
    // `semantics` as the part it cannot recover and the renderer writes the
    // `-native` placeholder in its place: the loss is stated, never silent,
    // and the studio's carry-forward below is what keeps the body.
    let drafted = store.draft("tenant::label").expect("tenant::label").clone();
    assert!(
        drafted[tcl_spec_studio::draft::UNRENDERABLE_KEY]
            .as_array()
            .is_some_and(|lost| lost.iter().any(|key| key == "semantics")),
        "{drafted:?}"
    );
    let rendered = tcl_spec_studio::render_spectcl::render_pack(&[drafted], "tenant");
    assert!(
        rendered.contains("semantics -native tenant::label::semantics"),
        "{rendered}"
    );
    for name in ["tenant::label", "tenant::tag", "tenant"] {
        let mut edited = store.draft(name).expect(name).clone();
        edited.insert("return_type".to_owned(), serde_json::json!("String"));
        let write = store.set_command(name, &edited, false);
        assert!(
            write.dropped.is_empty(),
            "{name}: {:?}\n{}",
            write.dropped,
            store.source()
        );
    }
    let studio = store.source().to_owned();
    assert_eq!(
        studio
            .matches("evaluate -implementation tenant.label.v1")
            .count(),
        3,
        "{studio}"
    );
    assert!(!studio.contains("-native"), "{studio}");
    let from_studio = scratch_workspace("studio", Some(&studio));
    assert_the_explorer_folds(&from_studio);

    for root in [workspace, bare, from_export, from_studio] {
        let _ = std::fs::remove_dir_all(root);
    }
}
