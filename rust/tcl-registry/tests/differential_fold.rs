//! Differential verification of the registry `const_fold` callbacks against
//! real C Tcl.
//!
//! Every fold the optimiser performs for a pure builtin command substitution
//! (the O129 path and the SCCP constant-propagation path) must be
//! byte-identical to what the reference interpreter produces — otherwise a
//! wrong literal gets baked into the source, which surfaces as a
//! false-positive "optimisation" (a behaviour change disguised as a fold).
//!
//! This harness mirrors `tcl-compiler`'s `fold_builtin_cmd_subst_raw`: it
//! resolves a command through the registry (head → `spec.const_fold`, or
//! head + subcommand → `subcommand.const_fold`), runs the *same* command on a
//! real `tclsh9.0`, and asserts the fold — **when it fires** — matches. A miss
//! (`None`) is always acceptable: the optimiser simply leaves the call
//! unfolded, so a missed fold is never wrong. Only a fold that produces the
//! *wrong* value fails the test.
//!
//! It is the Rust-side counterpart of `tests/test_const_fold_vs_tcl.py` on
//! `main`, and the differential-pinning tool the deferred O129 long-tail
//! (the `string is` number classes, the full `format` flag/width/precision
//! matrix, `scan`) is landed against — one verb/class per strip.
//!
//! Skips cleanly (the test passes trivially) when no `tclsh9.0` is on `PATH`,
//! the same contract as the Python harness's `skipif`.

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

/// Locate a `tclsh9.0` (or a `tclsh` reporting a `9.x` patchlevel) on `PATH`.
fn find_tclsh9() -> Option<String> {
    for cand in ["tclsh9.0", "tclsh"] {
        if let Some((true, out)) = run_tcl(cand, "puts -nonewline [info patchlevel]") {
            if out.starts_with("9.") {
                return Some(cand.to_string());
            }
        }
    }
    None
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

/// Resolve a command through the registry exactly as the optimiser does and
/// run its `const_fold`, returning the **raw** folded value (the same string
/// `tcl_value` compares against — no propagation-word quoting).
fn registry_fold(
    reg: &CommandRegistry,
    head: &str,
    sub: Option<&str>,
    args: &[&str],
) -> Option<String> {
    let spec = reg.get(head)?;
    let fold = match sub {
        None => spec.const_fold?,
        Some(s) => spec.subcommand(s)?.const_fold?,
    };
    fold(args)
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

/// The broad list / string / dict / subst fold matrix, ported from the
/// `_FOLDABLE` set in `tests/test_const_fold_vs_tcl.py`. `string is` number
/// classes are included even though they currently bail — they are pinned
/// here the moment a fold lands.
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
    ("string", Some("last"), &["l", "hello"]),
    ("string", Some("equal"), &["abc", "abc"]),
    ("string", Some("equal"), &["abc", "abd"]),
    ("string", Some("compare"), &["a", "b"]),
    // string is — char classes + integer fold today; double pins for when it lands
    ("string", Some("is"), &["alpha", "abc"]),
    ("string", Some("is"), &["digit", "123"]),
    ("string", Some("is"), &["space", "   "]),
    ("string", Some("is"), &["integer", "42"]),
    ("string", Some("is"), &["integer", "-7"]),
    ("string", Some("is"), &["integer", "4294967295"]),
    ("string", Some("is"), &["integer", "abc"]),
    ("string", Some("is"), &["integer", "1.5"]),
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
}

/// The `format` flag / width / precision matrix, ported from `_FORMATS`. The
/// landed fold covers the plain `%s` / `%d` / `%%` subset; the richer
/// conversions currently bail and are differentially pinned here for the
/// follow-up strips. (Cases tclsh-divergent across versions — `%#o`, negative
/// radix conversions — are deliberately excluded, since no single fold can be
/// sound for them across 8.x ↔ 9.0.)
const FORMATS: &[Case] = &[
    ("format", None, &["%d", "42"]),
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
    // No `folded > 0` floor here: until the flag/width/precision strip lands
    // most of this matrix bails, and that is correct. The plain `%d`/`%s`/`%%`
    // subset still fires, so the harness is exercised.
    check_matrix(&tclsh, &reg, FORMATS);
}
