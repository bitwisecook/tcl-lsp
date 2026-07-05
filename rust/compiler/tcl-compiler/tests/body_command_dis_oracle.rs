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

//! Disassembly regression + C-Tcl-`dis` oracle for the opaque body commands
//! generalised in this work: `apply`, `namespace eval`, and `array for`.
//!
//! C Tcl compiles each of these to a runtime call with the script body pushed
//! as a single **unparsed literal** — it never compiles the body's inner
//! commands. (Contrast `dict for`, which C Tcl *does* compile inline — so it is
//! deliberately not in this list.) Our static analysis now reaches inside these
//! bodies (via `body_units`), but that is **analysis-only**: codegen must keep
//! emitting the opaque call with the body as one literal, byte-for-byte as
//! before, so runtime behaviour matches C Tcl.
//!
//! The invariant is locked two ways:
//!   1. Always: our rendered disassembly keeps the whole body as one contiguous
//!      literal and emits exactly one runtime `invokeStk` (no inline body ops).
//!   2. With `tclsh9.0` on `PATH`: C Tcl's own `tcl::unsupported::disassemble`
//!      is confirmed to push the same body as a literal — grounding the
//!      invariant in C Tcl rather than only asserting it.
//!
//! Bodies are kept quote-free so the disassembly's literal escaping does not
//! complicate the substring match.

use std::io::Write;
use std::process::{Command, Stdio};

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::codegen::format::format_module_asm;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;

/// One body-command case. `body_fragment` is a contiguous, quote-free slice of
/// the body that only survives intact if the body is pushed as a single literal
/// (an inline-compiled body would split it across instructions). `dis_setup`
/// runs before the command inside the oracle proc so referenced vars exist.
struct Case {
    name: &'static str,
    source: &'static str,
    body_fragment: &'static str,
    dis_setup: &'static str,
}

const CASES: &[Case] = &[
    Case {
        name: "apply",
        source: "apply {{s} { lappend out $s }} foo\n",
        body_fragment: "lappend out $s",
        dis_setup: "",
    },
    Case {
        name: "namespace eval",
        source: "namespace eval ::ns { variable counter 0 }\n",
        body_fragment: "variable counter 0",
        dis_setup: "",
    },
    Case {
        name: "array for",
        source: "array for {k v} a { lappend out $v }\n",
        body_fragment: "lappend out $v",
        dis_setup: "array set a {x 1}\n    ",
    },
];

fn rust_disasm(source: &str) -> String {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(source, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);
    format_module_asm(&asm)
}

#[test]
fn our_codegen_keeps_body_commands_opaque() {
    for case in CASES {
        let dis = rust_disasm(case.source);

        // The body survives as one contiguous literal — it was not compiled.
        assert!(
            dis.contains(case.body_fragment),
            "{}: body `{}` not present as a contiguous literal (inline-compiled?):\n{dis}",
            case.name,
            case.body_fragment
        );

        // Exactly one runtime invoke — the outer command (an `invokeStk` for
        // the resolved-ensemble / plain form, or an `invokeReplace` for the
        // ensemble-rewrite form C Tcl uses for `namespace eval`). An
        // inline-compiled body would add opcodes / invokes for its inner
        // commands.
        let invokes = dis
            .lines()
            .filter(|l| {
                let t = l.trim_start();
                t.starts_with('(') && (t.contains("invokeStk") || t.contains("invokeReplace"))
            })
            .count();
        assert_eq!(
            invokes, 1,
            "{}: expected exactly one runtime invoke (opaque body); got {invokes}:\n{dis}",
            case.name
        );
    }
}

// ---- C Tcl `dis` oracle ------------------------------------------------------

fn run_tcl(tclsh: &str, script: &str) -> Option<String> {
    let mut child = Command::new(tclsh)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    child.stdin.take()?.write_all(script.as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn find_tclsh9() -> Option<String> {
    for cand in ["tclsh9.0", "tclsh"] {
        if let Some(out) = run_tcl(cand, "puts -nonewline [info patchlevel]")
            && out.starts_with("9.")
        {
            return Some(cand.to_string());
        }
    }
    None
}

#[test]
fn c_tcl_dis_confirms_body_pushed_as_literal() {
    let Some(tclsh) = find_tclsh9() else {
        eprintln!("skipping C-Tcl dis oracle: no tclsh9.0 on PATH");
        return;
    };
    for case in CASES {
        let script = format!(
            "proc p {{}} {{\n    {}{}\n}}\n\
             puts [tcl::unsupported::disassemble proc p]\n",
            case.dis_setup,
            case.source.trim_end()
        );
        let dis = run_tcl(&tclsh, &script).expect("tclsh runs");

        // C Tcl pushes the body as a literal on a `push` instruction — the
        // contiguous fragment appears in that instruction's comment. (For an
        // inline-compiled command like `dict for`, the body would not survive
        // as one literal, so this discriminates opaque from compiled.)
        let on_push = dis.lines().any(|l| {
            let t = l.trim_start();
            t.starts_with('(') && t.contains("push") && t.contains(case.body_fragment)
        });
        assert!(
            on_push,
            "{}: C Tcl did not push the body `{}` as a literal (inline-compiled?):\n{dis}",
            case.name, case.body_fragment
        );
    }
}
