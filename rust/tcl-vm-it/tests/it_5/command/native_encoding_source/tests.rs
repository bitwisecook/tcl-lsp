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

//! `source` and `encoding system` over a supplied host's encoding stages.

use crate::prelude::*;
use tcl_platform::SystemEncoding;
use tcl_test_support::encoding_names::{EncodingHost, EncodingStages};

fn supplied_with(engine: &str, default: SystemEncoding, script: &[u8]) -> (Vm, Rc<EncodingStages>) {
    let mut vm = crate::fixture::interpreter(tcl_vm::test_support::profile(engine));
    let stages = Rc::new(EncodingStages::default());
    let host = EncodingHost::new(
        Rc::new(tcl_vm::host_native::NativeHost::new()),
        Rc::clone(&stages),
        default,
        script.to_vec(),
    );
    vm.set_host(Rc::new(host));
    stages.events.borrow_mut().clear();
    (vm, stages)
}

fn supplied(engine: &str, script: &[u8]) -> (Vm, Rc<EncodingStages>) {
    supplied_with(engine, SystemEncoding::Identity, script)
}

fn words(words: &[&[u8]]) -> Vec<Value> {
    words
        .iter()
        .map(|word| Value::new_native_string_bytes(*word))
        .collect()
}

fn source(vm: &mut Vm, arguments: &[&[u8]]) -> Completion<Value> {
    crate::fixture::call(vm, "source", &words(arguments))
}

fn set_system(vm: &mut Vm, name: &[u8]) -> Completion<Value> {
    crate::fixture::call(vm, "encoding", &words(&[b"system", name]))
}

fn effect(vm: &Vm, name: &str) -> Option<Vec<u8>> {
    vm.get_var(name)
        .map(|value| crate::fixture::bytes(vm, &value))
}

#[test]
fn original_source_opens_before_lookup_and_consumes_selected_encoding() {
    // naming.encoding.original-byte-name-lookup-and-source-order
    // docs/design/analysis/name-resolution-proofs/encoding-original-byte-name-lookup-and-source-order.md
    let (mut vm, stages) = supplied("tcl8.6", b"set ::encoded \xe9; return YES");
    let completion = source(&mut vm, &[b"-encoding", b"iso8859-1\0TAIL", b"/source.tcl"]);
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(completion.result.to_str().as_ref(), "YES");
    assert_eq!(vm.get_var("::encoded").unwrap().to_str().as_ref(), "é");
    assert_eq!(
        stages.events.borrow().as_slice(),
        ["filesystem", "open", "read", "close"]
    );
    stages.events.borrow_mut().clear();
    let completion = source(&mut vm, &[b"-encoding", b"\xff", b"/absent.tcl"]);
    assert_eq!(completion.code, Code::Error);
    assert!(crate::fixture::bytes(&vm, &completion.result).starts_with(b"couldn't read file"));
    assert_eq!(stages.events.borrow().as_slice(), ["filesystem", "open"]);
}

#[test]
fn original_filename_utf8_conversion_preserves_counted_header_and_lookup_effects() {
    // naming.source.original-counted-filename-filesystem-extent
    // docs/design/analysis/name-resolution-proofs/source-original-counted-filename-filesystem-extent.md
    for engine in ["tcl8.4", "tcl8.6", "tcl9.1", "jim"] {
        for suffix in [
            b"".as_slice(),
            b"\0TAIL".as_slice(),
            b"\xc0\x80TAIL".as_slice(),
        ] {
            let (mut vm, stages) = supplied(engine, b"set ::filename_effect YES; return DONE");
            if engine != "jim" {
                assert_eq!(set_system(&mut vm, b"utf-8").code, Code::Ok);
            }
            let original = [b"/source.tcl".as_slice(), suffix].concat();
            let completion = source(&mut vm, &[original.as_slice()]);
            let success =
                suffix.is_empty() || engine == "tcl8.4" || (engine == "jim" && suffix == b"\0TAIL");
            if success {
                assert_eq!(completion.code, Code::Ok, "{engine}");
                assert_eq!(effect(&vm, "::filename_effect").unwrap(), b"YES");
                assert_eq!(
                    stages.events.borrow().as_slice(),
                    ["filesystem", "open", "read", "close"]
                );
            } else {
                assert_eq!(completion.code, Code::Error, "{engine}");
                assert!(effect(&vm, "::filename_effect").is_none());
                let reason = if engine == "jim" {
                    b"No such file or directory".as_slice()
                } else {
                    b"invalid argument".as_slice()
                };
                assert_eq!(
                    crate::fixture::bytes(&vm, &completion.result),
                    [
                        b"couldn't read file \"".as_slice(),
                        tcl_core_types::c_string_extent(&original),
                        b"\": ",
                        reason
                    ]
                    .concat()
                );
                assert_eq!(
                    stages.events.borrow().as_slice(),
                    if engine == "jim" {
                        ["filesystem", "open"].as_slice()
                    } else {
                        [].as_slice()
                    }
                );
            }
        }
    }
}

#[test]
fn original_filename_actual_converter_and_unavailable_capability_stay_distinct() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    for available in [true, false] {
        let (mut vm, stages) = supplied("tcl8.6", b"set ::filename_effect YES; return DONE");
        assert_eq!(set_system(&mut vm, b"ascii").code, Code::Ok);
        if available {
            *stages.filename_conversion.borrow_mut() = Some(Ok(b"/source.tcl".to_vec()));
        }
        let completion = vm.try_invoke_command("source", &words(&[b"supplied-name"]));
        if available {
            assert_eq!(completion.unwrap().code, Code::Ok);
            assert_eq!(effect(&vm, "::filename_effect").unwrap(), b"YES");
            assert_eq!(
                stages.events.borrow().as_slice(),
                [
                    "filesystem",
                    "filename-conversion",
                    "filesystem",
                    "open",
                    "read",
                    "close"
                ]
            );
        } else {
            assert_eq!(
                completion.err(),
                Some(tcl_runtime_api::NativeExecutionError::ValueAccessRefusal(
                    tcl_syntax::raw_string::NativeValueAccessRefusal::CommandProtocolUnavailable(
                        "source filesystem filename conversion",
                    ),
                ))
            );
            assert!(effect(&vm, "::filename_effect").is_none());
            assert_eq!(
                stages.events.borrow().as_slice(),
                ["filesystem", "filename-conversion"]
            );
        }
    }
}

#[test]
fn original_filename_iso88591_host_default_keeps_plain_source_working() {
    // naming.source.original-counted-filename-filesystem-extent
    // docs/design/analysis/name-resolution-proofs/source-original-counted-filename-filesystem-extent.md
    let (mut vm, stages) = supplied_with(
        "tcl8.6",
        SystemEncoding::Iso88591,
        b"set ::filename_effect YES; return DONE",
    );
    let completion = source(&mut vm, &[b"/source.tcl"]);
    assert_eq!(completion.code, Code::Ok);
    assert_eq!(effect(&vm, "::filename_effect").unwrap(), b"YES");
    assert_eq!(
        stages.events.borrow().as_slice(),
        ["filesystem", "open", "read", "close"]
    );
}
