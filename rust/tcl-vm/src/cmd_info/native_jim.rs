// SPDX-License-Identifier: AGPL-3.0-or-later
//! Jim parser reports retain original words and invoke the actual current lsort.

use crate::interp::{Vm, ok};
use crate::value::Value;
use tcl_registry::commands::tcl::{NativeJimInfoProtocol, NativeJimInfoReport};
use tcl_runtime_api::Completion;
use tcl_syntax::value::ValueOps;

pub(super) fn command_inventory(
    vm: &mut Vm,
    scope: tcl_registry::commands::tcl::NativeJimInfoScope,
    original: &[Value],
    kind: tcl_registry::commands::tcl::NativeJimCommandInventoryKind,
) -> Completion<Value> {
    let original_namespace = match vm.with_jim_current_namespace(|original| Ok(original.clone())) {
        Ok(namespace) => namespace,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let namespace = match vm.native_string_bytes(&original_namespace) {
        Ok(namespace) => namespace,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let first_operand = match original
        .get(1)
        .map(|value| vm.native_name_operand_bytes(value))
        .transpose()
    {
        Ok(first) => first,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    match scope.command_inventory(original.len() - 1, first_operand.as_deref(), &namespace) {
        tcl_registry::commands::tcl::NativeJimCommandInventory::NamespaceHelper => vm
            .invoke_host_original_object_vector(
                &Value::new_native_string_bytes(b"namespace info".as_slice()),
                original,
            ),
        tcl_registry::commands::tcl::NativeJimCommandInventory::Flat {
            pattern,
            include_spaces,
        } => {
            match tcl_cmd_core::info::jim_core_command_list(
                vm,
                pattern.map(|index| &original[index + 1]),
                include_spaces,
                kind,
            ) {
                Ok(value) => ok(value),
                Err(error) => crate::command::completion_from_cmd_error(vm, error),
            }
        }
        tcl_registry::commands::tcl::NativeJimCommandInventory::WrongArguments => {
            let selector = match vm.native_name_operand_bytes(&original[0]) {
                Ok(selector) => selector,
                Err(error) => return vm.refuse_host_command(error.to_string()),
            };
            crate::command::completion_from_cmd_error(
                vm,
                tcl_cmd_core::CmdError::new_bytes(
                    tcl_registry::commands::tcl::NativeJimCommandInventory::wrong_arguments_message(
                        &selector,
                    ),
                )
                .with_native_string_result(tcl_syntax::native_string::NativeStringProtocol::Jim084),
            )
        }
    }
}

pub(super) fn report(
    vm: &mut Vm,
    protocol: NativeJimInfoProtocol,
    report: NativeJimInfoReport,
    original: &[Value],
) -> Completion<Value> {
    // Source proof: naming.info.jim-original-selector-table
    // docs/design/analysis/name-resolution-proofs/info-jim-original-selector-table.md
    let choices = match choices(vm, protocol, report) {
        Ok(bytes) => bytes,
        Err(completion) => return completion,
    };
    let head = match vm.native_name_operand_bytes(&original[report.head()]) {
        Ok(bytes) => bytes,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let selector = match report
        .selector()
        .map(|index| vm.native_name_operand_bytes(&original[index]))
        .transpose()
    {
        Ok(bytes) => bytes,
        Err(error) => return vm.refuse_host_command(error.to_string()),
    };
    let bytes = report.message(&head, selector.as_deref().unwrap_or_default(), &choices);
    if report.succeeds() {
        ok(Value::new_native_string_bytes(bytes))
    } else {
        crate::command::completion_from_cmd_error(
            vm,
            tcl_cmd_core::CmdError::new_bytes(bytes)
                .with_native_string_result(tcl_syntax::native_string::NativeStringProtocol::Jim084),
        )
    }
}

fn choices(
    vm: &mut Vm,
    protocol: NativeJimInfoProtocol,
    report: NativeJimInfoReport,
) -> Result<Vec<u8>, Completion<Value>> {
    let Some(separator) = report.choices_separator() else {
        return Ok(Vec::new());
    };
    let names = Value::native_list_constructor(
        protocol
            .names()
            .iter()
            .map(|name| Value::new_native_string_bytes(name.as_bytes()))
            .collect(),
        tcl_syntax::native_string::NativeStringProtocol::Jim084,
    );
    let completion = vm.invoke_host_original_object_vector(
        &Value::new_native_string_bytes(b"lsort".as_slice()),
        &[names],
    );
    if let Some(refusal) = vm.refused_completion() {
        return Err(refusal);
    }
    if completion.code != crate::Code::Ok {
        return vm
            .native_name_operand_bytes(&completion.result)
            .map(|bytes| bytes.to_vec())
            .map_err(|error| vm.refuse_host_command(error.to_string()));
    }
    let elements = vm
        .list_elements(&completion.result)
        .map_err(|error| vm.refuse_host_command(error.to_string()))?;
    let mut bytes = Vec::new();
    for (index, element) in elements.iter().enumerate() {
        if index > 0 {
            bytes.extend_from_slice(separator);
        }
        bytes.extend_from_slice(
            &vm.native_name_operand_bytes(element)
                .map_err(|error| vm.refuse_host_command(error.to_string()))?,
        );
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXCESS: &[&[u8]] = &[b"__r2286_operand__" as &[u8]; 10];
    const CASES: &[(&str, &[u8], &[&[u8]])] = &[
        ("inventory-miss", b"__r2286_absent__", &[]),
        ("inventory-jim-list", b"-commands", &[]),
        (
            "exists-missing",
            b"exists",
            &[b"__r2286_inventory_missing__"],
        ),
        ("exact-alias", b"alias", EXCESS),
        ("exact-aliases", b"aliases", EXCESS),
        ("exact-args", b"args", EXCESS),
        ("exact-body", b"body", EXCESS),
        ("exact-channels", b"channels", EXCESS),
        ("exact-class", b"class", EXCESS),
        ("exact-cmdcount", b"cmdcount", EXCESS),
        ("exact-cmdtype", b"cmdtype", EXCESS),
        ("exact-commands", b"commands", EXCESS),
        ("exact-complete", b"complete", EXCESS),
        ("exact-constant", b"constant", EXCESS),
        ("exact-consts", b"consts", EXCESS),
        ("exact-coroutine", b"coroutine", EXCESS),
        ("exact-default", b"default", EXCESS),
        ("exact-errorstack", b"errorstack", EXCESS),
        ("exact-exists", b"exists", EXCESS),
        ("exact-frame", b"frame", EXCESS),
        ("exact-functions", b"functions", EXCESS),
        ("exact-globals", b"globals", EXCESS),
        ("exact-help", b"help", EXCESS),
        ("exact-hostname", b"hostname", EXCESS),
        ("exact-level", b"level", EXCESS),
        ("exact-library", b"library", EXCESS),
        ("exact-loaded", b"loaded", EXCESS),
        ("exact-locals", b"locals", EXCESS),
        ("exact-nameofexecutable", b"nameofexecutable", EXCESS),
        ("exact-object", b"object", EXCESS),
        ("exact-patchlevel", b"patchlevel", EXCESS),
        ("exact-procs", b"procs", EXCESS),
        ("exact-references", b"references", EXCESS),
        ("exact-returncodes", b"returncodes", EXCESS),
        ("exact-script", b"script", EXCESS),
        ("exact-sharedlibextension", b"sharedlibextension", EXCESS),
        ("exact-source", b"source", EXCESS),
        ("exact-stacktrace", b"stacktrace", EXCESS),
        ("exact-statics", b"statics", EXCESS),
        ("exact-tainted", b"tainted", EXCESS),
        ("exact-tclversion", b"tclversion", EXCESS),
        ("exact-usage", b"usage", EXCESS),
        ("exact-vars", b"vars", EXCESS),
        ("exact-version", b"version", EXCESS),
        ("prefix-co", b"co", EXCESS),
        ("prefix-cor", b"cor", EXCESS),
        ("prefix-cl", b"cl", EXCESS),
        ("prefix-cm", b"cm", EXCESS),
        ("prefix-sta", b"sta", EXCESS),
        ("prefix-vers", b"vers", EXCESS),
        ("prefix-empty", b"", EXCESS),
    ];

    fn native_row(id: &str) -> (bool, Vec<u8>) {
        let path = format!(
            "{}/../tcl-registry/tests/data/native_info_inventory_original/jim/{id}/stdout.tsv",
            env!("CARGO_MANIFEST_DIR")
        );
        let captured = std::fs::read_to_string(path).unwrap();
        let row = captured
            .lines()
            .find(|line| line.starts_with("INFO|"))
            .unwrap();
        let fields: Vec<_> = row.split('|').collect();
        let bytes = fields[3]
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        (fields[1] == "0", bytes)
    }

    #[test]
    fn original_jim_info_parser_reports_match_all_native_info_rows() {
        // Native proof: naming.info.jim-original-selector-table
        // docs/design/analysis/name-resolution-proofs/info-jim-original-selector-table.md
        for &(id, selector, rest) in CASES {
            let mut words = vec![b"info".as_slice(), selector];
            words.extend_from_slice(rest);
            let mut vm =
                crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
            let arguments: Vec<_> = words[1..]
                .iter()
                .map(|word| Value::new_native_string_bytes(*word))
                .collect();
            let actual = vm.try_invoke_command("info", &arguments).unwrap();
            let (success, bytes) = native_row(id);
            assert_eq!(
                actual.code,
                if success {
                    crate::Code::Ok
                } else {
                    crate::Code::Error
                },
                "{id}: {actual:?}"
            );
            assert_eq!(
                vm.native_name_operand_bytes(&actual.result)
                    .unwrap()
                    .as_ref(),
                bytes,
                "{id}"
            );
        }
    }

    fn sort_failure(vm: &mut Vm, _: &[Value]) -> Completion<Value> {
        crate::command::completion_from_cmd_error(vm, tcl_cmd_core::CmdError::new("SORT_ERROR"))
    }

    #[test]
    fn current_lsort_failure_is_reported_by_the_selected_jim_parser() {
        // Source proof: naming.info.jim-original-selector-table
        // docs/design/analysis/name-resolution-proofs/info-jim-original-selector-table.md
        let mut vm = crate::native_fixture::core(crate::environment::profile_for_dialect("jim"));
        vm.register("lsort", sort_failure);
        let result = vm
            .try_invoke_command("info", &[Value::string("__absent__")])
            .unwrap();
        assert_eq!(result.code, crate::Code::Error);
        assert_eq!(
            vm.native_name_operand_bytes(&result.result)
                .unwrap()
                .as_ref(),
            b"info, unknown command \"__absent__\": should be SORT_ERROR"
        );
        let result = vm
            .try_invoke_command("info", &[Value::string("-commands")])
            .unwrap();
        assert_eq!(result.code, crate::Code::Ok);
        assert_eq!(
            vm.native_name_operand_bytes(&result.result)
                .unwrap()
                .as_ref(),
            b"SORT_ERROR"
        );
    }
}
