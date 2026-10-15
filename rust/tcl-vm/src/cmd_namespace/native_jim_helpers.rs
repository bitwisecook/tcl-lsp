// SPDX-License-Identifier: AGPL-3.0-or-later
//! Jim helper fallback performs ordinary lookup of the original selector's head.

use super::*;
use crate::interp::native_name_consumers::OriginalNameConsumerScope;
use tcl_cmd_core::CmdError;
use tcl_syntax::value::ValueError;

pub(super) fn dispatch(
    vm: &mut Vm,
    selector: &Value,
    arguments: &[Value],
    scope: &OriginalNameConsumerScope,
) -> Completion<Value> {
    let reached = (|| -> Result<_, CmdError> {
        scope.current_for_value(vm, selector)?;
        let context = vm.native_jim_object_context()?;
        scope.current_for_value(vm, selector)?;
        let original = scope.read(vm, selector)?;
        let head = vm
            .actual_native_invocation_dialect()
            .native_namespace_scripted_helper(&original)
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim original namespace helper head",
            ))?;
        let head = Value::new_native_string_bytes(head);
        head.bind_native_jim_context(&context)?;
        scope.current_for_value(vm, selector)?;
        let completion = vm.invoke_host_original_object_vector(&head, arguments);
        scope.current_for_value(vm, selector)?;
        Ok(completion)
    })();
    match reached {
        Ok(completion) => completion,
        Err(error) => crate::command::completion_from_cmd_error(vm, error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_registry::native_scripted_distribution::NativeScriptedLibrary;

    const OBSERVED: &str = include_str!(
        "../../tests/data/native_jim_helper_literal_head/prefix-selection-20261015.tsv"
    );

    fn row(kind: &str, phase: &str) -> Vec<&'static str> {
        OBSERVED
            .lines()
            .find_map(|line| {
                let fields: Vec<_> = line.split('|').collect();
                (fields[0] == kind && fields[1] == phase).then(|| fields[2..].to_vec())
            })
            .unwrap_or_else(|| panic!("missing original helper observation {kind}/{phase}"))
    }

    fn header(kind: &str, phase: &str, original: &Value) {
        assert_eq!(
            [
                u8::from(original.resident_string_bytes().is_some()).to_string(),
                original.native_object_type_name().to_owned()
            ]
            .as_slice(),
            row(kind, phase).as_slice(),
            "{kind}/{phase}"
        );
    }

    #[test]
    fn namespace_helpers_require_the_selected_source_extension_and_preserve_overlap() {
        for reverse in [false, true] {
            let mut vm = crate::native_fixture::interpreter(
                crate::environment::profile_for_dialect("jimtcl"),
            );
            let dialect = vm.actual_native_invocation_dialect();
            for kind in tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::ALL {
                let helper = dialect.native_jim_namespace_helper(kind).unwrap();
                assert!(
                    vm.lookup_command(helper.name()).is_none(),
                    "{}",
                    helper.name()
                );
            }
            let libraries = if reverse {
                [
                    NativeScriptedLibrary::NamespaceInfo,
                    NativeScriptedLibrary::NamespaceHelpers,
                ]
            } else {
                [
                    NativeScriptedLibrary::NamespaceHelpers,
                    NativeScriptedLibrary::NamespaceInfo,
                ]
            };
            for library in libraries {
                vm.install_scripted_library(library);
            }
            let before = vm
                .registered_command_token_bytes(b"namespace info")
                .unwrap();
            for kind in tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::ALL {
                let helper = dialect.native_jim_namespace_helper(kind).unwrap();
                let Some(crate::command::Command::Proc(procedure)) =
                    vm.lookup_command(helper.name())
                else {
                    panic!("real original helper {}", helper.name());
                };
                assert_eq!(
                    procedure.body_src.resident_string_bytes().unwrap().as_ref(),
                    helper.body().as_bytes()
                );
            }
            for library in libraries {
                vm.install_scripted_library(library);
            }
            assert_eq!(
                vm.registered_command_token_bytes(b"namespace info")
                    .unwrap(),
                before
            );
            let completion = vm
                .eval_source(
                    "rename {namespace info} {}; proc {namespace which} {args} {return USER}",
                )
                .unwrap();
            assert_eq!(completion.code, Code::Ok);
            let replacement = vm
                .registered_command_token_bytes(b"namespace which")
                .unwrap();
            vm.set_dialect_profile(crate::environment::profile_for_dialect("jimtcl"));
            for library in libraries {
                vm.install_scripted_library(library);
            }
            assert!(vm.lookup_command("namespace info").is_none());
            assert_eq!(
                vm.registered_command_token_bytes(b"namespace which")
                    .unwrap(),
                replacement
            );
        }
    }

    #[test]
    fn original_namespace_helper_lookup_preserves_selector_and_argument_identity() {
        assert_eq!(OBSERVED.lines().count(), 200);
        for (label, selector, count, change) in [
            ("EXACT", b"upvar".as_slice(), 1, 0),
            ("ABBREVIATED", b"upv", 1, 0),
            ("EXACT_ARITY", b"upvar", 0, 0),
            ("ABBREVIATED_ARITY", b"upv", 0, 0),
            ("EXACT_NUL", b"upvar\0TAIL", 1, 0),
            ("ABBREVIATED_NUL", b"upv\0TAIL", 1, 0),
            ("ABBREVIATED_CANONICAL_REPLACED", b"upv", 1, 1),
            ("ABBREVIATED_SHORT_REPLACED", b"upv", 1, 2),
            ("EXACT_SHORT_REPLACED", b"upvar", 1, 2),
            ("EXACT_BOTH_REPLACED", b"upvar", 1, 3),
            ("ABBREVIATED_BOTH_REPLACED", b"upv", 1, 3),
            ("EXACT_NUL_BOTH_REPLACED", b"upvar\0TAIL", 1, 3),
        ] {
            let mut vm = crate::native_fixture::interpreter(
                crate::environment::profile_for_dialect("jimtcl"),
            );
            // The original SDK calls InitStaticExtensions, including nshelper.
            vm.install_scripted_library(NativeScriptedLibrary::NamespaceHelpers);
            if change & 1 != 0 {
                assert_eq!(
                    vm.eval_source("proc {namespace upvar} {args} {return [list FULL {*}$args]}")
                        .unwrap()
                        .code,
                    Code::Ok
                );
            }
            if change & 2 != 0 {
                assert_eq!(
                    vm.eval_source("proc {namespace upv} {args} {return [list SHORT {*}$args]}")
                        .unwrap()
                        .code,
                    Code::Ok
                );
            }
            let originals = Originals {
                caller: Value::int(42),
                head: Value::new_native_string_bytes(b"namespace".as_slice()),
                selector: Value::new_native_string_bytes(selector),
            };
            for reuse in [false, true] {
                phase(&mut vm, label, reuse, count, &originals);
            }
        }
    }

    #[test]
    fn explicit_source_selection_retires_only_its_original_bindings_on_engine_change() {
        let jim = crate::environment::profile_for_dialect("jimtcl");
        let c = crate::environment::profile_for_dialect("tcl8.6");
        let mut vm = crate::native_fixture::interpreter(jim);
        assert!(vm.set_native_engine_profile(c));
        assert!(vm.set_native_engine_profile(jim));
        assert!(vm.lookup_command("namespace which").is_none());
        vm.install_scripted_library(NativeScriptedLibrary::NamespaceHelpers);
        let original = vm
            .registered_command_token_bytes(b"namespace origin")
            .unwrap();
        assert_eq!(
            vm.eval_source("proc {namespace which} {args} {return USER}")
                .unwrap()
                .code,
            Code::Ok
        );
        let replacement = vm
            .registered_command_token_bytes(b"namespace which")
            .unwrap();
        assert!(vm.set_native_engine_profile(c));
        assert!(vm.lookup_command("namespace origin").is_none());
        assert_eq!(
            vm.registered_command_token_bytes(b"namespace which")
                .unwrap(),
            replacement
        );
        vm.install_scripted_library(NativeScriptedLibrary::NamespaceHelpers);
        assert!(vm.lookup_command("namespace origin").is_none());
        assert!(vm.set_native_engine_profile(jim));
        assert_ne!(
            vm.registered_command_token_bytes(b"namespace origin")
                .unwrap(),
            original
        );
        assert_eq!(
            vm.registered_command_token_bytes(b"namespace which")
                .unwrap(),
            replacement
        );
    }
    #[test]
    fn original_import_entry_requires_a_genuine_traversal_receipt_after_rename() {
        for (setup, invocation) in [
            ("", "{namespace import} ::missing::*"),
            ("", "namespace import ::missing::*"),
            (
                "rename {namespace import} retainedImport",
                "retainedImport ::missing::*",
            ),
        ] {
            let mut vm = crate::native_fixture::interpreter(
                crate::environment::profile_for_dialect("jimtcl"),
            );
            vm.install_scripted_library(NativeScriptedLibrary::NamespaceHelpers);
            let Some(crate::command::Command::Proc(original)) =
                vm.lookup_command("namespace import")
            else {
                panic!("installed original Import declaration");
            };
            assert!(original.requires_original_jim_import_traversal());
            let original = original.declaration();
            let header = original.body_src.native_object_type_name().to_owned();
            assert_eq!(vm.eval_source(setup).unwrap().code, Code::Ok);
            let failure = vm.eval_source(invocation).unwrap_err();
            assert!(failure.is_host(), "{invocation}");
            assert_eq!(vm.current_level(), 0, "Import never entered a frame");
            assert_eq!(original.body_src.native_object_type_name(), header);
        }
    }

    #[test]
    fn import_provenance_is_not_acquired_from_user_spelling_or_copied_source() {
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect("jimtcl"));
        vm.install_scripted_library(NativeScriptedLibrary::NamespaceHelpers);
        let helper = vm
            .actual_native_invocation_dialect()
            .native_jim_namespace_helper(
                tcl_registry::native_jim_lookup::NativeJimNamespaceHelperKind::Import,
            )
            .unwrap();
        assert!(
            vm.register_bootstrap_procedure("copiedImport", helper.parameters(), helper.body())
                .is_some()
        );
        let Some(crate::command::Command::Proc(copied)) = vm.lookup_command("copiedImport") else {
            panic!("ordinary copied procedure");
        };
        assert!(!copied.requires_original_jim_import_traversal());
        assert_eq!(
            copied.body_src.resident_string_bytes().unwrap().as_ref(),
            helper.body().as_bytes()
        );
        let completion = vm.eval_source(
            "rename {namespace import} retainedImport; proc {namespace import} {args} {return USER}; namespace import ignored"
        ).unwrap();
        assert_eq!(completion.code, Code::Ok);
        assert_eq!(
            completion.result.resident_string_bytes().unwrap().as_ref(),
            b"USER"
        );
        let Some(crate::command::Command::Proc(replacement)) =
            vm.lookup_command("namespace import")
        else {
            panic!("ordinary replacement procedure");
        };
        assert!(!replacement.requires_original_jim_import_traversal());
        let Some(crate::command::Command::Proc(retained)) = vm.lookup_command("retainedImport")
        else {
            panic!("renamed original procedure");
        };
        assert!(retained.requires_original_jim_import_traversal());
    }

    struct Originals {
        caller: Value,
        head: Value,
        selector: Value,
    }

    fn phase(vm: &mut Vm, label: &str, reuse: bool, count: usize, originals: &Originals) {
        let phase = format!("{label}_{}", if reuse { "REUSE" } else { "COLD" });
        let Originals {
            caller,
            head,
            selector,
        } = originals;
        if reuse {
            caller.invalidate_native_string_for_test();
        }
        let argument = Value::int(84);
        header("CALLER_BEFORE", &phase, caller);
        header("MEMBER_BEFORE", &phase, selector);
        header("ARGUMENT_BEFORE", &phase, &argument);
        let context = crate::interp::OriginalJimNamespaceContext::for_lambda(vm, caller).unwrap();
        vm.push_call_frame(Some("original_namespace_helper_control".into()), Vec::new());
        vm.install_jim_procedure_namespace_owner(None, caller.clone());
        vm.push_procedure_namespace_context(crate::interp::ProcedureNamespace::OriginalJim(
            context,
        ));
        let mut arguments = vec![selector.clone()];
        if count == 1 {
            arguments.push(argument.clone());
        }
        let completion = vm.invoke_host_original_object_vector(head, &arguments);
        header("CALLER_AFTER", &phase, caller);
        header("MEMBER_AFTER", &phase, selector);
        header("ARGUMENT_AFTER", &phase, &argument);
        header("RESULT_HEADER", &phase, &completion.result);
        let expected = row("RESULT", &phase);
        let bytes: Vec<_> = expected[1]
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        assert_eq!(completion.code.as_int().to_string(), expected[0], "{phase}");
        assert_eq!(
            vm.native_name_operand_bytes_checked(&completion.result)
                .unwrap()
                .as_ref(),
            bytes,
            "{phase}"
        );
        if completion.code == Code::Ok {
            let members = vm
                .native_object_list_elements_in(
                    &completion.result,
                    tcl_syntax::native_string::NativeStringProtocol::Jim084,
                )
                .unwrap();
            let members = members.elements().unwrap();
            assert_eq!(
                u8::from(members.len() == 2 && members[1].is_same_object(&argument)).to_string(),
                row("RESULT_ARGUMENT_IS_ORIGINAL", &phase)[0],
                "{phase}"
            );
        }
        assert!(vm.execution_refusal.is_none(), "{phase}");
        vm.pop_ns();
        vm.pop_call_frame();
    }
}
