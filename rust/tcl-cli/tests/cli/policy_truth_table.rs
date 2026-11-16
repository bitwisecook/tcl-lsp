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

//! The diagnostic-policy truth table's rows, run through the built `tcl diag` and `tcl opt`.

use super::*;

/// A row's `tcl-lsp/config.ini` under `xdg`, and `proj/.tcl-lsp.ini` and
/// `proj/<name>.tcl` (the program, or the bytes for an abstaining document)
/// in `scratch` — the two truth-table passes' shared layout
/// (`docs/design/compiler/diagnostic-policy.md` § The truth table). Returns
/// the input file's path and the `XDG_CONFIG_HOME` directory the run
/// resolves the global layer under.
///
/// `xdg` is one directory for every row of a pass, its `config.ini`
/// rewritten per row: the CLI reads the file afresh on each run, and one
/// `XDG_CONFIG_HOME` lets the rows share a reused `tcl` process.
fn truth_table_scratch(
    scratch: &Scratch,
    xdg: &Scratch,
    row: &tcl_lsp_core::diagnostic_policy::truth_table::Row,
) -> (String, std::ffi::OsString) {
    use tcl_lsp_core::diagnostic_policy::truth_table::Row;

    xdg.write("tcl-lsp/config.ini", &Row::ini(row.global));
    let xdg = xdg.0.clone().into_os_string();
    scratch.write("proj/.tcl-lsp.ini", &Row::ini(row.project));
    let rel = format!("proj/{}.tcl", row.name);
    let file = match row.bytes {
        Some(bytes) => scratch.write_bytes(&rel, bytes),
        None => scratch.write(&rel, row.program),
    };
    let file = file.to_str().expect("utf-8 scratch path").to_owned();
    (file, xdg)
}

/// The slot's `--disable` / `--enable` codes as repeated flag pairs,
/// appended to `args`.
fn push_slot_flags(
    args: &mut Vec<String>,
    flags: &tcl_lsp_core::diagnostic_policy::truth_table::SlotFlags,
) {
    for code in &flags.disable {
        args.push("--disable".to_owned());
        args.push(code.clone());
    }
    for code in &flags.enable {
        args.push("--enable".to_owned());
        args.push(code.clone());
    }
}

