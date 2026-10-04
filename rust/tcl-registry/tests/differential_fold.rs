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
    let Some(tclsh) = find_tclsh("9.0") else {
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
/// dialect.
#[test]
fn namespace_string_op_folds_match_tcl86() {
    let Some(tclsh) = find_tclsh("8.6") else {
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
    ("format", None, &["%.0d", "0"]), // 8.4: no digit; 8.5 on: `0`
    ("format", None, &["%.0x", "0"]),
    ("format", None, &["%.0o", "0"]),
    ("format", None, &["%.0u", "0"]),
    ("format", None, &["%5.0d", "0"]),
    ("format", None, &["%+.0d", "0"]),
    ("format", None, &["%-3.0d|", "0"]),
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
    let Some(tclsh) = find_tclsh("9.0") else {
        return;
    };
    let reg = CommandRegistry::build_default();
    // No `folded > 0` floor here: the matrix mixes folds with a few cases that
    // intentionally bail (e.g. `%#o`). Most fire, so the harness is exercised;
    // `check_matrix` asserts every fold that fires matches tclsh9.0.
    check_matrix(&tclsh, &reg, FORMATS);
}

/// The version-aware folds under iRules, which embeds an 8.4 fork whose release
/// was measured (`DialectProfile::evaluation_point`), against the real
/// `tclsh8.4`: where both answer they agree, and the rows that depend on the
/// release answer under the point where, with no release, they decline.
#[test]
fn versioned_folds_under_irules_match_tclsh84() {
    const DEPENDENT: &[Case] = &[
        ("string", Some("is"), &["integer", "4294967296"]),
        ("string", Some("is"), &["integer", "4294967295"]),
        ("string", Some("is"), &["integer", "-4294967296"]),
        ("string", Some("is"), &["wideinteger", "4294967296"]),
        ("string", Some("is"), &["entier", "5"]),
        ("format", None, &["%d", "010"]),
        ("format", None, &["%x", "-1"]),
    ];
    let Some(tclsh) = find_tclsh("8.4") else {
        return;
    };
    let reg = CommandRegistry::build_default();
    let point = tcl_dialect::DialectProfile::irules().evaluation_point();
    assert_eq!(point, Some(tcl_dialect::TclVersion::V8_4));

    let folded = check_matrix_under(&tclsh, &reg, FOLDABLE, point)
        + check_matrix_under(&tclsh, &reg, FORMATS, point)
        + check_matrix_under(&tclsh, &reg, DEPENDENT, point);
    assert!(
        folded > 0,
        "no row folded: the harness is not wired to the registry"
    );

    let mut moved = 0usize;
    for &(head, sub, args) in DEPENDENT {
        let under_point = registry_fold_under(&reg, head, sub, args, point);
        let invariant = registry_fold_under(&reg, head, sub, args, None);
        if under_point.is_some() && invariant.is_none() {
            moved += 1;
        }
        assert!(
            invariant.is_none() || invariant == under_point,
            "[{}] the invariant answer {invariant:?} is not the 8.4 one {under_point:?}",
            tcl_command(head, sub, args)
        );
    }
    assert!(
        moved >= 3,
        "the point answers where no release does for only {moved} of the dependent rows"
    );
}

/// A vendor profile whose base nothing measured has no evaluation point, and a
/// version-aware fold under it declines where the releases differ: `expect`
/// models an 8.6 base, and `string is integer 4294967296` is 0 on 8.x and 1 on
/// 9.x.
#[test]
fn an_unmeasured_vendor_base_leaves_a_versioned_fold_to_the_invariant_subset() {
    let reg = CommandRegistry::build_default();
    let expect = tcl_dialect::DialectProfile::find("expect").expect("catalogue profile");
    assert_eq!(expect.runtime_base, Some(tcl_dialect::TclVersion::V8_6));
    assert_eq!(expect.evaluation_point(), None);
    for args in [&["integer", "4294967296"], &["integer", "-4294967296"]] {
        assert_eq!(
            registry_fold_under(
                &reg,
                "string",
                Some("is"),
                args,
                expect.const_fold_version()
            ),
            None,
            "{args:?}: no point, no fold where 8.x and 9.x differ"
        );
    }
    assert_eq!(
        registry_fold_under(
            &reg,
            "string",
            Some("is"),
            &["integer", "4294967295"],
            expect.const_fold_version()
        )
        .as_deref(),
        Some("1"),
        "every release agrees on the largest 32-bit integer"
    );
}

/// `format` on its registry-owned route, per release found on `PATH`,
/// against the real `tclsh`: every case of the format matrix and the
/// release-gated conversions (`%b` from 8.6; `%p`, `%llu` and the `0d` /
/// `0o` prefixes from 9.0). When `tclsh` raises the route must decline, and
/// when the route answers it must print what `tclsh` prints. The iRules
/// profile is held to `tclsh8.4` as well: it is a fork of that release, and the
/// route answers under it as 8.4 does.
#[test]
fn format_witnesses_match_every_release_on_path() {
    use tcl_registry::value_transfer::{
        Budget, EvalAnswer, ExactValueOrUnavailable, LiteralInputs, resolve_semantics,
    };
    const GATED: &[Case] = &[
        ("format", None, &["%b", "5"]),
        ("format", None, &["%p", "255"]),
        ("format", None, &["%llu", "5"]),
        ("format", None, &["%#o", "8"]),
        ("format", None, &["%#d", "5"]),
    ];
    let profiles = tcl_dialect::TclVersion::ALL
        .map(|version| (version, version.dialect_profile_name()))
        .into_iter()
        .chain([(tcl_dialect::TclVersion::V8_4, "f5-irules")]);
    for (version, dialect) in profiles {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let reg = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let profile = tcl_dialect::DialectProfile::find(dialect);
        let semantics = resolve_semantics(reg.get("format").expect("format"), None, None);
        let semantics = semantics.semantics().expect("the format route");
        let mut answered = 0usize;
        for &(head, sub, args) in FORMATS.iter().chain(GATED) {
            let inputs = LiteralInputs::new(head, sub, args, profile);
            let route = match semantics.evaluate(&inputs, &mut Budget::evaluation()) {
                EvalAnswer::Evaluated(outcome) => match outcome.result {
                    ExactValueOrUnavailable::Exact(value) => String::from_utf8(value.bytes).ok(),
                    ExactValueOrUnavailable::Unavailable(_) => None,
                },
                EvalAnswer::Pending | EvalAnswer::Declined(_) => None,
            };
            let oracle = tcl_value(&tclsh, &tcl_command(head, sub, args));
            if let Some(route) = route {
                assert_eq!(
                    Some(route),
                    oracle,
                    "tclsh{} ({dialect}) {args:?}",
                    version.version_string()
                );
                answered += 1;
            }
        }
        assert!(
            answered * 2 > FORMATS.len(),
            "tclsh{} ({dialect}): the route answered only {answered} cases",
            version.version_string()
        );
    }
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
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
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
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
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
            // A first element starting with `#` is brace-quoted from 8.5
            // and bare in 8.4.
            ("lappend", "", &["#", "b"]),
            ("lappend", "#", &["b"]),
            ("lappend", "#x", &["b"]),
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

/// One regexp-owner witness: a [`StorageWitness`] with no subcommand.
type RegexWitness = (
    &'static str,
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

/// The regexp owner's witnesses, each measured identical on tclsh 8.4.20,
/// 8.5.19, 8.6.18, 9.0.4 and 9.1b0 except where noted; every release
/// answers the first five.
const REGEX_WITNESSES: &[RegexWitness] = &[
    (
        "regexp",
        &["(x)(y)", "zz", "a", "b"],
        &[("a", "before"), ("b", "before")],
        &["a", "b"],
    ),
    (
        "regexp",
        &["-inline", "-indices", "(a)(b)?", "ac"],
        &[],
        &[],
    ),
    ("regexp", &["-all", "a*", "xaax"], &[], &[]),
    ("regsub", &["-all", "", "abc", "-"], &[], &[]),
    ("regexp", &["-about", "a"], &[], &[]),
    ("regexp", &["-about", "(?:a)"], &[], &[]),
    ("regexp", &["-about", "a(b)c"], &[], &[]),
    (
        "regexp",
        &["(a)(b)?", "ac", "m", "g1", "g2"],
        &[],
        &["m", "g1", "g2"],
    ),
    (
        "regexp",
        &["-indices", "(a)(b)?", "ac", "m", "g1", "g2"],
        &[],
        &["m", "g1", "g2"],
    ),
    (
        "regexp",
        &["-all", "(a)", "banana", "m", "g"],
        &[("m", "x")],
        &["m", "g"],
    ),
    (
        "regexp",
        &["-all", "z", "abc", "m"],
        &[("m", "keep")],
        &["m"],
    ),
    ("regexp", &["-nocase", "B+", "abBbc", "m"], &[], &["m"]),
    (
        "regexp",
        &["-start", "2", "-inline", ".", "abcdef"],
        &[],
        &[],
    ),
    (
        "regexp",
        &["-start", "010", "-inline", ".", "abcdefghijkl"],
        &[],
        &[],
    ),
    ("regexp", &["-all", "-inline", "a|(b)", "ab"], &[], &[]),
    // The list rendering differs: `#a a` on 8.4, `{#a} a` from 8.5.
    ("regexp", &["-inline", "#(a)", "#a"], &[], &[]),
    ("regexp", &["-inline", "a", "a", "m"], &[], &[]),
    ("regexp", &["(", "x"], &[], &[]),
    ("regsub", &["-all", "a", "banana", "o", "v"], &[], &["v"]),
    ("regsub", &["z", "abc", "X", "v"], &[("v", "old")], &["v"]),
    ("regsub", &["(b)(c)?", "abd", "[\\2|\\1]"], &[], &[]),
    (
        "regsub",
        &["-command", "a", "abc", "string toupper"],
        &[],
        &[],
    ),
];

/// The regexp owner's witnesses, per release found on `PATH`, each
/// under that release's profile against the real `tclsh`: a no-match leaves
/// its match variables as they were, a match writes them — an unmatched
/// subgroup the empty string, or `-1 -1` with `-indices` — `-inline`
/// answers the list, `-all` counts, `-about` answers the pattern's shape,
/// and `regsub` substitutes and writes its variable whether or not anything
/// matched. When the route answers it must match; when `tclsh` raises the
/// route must decline; a decline where `tclsh` answers is allowed (`-start
/// 010` is 8 up to 8.6 and 10 from 9.0, the `-command` callback form), but
/// the first five witnesses answer under every release.
#[test]
fn regexp_witnesses_match_every_release_on_path() {
    // The first five witnesses, which every release answers.
    const REQUIRED: [usize; 5] = [0, 1, 2, 3, 4];
    let reg = CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name());
        let mut agreed = 0usize;
        for (index, &witness) in REGEX_WITNESSES.iter().enumerate() {
            let (command, args, priors, observed) = witness;
            let want = storage_oracle(&tclsh, (command, None, args, priors, observed));
            let got = storage_route(&reg, profile, (command, None, args, priors, observed));
            match (&want, &got) {
                (Some(want), Some(got)) => {
                    assert_eq!(
                        got,
                        want,
                        "tclsh{}: {} {:?}",
                        version.version_string(),
                        witness.0,
                        witness.1
                    );
                    agreed += 1;
                }
                (None, Some(got)) => panic!(
                    "tclsh{} raises on {} {:?}, the route answered {got:?}",
                    version.version_string(),
                    witness.0,
                    witness.1
                ),
                (_, None) => assert!(
                    !REQUIRED.contains(&index),
                    "tclsh{}: the route declined the witness {} {:?}",
                    version.version_string(),
                    witness.0,
                    witness.1
                ),
            }
        }
        assert!(
            agreed >= 18,
            "tclsh{}: only {agreed} witnesses agreed",
            version.version_string()
        );
    }
}

/// The destructuring writers' witnesses: the first four are `scan`'s
/// partial conversion, `lassign`'s repeated variable, `binary scan`'s two
/// fields and `array set`'s two elements, then come the forms each route
/// reads and the ones it declines.
const DESTRUCTURE_WITNESSES: &[StorageWitness] = &[
    (
        "scan",
        None,
        &["12 nope", "%d %d", "a", "b"],
        &[("a", "A"), ("b", "B")],
        &["a", "b"],
    ),
    (
        "lassign",
        None,
        &["first second extra", "a", "a"],
        &[("a", "A")],
        &["a"],
    ),
    (
        "binary",
        Some("scan"),
        &["\u{1}\u{2}", "cc", "a", "b"],
        &[],
        &["a", "b"],
    ),
    (
        "array",
        Some("set"),
        &["arr", "k1 v1 k2 v2"],
        &[],
        &["arr(k1)", "arr(k2)"],
    ),
    ("scan", None, &["12 34", "%d %d"], &[], &[]),
    ("scan", None, &["abc", "%d"], &[], &[]),
    ("scan", None, &["12 ab", "%d %d %s"], &[], &[]),
    (
        "scan",
        None,
        &["", "%d %d", "a", "b"],
        &[("a", "A"), ("b", "B")],
        &["a", "b"],
    ),
    (
        "scan",
        None,
        &["hi x", "%s %c%n", "w", "c", "n"],
        &[],
        &["w", "c", "n"],
    ),
    (
        "scan",
        None,
        &["12345", "%2d%3d", "a", "b"],
        &[],
        &["a", "b"],
    ),
    (
        "scan",
        None,
        &["abc123", "%[a-z]%d", "w", "n"],
        &[],
        &["w", "n"],
    ),
    ("scan", None, &["12 34", "%*d %d", "a"], &[], &["a"]),
    (
        "scan",
        None,
        &["1.5 2", "%f %d", "f", "d"],
        &[],
        &["f", "d"],
    ),
    (
        "scan",
        None,
        &["ff 017", "%x %i", "h", "i"],
        &[],
        &["h", "i"],
    ),
    ("scan", None, &["-0xff", "%x", "h"], &[("h", "H")], &["h"]),
    ("scan", None, &["101", "%b", "a"], &[("a", "A")], &["a"]),
    (
        "scan",
        None,
        &["4294967296", "%d", "a"],
        &[("a", "A")],
        &["a"],
    ),
    (
        "scan",
        None,
        &["12 34", "%2$d %1$d", "a", "b"],
        &[],
        &["a", "b"],
    ),
    ("scan", None, &["12", "%d %d", "a"], &[], &[]),
    ("scan", None, &["-1 12", "%u %u"], &[], &[]),
    ("scan", None, &["-0 -0.0", "%f %f"], &[], &[]),
    ("scan", None, &["-inf 1", "%f %d"], &[], &[]),
    (
        "scan",
        None,
        &["Infinity", "%g", "g"],
        &[("g", "G")],
        &["g"],
    ),
    ("scan", None, &["nan 1.5", "%f %f"], &[], &[]),
    (
        "binary",
        Some("scan"),
        &["\u{1}", "cc", "a", "b"],
        &[("b", "B")],
        &["a", "b"],
    ),
    ("binary", Some("scan"), &["\u{1}\u{2}", "cc", "a"], &[], &[]),
    (
        "binary",
        Some("scan"),
        &["abc", "a2A*", "x", "y"],
        &[],
        &["x", "y"],
    ),
    ("binary", Some("scan"), &["ab  ", "A*", "x"], &[], &["x"]),
    (
        "binary",
        Some("scan"),
        &["abc", "H*b8", "h", "b"],
        &[],
        &["h", "b"],
    ),
    (
        "binary",
        Some("scan"),
        &["\u{1}\u{2}", "c*", "l"],
        &[],
        &["l"],
    ),
    (
        "binary",
        Some("scan"),
        &["\u{1}\u{2}", "c0x1c", "e", "c"],
        &[],
        &["e", "c"],
    ),
    (
        "binary",
        Some("scan"),
        &["abc", "a4", "x"],
        &[("x", "X")],
        &["x"],
    ),
    (
        "binary",
        Some("scan"),
        &["\u{0}\u{0} @", "r", "v"],
        &[],
        &["v"],
    ),
    (
        "binary",
        Some("scan"),
        &["\u{7f}\u{7e}", "su", "v"],
        &[],
        &["v"],
    ),
    ("binary", Some("scan"), &["abc", "Z", "v"], &[], &[]),
    (
        "binary",
        Some("scan"),
        &["\u{0}\u{0}\u{0}\u{0}\u{0}\u{0}\u{f0}\u{7f}", "d", "v"],
        &[],
        &["v"],
    ),
    (
        "binary",
        Some("scan"),
        &[
            "\u{ff}\u{ff}\u{ff}\u{ff}\u{ff}\u{ff}\u{ff}\u{ff}",
            "wuw",
            "u",
            "s",
        ],
        &[],
        &["u", "s"],
    ),
    (
        "binary",
        Some("scan"),
        &["\u{cd}\u{cc}\u{cc}\u{3d}", "f", "v"],
        &[],
        &["v"],
    ),
    (
        "lassign",
        None,
        &["a b", "x", "y", "z"],
        &[("z", "Z")],
        &["x", "y", "z"],
    ),
    ("lassign", None, &["a {b c} d", "x"], &[], &["x"]),
    ("lassign", None, &["x #a b", "y"], &[], &["y"]),
    ("lassign", None, &["a b"], &[], &[]),
    ("lassign", None, &["a {b", "x"], &[], &[]),
    ("array", Some("set"), &["arr", "k 1 k 2"], &[], &["arr(k)"]),
    ("array", Some("set"), &["arr", "k1 v1 k2"], &[], &[]),
    ("array", Some("set"), &["arr", ""], &[], &[]),
];

/// `scan`'s conversion count: a `%n` and a
/// suppressed success are conversions, as C's `nconversions` counts them,
/// so an input that runs out after one is not the underflow. Measured
/// alike on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1b0: `scan {} %n%d
/// n a` is 1 with `n` written 0 and `a` preserved, `scan abc %s%n a n` is
/// 2, the suppressed forms are 0 with `a` preserved, and only `scan {}
/// %*d%d a` is -1. Every row answers on every release.
const SCAN_COUNT_WITNESSES: &[StorageWitness] = &[
    (
        "scan",
        None,
        &["", "%n%d", "n", "a"],
        &[("n", "5"), ("a", "7")],
        &["n", "a"],
    ),
    (
        "scan",
        None,
        &["abc", "%s%n", "a", "n"],
        &[("a", "7"), ("n", "5")],
        &["a", "n"],
    ),
    ("scan", None, &["", "%*n%d", "a"], &[("a", "7")], &["a"]),
    ("scan", None, &["5", "%*d%d", "a"], &[("a", "7")], &["a"]),
    ("scan", None, &["", "%*d%d", "a"], &[("a", "7")], &["a"]),
    ("scan", None, &["", "%n%d"], &[], &[]),
    ("scan", None, &["", "%*n%d"], &[], &[]),
    ("scan", None, &["5", "%*d%d"], &[], &[]),
    ("scan", None, &["", "%*d%d"], &[], &[]),
];

/// The destructuring writers' witnesses, per release found on
/// `PATH`, each under that release's profile against the real `tclsh`
/// (8.5 on for `lassign`, which 8.4 lacks): a converted field writes its
/// variable and one the input did not reach keeps its value, a repeated
/// variable composes, `array set` writes one element per key. When the
/// route answers it must match; when `tclsh` raises the route must
/// decline; a decline where `tclsh` answers is allowed — a 32-bit overflow,
/// a positional conversion, a float or `0x` spelling 8.4 reads another way,
/// a field a release lacks — but the first four and the conversion count's
/// witnesses answer on every release that has the command.
#[test]
fn destructuring_witnesses_match_every_release_on_path() {
    let reg = CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name());
        let mut agreed = 0usize;
        let witnesses = DESTRUCTURE_WITNESSES.iter().chain(SCAN_COUNT_WITNESSES);
        for (index, &witness) in witnesses.enumerate() {
            let want = storage_oracle(&tclsh, witness);
            let got = storage_route(&reg, profile, witness);
            let (command, sub, args, _, _) = witness;
            let answers = index < 4 || index >= DESTRUCTURE_WITNESSES.len();
            match (&want, &got) {
                (Some(want), Some(got)) => {
                    assert_eq!(
                        got,
                        want,
                        "tclsh{}: {command} {sub:?} {args:?}",
                        version.version_string()
                    );
                    agreed += 1;
                }
                (None, Some(got)) => panic!(
                    "tclsh{} raises on {command} {sub:?} {args:?}, the route answered {got:?}",
                    version.version_string()
                ),
                (Some(_), None) => assert!(
                    !answers,
                    "tclsh{}: the route declined the witness {command} {sub:?} {args:?}",
                    version.version_string()
                ),
                (None, None) => {}
            }
        }
        assert!(
            agreed >= 21 + SCAN_COUNT_WITNESSES.len(),
            "tclsh{}: only {agreed} witnesses agreed",
            version.version_string()
        );
    }
}

