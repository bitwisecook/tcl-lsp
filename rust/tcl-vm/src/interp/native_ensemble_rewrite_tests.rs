// SPDX-License-Identifier: AGPL-3.0-or-later
//! Native reset observations exercised through ordinary and internal VM entries.

use super::*;
use tcl_registry::native_ensemble_rewrite::EnsembleRewriteResetEvent;

fn decode_hex(text: &str) -> Vec<u8> {
    text.as_bytes()
        .chunks_exact(2)
        .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
        .collect()
}

fn selected_vm(engine: &str) -> Vm {
    let profile = tcl_registry::model::ingress::resolve_environment(engine).unit_profile();
    crate::native_fixture::interpreter(profile)
}

fn usage() -> Vec<crate::command::NativeArgumentUsageRewrite> {
    vec![crate::command::NativeArgumentUsageRewrite {
        original_prefix: vec![Value::string("public"), Value::string("member")],
        removed_words: 1,
    }]
}

#[test]
fn array_default_usage_matches_sixteen_native_original_headers() {
    let mut count = 0;
    for row in
        include_str!("../../../../runtime/rust/tests/data/native_array_default/usage.tsv").lines()
    {
        let fields: Vec<_> = row.split('\t').collect();
        let engine = match fields[0] {
            "9.0.4" => "tcl9.0",
            "9.1.0" => "tcl9.1",
            _ => unreachable!(),
        };
        let mut vm = selected_vm(engine);
        let completion = vm.try_eval_source_bytes(&decode_hex(fields[2])).unwrap();
        assert_eq!(completion.code, Code::Ok, "{row}");
        assert_eq!(
            vm.native_name_operand_bytes(&completion.result)
                .unwrap()
                .as_ref(),
            decode_hex(fields[3]),
            "{row}"
        );
        count += 1;
    }
    assert_eq!(count, 16);
}

struct ReenterOrdinary;
impl crate::command::NativeCommand for ReenterOrdinary {
    fn invoke(&self, vm: &mut Vm, args: &[Value]) -> Completion<Value> {
        assert!(args.is_empty());
        let head = vm.invoked_name_value().unwrap();
        assert!(!vm.native_invocation.usage_rewrites.is_empty());
        let revision = vm.native_invocation.usage_revision;
        let completion = vm
            .try_invoke_command("set", &[Value::string("scratch"), Value::string("yes")])
            .unwrap();
        assert_eq!(completion.code, Code::Ok);
        assert_eq!(
            vm.invoked_name_value().unwrap().native_object_identity(),
            head.native_object_identity()
        );
        assert!(vm.native_invocation.usage_rewrites.is_empty());
        assert_ne!(vm.native_invocation.usage_revision, revision);
        ok(Value::empty())
    }
}

#[test]
fn callback_restores_handler_metadata_without_restoring_withdrawn_rewrite() {
    for engine in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = selected_vm(engine);
        vm.register_native_command("private", Rc::new(ReenterOrdinary));
        let head = Value::string("private");
        let rewrite = usage();
        vm.native_invocation.usage_rewrites = rewrite.clone();
        let completion = vm.invoke_command_value_internal_at(
            ROOT_NS,
            &head,
            &[],
            &rewrite,
            tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
        );
        assert_eq!(completion.code, Code::Ok, "{engine}");
        assert!(vm.native_invocation.usage_rewrites.is_empty(), "{engine}");
    }
}

#[test]
fn internal_invoke_preserves_rewrite_but_native_bytecode_entry_withdraws_it() {
    for engine in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = selected_vm(engine);
        let rewrite = usage();
        vm.native_invocation.usage_rewrites = rewrite.clone();
        let completion = vm.invoke_command_value_internal_at(
            ROOT_NS,
            &Value::string("set"),
            &[Value::string("scratch"), Value::string("yes")],
            &rewrite,
            tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
        );
        assert_eq!(completion.code, Code::Ok, "{engine}");
        assert_eq!(vm.native_invocation.usage_rewrites.len(), 1, "{engine}");
        let completion = vm.try_eval_source_bytes(b"set scratch changed").unwrap();
        assert_eq!(completion.code, Code::Ok, "{engine}");
        let protocol = vm
            .actual_native_invocation_dialect()
            .native_ensemble_rewrite_protocol()
            .unwrap();
        assert_eq!(
            vm.native_invocation.usage_rewrites.is_empty(),
            protocol.resets_at(EnsembleRewriteResetEvent::BytecodeEntry),
            "{engine}"
        );
    }
}

#[test]
fn execution_trace_cannot_reinstall_a_captured_usage_prefix() {
    let mut vm = selected_vm("tcl9.0");
    let completion = vm
        .try_eval_source_bytes(b"proc watch args {};trace add execution set enter watch")
        .unwrap();
    assert_eq!(completion.code, Code::Ok);
    let rewrite = usage();
    vm.native_invocation.usage_rewrites = rewrite.clone();
    let completion = vm.invoke_command_value_internal_at(
        ROOT_NS,
        &Value::string("set"),
        &[],
        &rewrite,
        tcl_registry::command_lookup::CommandLookupOrigin::Ordinary,
    );
    assert_eq!(completion.code, Code::Error);
    assert_eq!(
        vm.native_name_operand_bytes(&completion.result)
            .unwrap()
            .as_ref(),
        b"wrong # args: should be \"set varName ?newValue?\""
    );
    assert!(vm.native_invocation.usage_rewrites.is_empty());
}
