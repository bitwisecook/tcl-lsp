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

//! Raw-byte end-to-end tests for the shipping native `TclVM` driver.

use std::process::{Command, Output};

fn run(script: &str, locale: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tclvm"))
        .args(["-c", script])
        .env("LC_ALL", locale)
        .output()
        .expect("run tclvm")
}

fn run_version(script: &str, locale: &str, version: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_tclvm"))
        .args(["--tcl-version", version, "-c", script])
        .env("LC_ALL", locale)
        .output()
        .expect("run versioned tclvm")
}

#[test]
fn binary_translation_and_explicit_utf8_match_tcl_9_bytes() {
    // Tcl 9.0.4 io-39.14/io-39.15: binary translation selects the
    // byte-preserving ISO-8859-1 channel encoding.
    let binary = run(
        "fconfigure stdout -translation binary; \
         puts -nonewline [binary format H* ff41]",
        "C.UTF-8",
    );
    assert!(binary.status.success(), "{:?}", binary.stderr);
    assert_eq!(binary.stdout, [0xff, b'A']);

    let utf8 = run(
        "fconfigure stdout -encoding utf-8 -translation lf; \
         puts -nonewline [binary format H* ff41]",
        "C.UTF-8",
    );
    assert!(utf8.status.success(), "{:?}", utf8.stderr);
    assert_eq!(utf8.stdout, [0xc3, 0xbf, b'A']);
}

#[test]
fn strict_profile_writes_prefix_and_reports_structured_eilseq() {
    // Tcl 9.0.4 io-75.9: strict conversion writes the representable prefix,
    // then raises the POSIX EILSEQ completion without appending a newline.
    let output = run(
        "fconfigure stdout -encoding iso8859-1 -translation lf -profile strict; \
         set c [catch {puts -nonewline \"A\\u0178B\"} m o]; \
         fconfigure stdout -encoding utf-8; \
         puts -nonewline \"\\n[list $c $m [dict get $o -errorcode]]\"",
        "C.UTF-8",
    );
    assert!(output.status.success(), "{:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"A\n1 {error writing \"stdout\": invalid or incomplete multibyte or wide character} {POSIX EILSEQ {invalid or incomplete multibyte or wide character}}"
    );
}

#[test]
fn configuration_errors_keep_their_structured_tcl_identity() {
    let output = run(
        "catch {fconfigure stdout -profile bogus} m o; \
         puts -nonewline [list $m [dict get $o -errorcode]]",
        "C.UTF-8",
    );
    assert!(output.status.success(), "{:?}", output.stderr);
    assert_eq!(
        output.stdout,
        b"{bad profile name \"bogus\": must be replace, strict, or tcl8} {TCL ENCODING PROFILE bogus}"
    );
}

#[test]
fn binary_open_mode_and_child_standard_channel_configuration_persist() {
    let output = run(
        "set f [open /dev/stdout wb]; \
         puts -nonewline $f [binary format H* ff]; close $f; \
         set f [open /dev/stdout {WRONLY {BINARY}}]; \
         puts -nonewline $f [binary format H* ff]; close $f; \
         interp create child; \
         child eval {fconfigure stdout -translation binary}; \
         puts -nonewline [binary format H* ff]",
        "C.UTF-8",
    );
    assert!(output.status.success(), "{:?}", output.stderr);
    assert_eq!(output.stdout, [0xff, 0xff, 0xff]);
}

#[test]
fn open_access_validation_tracks_the_runtime_release() {
    let malformed_nine = run(
        "set m \"\\{RDONLY\"; catch {open /dev/null $m} msg opts; \
         puts -nonewline [dict get $opts -errorcode]",
        "C.UTF-8",
    );
    assert!(
        malformed_nine.status.success(),
        "{:?}",
        malformed_nine.stderr
    );
    assert_eq!(malformed_nine.stdout, b"TCL OPENMODE INVALID");

    let repeated_eight = run_version(
        "set f [open /dev/null {RDONLY WRONLY}]; \
         puts -nonewline $f x; close $f; puts -nonewline ok",
        "C.UTF-8",
        "8.6",
    );
    assert!(
        repeated_eight.status.success(),
        "{:?}",
        repeated_eight.stderr
    );
    assert_eq!(repeated_eight.stdout, b"ok");

    let malformed_eight = run_version(
        "set m \"\\{RDONLY\"; catch {open /dev/null $m} msg opts; \
         puts -nonewline [dict get $opts -errorcode]",
        "C.UTF-8",
        "8.6",
    );
    assert!(
        malformed_eight.status.success(),
        "{:?}",
        malformed_eight.stderr
    );
    assert_eq!(malformed_eight.stdout, b"TCL VALUE LIST BRACE");
}

#[test]
fn mutable_system_encoding_is_shared_and_only_seeds_future_channels() {
    let path = format!("/tmp/tclvm-system-{}", std::process::id());
    let script = format!(
        "set path {path}; \
         set before [fconfigure stdout -encoding]; \
         encoding system iso8859-1; \
         interp create child; \
         set childSystem [child eval {{encoding system}}]; \
         set f [open $path w]; set opened [fconfigure $f -encoding]; \
         puts -nonewline $f [binary format H* ff]; close $f; \
         puts -nonewline [list $before [fconfigure stdout -encoding] \
             $childSystem $opened]"
    );
    let output = run(&script, "C.UTF-8");
    assert!(output.status.success(), "{:?}", output.stderr);
    assert_eq!(output.stdout, b"utf-8 utf-8 iso8859-1 iso8859-1");
    assert_eq!(std::fs::read(&path).expect("read encoded file"), [0xff]);
    std::fs::remove_file(path).expect("remove encoded file");
}

#[test]
fn binary_configuration_is_queried_and_worker_output_is_encoded_before_sharing() {
    let query = run(
        "fconfigure stdout -profile replace -encoding utf-8 -translation crlf; \
         fconfigure stdout -translation binary; \
         puts -nonewline [list [fconfigure stdout -encoding] \
             [fconfigure stdout -translation] [fconfigure stdout -profile] \
             [fconfigure stdout -eofchar]]",
        "C.UTF-8",
    );
    assert!(query.status.success(), "{:?}", query.stderr);
    assert_eq!(query.stdout, b"iso8859-1 lf replace {}");

    let worker = run(
        "set w [thread::create]; \
         thread::send $w {fconfigure stdout -translation binary; \
             puts -nonewline [binary format H* ff41]}; \
         thread::release $w",
        "C.UTF-8",
    );
    assert!(worker.status.success(), "{:?}", worker.stderr);
    assert_eq!(worker.stdout, [0xff, b'A']);
}

#[test]
fn c_locale_bootstrap_and_stderr_defaults_match_tcl_9() {
    let c_locale = run("puts -nonewline [binary format H* ff41]", "C");
    assert!(c_locale.status.success(), "{:?}", c_locale.stderr);
    assert_eq!(c_locale.stdout, [0xff, b'A']);

    let utf8_locale = run("puts -nonewline [binary format H* ff41]", "C.UTF-8");
    assert!(utf8_locale.status.success(), "{:?}", utf8_locale.stderr);
    assert_eq!(utf8_locale.stdout, [0xc3, 0xbf, b'A']);

    let stderr = run(
        "fconfigure stderr -encoding iso8859-1; \
         puts -nonewline stderr \"A\\u0178B\"",
        "C.UTF-8",
    );
    assert!(stderr.status.success(), "{:?}", stderr.stderr);
    assert!(stderr.stdout.is_empty());
    assert_eq!(stderr.stderr, b"A?B");
}

#[test]
fn cli_errors_observe_mutated_standard_error_translation() {
    let output = run(
        "fconfigure stderr -translation crlf; error \"bad\\nthing\"",
        "C.UTF-8",
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"bad\r\nthing\r\n");
}

#[test]
fn cli_strict_stderr_conversion_reports_tcl_fallback_marker() {
    let output = run(
        "fconfigure stderr -encoding iso8859-1 -translation crlf -profile strict; \
         error \"A\\u0178B\"",
        "C.UTF-8",
    );
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"A\r\n\t(encoding error in stderr)\r\n");
}

