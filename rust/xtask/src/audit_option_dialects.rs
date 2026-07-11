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

//! Audit `OptionSpec` dialect gates against real tclsh 8.4/8.5/8.6/9.0.
//!
//! For each option in the probe table, generate a small Tcl probe and run
//! it against each built tclsh. An option is *supported* if the probe runs
//! without hitting a "bad option" / "unknown option" error or a
//! syntax-level rejection; channel / permission / runtime errors still
//! count as "recognised" (the command parsed and the option was accepted).
//! The per-version availability is written to
//! `tmp/option_dialect_audit.json` so the registry can be updated with
//! accurate `dialects` frozensets.
//!
//! The JSON artifact at `tmp/option_dialect_audit.json` is deterministic:
//! the probe table, version order, and 2-space indent layout are all fixed.
//! The console progress log prints a line per probe, including
//! `repr()`-formatted failure diagnostics and a list-repr summary. An
//! unbuilt tclsh yields `"tclsh not found"` for every probe (a missing-tree
//! short-circuit), so the audit is meaningful on whatever subset of the
//! four trees is built.

use std::io::{Read, Write};
use std::path::Path;
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

use crate::util::repo_root;

/// The built tclsh trees, in registry order (this is also the order the
/// `supported_in` lists and the JSON entries are emitted in).
const TCL_VERSIONS: &[(&str, &str)] = &[
    ("tcl8.4", "tmp/tcl8.4.20/unix"),
    ("tcl8.5", "tmp/tcl8.5.19/unix"),
    ("tcl8.6", "tmp/tcl8.6.16/unix"),
    ("tcl9.0", "tmp/tcl9.0.4/unix"),
];

/// Substrings that mean the option was *not* recognised (as opposed to a
/// runtime failure). Compared case-insensitively against the combined
/// stdout+stderr.
const NOT_SUPPORTED_TOKENS: &[&str] = &[
    "bad option",
    "bad switch", // Tcl 8.4 spelling for some commands.
    "unknown option",
    "unknown switch",
    "no such option",
    "invalid option",
    "unrecognized option",
    "ambiguous option",
    "ambiguous switch",
    "wrong # args:",
    "bad subcommand", // subcommand-level option attached to wrong sub.
];

