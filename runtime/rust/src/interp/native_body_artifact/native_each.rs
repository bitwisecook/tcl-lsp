// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original compiled foreach/lmap auxiliary groups and iterator ownership.

use super::*;
use tcl_registry::native_control_compilation::{NativeControlCompilation, NativeControlOutcome};
use tcl_registry::native_each_compilation::{
    NativeCompiledEachStorage as Storage, NativeEachCollection, NativeEachInstruction,
    native_compiled_each_storage,
};

pub(super) struct EachOperation {
    recipe: NativeEachInstruction,
    pub(super) prepared: PreparedControlOperands,
    slots: Vec<Vec<usize>>,
    empty: Option<usize>,
}

impl Builder<'_> {
    pub(super) fn each_operation(
        &mut self,
        captured: &NativeCompilerWords<'_>,
        selected: NativeControlCompilation<NativeEachInstruction>,
        depth: u32,
    ) -> Result<Operation, ValueError> {
        let prepared = self.prepare_control_steps(captured, &selected.preparations, depth)?;
        let recipe = match selected.outcome {
            NativeControlOutcome::Generic => return Ok(Operation::Invoke),
            NativeControlOutcome::Rejected(failure) => {
                return Err(self.reject_native_compilation(failure));
            }
            NativeControlOutcome::Inline(recipe) => recipe,
        };
        let protocol = self
            .interp
            .native_invocation_dialect()
            .native_compiled_variable_protocol()
            .ok_or_else(|| unavailable("native iterator local issuer"))?;
        let slots = recipe
            .groups
            .iter()
            .map(|group| {
                group
                    .variables
                    .iter()
                    .map(|name| {
                        self.lvt
                            .find_native(protocol, name)
                            .ok_or_else(|| unavailable("native iterator auxiliary local"))
                    })
                    .collect::<Result<Vec<_>, ValueError>>()
            })
            .collect::<Result<Vec<_>, ValueError>>()?;
        let empty = (recipe.collection == NativeEachCollection::Foreach)
            .then(|| self.literals.intern_bytes(b""));
        Ok(Operation::Each(EachOperation {
            recipe,
            prepared,
            slots,
            empty,
        }))
    }
}

// TclListObjCopy(NULL) returns an rc-0 working header in C8.5 STEP4.
// Its backing owns the original members until the group finishes.
struct GroupListCopy(*mut TclObj);
impl GroupListCopy {
    fn new(original: *mut TclObj, protocol: NativeStringProtocol) -> Result<Self, ValueError> {
        Ok(Self(
            crate::list::native_list_copy(original, protocol)?.into_native_unowned(),
        ))
    }
}
impl Drop for GroupListCopy {
    fn drop(&mut self) {
        // SAFETY: this reached working header stays live for the whole group.
        if unsafe { (*self.0).ref_count == 0 } {
            drop_fresh(self.0);
        } else {
            // SAFETY: a callback-retained working header has a native reference.
            unsafe { obj::decr_ref_count(self.0) };
        }
    }
}

struct StackGroup {
    original: obj::Owned,
    length: usize,
}

#[derive(Default)]
struct StackGroups(Vec<StackGroup>);
impl core::ops::Deref for StackGroups {
    type Target = Vec<StackGroup>;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
impl core::ops::DerefMut for StackGroups {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
impl Drop for StackGroups {
    fn drop(&mut self) {
        while let Some(group) = self.0.pop() {
            drop(group);
        }
    }
}

struct StackIteration {
    _auxiliary: obj::Owned,
    counts: obj::Owned,
    maximum: usize,
}
impl StackIteration {
    fn new(recipe: &NativeEachInstruction, maximum: usize) -> Self {
        let counts = obj::Owned::fresh(obj::new_string_bytes(b""));
        let auxiliary = obj::Owned::fresh(obj::new_string_bytes(b""));
        obj::change_type(
            auxiliary.as_ptr(),
            core::ptr::null(),
            core::ptr::from_ref(recipe) as usize as u64,
        );
        Self {
            _auxiliary: auxiliary,
            counts,
            maximum,
        }
    }
    fn advance(&self, iteration: usize) {
        obj::change_type(self.counts.as_ptr(), core::ptr::null(), iteration as u64);
    }
}

impl Interp {
    fn each_temporary(&mut self, slot: usize) -> Result<*mut TclObj, Code> {
        let value = self
            .frames
            .borrow()
            .native_compiled_temporary(slot)
            .ok()
            .flatten();
        value.ok_or_else(|| {
            self.report_cmd_error(unavailable("native iterator actual temporary value").into())
        })
    }

