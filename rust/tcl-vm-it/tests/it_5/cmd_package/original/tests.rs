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

//! Fixed software owner controls; no external package callback chronology claim.

#[cfg(debug_assertions)]
use crate::prelude::*;
use std::cell::Cell;
#[cfg(debug_assertions)]
use tcl_platform::{
    NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
    NativeIntegerKind,
};
use tcl_runtime_api::{Code, NativeExecutionError};
use tcl_syntax::native_string::NativeStringUnavailable;
use tcl_syntax::raw_string::{NativeStringAccessError, NativeValueAccessRefusal};
use tcl_syntax::scalar_getter::NativeScalarCache;
fn native(profile: &str) -> Vm {
    crate::fixture::interpreter(tcl_vm::test_support::profile(profile))
}
struct Formatter {
    failed: bool,
    calls: Cell<usize>,
}
impl NativeIntegerFormatter for Formatter {
    fn build(&self) -> NativeIntegerFormatterBuild {
        NativeIntegerFormatterBuild {
            version: [8, 4, 20, 0],
            sha256: [7; 32],
            long_bits: 64,
        }
    }
    fn format(
        &self,
        kind: NativeIntegerKind,
        value: i64,
    ) -> Result<Vec<u8>, NativeIntegerFormatterUnavailable> {
        assert_eq!(kind, NativeIntegerKind::Long);
        assert_eq!(value, i64::MIN);
        self.calls.set(self.calls.get() + 1);
        if self.failed {
            Err(NativeIntegerFormatterUnavailable::Operation)
        } else {
            Ok(value.to_string().into_bytes())
        }
    }
}
struct FormatterHost {
    actual: Rc<dyn tcl_platform::Host>,
    formatter: Rc<Formatter>,
    acquisitions: Rc<Cell<usize>>,
}
impl tcl_platform::Host for FormatterHost {
    fn capabilities(&self) -> tcl_platform::Capabilities {
        self.actual.capabilities()
    }
    fn clock(&self) -> &dyn tcl_platform::Clock {
        self.actual.clock()
    }
    fn stdio(&self) -> &dyn tcl_platform::StdIo {
        self.actual.stdio()
    }
    fn env(&self) -> &dyn tcl_platform::Env {
        self.actual.env()
    }
    fn native_integer_formatter(&self) -> Option<&dyn NativeIntegerFormatter> {
        self.acquisitions.set(self.acquisitions.get() + 1);
        Some(&*self.formatter)
    }
}
#[test]
fn original_package_version_string_uses_actual_formatter_and_full_first_failure() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Supplied software formatter forwarding and first cause only; no C84 minimum-integer formatter equivalence.
    for failed in [false, true] {
        let mut vm = native("tcl8.4");
        let formatter = Rc::new(Formatter {
            failed,
            calls: Cell::new(0),
        });
        let acquisitions = Rc::new(Cell::new(0));
        vm.set_host(Rc::new(FormatterHost {
            actual: Rc::new(tcl_vm::host_native::NativeHost::new()),
            formatter: Rc::clone(&formatter),
            acquisitions: Rc::clone(&acquisitions),
        }));
        let first = Value::from_native_scalar_cache(
            NativeScalarCache::Tcl84Long(i64::MIN),
            None,
            tcl_registry::InvocationDialect::of_static_profile(vm.dialect_profile()),
        )
        .unwrap();
        let later = Value::from_native_scalar_cache(
            NativeScalarCache::Tcl84Long(2),
            None,
            tcl_registry::InvocationDialect::of_static_profile(vm.dialect_profile()),
        )
        .unwrap();
        let operands = [Value::string("vcompare"), first, later];
        let reached = crate::fixture::try_package(&mut vm, &operands);
        assert_eq!(formatter.calls.get(), 1);
        assert_eq!(acquisitions.get(), 1);
        if failed {
            assert_eq!(
                reached.unwrap_err(),
                NativeExecutionError::ValueAccessRefusal(NativeValueAccessRefusal::StringAccess(
                    NativeStringAccessError::Unavailable(NativeStringUnavailable::StringUpdater)
                ))
            );
        } else {
            let completion = reached.unwrap();
            assert_eq!(completion.code, Code::Error);
            assert_eq!(
                completion.result.string_bytes().as_ref(),
                b"expected version number but got \"-9223372036854775808\""
            );
        }
    }
}
#[test]
fn original_embedding_package_provide_uses_actual_database_protocol() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // Selected software package protocol only; the suffix input is a fixed source control, not an additional native observation.
    for (actual, logical, expected) in [
        ("tcl8.6", tcl_dialect::TclVersion::V9_1, Code::Error),
        ("tcl9.0", tcl_dialect::TclVersion::V8_4, Code::Ok),
    ] {
        let mut vm = native(actual);
        vm.set_runtime_version(logical);
        let operands = [
            Value::string("original-embedding"),
            Value::from_native_string_bytes(b"1+\xff".as_slice()),
        ];
        let mut words = vec![Value::string("provide")];
        words.extend(operands);
        let completion = crate::fixture::package(&mut vm, &words);
        assert_eq!(completion.code, expected);
        let provided = crate::fixture::package(
            &mut vm,
            &[
                Value::string("provide"),
                Value::string("original-embedding"),
            ],
        );
        assert_eq!(
            !provided.result.string_bytes().is_empty(),
            expected == Code::Ok
        );
    }
}
