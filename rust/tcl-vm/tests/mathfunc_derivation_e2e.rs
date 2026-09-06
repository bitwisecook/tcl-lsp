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

//! `tcl::mathfunc::*` and `::tcl::mathop::*` registration is **derived**, not
//! typed (ledger row B3).
//!
//! Before this, the VM hand-registered 37 math-function names and listed 27
//! operator spellings in a `macro_rules!` invocation. The hand-typed function
//! list had gone stale by the whole TIP 745 (Tcl 9.1) C99 batch — 21 names
//! `tcl_syntax::expr::mathfunc::dispatch_with_backend` already implemented,
//! and that `runtime/rust` (which derives its list) already registered, but
//! that the VM never bound as commands. `expr {cbrt(27)}` was
//! `invalid command name "tcl::mathfunc::cbrt"` under every pin, 9.1
//! included.
//!
//! Both lists now come from layer 1 — `mathfunc::all()` and the
//! `mathop_shape` of `expr::operators` — so this file is the drift gate in
//! both directions plus the availability and value evidence.
//!
//! Values are pinned against `tclsh9.1` (9.1b0) and compared against it
//! directly when it is on `PATH`.

use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;

use tcl_compiler::compile_service::BytecodeCompileService;
use tcl_dialect::TclVersion;
use tcl_vm::{CompileService, Vm};

#[derive(Clone, Default)]
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

/// Compile + run `src` with the VM pinned to `version`; return
/// `(ok, stdout)`.
fn run_at(version: TclVersion, src: &str) -> (bool, String) {
    let profile = tcl_registry::model::ingress::resolve_environment(version.dialect_profile_name())
        .analyser_profile();
    let svc = BytecodeCompileService::for_profile(profile);
    let asm = svc.compile(src).expect("compile");
    let cap = Capture::default();
    let mut vm = Vm::with_output(Box::new(cap.clone()));
    vm.set_dialect_profile(profile);
    vm.set_compiler(Box::new(BytecodeCompileService::for_profile(profile)));
    let c = vm.run_module(&asm);
    let out = String::from_utf8_lossy(&cap.0.borrow()).trim().to_string();
    (c.code.is_ok(), out)
}

/// The 21 names the hand-typed registration list was missing — TIP 745's C99
/// batch (plus `signbit`/`trunc`/`logb`/`ldexp`, which arrived with it).
///
/// Every one is `9.1`-only per the shared table, and every one already had a
/// working body in `dispatch_with_backend`: the delta was purely that the VM
/// never registered the command name.
const TIP745_BATCH: &[&str] = &[
    "acosh",
    "asinh",
    "atanh",
    "cbrt",
    "copysign",
    "dim",
    "erf",
    "erfc",
    "exp2",
    "expm1",
    "fma",
    "gamma",
    "ldexp",
    "lgamma",
    "log1p",
    "log2",
    "logb",
    "nextafter",
    "remainder",
    "signbit",
    "trunc",
];

/// Drift gate: every name the shared table lists is registered as a command
/// under a 9.1 pin. A name added to `mathfunc::all()` needs no VM edit — but
/// a regression back to a hand-typed list fails here.
#[test]
fn every_shared_mathfunc_name_is_registered() {
    let names: Vec<&'static str> = tcl_syntax::expr::mathfunc::all()
        .into_iter()
        .map(|spec| spec.name)
        .collect();
    let script = format!(
        "foreach f {{{}}} {{ if {{[llength [info commands ::tcl::mathfunc::$f]] != 1}} \
         {{ puts \"missing $f\" }} }}\nputs done\n",
        names.join(" ")
    );
    let (ok, out) = run_at(TclVersion::V9_1, &script);
    assert!(ok, "must not error: {out}");
    assert_eq!(out, "done", "unregistered math functions");
    // FP guard on the gate itself: the table is not trivially small, and it
    // really does carry the batch the old list missed.
    assert!(names.len() >= 58, "shared table shrank: {}", names.len());
    for name in TIP745_BATCH {
        assert!(names.contains(name), "{name} left the shared table");
    }
}