/// `binary format` witnesses: the format and its arguments. Program (2)
/// first, then the fields every release packs alike, then the spellings and
/// fields the releases part on, which the route declines.
const BINARY_FORMAT_WITNESSES: &[&[&str]] = &[
    &["H*", "414243444546"],
    &["a3 c", "foo", "65"],
    &["A5", "ab"],
    &["a*", "hello"],
    &["a0", "x"],
    &["b8 B8", "10100000", "10100000"],
    &["h2 H3", "4a", "4a5"],
    &["b*", "101"],
    &[
        "c s S i I w W",
        "-1",
        "70000",
        "258",
        "4294967297",
        "1",
        "-2",
        "3",
    ],
    &["c* c2", "1 2 3", "4 5 6"],
    &["c", " 5 "],
    &["c", "+5"],
    &["c", "300"],
    &["f d", "1.5", "0.1"],
    &["d*", "1.5 2 .5 5. 1e3"],
    &["d", "-0.0"],
    &["f", "1e-40"],
    &["c x2 X c", "1", "2"],
    &["@5 c", "1"],
    &["c @0 c", "1", "2"],
    &["c @* c", "1", "2"],
    &["c X* c", "1", "2"],
    &["", ""],
    &["c", "1", "2"],
    &["a4", "\u{ff}"],
    &["c", "010"],
    &["c", "0b101"],
    &["c", "0o17"],
    &["c", "1_0"],
    &["w", "18446744073709551616"],
    &["c* c2", "1 2 010", "4 5 010"],
    &["d", "010"],
    &["d", "-0"],
    &["d", "1e400"],
    &["d", "1e-310"],
    &["d", "0x1p3"],
    &["d", "1.5 2"],
    &["f", "1e40"],
    &["f", "3.4028235677973366e38"],
    &["t n m", "5", "5", "5"],
    &["r R q Q", "1.5", "1.5", "1.5", "1.5"],
    &["iu", "42"],
    &["x*"],
    &["c @ c", "1", "2"],
    &["c", "1 2"],
    &["c4", "1 2 3"],
    &["Z", "1"],
    &["c"],
    &["b4", "102"],
];

