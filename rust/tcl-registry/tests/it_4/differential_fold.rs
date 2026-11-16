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

//! Differential verification of the registry `const_fold` callbacks against
//! real C Tcl.
//!
//! Every fold the optimiser performs for a pure builtin command substitution
//! (the O129 path and the SCCP constant-propagation path) must be
//! byte-identical to what the reference interpreter produces — otherwise a
//! wrong literal gets baked into the source, which surfaces as a
//! false-positive "optimisation" (a behaviour change disguised as a fold).
//!
//! This harness's `fold_builtin_cmd_subst_raw`: it
//! resolves a command through the registry and runs its fold via
//! `run_const_fold` (with the `tcl9.0` dialect, so version-aware folds resolve
//! against the 9.0 reference), runs the *same* command on a real `tclsh9.0`,
//! and asserts the fold — **when it fires** — matches. A miss (`None`) is
//! always acceptable: the optimiser simply leaves the call unfolded, so a
//! missed fold is never wrong. Only a fold that produces the *wrong* value
//! fails the test.
//!
//! It is the Rust-side const-fold-vs-tclsh test. The whole O129 long-tail
//! it was built to pin — the `string is`
//! number classes, the full `format` flag/width/precision matrix, and `scan`
//! — now folds and is verified here.
//!
//! A release with no reference interpreter fails the test where
//! `TCL_LSP_REQUIRE_TCLSH` requires it, and is reported as skipped otherwise
//! ([`tcl_test_support::witness_tclsh`]): a run never passes as if it had
//! compared under a release it did not.

use std::io::Write;
use std::process::{Command, Stdio};

use tcl_registry::CommandRegistry;
use tcl_registry::value_transfer::{PlaceRef, StoreOutcome};

mod captures_and_destructuring;
mod cell_updates;
mod constant_folds;
mod literal_routes;
mod ordered_evaluation;
mod switch_and_case;
mod templates;

/// Run `script` on `tclsh` via stdin, returning `(exit_ok, stdout)`.
fn run_tcl(tclsh: &str, script: &str) -> Option<(bool, String)> {
    let mut child = Command::new(tclsh)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    {
        let mut stdin = child.stdin.take()?;
        stdin.write_all(script.as_bytes()).ok()?;
        // stdin dropped here, closing the pipe so tclsh sees EOF and runs.
    }
    let out = child.wait_with_output().ok()?;
    Some((
        out.status.success(),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    ))
}

/// The reference interpreter for the release `series` (`"8.6"`, `"9.0"`), as
/// the shared oracle lookup finds it ([`tcl_test_support::witness_tclsh`]): a
/// release with none fails the test where `TCL_LSP_REQUIRE_TCLSH` requires
/// it, and is reported as skipped otherwise.
fn find_tclsh(series: &str) -> Option<String> {
    let version = tcl_dialect::TclVersion::from_version_string(series)?;
    tcl_test_support::witness_tclsh(version).map(|tclsh| tclsh.path.to_string_lossy().into_owned())
}

/// The value `tclsh` computes for `[cmd]`, or `None` if tclsh raises (in
/// which case the fold must also decline — we never fold an error to a value).
///
/// A `tclsh` that reads its script from standard input carries on after an
/// error and exits 0, so the call is caught and the exit made to say it.
fn tcl_value(tclsh: &str, cmd: &str) -> Option<String> {
    let script = format!("if {{[catch {{set __v [{cmd}]}}]}} {{exit 1}}\nputs -nonewline $__v");
    match run_tcl(tclsh, &script)? {
        (true, out) => Some(out),
        (false, _) => None,
    }
}

/// Render a command for `tclsh` from the same `(head, sub, args)` the fold
/// sees. Every argument is brace-wrapped, so the literal text tclsh receives
/// for each word is *exactly* the string passed to the fold callback — the
/// brace interior, matching what `literal_words` yields the optimiser. This
/// keeps the differential honest: both sides operate on identical inputs.
fn tcl_command(head: &str, sub: Option<&str>, args: &[&str]) -> String {
    let mut s = head.to_string();
    if let Some(sub) = sub {
        s.push(' ');
        s.push_str(sub);
    }
    for a in args {
        s.push_str(" {");
        s.push_str(a);
        s.push('}');
    }
    s
}