/// Per-option probe scripts: `(command, subcommand?, option, probe)`. The
/// probe exercises the option in a way that gives a definitive answer —
/// acceptance (lexical/dispatch wise) versus a "bad option" rejection.
/// Declaration order is preserved in the JSON output and the console log,
/// matching `dict` insertion order.
const PROBES: &[(&str, Option<&str>, &str, &str)] = &[
    // ---- lsearch ----
    ("lsearch", None, "-stride", "lsearch -stride 2 {a 1} *"),
    (
        "lsearch",
        None,
        "-bisect",
        "lsearch -bisect -sorted -decreasing {3 2 1} 2",
    ),
    (
        "lsearch",
        None,
        "-subindices",
        "lsearch -subindices -index 0 {a b} a",
    ),
    ("lsearch", None, "-index", "lsearch -index 0 {a b} a"),
    ("lsearch", None, "-inline", "lsearch -inline {a} a"),
    ("lsearch", None, "-not", "lsearch -not {a b} a"),
    ("lsearch", None, "-start", "lsearch -start 0 {a b} b"),
    // ---- lsort ----
    ("lsort", None, "-stride", "lsort -stride 2 {a 1 b 2}"),
    ("lsort", None, "-indices", "lsort -indices {a b c}"),
    ("lsort", None, "-unique", "lsort -unique {a b c}"),
    (
        "lsort",
        None,
        "-command",
        "lsort -command {string compare} {a b c}",
    ),
    // ---- regsub ----
    (
        "regsub",
        None,
        "-command",
        "regsub -command {a} {abc} {string toupper}",
    ),
    ("regsub", None, "-expanded", "regsub -expanded {a} {abc} X"),
    ("regsub", None, "-line", "regsub -line {a} {abc} X"),
    ("regsub", None, "-linestop", "regsub -linestop {a} {abc} X"),
    (
        "regsub",
        None,
        "-lineanchor",
        "regsub -lineanchor {a} {abc} X",
    ),
    ("regsub", None, "-all", "regsub -all {a} {aaa} X"),
    ("regsub", None, "-start", "regsub -start 0 {a} {abc} X"),
    // ---- regexp ----
    ("regexp", None, "-expanded", "regexp -expanded {a} {abc}"),
    ("regexp", None, "-line", "regexp -line {a} {abc}"),
    ("regexp", None, "-linestop", "regexp -linestop {a} {abc}"),
    (
        "regexp",
        None,
        "-lineanchor",
        "regexp -lineanchor {a} {abc}",
    ),
    ("regexp", None, "-all", "regexp -all {a} {aaa}"),
    ("regexp", None, "-inline", "regexp -inline {a} {abc}"),
    ("regexp", None, "-indices", "regexp -indices {a} {abc}"),
    ("regexp", None, "-start", "regexp -start 0 {a} {abc}"),
    ("regexp", None, "-about", "regexp -about {a}"),
    // ---- exec ----
    (
        "exec",
        None,
        "-ignorestderr",
        "exec -ignorestderr -- echo hi",
    ),
    ("exec", None, "-keepnewline", "exec -keepnewline -- echo hi"),
    // ---- glob ----
    (
        "glob",
        None,
        "-directory",
        "glob -directory /tmp -nocomplain *",
    ),
    ("glob", None, "-join", "glob -join -nocomplain /tmp *"),
    ("glob", None, "-path", "glob -path /tmp/x -nocomplain *"),
    (
        "glob",
        None,
        "-tails",
        "glob -tails -directory /tmp -nocomplain *",
    ),
    (
        "glob",
        None,
        "-types",
        "glob -types f -directory /tmp -nocomplain *",
    ),
    (
        "glob",
        None,
        "-nocomplain",
        "glob -nocomplain /nonexistent/x/*",
    ),
    // ---- file copy / delete / rename / link ----
    (
        "file",
        Some("copy"),
        "-force",
        "file copy -force -- /tmp/_audit_a /tmp/_audit_b",
    ),
    (
        "file",
        Some("delete"),
        "-force",
        "file delete -force -- /tmp/_audit_z",
    ),
    (
        "file",
        Some("rename"),
        "-force",
        "file rename -force -- /tmp/_audit_a /tmp/_audit_c",
    ),
    (
        "file",
        Some("link"),
        "-symbolic",
        "file link -symbolic /tmp/_audit_link /tmp/_audit_target",
    ),
    (
        "file",
        Some("link"),
        "-hard",
        "file link -hard /tmp/_audit_link /tmp/_audit_target",
    ),
    // ---- chan / fconfigure (channel options) ----
    (
        "fconfigure",
        None,
        "-blocking",
        "set f [open /tmp/_audit_ch w]; fconfigure $f -blocking 0; close $f",
    ),
    (
        "fconfigure",
        None,
        "-buffering",
        "set f [open /tmp/_audit_ch w]; fconfigure $f -buffering line; close $f",
    ),
    (
        "fconfigure",
        None,
        "-buffersize",
        "set f [open /tmp/_audit_ch w]; fconfigure $f -buffersize 4096; close $f",
    ),
    (
        "fconfigure",
        None,
        "-encoding",
        "set f [open /tmp/_audit_ch w]; fconfigure $f -encoding utf-8; close $f",
    ),
    (
        "fconfigure",
        None,
        "-eofchar",
        "set f [open /tmp/_audit_ch w]; fconfigure $f -eofchar {}; close $f",
    ),
    (
        "fconfigure",
        None,
        "-translation",
        "set f [open /tmp/_audit_ch w]; fconfigure $f -translation auto; close $f",
    ),
    (
        "fconfigure",
        None,
        "-profile",
        "set f [open /tmp/_audit_ch w]; fconfigure $f -profile strict; close $f",
    ),
    (
        "fconfigure",
        None,
        "-keepalive",
        "set s [socket -server {} -myaddr 127.0.0.1 0]; set p [lindex [fconfigure $s -sockname] 2]; set c [socket 127.0.0.1 $p]; fconfigure $c -keepalive 1; close $c; close $s",
    ),
    (
        "fconfigure",
        None,
        "-nodelay",
        "set s [socket -server {} -myaddr 127.0.0.1 0]; set p [lindex [fconfigure $s -sockname] 2]; set c [socket 127.0.0.1 $p]; fconfigure $c -nodelay 1; close $c; close $s",
    ),
    (
        "fconfigure",
        None,
        "-inputmode",
        "fconfigure stdin -inputmode normal",
    ),
    // ---- clock scan options ----
    ("clock", Some("scan"), "-base", "clock scan now -base 0"),
    (
        "clock",
        Some("scan"),
        "-format",
        "clock scan {2020-01-01} -format {%Y-%m-%d}",
    ),
    (
        "clock",
        Some("scan"),
        "-gmt",
        "clock scan {2020-01-01} -gmt 1 -format {%Y-%m-%d}",
    ),
    (
        "clock",
        Some("scan"),
        "-locale",
        "clock scan {2020-01-01} -locale C -format {%Y-%m-%d}",
    ),
    (
        "clock",
        Some("scan"),
        "-timezone",
        "clock scan {2020-01-01} -timezone :UTC -format {%Y-%m-%d}",
    ),
    (
        "clock",
        Some("scan"),
        "-validate",
        "clock scan {2020-13-01} -validate 0 -format {%Y-%m-%d}",
    ),
    // ---- socket ----
    (
        "socket",
        None,
        "-async",
        "set s [socket -async 127.0.0.1 1]; close $s",
    ),
    (
        "socket",
        None,
        "-myaddr",
        "set s [socket -server {} -myaddr 127.0.0.1 0]; close $s",
    ),
    (
        "socket",
        None,
        "-myport",
        "set s [socket -server {} -myport 0 -myaddr 127.0.0.1 0]; close $s",
    ),
    (
        "socket",
        None,
        "-server",
        "set s [socket -server {} -myaddr 127.0.0.1 0]; close $s",
    ),
    (
        "socket",
        None,
        "-reuseaddr",
        "set s [socket -server {} -reuseaddr 1 -myaddr 127.0.0.1 0]; close $s",
    ),
    (
        "socket",
        None,
        "-reuseport",
        "set s [socket -server {} -reuseport 1 -myaddr 127.0.0.1 0]; close $s",
    ),
    // ---- source ----
    (
        "source",
        None,
        "-encoding",
        "set f [open /tmp/_audit_src w]; close $f; source -encoding utf-8 /tmp/_audit_src",
    ),
    // ---- unset ----
    (
        "unset",
        None,
        "-nocomplain",
        "unset -nocomplain ::nonexistent_var_42",
    ),
    // ---- string compare/equal ----
    (
        "string",
        Some("compare"),
        "-nocase",
        "string compare -nocase A a",
    ),
    (
        "string",
        Some("compare"),
        "-length",
        "string compare -length 1 ab ac",
    ),
    (
        "string",
        Some("equal"),
        "-nocase",
        "string equal -nocase A a",
    ),
    (
        "string",
        Some("equal"),
        "-length",
        "string equal -length 1 ab ac",
    ),
    (
        "string",
        Some("is"),
        "-strict",
        "string is integer -strict 123",
    ),
    (
        "string",
        Some("is"),
        "-failindex",
        "string is integer -failindex foo 12X",
    ),
    (
        "string",
        Some("map"),
        "-nocase",
        "string map -nocase {A B} aA",
    ),
    (
        "string",
        Some("match"),
        "-nocase",
        "string match -nocase A a",
    ),
    // ---- switch ----
    ("switch", None, "-exact", "switch -exact a {a {set x 1}}"),
    ("switch", None, "-glob", "switch -glob a {a* {set x 1}}"),
    (
        "switch",
        None,
        "-regexp",
        "switch -regexp a {{^a$} {set x 1}}",
    ),
    ("switch", None, "-nocase", "switch -nocase A {a {set x 1}}"),
    (
        "switch",
        None,
        "-matchvar",
        "switch -regexp -matchvar m a {{(.*)} {set x 1}}",
    ),
    // ---- subst ----
    (
        "subst",
        None,
        "-nobackslashes",
        r"subst -nobackslashes {\n}",
    ),
    ("subst", None, "-nocommands", "subst -nocommands {[set x]}"),
    ("subst", None, "-novariables", r"subst -novariables {\$x}"),
    // ---- interp ----
    (
        "interp",
        Some("create"),
        "-safe",
        "set i [interp create -safe]; interp delete $i",
    ),
    (
        "interp",
        Some("cancel"),
        "-unwind",
        "set i [interp create]; interp cancel -unwind -- $i {}",
    ),
    (
        "interp",
        Some("invokehidden"),
        "-global",
        "set i [interp create]; interp hide $i set; interp invokehidden $i -global set x 1; interp delete $i",
    ),
    (
        "interp",
        Some("invokehidden"),
        "-namespace",
        "set i [interp create]; interp hide $i set; interp invokehidden $i -namespace :: set x 1; interp delete $i",
    ),
    // ---- package ----
    (
        "package",
        Some("present"),
        "-exact",
        "catch {package present -exact Tcl 9.0}",
    ),
    (
        "package",
        Some("require"),
        "-exact",
        "catch {package require -exact Tcl 9.0}",
    ),
    // ---- puts ----
    ("puts", None, "-nonewline", "puts -nonewline {}"),
    // ---- load ----
    ("load", None, "-global", "catch {load -global /nonexistent}"),
    ("load", None, "-lazy", "catch {load -lazy /nonexistent}"),
    // ---- unload ----
    (
        "unload",
        None,
        "-nocomplain",
        "catch {unload -nocomplain /nonexistent}",
    ),
    (
        "unload",
        None,
        "-keeplibrary",
        "catch {unload -keeplibrary /nonexistent}",
    ),
    // ---- encoding ----
    (
        "encoding",
        None,
        "-profile",
        "catch {encoding convertfrom -profile strict utf-8 hi}",
    ),
    // ---- vwait (Tcl 9.0 added many) ----
    (
        "vwait",
        None,
        "-all",
        "after 1 {set ::vw 1}; catch {vwait -all -timeout 100 -variable ::vw}",
    ),
    (
        "vwait",
        None,
        "-extended",
        "after 1 {set ::vw 1}; catch {vwait -extended -timeout 100 -variable ::vw}",
    ),
    (
        "vwait",
        None,
        "-nofileevents",
        "after 1 {set ::vw 1}; catch {vwait -nofileevents -timeout 100 -variable ::vw}",
    ),
    (
        "vwait",
        None,
        "-noidleevents",
        "after 1 {set ::vw 1}; catch {vwait -noidleevents -timeout 100 -variable ::vw}",
    ),
    (
        "vwait",
        None,
        "-notimerevents",
        "after 1 {set ::vw 1}; catch {vwait -notimerevents -timeout 100 -variable ::vw}",
    ),
    (
        "vwait",
        None,
        "-nowindowevents",
        "after 1 {set ::vw 1}; catch {vwait -nowindowevents -timeout 100 -variable ::vw}",
    ),
    (
        "vwait",
        None,
        "-readable",
        "catch {vwait -readable stdin -timeout 1}",
    ),
    (
        "vwait",
        None,
        "-timeout",
        "after 1 {set ::vw 1}; catch {vwait -timeout 100 -variable ::vw}",
    ),
    (
        "vwait",
        None,
        "-variable",
        "after 1 {set ::vw 1}; vwait ::vw",
    ),
    (
        "vwait",
        None,
        "-writable",
        "catch {vwait -writable stdout -timeout 1}",
    ),
];

