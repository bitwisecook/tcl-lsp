// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original compiled try ranges, their anonymous cells, and RETURN_STK cleanup.
use super::native_list_conversion::BodyObjectOperation;
use super::*;
use tcl_registry::native_control_compilation::{NativeControlCompilation, NativeControlOutcome};
use tcl_registry::native_try_compilation::{NativeTryCondition, NativeTryInstruction};

pub(super) struct TryOperation {
    pub(super) recipe: NativeTryInstruction,
    pub(super) prepared: PreparedControlOperands,
    bindings: Vec<(Option<usize>, Option<usize>)>,
    empty: Option<usize>,
    success_options: Option<usize>,
    during: Option<usize>,
}
impl Builder<'_> {
    pub(super) fn try_operation(
        &mut self,
        captured: &NativeCompilerWords<'_>,
        selected: NativeControlCompilation<NativeTryInstruction>,
        depth: u32,
    ) -> Result<Operation, tcl_cmd_core::CmdError> {
        let prepared = self.prepare_control_steps(captured, &selected.preparations, depth)?;
        let recipe = match selected.outcome {
            NativeControlOutcome::Inline(recipe) => recipe,
            NativeControlOutcome::Generic => return Ok(Operation::Invoke),
            NativeControlOutcome::Rejected(failure) => {
                return Err((self.reject_native_compilation(failure)).into());
            }
        };
        let protocol = self
            .interp
            .native_invocation_dialect()
            .native_compiled_variable_protocol()
            .ok_or_else(|| unavailable("native try local issuer"))?;
        let mut bindings = Vec::with_capacity(recipe.handlers.len());
        for handler in &recipe.handlers {
            let slot = |name: &Option<Vec<u8>>| {
                name.as_ref()
                    .map(|name| {
                        self.lvt
                            .find_native(protocol, name)
                            .ok_or_else(|| unavailable("native try prepared handler local"))
                    })
                    .transpose()
            };
            bindings.push((slot(&handler.result)?, slot(&handler.options)?));
        }
        let empty = recipe
            .handlers
            .iter()
            .any(|handler| handler.empty_body)
            .then(|| self.literals.intern_bytes(b""));
        let success_options =
            (!recipe.catches_success && recipe.finally.is_some() && !recipe.handlers.is_empty())
                .then(|| self.literals.intern_bytes(b"-level 0 -code 0"));
        let during = (recipe.finally.is_some()
            || recipe
                .handlers
                .iter()
                .any(|handler| handler.body.is_some() && !handler.empty_body))
        .then(|| self.literals.intern_bytes(b"-during"));
        Ok(Operation::Try(TryOperation {
            recipe,
            prepared,
            bindings,
            empty,
            success_options,
            during,
        }))
    }
}

