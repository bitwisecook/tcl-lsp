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
//! Skips cleanly (the test passes trivially) when no `tclsh9.0` is on `PATH`,
//! the same as the harness's skip contract.

use std::io::Write;
use std::process::{Command, Stdio};

use tcl_registry::CommandRegistry;

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

/// Locate a `tclsh<series>` (or a bare `tclsh` reporting that series'
/// patchlevel) on `PATH` — e.g. `find_tclsh("9.")` for any 9.x,
/// `find_tclsh("8.6")` for the 8.6 reference.
fn find_tclsh(series: &str) -> Option<String> {
    let versioned = format!("tclsh{}", series.trim_end_matches('.'));
    for cand in [versioned.as_str(), "tclsh"] {
        if let Some((true, out)) = run_tcl(cand, "puts -nonewline [info patchlevel]")
            && out.starts_with(series)
        {
            return Some(cand.to_string());
        }
    }
    None
}

/// Locate a `tclsh9.0` (or a `tclsh` reporting a `9.x` patchlevel) on `PATH`.
fn find_tclsh9() -> Option<String> {
    find_tclsh("9.")
}

/// The value `tclsh` computes for `[cmd]`, or `None` if tclsh raises (in
/// which case the fold must also decline — we never fold an error to a value).
fn tcl_value(tclsh: &str, cmd: &str) -> Option<String> {
    match run_tcl(tclsh, &format!("puts -nonewline [{cmd}]"))? {
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
/// exits quietly without running it.
fn tcl_quoted_word(text: &str) -> String {
    let mut word = String::with_capacity(text.len() + 2);
    word.push('"');
    for c in text.chars() {
        if matches!(c, '\\' | '"' | '$' | '[' | ']' | '{' | '}') {
            word.push('\\');
        }
        word.push(c);
    }
    word.push('"');
    word
}

/// Resolve a command through the registry exactly as the optimiser does and
/// run its fold via `run_const_fold`, returning the **raw** folded value (the
/// same string `tcl_value` compares against — no propagation-word quoting).
/// The dialect is `tcl9.0`, so version-aware folds are validated against the
/// `tclsh9.0` reference.
fn registry_fold(
    reg: &CommandRegistry,
    head: &str,
    sub: Option<&str>,
    args: &[&str],
) -> Option<String> {
    let spec = reg.get(head)?;
    match sub {
        None => spec.run_const_fold(args, Some(tcl_dialect::TclVersion::V9_0)),
        Some(s) => spec
            .subcommand(s)?
            .run_const_fold(args, Some(tcl_dialect::TclVersion::V9_0)),
    }
}

/// `(head, subcommand, fold-args)` — the tclsh command is derived from these
/// by `tcl_command`, so the two sides can never drift.
type Case = (&'static str, Option<&'static str>, &'static [&'static str]);

/// Drive a matrix through `tcl_value` and `registry_fold`, asserting equality
/// whenever the fold fires. Returns the number of cases that actually folded
/// (so the caller can assert the harness exercised *something*).
fn check_matrix(tclsh: &str, reg: &CommandRegistry, cases: &[Case]) -> usize {
    let mut folded = 0usize;
    for &(head, sub, args) in cases {
        let cmd = tcl_command(head, sub, args);
        let Some(want) = tcl_value(tclsh, &cmd) else {
            continue; // tclsh raised — the fold is allowed (and expected) to bail
        };
        let Some(got) = registry_fold(reg, head, sub, args) else {
            continue; // a miss is never wrong
        };
        assert_eq!(
            got, want,
            "[{cmd}] folded to {got:?} but tclsh9 gives {want:?}"
        );
        folded += 1;
    }
    folded
}

/// The broad list / string / dict / subst fold matrix. The `string is` number
/// classes (integer / wideinteger / entier / double / dict) fold via the
/// version-aware folder and are pinned here under the `tcl9.0` dialect.
const FOLDABLE: &[Case] = &[
    // list ops
    ("list", None, &["a", "b", "c"]),
    ("list", None, &[]),
    ("list", None, &["a b", "c"]),
    ("llength", None, &["a b c"]),
    ("llength", None, &[""]),
    ("llength", None, &["a {b c} d"]),
    ("lindex", None, &["a b c", "1"]),
    ("lindex", None, &["a b c", "end"]),
    ("lindex", None, &["a b c", "end-1"]),
    ("lrange", None, &["a b c d", "1", "2"]),
    ("lrange", None, &["a b c d", "0", "end"]),
    ("lreverse", None, &["a b c"]),
    ("lreverse", None, &["a b", "c"]),
    ("lrepeat", None, &["3", "x"]),
    ("concat", None, &["a", "b", "c"]),
    ("concat", None, &["a b", "c d"]),
    // Backslash-bearing concat inputs must decline rather than trim
    // incorrectly; this case fails if the registry fold starts returning
    // the old approximate value again.
    ("concat", None, &["a", "b\\ "]),
    ("join", None, &["a b c", "-"]),
    ("join", None, &["a b c"]),
    ("join", None, &["a b c", ":"]),
    ("split", None, &["a,b,c", ","]),
    ("split", None, &["abc", ""]),
    // string ops
    ("string", Some("length"), &["hello"]),
    ("string", Some("index"), &["hello", "1"]),
    ("string", Some("index"), &["hello", "end"]),
    ("string", Some("range"), &["hello", "1", "3"]),
    ("string", Some("reverse"), &["hello"]),
    ("string", Some("toupper"), &["abc"]),
    ("string", Some("tolower"), &["ABC"]),
    ("string", Some("totitle"), &["abc"]),
    ("string", Some("cat"), &["a", "b", "c"]),
    ("string", Some("repeat"), &["ab", "3"]),
    ("string", Some("trim"), &["  x  "]),
    ("string", Some("map"), &["a X b Y", "abab"]),
    ("string", Some("first"), &["l", "hello"]),
    ("string", Some("first"), &["", "hello"]),
    ("string", Some("first"), &["", "hello", "1"]),
    ("string", Some("last"), &["l", "hello"]),
    ("string", Some("last"), &["", "hello"]),
    ("string", Some("last"), &["", "hello", "1"]),
    ("string", Some("equal"), &["abc", "abc"]),
    ("string", Some("equal"), &["abc", "abd"]),
    ("string", Some("compare"), &["a", "b"]),
    // string is — char / boolean / list classes + the version-aware number
    // classes (integer / wideinteger / entier / double / dict, here under 9.0)
    ("string", Some("is"), &["alpha", "abc"]),
    ("string", Some("is"), &["digit", "123"]),
    ("string", Some("is"), &["space", "   "]),
    ("string", Some("is"), &["integer", "42"]),
    ("string", Some("is"), &["integer", "-7"]),
    ("string", Some("is"), &["integer", "4294967295"]),
    ("string", Some("is"), &["integer", "abc"]),
    ("string", Some("is"), &["integer", "1.5"]),
    ("string", Some("is"), &["integer", "9999999999"]), // 9.0 unbounded → 1
    ("string", Some("is"), &["wideinteger", "42"]),
    (
        "string",
        Some("is"),
        &["wideinteger", "9223372036854775808"],
    ), // > 2^63-1 → 0 on 9.0
    (
        "string",
        Some("is"),
        &["entier", "99999999999999999999999999"],
    ),
    ("string", Some("is"), &["dict", "a 1 b 2"]),
    ("string", Some("is"), &["dict", "a 1 b"]),
    ("string", Some("is"), &["double", "3.14"]),
    ("string", Some("is"), &["double", "5.e3"]),
    ("string", Some("is"), &["double", "1E-10"]),
    ("string", Some("is"), &["double", "inf"]),
    ("string", Some("is"), &["double", "nan"]),
    ("string", Some("is"), &["double", "1e"]),
    ("string", Some("is"), &["double", "abc"]),
    ("string", Some("is"), &["double", "-2.5"]),
    // dict ops
    ("dict", Some("get"), &["a 1 b 2", "b"]),
    ("dict", Some("size"), &["a 1 b 2"]),
    ("dict", Some("keys"), &["a 1 b 2"]),
    ("dict", Some("values"), &["a 1 b 2"]),
    ("dict", Some("exists"), &["a 1 b 2", "a"]),
    // subst (literal single-arg form only)
    ("subst", None, &["hello world"]),
    ("subst", None, &["plain"]),
    // scan — inline form, integer / hex / octal / char / string conversions
    ("scan", None, &["42", "%d"]),
    ("scan", None, &["-5", "%d"]),
    ("scan", None, &["ff", "%x"]),
    ("scan", None, &["-ff", "%x"]),
    ("scan", None, &["7fffffff", "%x"]),
    ("scan", None, &["17", "%o"]),
    ("scan", None, &["A", "%c"]),
    ("scan", None, &["abc", "%s"]),
    ("scan", None, &["1 2 3", "%d %d %d"]),
    ("scan", None, &["x42", "x%d"]),
    ("scan", None, &["a(b)", "%s"]),
];

/// `namespace qualifiers` / `namespace tail` — pure string
/// splitting at the last `::`, kept as its own matrix because it is also
/// replayed against 8.6 by [`namespace_string_op_folds_match_tcl86`] to pin
/// the version-invariant (`const_fold`, not `const_fold_versioned`)
/// registration.
const NAMESPACE_STRING_OPS: &[Case] = &[
    ("namespace", Some("qualifiers"), &["::a::b::c"]),
    ("namespace", Some("qualifiers"), &["a::b"]),
    ("namespace", Some("qualifiers"), &["c"]),
    ("namespace", Some("qualifiers"), &[""]),
    ("namespace", Some("qualifiers"), &["::"]),
    ("namespace", Some("qualifiers"), &[":::"]),
    ("namespace", Some("qualifiers"), &["a:::b"]),
    ("namespace", Some("qualifiers"), &["::a::b::"]),
    ("namespace", Some("qualifiers"), &["::x:y"]),
    ("namespace", Some("qualifiers"), &["::foo"]),
    ("namespace", Some("qualifiers"), &["foo::"]),
    ("namespace", Some("qualifiers"), &["::a::b::c::"]),
    ("namespace", Some("qualifiers"), &["a"]),
    ("namespace", Some("qualifiers"), &[":"]),
    ("namespace", Some("qualifiers"), &["::::"]),
    ("namespace", Some("qualifiers"), &["x::y::z"]),
    ("namespace", Some("qualifiers"), &["::ticklecharts::Gauge"]),
    ("namespace", Some("qualifiers"), &["a::b c::d"]),
    ("namespace", Some("qualifiers"), &["a::"]),
    ("namespace", Some("qualifiers"), &["::a"]),
    // A name for a namespace that does not exist splits identically: the
    // subcommands are string operations, never namespace lookups.
    ("namespace", Some("qualifiers"), &["::never::here::x"]),
    ("namespace", Some("tail"), &["::a::b::c"]),
    ("namespace", Some("tail"), &["a::b"]),
    ("namespace", Some("tail"), &["c"]),
    ("namespace", Some("tail"), &[""]),
    ("namespace", Some("tail"), &["::"]),
    ("namespace", Some("tail"), &[":::"]),
    ("namespace", Some("tail"), &["a:::b"]),
    ("namespace", Some("tail"), &["::a::b::"]),
    ("namespace", Some("tail"), &["::x:y"]),
    ("namespace", Some("tail"), &["::foo"]),
    ("namespace", Some("tail"), &["foo::"]),
    ("namespace", Some("tail"), &["::a::b::c::"]),
    ("namespace", Some("tail"), &["a"]),
    ("namespace", Some("tail"), &[":"]),
    ("namespace", Some("tail"), &["::::"]),
    ("namespace", Some("tail"), &["x::y::z"]),
    ("namespace", Some("tail"), &["::ticklecharts::Gauge"]),
    ("namespace", Some("tail"), &["a::b c::d"]),
    ("namespace", Some("tail"), &["a::"]),
    ("namespace", Some("tail"), &["::a"]),
    ("namespace", Some("tail"), &["::never::here::x"]),
];

#[test]
fn registry_folds_match_tcl9() {
    let Some(tclsh) = find_tclsh9() else {
        eprintln!("skipping registry_folds_match_tcl9: no tclsh9.0 on PATH");
        return;
    };
    let reg = CommandRegistry::build_default();
    let folded = check_matrix(&tclsh, &reg, FOLDABLE);
    assert!(
        folded > 0,
        "no FOLDABLE case folded — the differential harness is not wired to the registry"
    );
    let ns_folded = check_matrix(&tclsh, &reg, NAMESPACE_STRING_OPS);
    assert_eq!(
        ns_folded,
        NAMESPACE_STRING_OPS.len(),
        "every `namespace qualifiers`/`tail` row must fold (issue #1096)"
    );
}

/// The same matrix replayed against `tclsh8.6`.  `namespace qualifiers` /
/// `tail` are registered as *version-invariant* folds, which is only sound if
/// 8.6 and 9.0 agree on every row — this is the test that pins it, so a future
/// divergence turns into a failure here rather than a wrong fold under an 8.6
/// dialect.  Skips cleanly when no 8.6 interpreter is installed.
#[test]
fn namespace_string_op_folds_match_tcl86() {
    let Some(tclsh) = find_tclsh("8.6") else {
        eprintln!("skipping namespace_string_op_folds_match_tcl86: no tclsh8.6 on PATH");
        return;
    };
    let reg = CommandRegistry::build_default();
    let folded = check_matrix(&tclsh, &reg, NAMESPACE_STRING_OPS);
    assert_eq!(
        folded,
        NAMESPACE_STRING_OPS.len(),
        "every `namespace qualifiers`/`tail` row must fold identically on 8.6"
    );
}

/// The `format` matrix. The fold now covers the whole
/// conversion set — every integer verb (`%d`/`%i`/`%x`/`%X`/`%o`/`%b`/`%u`,
/// version-aware: leading-zero octal/decimal, 8.x↔9.0 width/wrap), `%c`, the
/// float family (`%f`/`%e`/`%g`), and `%s` — with the full flag/width/precision
/// grid.  Pinned here under `tcl9.0`; the genuinely version-divergent forms a
/// single fold can't be sound for (`%#o`, `%#X`) simply bail (a miss).
const FORMATS: &[Case] = &[
    ("format", None, &["%d", "42"]),
    ("format", None, &["%d", "010"]), // 9.0: leading-zero decimal -> 10
    ("format", None, &["%d", "08"]),  // 9.0: decimal 8 (octal-invalid on 8.x)
    ("format", None, &["%d", "2147483648"]), // 9.0: 32-bit wrap -> -2147483648
    ("format", None, &["%d", "5000000000"]), // 9.0: wrap -> 705032704
    ("format", None, &["%d", "-2147483649"]),
    ("format", None, &["%d", "-7"]),
    ("format", None, &["%5d", "42"]),
    ("format", None, &["%-5d", "42"]),
    ("format", None, &["%05d", "7"]),
    ("format", None, &["%05d", "-7"]),
    ("format", None, &["%+d", "42"]),
    ("format", None, &["%+05d", "42"]),
    ("format", None, &["% d", "42"]),
    ("format", None, &["% d", "-7"]),
    ("format", None, &["%i", "42"]),
    ("format", None, &["%.5d", "42"]),
    ("format", None, &["%.3d", "-4"]),
    ("format", None, &["%5.3d", "42"]),
    ("format", None, &["%05.3d", "42"]),
    ("format", None, &["%.0d", "0"]),
    ("format", None, &["%d", "2147483647"]),
    ("format", None, &["%d", "-2147483648"]),
    ("format", None, &["%x", "255"]),
    ("format", None, &["%x", "-1"]), // 9.0: 32-bit two's complement
    ("format", None, &["%x", "-255"]),
    ("format", None, &["%x", "010"]), // 9.0: decimal 10 -> hex a
    ("format", None, &["%x", "5000000000"]), // 9.0: wrap
    ("format", None, &["%o", "-1"]),
    ("format", None, &["%x", "0"]),
    ("format", None, &["%#x", "255"]),
    ("format", None, &["%#08x", "255"]),
    ("format", None, &["%X", "255"]),
    ("format", None, &["%o", "8"]),
    ("format", None, &["%08x", "255"]),
    ("format", None, &["%-8x", "255"]),
    ("format", None, &["%5x", "255"]),
    ("format", None, &["%.4x", "255"]),
    ("format", None, &["%s", "hi"]),
    ("format", None, &["%10s", "hi"]),
    ("format", None, &["%-10s", "hi"]),
    ("format", None, &["%.3s", "hello"]),
    ("format", None, &["%5.3s", "hello"]),
    ("format", None, &["%.0s", "hi"]),
    ("format", None, &["%c", "65"]),
    ("format", None, &["%5c", "65"]),
    ("format", None, &["%-5c", "65"]),
    // fixed-point float
    ("format", None, &["%f", "3.14"]),
    ("format", None, &["%.2f", "3.14159"]),
    ("format", None, &["%.0f", "2.5"]),
    ("format", None, &["%.0f", "3.5"]),
    ("format", None, &["%.2f", "0.125"]),
    ("format", None, &["%f", "42"]),
    ("format", None, &["%8.2f", "3.14159"]),
    ("format", None, &["%-8.2f", "3.14159"]),
    ("format", None, &["%08.3f", "2.5"]),
    ("format", None, &["%+.2f", "3.14"]),
    ("format", None, &["%.2f", "-2.675"]),
    ("format", None, &["%.10f", "0.1"]),
    // scientific / general float
    ("format", None, &["%e", "31400.0"]),
    ("format", None, &["%.3e", "31400.0"]),
    ("format", None, &["%E", "31400.0"]),
    ("format", None, &["%e", "0.0314"]),
    ("format", None, &["%.2e", "9.999"]),
    ("format", None, &["%e", "1e100"]),
    ("format", None, &["%015.3e", "31400.0"]),
    ("format", None, &["%g", "3.14"]),
    ("format", None, &["%g", "1000000.0"]),
    ("format", None, &["%g", "100000.0"]),
    ("format", None, &["%g", "0.00001"]),
    ("format", None, &["%g", "1.0"]),
    ("format", None, &["%G", "1234567.0"]),
    ("format", None, &["%.3g", "0.000123456"]),
    ("format", None, &["%u", "42"]),
    ("format", None, &["%u", "-1"]),         // 9.0: u32 -> 4294967295
    ("format", None, &["%u", "4294967296"]), // 9.0: wrap -> 0
    ("format", None, &["%u", "010"]),        // 9.0: decimal 10
    ("format", None, &["%05u", "42"]),
    ("format", None, &["%u", "2147483648"]),
    ("format", None, &["%b", "5"]),
    ("format", None, &["%b", "255"]),
    ("format", None, &["%08b", "5"]),
    ("format", None, &["%#b", "5"]),
    ("format", None, &["%c", "65"]),
    ("format", None, &["%5c", "65"]),
    ("format", None, &["%%"]),
];

#[test]
fn format_folds_match_tcl9() {
    let Some(tclsh) = find_tclsh9() else {
        eprintln!("skipping format_folds_match_tcl9: no tclsh9.0 on PATH");
        return;
    };
    let reg = CommandRegistry::build_default();
    // No `folded > 0` floor here: the matrix mixes folds with a few cases that
    // intentionally bail (e.g. `%#o`). Most fire, so the harness is exercised;
    // `check_matrix` asserts every fold that fires matches tclsh9.0.
    check_matrix(&tclsh, &reg, FORMATS);
}

/// The route's answer for `command v values…` with `v` holding `prior`,
/// under `profile`: the registry's cell update over literal words.
fn cell_update_route(
    reg: &CommandRegistry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    command: &str,
    prior: &str,
    values: &[&str],
) -> Option<String> {
    use tcl_registry::ArgRole;
    use tcl_registry::value_transfer::{
        Budget, EvalAnswer, ExactValue, ExactValueOrUnavailable, LiteralInputs, OperandId,
        resolve_semantics,
    };

    let spec = reg.get(command).expect(command);
    let semantics = resolve_semantics(spec, None, None);
    let semantics = semantics.semantics().expect("a storage route");
    let mut args = vec!["v"];
    args.extend_from_slice(values);
    let mut inputs = LiteralInputs::new(command, None, &args, profile)
        .with_prior("v", ExactValue::from_literal(prior));
    // The roles the resolver gives the same words, so a route that finds
    // its place by role (`set`'s write and read forms) runs as it does
    // over the lattice.
    for role in [ArgRole::VarWrite, ArgRole::VarRead] {
        for index in reg.arg_indices_for_role(command, &args, role) {
            inputs = inputs.with_role(OperandId(index), role);
        }
    }
    match semantics.evaluate(&inputs, &mut Budget::evaluation()) {
        EvalAnswer::Evaluated(outcome) => match outcome.result {
            ExactValueOrUnavailable::Exact(value) => String::from_utf8(value.bytes).ok(),
            ExactValueOrUnavailable::Unavailable(_) => None,
        },
        EvalAnswer::Pending | EvalAnswer::Declined(_) => None,
    }
}

/// What `tclsh` leaves in `v` after `command v values…` with `v` holding
/// `prior`, each value passed as a word that carries it exactly.
fn cell_update_oracle(tclsh: &str, command: &str, prior: &str, values: &[&str]) -> Option<String> {
    let mut script = format!("set v {}; {command} v", tcl_quoted_word(prior));
    for value in values {
        script.push(' ');
        script.push_str(&tcl_quoted_word(value));
    }
    script.push_str("; puts -nonewline $v");
    match run_tcl(tclsh, &script)? {
        (true, out) => Some(out),
        (false, _) => None,
    }
}

/// `string range` through its route under `version`, against `tclsh`.
fn check_range_witnesses(tclsh: &str, reg: &CommandRegistry, version: tcl_dialect::TclVersion) {
    let range = reg
        .get("string")
        .expect("string")
        .subcommand("range")
        .expect("range");
    let range_cases: &[[&str; 3]] = &[
        ["hello", "1", "3"],
        ["abcdefghijkl", "010", "end"],
        ["abcdefghijkl", "3", "6"],
        ["abcdef", "-2", "2"],
        ["abc", "end-1", "end"],
        ["abc", "3", "1"],
        ["abcdefghijkl", "0x2", "end-010"],
        [" a ", "0", "end"],
    ];
    for case in range_cases {
        let want = tcl_value(tclsh, &tcl_command("string", Some("range"), case));
        let got = range.run_const_fold(case, Some(version));
        match (want, got) {
            (Some(want), Some(got)) => assert_eq!(
                got,
                want,
                "tclsh{}: string range {case:?}",
                version.version_string()
            ),
            (None, Some(got)) => panic!(
                "tclsh{} raises on string range {case:?}, the fold answered {got:?}",
                version.version_string()
            ),
            (_, None) => {}
        }
    }
}

/// `dict <sub> d <words…>` through the keyed update the resolver selects
/// under `profile`, with `d` holding `prior`; `None` for a decline or when
/// the resolver finds no such command.
fn keyed_update_route(
    reg: &CommandRegistry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    sub: &str,
    prior: &str,
    words: &[&str],
) -> Option<String> {
    use tcl_registry::ArgRole;
    use tcl_registry::invocation_words::{InvocationWord, InvocationWords};
    use tcl_registry::value_transfer::{
        Budget, EvalAnswer, ExactValue, ExactValueOrUnavailable, LiteralInputs, OperandId,
    };

    let mut args = vec![sub, "d"];
    args.extend_from_slice(words);
    let literal_words: Vec<InvocationWord<'_>> =
        args.iter().copied().map(InvocationWord::Literal).collect();
    let resolved = reg
        .resolve_structured_invocation(
            InvocationWords::structured(InvocationWord::Literal("dict"), &literal_words),
            reg.own_surface_query(),
        )
        .resolved()?;
    let semantics = resolved.semantics.value.semantics()?;
    let mut inputs = LiteralInputs::new("dict", Some(sub), &args[1..], profile)
        .with_prior("d", ExactValue::from_literal(prior));
    for index in reg.arg_indices_for_role("dict", &args, ArgRole::VarWrite) {
        inputs = inputs.with_role(OperandId(index), ArgRole::VarWrite);
    }
    match semantics.evaluate(&inputs, &mut Budget::evaluation()) {
        EvalAnswer::Evaluated(outcome) => match outcome.result {
            ExactValueOrUnavailable::Exact(value) => String::from_utf8(value.bytes).ok(),
            ExactValueOrUnavailable::Unavailable(_) => None,
        },
        EvalAnswer::Pending | EvalAnswer::Declined(_) => None,
    }
}

/// The keyed updates of `dict`, per release found on `PATH`, against the
/// real `tclsh`: when the route answers it must match, and when `tclsh`
/// raises it must decline. Under 8.4, which has no `dict`, the resolver
/// finds no route at all.
#[test]
fn keyed_update_witnesses_match_every_release_on_path() {
    let mut releases = 0usize;
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        releases += 1;
        let dialect = version.dialect_profile_name();
        let reg = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let profile = tcl_dialect::DialectProfile::find(dialect);
        if version == tcl_dialect::TclVersion::V8_4 {
            assert_eq!(
                keyed_update_route(reg, profile, "set", "", &["a", "1"]),
                None,
                "8.4 has no dict, so the resolver finds no route"
            );
            continue;
        }
        let cases: &[(&str, &str, &[&str])] = &[
            ("set", "", &["a", "1"]),
            ("set", "a 1 b 2", &["a", "3"]),
            ("set", "a {x 1}", &["a", "y", "2"]),
            ("set", "b 2 a 1", &["c", "3"]),
            ("set", " a  1 ", &["b", "2"]),
            ("set", "a 1 a 2", &["b", "3"]),
            ("set", "a 1 b", &["c", "3"]),
            ("set", "a b", &["a", "c", "d"]),
            ("unset", "a 1 b 2", &["a"]),
            ("unset", " a  1 ", &["zz"]),
            ("unset", "a {x 1}", &["a", "x"]),
            ("unset", "a 1", &["x", "y"]),
            ("incr", "k 5", &["k"]),
            ("incr", "k 5", &["k", "-7"]),
            ("incr", "k 010", &["k"]),
            ("incr", "k 5", &["k", "010"]),
            ("incr", "k { 5 }", &["k"]),
            ("incr", "k 9223372036854775807", &["k"]),
            ("incr", "k abc", &["k"]),
            ("incr", "", &["k", "010"]),
            ("incr", "", &["k", "2.5"]),
            ("append", "k foo", &["k", "bar"]),
            ("append", "", &["k"]),
            ("lappend", "", &["k", "a", "b c"]),
            ("lappend", "k v", &["k"]),
            ("lappend", "k \\{", &["k", "v"]),
        ];
        let mut agreed = 0usize;
        for &(sub, prior, words) in cases {
            let mut script = format!("set d {}; dict {sub} d", tcl_quoted_word(prior));
            for word in words {
                script.push(' ');
                script.push_str(&tcl_quoted_word(word));
            }
            script.push_str("; puts -nonewline $d");
            let want = match run_tcl(&tclsh, &script) {
                Some((true, out)) => Some(out),
                _ => None,
            };
            let got = keyed_update_route(reg, profile, sub, prior, words);
            match (want, got) {
                (Some(want), Some(got)) => {
                    assert_eq!(
                        got,
                        want,
                        "tclsh{}: dict {sub} over {prior:?} with {words:?}",
                        version.version_string()
                    );
                    agreed += 1;
                }
                (None, Some(got)) => panic!(
                    "tclsh{} raises on dict {sub} over {prior:?} with {words:?}, the route answered {got:?}",
                    version.version_string()
                ),
                (_, None) => {}
            }
        }
        assert!(
            agreed >= 18,
            "tclsh{}: only {agreed} keyed-update witnesses agreed",
            version.version_string()
        );
    }
    if releases == 0 {
        eprintln!("no tclsh on PATH: the keyed-update witnesses were not exercised");
    }
}