/// What the `binary format` route answers for `witness` under `profile`,
/// as the hex of its bytes, or `None` when it declines.
fn binary_format_route(
    reg: &CommandRegistry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    witness: &[&str],
) -> Option<String> {
    use std::fmt::Write as _;
    use tcl_registry::value_transfer::{
        Budget, EvalAnswer, ExactValueOrUnavailable, LiteralInputs, RepresentationEvidence,
        resolve_semantics,
    };
    let spec = reg.get("binary").expect("binary");
    let semantics = resolve_semantics(spec, Some(spec.subcommand("format").expect("format")), None);
    let semantics = semantics.semantics().expect("the binary format route");
    let inputs = LiteralInputs::new("binary", Some("format"), witness, profile);
    let EvalAnswer::Evaluated(outcome) = semantics.evaluate(&inputs, &mut Budget::evaluation())
    else {
        return None;
    };
    let ExactValueOrUnavailable::Exact(value) = outcome.result else {
        return None;
    };
    assert_eq!(
        value.representation,
        RepresentationEvidence::Constructed(tcl_registry::TclType::ByteArray),
        "{witness:?}"
    );
    let text = String::from_utf8(value.bytes).expect("text");
    let mut hex = String::new();
    for c in text.chars() {
        let byte = u8::try_from(u32::from(c)).expect("a byte");
        let _ = write!(hex, "{byte:02x}");
    }
    Some(hex)
}

/// What `tclsh` packs for `witness`, as hex, or `None` when it raises.
fn binary_format_oracle(tclsh: &str, witness: &[&str]) -> Option<String> {
    let mut script = String::from("if {[catch {binary format");
    for word in witness {
        script.push(' ');
        script.push_str(&tcl_quoted_word(word));
    }
    script.push_str(
        "} __packed]} {exit 1}\nbinary scan $__packed H* __hex\nputs -nonewline $__hex\n",
    );
    match run_tcl(tclsh, &script)? {
        (true, out) => Some(out),
        (false, _) => None,
    }
}

/// Program (2) of the interface page and the `binary format` witnesses,
/// per release found on `PATH`, each under that release's profile
/// against the real `tclsh`: when the route answers it packs the bytes
/// `tclsh` packs, a byte array by construction; when `tclsh` raises the
/// route declines; a decline where `tclsh` answers is allowed only past the
/// shared fields — the spellings, fields and ranges the releases part on.
#[test]
fn binary_format_witnesses_match_every_release_on_path() {
    /// The witnesses every release packs alike, which the route must answer.
    const SHARED: usize = 25;
    let reg = CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name());
        for (index, &witness) in BINARY_FORMAT_WITNESSES.iter().enumerate() {
            let want = binary_format_oracle(&tclsh, witness);
            let got = binary_format_route(&reg, profile, witness);
            match (&want, &got) {
                (Some(want), Some(got)) => assert_eq!(
                    got,
                    want,
                    "tclsh{}: binary format {witness:?}",
                    version.version_string()
                ),
                (None, Some(got)) => panic!(
                    "tclsh{} raises on binary format {witness:?}, the route answered {got}",
                    version.version_string()
                ),
                (Some(_), None) => assert!(
                    index >= SHARED
                        || (witness[0] == "a4" && version < tcl_dialect::TclVersion::V9_0),
                    "tclsh{}: the route declined binary format {witness:?}",
                    version.version_string()
                ),
                (None, None) => {}
            }
        }
    }
}

/// `dict with`'s key projection against the real `tclsh`, from 8.5,
/// the release `dict` arrives in: the variables a body sees on entry, beyond
/// the dictionary variable, are the plan's binders — every key of the
/// dictionary, a repeated key once, or of the nested one a key path names —
/// and the page's program answers `done` and leaves `d` as `a 2`.
#[test]
fn dict_with_binds_the_keys_tclsh_binds() {
    use tcl_registry::value_transfer::{
        BinderName, ExactValue, LiteralInputs, PlanAnswer, resolve_semantics,
    };
    const DICTS: &[(&str, &[&str])] = &[
        ("a 1", &[]),
        ("a 1 b 2 a 3", &[]),
        ("", &[]),
        ("x {p 1 q 2} y 3", &["x"]),
        ("x {y {k v}}", &["x", "y"]),
    ];
    let reg = CommandRegistry::build_default();
    let spec = reg.get("dict").expect("dict");
    let semantics = resolve_semantics(spec, Some(spec.subcommand("with").expect("with")), None);
    let semantics = semantics.semantics().expect("the dict with plan");
    for version in tcl_dialect::TclVersion::ALL {
        if version < tcl_dialect::TclVersion::V8_5 {
            continue;
        }
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name());
        for &(dict, path) in DICTS {
            let mut words = vec!["d"];
            words.extend_from_slice(path);
            words.push("return [lsort [info locals]]");
            let inputs = LiteralInputs::new("dict", Some("with"), &words, profile)
                .with_prior("d", ExactValue::from_literal(dict));
            let PlanAnswer::Body { binders, .. } = semantics.structure(&inputs) else {
                panic!("{dict:?} {path:?}: no body plan");
            };
            let mut planned: Vec<String> = binders
                .iter()
                .map(|binder| match &binder.name {
                    BinderName::Declared(name) => name.clone(),
                    BinderName::Operand(id) => words[id.0 - 1].to_owned(),
                })
                .collect();
            planned.sort();
            let mut script = format!(
                "proc p {{}} {{\n    set d {}\n    dict with d",
                tcl_quoted_word(dict)
            );
            for key in path {
                script.push(' ');
                script.push_str(key);
            }
            script.push_str(" {return [lsort [info locals]]}\n}\nputs -nonewline [p]\n");
            let (true, locals) = run_tcl(&tclsh, &script).expect("tclsh runs") else {
                panic!("tclsh{}: {script}", version.version_string());
            };
            let bound: Vec<&str> = locals
                .split(' ')
                .filter(|name| *name != "d" && !name.is_empty())
                .collect();
            assert_eq!(
                planned,
                bound,
                "tclsh{}: dict with over {dict:?} {path:?}",
                version.version_string()
            );
        }
        let program =
            "set d {a 1}\nputs [dict with d {incr a; set result done}]\nputs -nonewline $d\n";
        assert_eq!(
            run_tcl(&tclsh, program),
            Some((true, "done\na 2".to_owned())),
            "tclsh{}",
            version.version_string()
        );
    }
}

/// `subst`'s literal switches and braced template, as the driver presents a
/// call whose words are all written.
struct TemplateInputs<'a> {
    view: tcl_registry::value_transfer::ResolvedInvocationView<'a>,
    context: tcl_registry::value_transfer::AnalysisContext,
}