/// These are genuine native operand-stack roles, or nonowning references into
/// the actual entered frame's anonymous result/options cells.
pub(super) struct BodyProtectedCompletion {
    code: Code,
    storage: ProtectedStorage,
    original: Rc<BodyObjectOperation>,
}
enum ProtectedStorage {
    Stack {
        result: obj::Owned,
        options: obj::Owned,
    },
    Locals {
        activation: u64,
        result: usize,
        options: usize,
        headers: crate::interp::native_arguments::OriginalArguments,
        result_original: *mut TclObj,
        options_original: *mut TclObj,
    },
}
impl BodyProtectedCompletion {
    fn originals(&self, interp: &mut Interp) -> Result<(*mut TclObj, *mut TclObj), Code> {
        self.original
            .check(interp)
            .map_err(|error| interp.report_cmd_error(error))?;
        match &self.storage {
            ProtectedStorage::Stack { result, options } => {
                obj::check_native_liveness(result.as_ptr())
                    .and_then(|()| obj::check_native_liveness(options.as_ptr()))
                    .map_err(|error| interp.report_cmd_error(self.original.refuse(error)))?;
                Ok((result.as_ptr(), options.as_ptr()))
            }
            ProtectedStorage::Locals {
                activation,
                result,
                options,
                headers,
                result_original,
                options_original,
            } => {
                headers
                    .check_original_for(interp, 0, *result_original)
                    .and_then(|()| headers.check_original_for(interp, 1, *options_original))
                    .map_err(|error| interp.report_cmd_error(error))?;
                let originals = {
                    let frames = interp.frames.borrow();
                    if frames.current_activation() != *activation {
                        None
                    } else {
                        frames
                            .native_compiled_temporary(*result)
                            .ok()
                            .flatten()
                            .zip(frames.native_compiled_temporary(*options).ok().flatten())
                    }
                };
                originals
                    .filter(|(result, options)| {
                        result == result_original && options == options_original
                    })
                    .ok_or_else(|| {
                        interp.report_cmd_error(
                            unavailable("native protected completion frame lifetime").into(),
                        )
                    })
            }
        }
    }
    fn store(self, interp: &mut Interp, slots: &[usize]) -> Result<Self, Code> {
        self.original
            .check(interp)
            .map_err(|error| interp.report_cmd_error(error))?;
        if slots.is_empty() {
            return Ok(self);
        }
        let [result_slot, options_slot, ..] = slots else {
            return Err(
                interp.report_cmd_error(unavailable("native try anonymous slot geometry").into())
            );
        };
        let original = self.original;
        let (result, options) = match self.storage {
            ProtectedStorage::Stack { result, options } => (result, options),
            storage @ ProtectedStorage::Locals { .. } => {
                return Ok(Self {
                    code: self.code,
                    storage,
                    original,
                });
            }
        };
        let result_original = result.as_ptr();
        let options_original = options.as_ptr();
        let headers = crate::interp::native_arguments::OriginalArguments::enter(
            interp,
            &[result_original, options_original],
        )
        .map_err(|error| interp.report_cmd_error(error))?;
        let activation = interp.frames.borrow().current_activation();
        let reached = (|| {
            interp
                .store_native_compiled_temporary_checked(*options_slot, options_original)
                .map_err(|_| {
                    interp.report_cmd_error(
                        unavailable("native try anonymous completion cells").into(),
                    )
                })?;
            original
                .check(interp)
                .map_err(|error| interp.report_cmd_error(error))?;
            interp
                .store_native_compiled_temporary_checked(*result_slot, result_original)
                .map_err(|_| {
                    interp.report_cmd_error(
                        unavailable("native try anonymous completion cells").into(),
                    )
                })?;
            original
                .check(interp)
                .map_err(|error| interp.report_cmd_error(error))?;
            Ok(())
        })();
        drop(options);
        drop(result);
        original
            .check(interp)
            .map_err(|error| interp.report_cmd_error(error))?;
        reached?;
        let stored = Self {
            code: self.code,
            storage: ProtectedStorage::Locals {
                activation,
                result: *result_slot,
                options: *options_slot,
                headers,
                result_original,
                options_original,
            },
            original,
        };
        stored.originals(interp)?;
        Ok(stored)
    }
}
impl Interp {
    /// PUSH_RESULT and PUSH_RETURN_OPTIONS retain the exact native originals.
    pub(super) fn capture_body_completion(
        &mut self,
        original: &Rc<BodyObjectOperation>,
        code: Code,
    ) -> Result<BodyProtectedCompletion, Code> {
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let reached = (|| {
            let result = obj::Owned::retain(self.result_obj());
            let options = crate::cmd_error::completion_options(self, code)
                .map_err(|error| self.refuse_native_execution(error))?;
            let options = obj::Owned::fresh(options);
            if self.host_refusal_pending() {
                return Err(Code::Error);
            }
            Ok(BodyProtectedCompletion {
                code,
                storage: ProtectedStorage::Stack { result, options },
                original: original.clone(),
            })
        })();
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }

    /// RETURN_STK merges the original options operand independently from a public
    /// command binding, then publishes the same retained result header.
    pub(super) fn reapply_body_completion(
        &mut self,
        original: &Rc<BodyObjectOperation>,
        completion: &BodyProtectedCompletion,
    ) -> Result<Code, Code> {
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let reached = (|| {
            let (result, options) = completion.originals(self)?;
            let result = obj::Owned::retain(result);
            let options = obj::Owned::retain(options);
            let (ops, protocol) = crate::return_options::NativeReturnOps::selected(self)
                .map_err(|error| self.report_cmd_error(error))?;
            let mut ops = ops.with_original_objects(
                original
                    .objects(self)
                    .map_err(|error| self.report_cmd_error(error))?,
            );
            let merged =
                tcl_cmd_core::native_return_merge::merge_stack(&mut ops, protocol, &options)
                    .map_err(|error| self.report_cmd_error(error))?;
            let objects = original
                .objects(self)
                .map_err(|error| self.report_cmd_error(error))?;
            let code = self
                .process_original_c_return_options_with_objects(
                    tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate,
                    merged.code,
                    i64::from(merged.level),
                    merged.options.as_ptr(),
                    &objects,
                )
                .map_err(|error| self.report_cmd_error(error.into()))?;
            self.set_result(result.as_ptr());
            Ok(code)
        })();
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }

    fn splice_try_during(
        &mut self,
        original: &Rc<BodyObjectOperation>,
        artifact: &NativeBodyArtifact,
        operation: &TryOperation,
        previous: &BodyProtectedCompletion,
        replacement: BodyProtectedCompletion,
    ) -> Result<BodyProtectedCompletion, Code> {
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let reached = (|| {
            if replacement.code != Code::Error {
                return Ok(replacement);
            }
            let (_, prior_options) = previous.originals(self)?;
            let ProtectedStorage::Stack { result, options } = replacement.storage else {
                return Err(self
                    .report_cmd_error(unavailable("native try new exception stack roles").into()));
            };
            let key = artifact
                .literals
                .original(operation.during.expect("native try cleanup key"))
                .expect("native -during literal");
            let options_header = crate::interp::native_arguments::OriginalArguments::enter(
                self,
                &[options.as_ptr()],
            )
            .map_err(|error| self.report_cmd_error(error))?;
            let mut dictionary = crate::dict::PreparedNativeDictionary::prepare_selected_using(
                Some(options.as_ptr()),
                artifact.stamp.source_protocol,
                tcl_cmd_core::native_dictionary::NativeDictionaryPreparation::CopyBeforeConversion,
                &|| {
                    original.check(self)?;
                    options_header.check_for(self)
                },
                &|error| original.refuse(error),
            )
            .map_err(|error| self.report_cmd_error(error.into()))?;
            drop(options);
            original
                .check(self)
                .map_err(|error| self.report_cmd_error(error))?;
            previous.originals(self)?;
            dictionary
                .set_member_using(
                    key,
                    prior_options,
                    &|| {
                        original.check(self)?;
                        previous
                            .originals(&mut self.clone())
                            .map(|_| ())
                            .map_err(|_| {
                                original.refuse(unavailable("native try prior completion roles"))
                            })
                    },
                    &|error| original.refuse(error),
                    &|key, current| original.bytes_using(self, key, current),
                )
                .map_err(|error| self.report_cmd_error(error.into()))?;
            Ok(BodyProtectedCompletion {
                code: Code::Error,
                storage: ProtectedStorage::Stack {
                    result,
                    options: dictionary.into_value(),
                },
                original: original.clone(),
            })
        })();
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }

    fn bind_try_local(
        &mut self,
        original: &Rc<BodyObjectOperation>,
        name: &[u8],
        slot: usize,
        value: *mut TclObj,
    ) -> Code {
        if let Err(error) = original.check(self) {
            return self.report_cmd_error(error);
        }
        let reached = (|| {
            let captured = crate::vars::capture_original_indexed_receiver(
                &mut self.frames.borrow_mut(),
                &mut self.namespaces.borrow_mut(),
                slot,
                None,
                true,
            );
            let (receiver, home) = match captured {
                Ok(Some(value)) => value,
                Ok(None) => {
                    return self
                        .report_cmd_error(unavailable("native try handler variable cell").into());
                }
                Err(error) => return crate::builtins::var_error(self, name, error),
            };
            if let Err(error) = receiver.store(value) {
                return crate::builtins::var_error(self, name, error);
            }
            if let Err(error) = original.check(self) {
                return self.report_cmd_error(error);
            }
            if let Err(error) = receiver.check_current() {
                return crate::builtins::var_error(self, name, error);
            }
            if self.has_variable_traces() {
                let access = self.trace_access(name, name, None, &home, false);
                if match self.fire_var_trace_resolved(&home, &access, b"write") {
                    Ok(outcome) => outcome.is_guest_error(),
                    Err(first) => return self.refuse_native_execution(first),
                } {
                    return crate::builtins::var_error(
                        self,
                        name,
                        crate::frame::VarError::TraceError,
                    );
                }
            }
            if self.host_refusal_pending() {
                Code::Error
            } else {
                Code::Ok
            }
        })();
        if let Err(error) = original.check(self) {
            return self.report_cmd_error(error);
        }
        reached
    }