/// One audited option and the versions that accept it.
struct Entry {
    command: &'static str,
    subcommand: Option<&'static str>,
    option: &'static str,
    supported_in: Vec<&'static str>,
}

/// Run the dialect audit: probe every option against each built tclsh,
/// write the JSON artifact, and print the per-option / summary log.
/// Always exits 0 (matching `return 0`).
pub fn run() -> Result<ExitCode> {
    let root = repo_root();
    let mut entries: Vec<Entry> = Vec::with_capacity(PROBES.len());

    for &(cmd, sub, opt, script) in PROBES {
        println!("\n=== {} ===", label(cmd, sub, opt));
        let mut supported = Vec::new();
        for &(ver, rel_dir) in TCL_VERSIONS {
            let (ok, out) = probe(&root.join(rel_dir), script);
            if ok {
                supported.push(ver);
                println!("  {ver}: ✓");
            } else {
                let first_line: String =
                    out.lines().next().unwrap_or("").chars().take(60).collect();
                println!("  {ver}: ✗  {}", py_repr(&first_line));
            }
        }
        entries.push(Entry {
            command: cmd,
            subcommand: sub,
            option: opt,
            supported_in: supported,
        });
    }

    let out_path = root.join("tmp").join("option_dialect_audit.json");
    if let Some(parent) = out_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("creating {}", parent.display()))?;
    }
    std::fs::write(&out_path, render_json(&entries))
        .with_context(|| format!("writing {}", out_path.display()))?;
    println!("\nFull results written to {}", out_path.display());

    println!("\n=== Summary (options NOT universally available) ===");
    let all = TCL_VERSIONS.len();
    for entry in &entries {
        if entry.supported_in.len() != all {
            let label = label(entry.command, entry.subcommand, entry.option);
            println!("  {label:50}  {}", py_list_repr(&entry.supported_in));
        }
    }

    Ok(ExitCode::SUCCESS)
}

