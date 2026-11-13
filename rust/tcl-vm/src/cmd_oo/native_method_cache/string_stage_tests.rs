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

//! Actual VM cache String stages; physical C Method roles remain separate.
use super::*;
use crate::interp::native_numeric_ops::OriginalNumericOps;
use std::cell::Cell;
use tcl_platform::{
    NativeIntegerFormatter, NativeIntegerFormatterBuild, NativeIntegerFormatterUnavailable,
    NativeIntegerKind,
};
use tcl_syntax::{number::Number, scalar_getter::NativeScalarCache};

struct Formatter {
    version: [i32; 4],
    mode: u8,
    queries: Cell<usize>,
    calls: Cell<usize>,
    selected: Weak<RefCell<Method>>,
    original: RefCell<Option<Value>>,
}
impl NativeIntegerFormatter for Formatter {
    fn build(&self) -> NativeIntegerFormatterBuild {
        NativeIntegerFormatterBuild {
            version: self.version,
            sha256: [7; 32],
            long_bits: 64,
        }
    }
    fn format(
        &self,
        _: NativeIntegerKind,
        value: i64,
    ) -> Result<Vec<u8>, NativeIntegerFormatterUnavailable> {
        self.calls.set(self.calls.get() + 1);
        if self.mode == 2 {
            let retired = self.original.borrow_mut().take();
            drop(retired);
        }
        if self.mode == 1 {
            return Err(NativeIntegerFormatterUnavailable::Operation);
        }
        Ok(value.to_string().into_bytes())
    }
}
struct Host {
    actual: Rc<dyn tcl_platform::Host>,
    formatter: Rc<Formatter>,
}
impl tcl_platform::Host for Host {
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
    fn filesystem(&self) -> Option<&dyn tcl_platform::Filesystem> {
        self.actual.filesystem()
    }
    fn numeric_environment(&self) -> Option<&dyn tcl_platform::NumericEnvironment> {
        self.actual.numeric_environment()
    }
    fn native_integer_formatter(&self) -> Option<&dyn NativeIntegerFormatter> {
        self.formatter.queries.set(self.formatter.queries.get() + 1);
        Some(self.formatter.as_ref())
    }
}
fn provided(version: &str, mode: u8) -> (Vm, OoId, Rc<Formatter>) {
    let mut vm =
        crate::native_fixture::interpreter(crate::environment::profile_for_dialect(version));
    let setup = vm
        .eval_source("oo::class create C {method 42 {} {return ACTUAL}}; C create obj")
        .unwrap();
    assert_eq!(setup.code, Code::Ok);
    let target = super::super::resolve_object_value(&mut vm, &Value::string("obj"))
        .unwrap()
        .unwrap();
    let class = vm.oo.objects[&target].class;
    let owner = vm.oo.classes[&class].methods.owner("42").unwrap();
    let selected = Rc::downgrade(&owner);
    drop(owner);
    let formatter = Rc::new(Formatter {
        version: match version {
            "tcl8.6" => [8, 6, 18, 0],
            "tcl9.0" => [9, 0, 4, 0],
            "tcl9.1" => [9, 1, 0, 0],
            _ => unreachable!(),
        },
        mode,
        queries: Cell::new(0),
        calls: Cell::new(0),
        selected,
        original: RefCell::new(None),
    });
    vm.set_host(Rc::new(Host {
        actual: vm.host_rc(),
        formatter: Rc::clone(&formatter),
    }));
    (vm, target, formatter)
}
fn numeric(vm: &Vm) -> Value {
    Value::from_native_scalar_cache(
        NativeScalarCache::Number(Number::Int(42)),
        None,
        vm.actual_native_invocation_dialect(),
    )
    .unwrap()
}
fn selected(vm: &Vm, target: OoId) -> Rc<Vec<Step>> {
    let steps = vm
        .oo
        .object_precedence(target)
        .into_iter()
        .filter(|step| vm.oo.step_method(step, "42").is_some())
        .collect();
    Rc::new(retain_owners(
        &vm.oo,
        steps,
        "42",
        NativeTclooMethodReportRole::Named,
    ))
}
#[test]
fn original_method_cache_string_formats_an_integer_key_without_the_host() {
    // naming.tcloo.original-method-cache-and-active-call-lifetime
    // docs/design/analysis/name-resolution-proofs/tcloo-original-method-cache-and-active-call-lifetime.md
    // C8.6, 9.0 and 9.1 spell an integer with TclFormatInt (tclObj.c
    // UpdateStringOfInt): the key's String stage queries the host but never calls it.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        let (mut vm, target, formatter) = provided(version, 0);
        let original = numeric(&vm);
        let references = original.native_object_reference_count();
        let steps = selected(&vm, target);
        assert_eq!(formatter.selected.strong_count(), 2);
        let chain = remember(&mut vm, target, "42", &original, true, steps).unwrap();
        assert_eq!(formatter.queries.get(), 1);
        assert_eq!(formatter.calls.get(), 0);
        assert_eq!(formatter.selected.strong_count(), 2);
        assert_eq!(original.resident_string_bytes().unwrap().as_ref(), b"42");
        assert!(original.native_method_name_chain().is_some());
        assert_eq!(original.native_object_reference_count(), references + 1);
        drop(chain);
        assert_eq!(formatter.selected.strong_count(), 1);
        let scope = NativeOperationScope::enter(&mut vm).unwrap();
        let protocol = vm.actual_native_string_protocol().unwrap();
        assert_eq!(
            OriginalNumericOps::in_original_scope(&mut vm, &scope)
                .unwrap()
                .string(&original, protocol)
                .unwrap()
                .as_ref(),
            b"42"
        );
        assert_eq!(formatter.queries.get(), 1);
        assert_eq!(formatter.calls.get(), 0);
        assert!(vm.execution_refusal.is_none());
    }
}
#[test]
fn original_method_cache_string_never_reaches_a_failing_or_retiring_formatter() {
    // naming.embedding.original-host-publication-and-fact-transport
    // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
    // C8.6, 9.0 and 9.1 spell an integer with TclFormatInt (tclObj.c
    // UpdateStringOfInt): a supplied formatter that would fail or drop the last
    // header is never called, so the cache is adopted.
    for version in ["tcl8.6", "tcl9.0", "tcl9.1"] {
        for mode in [1, 2] {
            let (mut vm, target, formatter) = provided(version, mode);
            let scope = NativeOperationScope::enter(&mut vm).unwrap();
            let original = numeric(&vm);
            let held = original.native_lifetime_lease();
            *formatter.original.borrow_mut() = Some(original);
            let steps = selected(&vm, target);
            let chain = remember(&mut vm, target, "42", held.value(), true, steps).unwrap();
            assert_eq!(formatter.queries.get(), 1);
            assert_eq!(formatter.calls.get(), 0);
            assert!(held.value().native_object_is_live());
            assert_eq!(
                held.value().resident_string_bytes().unwrap().as_ref(),
                b"42"
            );
            assert!(held.value().native_method_name_chain().is_some());
            drop(chain);
            assert!(vm.execution_refusal.is_none());
            scope.check(&mut vm).unwrap();
        }
    }
}
