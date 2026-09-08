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

use std::cell::RefCell;
use std::rc::Rc;

use tcl_dialect::TclVersion;
use tcl_host_native::NativeHost;
use tcl_platform::{Capabilities, Clock, Env, Filesystem, Host, Process, StdIo};
use tcl_runtime::interp::{Code, Interp};
use tcl_runtime::obj::TclObj;
use tcl_runtime::CompletionCode;

const NON_UTF8_VALUE: &[u8] = &[0xff, 0x00, b'A', 0x80];

fn binary_success(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    interp.set_result_bytes(NON_UTF8_VALUE);
    Code::Ok
}

fn binary_error(interp: &mut Interp, _argv: &[*mut TclObj]) -> Code {
    interp.set_result_bytes(NON_UTF8_VALUE);
    Code::Error
}

fn dict_value(dict: &[u8], key: &[u8]) -> Vec<u8> {
    let fields = tcl_runtime::parse::split_list(dict).expect("completion options are a Tcl dict");
    assert_eq!(
        fields.len() % 2,
        0,
        "completion options have key/value pairs"
    );
    fields
        .chunks_exact(2)
        .find_map(|pair| (pair[0] == key).then(|| pair[1].clone()))
        .unwrap_or_else(|| {
            panic!(
                "completion options are missing {}",
                String::from_utf8_lossy(key)
            )
        })
}

/// A native host whose platform facilities remain real while stdout/stderr are
/// byte captures. This exercises the same `puts` capability route as the
/// command-line runner without redirecting process-global descriptors.
struct CaptureHost {
    native: NativeHost,
    stdout: RefCell<Vec<u8>>,
}

impl CaptureHost {
    fn new() -> Self {
        Self {
            native: NativeHost::new(),
            stdout: RefCell::new(Vec::new()),
        }
    }

    fn stdout(&self) -> Vec<u8> {
        self.stdout.borrow().clone()
    }
}

impl StdIo for CaptureHost {
    fn write_stdout(&self, bytes: &[u8]) {
        self.stdout.borrow_mut().extend_from_slice(bytes);
    }

    fn write_stderr(&self, _bytes: &[u8]) {}
}

impl Host for CaptureHost {
    fn capabilities(&self) -> Capabilities {
        self.native.capabilities()
    }

    fn clock(&self) -> &dyn Clock {
        self.native.clock()
    }

    fn stdio(&self) -> &dyn StdIo {
        self
    }

    fn env(&self) -> &dyn Env {
        self.native.env()
    }

    fn filesystem(&self) -> Option<&dyn Filesystem> {
        self.native.filesystem()
    }

    fn process(&self) -> Option<&dyn Process> {
        self.native.process()
    }
}

#[test]
fn script_completion_preserves_non_utf8_success_and_error_values() {
    let mut interp = Interp::new();
    interp.register_builtin(b"binary_success", binary_success);
    interp.register_builtin(b"binary_error", binary_error);

    let success = interp.eval_completion(b"binary_success");
    assert_eq!(success.code, CompletionCode::Ok);
    assert_eq!(success.result, NON_UTF8_VALUE);
    assert!(!success.options.is_empty());

    let failure = interp.eval_sourced_completion(b"binary_error", b"binary-error.tcl");
    assert_eq!(failure.code, CompletionCode::Error);
    assert_eq!(failure.result, NON_UTF8_VALUE);
    assert!(
        failure
            .options
            .windows(NON_UTF8_VALUE.len())
            .any(|window| window == NON_UTF8_VALUE),
        "the byte-valued error must stay exact inside -errorinfo too"
    );
}

#[test]
fn raw_byte_puts_and_final_completion_share_the_lossless_boundary() {
    let host = Rc::new(CaptureHost::new());
    let mut interp = Interp::new();
    interp.set_host(host.clone());
    interp.register_builtin(b"binary_success", binary_success);

    let completion = interp.eval_completion(b"puts -nonewline [binary_success]");
    assert_eq!(completion.code, CompletionCode::Ok);
    assert!(completion.result.is_empty());
    assert_eq!(host.stdout(), NON_UTF8_VALUE);
}

#[test]
fn binary_channel_output_uses_the_shared_tcl9_conversion_boundary() {
    let host = Rc::new(CaptureHost::new());
    let mut interp = Interp::new();
    interp.set_runtime_version(TclVersion::V9_0);
    interp.set_host(host.clone());

    let completion = interp.eval_completion(
        b"fconfigure stdout -translation binary; \
          puts -nonewline [binary format H* ff41]",
    );
    assert_eq!(completion.code, CompletionCode::Ok);
    assert_eq!(host.stdout(), [0xff, b'A']);
}