/// Drift gate for the operator half: every spelling with a `mathop_shape` in
/// `expr::operators` is registered under `::tcl::mathop`, in both
/// directions.
#[test]
fn every_shared_mathop_spelling_is_registered() {
    use tcl_syntax::expr::operators::{ALL_BIN_OPS, ALL_UNARY_OPS};

    let mut expected: Vec<&'static str> = ALL_BIN_OPS
        .iter()
        .filter_map(|op| op.spec().mathop_shape.map(|_| op.spec().spelling))
        .collect();
    for op in ALL_UNARY_OPS {
        if op.spec().mathop_shape.is_some() && !expected.contains(&op.spec().spelling) {
            expected.push(op.spec().spelling);
        }
    }
    expected.sort_unstable();

    let (ok, out) = run_at(
        TclVersion::V9_1,
        "foreach c [lsort [info commands ::tcl::mathop::*]] \
           { puts -nonewline \"[namespace tail $c] \" }\nputs {}\n",
    );
    assert!(ok, "must not error: {out}");
    let mut actual: Vec<&str> = out.split_whitespace().collect();
    actual.sort_unstable();
    assert_eq!(actual, expected, "registered mathop spellings");

    // TP: each one actually dispatches, rather than merely existing — the
    // single fn pointer reads the op off the invoked word, so a name that
    // registered but did not route would fold to the wrong operator.
    let (ok, out) = run_at(
        TclVersion::V9_1,
        "puts [list [::tcl::mathop::+ 1 2 3] [::tcl::mathop::- 10 4] \
         [::tcl::mathop::** 2 10] [::tcl::mathop::eq a a] [::tcl::mathop::! 0] \
         [::tcl::mathop::~ 0] [::tcl::mathop::in b {a b}] [::tcl::mathop::<< 1 4]]\n",
    );
    assert!(ok, "must not error: {out}");
    // tclsh9.1: 6 6 1024 1 1 -1 1 16
    assert_eq!(out, "6 6 1024 1 1 -1 1 16");
}

/// Availability flows through `RuntimeExprSurface`: a 9.1-only function does
/// not resolve under an 8.4/8.5/8.6/9.0 pin and does under 9.1 — with the
/// *release's own* wording for an unresolvable function, which differs
/// across the TIP 232 boundary and which the VM reproduces byte-for-byte:
///
/// ```text
/// tclsh8.4 % set f acosh; catch {expr "${f}(0.5)"} m; set m
/// unknown math function "acosh"
/// tclsh8.5 % … → invalid command name "tcl::mathfunc::acosh"
/// tclsh8.6 % … → invalid command name "tcl::mathfunc::acosh"
/// tclsh9.0 % … → invalid command name "tcl::mathfunc::acosh"
/// ```
///
/// 8.4's `expr` has a fixed C function table rather than an overridable
/// `::tcl::mathfunc` namespace, so an unknown name never becomes a command
/// lookup there.
///
/// The probe is the *message*, not the completion code: several of these
/// functions have restricted domains (`acosh(0.5)` is a domain error even on
/// 9.1) or take two or three operands, so a uniform one-operand call still
/// fails on 9.1 — just not for want of the function.
#[test]
fn tip745_functions_are_gated_to_tcl91() {
    let probe = |pattern: &str| {
        format!(
            "foreach f {{{}}} {{ catch {{expr \"${{f}}(0.5)\"}} m; \
               puts [string match {{{pattern}}} $m] }}\n",
            TIP745_BATCH.join(" ")
        )
    };
    for (version, pattern) in [
        (TclVersion::V8_4, "unknown math function*"),
        (TclVersion::V8_5, "invalid command name*"),
        (TclVersion::V8_6, "invalid command name*"),
        (TclVersion::V9_0, "invalid command name*"),
    ] {
        let (ok, out) = run_at(version, &probe(pattern));
        assert!(ok, "[{version:?}] must not error: {out}");
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), TIP745_BATCH.len(), "[{version:?}] {out}");
        for (name, got) in TIP745_BATCH.iter().zip(&lines) {
            assert_eq!(*got, "1", "[{version:?}] {name} must not resolve");
        }
    }

    // FN guard: under 9.1 none of them produces an unresolvable-function
    // message, even where the call itself is a domain or arity error.
    for pattern in ["unknown math function*", "invalid command name*"] {
        let (ok, out) = run_at(TclVersion::V9_1, &probe(pattern));
        assert!(ok, "must not error: {out}");
        for (name, got) in TIP745_BATCH.iter().zip(out.lines()) {
            assert_eq!(got, "0", "[9.1] {name} must resolve ({pattern})");
        }
    }

    // TP on the surrounding messages, byte-pinned to tclsh9.1.
    let (ok, out) = run_at(
        TclVersion::V9_1,
        "catch {expr {cbrt(0.5,1)}} m; puts $m\n\
         catch {expr {acosh(0.5)}} m; puts $m\n\
         catch {expr {nope(1)}} m; puts $m\n",
    );
    assert!(ok, "must not error: {out}");
    let mut lines = out.lines();
    assert_eq!(
        lines.next(),
        Some("too many arguments for math function \"cbrt\"")
    );
    assert_eq!(
        lines.next(),
        Some("domain error: argument not in valid range")
    );
    assert_eq!(
        lines.next(),
        Some("invalid command name \"tcl::mathfunc::nope\"")
    );
}