/// `"cmd [sub ]opt"` — the human-readable option label.
fn label(cmd: &str, sub: Option<&str>, opt: &str) -> String {
    match sub {
        Some(s) => format!("{cmd} {s} {opt}"),
        None => format!("{cmd} {opt}"),
    }
}

/// Run *script* under the tclsh in `tclsh_dir` and return `(ok, output)`.
/// `ok` is false only when the output indicates the option was not
/// recognised; runtime errors (network, permission, channel) count as
/// "recognised". A missing tclsh short-circuits to `(false, "tclsh not
/// found")` exactly as `exists()` check does.
fn probe(tclsh_dir: &Path, script: &str) -> (bool, String) {
    let tclsh = tclsh_dir.join("tclsh");
    if !tclsh.exists() {
        return (false, "tclsh not found".to_string());
    }
    // `catch` so the script doesn't die on runtime errors.
    let wrapped = format!("if {{[catch {{{script}}} err]}} {{puts ERR:$err}} else {{puts OK}}");
    match run_with_timeout(&tclsh, tclsh_dir, &wrapped, Duration::from_secs(5)) {
        Run::Timeout => (true, "timeout (option recognised, test ran)".to_string()),
        Run::SpawnError(e) => (false, format!("spawn error: {e}")),
        Run::Done(output) => (classify(&output), output),
    }
}

