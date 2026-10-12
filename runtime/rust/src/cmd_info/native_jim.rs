// SPDX-License-Identifier: AGPL-3.0-or-later
//! Jim parser reports retain original words and invoke the actual current lsort.

use crate::interp::{Code, Interp, new_string};
use crate::obj::{Owned, TclObj};
use tcl_registry::commands::tcl::{NativeJimInfoProtocol, NativeJimInfoReport};
use tcl_syntax::value::ValueOps;

pub(super) fn command_inventory(
    interp: &mut Interp,
    scope: tcl_registry::commands::tcl::NativeJimInfoScope,
    original: &[*mut TclObj],
    kind: tcl_registry::commands::tcl::NativeJimCommandInventoryKind,
) -> Code {
    let namespace = match interp.jim_current_namespace_object() {
        Ok(namespace) => namespace,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let namespace = match interp.native_string_bytes(&namespace.as_ptr()) {
        Ok(namespace) => namespace,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let first_operand = match original
        .get(1)
        .map(|value| interp.native_string_bytes(value))
        .transpose()
    {
        Ok(first) => first,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    match scope.command_inventory(original.len() - 1, first_operand.as_deref(), &namespace) {
        tcl_registry::commands::tcl::NativeJimCommandInventory::NamespaceHelper => {
            let head = Owned::fresh(new_string(b"namespace info"));
            let mut invocation = Vec::with_capacity(original.len() + 1);
            invocation.push(head.as_ptr());
            invocation.extend_from_slice(original);
            interp.dispatch(&invocation)
        }
        tcl_registry::commands::tcl::NativeJimCommandInventory::Flat {
            pattern,
            include_spaces,
        } => {
            match tcl_cmd_core::info::jim_core_command_list(
                interp,
                pattern.map(|index| &original[index + 1]),
                include_spaces,
                kind,
            ) {
                Ok(value) => {
                    interp.set_result(value);
                    Code::Ok
                }
                Err(error) => interp.report_cmd_error(error),
            }
        }
        tcl_registry::commands::tcl::NativeJimCommandInventory::WrongArguments => {
            let selector = match interp.native_string_bytes(&original[0]) {
                Ok(selector) => selector,
                Err(error) => return interp.report_cmd_error(error.into()),
            };
            interp.report_cmd_error(
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
    interp: &mut Interp,
    protocol: NativeJimInfoProtocol,
    report: NativeJimInfoReport,
    original: &[*mut TclObj],
) -> Code {
    // Source proof: naming.info.jim-original-selector-table
    // docs/design/analysis/name-resolution-proofs/info-jim-original-selector-table.md
    let choices = match choices(interp, protocol, report) {
        Ok(bytes) => bytes,
        Err(code) => return code,
    };
    let head = match interp.native_string_bytes(&original[report.head()]) {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let selector = match report
        .selector()
        .map(|index| interp.native_string_bytes(&original[index]))
        .transpose()
    {
        Ok(bytes) => bytes,
        Err(error) => return interp.report_cmd_error(error.into()),
    };
    let bytes = report.message(&head, selector.as_deref().unwrap_or_default(), &choices);
    if report.succeeds() {
        interp.set_result_bytes(&bytes);
        Code::Ok
    } else {
        interp.report_cmd_error(
            tcl_cmd_core::CmdError::new_bytes(bytes)
                .with_native_string_result(tcl_syntax::native_string::NativeStringProtocol::Jim084),
        )
    }
}

fn choices(
    interp: &mut Interp,
    protocol: NativeJimInfoProtocol,
    report: NativeJimInfoReport,
) -> Result<Vec<u8>, Code> {
    let Some(separator) = report.choices_separator() else {
        return Ok(Vec::new());
    };
    let head = Owned::fresh(new_string(b"lsort"));
    let names: Vec<_> = protocol
        .names()
        .iter()
        .map(|name| Owned::fresh(new_string(name.as_bytes())))
        .collect();
    let words: Vec<_> = names.iter().map(Owned::as_ptr).collect();
    let list = Owned::fresh(interp.new_list_object(&words));
    let code = interp.dispatch(&[head.as_ptr(), list.as_ptr()]);
    if interp.host_refusal_pending() {
        return Err(code);
    }
    let result = Owned::retain(interp.get_obj_result());
    if code != Code::Ok {
        return interp
            .native_string_bytes(&result.as_ptr())
            .map(|bytes| bytes.to_vec())
            .map_err(|error| interp.report_cmd_error(error.into()));
    }
    let elements = interp
        .list_elements(&result.as_ptr())
        .map_err(|error| interp.report_cmd_error(error.into()))?;
    let mut bytes = Vec::new();
    for (index, element) in elements.iter().enumerate() {
        if index > 0 {
            bytes.extend_from_slice(separator);
        }
        bytes.extend_from_slice(
            &interp
                .native_string_bytes(element)
                .map_err(|error| interp.report_cmd_error(error.into()))?,
        );
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXCESS: &[&[u8]] = &[b"__r2286_operand__" as &[u8]; 10];
    type ScopeCase = (&'static str, &'static [u8], &'static [&'static [u8]]);
    const CASES: &[ScopeCase] = &[
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
            "{}/../../rust/tcl-registry/tests/data/native_info_inventory_original/jim/{id}/stdout.tsv",
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
            let mut interp = Interp::with_native_core(
                crate::interp::default_host(),
                crate::environment::profile_for_dialect("jim"),
                tcl_registry::special_vars::NativeBootstrapInputs::default(),
            )
            .unwrap();
            let objects: Vec<_> = words
                .iter()
                .map(|word| Owned::fresh(new_string(word)))
                .collect();
            let argv: Vec<_> = objects.iter().map(Owned::as_ptr).collect();
            let code = interp.dispatch(&argv);
            assert!(
                !interp.host_refusal_pending(),
                "{id}: {:?}",
                interp.native_access_refusal()
            );
            let (success, bytes) = native_row(id);
            assert_eq!(
                code,
                if success { Code::Ok } else { Code::Error },
                "{id}: {:?}",
                interp.result_bytes()
            );
            assert_eq!(
                interp
                    .native_string_bytes(&interp.get_obj_result())
                    .unwrap()
                    .as_ref(),
                bytes,
                "{id}"
            );
        }
    }

    fn sort_failure(interp: &mut Interp, _: &[*mut TclObj]) -> Code {
        interp.set_error(b"SORT_ERROR")
    }

    #[test]
    fn current_lsort_failure_is_reported_by_the_selected_jim_parser() {
        // Source proof: naming.info.jim-original-selector-table
        // docs/design/analysis/name-resolution-proofs/info-jim-original-selector-table.md
        let mut interp = Interp::with_native_core(
            crate::interp::default_host(),
            crate::environment::profile_for_dialect("jim"),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .unwrap();
        interp.register_builtin(b"lsort", sort_failure);
        let head = Owned::fresh(new_string(b"info"));
        let absent = Owned::fresh(new_string(b"__absent__"));
        assert_eq!(
            interp.dispatch(&[head.as_ptr(), absent.as_ptr()]),
            Code::Error
        );
        assert_eq!(
            interp.result_bytes(),
            b"info, unknown command \"__absent__\": should be SORT_ERROR"
        );
        let commands = Owned::fresh(new_string(b"-commands"));
        assert_eq!(
            interp.dispatch(&[head.as_ptr(), commands.as_ptr()]),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"SORT_ERROR");
    }
}