/// A Tcl word whose value is exactly `text`, whatever it holds: double-quoted,
/// with a backslash before every character that would substitute or end the
/// word (`\`, `"`, `$`, `[`, `]`) and before each brace, so a lone `{` cannot
/// leave the script incomplete. Brace-wrapping cannot carry an unbalanced
/// value: `lappend v {{}` never finishes the command, and `tclsh` reading stdin
/// exits quietly without running it. A character beyond ASCII is spelt as a
/// `\uXXXX` escape: `tclsh` reads the piped script in the system encoding,
/// which is not UTF-8 without a locale, and every release reads the escape
/// as the one character it names.
fn tcl_quoted_word(text: &str) -> String {
    use std::fmt::Write as _;
    let mut word = String::with_capacity(text.len() + 2);
    word.push('"');
    for c in text.chars() {
        if matches!(c, '\\' | '"' | '$' | '[' | ']' | '{' | '}') {
            word.push('\\');
        }
        match u32::from(c) {
            code @ 0x80..=0xFFFF => {
                let _ = write!(word, "\\u{code:04x}");
            }
            _ => word.push(c),
        }
    }
    word.push('"');
    word
}

/// Resolve a command through the registry exactly as the optimiser does and
/// run its fold via `run_const_fold` under the release the caller names,
/// returning the **raw** folded value (the same string `tcl_value` compares
/// against — no propagation-word quoting). `None` stands for a profile with no
/// evaluation point, which keeps a version-aware fold to what every release
/// gives; the `tcl9.0` matrices pass `Some(V9_0)`, so version-aware folds resolve
/// against the 9.0 reference.
fn registry_fold_under(
    reg: &CommandRegistry,
    head: &str,
    sub: Option<&str>,
    args: &[&str],
    version: Option<tcl_dialect::TclVersion>,
) -> Option<String> {
    let spec = reg.get(head)?;
    match sub {
        None => spec.run_const_fold(args, version),
        Some(s) => spec.subcommand(s)?.run_const_fold(args, version),
    }
}

/// `(head, subcommand, fold-args)` — the tclsh command is derived from these
/// by `tcl_command`, so the two sides can never drift.
type Case = (&'static str, Option<&'static str>, &'static [&'static str]);

/// Drive a matrix through `tcl_value` and `registry_fold_under`, asserting equality
/// whenever the fold fires. Returns the number of cases that actually folded
/// (so the caller can assert the harness exercised *something*).
fn check_matrix(tclsh: &str, reg: &CommandRegistry, cases: &[Case]) -> usize {
    check_matrix_under(tclsh, reg, cases, Some(tcl_dialect::TclVersion::V9_0))
}

/// The same under the release the caller names, against the `tclsh` of that
/// release.
fn check_matrix_under(
    tclsh: &str,
    reg: &CommandRegistry,
    cases: &[Case],
    version: Option<tcl_dialect::TclVersion>,
) -> usize {
    let mut folded = 0usize;
    for &(head, sub, args) in cases {
        let cmd = tcl_command(head, sub, args);
        let Some(want) = tcl_value(tclsh, &cmd) else {
            continue; // tclsh raised — the fold is allowed (and expected) to bail
        };
        let Some(got) = registry_fold_under(reg, head, sub, args, version) else {
            continue; // a miss is never wrong
        };
        assert_eq!(
            got, want,
            "[{cmd}] folded to {got:?} under {version:?} but {tclsh} gives {want:?}"
        );
        folded += 1;
    }
    folded
}

/// One storage witness: `command ?sub? args…` with `priors` set first,
/// then the command's result and each `observed` place read after it.
type StorageWitness = (
    &'static str,
    Option<&'static str>,
    &'static [&'static str],
    &'static [(&'static str, &'static str)],
    &'static [&'static str],
);