/// Classify combined tclsh output: `false` if it contains a
/// "not recognised" token (case-insensitively), else `true`.
fn classify(output: &str) -> bool {
    let lower = output.to_lowercase();
    !NOT_SUPPORTED_TOKENS.iter().any(|t| lower.contains(t))
}

/// Outcome of running a probe under a wall-clock deadline.
enum Run {
    /// Process exited; carries the trimmed stdout+stderr.
    Done(String),
    /// The deadline elapsed and the child was killed.
    Timeout,
    /// The child could not be spawned.
    SpawnError(std::io::Error),
}

/// Spawn tclsh with `input` on stdin and `LD_LIBRARY_PATH` set to its build
/// dir, returning its combined output or [`Run::Timeout`] if it outruns
/// `timeout`. Reader threads drain stdout/stderr so a chatty child can't
/// deadlock the wait loop.
fn run_with_timeout(tclsh: &Path, dir: &Path, input: &str, timeout: Duration) -> Run {
    let mut child = match Command::new(tclsh)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .env("LD_LIBRARY_PATH", dir)
        .spawn()
    {
        Ok(c) => c,
        Err(e) => return Run::SpawnError(e),
    };

    if let Some(mut stdin) = child.stdin.take() {
        let _ = stdin.write_all(input.as_bytes());
        // Dropping `stdin` closes the pipe so tclsh sees EOF.
    }
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let out_reader = thread::spawn(move || drain(stdout.as_mut()));
    let err_reader = thread::spawn(move || drain(stderr.as_mut()));

    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => {
                let out = out_reader.join().unwrap_or_default();
                let err = err_reader.join().unwrap_or_default();
                return Run::Done(format!("{out}{err}").trim().to_string());
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = out_reader.join();
                    let _ = err_reader.join();
                    return Run::Timeout;
                }
                thread::sleep(Duration::from_millis(25));
            }
            Err(_) => {
                let _ = child.kill();
                let _ = out_reader.join();
                let _ = err_reader.join();
                return Run::Timeout;
            }
        }
    }
}

