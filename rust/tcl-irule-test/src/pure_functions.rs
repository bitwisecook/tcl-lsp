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

//! The pure iRules functions in the simulator: each registered as a host
//! command over its shared core (`tcl_cmd_core::irules::call`), the one the
//! registry's direct route runs, under the mock name
//! `::itest::cmd::register_all` looks for — so it takes the place of the
//! generated stub that returns the empty string. An input the core does not
//! model (one outside F5's published reference) falls back to that stub.

use std::rc::Rc;

use tcl_cmd_core::irules::{self, Output};
use tcl_vm::{Code, Completion, NativeCommand, Value, Vm};

/// The mock procedure name `::itest::cmd::_mock_proc_name` gives an iRules
/// command: `NS::sub` is `<ns>_<sub>` with the namespace in lower case, a
/// top-level command `cmd_<name>`, hyphens and dots as underscores.
fn mock_name(command: &str) -> String {
    let safe = |part: &str| part.replace(['-', '.'], "_");
    match command.split_once("::") {
        Some((namespace, sub)) => format!("{}_{}", safe(&namespace.to_lowercase()), safe(sub)),
        None => format!("cmd_{}", safe(command)),
    }
}

/// One pure function as a host command.
struct PureFunction {
    command: &'static str,
    mock: String,
}

impl NativeCommand for PureFunction {
    fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        let texts: Vec<Rc<str>> = args.iter().map(Value::to_str).collect();
        let words: Vec<&str> = texts.iter().map(AsRef::as_ref).collect();
        let value = match irules::call(self.command, &words) {
            Ok(Output::Text(text)) => Value::string(text),
            Ok(Output::Int(value)) => Value::int(value),
            Ok(Output::Bytes(bytes)) => {
                Value::string(bytes.iter().copied().map(char::from).collect::<String>())
            }
            Err(irules::Unmodelled) => return self.stub(vm, args),
        };
        Completion::new(Code::Ok, value, Value::empty())
    }
}

impl PureFunction {
    /// The generated stub's answer for an input the core does not model: the
    /// stub `register_all` would have registered, when the table names one.
    fn stub(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        let table = format!("::itest::cmd::_stub_actions({})", self.mock);
        let action = vm
            .read_variable(&table)
            .ok()
            .and_then(|action| action.as_list().ok());
        let Some(action) = action else {
            return Completion::new(Code::Ok, Value::empty(), Value::empty());
        };
        let mut words: Vec<Value> = action.iter().cloned().collect();
        words.extend_from_slice(args);
        vm.invoke_command("::itest::cmd::_stub", &words)
    }
}

/// Register every pure function the shared cores run, under its mock name,
/// before `::orch::init` registers the commands.
pub(crate) fn register(vm: &mut Vm) {
    for &command in irules::COMMANDS {
        let mock = mock_name(command);
        let name = format!("::itest::cmd::{mock}");
        vm.register_native_command(&name, Rc::new(PureFunction { command, mock }));
    }
}

#[cfg(test)]
mod tests {
    use std::fmt::Write as _;
    use std::path::PathBuf;

    use tcl_dialect::{DialectProfile, TclVersion};
    use tcl_registry::CommandRegistry;
    use tcl_registry::value_transfer::{
        Budget, EvalAnswer, ExactValueOrUnavailable, LiteralInputs, resolve_semantics,
    };
    use tcl_syntax::list::{join_list, list_element};

    use crate::LiveSession;

    /// How a vector's answer reads: text, or bytes written as hex.
    #[derive(Clone, Copy, PartialEq, Eq)]
    enum Reads {
        Text,
        Hex,
    }