#[test]
fn binary_open_mode_uses_the_shared_byte_preserving_configuration() {
    let mut interp = Interp::new();
    interp.set_runtime_version(TclVersion::V9_0);
    let stem = format!("tcl-runtime-binary-open-{}", std::process::id());
    let path = std::env::temp_dir().join(format!("{stem}-simple"));
    let list_path = std::env::temp_dir().join(format!("{stem}-list"));
    let rdwr_path = std::env::temp_dir().join(format!("{stem}-rdwr"));
    let script = format!(
        "set f [open {} wb]; \
         puts -nonewline $f [binary format H* ff41]; close $f; \
         set f [open {} {{WRONLY CREAT TRUNC {{BINARY}}}}]; \
         puts -nonewline $f [binary format H* ff41]; close $f; \
         set f [open {} {{RDWR CREAT TRUNC BINARY}}]; \
         puts -nonewline $f [binary format H* ff41]; close $f",
        tcl_syntax::list::list_element(&path.to_string_lossy()),
        tcl_syntax::list::list_element(&list_path.to_string_lossy()),
        tcl_syntax::list::list_element(&rdwr_path.to_string_lossy()),
    );
    let completion = interp.eval_completion(script.as_bytes());
    assert_eq!(completion.code, CompletionCode::Ok);
    for output in [&path, &list_path, &rdwr_path] {
        assert_eq!(
            std::fs::read(output).expect("read binary output"),
            [0xff, b'A']
        );
        std::fs::remove_file(output).expect("remove binary output");
    }
}

#[test]
fn open_access_validation_tracks_the_runtime_release() {
    let mut interp = Interp::new();
    let path =
        std::env::temp_dir().join(format!("tcl-runtime-open-release-{}", std::process::id()));
    std::fs::write(&path, b"seed").expect("create access-mode fixture");

    interp.set_runtime_version(TclVersion::V8_6);
    let repeated = interp.eval_completion(
        format!(
            "set f [open {} {{RDONLY WRONLY}}]; \
             puts -nonewline $f x; close $f",
            tcl_syntax::list::list_element(&path.to_string_lossy()),
        )
        .as_bytes(),
    );
    assert_eq!(repeated.code, CompletionCode::Ok);
    assert_eq!(
        interp
            .eval_completion(
                b"set m \"\\{RDONLY\"; catch {open ignored $m} msg opts; \
                  dict get $opts -errorcode"
            )
            .result,
        b"TCL VALUE LIST BRACE"
    );

    interp.set_runtime_version(TclVersion::V9_0);
    assert_eq!(
        interp
            .eval_completion(
                b"set m \"\\{RDONLY\"; catch {open ignored $m} msg opts; \
                  dict get $opts -errorcode"
            )
            .result,
        b"TCL OPENMODE INVALID"
    );
    std::fs::remove_file(path).expect("remove access-mode fixture");
}

#[test]
fn child_configuration_updates_the_shared_standard_channel_handle() {
    let host = Rc::new(CaptureHost::new());
    let mut interp = Interp::new();
    interp.set_runtime_version(TclVersion::V9_0);
    interp.set_host(host.clone());

    let completion = interp.eval_completion(
        b"interp create child; \
          child eval {fconfigure stdout -translation binary}; \
          puts -nonewline [binary format H* ff]",
    );
    assert_eq!(completion.code, CompletionCode::Ok);
    assert_eq!(host.stdout(), [0xff]);
}

#[test]
fn strict_conversion_writes_its_prefix_and_reports_structured_eilseq() {
    let host = Rc::new(CaptureHost::new());
    let mut interp = Interp::new();
    interp.set_runtime_version(TclVersion::V9_0);
    interp.set_host(host.clone());

    let completion = interp.eval_completion(
        b"fconfigure stdout -encoding iso8859-1 -profile strict; \
          set c [catch {puts -nonewline \"A\\u0178B\"} m o]; \
          set ::observedCode [dict get $o -errorcode]; set c",
    );
    assert_eq!(completion.code, CompletionCode::Ok);
    assert_eq!(completion.result, b"1");
    assert_eq!(host.stdout(), b"A");
    assert_eq!(interp.eval_str(b"set m"), Code::Ok);
    assert_eq!(
        interp.result_bytes(),
        b"error writing \"stdout\": invalid or incomplete multibyte or wide character"
    );
    assert_eq!(interp.eval_str(b"set ::observedCode"), Code::Ok);
    assert_eq!(
        interp.result_bytes(),
        b"POSIX EILSEQ {invalid or incomplete multibyte or wide character}"
    );
}

