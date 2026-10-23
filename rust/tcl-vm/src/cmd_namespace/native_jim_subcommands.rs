// SPDX-License-Identifier: AGPL-3.0-or-later
//! `ParseSubCmd` consumes the original object's distinct native table cache.

use super::{Code, Completion, Value, Vm, err};
use crate::interp::native_name_consumers::OriginalNameConsumerScope;
use tcl_cmd_core::CmdError;
use tcl_registry::native_jim_lookup::{
    NativeJimNamespaceSubcommand, NativeJimNamespaceSubcommandTable,
};
use tcl_syntax::value::ValueError;

pub(super) fn select(
    vm: &mut Vm,
    original: &Value,
    count: usize,
    scope: &OriginalNameConsumerScope,
) -> Result<&'static NativeJimNamespaceSubcommand, Completion<Value>> {
    let reached = (|| -> Result<_, CmdError> {
        scope.current_for_value(vm, original)?;
        let dialect = vm.actual_native_invocation_dialect();
        let table = dialect.native_jim_namespace_subcommand_table().ok_or(
            ValueError::CommandProtocolUnavailable("original namespace subcommand table issuer"),
        )?;
        let entry = if let Some(cache) = original.native_jim_subcommand_cache() {
            table
                .cached_entry(&cache)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "foreign original namespace subcommand cache",
                ))?
        } else {
            let bytes = scope.read(vm, original)?;
            let selection = table.lookup_using(&bytes, &mut |index| {
                scope.current_for_value(vm, original)?;
                let compared = vm.native_jim_compare_immediate_checked(original, &table, index);
                scope.current_for_value(vm, original)?;
                compared.map_err(CmdError::from)
            })?;
            let cache =
                match selection {
                    Ok(cache) => cache,
                    Err(failure) => {
                        let commands = command_list(vm, table, b", ", original, scope)?;
                        let head = vm.invoked_name_value().ok_or(
                            ValueError::CommandProtocolUnavailable("original Jim diagnostic head"),
                        )?;
                        let original_head = head.native_lifetime_lease();
                        drop(head);
                        let head = scope.read(vm, original_head.value())?;
                        drop(original_head);
                        let member = scope.read(vm, original)?;
                        let names = scope.read(vm, &commands.original)?;
                        scope.current_for_value(vm, original)?;
                        return Ok(Err(err(
                            tcl_cmd_core::namespace::jim_namespace_subcommand_diagnostic(
                                &head,
                                &member,
                                &names,
                                matches!(failure, tcl_cmd_core::prefix::Resolution::Ambiguous),
                            ),
                        )));
                    }
                };
            let entry =
                table
                    .cached_entry(&cache)
                    .ok_or(ValueError::CommandProtocolUnavailable(
                        "original namespace selected table entry",
                    ))?;
            original.install_native_jim_subcommand_cache_checked(cache, dialect, &mut || {
                scope.current_for_value(vm, original)
            })?;
            entry
        };
        scope.current_for_value(vm, original)?;
        if !entry.accepts_arguments(count) {
            return Ok(Err(arity(vm, original, entry, scope)?));
        }
        Ok(Ok(entry))
    })();
    match reached {
        Ok(selected) => selected,
        Err(error) => Err(crate::command::completion_from_cmd_error(vm, error)),
    }
}

/// Private `ListJoin` output or the actual retained result of a failed lsort.
struct OriginalParserCommandList {
    original: Value,
    fresh: bool,
}

impl Drop for OriginalParserCommandList {
    fn drop(&mut self) {
        if self.fresh
            && self.original.native_object_is_live()
            && self.original.native_object_reference_count() == 0
        {
            self.original.retire_unowned_native_header();
        }
    }
}