impl TemplateInputs<'_> {
    /// `subst switches… {template}` under `profile`.
    fn new<'a>(switches: &[&'a str], template: &'a str, profile: &str) -> TemplateInputs<'a> {
        use tcl_registry::value_transfer::{
            AnalysisContext, InvocationLayout, OperandView, ResolvedInvocationView,
        };
        TemplateInputs {
            view: ResolvedInvocationView {
                canonical_command: "subst",
                subcommand: None,
                form: None,
                layout: InvocationLayout::Source,
                operands: switches
                    .iter()
                    .chain(std::iter::once(&template))
                    .map(|&text| OperandView {
                        text,
                        kind: tcl_registry::InvocationWordKind::Literal,
                        role: None,
                    })
                    .collect(),
                argument_offset: 0,
            },
            context: AnalysisContext::detached(tcl_dialect::DialectProfile::find(profile)),
        }
    }
}

impl tcl_registry::value_transfer::AnalysisInputs for TemplateInputs<'_> {
    fn invocation(&self) -> &tcl_registry::value_transfer::ResolvedInvocationView<'_> {
        &self.view
    }

    fn operand(
        &self,
        id: tcl_registry::value_transfer::OperandId,
        _domain: tcl_registry::value_transfer::FactDomain,
    ) -> tcl_registry::value_transfer::FactView {
        use tcl_registry::value_transfer::{DeclineReason, ExactValue, FactView};
        self.view
            .operand(id)
            .map_or(FactView::Top(DeclineReason::NotExact), |operand| {
                FactView::Exact(ExactValue::from_literal(operand.text), None)
            })
    }

    fn place(
        &self,
        _id: tcl_registry::value_transfer::OperandId,
    ) -> Result<tcl_registry::value_transfer::PlaceRef, tcl_registry::value_transfer::DeclineReason>
    {
        Err(tcl_registry::value_transfer::DeclineReason::Unsupported)
    }

    fn variable(
        &self,
        _name: &str,
        _domain: tcl_registry::value_transfer::FactDomain,
    ) -> tcl_registry::value_transfer::FactView {
        tcl_registry::value_transfer::FactView::Top(
            tcl_registry::value_transfer::DeclineReason::NotExact,
        )
    }

    fn prior_store(
        &self,
        _place: &tcl_registry::value_transfer::PlaceRef,
        _domain: tcl_registry::value_transfer::FactDomain,
    ) -> tcl_registry::value_transfer::FactView {
        tcl_registry::value_transfer::FactView::Top(
            tcl_registry::value_transfer::DeclineReason::NotExact,
        )
    }

    fn word_structure(
        &self,
        id: tcl_registry::value_transfer::OperandId,
    ) -> Result<
        tcl_registry::value_transfer::WordStructure,
        tcl_registry::value_transfer::DeclineReason,
    > {
        use tcl_registry::value_transfer::{DeclineReason, WordPart, WordStructure};
        let template = self.view.operand(id).ok_or(DeclineReason::NotExact)?.text;
        let end = u32::try_from(template.len() + 1).expect("a short template");
        Ok(WordStructure {
            braced: true,
            quoted: false,
            parts: vec![WordPart::Literal {
                span: tcl_lexer::Span::new(1, end),
                text: template.to_owned(),
            }],
        })
    }

    fn body(
        &self,
        _id: tcl_registry::value_transfer::OperandId,
    ) -> Result<tcl_registry::value_transfer::BodyRegion, tcl_registry::value_transfer::DeclineReason>
    {
        Err(tcl_registry::value_transfer::DeclineReason::Unsupported)
    }

    fn nested(
        &self,
        _script: &str,
        _state: &mut tcl_registry::value_transfer::EvaluationState,
    ) -> tcl_registry::value_transfer::EvalAnswer {
        tcl_registry::value_transfer::EvalAnswer::Declined(
            tcl_registry::value_transfer::DeclineReason::Unsupported,
        )
    }

    fn math_function(
        &self,
        _name: &str,
    ) -> Result<
        tcl_registry::value_transfer::BindingIdentity,
        tcl_registry::value_transfer::DeclineReason,
    > {
        Err(tcl_registry::value_transfer::DeclineReason::Unsupported)
    }

    fn context(&self) -> &tcl_registry::value_transfer::AnalysisContext {
        &self.context
    }
}

/// The variables every template witness reads, set before it runs.
const TEMPLATE_PRELUDE: &str = "set b 5\nset name N\nset c 1\narray set a {5 five}\n";

/// What `subst switches… {template}` prints after the prelude, or `None`
/// when it raises (a piped script's error leaves the exit status alone).
fn subst_oracle(tclsh: &str, switches: &[&str], template: &str) -> Option<String> {
    let mut script = format!("{TEMPLATE_PRELUDE}if {{[catch {{subst");
    for switch in switches {
        script.push(' ');
        script.push_str(switch);
    }
    script.push_str(" {");
    script.push_str(template);
    script.push_str("}} __out]} {exit 1}\nputs -nonewline $__out\n");
    match run_tcl(tclsh, &script)? {
        (true, out) => Some(out),
        (false, _) => None,
    }
}

/// The template rebuilt from its plan: each escape decoded under
/// `profile`'s grammar, each read of `b` or `name` its prelude value, and
/// each region's script run after the prelude; `None` for any other read,
/// or a region that raises.
fn template_rebuilt(
    tclsh: &str,
    template: &str,
    plan: &tcl_registry::value_transfer::TemplateWordPlan,
    profile: &str,
) -> Option<String> {
    let escapes = tcl_dialect::DialectProfile::find(profile)
        .map(|profile| profile.grammar.escapes)
        .unwrap_or_default();
    // The plan's spans count the opening brace; the template's text does not.
    let content = |span: tcl_lexer::Span| (span.start() as usize - 1, span.end() as usize - 1);
    let mut pieces: Vec<(usize, usize, String)> = Vec::new();
    for &escape in &plan.escapes {
        let (start, end) = content(escape);
        let decoded = tcl_lexer::backslash_subst_in(&template[start..end], escapes);
        pieces.push((start, end, decoded.into_owned()));
    }
    for read in &plan.reads {
        let value = match (read.name.as_str(), &read.element) {
            ("b", None) => "5",
            ("name", None) => "N",
            _ => return None,
        };
        let (start, end) = content(read.span);
        pieces.push((start, end, value.to_owned()));
    }
    for region in &plan.script_regions {
        let script = format!(
            "{TEMPLATE_PRELUDE}if {{[catch {{{}}} __out]}} {{exit 1}}\nputs -nonewline $__out\n",
            region.script.script
        );
        let (true, value) = run_tcl(tclsh, &script)? else {
            return None;
        };
        let (start, end) = content(region.span);
        pieces.push((start, end, value));
    }
    pieces.sort();
    let mut out = String::new();
    let mut at = 0;
    for (start, end, value) in pieces {
        assert!(start >= at, "{template:?}: overlapping pieces in {plan:?}");
        out.push_str(&template[at..start]);
        out.push_str(&value);
        at = end;
    }
    out.push_str(&template[at..]);
    Some(out)
}

/// `subst`'s template-word plan against the real `tclsh`, per
/// release on `PATH` and under that release's profile, over the page's
/// fourteen programs (`docs/design/compiler/value-transfers.md` § *The
/// template-word plan*; the two procedure programs as the templates they
/// substitute, the computed switch as its proven spelling) and four more —
/// an array index, a backslash `-nobackslashes` leaves, and an unclosed
/// bracket with and without `-nocommands`: where `tclsh` raises — the 9.1
/// positive family below 9.1, the two families together everywhere, the
/// unclosed bracket it substitutes — the plan is the command's error; where it answers, the plan's kinds are the ones `tclsh`
/// runs, probed one kind at a time, and the output rebuilt from the plan's
/// escapes, reads and script regions is the output `tclsh` prints.
#[test]
fn template_witnesses_match_every_release_on_path() {
    use tcl_registry::value_transfer::{DeclineReason, PlanAnswer, resolve_semantics};
    const WITNESSES: &[(&[&str], &str)] = &[
        (&["-novariables"], "a$b[set b]"),
        (&["-nocommands"], "a$b[set b]"),
        (&["-novariables"], "x[expr {$b+1}]"),
        (&["-novariables"], "a[string length $b]"),
        (&["-novariables", "-nocommands"], "a$b[set b]\\x41"),
        (&["-nobackslashes"], "a\\tb"),
        (&[], "a\\$b[set b]"),
        (&["-novariables"], "hello $name"),
        (&["-nocommands"], "a$b"),
        (&[], "[set c 2]"),
        (&["-novariables"], "[incr c]"),
        (&["-variables"], "a$b[set b]"),
        (&["-backslashes"], "a$b[set b]\\x41"),
        (&["-nocommands", "-variables"], "a$b"),
        (&["-nocommands"], "$a([set b])"),
        (&["-nobackslashes"], "a\\$b"),
        (&[], "a[set b"),
        (&["-nocommands"], "a[set b"),
    ];
    let reg = CommandRegistry::build_default();
    let semantics = resolve_semantics(reg.get("subst").expect("subst"), None, None);
    let semantics = semantics.semantics().expect("the template plan");
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let profile = version.dialect_profile_name();
        for &(switches, template) in WITNESSES {
            let plan = semantics.structure(&TemplateInputs::new(switches, template, profile));
            let label = format!(
                "tclsh{}: subst {switches:?} {{{template}}}",
                version.version_string()
            );
            let Some(printed) = subst_oracle(&tclsh, switches, template) else {
                assert_eq!(
                    plan,
                    PlanAnswer::Declined(DeclineReason::WrongRepresentation),
                    "{label}: tclsh raises"
                );
                continue;
            };
            let PlanAnswer::TemplateWord(plan) = plan else {
                panic!("{label}: tclsh prints {printed:?}, the plan is {plan:?}");
            };
            let runs = |probe: &str, ran: &str| {
                subst_oracle(&tclsh, switches, probe).as_deref() == Some(ran)
            };
            assert_eq!(
                (
                    plan.kinds.backslashes,
                    plan.kinds.commands,
                    plan.kinds.variables
                ),
                (runs("\\x41", "A"), runs("[set b]", "5"), runs("$b", "5")),
                "{label}: the kinds"
            );
            if let Some(rebuilt) = template_rebuilt(&tclsh, template, &plan, profile) {
                assert_eq!(rebuilt, printed, "{label}: rebuilt from {plan:?}");
            }
        }
    }
}

