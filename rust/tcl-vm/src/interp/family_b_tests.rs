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
use crate::command::NativeCommand;
use std::cell::Cell;
use tcl_registry::special_vars::BootstrapDialect;
use tcl_runtime_api::GLOBAL_FRAME;

const GUARDED_IDENTITY: GuardIdentity = GuardIdentity::new(1, 41);

fn release_profile(name: &str) -> &'static tcl_dialect::DialectProfile {
    tcl_registry::model::ingress::resolve_environment(name).unit_profile()
}

/// The release's own engine, with no compiler.
fn release_vm(version: tcl_dialect::TclVersion) -> Vm {
    crate::native_fixture::core(release_profile(version.dialect_profile_name()))
}

/// Re-select both the actual engine and the logical profile of `name`.
fn select_release(vm: &mut Vm, name: &str) {
    let profile = release_profile(name);
    assert!(vm.set_native_engine_profile(profile), "{name}");
    vm.set_dialect_profile(profile);
}

/// Invoke `words` as a host does; anything but `TCL_OK` fails the test.
fn run(vm: &mut Vm, words: &[&str]) {
    let (name, arguments) = words.split_first().unwrap();
    let arguments: Vec<_> = arguments.iter().map(|word| Value::string(*word)).collect();
    let completion = vm.try_invoke_command(name, &arguments).unwrap();
    assert_eq!(
        completion.code,
        Code::Ok,
        "{words:?}: {:?}",
        completion.result
    );
}