    fn each_store_temporary(&mut self, slot: usize, value: *mut TclObj) -> Result<(), Code> {
        let stored = self
            .frames
            .borrow_mut()
            .store_native_compiled_temporary(slot, value);
        stored.map_err(|_| {
            self.report_cmd_error(unavailable("native iterator actual temporary cell").into())
        })
    }

    fn each_counter(&mut self, slot: usize, iteration: i64) -> Result<(), Code> {
        let previous = self
            .frames
            .borrow()
            .native_compiled_temporary(slot)
            .ok()
            .flatten();
        if let Some(original) = previous {
            obj::change_type(original, &obj::TCL_INT_TYPE, iteration as u64);
            obj::invalidate_string(original);
            Ok(())
        } else {
            let value = obj::Owned::fresh(obj::new_wide_int_obj(iteration));
            self.each_store_temporary(slot, value.as_ptr())
        }
    }

    fn each_list_length(&mut self, original: *mut TclObj) -> Result<usize, Code> {
        ValueOps::list_len(self, &original).map_err(|error| self.report_cmd_error(error.into()))
    }

    fn each_assign(&mut self, name: &[u8], slot: usize, original: *mut TclObj) -> Result<(), Code> {
        let target = EvaluatedTarget {
            root: name.to_vec(),
            element: None,
            original_name: None,
            original_index: None,
            combined: false,
        };
        let captured = self.body_capture_target(&target, Some(slot), true)?;
        let traced = self.original_variable_trace_requires_name(&captured.home, None, b"write");
        let temporary = (traced
            && self.native_invocation_dialect().tcl_version == Some(tcl_dialect::TclVersion::V8_4))
        .then(|| obj::Owned::retain(original));
        captured
            .receiver
            .store(original)
            .map_err(|error| crate::builtins::var_error(self, name, error))?;
        if traced {
            let access = self.trace_access(name, name, None, &captured.home, false);
            if self.fire_var_trace_resolved(&captured.home, &access, b"write") {
                return Err(crate::builtins::var_error(
                    self,
                    name,
                    crate::frame::VarError::TraceError,
                ));
            }
        }
        drop(temporary);
        if self.host_refusal_pending() {
            return Err(Code::Error);
        }
        Ok(())
    }