/// The diagnostic-policy truth table's diagnostics rows, run through the
/// built `tcl diag --show-suppressed`: each row's observed diagnostics and
/// suppressions hold against what `Surface::Cli` expects of it
/// (`docs/design/compiler/diagnostic-policy.md` § The truth table). One
/// spawn per row; not in the smoke tier.
#[test]
fn truth_table_rows_render_through_tcl_diag() {
    use tcl_compiler::analyser::Severity;
    use tcl_compiler::compiler_checks::DiagCode;
    use tcl_lsp_core::diagnostic_policy::truth_table::{
        Observed, ObservedState, ROWS, Surface, check,
    };

    /// The CLI's `severity` label back to the producers' [`Severity`]: the
    /// one lossy step (`"info"` for both `Info` and `Suggestion`) matches no
    /// row, since only `Error` and `Warning` are ever asserted with `ShownAt`.
    fn severity_of(label: &str) -> Option<Severity> {
        match label {
            "error" => Some(Severity::Error),
            "warning" => Some(Severity::Warning),
            "info" => Some(Severity::Info),
            "hint" => Some(Severity::Hint),
            _ => None,
        }
    }

    /// One file's `diagnostics` and `suppressed` rows (from `--json
    /// --show-suppressed`) as observations: `diag`'s lines are already
    /// 1-based, and a gap's `line` is `null`. A not-run row is one
    /// observation per code it lists.
    fn observed_in(file: &serde_json::Value) -> Vec<Observed> {
        let mut observed: Vec<Observed> = file["diagnostics"]
            .as_array()
            .expect("diagnostics array")
            .iter()
            .filter_map(|d| {
                Some(Observed {
                    code: d["code"].as_str()?.parse::<DiagCode>().ok()?,
                    line: d["line"].as_u64().and_then(|n| u32::try_from(n).ok()),
                    state: ObservedState::Shown(d["severity"].as_str().and_then(severity_of)),
                })
            })
            .collect();
        for s in file["suppressed"]
            .as_array()
            .expect("suppressed array (--show-suppressed was given)")
        {
            let reason = s["reason"].as_str().unwrap_or_default().to_owned();
            if let Some(codes) = s["codes"].as_array() {
                let producer = s["producer"].as_str().unwrap_or_default();
                observed.extend(codes.iter().filter_map(|code| {
                    Some(Observed {
                        code: code.as_str()?.parse::<DiagCode>().ok()?,
                        line: None,
                        state: ObservedState::NotRun {
                            producer: producer.to_owned(),
                            reason: reason.clone(),
                        },
                    })
                }));
            } else if let Some(code) = s["code"]
                .as_str()
                .and_then(|code| code.parse::<DiagCode>().ok())
            {
                observed.push(Observed {
                    code,
                    line: s["line"].as_u64().and_then(|n| u32::try_from(n).ok()),
                    state: ObservedState::Suppressed(reason),
                });
            }
        }
        observed
    }

    let mut failures: Vec<String> = Vec::new();
    let xdg = Scratch::new("truth-table-diag-xdg");
    for row in ROWS.iter().filter(|row| row.runs_on(Surface::Cli)) {
        let scratch = Scratch::new("truth-table-diag");
        let (file, xdg) = truth_table_scratch(&scratch, &xdg, row);
        let mut args: Vec<String> = vec![
            "diag".to_owned(),
            "--json".to_owned(),
            "--show-suppressed".to_owned(),
            "--dialect".to_owned(),
            row.dialect.to_owned(),
        ];
        push_slot_flags(&mut args, &row.slot_flags());
        args.push(file);
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

        let stdout = run_tcl_env(&arg_refs, &[("XDG_CONFIG_HOME", xdg.as_os_str())]);
        let report: serde_json::Value = serde_json::from_slice(&stdout).unwrap_or_else(|err| {
            panic!(
                "row `{}`: diag --json ({err}): {}",
                row.name,
                String::from_utf8_lossy(&stdout)
            )
        });
        let file_report = report
            .as_array()
            .and_then(|files| files.first())
            .unwrap_or_else(|| panic!("row `{}`: no file in the report: {report}", row.name));

        if let Err(failure) = check(row, Surface::Cli, &observed_in(file_report)) {
            failures.push(failure);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The truth table's rewrite rows, run through the built `tcl opt`: the
/// fold applies exactly when `Surface::CliRewrite` wants it shown
/// (`docs/design/compiler/diagnostic-policy.md` § The truth table). One
/// spawn per row; not in the smoke tier.
#[test]
fn truth_table_rewrite_rows_render_through_tcl_opt() {
    use tcl_lsp_core::diagnostic_policy::truth_table::{
        Observed, ObservedState, ROWS, Surface, check,
    };

    let mut failures: Vec<String> = Vec::new();
    let xdg = Scratch::new("truth-table-opt-xdg");
    for row in ROWS.iter().filter(|row| row.runs_on(Surface::CliRewrite)) {
        let scratch = Scratch::new("truth-table-opt");
        let (file, xdg) = truth_table_scratch(&scratch, &xdg, row);
        let flags = row.slot_flags();
        let mut args: Vec<String> = vec!["opt".to_owned()];
        if let Some(profile) = &flags.profile {
            args.push("--profile".to_owned());
            args.push(profile.clone());
        }
        push_slot_flags(&mut args, &flags);
        args.push(file);
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();

        let stdout = run_tcl_env(&arg_refs, &[("XDG_CONFIG_HOME", xdg.as_os_str())]);
        let applied = String::from_utf8_lossy(&stdout).contains("set x 3");
        let observed: Vec<Observed> = row
            .expected(Surface::CliRewrite)
            .iter()
            .map(|expect| Observed {
                code: expect.code,
                line: expect.line,
                state: ObservedState::Applied(applied),
            })
            .collect();
        if let Err(failure) = check(row, Surface::CliRewrite, &observed) {
            failures.push(failure);
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
