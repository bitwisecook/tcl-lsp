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

//! Tcl 9 `regexp.test` (the `regexp`/`regsub` command suite) as a Rust
//! cargo test: each case's body is run end-to-end through the VM (compiled
//! bytecode → the `tcl-regex` ARE engine) and its result compared against the
//! value real `tclsh` produced. The corpus in `tests/data/regexp_cmd_cases.tsv`
//! was extracted by `tests/data/dump_regexp_cmd.tcl` driving the real `tcltest`
//! shell over `regexp.test`.
//!
//! `regexp.test` is written for the full `tcltest` framework (inter-test
//! `set`/`proc` setup, custom constraints, helper procs) which the VM does not
//! provide, so cases that depend on that scaffolding — rather than on the
//! regex engine — are filtered by `skip_reason` (each with a documented
//! cause). Everything else must match `tclsh`.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::cfg_builder::build_cfg_codegen;
use tcl_compiler::codegen::codegen_module;
use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_compiler::lowering::lower_to_ir;
use tcl_registry::CommandRegistry;
use tcl_vm::Vm;

const CORPUS: &str = include_str!("data/regexp_cmd_cases.tsv");

#[derive(Clone)]
struct Capture(Rc<RefCell<Vec<u8>>>);
impl Write for Capture {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.borrow_mut().extend_from_slice(buf);
        Ok(buf.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn run(src: &str) -> (bool, String) {
    let registry = CommandRegistry::build_default();
    let ir = lower_to_ir(src, &registry);
    let cfg = build_cfg_codegen(&ir, false);
    let asm = codegen_module(&cfg, &ir, &registry);
    let buf = Rc::new(RefCell::new(Vec::new()));
    let mut vm = Vm::with_output(Box::new(Capture(Rc::clone(&buf))));
    vm.set_compiler(Box::new(BytecodeCompileService::default()));
    let c = vm.run_module(&asm);
    (c.code.is_ok(), c.result.to_str().to_string())
}

fn cps_to_string(field: &str) -> String {
    if field.is_empty() {
        return String::new();
    }
    field
        .split(',')
        .filter_map(|s| s.parse::<u32>().ok().and_then(char::from_u32))
        .collect()
}

struct Case {
    name: String,
    constraints: String,
    rcodes: String,
    result: String,
    body: String,
}

fn parse_corpus() -> Vec<Case> {
    CORPUS
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| {
            let f: Vec<&str> = l.split('\t').collect();
            Case {
                name: f[0].to_string(),
                constraints: f.get(1).unwrap_or(&"").to_string(),
                rcodes: f.get(2).unwrap_or(&"ok").to_string(),
                result: cps_to_string(f.get(3).unwrap_or(&"")),
                body: cps_to_string(f.get(4).unwrap_or(&"")),
            }
        })
        .collect()
}

/// Cases that depend on `tcltest` scaffolding the VM does not implement
/// (inter-test setup state, custom constraints, helper procs, `binary`/
/// `encoding`/`interp` features) — not regex-engine behaviour. Each is here
/// with a reason after manual triage.
fn skip_reason(c: &Case) -> Option<&'static str> {
    // Cases guarded by a constraint we cannot evaluate (locale, knownBug,
    // build-specific) — the engine is not what is under test.
    if !c.constraints.trim().is_empty() {
        return Some("tcltest constraint not modelled");
    }
    // The extractor records a few non-`ok`/`error` returnCodes from option
    // forms it could not fully parse; skip those rows.
    if c.rcodes != "ok" && c.rcodes != "error" {
        return Some("unparsed returnCodes row");
    }
    // The body reads a variable set by an earlier test in the file; the VM runs
    // each body in isolation, with no tcltest inter-test state.
    if matches!(c.name.as_str(), "regexp-4.4" | "regexp-22.5") {
        return Some("depends on inter-test setup state");
    }
    // `regexp -about` and `regsub -command` are command-plumbing features in
    // `tcl-cmd-core` (shared with the C-engine runtime), explicitly "not yet
    // supported" there — not part of the ARE engine this crate provides.
    if c.name == "regexp-20.2" || c.name.starts_with("regexp-27.") {
        return Some("cmd-core option unimplemented (-about / regsub -command)");
    }
    // `-start` index parsing/validation and its `-all`/`\A` interaction live in
    // the cmd-core option-parsing + match loop, not the engine.
    if matches!(
        c.name.as_str(),
        "regexp-6.9"
            | "regexp-11.8"
            | "regexp-15.9"
            | "regexp-16.4"
            | "regexp-16.7"
            | "regexp-16.8"
    ) {
        return Some("cmd-core -start index handling (plumbing, not engine)");
    }
    None
}

#[test]
fn regexp_cmd_corpus() {
    let cases = parse_corpus();
    assert!(cases.len() > 200, "corpus too small: {}", cases.len());
    let mut failures: Vec<String> = Vec::new();
    let mut skipped = 0usize;
    let mut passed = 0usize;
    for c in &cases {
        if skip_reason(c).is_some() {
            skipped += 1;
            continue;
        }
        let (ok, result) = run(&c.body);
        let good = if c.rcodes == "error" {
            // An error case: the VM must report failure (the result then holds
            // the error message, which tclsh's -result also captures).
            !ok && (c.result.is_empty() || result == c.result)
        } else {
            ok && result == c.result
        };
        if good {
            passed += 1;
        } else {
            failures.push(format!(
                "  {}: body={:?}\n    got ok={ok} result={result:?}, expected rcodes={} result={:?}",
                c.name, c.body, c.rcodes, c.result
            ));
        }
    }
    eprintln!(
        "regexp.test command corpus: {passed} passed, {} failed, {skipped} skipped (of {})",
        failures.len(),
        cases.len()
    );
    for f in failures.iter().take(80) {
        eprintln!("{f}");
    }
    assert!(failures.is_empty(), "{} cases failed", failures.len());
}