#[test]
fn configuration_errors_keep_their_structured_tcl_identity() {
    let mut interp = Interp::new();
    interp.set_runtime_version(TclVersion::V9_0);
    let completion = interp.eval_completion(
        b"catch {fconfigure stdout -profile bogus} m o; \
          list $m [dict get $o -errorcode]",
    );
    assert_eq!(completion.code, CompletionCode::Ok);
    assert_eq!(
        completion.result,
        b"{bad profile name \"bogus\": must be replace, strict, or tcl8} {TCL ENCODING PROFILE bogus}"
    );
    assert_eq!(
        interp
            .eval_completion(b"encoding system ascii; encoding system {}; encoding system")
            .result,
        b"iso8859-1"
    );
}

#[test]
fn mutable_system_encoding_is_shared_and_only_seeds_future_channels() {
    let host = Rc::new(CaptureHost::new());
    let mut interp = Interp::new();
    interp.set_runtime_version(TclVersion::V9_0);
    interp.set_host(host);
    let path = std::env::temp_dir().join(format!(
        "tcl-runtime-system-encoding-{}",
        std::process::id()
    ));
    let script = format!(
        "set before [fconfigure stdout -encoding]; \
         encoding system iso8859-1; \
         interp create child; \
         set childSystem [child eval {{encoding system}}]; \
         set f [open {} w]; \
         set opened [fconfigure $f -encoding]; \
         puts -nonewline $f \\u00ff; close $f; \
         list $before [fconfigure stdout -encoding] $childSystem $opened",
        tcl_syntax::list::list_element(&path.to_string_lossy()),
    );
    let completion = interp.eval_completion(script.as_bytes());
    assert_eq!(completion.code, CompletionCode::Ok);
    assert_eq!(completion.result, b"utf-8 utf-8 iso8859-1 iso8859-1");
    assert_eq!(std::fs::read(&path).expect("read encoded file"), [0xff]);
    std::fs::remove_file(path).expect("remove encoded file");
}

