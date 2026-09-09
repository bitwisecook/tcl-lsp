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

#![cfg(have_tommath)]

use std::cell::RefCell;
use std::path::Path;
use std::rc::Rc;

use tcl_dialect::TclVersion;
use tcl_host_native::NativeHost;
use tcl_platform::{Capabilities, Clock, Env, Filesystem, Host, HostError, Process, StdIo};
use tcl_runtime::interp::{Code, Interp};
use tcl_test_support::{locate_source_tree, upstream_test_definition};

struct CaptureHost {
    native: NativeHost,
    stdout: RefCell<Vec<u8>>,
    tcl_library: String,
}

impl CaptureHost {
    fn new(tcl_library: String) -> Self {
        Self {
            native: NativeHost::new(),
            stdout: RefCell::new(Vec::new()),
            tcl_library,
        }
    }
}

impl StdIo for CaptureHost {
    fn write_stdout(&self, bytes: &[u8]) {
        self.stdout.borrow_mut().extend_from_slice(bytes);
    }

    fn write_stderr(&self, _bytes: &[u8]) {}
}

impl Env for CaptureHost {
    fn get(&self, key: &str) -> Option<String> {
        if key == "TCL_LIBRARY" {
            Some(self.tcl_library.clone())
        } else {
            self.native.env().get(key)
        }
    }

    fn set(&self, key: &str, value: &str) {
        self.native.env().set(key, value);
    }

    fn vars(&self) -> Vec<(String, String)> {
        let mut vars = self.native.env().vars();
        vars.retain(|(key, _)| key != "TCL_LIBRARY");
        vars.push(("TCL_LIBRARY".to_owned(), self.tcl_library.clone()));
        vars
    }

    fn cwd(&self) -> Result<String, HostError> {
        self.native.env().cwd()
    }

    fn chdir(&self, path: &str) -> Result<(), HostError> {
        self.native.env().chdir(path)
    }

    fn current_exe(&self) -> Option<String> {
        self.native.env().current_exe()
    }
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
        self
    }

    fn system_encoding(&self) -> tcl_platform::SystemEncoding {
        self.native.system_encoding()
    }

    fn filesystem(&self) -> Option<&dyn Filesystem> {
        self.native.filesystem()
    }

    fn process(&self) -> Option<&dyn Process> {
        self.native.process()
    }
}

#[test]
fn upstream_channel_output_cases_use_real_tcltest() {
    let repository = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let Some(source_tree) = locate_source_tree(&repository, TclVersion::V9_0, None)
        .expect("Tcl 9 source tree discovery")
    else {
        eprintln!("skipping: no Tcl 9.0.4 source tree available");
        return;
    };
    assert_eq!(source_tree.patchlevel, "9.0.4", "real-library oracle pin");

    let io_source = std::fs::read_to_string(source_tree.tests_dir().join("io.test"))
        .expect("read pinned io.test");
    let io_39_14 = upstream_test_definition(&io_source, "test io-39.14 {", "test io-39.15 {")
        .expect("extract io-39.14");
    let io_39_15 = upstream_test_definition(&io_source, "test io-39.15 {", "test io-39.16 {")
        .expect("extract io-39.15");

    let fixture =
        std::env::temp_dir().join(format!("tcl-runtime-channel-oracle-{}", std::process::id()));
    std::fs::create_dir_all(&fixture).expect("create channel oracle fixture");
    let path1 = fixture.join("io-39.bin");
    let path2 = fixture.join("io-75.bin");
    let host = Rc::new(CaptureHost::new(
        source_tree.library_dir().to_string_lossy().into_owned(),
    ));
    let mut interp = Interp::with_host(host.clone());
    assert_eq!(
        interp.eval_str(b"if {1} {set ::channel_oracle_ready 1}"),
        Code::Ok,
        "core command surface failed before init: {}",
        String::from_utf8_lossy(&interp.result_bytes())
    );
    assert_eq!(
        interp.init_library(),
        Code::Ok,
        "real Tcl init failed: {}",
        String::from_utf8_lossy(&interp.result_bytes())
    );

    let script = format!(
        "package require tcltest\n\
         namespace import -force ::tcltest::*\n\
         set path(test1) {}\n\
         set path(test2) {}\n\
         {io_39_14}\n\
         {io_39_15}\n\
         test io-75.9-runtime {{strict output conversion keeps its valid prefix}} -setup {{\n\
             set f [open $path(test2) w]\n\
             fconfigure $f -encoding iso8859-1 -profile strict\n\
         }} -body {{\n\
             set code [catch {{puts -nonewline $f \"A\\u2022\"}} msg opts]\n\
             close $f\n\
             list $code [string match {{error writing \"*\": invalid or incomplete multibyte or wide character}} $msg] [dict get $opts -errorcode]\n\
         }} -result [list 1 1 {{POSIX EILSEQ {{invalid or incomplete multibyte or wide character}}}}]\n\
         ::tcltest::cleanupTests\n",
        tcl_syntax::list::list_element(&path1.to_string_lossy()),
        tcl_syntax::list::list_element(&path2.to_string_lossy()),
    );
    assert_eq!(
        interp.eval_str(script.as_bytes()),
        Code::Ok,
        "channel tcltest execution failed: {}",
        String::from_utf8_lossy(&interp.result_bytes())
    );
    assert_eq!(
        std::fs::read(&path2).expect("read strict output prefix"),
        b"A"
    );
    let summary = String::from_utf8(host.stdout.borrow().clone()).expect("UTF-8 tcltest output");
    assert!(
        summary.contains("Total\t3\tPassed\t3\tSkipped\t0\tFailed\t0"),
        "{summary}"
    );
    std::fs::remove_dir_all(fixture).expect("remove channel oracle fixture");
}