/// What a writing route leaves: the result, then each `observed` place — a
/// write's value, a preserved place's prior, `<unset>` for one never set —
/// or `None` when the route declines.
fn storage_route(
    reg: &CommandRegistry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    (command, sub, args, priors, observed): StorageWitness,
) -> Option<Vec<String>> {
    use tcl_registry::ArgRole;
    use tcl_registry::value_transfer::{
        Budget, EvalAnswer, ExactValue, ExactValueOrUnavailable, LiteralInputs, OperandId,
        StoreOutcome, resolve_semantics,
    };

    let spec = reg.get(command).expect(command);
    let semantics = match sub {
        Some(name) => {
            let sub = spec.subcommand(name).expect(name);
            resolve_semantics(spec, Some(sub), None)
        }
        None => resolve_semantics(spec, None, None),
    };
    let semantics = semantics.semantics().expect("a writing route");
    // The operands in the resolver's coordinates: the subcommand word first.
    let words: Vec<&str> = sub.into_iter().chain(args.iter().copied()).collect();
    let mut inputs = LiteralInputs::new(command, sub, args, profile);
    for (name, value) in priors {
        inputs = inputs.with_prior(name, ExactValue::from_literal(value));
    }
    // The roles the resolver gives the same words: the variables the
    // route's stores must name.
    for index in reg.arg_indices_for_role(command, &words, ArgRole::VarWrite) {
        inputs = inputs.with_role(OperandId(index), ArgRole::VarWrite);
    }
    let EvalAnswer::Evaluated(outcome) = semantics.evaluate(&inputs, &mut Budget::evaluation())
    else {
        return None;
    };
    let ExactValueOrUnavailable::Exact(result) = outcome.result else {
        return None;
    };
    let mut held: Vec<(String, String)> = priors
        .iter()
        .map(|(name, value)| ((*name).to_owned(), (*value).to_owned()))
        .collect();
    for store in outcome.ordered_stores {
        let (name, value) = match store {
            StoreOutcome::Write { target, value } => (words[(target.0).0].to_owned(), value),
            StoreOutcome::WriteElement { target, key, value } => {
                (format!("{}({key})", words[(target.0).0]), value)
            }
            StoreOutcome::Preserve { .. } => continue,
            other => panic!("{command} {args:?}: unexpected store {other:?}"),
        };
        let value = String::from_utf8(value.bytes).expect("text");
        held.retain(|(held_name, _)| *held_name != name);
        held.push((name, value));
    }
    let mut seen = vec![String::from_utf8(result.bytes).expect("text")];
    for name in observed {
        seen.push(
            held.iter()
                .find(|(held_name, _)| held_name == name)
                .map_or_else(|| "<unset>".to_owned(), |(_, value)| value.clone()),
        );
    }
    Some(seen)
}

/// What `tclsh` leaves for the same witness, one line per value, or `None`
/// when the command raises.
fn storage_oracle(
    tclsh: &str,
    (command, sub, args, priors, observed): StorageWitness,
) -> Option<Vec<String>> {
    use std::fmt::Write as _;
    let mut script = String::new();
    for (name, value) in priors {
        let _ = writeln!(script, "set {name} {}", tcl_quoted_word(value));
    }
    // `tclsh` reading a script from standard input carries on past an
    // error and exits 0, so the command's own error ends the run with a
    // failing status.
    let _ = write!(script, "if {{[catch {{{command}");
    for arg in sub.into_iter().chain(args.iter().copied()) {
        script.push(' ');
        script.push_str(&tcl_quoted_word(arg));
    }
    script.push_str("} __result]} {exit 1}\nputs $__result\n");
    let _ = writeln!(
        script,
        "foreach __name {{{}}} {{\n\
         if {{[info exists $__name]}} {{puts [set $__name]}} else {{puts <unset>}}\n\
         }}",
        observed.join(" ")
    );
    match run_tcl(tclsh, &script)? {
        (true, out) => Some(out.lines().map(str::to_owned).collect()),
        (false, _) => None,
    }
}
