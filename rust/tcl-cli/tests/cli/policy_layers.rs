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

//! The diagnostic policy layers (global file, project file, profile, flags) as `tcl diag` and `tcl opt` resolve them.

use super::*;

/// A `while` whose counter the body never touches, against a bound the
/// solver cannot decide (a parameter, so no header fact settles it): W242,
/// the one code the catalogue declares default-off. Against a literal bound
/// the header is decided true at every test and the loop draws W241 instead.
const UNPROVABLE_LOOP: &str =
    "proc p {n} {\n    set i 0\n    while {$i < $n} {\n        puts $i\n    }\n}\n";

/// An abstaining document keeps the codes its bytes justify — the integrity
/// code and a bidirectional control, W305, which reads the decoded text as
/// text — as the editor's abstention does. A UTF-16
/// byte-order mark ahead of UTF-8 text is what abstains here; the tail
/// decodes losslessly, so the override it carries is still there to find.
#[test]
fn diag_keeps_a_bidi_control_on_an_abstaining_document() {
    let scratch = Scratch::new("abstain-bidi");
    let no_config = scratch.write("config/.keep", "");
    let xdg = no_config.parent().expect("config dir").as_os_str();
    let env: &[(&str, &std::ffi::OsStr)] = &[("XDG_CONFIG_HOME", xdg)];
    let bom_then = |text: &str| {
        let mut bytes = vec![0xFF, 0xFE];
        bytes.extend_from_slice(text.as_bytes());
        bytes
    };
    let with = scratch.write_bytes("with.tcl", &bom_then("# \u{202E}hidden\nputs hi\n"));
    let without = scratch.write_bytes("without.tcl", &bom_then("# hidden\nputs hi\n"));
    let codes = |path: &std::path::Path| -> Vec<String> {
        let path = path.to_str().expect("utf-8 path");
        diag_codes_by_file(&run_tcl_env(&["diag", "--json", path], env))
            .into_iter()
            .map(|(_, code)| code)
            .collect()
    };
    assert_eq!(
        codes(&with),
        ["W109", "W305"],
        "the integrity code and the bidi control survive abstention"
    );
    assert_eq!(
        codes(&without),
        ["W109"],
        "without the control the integrity code stands alone"
    );
}

/// The catalogue's default-off codes are off for `tcl diag` as they are in
/// the editor, and `--enable` turns one on — the seed is the lowest layer,
/// under every flag (`docs/design/compiler/diagnostic-policy.md`
/// § Configuration).
#[test]
fn diag_seeds_the_default_off_codes_like_the_editor() {
    let scratch = Scratch::new("default-off");
    let no_config = scratch.write("config/.keep", "");
    let xdg = no_config.parent().expect("config dir").as_os_str();
    let env: &[(&str, &std::ffi::OsStr)] = &[("XDG_CONFIG_HOME", xdg)];
    let off = diag_codes_by_file(&run_tcl_env(
        &["diag", "--json", "--source", UNPROVABLE_LOOP],
        env,
    ));
    assert!(
        !off.iter().any(|(_, code)| code == "W242"),
        "W242 is default-off and must not fire unasked: {off:?}"
    );
    let on = diag_codes_by_file(&run_tcl_env(
        &[
            "diag",
            "--json",
            "--enable",
            "W242",
            "--source",
            UNPROVABLE_LOOP,
        ],
        env,
    ));
    assert!(
        on.iter().any(|(_, code)| code == "W242"),
        "`--enable W242` must reach a default-off code: {on:?}"
    );
}

