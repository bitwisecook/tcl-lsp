// SPDX-License-Identifier: AGPL-3.0-or-later
//! Jim Parent consumes and publishes actual original object holders.

use super::{CmdError, Value, ValueError, Vm};
use tcl_syntax::naming::NativeJimNamespaceParentResult;

impl Vm {
    pub(crate) fn original_jim_namespace_parent_object(
        &mut self,
        original: &Value,
        check: &impl Fn(&mut Vm) -> Result<(), CmdError>,
    ) -> Result<Value, CmdError> {
        let original = original.native_lifetime_lease();
        let scope = super::super::native_operation_currency::NativeOperationScope::enter(self)?;
        let current = |vm: &mut Vm| {
            check(vm)?;
            super::super::native_numeric_ops::OriginalNumericOps::in_original_scope(vm, &scope)?
                .current_for_value(original.value())
        };
        current(self)?;
        let dialect = self.actual_native_invocation_dialect();
        let bytes = self.native_name_operand_bytes_checked_using(original.value(), &current);
        current(self)?;
        let bytes = bytes?;
        if !bytes.is_empty() {
            let table = dialect
                .native_jim_namespace_parent_root_comparison_table()
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "Jim parent root comparison declaration",
                ))?;
            let root = self.native_jim_compare_immediate_checked(original.value(), &table, 0);
            current(self)?;
            if root? {
                return self.original_jim_parent_result(&current);
            }
        }
        let protocol = dialect
            .native_name_protocol()
            .filter(|p| p.is_jim084())
            .ok_or(ValueError::CommandProtocolUnavailable(
                "Jim parent text issuer",
            ))?;
        match protocol
            .jim_namespace_parent_result(&bytes)
            .map_err(|_| ValueError::CommandProtocolUnavailable("Jim parent text result purpose"))?
        {
            NativeJimNamespaceParentResult::Unchanged => {}
            NativeJimNamespaceParentResult::CountedQualifier(bytes) => {
                self.adopt_native_interp_result(Value::new_native_string_bytes(bytes))?;
                current(self)?;
            }
            NativeJimNamespaceParentResult::RootedQualifier(bytes) => {
                self.append_original_jim_parent_qualifier(Some(bytes), &current)?;
            }
            NativeJimNamespaceParentResult::RootedSharedEmpty => {
                self.append_original_jim_parent_qualifier(None, &current)?;
            }
        }
        self.original_jim_parent_result(&current)
    }

    fn original_jim_parent_result(
        &mut self,
        check: &impl Fn(&mut Vm) -> Result<(), CmdError>,
    ) -> Result<Value, CmdError> {
        check(self)?;
        let value = self.with_native_interp_result(Value::native_lifetime_lease)?;
        value.value().check_native_header()?;
        check(self)?;
        Ok(value.into_value())
    }

    fn append_original_jim_parent_qualifier(
        &mut self,
        fresh: Option<&[u8]>,
        check: &impl Fn(&mut Vm) -> Result<(), CmdError>,
    ) -> Result<(), CmdError> {
        let context = self.native_jim_object_context()?;
        let qualifier = fresh.map_or_else(
            || context.empty_object().native_lifetime_lease().into_value(),
            |bytes| Value::new_native_string_bytes(bytes).into_native_unowned_lifetime(),
        );
        let reached = (|| {
            check(self)?;
            let append = self
                .actual_native_invocation_dialect()
                .native_object_append_protocol(None)
                .ok_or(ValueError::CommandProtocolUnavailable(
                    "Jim parent qualifier append",
                ))?;
            let published =
                self.adopt_native_interp_result(Value::new_native_string_bytes(b"::".as_slice()))?;
            check(self)?;
            let mut first = None;
            let appended =
                tcl_cmd_core::native_append::append_published_unshared_jim_object_checked(
                    &crate::value::VmAppendObjects,
                    append.recipe(),
                    published.value(),
                    &qualifier,
                    &mut || {
                        if let Err(error) = check(self) {
                            first.get_or_insert(error);
                            return Err(ValueError::CommandProtocolUnavailable(
                                "Jim parent original operation",
                            ));
                        }
                        qualifier.check_native_header()?;
                        published.value().check_native_header()?;
                        if published.value().native_object_reference_count() != 1
                            || published.value().native_object_is_shared()
                            || !self.with_native_interp_result(|actual| {
                                actual.is_same_object(published.value())
                            })?
                        {
                            return Err(ValueError::CommandProtocolUnavailable(
                                "Jim parent actual published unshared receiver",
                            ));
                        }
                        Ok(())
                    },
                );
            if let Some(error) = first {
                return Err(error);
            }
            check(self)?;
            appended?;
            // Jim balances this temporary source reference only after append.
            drop(
                qualifier
                    .native_lifetime_lease()
                    .into_value()
                    .into_native_reference(),
            );
            check(self)
        })();
        if let Err(error) = &reached
            && let Some(cause) = error.native_execution_refusal()
        {
            self.retain_tcl_host_failure(crate::error::TclHostFailure::Execution(cause.clone()));
        }
        if reached.is_err()
            && fresh.is_some()
            && qualifier.native_object_is_live()
            && qualifier.native_object_reference_count() == 0
        {
            qualifier.retire_unowned_native_header();
        }
        let current = check(self);
        match reached {
            Err(first) => Err(first),
            Ok(()) => current,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::Cell;

    const OBSERVED: &str = include_str!(
        "../../../tests/data/native_jim_namespace_parent/getter-identity-20261015.tsv"
    );

    fn observation(kind: &str, case: &str) -> Vec<&'static str> {
        OBSERVED
            .lines()
            .find_map(|line| {
                let fields: Vec<_> = line.split('|').collect();
                (fields[0] == kind && fields[1] == case).then(|| fields[2..].to_vec())
            })
            .unwrap_or_else(|| panic!("missing original Parent observation {kind}/{case}"))
    }

    #[test]
    fn parent_keeps_original_result_primary_and_empty_identity() {
        for (case, input) in [
            ("RELATIVE_REUSE", b"absent::child".as_slice()),
            ("PLAIN_REUSE", b"absent"),
            ("ABSOLUTE_REUSE", b"::absent::child"),
            ("ROOT_REUSE", b"::"),
            ("EMPTY_REUSE", b""),
            ("NUL_ROOT_REUSE", b"::\0tail"),
            ("NUL_RELATIVE_REUSE", b"absent::child\0tail"),
            ("ONE_COLON_REUSE", b":"),
            ("TRIPLE_COLON_REUSE", b":::"),
        ] {
            let mut vm = crate::native_fixture::interpreter(
                crate::environment::profile_for_dialect("jimtcl"),
            );
            vm.reset_native_jim_result().unwrap();
            let context = vm.native_jim_object_context().unwrap();
            let empty_count = context.empty_object().native_object_reference_count();
            let original = Value::new_native_string_bytes(input);
            let result = vm
                .original_jim_namespace_parent_object(&original, &|_| Ok(()))
                .unwrap();
            let expected = observation("RESULT", case);
            let bytes: Vec<u8> = expected[1]
                .as_bytes()
                .as_chunks::<2>()
                .0
                .iter()
                .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
                .collect();
            assert_eq!(
                result.resident_string_bytes().unwrap().as_ref(),
                bytes,
                "{case}"
            );
            assert_eq!(
                result.native_object_type_name(),
                observation("RESULT_HEADER", case)[1],
                "{case}"
            );
            assert_eq!(
                result.is_same_object(&context.empty_object()),
                observation("RESULT_IS_EMPTY", case)[0] == "1",
                "{case}"
            );
            assert!(!result.is_same_object(&original), "{case}");
            assert!(
                vm.with_native_interp_result(|actual| actual.is_same_object(&result))
                    .unwrap()
            );
            if matches!(case, "PLAIN_REUSE" | "ONE_COLON_REUSE") {
                assert_eq!(
                    context.empty_object().native_object_reference_count(),
                    empty_count - 1,
                    "{case}"
                );
            }
            assert!(vm.execution_refusal.is_none());
        }
    }

    #[test]
    fn unchanged_parent_retains_the_result_published_during_the_original_getter() {
        let mut vm =
            crate::native_fixture::interpreter(crate::environment::profile_for_dialect("jimtcl"));
        let original = Value::list(Vec::new());
        let original_view = original.native_lifetime_lease();
        let published = Value::int(17);
        let reached = Cell::new(false);
        let result = vm
            .original_jim_namespace_parent_object(&original, &|vm| {
                if original_view.value().resident_string_bytes().is_some() && !reached.replace(true)
                {
                    vm.adopt_native_interp_result(published.clone())?;
                }
                Ok(())
            })
            .unwrap();
        assert!(reached.get());
        assert!(result.is_same_object(&published));
        assert!(result.resident_string_bytes().is_none());
    }
}