/// One `switch` witness: the words before the subject, the subject, the
/// arms as `(pattern, whether its body is the fall-through body)`, whether
/// the arms are one clause-list word, and the variables read back after it.
type SwitchWitness = (
    &'static [&'static str],
    &'static str,
    &'static [(&'static str, bool)],
    bool,
    &'static [&'static str],
);

/// The selection witnesses — the first five are ordered patterns, the final
/// default, a `-` arm whose next pattern never matches supplying its body,
/// regexp captures and a malformed regexp — each measured
/// on tclsh 8.4.20, 8.5.19, 8.6.18, 9.0.4 and 9.1b0: the captures and
/// `-nocase` raise `bad option` on 8.4, and a subject spelled like an
/// option is one there.
const SWITCH_WITNESSES: &[SwitchWitness] = &[
    (
        &["-glob", "--"],
        "abc",
        &[("a*", false), ("ab*", false), ("default", false)],
        true,
        &[],
    ),
    (
        &["-exact", "--"],
        "zzz",
        &[("a", false), ("default", false)],
        false,
        &[],
    ),
    (
        &["-glob", "--"],
        "x",
        &[("x", true), ("y", false)],
        true,
        &[],
    ),
    (
        &["-regexp", "-matchvar", "m", "-indexvar", "i", "--"],
        "abc",
        &[("(a)(x)?b", false)],
        true,
        &["m", "i"],
    ),
    (&["-regexp", "--"], "abc", &[("(", false)], true, &[]),
    (
        &["--"],
        "zzz",
        &[("default", false), ("b", false)],
        false,
        &[],
    ),
    (&["-nocase", "--"], "ABC", &[("abc", false)], true, &[]),
    (
        &["-regexp", "-matchvar", "m", "--"],
        "abc",
        &[("x", false), ("default", false)],
        true,
        &["m"],
    ),
    (
        &["-glob"],
        "-x",
        &[("a", false), ("default", false)],
        true,
        &[],
    ),
    (
        &["-glob"],
        "-x",
        &[("a", false), ("default", false)],
        false,
        &[],
    ),
    (
        &["-regexp", "--"],
        "aXb",
        &[("a.b", true), ("zz", false), ("q", false)],
        false,
        &[],
    ),
    (&["-exact"], "b", &[("a", false), ("b", false)], true, &[]),
    (&["-glob", "--"], "a[b", &[("a\\[*", false)], true, &[]),
    (
        &["-regexp", "-nocase", "--"],
        "ABC",
        &[("^a", false)],
        true,
        &[],
    ),
    // The glob grammar's edges, which every release reads alike.
    (
        &["-glob", "--"],
        "m",
        &[("[z-a]", false), ("default", false)],
        true,
        &[],
    ),
    (
        &["-glob", "--"],
        "b",
        &[("[!a]", false), ("default", false)],
        false,
        &[],
    ),
    (
        &["-glob", "--"],
        "[a",
        &[("[a", false), ("default", false)],
        true,
        &[],
    ),
    (
        &["-glob", "--"],
        "a",
        &[("[a", false), ("default", false)],
        true,
        &[],
    ),
];

/// The words `switch` receives for `witness`, each body `body(arm)` or the
/// fall-through body.
fn switch_words(
    (before, subject, arms, list, _): SwitchWitness,
    body: impl Fn(usize) -> String,
) -> Vec<String> {
    let clauses: Vec<String> = arms
        .iter()
        .enumerate()
        .flat_map(|(arm, &(pattern, falls))| {
            [
                pattern.to_owned(),
                if falls { "-".to_owned() } else { body(arm) },
            ]
        })
        .collect();
    let mut words: Vec<String> = before.iter().map(|&word| word.to_owned()).collect();
    words.push(subject.to_owned());
    if list {
        let mut rendered = Vec::new();
        for (index, clause) in clauses.iter().enumerate() {
            if index != 0 {
                rendered.push(b' ');
            }
            tcl_syntax::list::append_list_element(&mut rendered, clause.as_bytes(), index == 0);
        }
        words.push(String::from_utf8(rendered).expect("text"));
    } else {
        words.extend(clauses);
    }
    words
}

/// What `switch`'s selection answers for `witness` under `profile`: the arm
/// whose body runs (`none` when none does), then each observed variable —
/// its written value, or `<unset>` — or `None` when the selection declines.
fn switch_route(
    reg: &CommandRegistry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    witness: SwitchWitness,
) -> Option<Vec<String>> {
    use tcl_registry::value_transfer::{
        Budget, FactDomain, LiteralInputs, StoreOutcome, TransferAnswer, resolve_semantics,
    };
    let semantics = resolve_semantics(reg.get("switch").expect("switch"), None, None);
    let semantics = semantics.semantics().expect("switch's selection contract");
    let words = switch_words(witness, |arm| format!("B{arm}"));
    let args: Vec<&str> = words.iter().map(String::as_str).collect();
    // Each word delimited as the oracle spells it: `-` bare, every other
    // word quoted.
    let inputs = args.iter().enumerate().fold(
        LiteralInputs::new("switch", None, &args, profile),
        |inputs, (index, &word)| {
            let id = tcl_registry::value_transfer::OperandId(index);
            if word == "-" {
                inputs.with_bare(id)
            } else {
                inputs.with_quoted(id)
            }
        },
    );
    let TransferAnswer::Selection(fact) =
        semantics.transfer(FactDomain::Selection, &inputs, &mut Budget::evaluation())
    else {
        return None;
    };
    assert_eq!(fact.selected.len(), 1, "one member");
    let mut seen = vec![fact.bodies[0].map_or_else(|| "none".to_owned(), |arm| arm.to_string())];
    for name in witness.4 {
        let written = fact.writes[0].iter().rev().find_map(|store| match store {
            StoreOutcome::Write { target, value } if args[(target.0).0] == *name => {
                Some(String::from_utf8(value.bytes.clone()).expect("text"))
            }
            _ => None,
        });
        seen.push(written.unwrap_or_else(|| "<unset>".to_owned()));
    }
    Some(seen)
}

/// What `tclsh` answers for the same witness, one line per value, or
/// `None` when the command raises.
fn switch_oracle(tclsh: &str, witness: SwitchWitness) -> Option<Vec<String>> {
    use std::fmt::Write as _;
    let words = switch_words(witness, |arm| format!("set __body {arm}"));
    let mut script = String::from("set __body none\nif {[catch {switch");
    for word in &words {
        script.push(' ');
        // A fall-through body is spelled bare: 9.1b0's byte-compiled
        // `switch` reads only a bare `-` as one (`IsFallthroughToken` in
        // `tclCompCmdsSZ.c` measures the word with its delimiters), where
        // 8.4 to 9.0, and 9.1's own interpreted path, read the value.
        if word == "-" {
            script.push('-');
        } else {
            script.push_str(&tcl_quoted_word(word));
        }
    }
    script.push_str("}]} {exit 1}\nputs $__body\n");
    let _ = writeln!(
        script,
        "foreach __name {{{}}} {{\n\
         if {{[info exists $__name]}} {{puts [set $__name]}} else {{puts <unset>}}\n\
         }}",
        witness.4.join(" ")
    );
    match run_tcl(tclsh, &script)? {
        (true, out) => Some(out.lines().map(str::to_owned).collect()),
        (false, _) => None,
    }
}

/// `switch`'s selection, per release found on `PATH`, each under
/// that release's profile against the real `tclsh`: when the selection
/// answers, the body it names runs and the variables it writes hold what
/// it wrote; when `tclsh` raises, the selection declines; the first five
/// witnesses answer under every release that runs them, the captures from
/// 8.5.
#[test]
fn switch_witnesses_match_every_release_on_path() {
    // The plan's witnesses, which every release that runs them answers.
    const REQUIRED: [usize; 5] = [0, 1, 2, 3, 4];
    let reg = CommandRegistry::build_default();
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let profile = tcl_dialect::DialectProfile::find(version.dialect_profile_name());
        let mut agreed = 0usize;
        for (index, &witness) in SWITCH_WITNESSES.iter().enumerate() {
            let want = switch_oracle(&tclsh, witness);
            let got = switch_route(&reg, profile, witness);
            match (&want, &got) {
                (Some(want), Some(got)) => {
                    assert_eq!(
                        got,
                        want,
                        "tclsh{}: switch {:?}",
                        version.version_string(),
                        switch_words(witness, |arm| format!("B{arm}"))
                    );
                    agreed += 1;
                }
                (None, Some(got)) => panic!(
                    "tclsh{} raises on switch {:?}, the selection answered {got:?}",
                    version.version_string(),
                    switch_words(witness, |arm| format!("B{arm}"))
                ),
                (Some(_), None) => assert!(
                    !REQUIRED.contains(&index),
                    "tclsh{}: the selection declined the witness switch {:?}",
                    version.version_string(),
                    switch_words(witness, |arm| format!("B{arm}"))
                ),
                (None, None) => {}
            }
        }
        // 8.4 raises on the captures and `-nocase`; every other release
        // answers all but the subject spelled like an option, which the
        // plan's layout abstains on.
        assert!(
            agreed >= 7,
            "tclsh{}: only {agreed} witnesses agreed",
            version.version_string()
        );
    }
}