    /// The published vectors: each call's words, what the reference gives
    /// (`clouddocs.f5.com/api/irules/<command>.html`, or the standard it
    /// names — RFC 4648 § 10, the CRC catalogue's `CRC-32` check, RFC 1321
    /// § A.5, FIPS 180's examples), and how it reads.
    const VECTORS: &[(&str, &[&str], &str, Reads)] = &[
        ("b64encode", &[""], "", Reads::Text),
        ("b64encode", &["f"], "Zg==", Reads::Text),
        ("b64encode", &["fo"], "Zm8=", Reads::Text),
        ("b64encode", &["foo"], "Zm9v", Reads::Text),
        ("b64encode", &["foob"], "Zm9vYg==", Reads::Text),
        ("b64encode", &["fooba"], "Zm9vYmE=", Reads::Text),
        ("b64encode", &["foobar"], "Zm9vYmFy", Reads::Text),
        ("b64encode", &["abc"], "YWJj", Reads::Text),
        ("b64decode", &["Zm9vYmFy"], "foobar", Reads::Text),
        ("b64decode", &["Zm9vYg=="], "foob", Reads::Text),
        ("b64decode", &["Zg=="], "f", Reads::Text),
        ("crc32", &["123456789"], "-873187034", Reads::Text),
        ("crc32", &["abc"], "891568578", Reads::Text),
        ("crc32", &[""], "0", Reads::Text),
        ("md5", &[""], "d41d8cd98f00b204e9800998ecf8427e", Reads::Hex),
        (
            "md5",
            &["a"],
            "0cc175b9c0f1b6a831c399e269772661",
            Reads::Hex,
        ),
        (
            "md5",
            &["abc"],
            "900150983cd24fb0d6963f7d28e17f72",
            Reads::Hex,
        ),
        (
            "md5",
            &["message digest"],
            "f96b697d7cb7938d525a2f31aaf161d0",
            Reads::Hex,
        ),
        (
            "sha1",
            &["abc"],
            "a9993e364706816aba3e25717850c26c9cd0d89d",
            Reads::Hex,
        ),
        (
            "sha1",
            &["abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"],
            "84983e441c3bd26ebaae4aa1f95129e5e54670f1",
            Reads::Hex,
        ),
        (
            "sha256",
            &["abc"],
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            Reads::Hex,
        ),
        (
            "sha384",
            &["abc"],
            "cb00753f45a35e8bb5a03d699ac65007272c32ab0eded1631a8b605a43ff5bed\
             8086072ba1e7cc2358baeca134c825a7",
            Reads::Hex,
        ),
        (
            "sha512",
            &["abc"],
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
             2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
            Reads::Hex,
        ),
        (
            "findstr",
            &["<sip:+12065551234@sip.example.com>", "@", "1", ">"],
            "sip.example.com",
            Reads::Text,
        ),
        (
            "findstr",
            &["aaa123456xxyz", "aaa", "3", "xyz"],
            "123456x",
            Reads::Text,
        ),
        (
            "findstr",
            &[
                "<meta HTTP-EQUIV=\"REFRESH\" CONTENT=\"0; URL=https://host.domain.com/path/file.ext?...&var=val\">",
                "URL=",
                "4",
                "\">",
            ],
            "https://host.domain.com/path/file.ext?...&var=val",
            Reads::Text,
        ),
        (
            "substr",
            &["abcdefghijklm", "2", "x"],
            "cdefghijklm",
            Reads::Text,
        ),
        ("substr", &["abcdefghijklm", "2", "gh"], "cdef", Reads::Text),
        ("substr", &["abcdefghijklm", "2", "4"], "cdef", Reads::Text),
        (
            "substr",
            &["abcdefghijklm", "2", "20"],
            "cdefghijklm",
            Reads::Text,
        ),
        (
            "getfield",
            &["www.example.com:8080", ":", "1"],
            "www.example.com",
            Reads::Text,
        ),
        (
            "getfield",
            &["www.example.com:8080", ":", "2"],
            "8080",
            Reads::Text,
        ),
        (
            "getfield",
            &["sub.domain.com", ".domain.com", "1"],
            "sub",
            Reads::Text,
        ),
        (
            "domain",
            &["www.sub.my.domain.com", "1"],
            "com",
            Reads::Text,
        ),
        (
            "domain",
            &["www.sub.my.domain.com", "2"],
            "domain.com",
            Reads::Text,
        ),
        (
            "domain",
            &["www.sub.my.domain.com", "3"],
            "my.domain.com",
            Reads::Text,
        ),
        (
            "domain",
            &["www.sub.my.domain.com", "4"],
            "sub.my.domain.com",
            Reads::Text,
        ),
        (
            "URI::basename",
            &["/main/index.jsp?user=test&login=check"],
            "index.jsp",
            Reads::Text,
        ),
        (
            "URI::path",
            &["/path/to/file.ext?param=value"],
            "/path/to/",
            Reads::Text,
        ),
        (
            "URI::path",
            &["/path/to/file.ext?param=value", "depth"],
            "2",
            Reads::Text,
        ),
        (
            "URI::path",
            &["/path/to/file.ext?param=value", "1"],
            "/path/to/",
            Reads::Text,
        ),
        (
            "URI::path",
            &["/path/to/file.ext?param=value", "2"],
            "/to/",
            Reads::Text,
        ),
        (
            "URI::path",
            &["/path/to/file.ext?param=value", "1", "3"],
            "/path/to/",
            Reads::Text,
        ),
        (
            "URI::query",
            &["/path/to/file.ext?param1=value1&param2=value2"],
            "param1=value1&param2=value2",
            Reads::Text,
        ),
        (
            "URI::query",
            &["/path/to/file.ext?param1=value1&param2=value2", "param2"],
            "value2",
            Reads::Text,
        ),
        (
            "URI::query",
            &["?param1=val1&param2=val2", "param1"],
            "val1",
            Reads::Text,
        ),
        (
            "URI::query",
            &["param1=val1&param2=val2", "param1"],
            "",
            Reads::Text,
        ),
        (
            "URI::host",
            &["http://example.com/file.ext"],
            "example.com",
            Reads::Text,
        ),
        (
            "URI::host",
            &["http://example.com:80/file.ext"],
            "example.com",
            Reads::Text,
        ),
        (
            "URI::host",
            &["https://example.com:443/file.ext"],
            "example.com",
            Reads::Text,
        ),
        ("URI::host", &["/example.com"], "", Reads::Text),
        (
            "URI::host",
            &["/uri?url=http://example.com/uri"],
            "",
            Reads::Text,
        ),
        (
            "URI::port",
            &["http://example.com/file.ext"],
            "80",
            Reads::Text,
        ),
        (
            "URI::port",
            &["https://example.com:443/file.ext"],
            "443",
            Reads::Text,
        ),
        (
            "URI::port",
            &["https://example.com:8443/file.ext"],
            "8443",
            Reads::Text,
        ),
        (
            "URI::port",
            &["ftp://example.com/file.ext"],
            "21",
            Reads::Text,
        ),
        (
            "URI::port",
            &["sip://example.com/file.ext"],
            "5060",
            Reads::Text,
        ),
        ("URI::port", &["/example.com"], "80", Reads::Text),
        (
            "URI::port",
            &["/uri?url=http://example.com/uri"],
            "80",
            Reads::Text,
        ),
        ("URI::protocol", &["http://test.com"], "http", Reads::Text),
        ("URI::protocol", &["https://test.com"], "https", Reads::Text),
        (
            "URI::protocol",
            &["myproto://test.com"],
            "myproto",
            Reads::Text,
        ),
        ("URI::protocol", &["/test.com"], "", Reads::Text),
        (
            "URI::protocol",
            &["/uri?url=http://test.example.com/uri"],
            "",
            Reads::Text,
        ),
        (
            "URI::decode",
            &[
                "parameter=my%20URL%20encoded%20parameter%20value%20with%20metacharacters%20(%26*%40%23%5b%5d)",
            ],
            "parameter=my URL encoded parameter value with metacharacters (&*@#[])",
            Reads::Text,
        ),
        (
            "URI::encode",
            &["my URL encoded parameter value with metacharacters (&*@#)"],
            "my%20URL%20encoded%20parameter%20value%20with%20metacharacters%20(%26*%40%23)",
            Reads::Text,
        ),
        (
            "URI::compare",
            &["/dir1/somepath", "/dir1/somepath"],
            "1",
            Reads::Text,
        ),
        (
            "URI::compare",
            &["/dir1/somepath", "/dir1/otherpath"],
            "0",
            Reads::Text,
        ),
        (
            "IP::addr",
            &["10.42.2.0/24", "equals", "10.42.2.1"],
            "1",
            Reads::Text,
        ),
        (
            "IP::addr",
            &["10.42.2.2", "equals", "10.42.2.0/24"],
            "1",
            Reads::Text,
        ),
        (
            "IP::addr",
            &["10.42.2.2", "equals", "10.42.3.2"],
            "0",
            Reads::Text,
        ),
    ];

