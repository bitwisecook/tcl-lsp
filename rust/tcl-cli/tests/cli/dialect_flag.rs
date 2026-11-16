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

//! `--dialect`: the canonical ids, pack-declared environments, aliases and the refusal of every other name.

use super::*;

/// `--dialect` takes every selectable environment's canonical id (`jim`, `tk`,
/// a tool shell), an alias (`vivado`, `jimsh`), an editor language id
/// (`tcl-jim`) and a language id that only selects an environment (`tcl-bpf`,
/// `tcl-libero`, `tcl-spec`, `tcl-apl`), and rejects anything else.
#[test]
fn the_dialect_flag_takes_the_registry_names_and_rejects_the_rest() {
    for dialect in [
        "jim",
        "tk",
        "xilinx-eda-tcl",
        "vivado",
        "jimsh",
        "tcl-jim",
        "tcl-bpf",
        "tcl-libero",
        "tcl-spec",
        "tcl-apl",
    ] {
        run_tcl(&["diag", "--dialect", dialect, "--source", "puts hello"]);
    }
    let rejected = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(["diag", "--dialect", "nonsense", "--source", "puts hello"])
        .reused_output()
        .expect("failed to spawn tcl binary");
    assert!(!rejected.status.success(), "an unknown dialect is refused");
    let stderr = String::from_utf8_lossy(&rejected.stderr);
    for name in ["jim", "tk", "tcl9.1", "xilinx-eda-tcl"] {
        assert!(stderr.contains(name), "the list omits `{name}`: {stderr}");
    }
}

/// Every verb that takes `--dialect` validates it through the one resolver,
/// so the verbs that build their own values (`command-info`, `registry-dump`,
/// `help`, `pkg discover`, `diff`) accept a language id that only selects an
/// environment and refuse an unknown name. `help --dialect all` is the filter's
/// own default and names no environment.
#[test]
fn every_dialect_flag_reaches_the_one_resolver() {
    let dialect = "tcl-libero";
    run_tcl(&["command-info", "puts", "--dialect", dialect]);
    run_tcl(&["registry-dump", "--dialect", dialect, "-o", "-"]);
    run_tcl(&["help", "diagnostics", "--dialect", dialect]);
    run_tcl(&[
        "diff",
        "--left-source",
        "puts a",
        "--right-source",
        "puts a",
        "--dialect",
        dialect,
    ]);
    run_tcl(&["help", "diagnostics", "--dialect", "all"]);
    for verb in [
        &["command-info", "puts"][..],
        &["registry-dump"],
        &["help", "diagnostics"],
        &[
            "diff",
            "--left-source",
            "puts a",
            "--right-source",
            "puts a",
        ],
    ] {
        let rejected = Command::new(env!("CARGO_BIN_EXE_tcl"))
            .args(verb)
            .args(["--dialect", "nonsense"])
            .reused_output()
            .expect("failed to spawn tcl binary");
        assert!(!rejected.status.success(), "{verb:?} refuses `nonsense`");
        let stderr = String::from_utf8_lossy(&rejected.stderr);
        assert!(stderr.contains("unknown dialect `nonsense`"), "{stderr}");
    }
}

/// `tcl diag --help` lists the canonical ids with their descriptions although
/// the argument gates no value, and says the list is not exhaustive.
#[test]
fn the_dialect_help_lists_the_canonical_ids() {
    let help = String::from_utf8(run_tcl(&["diag", "--help"])).expect("help is UTF-8");
    for name in ["tcl8.6", "f5-irules", "jim", "tk", "xilinx-eda-tcl"] {
        assert!(help.contains(&format!("- {name}: ")), "{name}: {help}");
    }
    let flowed = help.split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(flowed.contains("discovered `.tclspec` packs"), "{help}");
}

/// The shell completion scripts still offer the canonical ids as `--dialect`
/// values, though the argument accepts any name.
#[test]
fn the_completion_scripts_offer_the_canonical_ids() {
    for shell in ["bash", "zsh", "fish"] {
        let script =
            String::from_utf8(run_tcl(&["completion", shell])).expect("a completion script");
        for name in ["tcl8.6", "f5-irules", "xilinx-eda-tcl"] {
            assert!(script.contains(name), "{shell}: {name}");
        }
    }
}

/// A scratch workspace, removed on drop.
struct Workspace(PathBuf);

