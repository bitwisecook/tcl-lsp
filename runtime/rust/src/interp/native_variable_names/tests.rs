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

use super::*;

thread_local! {
    static NAME_FREE_MODE: std::cell::Cell<u8> = const { std::cell::Cell::new(0) };
    static NAME_FREE_CALLS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static NAME_FREE_BORROWS_RELEASED: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
    static NAME_FREE_CONTEXT_ACCEPTED: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
    static NAME_CALLBACK_BOUND: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static NAME_CALLBACK_TARGET_BORN: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static NAME_CALLBACK_CODE: std::cell::Cell<Code> = const { std::cell::Cell::new(Code::Other(37)) };
}

thread_local! {
    static ALIAS_SIMPLE_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

thread_local! {
    static GLOBAL_CACHE_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

struct NamespaceOutputHost {
    inner: Rc<dyn tcl_platform::Host>,
    output: std::cell::RefCell<Vec<u8>>,
}
impl tcl_platform::StdIo for NamespaceOutputHost {
    fn write_stdout(&self, bytes: &[u8]) {
        self.output.borrow_mut().extend_from_slice(bytes);
    }
    fn write_stderr(&self, bytes: &[u8]) {
        self.inner.stdio().write_stderr(bytes);
    }
}
impl tcl_platform::Host for NamespaceOutputHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        self.inner.capabilities()
    }
    fn clock(&self) -> &dyn tcl_platform::Clock {
        self.inner.clock()
    }
    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        self
    }
    fn env(&self) -> &dyn tcl_platform::Env {
        self.inner.env()
    }
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        self.inner.numeric_environment()
    }
    fn native_integer_formatter(&self) -> Option<&dyn tcl_platform::NativeIntegerFormatter> {
        self.inner.native_integer_formatter()
    }
    fn system_encoding(&self) -> tcl_platform::SystemEncoding {
        self.inner.system_encoding()
    }
    fn filesystem(&self) -> Option<&dyn tcl_platform::Filesystem> {
        self.inner.filesystem()
    }
    fn sockets(&self) -> Option<&dyn tcl_platform::Sockets> {
        self.inner.sockets()
    }
    fn process(&self) -> Option<&dyn tcl_platform::Process> {
        self.inner.process()
    }
}

fn trace_source_observation(engine: &str, source: &[u8], expected: &[u8]) {
    crate::counters::reset();
    {
        let host = Rc::new(NamespaceOutputHost {
            inner: super::super::default_host(),
            output: std::cell::RefCell::new(Vec::new()),
        });
        let mut interp = Interp::with_native_core(
            host.clone(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let code = interp.eval_str(source);
        assert_eq!(code, Code::Ok, "{engine}: {:?}", interp.result_bytes());
        assert!(!interp.host_refusal_pending(), "{engine}");
        let actual = host.output.borrow();
        // The version identifies the native capture, while this control
        // compares trace setup and callback observations after that line.
        let after_version = |bytes: &[u8]| {
            let end = bytes.iter().position(|byte| *byte == b'\n').unwrap();
            bytes[end + 1..].to_vec()
        };
        assert_eq!(after_version(&actual), after_version(expected), "{engine}");
    }
    assert_eq!(crate::counters::finalize(), 0, "{engine}");
    assert_eq!(crate::counters::double_free_count(), 0, "{engine}");
}

thread_local! {
    static CACHE_LOOKUP_CASE: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static CACHE_LOOKUP_CALLBACKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
    static CACHE_LOOKUP_ROWS: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

mod callback_windows;
mod name_caches;
mod name_free_callbacks;
mod namespace_variables;
mod source_controls;

use tcl_syntax::native_object::NativeObjectCacheSnapshot as Cache;