    /// Calls outside what the reference states: the route declines each,
    /// and the simulator answers with the generated stub's empty string.
    const UNMODELLED: &[(&str, &[&str])] = &[
        // TMM raises `conversion error`, which the reference does not word.
        ("b64decode", &["\\a b c d"]),
        // "this does not really work" in 11.5.4 and 11.6.0.
        ("substr", &["abcdefghijklm", "2", "0"]),
        // A scheme the reference lists no default port for.
        ("URI::port", &["myproto://example.com/file.ext"]),
        ("getfield", &["a:b", ":", "3"]),
        ("URI::path", &["/path/to/file.ext", "0"]),
        ("IP::addr", &["10.0.0.0/8", "equals", "10.0.0.0/16"]),
    ];

    fn lib_dir() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tcl")
            .canonicalize()
            .expect("orchestrator tcl dir")
    }

    /// The registry's route for the call under the iRules profile: its
    /// answer's text, or `None` when it declines.
    fn route(reg: &CommandRegistry, command: &str, words: &[&str]) -> Option<String> {
        let spec = reg.get(command).expect(command);
        let resolved = resolve_semantics(spec, None, None);
        let semantics = resolved.semantics().expect("a declared route");
        let inputs = LiteralInputs::new(command, None, words, Some(DialectProfile::irules()));
        match semantics.evaluate(&inputs, &mut Budget::evaluation()) {
            EvalAnswer::Evaluated(outcome) => match outcome.result {
                ExactValueOrUnavailable::Exact(value) => {
                    Some(String::from_utf8(value.bytes).expect("text"))
                }
                ExactValueOrUnavailable::Unavailable(_) => None,
            },
            EvalAnswer::Pending | EvalAnswer::Declined(_) => None,
        }
    }

    /// Bytes held as characters U+0000 to U+00FF, written as hex.
    fn hex(text: &str) -> String {
        text.chars().fold(String::new(), |mut hex, c| {
            let _ = write!(hex, "{:02x}", u32::from(c));
            hex
        })
    }

    fn call_script(command: &str, words: &[&str]) -> String {
        let mut call = vec![command.to_owned()];
        call.extend(words.iter().map(|word| (*word).to_owned()));
        join_list(&call)
    }

    /// The iRules pure functions over their shared cores
    /// (`tcl_cmd_core::irules`): the simulator's host command and the
    /// registry's direct route give one answer, the reference's, for every
    /// published vector; outside the reference the route declines and the
    /// simulator keeps the generated stub; and the two Tcl-expressible ones
    /// match `tclsh` — `b64encode` as `binary encode base64`, `crc32` as `zlib
    /// crc32` masked to 32 bits (8.6 on, where Tcl has both).
    #[test]
    fn irules_pure_functions_match_the_simulator_and_the_oracle() {
        let mut session = LiveSession::new(&lib_dir()).expect("session");
        let profile = DialectProfile::irules();
        let mut reg = CommandRegistry::build_default();
        for &layer in profile.base_layers {
            reg.load_surface(layer);
        }
        let mut covered: Vec<&str> = Vec::new();
        for &(command, words, want, reads) in VECTORS {
            let script = call_script(command, words);
            let simulated = session.eval(&script).expect(&script);
            let routed = route(&reg, command, words).unwrap_or_else(|| {
                panic!("the route declines `{script}`, whose answer is published")
            });
            assert_eq!(
                simulated, routed,
                "simulator and route differ on `{script}`"
            );
            let seen = match reads {
                Reads::Text => routed,
                Reads::Hex => hex(&routed),
            };
            assert_eq!(seen, want, "`{script}`");
            covered.push(command);
        }
        for command in tcl_cmd_core::irules::COMMANDS {
            assert!(
                covered.contains(command),
                "`{command}` has no published vector"
            );
        }
        for &(command, words) in UNMODELLED {
            let script = call_script(command, words);
            assert_eq!(
                route(&reg, command, words),
                None,
                "`{script}` is unmodelled"
            );
            session.eval("::itest::reset_decisions").expect("reset");
            assert_eq!(session.eval(&script).expect(&script), "", "`{script}`");
        }

        let samples = [
            "",
            "a",
            "abc",
            "123456789",
            "foobar",
            "The quick brown fox jumps over the lazy dog",
            "~!@#$%^&*()_+`-={}|[]\\:\";'<>?,./",
        ];
        let mut compared = 0usize;
        for version in [TclVersion::V8_6, TclVersion::V9_0, TclVersion::V9_1] {
            let Some(tclsh) = tcl_test_support::witness_tclsh(version) else {
                continue;
            };
            for sample in samples {
                let script = format!(
                    "puts b64=[binary encode base64 {word}]\nputs crc=[zlib crc32 {word}]\n",
                    word = list_element(sample)
                );
                let outcome =
                    tcl_test_support::run_script(&tclsh.path, script.as_bytes()).expect("tclsh");
                let text = outcome.strict_text().expect("tclsh answers");
                let answer = |prefix: &str| {
                    text.lines()
                        .find_map(|line| line.strip_prefix(prefix))
                        .map(str::to_owned)
                };
                let base64 = route(&reg, "b64encode", &[sample]).expect("b64encode");
                assert_eq!(Some(base64), answer("b64="), "b64encode {sample:?}");
                let crc = route(&reg, "crc32", &[sample]).expect("crc32");
                let unsigned = crc.parse::<i64>().expect("an integer") & 0xffff_ffff;
                assert_eq!(
                    Some(unsigned.to_string()),
                    answer("crc="),
                    "crc32 {sample:?}"
                );
                compared += 1;
            }
        }
        eprintln!("compared {compared} samples against tclsh");
    }
}