/// The storage-outcome witnesses of the direct route, per release found on
/// `PATH`: `incr`, `append`, and `lappend` through the registry's cell
/// update, `set`'s write and read forms through the cell write, and `string
/// range` through its route, each under the release's profile, against the
/// real `tclsh`. When the route answers it must match;
/// when `tclsh` raises the route must decline; a decline where `tclsh`
/// answers is allowed (an unfolded value is never wrong).
#[test]
fn storage_outcome_witnesses_match_every_release_on_path() {
    let reg = CommandRegistry::build_default();
    let mut releases = 0usize;
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        releases += 1;
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name());
        let cases: &[(&str, &str, &[&str])] = &[
            ("incr", "5", &[]),
            ("incr", "3", &["10"]),
            ("incr", "10", &["-2"]),
            ("incr", "010", &[]),
            ("incr", "1", &[" 5"]),
            ("incr", "9223372036854775807", &[]),
            ("incr", "1_000", &[]),
            ("incr", "0x10", &["1"]),
            ("incr", "abc", &[]),
            ("incr", "1", &["2.5"]),
            ("append", "foo", &["bar", " baz"]),
            ("append", " a ", &["b"]),
            ("append", "", &["x"]),
            ("lappend", "a b", &["c", "d e"]),
            ("lappend", "", &["c"]),
            ("lappend", "{", &["v"]),
            ("lappend", "a", &["{", "b"]),
            ("set", "old", &[" a "]),
            ("set", "old", &["a\\b"]),
            ("set", " 7 ", &[]),
            ("set", "010", &[]),
        ];
        let mut agreed = 0usize;
        let mut commands_agreeing = std::collections::BTreeSet::new();
        for &(command, prior, values) in cases {
            let want = cell_update_oracle(&tclsh, command, prior, values);
            let got = cell_update_route(&reg, profile, command, prior, values);
            match (want, got) {
                (Some(want), Some(got)) => {
                    assert_eq!(
                        got,
                        want,
                        "tclsh{}: {command} over {prior:?} with {values:?}",
                        version.version_string()
                    );
                    agreed += 1;
                    commands_agreeing.insert(command);
                }
                (None, Some(got)) => panic!(
                    "tclsh{} raises on {command} over {prior:?} with {values:?}, the route answered {got:?}",
                    version.version_string()
                ),
                (_, None) => {}
            }
        }
        assert!(
            agreed >= 8,
            "tclsh{}: only {agreed} witnesses agreed",
            version.version_string()
        );
        assert_eq!(
            commands_agreeing.into_iter().collect::<Vec<_>>(),
            ["append", "incr", "lappend", "set"],
            "tclsh{}: every route answers somewhere",
            version.version_string()
        );
        check_range_witnesses(&tclsh, &reg, version);
    }
    if releases == 0 {
        eprintln!("no tclsh on PATH: the storage-outcome witnesses were not exercised");
    }
}
