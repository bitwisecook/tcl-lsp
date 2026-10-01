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

//! The capability seam, proven with real hosts over the VM's real `ValueOps`.
//!
//! The same `tcl-cmd-core::platform` command body runs against a fully-capable
//! `NativeHost` and a sandboxed one (subprocess capability off — the posture a
//! WASM/WASI host has). The native host executes; the sandboxed host yields the
//! faithful "unsupported" error rather than panicking — the native-vs-WASM
//! divergence the architecture is built around.

use tcl_cmd_core::platform;
use tcl_vm::host_native::NativeHost;
use tcl_vm::{Value, Vm};

#[test]
fn exec_runs_on_native_host() {
    let mut vm = Vm::new();
    let host = NativeHost::new();
    let args = [Value::string("echo"), Value::string("hello")];
    let result = platform::exec(&mut vm, &host, &args).expect("native host should run exec");
    assert_eq!(&*result.to_str(), "hello");
}

#[test]
fn exec_unsupported_on_sandboxed_host() {
    let mut vm = Vm::new();
    // The conditional `process` capability is off — exactly what `Host::process`
    // returns `None` for on every WASM target.
    let host = NativeHost::sandboxed();
    let args = [Value::string("echo"), Value::string("hello")];
    let err =
        platform::exec(&mut vm, &host, &args).expect_err("a sandboxed host has no subprocess");
    assert!(
        err.message().contains("no subprocess support"),
        "expected the faithful unsupported error, got: {}",
        err.message()
    );
}

#[test]
fn pwd_reads_working_directory() {
    let mut vm = Vm::new();
    let host = NativeHost::new();
    let result = platform::pwd(&mut vm, &host).expect("pwd should resolve");
    // An absolute path on the native (unix) host.
    assert!(result.to_str().starts_with('/'), "got: {}", result.to_str());
}

/// A host whose files are a table in memory, or no files at all: the
/// filesystem seam `source` reads through, with nothing on disk behind it.
mod seam {
    use std::collections::BTreeMap;
    use tcl_platform::{
        Capabilities, Clock, Env, Filesystem, Host, HostError, Metadata, StdIo, SystemEncoding,
    };
    use tcl_vm::host_native::NativeHost;

    pub struct Files(pub BTreeMap<String, Vec<u8>>);

    impl Filesystem for Files {
        fn exists(&self, path: &str) -> bool {
            self.0.contains_key(path)
        }
        fn metadata(&self, _path: &str) -> Result<Metadata, HostError> {
            Err(HostError::Unsupported)
        }
        fn read(&self, path: &str) -> Result<Vec<u8>, HostError> {
            self.0.get(path).cloned().ok_or(HostError::NotFound)
        }
        fn write(&self, _path: &str, _data: &[u8]) -> Result<(), HostError> {
            Err(HostError::Unsupported)
        }
        fn read_dir(&self, _path: &str) -> Result<Vec<String>, HostError> {
            Err(HostError::Unsupported)
        }
        fn create_dir_all(&self, _path: &str) -> Result<(), HostError> {
            Err(HostError::Unsupported)
        }
        fn remove(&self, _path: &str, _recursive: bool) -> Result<(), HostError> {
            Err(HostError::Unsupported)
        }
    }

    pub struct Seam {
        inner: NativeHost,
        files: Option<Files>,
        system: SystemEncoding,
    }

    impl Seam {
        pub fn with_files(files: &[(&str, &[u8])]) -> Self {
            Self {
                inner: NativeHost::new(),
                files: Some(Files(
                    files
                        .iter()
                        .map(|(path, bytes)| ((*path).to_owned(), bytes.to_vec()))
                        .collect(),
                )),
                system: SystemEncoding::Utf8,
            }
        }

        pub fn without_files() -> Self {
            Self {
                inner: NativeHost::new(),
                files: None,
                system: SystemEncoding::Utf8,
            }
        }

        pub fn system_encoding_is(mut self, system: SystemEncoding) -> Self {
            self.system = system;
            self
        }
    }

    impl Host for Seam {
        fn capabilities(&self) -> Capabilities {
            Capabilities::empty()
        }
        fn clock(&self) -> &dyn Clock {
            self.inner.clock()
        }
        fn stdio(&self) -> &dyn StdIo {
            self.inner.stdio()
        }
        fn env(&self) -> &dyn Env {
            self.inner.env()
        }
        fn system_encoding(&self) -> SystemEncoding {
            self.system
        }
        fn filesystem(&self) -> Option<&dyn Filesystem> {
            self.files.as_ref().map(|files| files as &dyn Filesystem)
        }
    }
}

fn vm_on(host: seam::Seam) -> Vm {
    let mut vm = Vm::new();
    vm.set_compiler(Box::new(
        tcl_compiler::compile_service::BytecodeCompileService::default(),
    ));
    vm.set_host(std::rc::Rc::new(host));
    vm
}