/// Every capture of an empty literal world renews its observation, so
/// entries are compared without it.
fn without_literal_observation(
    mut entry: tcl_runtime_api::NativeCompilationEntry,
) -> tcl_runtime_api::NativeCompilationEntry {
    entry.empty_literal_world = None;
    entry
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn construction_without_a_native_engine_refuses() {
    let source = tcl_dialect::DialectProfile::irules();
    assert!(
        tcl_native_model::InvocationDialect::of_profile(source)
            .native_bootstrap_protocol()
            .is_none()
    );
    assert!(
        Vm::with_native_core(
            Box::new(std::io::sink()),
            std::rc::Rc::new(crate::host_native::NativeHost::new()),
            source,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .is_none()
    );
}

#[test]
#[cfg(not(target_arch = "wasm32"))]
fn default_c_error_code_stays_private_until_return_options_are_requested() {
    for engine in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
        let mut vm = Vm::with_native_core(
            Box::new(Vec::<u8>::new()),
            std::rc::Rc::new(crate::host_native::NativeHost::new()),
            profile,
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        let result = crate::return_options::apply(
            &mut vm,
            &[
                Value::string("-code"),
                Value::string("error"),
                Value::string("-level"),
                Value::int(0),
                Value::string("BODY"),
            ],
            tcl_cmd_core::return_options::ReturnOptionsPurpose::User,
        );
        assert_eq!(result.code, Code::Error, "{engine}");
        assert!(crate::command::opt_get(&result.options, "-errorcode").is_none());
        assert_eq!(
            vm.native_errors
                .primitive_error_code
                .as_ref()
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"NONE",
            "{engine}"
        );
        let options = vm.completion_options_snapshot(&result).unwrap();
        assert_eq!(
            crate::command::opt_get(&options, "-errorcode")
                .unwrap()
                .string_bytes()
                .as_ref(),
            b"NONE",
            "{engine}"
        );
    }
}

struct TestNative;

impl NativeCommand for TestNative {
    fn invoke(&self, _vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
        ok(Value::empty())
    }
}

fn guarded_builtin(_vm: &mut Vm, _args: &[Value]) -> Completion<Value> {
    ok(Value::empty())
}

thread_local! {
    static PROBED_TOKEN: std::cell::Cell<Option<GuardToken>> =
        const { std::cell::Cell::new(None) };
}

mod command_name_bytes;
mod compile_admission;
mod hidden_commands;
mod host_activation;
mod intrinsic_guards;
mod native_entry_identity;
mod variables;

#[test]
fn runtime_version_threads_into_the_release_globals() {
    // §5.4 VM parity: the default is the 9.0.4 reference release the
    // VM's semantics are re-derived from…
    let vm = Vm::new();
    assert_eq!(vm.runtime_version(), tcl_dialect::TclVersion::V9_0);
    assert_eq!(
        vm.get_var("tcl_version").map(|v| v.to_str().to_string()),
        Some("9.0".to_owned())
    );
    assert_eq!(
        vm.get_var("tcl_patchLevel").map(|v| v.to_str().to_string()),
        Some("9.0.4".to_owned())
    );
    // …while a profile selects its own emulated core release. iRules is
    // an embedded Tcl 8.4 surface, so routing it through the profile must
    // also update the release globals rather than retaining the default.
    let mut vm = Vm::new();
    vm.set_runtime_version(tcl_dialect::DialectProfile::irules().vm_runtime_version);
    assert_eq!(vm.runtime_version(), tcl_dialect::TclVersion::V8_4);
    assert_eq!(
        vm.get_var("tcl_version").map(|v| v.to_str().to_string()),
        Some("8.4".to_owned())
    );
    assert_eq!(
        vm.get_var("tcl_patchLevel").map(|v| v.to_str().to_string()),
        Some("8.4.20".to_owned())
    );
}

#[test]
fn introspect_level_and_argv() {
    let mut vm = Vm::new();
    // Top level: depth 0, the global frame has no invoking call.
    assert_eq!(Introspect::level(&vm), 0);
    assert!(Introspect::level_argv(&vm, 0).unwrap().is_none());
    // A proc-call frame with its invoking words.
    vm.push_call_frame(
        Some("p".to_string()),
        vec![Value::string("p"), Value::string("x")],
    );
    assert_eq!(Introspect::level(&vm), 1);
    assert_eq!(
        &*Introspect::level_argv(&vm, 1).unwrap().unwrap().to_str(),
        "p x"
    );
    vm.pop_call_frame();
    assert_eq!(Introspect::level(&vm), 0);
}

#[test]
fn commands_dispatch_builtin_and_unknown() {
    let mut vm = Vm::new();
    // A builtin runs inline and yields its result.
    let c = vm
        .dispatch("list", &[Value::string("a"), Value::string("b c")])
        .unwrap();
    assert_eq!(c.code, Code::Ok);
    assert_eq!(&*c.result.to_str(), "a {b c}");
    // An unknown command name is an error completion.
    let c = vm.dispatch("no_such_command", &[]).unwrap();
    assert_eq!(c.code, Code::Error);
    assert_eq!(
        &*c.result.to_str(),
        "invalid command name \"no_such_command\""
    );
}

#[test]
fn frames_push_pop_current_link() {
    let mut vm = Vm::new();
    let to_s = |v: Option<Value>| v.map(|v| v.to_str().to_string());
    // A global, set while the global frame is current.
    vm.set(GLOBAL_FRAME, "g", Value::string("orig"));
    let outer = Frames::current(&vm);
    assert_eq!(outer, GLOBAL_FRAME);
    // Push a proc-call frame in a fresh namespace (interned to an NsId, which
    // round-trips through the arena back to its name on push).
    let ns = vm.intern_ns("foo");
    let inner = Frames::push(&mut vm, ns);
    assert_ne!(inner, outer);
    assert_eq!(Frames::current(&vm), inner);
    // `upvar`: link `gg` (inner) to the outer frame's global `g`; reads and
    // writes through `gg` reach `g`.
    Frames::link(&mut vm, inner, outer, "gg", "g");
    assert_eq!(to_s(vm.get(inner, "gg")), Some("orig".to_string()));
    vm.set(inner, "gg", Value::string("changed"));
    // Pop back to the global frame: the link is gone, the global updated.
    Frames::pop(&mut vm);
    assert_eq!(Frames::current(&vm), outer);
    assert_eq!(to_s(vm.get(GLOBAL_FRAME, "g")), Some("changed".to_string()));
}

#[test]
fn namespaces_current_and_find_command() {
    let mut vm = Vm::new();
    // At the top level the current namespace is the global root.
    assert_eq!(Namespaces::current(&vm), ROOT_NS);
    // Builtins resolve from the global namespace to stable, distinct ids.
    let a = Namespaces::find_command(&vm, ROOT_NS, "list").expect("list resolves");
    assert_eq!(a, Namespaces::find_command(&vm, ROOT_NS, "list").unwrap());
    assert_ne!(
        a,
        Namespaces::find_command(&vm, ROOT_NS, "llength").unwrap()
    );
    assert!(Namespaces::find_command(&vm, ROOT_NS, "no_such_command").is_none());
    // `current` tracks the namespace pushed by `Frames::push`.
    let foo = vm.intern_ns("foo");
    Frames::push(&mut vm, foo);
    assert_eq!(Namespaces::current(&vm), foo);
    Frames::pop(&mut vm);
    assert_eq!(Namespaces::current(&vm), ROOT_NS);
}

#[test]
fn commands_dispatch_id_composes() {
    let mut vm = Vm::new();
    // Resolve a command to a handle, then invoke it *by that handle* — the
    // find_command -> dispatch_id composition.
    let id = Namespaces::find_command(&vm, ROOT_NS, "list").expect("list resolves");
    let c = vm
        .dispatch_id(id, &[Value::string("a"), Value::string("b")])
        .unwrap();
    assert_eq!(c.code, Code::Ok);
    assert_eq!(&*c.result.to_str(), "a b");
    // A fabricated id yields an error completion (no such command).
    let c = vm.dispatch_id(CommandId(9999), &[]).unwrap();
    assert_eq!(c.code, Code::Error);
    assert_eq!(&*c.result.to_str(), "invalid command id");
}

#[test]
fn smoke_eval_canonical_snippet() {
    let mut vm = Vm::new();
    let result = vm
        .eval_expr("21 * 2")
        .expect("canonical expr must evaluate");
    assert_eq!(result.to_str().as_ref(), "42");
}