    pub(super) fn execute_body_each(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        operation: &EachOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let storage = native_compiled_each_storage(operation.recipe.version);
        let protocol = artifact.stamp.source_protocol;
        let collected = (operation.recipe.collection == NativeEachCollection::Lmap)
            .then(|| obj::Owned::fresh(crate::list::new_list_obj_native(&[], protocol)));
        let mut stack_groups = StackGroups::default();
        // Every value expression is outside the body exception range.
        for (index, group) in operation.recipe.groups.iter().enumerate() {
            let original = self.body_control_operand(
                artifact,
                command,
                &operation.prepared,
                &group.values,
                execution,
            )?;
            if execution.done {
                return Ok(Code::Return);
            }
            if storage == Storage::StackLists {
                stack_groups.push(StackGroup {
                    original,
                    length: 0,
                });
            } else {
                self.each_store_temporary(
                    operation.prepared.temporaries[index],
                    original.as_ptr(),
                )?;
            }
        }
        let mut maximum = 0;
        if storage == Storage::StackLists {
            for (group, source) in operation.recipe.groups.iter().zip(stack_groups.iter_mut()) {
                source.length = self.each_list_length(source.original.as_ptr())?;
                if obj::is_shared(source.original.as_ptr()) {
                    source.original =
                        crate::list::native_list_copy(source.original.as_ptr(), protocol)
                            .map_err(|error| self.report_cmd_error(error.into()))?;
                }
                maximum = maximum.max(source.length.div_ceil(group.variables.len()));
            }
        }
        let counter = if storage == Storage::StackLists {
            None
        } else {
            let slot = *operation.prepared.temporaries.last().ok_or_else(|| {
                self.report_cmd_error(unavailable("native iterator counter local").into())
            })?;
            self.each_counter(slot, -1)?;
            Some(slot)
        };
        let stack_iteration = (storage == Storage::StackLists)
            .then(|| StackIteration::new(&operation.recipe, maximum));
        let mut iteration = 0usize;
        loop {
            if let Some(counter) = counter {
                let value = i64::try_from(iteration).map_err(|_| {
                    self.report_cmd_error(unavailable("native iterator counter extent").into())
                })?;
                self.each_counter(counter, value)?;
                let mut continues = false;
                for (index, group) in operation.recipe.groups.iter().enumerate() {
                    let original = self.each_temporary(operation.prepared.temporaries[index])?;
                    let length = self.each_list_length(original)?;
                    continues |= iteration < length.div_ceil(group.variables.len());
                }
                if !continues {
                    break;
                }
            } else if let Some(cursor) = &stack_iteration {
                if iteration >= cursor.maximum {
                    break;
                }
                cursor.advance(iteration + 1);
            }
            for (index, group) in operation.recipe.groups.iter().enumerate() {
                let original = if storage == Storage::StackLists {
                    stack_groups[index].original.as_ptr()
                } else {
                    self.each_temporary(operation.prepared.temporaries[index])?
                };
                let copy = if storage == Storage::LocalGroupCopy {
                    Some(
                        GroupListCopy::new(original, protocol)
                            .map_err(|error| self.report_cmd_error(error.into()))?,
                    )
                } else {
                    None
                };
                let selected = copy.as_ref().map_or(original, |copy| copy.0);
                let members = if storage == Storage::LocalRefetch {
                    None
                } else {
                    Some(
                        crate::list::list_elements_native_checked(selected, protocol)
                            .map_err(|error| self.report_cmd_error(error.into()))?,
                    )
                };
                let first_value_index =
                    iteration
                        .checked_mul(group.variables.len())
                        .ok_or_else(|| {
                            self.report_cmd_error(
                                unavailable("native iterator value extent").into(),
                            )
                        })?;
                for (value_index, (variable, name)) in
                    (first_value_index..).zip(group.variables.iter().enumerate())
                {
                    let refreshed;
                    let elements = if let Some(elements) = &members {
                        elements
                    } else {
                        refreshed = crate::list::list_elements_native_checked(selected, protocol)
                            .map_err(|_| {
                            self.report_cmd_error(
                                unavailable("native C8.4 compiled iterator fatal refetch").into(),
                            )
                        })?;
                        &refreshed
                    };
                    let fresh = elements
                        .get(value_index)
                        .is_none()
                        .then(|| obj::new_string_bytes(b""));
                    let value = elements
                        .get(value_index)
                        .copied()
                        .or(fresh)
                        .expect("native iterator value");
                    let lifetime = fresh.map(obj::NativeObjectLifetime::retain);
                    let result = self.each_assign(name, operation.slots[index][variable], value);
                    if fresh.is_some() && obj::allocation_is_live(value) {
                        // SAFETY: the lifetime lease keeps the actual fresh header readable.
                        if unsafe { (*value).ref_count == 0 } {
                            drop_fresh(value);
                        }
                    }
                    drop(lifetime);
                    result?;
                }
            }
            let code = self.execute_body_region(artifact, operation.recipe.body_span, execution);
            if execution.done || self.host_refusal_pending() || self.exit_pending() {
                return Ok(code);
            }
            match code {
                Code::Ok => {
                    if let Some(accumulator) = &collected {
                        crate::list::append_prepared_native_elements(
                            accumulator.as_ptr(),
                            &[self.result_obj()],
                            protocol,
                        )
                        .map_err(|error| self.report_cmd_error(error.into()))?;
                    }
                }
                Code::Continue => {}
                Code::Break => break,
                other => return Ok(other),
            }
            iteration = iteration.checked_add(1).ok_or_else(|| {
                self.report_cmd_error(unavailable("native iterator iteration extent").into())
            })?;
        }
        drop(stack_iteration);
        drop(stack_groups);
        if let Some(collected) = collected {
            self.set_result(collected.as_ptr());
        } else {
            self.set_result(
                artifact
                    .literals
                    .original(operation.empty.expect("native foreach empty result"))
                    .expect("native foreach registered result"),
            );
        }
        Ok(Code::Ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    mod original_sources {
        include!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../rust/tcl-registry/tests/data/native_each_try_compilation/cases.rs"
        ));
    }