#[allow(clippy::too_many_lines)] // Keep original dispatch, join and cleanup stages together.
fn command_list(
    vm: &mut Vm,
    table: NativeJimNamespaceSubcommandTable,
    separator: &[u8],
    selector: &Value,
    scope: &OriginalNameConsumerScope,
) -> Result<OriginalParserCommandList, CmdError> {
    scope.current_for_value(vm, selector)?;
    let dialect = vm.actual_native_invocation_dialect();
    let strings =
        dialect
            .native_string_protocol()
            .ok_or(ValueError::CommandProtocolUnavailable(
                "original Jim command-list String issuer",
            ))?;
    let context = vm.native_jim_object_context()?;
    let names = table
        .entries()
        .iter()
        .map(|entry| Value::new_native_string_bytes(entry.name().as_bytes()))
        .collect();
    let list = OriginalParserCommandList {
        original: Value::native_list_constructor(names, strings).into_native_unowned_lifetime(),
        fresh: true,
    };
    let head = OriginalParserCommandList {
        original: Value::new_native_string_bytes(table.diagnostic_sort_head())
            .into_native_unowned_lifetime(),
        fresh: true,
    };
    for original in [&head.original, &list.original] {
        original.bind_native_jim_context(&context)?;
        scope.current_for_value(vm, selector)?;
    }
    let completion =
        vm.invoke_host_original_object_vector(&head.original, std::slice::from_ref(&list.original));
    drop(head);
    drop(list);
    scope.current_for_value(vm, selector)?;
    let code = completion.code;
    let sorted = vm.with_native_interp_result(Value::native_lifetime_lease)?;
    drop(completion);
    let current = |vm: &mut Vm| -> Result<(), CmdError> {
        scope.current_for_value(vm, selector)?;
        sorted.value().check_native_header()?;
        Ok(())
    };
    current(vm)?;
    if code != Code::Ok {
        return Ok(OriginalParserCommandList {
            original: sorted.into_value(),
            fresh: false,
        });
    }
    let members = {
        let actual = std::cell::RefCell::new(&mut *vm);
        sorted.value().native_object_list_elements_with_strings(
            strings,
            &mut || current(&mut actual.borrow_mut()),
            &mut |value| {
                let mut actual = actual.borrow_mut();
                current(&mut actual)?;
                let bytes = actual.native_name_operand_bytes_checked_using(value, &current);
                current(&mut actual)?;
                bytes
            },
        )?
    };
    current(vm)?;
    let joined = OriginalParserCommandList {
        original: Value::new_native_string_bytes(b"".as_slice()).into_native_unowned_lifetime(),
        fresh: true,
    };
    joined.original.bind_native_jim_context(&context)?;
    let append = dialect.native_object_append_protocol(None).ok_or(
        ValueError::CommandProtocolUnavailable("original Jim command-list append issuer"),
    )?;
    let private_current = |vm: &mut Vm| -> Result<(), CmdError> {
        current(vm)?;
        joined.original.check_native_header()?;
        members.check_generation()?;
        scope.require(
            vm,
            sorted.value().native_list_attachment_matches(&members),
            "original Jim sorted command-list backing",
        )?;
        scope.require(
            vm,
            joined.original.native_object_reference_count() == 0
                && !joined.original.native_object_is_shared(),
            "original Jim private command-list receiver",
        )
    };
    let length = members.elements()?.len();
    for index in 0..length {
        let original = members.elements()?[index].native_lifetime_lease();
        let member_current = |vm: &mut Vm| -> Result<(), CmdError> {
            private_current(vm)?;
            original.value().check_native_header()?;
            Ok(())
        };
        vm.native_name_operand_bytes_checked_using(original.value(), &member_current)?;
        member_current(vm)?;
        let mut first = None;
        let appended = tcl_cmd_core::native_append::append_unshared_jim_object_checked(
            &crate::value::VmAppendObjects,
            append.recipe(),
            &joined.original,
            original.value(),
            &mut || {
                member_current(vm).map_err(|error| {
                    first.get_or_insert(error);
                    ValueError::CommandProtocolUnavailable("original Jim command-list append owner")
                })
            },
        );
        if let Some(error) = first {
            return Err(error);
        }
        appended?;
        if index + 1 < length {
            let mut first = None;
            let appended = tcl_cmd_core::native_append::append_unshared_jim_cstring(
                &crate::value::VmAppendObjects,
                append.recipe(),
                &joined.original,
                separator,
                &mut || {
                    private_current(vm).map_err(|error| {
                        first.get_or_insert(error);
                        ValueError::CommandProtocolUnavailable(
                            "original Jim command-list separator owner",
                        )
                    })
                },
            );
            if let Some(error) = first {
                return Err(error);
            }
            appended?;
        }
    }
    private_current(vm)?;
    Ok(joined)
}

#[cfg(test)]
mod tests {
    use super::*;

    const OBSERVED: &str =
        include_str!("../../tests/data/native_jim_namespace_subcommand/table-20261015.tsv");

    #[test]
    fn rejected_selector_uses_the_actual_mutable_sort_result_and_preserves_sort_error_text() {
        const ORIGINAL: &str = include_str!(
            "../../../../runtime/rust/tests/data/native_jim_namespace_subcommand/table-20261015.tsv"
        );
        for (label, head, body) in [
            (
                "MISSING_REPLACED_SORT",
                "renamed",
                "return {which canonical}",
            ),
            (
                "MISSING_FAILED_SORT",
                "namespace",
                "return -code error SORTFAIL",
            ),
        ] {
            let mut vm = crate::native_fixture::interpreter(
                crate::environment::profile_for_dialect("jimtcl"),
            );
            assert_eq!(
                vm.eval_source(&format!("proc lsort {{args}} {{{body}}}"))
                    .unwrap()
                    .code,
                Code::Ok
            );
            vm.set_invoked_name(head);
            let selector = Value::new_native_string_bytes(b"unavailable".as_slice());
            for suffix in ["COLD", "REUSE"] {
                let scope = OriginalNameConsumerScope::enter(&mut vm).unwrap();
                let completion = select(&mut vm, &selector, 0, &scope).unwrap_err();
                let phase = format!("{label}_{suffix}");
                let expected = ORIGINAL
                    .lines()
                    .find_map(|line| {
                        line.strip_prefix(&format!("RESULT|{phase}|{}|", completion.code.as_int()))
                    })
                    .unwrap();
                let expected: Vec<_> = expected
                    .as_bytes()
                    .chunks_exact(2)
                    .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                    .collect();
                assert_eq!(
                    completion.result.resident_string_bytes().unwrap().as_ref(),
                    expected,
                    "{phase}"
                );
                assert!(
                    ORIGINAL.lines().any(|line| line
                        == format!(
                            "RESULT_HEADER|{phase}|1|{}",
                            completion.result.native_object_type_name()
                        )),
                    "{phase}"
                );
                scope.finish(&mut vm).unwrap();
            }
        }
    }

