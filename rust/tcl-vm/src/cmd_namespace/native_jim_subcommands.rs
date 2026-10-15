// SPDX-License-Identifier: AGPL-3.0-or-later
//! ParseSubCmd consumes the original object's distinct native table cache.

use super::*;
use crate::interp::native_name_consumers::OriginalNameConsumerScope;
use tcl_cmd_core::CmdError;
use tcl_registry::native_jim_lookup::NativeJimNamespaceSubcommand;
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
            let cache = match selection {
                Ok(cache) => cache,
                Err(_) => {
                    let names: Vec<_> = table.entries().iter().map(|entry| entry.name()).collect();
                    let protocol = dialect.native_name_protocol().ok_or(
                        ValueError::CommandProtocolUnavailable(
                            "original namespace selector issuer",
                        ),
                    )?;
                    return Ok(Err(err(
                        tcl_cmd_core::namespace::unknown_subcommand_message(
                            protocol,
                            &names,
                            protocol.namespace_subcommand_input(&bytes).selected(),
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

#[cfg(test)]
mod tests {
    use super::*;

    const OBSERVED: &str =
        include_str!("../../tests/data/native_jim_namespace_subcommand/table-20261015.tsv");

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