    fn try_matches(
        &mut self,
        original: &Rc<BodyObjectOperation>,
        artifact: &NativeBodyArtifact,
        condition: &NativeTryCondition,
        completion: &BodyProtectedCompletion,
    ) -> Result<bool, Code> {
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let reached = (|| {
            if i64::from(condition.code()) != completion.code.as_int() {
                return Ok(false);
            }
            let NativeTryCondition::Trap(pattern) = condition else {
                return Ok(true);
            };
            let (_, options) = completion.originals(self)?;
            let pairs = original
                .pairs(self, options, artifact.stamp.source_protocol)
                .map_err(|error| self.report_cmd_error(error))?;
            let mut code = None;
            for (key, value) in pairs.pairs.iter().copied() {
                pairs
                    .check(self, original)
                    .map_err(|error| self.report_cmd_error(error))?;
                let bytes = original
                    .bytes(self, key)
                    .map_err(|error| self.report_cmd_error(error))?;
                pairs
                    .check(self, original)
                    .map_err(|error| self.report_cmd_error(error))?;
                if bytes.as_ref() == b"-errorcode" {
                    code = Some(value);
                    break;
                }
            }
            let code = code.ok_or_else(|| {
                self.report_cmd_error(unavailable("native try trapped error code").into())
            })?;
            let members = original
                .list(self, code, artifact.stamp.source_protocol)
                .map_err(|error| self.report_cmd_error(error))?;
            pairs
                .check(self, original)
                .map_err(|error| self.report_cmd_error(error))?;
            if members.members.len() < pattern.len() {
                return Ok(false);
            }
            for (member, expected) in members.members.iter().zip(pattern) {
                members
                    .check(self, original)
                    .map_err(|error| self.report_cmd_error(error))?;
                pairs
                    .check(self, original)
                    .map_err(|error| self.report_cmd_error(error))?;
                let bytes = original
                    .bytes(self, *member)
                    .map_err(|error| self.report_cmd_error(error))?;
                members
                    .check(self, original)
                    .map_err(|error| self.report_cmd_error(error))?;
                pairs
                    .check(self, original)
                    .map_err(|error| self.report_cmd_error(error))?;
                if bytes.as_ref() != expected.as_slice() {
                    return Ok(false);
                }
            }
            Ok(true)
        })();
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }

    pub(super) fn execute_body_try(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        operation: &TryOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let original =
            &BodyObjectOperation::capture(self).map_err(|error| self.report_cmd_error(error))?;
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let reached = (|| {
            let recipe = &operation.recipe;
            let body_code = self.execute_control_body(
                artifact,
                command,
                &operation.prepared,
                &recipe.body,
                execution,
            );
            original
                .check(self)
                .map_err(|error| self.report_cmd_error(error))?;
            if execution.done || self.host_refusal_pending() || self.exit_pending() {
                return Ok(body_code);
            }
            if recipe.handlers.is_empty() && recipe.finally.is_none() {
                return Ok(body_code);
            }
            if body_code == Code::Ok && !recipe.catches_success && recipe.finally.is_none() {
                return Ok(Code::Ok);
            }
            let initial = if let (Code::Ok, Some(options)) = (body_code, operation.success_options)
            {
                BodyProtectedCompletion {
                    original: original.clone(),
                    code: body_code,
                    storage: ProtectedStorage::Stack {
                        result: obj::Owned::retain(self.result_obj()),
                        options: obj::Owned::retain(
                            artifact
                                .literals
                                .original(options)
                                .expect("native normal try options"),
                        ),
                    },
                }
            } else {
                self.capture_body_completion(original, body_code)?
            };
            let mut outcome = initial.store(self, &operation.prepared.temporaries)?;
            let mut matched = None;
            for (index, handler) in recipe.handlers.iter().enumerate() {
                if self.try_matches(original, artifact, &handler.condition, &outcome)? {
                    matched = Some(index);
                    break;
                }
            }
            if let Some(index) = matched {
                let handler = &recipe.handlers[index];
                let (result, options) = outcome.originals(self)?;
                let (result_slot, options_slot) = operation.bindings[index];
                let mut code = Code::Ok;
                if let Some(slot) = result_slot {
                    code = self.bind_try_local(
                        original,
                        handler.result.as_ref().unwrap(),
                        slot,
                        result,
                    );
                }
                if code == Code::Ok {
                    if let Some(slot) = options_slot {
                        code = self.bind_try_local(
                            original,
                            handler.options.as_ref().unwrap(),
                            slot,
                            options,
                        );
                    }
                }
                if self.host_refusal_pending() {
                    return Ok(Code::Error);
                }
                if code != Code::Ok && recipe.finally.is_none() {
                    return Ok(code);
                }
                if code == Code::Ok {
                    let target = &recipe.handlers[handler.target];
                    if target.empty_body {
                        self.set_result(
                            artifact
                                .literals
                                .original(operation.empty.expect("native empty handler"))
                                .expect("registered empty handler"),
                        );
                    } else {
                        code = self.execute_control_body(
                            artifact,
                            command,
                            &operation.prepared,
                            target.body.as_ref().expect("non-dash handler target"),
                            execution,
                        );
                    }
                }
                original
                    .check(self)
                    .map_err(|error| self.report_cmd_error(error))?;
                if execution.done || self.host_refusal_pending() || self.exit_pending() {
                    return Ok(code);
                }
                if code == Code::Ok && recipe.finally.is_none() {
                    return Ok(Code::Ok);
                }
                let replacement = self.capture_body_completion(original, code)?;
                let replacement =
                    self.splice_try_during(original, artifact, operation, &outcome, replacement)?;
                if recipe.finally.is_none() {
                    return self.reapply_body_completion(original, &replacement);
                }
                outcome = replacement.store(self, &operation.prepared.temporaries)?;
            }
            if let Some(finally) = &recipe.finally {
                let code = self.execute_control_body(
                    artifact,
                    command,
                    &operation.prepared,
                    finally,
                    execution,
                );
                original
                    .check(self)
                    .map_err(|error| self.report_cmd_error(error))?;
                if execution.done || self.host_refusal_pending() || self.exit_pending() {
                    return Ok(code);
                }
                if code != Code::Ok {
                    let replacement = self.capture_body_completion(original, code)?;
                    let replacement = self.splice_try_during(
                        original,
                        artifact,
                        operation,
                        &outcome,
                        replacement,
                    )?;
                    outcome = replacement.store(self, &operation.prepared.temporaries)?;
                }
            }
            self.reapply_body_completion(original, &outcome)
        })();
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(error))?;
        reached
    }
}