    #[test]
    fn original_cache_is_installed_before_arity_and_consumed_without_a_string_getter() {
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect("jimtcl"));
        vm.set_invoked_name("namespace");
        let selector = Value::new_native_string_bytes(b"parent".as_slice());
        let scope = OriginalNameConsumerScope::enter(&mut vm).unwrap();
        let completion = select(&mut vm, &selector, 2, &scope).unwrap_err();
        assert_eq!(completion.code, Code::Error);
        assert_eq!(selector.native_object_type_name(), "subcmd-lookup");
        let case = "TABLE_parent_2_REUSE";
        for (kind, actual) in [
            ("MEMBER_AFTER", selector.native_object_type_name()),
            ("RESULT_HEADER", completion.result.native_object_type_name()),
        ] {
            assert!(
                OBSERVED
                    .lines()
                    .any(|line| line == format!("{kind}|{case}|1|{actual}")),
                "{kind}"
            );
        }
        let expected = OBSERVED
            .lines()
            .find_map(|line| line.strip_prefix(&format!("RESULT|{case}|1|")))
            .unwrap();
        let bytes: Vec<_> = expected
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect();
        assert_eq!(
            completion.result.resident_string_bytes().unwrap().as_ref(),
            bytes
        );
        drop(completion);
        selector.invalidate_native_string_for_test();
        assert!(selector.resident_string_bytes().is_none());
        assert_eq!(
            select(&mut vm, &selector, 0, &scope).unwrap().name(),
            "parent"
        );
        assert!(selector.resident_string_bytes().is_none());
        scope.finish(&mut vm).unwrap();
    }
}

fn arity(
    vm: &mut Vm,
    original: &Value,
    entry: &NativeJimNamespaceSubcommand,
    scope: &OriginalNameConsumerScope,
) -> Result<Completion<Value>, CmdError> {
    scope.current_for_value(vm, original)?;
    let dialect = vm.actual_native_invocation_dialect();
    let protocol = dialect.native_object_append_protocol(None).ok_or(
        ValueError::CommandProtocolUnavailable("original Jim public arity append issuer"),
    )?;
    let published = vm.adopt_native_interp_result(Value::new_native_string_bytes(
        b"wrong # args: should be \"".as_slice(),
    ))?;
    let current = |vm: &mut Vm| {
        scope.current_for_value(vm, original)?;
        published.value().check_native_header()?;
        let selected = published.value().native_object_reference_count() == 1
            && !published.value().native_object_is_shared()
            && vm.with_native_interp_result(|actual| actual.is_same_object(published.value()))?;
        scope.require(vm, selected, "original Jim public arity published receiver")
    };
    current(vm)?;
    let head = vm
        .invoked_name_value()
        .ok_or(ValueError::CommandProtocolUnavailable(
            "original Jim public arity head",
        ))?;
    let head = vm.native_name_operand_bytes_checked_using(&head, &current);
    current(vm)?;
    let head = head?;
    let mut pieces: Vec<&[u8]> = vec![&head, b" ", entry.name().as_bytes()];
    if !entry.usage().is_empty() {
        pieces.extend([b" ".as_slice(), entry.usage().as_bytes()]);
    }
    pieces.push(b"\"");
    for piece in pieces {
        let mut first = None;
        let appended = tcl_cmd_core::native_append::append_published_unshared_jim_cstring(
            &crate::value::VmAppendObjects,
            protocol.recipe(),
            published.value(),
            piece,
            &mut || {
                current(vm).map_err(|error| {
                    first.get_or_insert(error);
                    ValueError::CommandProtocolUnavailable("original Jim public arity owner")
                })
            },
        );
        if let Some(error) = first {
            return Err(error);
        }
        current(vm)?;
        appended?;
    }
    let code = vm.apply_cmd_error_code(
        tcl_cmd_core::ResolvedCmdErrorCodeUpdate::Set(b"NONE".to_vec()),
        false,
    );
    current(vm)?;
    Ok(Completion::new_error_metadata(
        Code::Error,
        published.into_value(),
        crate::command::options_dict(Code::Error, 0, &[("-errorcode", code)]),
    ))
}