/// Run `script` from a file under `tclsh`, returning `(exit_ok, stdout)`.
/// Unlike [`run_tcl`], whose stdin commands are each compiled, a script
/// file's own top-level commands run on the interpreted path.
fn run_tcl_file(tclsh: &str, script: &str) -> Option<(bool, String)> {
    use std::sync::atomic::{AtomicUsize, Ordering};
    static NEXT: AtomicUsize = AtomicUsize::new(0);
    let path = std::env::temp_dir().join(format!(
        "differential-fold-{}-{}.tcl",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&path, script).ok()?;
    let output = Command::new(tclsh)
        .arg(&path)
        .stderr(Stdio::null())
        .output();
    let _ = std::fs::remove_file(&path);
    let output = output.ok()?;
    Some((
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
    ))
}

/// A fall-through body spelled `-` reads two ways on 9.1b0 when it is
/// quoted: its byte-compiled `switch` — here inside a procedure —
/// recognises only a bare `-` (`IsFallthroughToken` in `tclCompCmdsSZ.c`
/// measures the word with its quotes) and runs a quoted one as a command,
/// where its interpreted path — a script file's own top level — reads the
/// word's value and falls through, as 8.4.20 to 9.0.4 do on both paths.
/// So the selection declines the quoted form under a profile that may be
/// 9.1 (`tcl9.1`, and `tcl`, which names no release) and decides it under
/// `tcl8.6` and `tcl9.0`, the body it names being the one those releases
/// run; the bare form decides under every one of them.
#[test]
fn a_quoted_fallthrough_body_reads_two_ways_on_91() {
    use tcl_registry::value_transfer::{
        Axis, Budget, DeclineReason, FactDomain, LiteralInputs, OperandId, TransferAnswer,
        resolve_semantics,
    };
    let reg = CommandRegistry::build_default();
    let semantics = resolve_semantics(reg.get("switch").expect("switch"), None, None);
    let semantics = semantics.semantics().expect("switch's selection contract");
    let words = ["-glob", "--", "a", "a", "-", "b", "B"];
    let bodies = |dialect: &str, quoted: bool| {
        let profile = tcl_dialect::DialectProfile::find(dialect);
        let inputs = (0..words.len()).fold(
            LiteralInputs::new("switch", None, &words, profile),
            |inputs, index| {
                if index == 4 && !quoted {
                    inputs.with_bare(OperandId(index))
                } else {
                    inputs.with_quoted(OperandId(index))
                }
            },
        );
        match semantics.transfer(FactDomain::Selection, &inputs, &mut Budget::evaluation()) {
            TransferAnswer::Selection(fact) => Ok(fact.bodies),
            TransferAnswer::Declined(reason) => Err(reason),
            other => panic!("{dialect}: {other:?}"),
        }
    };
    let ambiguous = Err(DeclineReason::ReleaseAmbiguous(Axis::Availability(
        tcl_dialect::model::SpecSurface::TCL91[0],
    )));
    for dialect in ["tcl9.1", "tcl"] {
        assert_eq!(bodies(dialect, true), ambiguous, "{dialect}: quoted");
        assert_eq!(bodies(dialect, false), Ok(vec![Some(1)]), "{dialect}: bare");
    }
    for dialect in ["tcl8.6", "tcl9.0"] {
        assert_eq!(
            bodies(dialect, true),
            Ok(vec![Some(1)]),
            "{dialect}: quoted"
        );
        assert_eq!(bodies(dialect, false), Ok(vec![Some(1)]), "{dialect}: bare");
    }

    // The oracle: the arm's body, `B`, or the error of running `-`.
    let in_proc = |body: &str| {
        format!(
            "proc p {{}} {{switch -glob -- a a {body} b {{return B}}}}\n\
             if {{[catch p result]}} {{puts error}} else {{puts $result}}\n"
        )
    };
    let at_top = |body: &str| format!("puts [switch -glob -- a a {body} b {{set _ B}}]\n");
    for (series, quoted_in_proc) in [("8.6", "B"), ("9.0", "B"), ("9.1", "error")] {
        let Some(tclsh) = find_tclsh(series) else {
            continue;
        };
        for (body, in_proc_prints) in [("\"-\"", quoted_in_proc), ("-", "B")] {
            assert_eq!(
                run_tcl_file(&tclsh, &in_proc(body)),
                Some((true, format!("{in_proc_prints}\n"))),
                "tclsh{series}: {body} inside a procedure"
            );
            assert_eq!(
                run_tcl_file(&tclsh, &at_top(body)),
                Some((true, "B\n".to_owned())),
                "tclsh{series}: {body} at the top level"
            );
        }
    }
}

/// One `case` witness: the subject, whether the `in` word follows it, the
/// patterns in order (arm `N`'s body sets `__body` to `N`), whether the
/// clauses are one list word, and whether the final pattern has no body.
type CaseWitness = (&'static str, bool, &'static [&'static str], bool, bool);

/// The `case` witnesses, each measured on tclsh 8.4.20, 8.5.19 and
/// 8.6.18, which answer every one alike: glob patterns, the `in` word and
/// the one-word form, a literal pattern, pattern lists and a backslash that
/// makes a pattern one, `default` wherever it stands and matched literally,
/// the last `default` winning, a list's `default` element no fallback, `-`
/// an ordinary pattern, no match, the missing body raised only when the
/// scan reaches it, and a malformed pattern list raising.
const CASE_WITNESSES: &[CaseWitness] = &[
    ("abc", true, &["a*", "default"], false, false),
    ("zzz", true, &["a*", "default"], false, false),
    ("abc", false, &["a*", "default"], false, false),
    ("abc", false, &["a*", "default"], true, false),
    ("abc", true, &["a*", "default"], true, false),
    ("abc", true, &["abc"], false, false),
    ("abc", true, &["x a*", "default"], false, false),
    ("abc", true, &["a\\*", "default"], false, false),
    ("a*", true, &["a\\*", "default"], false, false),
    ("zzz", true, &["default", "a"], false, false),
    ("default", true, &["default", "def*"], false, false),
    ("abc", true, &["-", "default"], false, false),
    ("abc", true, &["q"], false, false),
    ("abc", true, &["a b"], false, false),
    ("a b", true, &["a b", "default"], false, false),
    ("a b", true, &["{a b}", "default"], false, false),
    ("abc", true, &["", "default"], false, false),
    ("", true, &["", "default"], false, false),
    ("abc", true, &["[a-c]*"], false, false),
    ("abc", true, &["A*", "default"], false, false),
    ("abc", true, &["a*\\", "default"], false, false),
    ("zzz", true, &["default", "default"], false, false),
    ("abc", true, &["default x", "a*"], false, false),
    ("zzz", true, &["default x", "b*"], false, false),
    ("in", true, &["in", "default"], false, false),
    ("abc", false, &["-", "default"], false, false),
    ("abc", true, &["a {"], false, false),
    ("zzz", true, &["a*", "b*"], false, true),
    // The plan abstains on these two, which tclsh answers: the missing
    // body the scan never reaches, and a subject spelled like an option.
    ("abc", true, &["a*", "b*"], false, true),
    ("-x", true, &["-*"], false, false),
];

/// The witnesses the plan abstains on: the last two.
const CASE_ABSTAINS: usize = 2;

/// The words `case` receives for `witness`, each body `body(arm)`.
fn case_words(
    (subject, separated, patterns, list, dangling): CaseWitness,
    body: impl Fn(usize) -> String,
) -> Vec<String> {
    let mut clauses = Vec::new();
    for (arm, &pattern) in patterns.iter().enumerate() {
        clauses.push(pattern.to_owned());
        if !(dangling && arm + 1 == patterns.len()) {
            clauses.push(body(arm));
        }
    }
    let mut words = vec![subject.to_owned()];
    if separated {
        words.push("in".to_owned());
    }
    if list {
        let mut rendered = Vec::new();
        for (index, clause) in clauses.iter().enumerate() {
            if index != 0 {
                rendered.push(b' ');
            }
            tcl_syntax::list::append_list_element(&mut rendered, clause.as_bytes(), index == 0);
        }
        words.push(String::from_utf8(rendered).expect("text"));
    } else {
        words.extend(clauses);
    }
    words
}

/// The arm whose body `case`'s selection runs for `witness` under
/// `profile` (`none` when none does), or `None` when it declines.
fn case_route(
    reg: &CommandRegistry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    witness: CaseWitness,
) -> Option<String> {
    use tcl_registry::value_transfer::{
        Budget, FactDomain, LiteralInputs, TransferAnswer, resolve_semantics,
    };
    let semantics = resolve_semantics(reg.get("case").expect("case"), None, None);
    let semantics = semantics.semantics().expect("case's selection contract");
    let words = case_words(witness, |arm| format!("B{arm}"));
    let args: Vec<&str> = words.iter().map(String::as_str).collect();
    let inputs = LiteralInputs::new("case", None, &args, profile);
    let TransferAnswer::Selection(fact) =
        semantics.transfer(FactDomain::Selection, &inputs, &mut Budget::evaluation())
    else {
        return None;
    };
    assert_eq!(fact.selected.len(), 1, "one member");
    assert_eq!(fact.selected, fact.bodies, "no fall-through body");
    assert!(fact.writes.iter().all(Vec::is_empty), "no writes");
    Some(fact.bodies[0].map_or_else(|| "none".to_owned(), |arm| arm.to_string()))
}

/// What `tclsh` answers for the same witness, or `None` when it raises.
fn case_oracle(tclsh: &str, witness: CaseWitness) -> Option<String> {
    let words = case_words(witness, |arm| format!("set __body {arm}"));
    let mut script = String::from("set __body none\nif {[catch {case");
    for word in &words {
        script.push(' ');
        script.push_str(&tcl_quoted_word(word));
    }
    script.push_str("}]} {exit 1}\nputs -nonewline $__body\n");
    match run_tcl(tclsh, &script)? {
        (true, out) => Some(out),
        (false, _) => None,
    }
}

/// `case`'s selection against the real `tclsh` of each release that
/// has the command — 8.4, 8.5 and 8.6, each under its own profile, and the
/// iRules profile on its 8.4 base (ruling 8): where tclsh answers, the
/// selection names the body tclsh runs, or abstains on the two witnesses
/// the layout its plan does not read; where tclsh raises, the selection
/// declines. From 9.0 there is no `case` to select with.
#[test]
fn case_witnesses_match_every_release_on_path() {
    let reg = CommandRegistry::build_default();
    for (series, dialect) in [
        ("8.4", "tcl8.4"),
        ("8.5", "tcl8.5"),
        ("8.6", "tcl8.6"),
        ("8.4", "f5-irules"),
    ] {
        let Some(tclsh) = find_tclsh(series) else {
            continue;
        };
        let profile = tcl_dialect::DialectProfile::find(dialect);
        assert!(profile.is_some(), "{dialect}");
        for (index, &witness) in CASE_WITNESSES.iter().enumerate() {
            let want = case_oracle(&tclsh, witness);
            let got = case_route(&reg, profile, witness);
            let words = case_words(witness, |arm| format!("B{arm}"));
            let abstains = index >= CASE_WITNESSES.len() - CASE_ABSTAINS;
            match (&want, &got) {
                (Some(want), Some(got)) => {
                    assert_eq!(got, want, "tclsh{series} ({dialect}): case {words:?}");
                    assert!(!abstains, "{dialect}: case {words:?} no longer abstains");
                }
                (None, Some(got)) => {
                    panic!("tclsh{series} raises on case {words:?}, the selection answered {got}")
                }
                (Some(_), None) => assert!(
                    abstains,
                    "tclsh{series} ({dialect}): the selection declined case {words:?}"
                ),
                (None, None) => {}
            }
        }
    }
    for series in ["9.0", "9.1"] {
        if let Some(tclsh) = find_tclsh(series) {
            assert_eq!(
                run_tcl(&tclsh, "puts [catch {case a in a {}} m]$m"),
                Some((true, "1invalid command name \"case\"\n".to_owned())),
                "tclsh{series} has no case"
            );
        }
    }
}

/// What stops an ordered evaluation: a nested command that ends it with an
/// error, or one nothing here can answer.
#[derive(Debug, PartialEq)]
enum Stop {
    Ended,
    Declined(String),
}

/// `command args…` through the registry's route with `x` holding `prior`:
/// the result's text and the stores it makes, each resolved to its place.
fn ordered_route(
    reg: &CommandRegistry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    command: &str,
    args: &[&str],
    prior: &str,
) -> Option<(String, Vec<(PlaceRef, StoreOutcome)>)> {
    use tcl_registry::ArgRole;
    use tcl_registry::value_transfer::{
        AnalysisInputs, Budget, EvalAnswer, ExactValue, ExactValueOrUnavailable, LiteralInputs,
        OperandId, resolve_semantics,
    };

    let semantics = resolve_semantics(reg.get(command)?, None, None);
    let semantics = semantics.semantics()?;
    let mut inputs = LiteralInputs::new(command, None, args, profile)
        .with_prior("x", ExactValue::from_literal(prior));
    for role in [ArgRole::VarWrite, ArgRole::VarRead] {
        for index in reg.arg_indices_for_role(command, args, role) {
            inputs = inputs.with_role(OperandId(index), role);
        }
    }
    let EvalAnswer::Evaluated(outcome) = semantics.evaluate(&inputs, &mut Budget::evaluation())
    else {
        return None;
    };
    let ExactValueOrUnavailable::Exact(result) = outcome.result else {
        return None;
    };
    let stores = outcome
        .ordered_stores
        .into_iter()
        .map(|store| Some((inputs.place(store.target().0).ok()?, store)))
        .collect::<Option<Vec<_>>>()?;
    Some((String::from_utf8(result.bytes).ok()?, stores))
}

/// An expression evaluated in the shared walker's order over the ordered
/// evaluation state: a read of `x` consults the state's writes before the
/// value `x` held when the expression began, and a nested `[incr x]`,
/// `[set x 10]` or `[incr x 10]` runs through its registry route over the
/// value the state gives `x` and applies the stores it makes to the state, in
/// order. `[error mid]` ends the evaluation with the writes made so far.
struct OrderedProbe<'a> {
    reg: &'a CommandRegistry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    start: i64,
    state: tcl_registry::value_transfer::EvaluationState,
}

impl<'a> OrderedProbe<'a> {
    fn new(
        reg: &'a CommandRegistry,
        profile: Option<&'static tcl_dialect::DialectProfile>,
    ) -> Self {
        use tcl_registry::value_transfer::{EvaluationState, NestedPolicy};
        Self {
            reg,
            profile,
            start: 1,
            state: EvaluationState::new(NestedPolicy::LocalWrites),
        }
    }

    /// What a read of `name` finds.
    fn read(&self, name: &str) -> Result<i64, Stop> {
        use tcl_registry::value_transfer::WrittenPlace;
        match self.state.written(name) {
            WrittenPlace::Exact(value) => String::from_utf8(value.bytes)
                .ok()
                .and_then(|text| text.parse().ok())
                .ok_or_else(|| Stop::Declined(format!("the value of {name}"))),
            WrittenPlace::Untouched if name == "x" => Ok(self.start),
            WrittenPlace::Untouched | WrittenPlace::Unknown => {
                Err(Stop::Declined(format!("a read of {name}")))
            }
        }
    }

    /// Run the nested command `script`.
    fn run(&mut self, script: &str) -> Result<i64, Stop> {
        let words: Vec<&str> = script.split_whitespace().collect();
        let Some((head, args)) = words.split_first() else {
            return Err(Stop::Declined(script.to_owned()));
        };
        if *head == "error" {
            return Err(Stop::Ended);
        }
        let prior = self.read("x")?.to_string();
        let (result, stores) = ordered_route(self.reg, self.profile, head, args, &prior)
            .ok_or_else(|| Stop::Declined(script.to_owned()))?;
        self.state.writes.extend(stores);
        result
            .parse()
            .map_err(|_| Stop::Declined(script.to_owned()))
    }

    /// A quoted word's text after Tcl substitutes it, left to right.
    fn substitute(&mut self, text: &str) -> Result<String, Stop> {
        let mut out = String::new();
        let mut rest = text;
        while let Some(at) = rest.find(['$', '[']) {
            out.push_str(&rest[..at]);
            let tail = &rest[at + 1..];
            if rest[at..].starts_with('$') {
                let len = tail
                    .find(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                    .unwrap_or(tail.len());
                out.push_str(&self.read(&tail[..len])?.to_string());
                rest = &tail[len..];
            } else {
                let close = tail.find(']').expect("a closed substitution");
                out.push_str(&self.run(&tail[..close])?.to_string());
                rest = &tail[close + 1..];
            }
        }
        out.push_str(rest);
        Ok(out)
    }

    /// The value of `expression`, braced as `expr` takes it.
    fn expr(&mut self, expression: &str) -> Result<i64, Stop> {
        let node = tcl_syntax::expr::parser::parse_expr_for_profile(expression, self.profile);
        tcl_syntax::expr::eval(&node, self)
    }
}

impl tcl_syntax::expr::ExprOps for OrderedProbe<'_> {
    type Value = i64;
    type Error = Stop;

    fn literal(&mut self, text: &str) -> Result<i64, Stop> {
        text.parse().map_err(|_| Stop::Declined(text.to_owned()))
    }
    fn string(&mut self, inner: &str, _substitutes: bool) -> Result<i64, Stop> {
        Err(Stop::Declined(inner.to_owned()))
    }
    fn var(&mut self, name: &str) -> Result<i64, Stop> {
        self.read(name)
    }
    fn command(&mut self, script: &str) -> Result<i64, Stop> {
        self.run(script)
    }
    fn call(&mut self, function: &str, _args: Vec<i64>) -> Result<i64, Stop> {
        Err(Stop::Declined(function.to_owned()))
    }
    fn arith(&mut self, op: tcl_syntax::expr::BinOp, left: i64, right: i64) -> Result<i64, Stop> {
        match op {
            tcl_syntax::expr::BinOp::Add => Ok(left + right),
            other => Err(Stop::Declined(format!("{other:?}"))),
        }
    }
    fn unary(&mut self, op: tcl_syntax::expr::UnaryOp, _value: i64) -> Result<i64, Stop> {
        Err(Stop::Declined(format!("{op:?}")))
    }
    fn compare_numeric(
        &mut self,
        left: &i64,
        right: &i64,
    ) -> Option<tcl_syntax::expr::NumericCompare> {
        Some(tcl_syntax::expr::NumericCompare::Ordered(left.cmp(right)))
    }
    fn compare_string(&mut self, left: &i64, right: &i64) -> std::cmp::Ordering {
        left.to_string().cmp(&right.to_string())
    }
    fn in_list(&mut self, _needle: &i64, _list: &i64) -> Result<bool, Stop> {
        Err(Stop::Declined("in".to_owned()))
    }
    fn to_bool(&mut self, value: &i64) -> Result<bool, Stop> {
        Ok(*value != 0)
    }
    fn bool_value(&mut self, b: bool) -> i64 {
        i64::from(b)
    }
    fn unsupported(&mut self, what: &str) -> Stop {
        Stop::Declined(what.to_owned())
    }
}

/// The interface page's seven ordered-state programs over the registry's own
/// routes, against the real `tclsh` of each release found on `PATH`: a read
/// consults the writes the expression's nested commands have made before the
/// value it began with, each nested `incr` and `set` takes the value the state
/// gives `x` and applies its stores in order, and the evaluation ends with the
/// writes so far where a nested command raises. The expression's value, what
/// `x` holds afterwards and how many writes the state made are the same as
/// `tclsh` leaves: 5 and 2 (one write), 0 and 1 (none: the right operand is
/// never reached), 21 and 10 (one), 5 and 3 (two), 2 and 2 (one: the arm not
/// taken writes nothing), 3 and 2 for the quoted word, which substitutes
/// before it is an expression (one), and, for `[incr x] + [error mid]`, no
/// value and 2 (one).
#[test]
fn the_seven_ordered_state_witnesses() {
    let programs: [(&str, Option<&str>, &str, &str, usize); 7] = [
        (
            "{$x + [incr x] + $x}",
            Some("5"),
            "2",
            "puts [expr {$x + [incr x] + $x}]",
            1,
        ),
        (
            "{0 && [incr x]}",
            Some("0"),
            "1",
            "puts [expr {0 && [incr x]}]",
            0,
        ),
        (
            "{$x + [set x 10] + $x}",
            Some("21"),
            "10",
            "puts [expr {$x + [set x 10] + $x}]",
            1,
        ),
        (
            "{[incr x] + [incr x]}",
            Some("5"),
            "3",
            "puts [expr {[incr x] + [incr x]}]",
            2,
        ),
        (
            "{$x ? [incr x] : [incr x 10]}",
            Some("2"),
            "2",
            "puts [expr {$x ? [incr x] : [incr x 10]}]",
            1,
        ),
        (
            "\"$x + [incr x]\"",
            Some("3"),
            "2",
            "puts [expr \"$x + [incr x]\"]",
            1,
        ),
        (
            "{[incr x] + [error mid]}",
            None,
            "2",
            "catch {expr {[incr x] + [error mid]}}",
            1,
        ),
    ];
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let dialect = version.dialect_profile_name();
        let reg = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let profile = tcl_dialect::DialectProfile::find(dialect);
        for (expression, value, after, statement, writes) in programs {
            let mut probe = OrderedProbe::new(reg, profile);
            let got = match expression.strip_prefix('"') {
                Some(inner) => probe
                    .substitute(inner.trim_end_matches('"'))
                    .and_then(|text| probe.expr(&text)),
                None => probe.expr(expression.trim_matches(|c| c == '{' || c == '}')),
            };
            let tcl = format!("set x 1\n{statement}\nputs $x\n");
            let (_, out) = run_tcl(&tclsh, &tcl).expect("tclsh runs");
            let mut lines = out.lines();
            let oracle_value = value.map(|_| lines.next().expect("a value").to_owned());
            let oracle_after = lines.next().expect("x afterwards").to_owned();
            let at = format!("tclsh{} {expression}", version.version_string());
            match (value, got) {
                (Some(value), Ok(got)) => {
                    assert_eq!(got.to_string(), value, "{at}");
                    assert_eq!(oracle_value.as_deref(), Some(value), "{at}: the oracle");
                }
                (None, Err(Stop::Ended)) => {}
                (_, got) => panic!("{at}: the probe answered {got:?}"),
            }
            assert_eq!(oracle_after, after, "{at}: the oracle's x");
            assert_eq!(probe.read("x"), Ok(after.parse().unwrap()), "{at}: x");
            assert_eq!(probe.state.writes.len(), writes, "{at}: the writes made");
        }
    }
}

/// What one place holds after a witness: an array, a scalar's value, or
/// nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Held {
    Array,
    Value(String),
    Unset,
}