/// The project and global layers resolve per input file (issue #2063): a
/// file under a `.tcl-lsp.ini` that turns W112 off reports none while its
/// sibling from another directory still does. The global `config.ini` is the
/// lowest layer, the flags sit above it, and a project file sits above both.
/// A configuration file only turns codes off (`[diagnostics] disabled = …`),
/// so the flag's `--enable` is what turns a code the global file disabled
/// back on.
#[test]
fn diag_resolves_the_project_and_global_layers_per_input_file() {
    let scratch = Scratch::new("layers");
    let trailing = "set x 1   \nputs $x\n";
    scratch.write("quiet/.tcl-lsp.ini", "[diagnostics]\ndisabled = W112\n");
    let quiet = scratch.write("quiet/nested/a.tcl", trailing);
    let loud = scratch.write("loud/b.tcl", trailing);
    let config = scratch.write("xdg/tcl-lsp/config.ini", "[diagnostics]\ndisabled = W112\n");
    let xdg = config
        .parent()
        .and_then(|p| p.parent())
        .expect("xdg root")
        .as_os_str();
    let empty = scratch.write("empty-xdg/.keep", "");
    let no_global = empty.parent().expect("empty xdg").as_os_str();

    let per_file = diag_codes_by_file(&run_tcl_env(
        &[
            "diag",
            "--json",
            quiet.to_str().unwrap(),
            loud.to_str().unwrap(),
        ],
        &[("XDG_CONFIG_HOME", no_global)],
    ));
    let has = |rows: &[(String, String)], file: &PathBuf, code: &str| {
        rows.iter().any(|(label, c)| {
            label.ends_with(file.file_name().unwrap().to_str().unwrap()) && c == code
        })
    };
    assert!(
        !has(&per_file, &quiet, "W112"),
        "the project file above a.tcl turns W112 off: {per_file:?}"
    );
    assert!(
        has(&per_file, &loud, "W112"),
        "b.tcl sits under no project file and keeps W112: {per_file:?}"
    );

    // The global file reaches both.
    let with_global = diag_codes_by_file(&run_tcl_env(
        &[
            "diag",
            "--json",
            quiet.to_str().unwrap(),
            loud.to_str().unwrap(),
        ],
        &[("XDG_CONFIG_HOME", xdg)],
    ));
    assert!(
        !has(&with_global, &quiet, "W112"),
        "global and project both disable W112 for a.tcl: {with_global:?}"
    );
    assert!(
        !has(&with_global, &loud, "W112"),
        "the global file disables W112 for b.tcl: {with_global:?}"
    );

    // `--enable` overrules the global file, and the project file overrules
    // the flag.
    let enabled = diag_codes_by_file(&run_tcl_env(
        &[
            "diag",
            "--json",
            "--enable",
            "W112",
            quiet.to_str().unwrap(),
            loud.to_str().unwrap(),
        ],
        &[("XDG_CONFIG_HOME", xdg)],
    ));
    assert!(
        has(&enabled, &loud, "W112"),
        "`--enable W112` overrules the global `disabled = W112`: {enabled:?}"
    );
    assert!(
        !has(&enabled, &quiet, "W112"),
        "the project `disabled = W112` overrules `--enable`: {enabled:?}"
    );

    // An inline `--source` has no path, so no project layer: the global
    // file alone decides, and `--enable` in the invocation layer overrules it.
    let inline = diag_codes_by_file(&run_tcl_env(
        &["diag", "--json", "--source", trailing],
        &[("XDG_CONFIG_HOME", xdg)],
    ));
    assert!(
        !inline.iter().any(|(_, code)| code == "W112"),
        "the global layer reaches an inline source: {inline:?}"
    );
    let flagged = diag_codes_by_file(&run_tcl_env(
        &["diag", "--json", "--enable", "W112", "--source", trailing],
        &[("XDG_CONFIG_HOME", xdg)],
    ));
    assert!(
        flagged.iter().any(|(_, code)| code == "W112"),
        "`--enable` sits above the global file: {flagged:?}"
    );
}

/// A project file turns back on a code the global file turned off: an INI
/// layer's contribution is a per-code tri-state, spelled `CODE = true`
/// (`docs/design/contracts/xdg-config.md` § `[diagnostics]`), and the
/// project layer sits above the global one.
#[test]
fn diag_a_project_file_turns_a_code_back_on() {
    let scratch = Scratch::new("turn-back-on");
    let trailing = "set x 1   \nputs $x\n";
    scratch.write("project/.tcl-lsp.ini", "[diagnostics]\nW112 = true\n");
    let inside = scratch.write("project/inside.tcl", trailing);
    let outside = scratch.write("sibling/outside.tcl", trailing);
    let config = scratch.write("xdg/tcl-lsp/config.ini", "[diagnostics]\ndisabled = W112\n");
    let xdg = config
        .parent()
        .and_then(std::path::Path::parent)
        .expect("xdg root")
        .as_os_str();
    let rows = diag_codes_by_file(&run_tcl_env(
        &[
            "diag",
            "--json",
            inside.to_str().unwrap(),
            outside.to_str().unwrap(),
        ],
        &[("XDG_CONFIG_HOME", xdg)],
    ));
    let has = |file: &str, code: &str| {
        rows.iter()
            .any(|(label, c)| label.ends_with(file) && c == code)
    };
    assert!(
        has("inside.tcl", "W112"),
        "the project's `W112 = true` overrules the global disable: {rows:?}"
    );
    assert!(
        !has("outside.tcl", "W112"),
        "outside the project the global disable stands: {rows:?}"
    );
}

/// #2062's own program: `tcl opt --profile full` keeps a dead store a
/// `# noqa: O109` marks, and eliminates it when the marker is gone.
#[test]
fn opt_keeps_a_store_a_noqa_o109_marks() {
    let scratch = Scratch::new("opt-o109");
    let marked = "proc f {} {\n    # noqa: O109\n    set x 1\n    set x 2\n    return $x\n}\n";
    let with = scratch.write("b.tcl", marked);
    let without = scratch.write("plain.tcl", &marked.replace("    # noqa: O109\n", ""));
    let opt = |path: &PathBuf| {
        String::from_utf8(run_tcl(&[
            "opt",
            path.to_str().expect("utf-8 path"),
            "--profile",
            "full",
        ]))
        .expect("utf-8 output")
    };
    let kept = opt(&with);
    assert_eq!(kept, marked, "no rewrite applies to the marked store");
    let removed = opt(&without);
    assert!(
        !removed.contains("set x 1"),
        "the unmarked store goes: {removed}"
    );
    assert!(removed.contains("O109"), "{removed}");
}