/// The unresolvable-function wording itself, compared against every real
/// tclsh on `PATH` — the evidence behind the release split above.
#[test]
fn unresolvable_function_wording_matches_real_tclsh_when_available() {
    let script = "set f acosh; catch {expr \"${f}(0.5)\"} m; puts $m\n";
    let mut checked = 0usize;
    for (version, want) in [
        (TclVersion::V8_4, "unknown math function \"acosh\""),
        (
            TclVersion::V8_5,
            "invalid command name \"tcl::mathfunc::acosh\"",
        ),
        (
            TclVersion::V8_6,
            "invalid command name \"tcl::mathfunc::acosh\"",
        ),
        (
            TclVersion::V9_0,
            "invalid command name \"tcl::mathfunc::acosh\"",
        ),
    ] {
        let bin = format!("tclsh{}", version.version_string());
        let env = format!("TCLSH{}", version.version_string().replace('.', ""));
        let Some(out) = tclsh_output(&env, &[&bin], script) else {
            continue;
        };
        assert_eq!(out, want, "[{version:?}] real tclsh wording");
        let (ok, mine) = run_at(version, script);
        assert!(ok, "[{version:?}] must not error: {mine}");
        assert_eq!(mine, want, "[{version:?}] VM wording");
        checked += 1;
    }
    if checked == 0 {
        eprintln!("no system tclsh found — pinned expectations still verified");
    }
}

/// Value vectors for the newly-reachable batch, taken from `tclsh9.1`
/// (9.1b0). `cbrt(27)` is deliberately not among them: that build's libm
/// answers `3.0000000000000004` where the system libm (and Rust's
/// `f64::cbrt`) answer exactly `3.0`, which is a math-library difference,
/// not a Tcl one — `cbrt(8)`/`cbrt(64)` agree everywhere.
const VECTORS: &[(&str, &str)] = &[
    ("cbrt(8)", "2.0"),
    ("cbrt(64)", "4.0"),
    ("erf(0.5)", "0.5204998778130465"),
    ("erfc(0.5)", "0.4795001221869535"),
    ("fma(2,3,4)", "10.0"),
    ("log2(8)", "3.0"),
    ("exp2(10)", "1024.0"),
    ("ldexp(1.5,3)", "12.0"),
    ("copysign(3,-1)", "-3.0"),
    ("dim(5,3)", "2.0"),
    ("dim(3,5)", "0.0"),
    ("signbit(-1.0)", "1"),
    ("signbit(1.0)", "0"),
    ("trunc(-2.7)", "-2.0"),
    ("logb(8.0)", "3.0"),
    ("remainder(5,3)", "-1.0"),
    ("acosh(1.0)", "0.0"),
    ("asinh(0.0)", "0.0"),
    ("atanh(0.0)", "0.0"),
    ("expm1(0.0)", "0.0"),
    ("log1p(0.0)", "0.0"),
    ("gamma(5)", "24.0"),
    ("lgamma(1.0)", "0.0"),
    ("nextafter(1.0,2.0)", "1.0000000000000002"),
];

#[test]
fn tip745_values_match_tclsh91() {
    for (src, want) in VECTORS {
        let (ok, out) = run_at(TclVersion::V9_1, &format!("puts [expr {{{src}}}]\n"));
        assert!(ok, "`expr {{{src}}}` must evaluate: {out}");
        assert_eq!(out, *want, "expr {{{src}}}");
    }
}

/// Byte-compare every vector against a real `tclsh9.1` when one is on
/// `PATH`; skips silently otherwise. This is what keeps `VECTORS` honest.
#[test]
fn tip745_vectors_match_real_tclsh91_when_available() {
    let mut script = String::new();
    for (src, _) in VECTORS {
        use std::fmt::Write as _;
        let _ = writeln!(script, "puts [expr {{{src}}}]");
    }
    let Some(out) = tclsh_output("TCLSH91", &["tclsh9.1"], &script) else {
        eprintln!("no tclsh9.1 found — pinned expectations still verified");
        return;
    };
    let actual: Vec<&str> = out.lines().collect();
    assert_eq!(actual.len(), VECTORS.len(), "line count: {out}");
    for ((src, want), got) in VECTORS.iter().zip(&actual) {
        assert_eq!(got, want, "tclsh9.1 expr {{{src}}}");
    }
}

/// Run `src` under a real tclsh, or `None` when that binary isn't available.
fn tclsh_output(bin_env: &str, names: &[&str], src: &str) -> Option<String> {
    use std::io::Write as _;
    let mut candidates: Vec<String> = Vec::new();
    if let Ok(explicit) = std::env::var(bin_env) {
        candidates.push(explicit);
    }
    candidates.extend(names.iter().map(ToString::to_string));
    for name in candidates {
        let Ok(mut child) = std::process::Command::new(&name)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
        else {
            continue;
        };
        if let Some(stdin) = child.stdin.as_mut() {
            let _ = stdin.write_all(src.as_bytes());
        }
        let Ok(out) = child.wait_with_output() else {
            continue;
        };
        return Some(String::from_utf8_lossy(&out.stdout).trim().to_string());
    }
    None
}