impl TryOperation {
    pub(super) fn compaction_hazards(
        &self,
    ) -> Vec<tcl_registry::native_compiler_pass::NativeCompilerPassHazard> {
        let dynamic = self.recipe.body.script.is_none()
            || self
                .recipe
                .handlers
                .iter()
                .filter_map(|handler| handler.body.as_ref())
                .any(|body| body.script.is_none())
            || self
                .recipe
                .finally
                .as_ref()
                .is_some_and(|body| body.script.is_none());
        if dynamic {
            vec![tcl_registry::native_compiler_pass::NativeCompilerPassHazard::ScriptEvaluation]
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::native_list_conversion::BodyObjectOperation;
    use super::*;
    fn bytes(hex: &str) -> Vec<u8> {
        assert!(hex.len().is_multiple_of(2));
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| {
                let digit = |byte: u8| match byte {
                    b'0'..=b'9' => byte - b'0',
                    b'a'..=b'f' => byte - b'a' + 10,
                    _ => panic!("original native hex"),
                };
                digit(pair[0]) * 16 + digit(pair[1])
            })
            .collect()
    }
    #[test]
    fn compiled_try_matches_original_native_handlers_fallthrough_and_finally() {
        // naming.try-original-compiler-topology
        // docs/design/analysis/name-resolution-proofs/try-original-compiler-topology.md
        // Cases19..39 compare reached guest completions, never a host refusal.
        let cases = include_str!("../../../tests/data/native_compiled_try/cases.tsv");
        for (profile, observations) in [
            (
                "tcl8.6",
                include_str!("../../../tests/data/native_compiled_try/8.6.18.tsv"),
            ),
            (
                "tcl9.0",
                include_str!("../../../tests/data/native_compiled_try/9.0.4.tsv"),
            ),
            (
                "tcl9.1",
                include_str!("../../../tests/data/native_compiled_try/9.1.0.tsv"),
            ),
        ] {
            assert_eq!(observations.lines().count(), 41);
            for (index, case) in cases.lines().enumerate().skip(19) {
                let (label, source) = case.split_once('\t').unwrap();
                let source = bytes(source);
                let mut interp = super::super::tests::interpreter(profile);
                let mut declaration = b"proc p {} {".to_vec();
                declaration.extend_from_slice(&source);
                declaration.push(b'}');
                assert_eq!(
                    interp.eval_str(&declaration),
                    Code::Ok,
                    "{profile} {label}: declaration"
                );
                let row = observations
                    .lines()
                    .nth(index + 1)
                    .unwrap()
                    .split('\t')
                    .collect::<Vec<_>>();
                assert_eq!(row[0].parse::<usize>().unwrap(), index);
                let code = interp.eval_str(b"p");
                assert!(
                    !interp.host_refusal_pending(),
                    "{profile} {label}: {:?}",
                    interp.native_access_refusal()
                );
                assert_eq!(
                    code.as_int(),
                    row[1].parse::<i64>().unwrap(),
                    "{profile} {label}: original completion"
                );
                assert_eq!(
                    interp.result_bytes(),
                    bytes(row[2]),
                    "{profile} {label}: original result"
                );
                if matches!(index,19..=24|34..=39) {
                    let procedure = interp.proc_def(b"p").unwrap();
                    let artifact = cache(procedure.body.as_ptr()).unwrap_or_else(|| {
                        panic!(
                            "{profile} {label} case {index}: genuine original compiled try artifact"
                        )
                    });
                    assert!(
                        artifact.scripts.values().any(|script| script
                            .commands
                            .iter()
                            .any(|command| matches!(command.operation, Operation::Try(_)))),
                        "{profile} {label}: compiled try instruction"
                    );
                }
            }
        }
    }
    #[test]
    fn original_compiled_try_trap_checks_frame_before_options_backing_adoption() {
        // naming.embedding.original-host-publication-and-fact-transport
        // docs/design/analysis/name-resolution-proofs/embedding-original-host-publication-and-fact-transport.md
        use crate::cmd_control::original_list_conversion_tests as fixture;
        use std::cell::Cell;
        for mode in [0, 1, 2, 3] {
            let mut interp = fixture::native("tcl9.0");
            assert_eq!(
                interp.eval_str(b"proc p {} {try {error BODY {} A} trap A {} {return HANDLER}}"),
                Code::Ok
            );
            assert_eq!(interp.eval_str(b"p"), Code::Ok);
            assert_eq!(interp.result_bytes(), b"HANDLER");
            let procedure = interp.proc_def(b"p").unwrap();
            let artifact = cache(procedure.body.as_ptr()).expect("real original compiled Try");
            assert!(artifact
                .scripts
                .values()
                .flat_map(|script| &script.commands)
                .any(|command| matches!(command.operation, Operation::Try(_))));
            let _current = fixture::install(&mut interp, mode);
            interp.frames.borrow_mut().push(crate::namespace::GLOBAL);
            let original = BodyObjectOperation::capture(&interp).unwrap();
            interp.set_result_bytes(b"PRIOR");
            let prior = interp.result_obj();
            let options = fixture::original(b"-errorcode A");
            let raw = options.as_ptr();
            let completion = BodyProtectedCompletion {
                code: Code::Error,
                storage: ProtectedStorage::Stack {
                    result: obj::Owned::retain(prior),
                    options,
                },
                original: original.clone(),
            };
            let reached = interp.try_matches(
                &original,
                &artifact,
                &NativeTryCondition::Trap(vec![b"A".to_vec()]),
                &completion,
            );
            if mode == 0 {
                assert!(reached.unwrap());
                assert!(core::ptr::eq(
                    obj::obj_type_ptr(raw),
                    &crate::dict::TCL_DICT_TYPE
                ));
            } else {
                assert_eq!(reached.unwrap_err(), Code::Error);
                assert!(original.check(&interp).is_err());
                if mode == 1 {
                    fixture::host(&interp.native_execution_refusal().unwrap());
                }
                assert!(!core::ptr::eq(
                    obj::obj_type_ptr(raw),
                    &crate::dict::TCL_DICT_TYPE
                ));
            }
            assert_eq!(interp.result_obj(), prior);
            assert_eq!(fixture::FREES.with(Cell::get), 1);
            assert_eq!(fixture::REFS.with(Cell::get), 1);
            assert_eq!(unsafe { (*raw).ref_count }, 1);
        }
    }
}