/// One prefix witness: the command and its words, the places that hold an
/// array before it, and the first release that has the command.
type PrefixWitness = (
    &'static str,
    &'static [&'static str],
    &'static [&'static str],
    &'static str,
);

/// What the place `name` holds before `command` runs: an array where the
/// witness says so; for `unset`, 1 in `p`, 2 in `q` and nothing in its
/// absent name; and `old` everywhere else.
fn held_before(command: &str, arrays: &[&str], name: &str) -> Held {
    if arrays.contains(&name) {
        Held::Array
    } else if command == "unset" {
        match name {
            "p" => Held::Value("1".to_owned()),
            "q" => Held::Value("2".to_owned()),
            _ => Held::Unset,
        }
    } else {
        Held::Value("old".to_owned())
    }
}

/// `catch {command args…}` under `tclsh` over the places as
/// [`held_before`] gives them: the message, and what each place holds
/// afterwards. The command must raise.
fn prefix_oracle(
    tclsh: &str,
    (command, args, arrays): (&str, &[&str], &[&str]),
    places: &[&str],
) -> (String, Vec<Held>) {
    use std::fmt::Write as _;
    let mut script = String::new();
    for &name in places {
        match held_before(command, arrays, name) {
            Held::Array => writeln!(script, "array set {name} {{k keep}}"),
            Held::Value(value) => writeln!(script, "set {name} {value}"),
            Held::Unset => Ok(()),
        }
        .expect("a script");
    }
    let invocation = tcl_command(command, None, args);
    writeln!(script, "puts [catch {{{invocation}}} msg]\nputs $msg").expect("a script");
    for &name in places {
        writeln!(
            script,
            "if {{[array exists {name}]}} {{puts array}} elseif {{[info exists {name}]}} \
             {{puts \"=[set {name}]\"}} else {{puts unset}}"
        )
        .expect("a script");
    }
    let (_, out) = run_tcl(tclsh, &script).expect("tclsh runs");
    let mut lines = out.lines();
    assert_eq!(lines.next(), Some("1"), "{invocation}: tclsh raises");
    let message = lines.next().expect("the message").to_owned();
    let held = lines
        .map(|line| match line {
            "array" => Held::Array,
            "unset" => Held::Unset,
            value => Held::Value(value.trim_start_matches('=').to_owned()),
        })
        .collect();
    (message, held)
}

