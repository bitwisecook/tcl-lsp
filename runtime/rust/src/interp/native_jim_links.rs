// SPDX-License-Identifier: AGPL-3.0-or-later
//! Recursive Jim alias access retains the original target-name getter.

use super::Interp;
mod publication;
use crate::frame::{Link, OriginalJimLinkTarget};
use crate::interp::Code;
use crate::obj::{self, TclObj};
use std::rc::Rc;
use tcl_syntax::native_jim_lookup::NativeJimLinkTargetInput;
use tcl_syntax::value::ValueOps;

impl Interp {
    pub(crate) fn unset_original_jim_variable(
        &mut self,
        original: *mut TclObj,
    ) -> Result<bool, Code> {
        let arguments =
            crate::interp::native_arguments::OriginalArguments::enter(self, &[original])
                .map_err(|error| self.report_cmd_error(error))?;
        let reached = self.unset_original_jim_variable_entered(original);
        arguments
            .check()
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }

    fn unset_original_jim_variable_entered(&mut self, original: *mut TclObj) -> Result<bool, Code> {
        let bytes = self
            .native_string_bytes(&original)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        if self.is_native_jim_dictionary_name(original, &bytes) {
            return self.unset_native_jim_dictionary_sugar(original);
        }
        self.install_original_jim_variable(original, &bytes)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let target = self
            .current_jim_variable_cache(original)
            .map_err(|error| self.report_cmd_error(error.into()))?
            .map(|cache| cache.cell.read_checked())
            .transpose()
            .map_err(|error| self.report_cmd_error(error.into()))?
            .flatten()
            .and_then(|value| match value {
                crate::frame::JimVariableRead::Link(link) => link.original_jim_target,
                _ => None,
            });
        if let Some(target) = target {
            let target_original = target
                .original_name()
                .map_err(|error| self.report_cmd_error(error.into()))?;
            let level = self.frames.borrow().jim_link_level(&target.frame);
            let Some(level) = level else {
                return Err(self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "retired Jim alias frame",
                    )
                    .into(),
                ));
            };
            let saved = self.frames.borrow_mut().set_active_level(level);
            let saved_namespace = self
                .current_ns
                .replace(self.frames.borrow().frame_ns(level));
            let result = self.unset_original_jim_variable(target_original);
            self.current_ns.set(saved_namespace);
            self.frames.borrow_mut().set_active_level(saved);
            target
                .original_name()
                .map_err(|error| self.report_cmd_error(error.into()))?;
            // Jim_UnsetVariable invokes linked targets with JIM_NONE. Its
            // enclosing original alias supplies the eventual diagnostic.
            return match result {
                Err(_) if !self.host_refusal_pending() => Ok(false),
                result => result,
            };
        }
        let (root, element) = self
            .variable_name_parts(&bytes)
            .map_err(|error| crate::builtins::var_error(self, &bytes, error))?;
        Ok(match element {
            Some(element) => self.var_unset_elem(&root, &element),
            None => {
                let scope = super::native_operation_currency::NativeOperationScope::enter(self)
                    .map_err(|first| self.refuse_native_execution(first))?;
                let captured = crate::vars::EnteredVariablePlace::capture(
                    self,
                    self.current_ns.get(),
                    None,
                    &root,
                    None,
                    false,
                    scope.currency(),
                );
                scope
                    .currency()
                    .ensure_current_or_refuse()
                    .map_err(|first| self.refuse_native_execution(first))?;
                let mut captured = captured.map_err(|error| {
                    let error = self.byte_store_failure(error);
                    crate::builtins::var_error(self, &bytes, error)
                })?;
                let reached = captured.remove_jim_primary(self, scope.currency());
                if let Err(crate::frame::JimLinkStoreError::Command(error)) = &reached {
                    if let Some(first) = error.native_execution_refusal() {
                        self.refuse_native_execution(first.clone());
                    }
                }
                drop(captured);
                scope
                    .currency()
                    .ensure_current_or_refuse()
                    .map_err(|first| self.refuse_native_execution(first))?;
                reached.map_err(|error| {
                    let error = self.byte_store_failure(error);
                    crate::builtins::var_error(self, &bytes, error)
                })?
            }
        })
    }
    pub(super) fn read_original_jim_link(
        &mut self,
        original: *mut TclObj,
        link: &Link,
        report: super::native_jim_lookup::OriginalJimVariableReport,
    ) -> Option<Result<*mut TclObj, Code>> {
        let target = Rc::clone(link.original_jim_target.as_ref()?);
        let target_original = match target.original_name() {
            Ok(original) => original,
            Err(error) => return Some(Err(self.report_cmd_error(error.into()))),
        };
        let selected_level = self.frames.borrow().jim_link_level(&target.frame);
        let level = match selected_level {
            Some(level) => level,
            None => {
                return Some(Err(self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "retired Jim alias frame",
                    )
                    .into(),
                )));
            }
        };
        let saved = self.frames.borrow_mut().set_active_level(level);
        let saved_namespace = self
            .current_ns
            .replace(self.frames.borrow().frame_ns(level));
        let result = self.read_original_named_variable_reported(target_original, report);
        self.current_ns.set(saved_namespace);
        self.frames.borrow_mut().set_active_level(saved);
        if let Err(error) = target.original_name() {
            return Some(Err(self.report_cmd_error(error.into())));
        }
        Some(match result {
            Err(code) if self.host_refusal_pending() => Err(code),
            Err(code) => match report {
                super::native_jim_lookup::OriginalJimVariableReport::Missing => {
                    self.report_original_jim_alias_read(original)
                }
                super::native_jim_lookup::OriginalJimVariableReport::Silent => Err(code),
            },
            Ok(value) => Ok(value),
        })
    }
    fn report_original_jim_alias_read(
        &mut self,
        original: *mut TclObj,
    ) -> Result<*mut TclObj, Code> {
        let bytes = self
            .native_object_string_bytes(original)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let protocol = self
            .native_invocation_dialect()
            .native_jim_lookup_protocol()
            .expect("actual Jim link read");
        let message = protocol.linked_variable_read_error(&bytes).map_err(|_| {
            self.report_cmd_error(
                tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                    "Jim alias read diagnostic",
                )
                .into(),
            )
        })?;
        Err(self.error(&message))
    }
    pub(super) fn read_original_jim_link_for_update(
        &mut self,
        link: &Link,
    ) -> Option<Result<Option<*mut TclObj>, Code>> {
        let target = Rc::clone(link.original_jim_target.as_ref()?);
        let target_original = match target.original_name() {
            Ok(original) => original,
            Err(error) => return Some(Err(self.report_cmd_error(error.into()))),
        };
        let selected_level = self.frames.borrow().jim_link_level(&target.frame);
        let level = match selected_level {
            Some(level) => level,
            None => {
                return Some(Err(self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "retired Jim alias frame",
                    )
                    .into(),
                )));
            }
        };
        let saved = self.frames.borrow_mut().set_active_level(level);
        let saved_namespace = self
            .current_ns
            .replace(self.frames.borrow().frame_ns(level));
        let result = self.read_original_named_variable_for_update(target_original);
        self.current_ns.set(saved_namespace);
        self.frames.borrow_mut().set_active_level(saved);
        if let Err(error) = target.original_name() {
            return Some(Err(self.report_cmd_error(error.into())));
        }
        Some(result)
    }
    pub(super) fn store_original_jim_link(
        &mut self,
        link: &Link,
        value: *mut TclObj,
    ) -> Option<Result<(), Code>> {
        let target = Rc::clone(link.original_jim_target.as_ref()?);
        let target_original = match target.original_name() {
            Ok(original) => original,
            Err(error) => return Some(Err(self.report_cmd_error(error.into()))),
        };
        let selected_level = self.frames.borrow().jim_link_level(&target.frame);
        let level = match selected_level {
            Some(level) => level,
            None => {
                return Some(Err(self.report_cmd_error(
                    tcl_syntax::value::ValueError::CommandProtocolUnavailable(
                        "retired Jim alias frame",
                    )
                    .into(),
                )));
            }
        };
        let saved = self.frames.borrow_mut().set_active_level(level);
        let saved_namespace = self
            .current_ns
            .replace(self.frames.borrow().frame_ns(level));
        let result = self.assign_original_named_variable(target_original, value);
        self.current_ns.set(saved_namespace);
        self.frames.borrow_mut().set_active_level(saved);
        if let Err(error) = target.original_name() {
            return Some(Err(self.report_cmd_error(error.into())));
        }
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn decode(hex: &str) -> Vec<u8> {
        assert_eq!(hex.len() % 2, 0);
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    #[test]
    fn original_alias_read_errors_match_thirty_six_native_results() {
        let cases =
            include_str!("../../../../rust/tcl-syntax/tests/data/native_jim_alias_read/cases.tsv");
        let engines = [
            (
                "tcl8.4",
                include_str!(
                    "../../../../rust/tcl-syntax/tests/data/native_jim_alias_read/8.4.tsv"
                ),
            ),
            (
                "tcl8.5",
                include_str!(
                    "../../../../rust/tcl-syntax/tests/data/native_jim_alias_read/8.5.tsv"
                ),
            ),
            (
                "tcl8.6",
                include_str!(
                    "../../../../rust/tcl-syntax/tests/data/native_jim_alias_read/8.6.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../../rust/tcl-syntax/tests/data/native_jim_alias_read/9.0.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../../rust/tcl-syntax/tests/data/native_jim_alias_read/9.1.tsv"
                ),
            ),
            (
                "jim",
                include_str!(
                    "../../../../rust/tcl-syntax/tests/data/native_jim_alias_read/jim.tsv"
                ),
            ),
        ];
        let mut count = 0;
        for (engine, expected) in engines {
            assert_eq!(cases.lines().count(), expected.lines().count());
            for (case, result) in cases.lines().zip(expected.lines()) {
                let (name, source) = case.split_once('\t').unwrap();
                let fields: Vec<_> = result.split('\t').collect();
                assert_eq!(fields[0], name);
                let mut interp = Interp::with_native_core(
                    crate::interp::default_host(),
                    crate::environment::profile_for_dialect(engine),
                    tcl_registry::special_vars::NativeBootstrapInputs {
                        package_path: Vec::new(),
                        default_library: None,
                    },
                )
                .expect("actual native core issuer");
                let code = interp.eval_str(&decode(source));
                assert_eq!(
                    code.as_int(),
                    fields[1].parse::<i64>().unwrap(),
                    "{engine}/{name}"
                );
                assert_eq!(interp.result_bytes(), decode(fields[2]), "{engine}/{name}");
                count += 1;
            }
        }
        assert_eq!(count, 36);
    }
}