    fn decode(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    #[test]
    fn original_each_artifacts_match_all_95_native_execution_and_layout_windows() {
        for (engine, expected) in [
            (
                "tcl8.4",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_each_try_compilation/8.4.20.tsv"
                ),
            ),
            (
                "tcl8.5",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_each_try_compilation/8.5.19.tsv"
                ),
            ),
            (
                "tcl8.6",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_each_try_compilation/8.6.18.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_each_try_compilation/9.0.4.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_each_try_compilation/9.1.0.tsv"
                ),
            ),
        ] {
            for fields in expected
                .lines()
                .skip(1)
                .take(19)
                .map(|row| row.split('\t').collect::<Vec<_>>())
            {
                crate::counters::reset();
                {
                    let case: usize = fields[0].parse().unwrap();
                    let (name, source) = original_sources::CASES[case];
                    let mut interp = super::super::tests::interpreter(engine);
                    let body = obj::Owned::fresh(obj::new_string_bytes(source));
                    interp.define_proc(b"p", Vec::new(), body.as_ptr());
                    let code = interp.eval_str(b"p");
                    assert!(
                        !interp.host_refusal_pending(),
                        "{engine}/{name}: {:?}",
                        interp.result_bytes()
                    );
                    assert_eq!(
                        code.as_int(),
                        fields[1].parse::<i64>().unwrap(),
                        "{engine}/{name}"
                    );
                    assert_eq!(interp.result_bytes(), decode(fields[2]), "{engine}/{name}");
                    let compiled_count = fields[4]
                        .split(',')
                        .filter(|instruction| {
                            instruction.ends_with(":foreach_start")
                                || instruction.ends_with(":foreach_start4")
                        })
                        .count();
                    if let Some(artifact) = cache(body.as_ptr()) {
                        let actual_count = artifact
                            .scripts
                            .values()
                            .flat_map(|script| &script.commands)
                            .filter(|command| matches!(command.operation, Operation::Each(_)))
                            .count();
                        assert_eq!(
                            actual_count, compiled_count,
                            "{engine}/{name}: genuine inline iterator"
                        );
                        let actual = artifact
                            .compiled_local_layout()
                            .unwrap()
                            .names
                            .iter()
                            .map(|name| {
                                name.as_ref()
                                    .map_or_else(Vec::new, |name| name.as_bytes().to_vec())
                            })
                            .collect::<Vec<_>>();
                        let expected = if fields[3].is_empty() {
                            Vec::new()
                        } else {
                            fields[3].split(',').map(decode).collect()
                        };
                        assert_eq!(
                            actual, expected,
                            "{engine}/{name}: actual named/anonymous local order"
                        );
                    } else {
                        assert_eq!(
                            compiled_count, 0,
                            "{engine}/{name}: compiled source must retain original artifact"
                        );
                    }
                }
                assert_eq!(
                    crate::counters::finalize(),
                    0,
                    "{engine}: native original owner leak"
                );
                assert_eq!(
                    crate::counters::double_free_count(),
                    0,
                    "{engine}: native original owner release"
                );
            }
        }
    }
}