/// `tcl opt` applies only the rewrites the document's policy shows (issue
/// #2062): a `# noqa` on the command keeps its fold off, a top-of-file
/// `# tcl-lsp: disable=*` keeps every rewrite off, and two inputs whose
/// directives differ are optimised each under its own policy rather than
/// folded into one text where the first file's directive would govern the
/// second.
#[test]
fn opt_applies_only_the_rewrites_the_policy_shows() {
    let scratch = Scratch::new("opt-policy");
    let empty = scratch.write("xdg/.keep", "");
    let xdg = empty.parent().expect("xdg").as_os_str();
    let env: &[(&str, &std::ffi::OsStr)] = &[("XDG_CONFIG_HOME", xdg)];
    // O101 folds the constant expression; a global stays a live store.
    let folding = "set x [expr {1 + 2}]\n";

    let plain = String::from_utf8(run_tcl_env(&["opt", "--source", folding], env)).unwrap();
    assert!(plain.contains("set x 3"), "the control folds: {plain}");

    let marked = format!("# noqa: O101\n{folding}");
    let kept = String::from_utf8(run_tcl_env(&["opt", "--source", &marked], env)).unwrap();
    assert!(
        kept.contains("[expr {1 + 2}]"),
        "a `# noqa` on the command keeps the fold off: {kept}"
    );

    let silenced = scratch.write("silenced.tcl", &format!("# tcl-lsp: disable=*\n{folding}"));
    let open = scratch.write("open.tcl", folding);
    let both = String::from_utf8(run_tcl_env(
        &["opt", silenced.to_str().unwrap(), open.to_str().unwrap()],
        env,
    ))
    .unwrap();
    assert!(
        both.contains("[expr {1 + 2}]"),
        "the silenced file's expression survives: {both}"
    );
    assert!(
        both.contains("set x 3"),
        "the open file's expression folds under its own policy: {both}"
    );
}

/// The owner's ruling: `--profile` is the profile in force over every
/// configuration file, and a file's `[optimiser] profile` — the project's,
/// then the global's — applies only when the flag is omitted
/// (`docs/design/compiler/diagnostic-policy.md` § Configuration).
#[test]
fn opt_a_named_profile_overrules_the_project_file() {
    let scratch = Scratch::new("opt-profile");
    let config = scratch.write("xdg/tcl-lsp/config.ini", "[optimiser]\nprofile = full\n");
    let xdg = config
        .parent()
        .and_then(|p| p.parent())
        .expect("xdg root")
        .as_os_str();
    let env: &[(&str, &std::ffi::OsStr)] = &[("XDG_CONFIG_HOME", xdg)];
    let folding = "set x [expr {1 + 2}]\n";
    scratch.write("proj/.tcl-lsp.ini", "[optimiser]\nprofile = readability\n");
    let inside = scratch.write("proj/fold.tcl", folding);
    let outside = scratch.write("loose/fold.tcl", folding);
    let opt = |args: &[&str]| String::from_utf8(run_tcl_env(args, env)).unwrap();

    let named = opt(&["opt", "--profile", "full", inside.to_str().unwrap()]);
    assert!(
        named.contains("set x 3"),
        "`--profile full` folds over the project's `readability`: {named}"
    );
    let unnamed = opt(&["opt", inside.to_str().unwrap()]);
    assert!(
        unnamed.contains("[expr {1 + 2}]"),
        "with no `--profile` the project's `readability` runs: {unnamed}"
    );

    let global = opt(&["opt", "--profile", "readability", outside.to_str().unwrap()]);
    assert!(
        global.contains("[expr {1 + 2}]"),
        "`--profile readability` wins over the global `full` too: {global}"
    );
    let defaulted = opt(&["opt", outside.to_str().unwrap()]);
    assert!(
        defaulted.contains("set x 3"),
        "outside the project the global `full` is the default: {defaulted}"
    );
}

/// The pass count is the profile in force's: a project `aggressive` runs a
/// second pass that removes the store the first pass's folds left unused
/// (O126), and a named `full` over it is one pass.
#[test]
fn opt_runs_the_passes_of_the_profile_in_force() {
    let scratch = Scratch::new("opt-passes");
    let empty = scratch.write("xdg/.keep", "");
    let xdg = empty.parent().expect("xdg").as_os_str();
    let env: &[(&str, &std::ffi::OsStr)] = &[("XDG_CONFIG_HOME", xdg)];
    scratch.write("proj/.tcl-lsp.ini", "[optimiser]\nprofile = aggressive\n");
    let file = scratch.write(
        "proj/chain.tcl",
        "proc p {} {\n    set a [expr {1 + 2}]\n    set b [expr {$a * 2}]\n    return $b\n}\n",
    );
    let file = file.to_str().unwrap();

    let fixpoint = String::from_utf8(run_tcl_env(&["opt", file], env)).unwrap();
    assert!(
        fixpoint.contains("# O126"),
        "the project's `aggressive` runs a second pass: {fixpoint}"
    );
    let single = String::from_utf8(run_tcl_env(&["opt", "--profile", "full", file], env)).unwrap();
    assert!(
        !single.contains("# O126") && single.contains("# O100"),
        "a named `full` is one pass that still folds: {single}"
    );
}