impl Workspace {
    pub(super) fn new(tag: &str) -> Self {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("tcl-cli-{tag}-{nanos}"));
        std::fs::create_dir_all(&root).expect("scratch workspace");
        Workspace(root)
    }

    pub(super) fn write(&self, relative: &str, text: &str) -> PathBuf {
        let path = self.0.join(relative);
        std::fs::create_dir_all(path.parent().expect("a parent directory")).expect("directory");
        std::fs::write(&path, text).expect("write file");
        path
    }

    /// `tcl` run in this workspace with the user pack tier pointed at an
    /// empty directory, so the result does not depend on the host's packs.
    pub(super) fn tcl(&self, args: &[&str]) -> Output {
        Command::new(env!("CARGO_BIN_EXE_tcl"))
            .current_dir(&self.0)
            .env("XDG_CONFIG_HOME", absent_config_home())
            .args(args)
            .reused_output()
            .expect("failed to spawn tcl binary")
    }
}

impl Drop for Workspace {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

/// A pack declaring one tool shell on a Tcl 8.6 core.
const MYPACK: &str = "speclib mypack 2.0 {\n\
    environment mypack-shell {\n\
        display_name {Mypack Shell}\n\
        core         tcl 8.6\n\
        alias        mypack\n\
    }\n\
}\n";

/// An environment a workspace pack declares is a `--dialect` value: the
/// argument is validated after the packs are published, not while the command
/// line is parsed.
#[test]
fn diag_accepts_an_environment_a_workspace_pack_declares() {
    let workspace = Workspace::new("pack-dialect");
    workspace.write(".tcl-lsp/mypack.tclspec", MYPACK);
    workspace.write("doc.tcl", "puts hello\n");
    for name in ["mypack-shell", "mypack"] {
        let output = workspace.tcl(&["diag", "--dialect", name, "doc.tcl"]);
        assert!(
            output.status.success(),
            "`--dialect {name}`: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

/// The same name is refused where no pack declares it, and a refusal in the
/// pack's own workspace lists the environment the pack added.
#[test]
fn diag_refuses_a_pack_environment_no_pack_declares() {
    let bare = Workspace::new("no-pack-dialect");
    bare.write("doc.tcl", "puts hello\n");
    let output = bare.tcl(&["diag", "--dialect", "mypack-shell", "doc.tcl"]);
    assert!(!output.status.success(), "an undeclared name is refused");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown dialect `mypack-shell`"),
        "{stderr}"
    );
    assert!(!valid_names(&stderr).contains(&"mypack-shell"), "{stderr}");

    let with_pack = Workspace::new("pack-dialect-listed");
    with_pack.write(".tcl-lsp/mypack.tclspec", MYPACK);
    with_pack.write("doc.tcl", "puts hello\n");
    let output = with_pack.tcl(&["diag", "--dialect", "nonsense", "doc.tcl"]);
    assert!(!output.status.success(), "an unknown name is refused");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(valid_names(&stderr).contains(&"mypack-shell"), "{stderr}");
}

/// The canonical ids an unknown-dialect refusal lists.
fn valid_names(stderr: &str) -> Vec<&str> {
    stderr
        .split("valid names are ")
        .nth(1)
        .and_then(|rest| rest.split(" (").next())
        .expect("the refusal lists the valid names")
        .split(',')
        .map(str::trim)
        .collect()
}

/// Jim's class, static-variable `proc`, `loop` and `sleep` in one document.
const JIM_PROGRAM: &str = concat!(
    "class system {model \"\"}\n",
    "proc {system model} {} {{model \"\"}} {\n",
    "\tif {$model ne \"\"} { return $model }\n",
    "\tif {[catch {set fp [open /etc/model r]}]} {\n",
    "\t\tset model {HD[R]}\n",
    "\t} else {\n",
    "\t\tset model [string trim [read $fp]]\n",
    "\t\tclose $fp\n",
    "\t}\n",
    "\treturn $model\n",
    "}\n",
    "loop i 0 3 { puts $i }\n",
    "sleep 0.1\n",
    "set s [system new]\n",
    "puts [$s model]\n",
);

/// Write `text` to a scratch `doc.tcl`, run `tcl diag --json` over it with
/// `extra` arguments, and return the exit code with the `(severity, code)` rows.
fn diag_severities(tag: &str, text: &str, extra: &[&str]) -> (Option<i32>, Vec<(String, String)>) {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let dir = std::env::temp_dir().join(format!("tcl-cli-dialect-{tag}-{nanos}"));
    std::fs::create_dir_all(&dir).expect("scratch dir");
    let path = dir.join("doc.tcl");
    std::fs::write(&path, text).expect("write document");

    let mut args = vec!["diag", path.to_str().expect("utf-8 path"), "--json"];
    args.extend_from_slice(extra);
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(&args)
        .reused_output()
        .expect("failed to spawn tcl binary");
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|e| {
        panic!(
            "diag JSON: {e}\nstdout: {}\nstderr: {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    let rows = report[0]["diagnostics"]
        .as_array()
        .expect("diagnostics array")
        .iter()
        .map(|d| {
            (
                d["severity"].as_str().expect("severity").to_owned(),
                d["code"].as_str().expect("code").to_owned(),
            )
        })
        .collect();
    std::fs::remove_dir_all(&dir).ok();
    (output.status.code(), rows)
}

/// The rows that make `tcl diag` exit non-zero.
pub(super) fn problems(rows: &[(String, String)]) -> Vec<&(String, String)> {
    rows.iter()
        .filter(|(severity, _)| severity == "error" || severity == "warning")
        .collect()
}

/// One Jim program under the dialect each run selects: clean under Jim, by
/// flag or by a `jimsh` shebang, and refused under Tcl 8.6. The 8.6 row is the
/// control: the clean results come from the dialect and not from a `diag`
/// that reports nothing. There the four-word `proc` is an arity error and
/// `loop` and `sleep` are Jim's commands, disabled in a Tcl document.
#[test]
fn diag_reads_the_jim_program_under_jim_and_refuses_it_under_tcl_8_6() {
    struct Run<'a> {
        label: &'a str,
        text: &'a str,
        arguments: &'a [&'a str],
        exit: i32,
        codes: &'a [&'a str],
    }
    let shebang = format!("#!/usr/bin/env jimsh\n{JIM_PROGRAM}");
    let runs = [
        Run {
            label: "jim-flag",
            text: JIM_PROGRAM,
            arguments: &["--dialect", "jim"],
            exit: 0,
            codes: &[],
        },
        Run {
            label: "jim-shebang",
            text: &shebang,
            arguments: &[],
            exit: 0,
            codes: &[],
        },
        Run {
            label: "jim-tcl86",
            text: JIM_PROGRAM,
            arguments: &["--dialect", "tcl8.6"],
            exit: 1,
            codes: &["E003", "W002"],
        },
    ];
    for run in runs {
        let label = run.label;
        let (code, found) = diag_severities(label, run.text, run.arguments);
        assert_eq!(code, Some(run.exit), "{label}: {found:?}");
        if run.exit == 0 {
            assert_eq!(
                problems(&found),
                Vec::<&(String, String)>::new(),
                "{label}: {found:?}"
            );
        }
        let codes: Vec<&str> = found.iter().map(|(_, code)| code.as_str()).collect();
        for wanted in run.codes {
            assert!(codes.contains(wanted), "{label}: {wanted} in {found:?}");
        }
    }
}

/// A tool shell's alias is accepted as the dialect.
#[test]
fn diag_accepts_the_vivado_alias() {
    let output = Command::new(env!("CARGO_BIN_EXE_tcl"))
        .args(["diag", "--dialect", "vivado", "--source", "puts hello"])
        .reused_output()
        .expect("failed to spawn tcl binary");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// A name that selects no environment fails and lists the twenty-one canonical
/// ids a user can select, and no others.
#[test]
fn diag_lists_exactly_the_selectable_ids_for_an_unknown_dialect() {
    let workspace = Workspace::new("unknown-dialect");
    let output = workspace.tcl(&["diag", "--dialect", "nonsense", "--source", "puts hello"]);
    assert!(!output.status.success(), "an unknown dialect is refused");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let listed: std::collections::BTreeSet<&str> = valid_names(&stderr).into_iter().collect();
    let expected: std::collections::BTreeSet<&str> = [
        "tcl8.4",
        "tcl8.5",
        "tcl8.6",
        "tcl9.0",
        "tcl9.1",
        "f5-irules",
        "f5-iapps",
        "f5-tmsh",
        "f5-bigip",
        "jim",
        "bpf",
        "expect",
        "spectcl",
        "sslictcl",
        "tk",
        "xilinx-eda-tcl",
        "intel-quartus-eda-tcl",
        "mentor-eda-tcl",
        "microchip-libero-eda-tcl",
        "synopsys-eda-tcl",
        "cadence-eda-tcl",
    ]
    .into_iter()
    .collect();
    assert_eq!(expected.len(), 21);
    assert_eq!(listed, expected, "{stderr}");
}