#[test]
fn completion_captures_live_error_options_before_publishing_globals() {
    let mut interp = Interp::new();
    interp.register_builtin(b"binary_success", binary_success);

    // Tcl 9.0.4 oracle (catching this same script in a fresh child interp):
    // 1 BOOM {-errorinfo {CUSTOM INFO} -errorcode {CUSTOM CODE} -code 1
    //         -level 0 -errorstack {} -errorline 1}
    let completion = interp.eval_completion(
        b"return -code error -level 0 -errorinfo {CUSTOM INFO} \
          -errorcode {CUSTOM CODE} [binary_success]",
    );
    assert_eq!(completion.code, CompletionCode::Error);
    assert_eq!(completion.result, NON_UTF8_VALUE);
    assert_eq!(dict_value(&completion.options, b"-code"), b"1");
    assert_eq!(dict_value(&completion.options, b"-level"), b"0");
    assert_eq!(
        dict_value(&completion.options, b"-errorcode"),
        b"CUSTOM CODE"
    );
    assert_eq!(
        dict_value(&completion.options, b"-errorinfo"),
        b"CUSTOM INFO"
    );

    // Capturing does not replace Tcl's ordinary outermost publication step.
    assert_eq!(interp.eval_str(b"set ::errorCode"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"CUSTOM CODE");
    assert_eq!(interp.eval_str(b"set ::errorInfo"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"CUSTOM INFO");
}

#[test]
fn completion_preserves_the_during_chain_before_publication() {
    let mut interp = Interp::new();
    let completion = interp.eval_completion(
        b"try {return -code error -level 0 -errorinfo {INNER INFO} \
            -errorcode {INNER CODE} INNER} \
          on error {} {return -code error -level 0 -errorinfo {OUTER INFO} \
            -errorcode {OUTER CODE} OUTER}",
    );

    // Tcl 9.0.4 reports OUTER with OUTER CODE and this exact nested exception:
    // -during {-errorinfo {INNER INFO\n    ("try" body line 1)}
    //          -errorcode {INNER CODE} -code 1 -level 0 -errorstack {}
    //          -errorline 1}
    assert_eq!(completion.code, CompletionCode::Error);
    assert_eq!(completion.result, b"OUTER");
    assert_eq!(
        dict_value(&completion.options, b"-errorcode"),
        b"OUTER CODE"
    );
    // The runtime does not yet add Tcl's try-specific errorInfo frame; the
    // boundary must nevertheless retain the live custom value byte-for-byte.
    assert_eq!(
        dict_value(&completion.options, b"-errorinfo"),
        b"OUTER INFO"
    );
    let during = dict_value(&completion.options, b"-during");
    assert_eq!(dict_value(&during, b"-code"), b"1");
    assert_eq!(dict_value(&during, b"-level"), b"0");
    assert_eq!(dict_value(&during, b"-errorcode"), b"INNER CODE");
    assert_eq!(dict_value(&during, b"-errorinfo"), b"INNER INFO");
}

#[test]
fn completion_exposes_every_tcl_completion_code_and_return_options() {
    struct Case {
        script: &'static [u8],
        code: CompletionCode,
        result: &'static [u8],
        option_code: &'static [u8],
        level: &'static [u8],
    }
    let cases = [
        Case {
            script: b"return VALUE",
            code: CompletionCode::Return,
            result: b"VALUE",
            option_code: b"0",
            level: b"1",
        },
        Case {
            script: b"break",
            code: CompletionCode::Break,
            result: b"",
            option_code: b"3",
            level: b"0",
        },
        Case {
            script: b"continue",
            code: CompletionCode::Continue,
            result: b"",
            option_code: b"4",
            level: b"0",
        },
        Case {
            script: b"return -code 37 -level 0 VALUE",
            code: CompletionCode::Other(37),
            result: b"VALUE",
            option_code: b"37",
            level: b"0",
        },
    ];

    // Exact Tcl 9.0.4 catch oracles, in case order:
    // 2 VALUE {-code 0 -level 1}
    // 3 {}    {-code 3 -level 0}
    // 4 {}    {-code 4 -level 0}
    // 37 VALUE {-code 37 -level 0}
    for case in cases {
        let completion = Interp::new().eval_completion(case.script);
        assert_eq!(
            completion.code,
            case.code,
            "script: {}",
            String::from_utf8_lossy(case.script)
        );
        assert_eq!(
            completion.result,
            case.result,
            "script: {}",
            String::from_utf8_lossy(case.script)
        );
        assert_eq!(dict_value(&completion.options, b"-code"), case.option_code);
        assert_eq!(dict_value(&completion.options, b"-level"), case.level);
    }
}

#[test]
fn sourced_completion_settles_return_before_snapshot_and_publication() {
    let mut interp = Interp::new();

    let ordinary = interp.eval_sourced_completion(b"return VALUE", b"ordinary.tcl");
    assert_eq!(ordinary.code, CompletionCode::Ok);
    assert_eq!(ordinary.result, b"VALUE");
    assert_eq!(dict_value(&ordinary.options, b"-code"), b"0");
    assert_eq!(dict_value(&ordinary.options, b"-level"), b"0");

    // Tcl 9.0.4: sourcing a file containing this command is caught as
    // `37 VALUE {-code 37 -level 0}`.
    let other =
        interp.eval_sourced_completion(b"return -code 37 -level 1 VALUE", b"other-code.tcl");
    assert_eq!(other.code, CompletionCode::Other(37));
    assert_eq!(other.result, b"VALUE");
    assert_eq!(dict_value(&other.options, b"-code"), b"37");
    assert_eq!(dict_value(&other.options, b"-level"), b"0");

    let failure = interp.eval_sourced_completion(
        b"return -code error -level 1 -errorinfo {SOURCE INFO} \
          -errorcode {SOURCE CODE} FAILURE",
        b"error-code.tcl",
    );
    assert_eq!(failure.code, CompletionCode::Error);
    assert_eq!(failure.result, b"FAILURE");
    assert_eq!(dict_value(&failure.options, b"-code"), b"1");
    assert_eq!(dict_value(&failure.options, b"-level"), b"0");
    assert_eq!(dict_value(&failure.options, b"-errorcode"), b"SOURCE CODE");
    assert_eq!(dict_value(&failure.options, b"-errorinfo"), b"SOURCE INFO");
    assert_eq!(interp.eval_str(b"set ::errorCode"), Code::Ok);
    assert_eq!(interp.result_bytes(), b"SOURCE CODE");
}