/// Read a child pipe to end, swallowing I/O errors (the caller only needs
/// best-effort output).
fn drain<R: Read>(reader: Option<&mut R>) -> String {
    let mut s = String::new();
    if let Some(r) = reader {
        let _ = r.read_to_string(&mut s);
    }
    s
}

/// Serialise the audit results as JSON with a 2-space indent and a trailing
/// newline. Every value here is ASCII (command/option/version identifiers),
/// so no escaping is required.
fn render_json(entries: &[Entry]) -> String {
    if entries.is_empty() {
        return "[]\n".to_string();
    }
    let mut s = String::from("[\n");
    for (i, e) in entries.iter().enumerate() {
        s.push_str("  {\n    \"command\": \"");
        s.push_str(e.command);
        s.push_str("\",\n    \"subcommand\": ");
        match e.subcommand {
            Some(sub) => {
                s.push('"');
                s.push_str(sub);
                s.push('"');
            }
            None => s.push_str("null"),
        }
        s.push_str(",\n    \"option\": \"");
        s.push_str(e.option);
        s.push_str("\",\n    \"supported_in\": ");
        if e.supported_in.is_empty() {
            s.push_str("[]\n");
        } else {
            s.push_str("[\n");
            for (j, v) in e.supported_in.iter().enumerate() {
                s.push_str("      \"");
                s.push_str(v);
                s.push('"');
                if j + 1 < e.supported_in.len() {
                    s.push(',');
                }
                s.push('\n');
            }
            s.push_str("    ]\n");
        }
        s.push_str("  }");
        if i + 1 < entries.len() {
            s.push(',');
        }
        s.push('\n');
    }
    s.push_str("]\n");
    s
}

/// Render a string in repr style for the failure diagnostics. Picks single
/// quotes unless the string contains a single quote and no double quote
/// (then double), escaping the active quote, backslash, and the common
/// control characters (`\n`, `\r`, `\t`, and `\xHH` for other ASCII
/// controls).
fn py_repr(s: &str) -> String {
    let has_single = s.contains('\'');
    let has_double = s.contains('"');
    let quote = if has_single && !has_double { '"' } else { '\'' };
    let mut out = String::with_capacity(s.len() + 2);
    out.push(quote);
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c == quote => {
                out.push('\\');
                out.push(c);
            }
            c if (c as u32) < 0x20 || c as u32 == 0x7f => {
                let b = c as u32;
                out.push_str("\\x");
                // `b` is < 0x80 here, so both nibbles are valid hex digits.
                out.push(char::from_digit(b >> 4, 16).unwrap_or('0'));
                out.push(char::from_digit(b & 0xf, 16).unwrap_or('0'));
            }
            c => out.push(c),
        }
    }
    out.push(quote);
    out
}

