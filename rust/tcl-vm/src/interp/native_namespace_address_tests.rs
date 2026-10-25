// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original namespace operands, guest errors and retained namespace ownership.

use super::{Value, Vm};
use tcl_core_types::ROOT_NS;
use tcl_runtime_api::Namespaces;

fn compare_public_rows(engine: &str, observation: &str) {
    let rows = tcl_syntax::list::split_list(observation).unwrap();
    assert_eq!(rows.len(), 18 * 5);
    let mut compared = 0;
    let mut excluded = Vec::new();
    let mut active_identity = String::new();
    let mut vm: Option<Vm> = None;
    for row in rows.as_chunks::<5>().0 {
        let id = tcl_syntax::list::split_list(&row[0]).unwrap();
        let identity = tcl_syntax::list::split_list(&id[0]).unwrap();
        let context = identity[0].as_ref();
        let variant = identity[1].as_ref();
        if variant == "nul-suffix" {
            excluded.push(row[0].to_string());
            continue;
        }
        if active_identity != id[0] {
            let profile = crate::environment::profile_for_dialect(engine);
            let mut actual = crate::native_fixture::interpreter(profile);
            let token = actual
                .activate_namespace_operand(context.as_bytes())
                .unwrap();
            actual.push_ns_eval_token_frame(token, Vec::new());
            vm = Some(actual);
            active_identity = id[0].to_string();
        }
        let vm = vm.as_mut().unwrap();
        let name: &[u8] = match variant {
            "empty" => b"",
            "absolute-root" => b"::",
            other => panic!("unrecognised oracle variant {other}"),
        };
        let mut arguments = vec![
            Value::new_native_string_bytes(if id[1] == "eval" {
                b"eval".as_slice()
            } else {
                b"exists".as_slice()
            }),
            Value::new_native_string_bytes(name),
        ];
        if id[1] == "eval" {
            arguments.push(Value::new_native_string_bytes(
                b"namespace current".as_slice(),
            ));
        } else {
            assert!(matches!(id[1].as_ref(), "exists-before" | "exists-after"));
        }
        let completion = vm.invoke_host_original_object_vector(
            &Value::new_native_string_bytes(b"namespace".as_slice()),
            &arguments,
        );
        assert!(vm.refused_completion().is_none(), "{engine}: {}", row[0]);
        assert_eq!(
            completion.code.as_int().to_string(),
            row[1],
            "{engine}: {}",
            row[0]
        );
        let protocol = vm
            .actual_native_invocation_dialect()
            .native_string_protocol()
            .unwrap();
        assert_eq!(
            completion
                .result
                .native_string_bytes(protocol)
                .unwrap()
                .as_ref(),
            row[2].as_bytes(),
            "{engine}: {}",
            row[0]
        );
        if !completion.code.is_ok() {
            let error_code =
                crate::command::opt_get_checked(vm, &completion.options, b"-errorcode")
                    .expect("actual completion options remain readable")
                    .expect("a guest error retains its error-code option");
            assert_eq!(
                error_code.native_string_bytes(protocol).unwrap().as_ref(),
                row[3].as_bytes(),
                "{engine}: {}",
                row[0]
            );
        }
        compared += 1;
    }
    assert_eq!(compared, 12);
    assert_eq!(excluded.len(), 6);
    eprintln!(
        "{engine}: {compared} original namespace command rows; six Tcl STRING Unicode-NUL rows outside this ASCII operand control: {excluded:?}; errorInfo formatting untested"
    );
}

macro_rules! public_rows {
    ($name:ident, $engine:literal, $release:literal) => {
        #[test]
        fn $name() {
            compare_public_rows(
                $engine,
                include_str!(concat!(
                    "../../../tcl-registry/tests/data/source_contract_vectors/namespace-address-purpose/",
                    $release,
                    "/stdout.bin"
                )),
            );
        }
    };
}

public_rows!(original_namespace_address_c84, "tcl8.4", "8.4.20");
public_rows!(original_namespace_address_c85, "tcl8.5", "8.5.19");
public_rows!(original_namespace_address_c86, "tcl8.6", "8.6.18");
public_rows!(original_namespace_address_c90, "tcl9.0", "9.0.4");
public_rows!(original_namespace_address_c91, "tcl9.1", "9.1.0");

#[test]
fn namespace_address_keeps_retained_context_and_live_root_distinct() {
    use tcl_cmd_core::namespace::NamespaceDeleteBackend;
    for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect(engine));
        let old = vm.activate_namespace_operand(b"::N").unwrap();
        let old_child = vm.activate_namespace_operand(b"::N::child").unwrap();
        vm.push_ns_eval_token_frame(old, Vec::new());
        vm.delete_selected_namespace(old).unwrap();
        let replacement = vm.activate_namespace_operand(b"::N").unwrap();
        let replacement_child = vm.activate_namespace_operand(b"::N::child").unwrap();
        assert_ne!(old, replacement);
        assert_ne!(old_child, replacement_child);
        assert_eq!(vm.current_ns_id(), old);
        assert_eq!(vm.find_namespace_bytes_checked(old, b"").unwrap(), None);
        assert_eq!(Namespaces::find_namespace_bytes(&vm, old, b""), None);
        assert_eq!(
            vm.find_namespace_bytes_checked(old, b"::").unwrap(),
            Some(ROOT_NS)
        );
        assert_eq!(
            vm.find_namespace_bytes_checked(old, b"child").unwrap(),
            Some(old_child)
        );
        assert_eq!(
            vm.find_namespace_bytes_checked(old, b"::N::child").unwrap(),
            Some(replacement_child)
        );
        assert!(vm.refused_completion().is_none());
    }
}

#[test]
fn namespace_address_retired_owner_is_unavailable_not_missing() {
    let vm = crate::native_fixture::core(crate::environment::profile_for_dialect("tcl9.0"));
    vm.name_world.borrow_mut().retire();
    assert!(vm.find_namespace_bytes_checked(ROOT_NS, b"").is_err());
}
