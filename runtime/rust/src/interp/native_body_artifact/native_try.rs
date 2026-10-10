// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original compiled try ranges, their anonymous cells, and RETURN_STK cleanup.
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
    ) -> Result<Operation, ValueError> {
        let prepared = self.prepare_control_steps(captured, &selected.preparations, depth)?;
        let recipe = match selected.outcome {
            NativeControlOutcome::Inline(recipe) => recipe,
            NativeControlOutcome::Generic => return Ok(Operation::Invoke),
            NativeControlOutcome::Rejected(failure) => {
                return Err(self.reject_native_compilation(failure));
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
    },
}
impl BodyProtectedCompletion {
    fn originals(&self, interp: &mut Interp) -> Result<(*mut TclObj, *mut TclObj), Code> {
        match &self.storage {
            ProtectedStorage::Stack { result, options } => Ok((result.as_ptr(), options.as_ptr())),
            ProtectedStorage::Locals {
                activation,
                result,
                options,
            } => {
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
                originals.ok_or_else(|| {
                    interp.report_cmd_error(
                        unavailable("native protected completion frame lifetime").into(),
                    )
                })
            }
        }
    }
    fn store(self, interp: &mut Interp, slots: &[usize]) -> Result<Self, Code> {
        if slots.is_empty() {
            return Ok(self);
        }
        let [result_slot, options_slot, ..] = slots else {
            return Err(
                interp.report_cmd_error(unavailable("native try anonymous slot geometry").into())
            );
        };
        let (result, options) = match self.storage {
            ProtectedStorage::Stack { result, options } => (result, options),
            storage @ ProtectedStorage::Locals { .. } => {
                return Ok(Self {
                    code: self.code,
                    storage,
                });
            }
        };
        let stored = {
            let mut frames = interp.frames.borrow_mut();
            frames
                .store_native_compiled_temporary(*options_slot, options.as_ptr())
                .and_then(|()| {
                    frames.store_native_compiled_temporary(*result_slot, result.as_ptr())
                })
                .map(|()| frames.current_activation())
        };
        let activation = stored.map_err(|_| {
            interp.report_cmd_error(unavailable("native try anonymous completion cells").into())
        })?;
        Ok(Self {
            code: self.code,
            storage: ProtectedStorage::Locals {
                activation,
                result: *result_slot,
                options: *options_slot,
            },
        })
    }
}
impl Interp {
    /// PUSH_RESULT and PUSH_RETURN_OPTIONS retain the exact native originals.
    pub(super) fn capture_body_completion(
        &mut self,
        code: Code,
    ) -> Result<BodyProtectedCompletion, Code> {
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
        })
    }

    /// RETURN_STK merges the original options operand independently from a public
    /// command binding, then publishes the same retained result header.
    pub(super) fn reapply_body_completion(
        &mut self,
        completion: &BodyProtectedCompletion,
    ) -> Result<Code, Code> {
        let (result, options) = completion.originals(self)?;
        let result = obj::Owned::retain(result);
        let options = obj::Owned::retain(options);
        let (mut ops, protocol) = crate::return_options::NativeReturnOps::selected(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let merged = tcl_cmd_core::native_return_merge::merge_stack(&mut ops, protocol, &options)
            .map_err(|error| self.report_cmd_error(error))?;
        let code = self
            .process_original_c_return_options(
                tcl_registry::native_return_options::NativeReturnOptionsApplication::Immediate,
                merged.code,
                i64::from(merged.level),
                merged.options.as_ptr(),
            )
            .map_err(|error| self.report_cmd_error(error.into()))?;
        self.set_result(result.as_ptr());
        Ok(code)
    }

    fn splice_try_during(
        &mut self,
        artifact: &NativeBodyArtifact,
        operation: &TryOperation,
        previous: &BodyProtectedCompletion,
        replacement: BodyProtectedCompletion,
    ) -> Result<BodyProtectedCompletion, Code> {
        if replacement.code != Code::Error {
            return Ok(replacement);
        }
        let (_, prior_options) = previous.originals(self)?;
        let ProtectedStorage::Stack { result, options } = replacement.storage else {
            return Err(
                self.report_cmd_error(unavailable("native try new exception stack roles").into())
            );
        };
        let key = artifact
            .literals
            .original(operation.during.expect("native try cleanup key"))
            .expect("native -during literal");
        let mut dictionary = crate::dict::PreparedNativeDictionary::prepare(
            Some(options.as_ptr()),
            artifact.stamp.source_protocol,
        )
        .map_err(|error| self.report_cmd_error(error.into()))?;
        drop(options);
        dictionary
            .set_member(key, prior_options)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        Ok(BodyProtectedCompletion {
            code: Code::Error,
            storage: ProtectedStorage::Stack {
                result,
                options: dictionary.into_value(),
            },
        })
    }

    fn bind_try_local(&mut self, name: &[u8], slot: usize, value: *mut TclObj) -> Code {
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
        if self.has_variable_traces() {
            let access = self.trace_access(name, name, None, &home, false);
            if self.fire_var_trace_resolved(&home, &access, b"write") {
                return crate::builtins::var_error(self, name, crate::frame::VarError::TraceError);
            }
        }
        if self.host_refusal_pending() {
            Code::Error
        } else {
            Code::Ok
        }
    }

    fn try_matches(
        &mut self,
        artifact: &NativeBodyArtifact,
        condition: &NativeTryCondition,
        completion: &BodyProtectedCompletion,
    ) -> Result<bool, Code> {
        if i64::from(condition.code()) != completion.code.as_int() {
            return Ok(false);
        }
        let NativeTryCondition::Trap(pattern) = condition else {
            return Ok(true);
        };
        let (_, options) = completion.originals(self)?;
        let pairs = crate::dict::native_dict_pairs(options, artifact.stamp.source_protocol)
            .map_err(|error| self.report_cmd_error(error.into()))?;
        let mut code = None;
        for (key, value) in pairs {
            if crate::dict::native_object_bytes(key, artifact.stamp.source_protocol)
                .map_err(|error| self.report_cmd_error(error.into()))?
                == b"-errorcode"
            {
                code = Some(value);
                break;
            }
        }
        let code = code.ok_or_else(|| {
            self.report_cmd_error(unavailable("native try trapped error code").into())
        })?;
        let members =
            crate::list::list_elements_native_checked(code, artifact.stamp.source_protocol)
                .map_err(|error| self.report_cmd_error(error.into()))?;
        if members.len() < pattern.len() {
            return Ok(false);
        }
        for (member, expected) in members.iter().zip(pattern) {
            if crate::dict::native_object_bytes(*member, artifact.stamp.source_protocol)
                .map_err(|error| self.report_cmd_error(error.into()))?
                != *expected
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    pub(super) fn execute_body_try(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        operation: &TryOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let recipe = &operation.recipe;
        let body_code = self.execute_control_body(
            artifact,
            command,
            &operation.prepared,
            &recipe.body,
            execution,
        );
        if execution.done || self.host_refusal_pending() || self.exit_pending() {
            return Ok(body_code);
        }
        if recipe.handlers.is_empty() && recipe.finally.is_none() {
            return Ok(body_code);
        }
        if body_code == Code::Ok && !recipe.catches_success && recipe.finally.is_none() {
            return Ok(Code::Ok);
        }
        let initial = if let (Code::Ok, Some(options)) = (body_code, operation.success_options) {
            BodyProtectedCompletion {
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
            self.capture_body_completion(body_code)?
        };
        let mut outcome = initial.store(self, &operation.prepared.temporaries)?;
        let mut matched = None;
        for (index, handler) in recipe.handlers.iter().enumerate() {
            if self.try_matches(artifact, &handler.condition, &outcome)? {
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
                code = self.bind_try_local(handler.result.as_ref().unwrap(), slot, result);
            }
            if code == Code::Ok {
                if let Some(slot) = options_slot {
                    code = self.bind_try_local(handler.options.as_ref().unwrap(), slot, options);
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
            if execution.done || self.host_refusal_pending() || self.exit_pending() {
                return Ok(code);
            }
            if code == Code::Ok && recipe.finally.is_none() {
                return Ok(Code::Ok);
            }
            let replacement = self.capture_body_completion(code)?;
            let replacement = self.splice_try_during(artifact, operation, &outcome, replacement)?;
            if recipe.finally.is_none() {
                return self.reapply_body_completion(&replacement);
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
            if execution.done || self.host_refusal_pending() || self.exit_pending() {
                return Ok(code);
            }
            if code != Code::Ok {
                let replacement = self.capture_body_completion(code)?;
                let replacement =
                    self.splice_try_during(artifact, operation, &outcome, replacement)?;
                outcome = replacement.store(self, &operation.prepared.temporaries)?;
            }
        }
        self.reapply_body_completion(&outcome)
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
}