/// Render a list of strings in repr style — `['a', 'b']`, single-quoted
/// items, comma-space separated; `[]` when empty.
fn py_list_repr(items: &[&str]) -> String {
    let mut sorted: Vec<&str> = items.to_vec();
    sorted.sort_unstable();
    let inner = sorted
        .iter()
        .map(|s| py_repr(s))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{inner}]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_flags_unrecognised_options() {
        assert!(classify("OK"));
        assert!(classify("ERR:can't open socket: connection refused"));
        assert!(!classify(r#"ERR:bad option "-foo": must be -bar"#));
        assert!(!classify("ERR:wrong # args: should be ..."));
        // Case-insensitive, matching `.lower()`.
        assert!(!classify("ERR:Unknown Option -x"));
    }

    #[test]
    fn label_includes_subcommand_only_when_present() {
        assert_eq!(label("lsearch", None, "-stride"), "lsearch -stride");
        assert_eq!(label("string", Some("is"), "-strict"), "string is -strict");
    }

    #[test]
    fn py_repr_quotes_and_escapes() {
        assert_eq!(py_repr("ok"), "'ok'");
        // A double quote inside keeps single outer quotes, double unescaped.
        assert_eq!(py_repr(r#"bad option "-x""#), r#"'bad option "-x"'"#);
        // A single quote and no double quote flips to double outer quotes.
        assert_eq!(py_repr("it's"), "\"it's\"");
        // Both quotes present → single outer, the single quote escaped.
        assert_eq!(py_repr("a'b\"c"), "'a\\'b\"c'");
        // Control characters use the named/`\xNN` escapes.
        assert_eq!(py_repr("tab\there"), "'tab\\there'");
        assert_eq!(py_repr("\x07"), "'\\x07'");
    }

    #[test]
    fn py_list_repr_sorts_and_quotes() {
        assert_eq!(
            py_list_repr(&["tcl9.0", "tcl8.5", "tcl8.6"]),
            "['tcl8.5', 'tcl8.6', 'tcl9.0']"
        );
        assert_eq!(py_list_repr(&[]), "[]");
    }

    #[test]
    fn render_json_two_space_indent() {
        let entries = vec![
            Entry {
                command: "lsearch",
                subcommand: None,
                option: "-stride",
                supported_in: vec!["tcl8.5", "tcl8.6", "tcl9.0"],
            },
            Entry {
                command: "string",
                subcommand: Some("is"),
                option: "-failindex",
                supported_in: vec![],
            },
        ];
        let expected = "[\n  {\n    \"command\": \"lsearch\",\n    \"subcommand\": null,\n    \"option\": \"-stride\",\n    \"supported_in\": [\n      \"tcl8.5\",\n      \"tcl8.6\",\n      \"tcl9.0\"\n    ]\n  },\n  {\n    \"command\": \"string\",\n    \"subcommand\": \"is\",\n    \"option\": \"-failindex\",\n    \"supported_in\": []\n  }\n]\n";
        assert_eq!(render_json(&entries), expected);
    }

    #[test]
    fn render_json_empty_is_bracket_pair() {
        assert_eq!(render_json(&[]), "[]\n");
    }

    #[test]
    fn probe_table_covers_known_options() {
        // Spot-check the table transcription against a few well-known gates:
        // `lsearch -stride` (8.6+), `string is -failindex`, and a Tcl 9 vwait
        // option are all present, in the expected shapes.
        assert!(
            PROBES
                .iter()
                .any(|&(c, s, o, _)| c == "lsearch" && s.is_none() && o == "-stride")
        );
        assert!(
            PROBES
                .iter()
                .any(|&(c, s, o, _)| c == "string" && s == Some("is") && o == "-failindex")
        );
        assert!(
            PROBES
                .iter()
                .any(|&(c, s, o, _)| c == "vwait" && s.is_none() && o == "-timeout")
        );
        // No duplicate (command, subcommand, option) keys.
        let mut keys: Vec<_> = PROBES.iter().map(|&(c, s, o, _)| (c, s, o)).collect();
        let total = keys.len();
        keys.sort_unstable();
        keys.dedup();
        assert_eq!(keys.len(), total, "probe table has duplicate keys");
    }

    /// Drift guard: every probed `(command, subcommand, option)` must be a real
    /// registry [`tcl_registry::hover::OptionSpec`]. This ties the audit's
    /// option set to the registry single source of truth — a probe cannot name
    /// an option the registry does not declare, and a renamed or removed option
    /// fails here rather than silently producing a stale `supported_in`. The
    /// audit runs every probe against every built Tcl version, so the existence
    /// check is deliberately dialect-agnostic (`None`).
    #[test]
    fn probe_options_exist_in_registry() {
        use tcl_registry::CommandRegistry;
        let reg = CommandRegistry::build_default();
        let missing: Vec<String> = PROBES
            .iter()
            .filter(|&&(cmd, sub, opt, _)| {
                let Some(spec) = reg.get(cmd) else {
                    return true;
                };
                match sub {
                    None => spec.find_option(opt, None, None).is_none(),
                    Some(s) => !spec.resolve_subcommand(s).is_some_and(|sc| {
                        sc.option_specs(None, spec.dialects)
                            .iter()
                            .any(|o| o.matches(opt))
                    }),
                }
            })
            .map(|&(cmd, sub, opt, _)| label(cmd, sub, opt))
            .collect();
        assert!(
            missing.is_empty(),
            "probe options absent from the registry OptionSpec (the audit has \
             drifted from the single source of truth): {missing:?}"
        );
    }
}