fn eval(vm: &mut Vm, script: &str) -> tcl_runtime_api::Completion<Value> {
    vm.eval_source(script).expect("the script compiles")
}

/// `source` reads through the host's filesystem and not the operating
/// system's: a file that exists only in the host's table is sourced, a path
/// the host does not hold is Tcl's `couldn't read file` with the POSIX reason,
/// and a host with no filesystem reads nothing.
#[test]
fn source_reads_through_the_hosts_filesystem() {
    let mut vm = vm_on(seam::Seam::with_files(&[(
        "/virtual/lib.tcl",
        b"proc virtual_double {x} {expr {$x * 2}}\nset sourced here",
    )]));
    let sourced = eval(&mut vm, "source /virtual/lib.tcl");
    assert_eq!(sourced.code, tcl_runtime_api::Code::Ok);
    assert_eq!(sourced.result.to_str().as_ref(), "here");
    assert_eq!(
        eval(&mut vm, "virtual_double 21").result.to_str().as_ref(),
        "42"
    );

    let missing = eval(&mut vm, "source /virtual/absent.tcl");
    assert_eq!(missing.code, tcl_runtime_api::Code::Error);
    assert_eq!(
        missing.result.to_str().as_ref(),
        "couldn't read file \"/virtual/absent.tcl\": no such file or directory"
    );

    let mut bare = vm_on(seam::Seam::without_files());
    let refused = eval(&mut bare, "source /virtual/lib.tcl");
    assert_eq!(refused.code, tcl_runtime_api::Code::Error);
    assert_eq!(
        refused.result.to_str().as_ref(),
        "couldn't read file \"/virtual/lib.tcl\": no such file or directory"
    );
}

/// `source -encoding` decodes the file as the encoding it names: the same
/// bytes are `é` as ISO-8859-1 and a replacement character as UTF-8, a name
/// the VM does not know is Tcl's `unknown encoding`, and `-nopkg` may stand
/// beside it in either order.
#[test]
fn source_honours_its_encoding_option() {
    let bytes: &[u8] = b"set r [string length \"\xe9\"]\nscan [string index \"\xe9\" 0] %c";
    let mut vm = vm_on(seam::Seam::with_files(&[("/virtual/latin1.tcl", bytes)]));
    let as_latin1 = eval(&mut vm, "source -encoding iso8859-1 /virtual/latin1.tcl");
    assert_eq!(as_latin1.code, tcl_runtime_api::Code::Ok);
    assert_eq!(as_latin1.result.to_str().as_ref(), "233");

    let as_utf8 = eval(&mut vm, "source -encoding utf-8 /virtual/latin1.tcl");
    assert_eq!(as_utf8.result.to_str().as_ref(), "65533");
    let default = eval(&mut vm, "source /virtual/latin1.tcl");
    assert_eq!(
        default.result.to_str().as_ref(),
        "65533",
        "Tcl 9 reads UTF-8"
    );

    for form in [
        "source -nopkg -encoding iso8859-1 /virtual/latin1.tcl",
        "source -encoding iso8859-1 -nopkg /virtual/latin1.tcl",
    ] {
        assert_eq!(
            eval(&mut vm, form).result.to_str().as_ref(),
            "233",
            "{form}"
        );
    }

    let unknown = eval(&mut vm, "source -encoding klingon /virtual/latin1.tcl");
    assert_eq!(unknown.code, tcl_runtime_api::Code::Error);
    assert_eq!(
        unknown.result.to_str().as_ref(),
        "unknown encoding \"klingon\""
    );

    let bare = eval(&mut vm, "source -encoding utf-8");
    assert_eq!(bare.code, tcl_runtime_api::Code::Error);
    assert_eq!(
        bare.result.to_str().as_ref(),
        "wrong # args: should be \"source ?-encoding name? ?-nopkg? fileName\""
    );
}

/// Tcl 9 reads a script as UTF-8 whatever the host's system encoding is; before
/// it the default is the system encoding, which the host states.
#[test]
fn source_defaults_to_the_system_encoding_before_tcl_nine() {
    let bytes: &[u8] = b"scan [string index \"\xe9\" 0] %c";
    let mut nine = vm_on(
        seam::Seam::with_files(&[("/virtual/latin1.tcl", bytes)])
            .system_encoding_is(tcl_platform::SystemEncoding::Iso88591),
    );
    let current = eval(&mut nine, "source /virtual/latin1.tcl");
    assert_eq!(current.result.to_str().as_ref(), "65533");

    let mut vm = vm_on(
        seam::Seam::with_files(&[("/virtual/latin1.tcl", bytes)])
            .system_encoding_is(tcl_platform::SystemEncoding::Iso88591),
    );
    vm.set_runtime_version(tcl_dialect::TclVersion::V8_6);
    let legacy = eval(&mut vm, "source /virtual/latin1.tcl");
    assert_eq!(legacy.result.to_str().as_ref(), "233");
}