#[test]
fn background_errors_observe_mutated_standard_error_translation() {
    let output = run(
        "interp bgerror {}; fconfigure stderr -translation crlf; \
         after 0 {error \"bad\\nthing\"}; update",
        "C.UTF-8",
    );
    assert!(output.status.success(), "{:?}", output.stderr);
    assert!(
        output.stderr.windows(2).any(|pair| pair == b"\r\n"),
        "background error did not use CRLF: {:?}",
        output.stderr
    );
    assert!(
        output
            .stderr
            .iter()
            .enumerate()
            .all(|(index, byte)| *byte != b'\n' || index > 0 && output.stderr[index - 1] == b'\r'),
        "background error bypassed channel translation: {:?}",
        output.stderr
    );
    assert!(
        output.stderr.ends_with(b"    (\"after\" script)\r\n"),
        "unexpected background error: {:?}",
        output.stderr
    );
}

#[test]
fn background_strict_stderr_conversion_reports_tcl_fallback_marker() {
    let output = run(
        "interp bgerror {}; \
         fconfigure stderr -encoding iso8859-1 -translation crlf -profile strict; \
         after 0 {error \"A\\u0178B\"}; update",
        "C.UTF-8",
    );
    assert!(output.status.success(), "{:?}", output.stderr);
    assert!(output.stdout.is_empty());
    assert_eq!(output.stderr, b"A\r\n\t(encoding error in stderr)\r\n");
}