/// The same invocation through the registry's route, each place's kind as
/// the existence rung would prove it: the message the route proves, if it
/// proves one, and what each place holds after the stores it says ran.
fn prefix_route(
    reg: &CommandRegistry,
    profile: Option<&'static tcl_dialect::DialectProfile>,
    (command, args, arrays): (&str, &[&str], &[&str]),
    places: &[(usize, &str)],
) -> (Option<String>, Vec<Held>) {
    use tcl_registry::ArgRole;
    use tcl_registry::value_transfer::{
        AnalysisInputs, BindingKind, Budget, CompletionOutcome, EvalAnswer, ExactValue,
        ExactValueOrUnavailable, Existence, LiteralInputs, OperandId, resolve_semantics,
    };
    let semantics = resolve_semantics(reg.get(command).expect(command), None, None);
    let semantics = semantics.semantics().expect("a registry-owned route");
    let mut inputs = LiteralInputs::new(command, None, args, profile);
    for &(index, name) in places {
        inputs = inputs.with_role(OperandId(index), ArgRole::VarWrite);
        let existence = match held_before(command, arrays, name) {
            Held::Array => Existence::Bound(BindingKind::Array),
            Held::Value(value) => {
                inputs = inputs.with_prior(name, ExactValue::from_literal(&value));
                Existence::Bound(BindingKind::Scalar)
            }
            Held::Unset => Existence::Unbound,
        };
        inputs = inputs.with_existence(name, existence);
    }
    let answer = semantics.evaluate(&inputs, &mut Budget::evaluation());
    let EvalAnswer::Evaluated(outcome) = answer else {
        panic!("{command}: the route answered {answer:?}");
    };
    let CompletionOutcome::Error {
        written, message, ..
    } = &outcome.completion
    else {
        panic!(
            "{command}: the route proves no error: {:?}",
            outcome.completion
        );
    };
    let mut held: Vec<(String, Held)> = places
        .iter()
        .map(|&(_, name)| (name.to_owned(), held_before(command, arrays, name)))
        .collect();
    for store in &outcome.ordered_stores[..*written] {
        let name = inputs.place(store.target().0).expect("a place").name;
        let slot = held
            .iter_mut()
            .find(|(place, _)| *place == name)
            .expect("a witness place");
        match store {
            StoreOutcome::Write { value, .. } => {
                slot.1 = Held::Value(String::from_utf8_lossy(&value.bytes).into_owned());
            }
            StoreOutcome::Unbind { .. } => slot.1 = Held::Unset,
            StoreOutcome::Preserve { .. } => {}
            other => panic!("{command}: a store the witness does not read: {other:?}"),
        }
    }
    let proved = match message {
        ExactValueOrUnavailable::Exact(text) => {
            Some(String::from_utf8_lossy(&text.bytes).into_owned())
        }
        ExactValueOrUnavailable::Unavailable(_) => None,
    };
    (proved, held.into_iter().map(|(_, held)| held).collect())
}

/// A command that raises part-way through its stores, run through the
/// registry's route with each place's kind as the existence rung proves it,
/// against the real `tclsh` of each release found on `PATH` from the
/// command's first: what each place holds after `catch {…}` is what the
/// stores the route says ran leave it, and the message is tclsh's wherever the
/// route proves one. `lassign` stops at the first place it cannot write (8.5
/// on), so `lassign {new second} a b` over an array `b` writes `a` alone, and
/// `lassign {x y z} a b c` leaves `c` as well; `regexp` stops the same way;
/// `scan` goes on past the array, so with `a` an array `scan {1 2} {%d %d} a
/// b` still writes `b`; and `unset p nosuch q` unbinds `p`, raises on the
/// absent name and leaves `q` at 2.
#[test]
fn the_prefix_rule_over_the_routes_matches_every_release_on_path() {
    use tcl_registry::ArgRole;
    let witnesses: [PrefixWitness; 6] = [
        ("lassign", &["new second", "a", "b"], &["b"], "8.5"),
        ("lassign", &["x y z", "a", "b", "c"], &["b"], "8.5"),
        ("regexp", &["(x)(y)", "xy", "a", "b"], &["b"], "8.4"),
        ("scan", &["1 2", "%d %d", "a", "b"], &["b"], "8.4"),
        ("scan", &["1 2", "%d %d", "a", "b"], &["a"], "8.4"),
        ("unset", &["p", "nosuch", "q"], &[], "8.4"),
    ];
    for version in tcl_dialect::TclVersion::ALL {
        let Some(tclsh) = find_tclsh(version.version_string()) else {
            continue;
        };
        let dialect = version.dialect_profile_name();
        let reg = tcl_registry::model::ingress::static_context_for(dialect).commands();
        let profile = tcl_dialect::DialectProfile::find(dialect);
        for (command, args, arrays, first) in witnesses {
            if version.version_string() < first {
                continue;
            }
            let at = format!(
                "tclsh{} {}",
                version.version_string(),
                tcl_command(command, None, args)
            );
            let places: Vec<(usize, &str)> = reg
                .arg_indices_for_role(command, args, ArgRole::VarWrite)
                .into_iter()
                .map(|index| (index, args[index]))
                .collect();
            let names: Vec<&str> = places.iter().map(|&(_, name)| name).collect();
            let (message, oracle) = prefix_oracle(&tclsh, (command, args, arrays), &names);
            let (proved, held) = prefix_route(reg, profile, (command, args, arrays), &places);
            if let Some(proved) = proved {
                assert_eq!(proved, message, "{at}: the message");
            }
            assert_eq!(held, oracle, "{at}: what each place holds");
        }
    }
}
