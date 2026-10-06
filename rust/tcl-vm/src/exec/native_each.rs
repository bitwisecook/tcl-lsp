// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original compiled foreach auxiliary storage and callback-safe iteration.

use super::{ForeachState, Frame, Tick, Value, Vm};
use tcl_bytecode::{CompiledVariableTarget, NativeEachAuxiliary};
use tcl_registry::native_each_compilation::{
    NativeCompiledEachStorage, native_compiled_each_storage,
};
use tcl_syntax::native_string::NativeStringProtocol;

impl Vm {
    pub(super) fn native_compiled_each_start(
        &mut self,
        frame: &mut Frame,
        auxiliary: &std::sync::Arc<NativeEachAuxiliary>,
        collect: bool,
    ) -> Tick {
        let actual = self
            .actual_native_invocation_dialect()
            .native_string_protocol();
        if actual != Some(NativeStringProtocol::C(auxiliary.version)) {
            return Tick::Return(
                self.refuse_host_command("foreign compiled foreach issuer".into()),
            );
        }
        let strings = actual.expect("checked foreach issuer");
        let storage = native_compiled_each_storage(auxiliary.version);
        let groups = auxiliary
            .variables
            .iter()
            .map(|group| {
                group
                    .iter()
                    .copied()
                    .map(CompiledVariableTarget::Slot)
                    .collect()
            })
            .collect::<Vec<Vec<_>>>();
        if groups.iter().any(Vec::is_empty) {
            return Tick::Return(
                self.refuse_host_command("empty compiled foreach auxiliary group".into()),
            );
        }
        let mut roots = Vec::new();
        let mut lists = Vec::new();
        let mut iter_max = 0;
        if storage == NativeCompiledEachStorage::StackLists {
            if !auxiliary.temporaries.is_empty() || frame.stack.len() < groups.len() {
                return Tick::Return(
                    self.refuse_host_command("compiled foreach stack geometry".into()),
                );
            }
            roots = frame.stack.split_off(frame.stack.len() - groups.len());
            for (index, original) in roots.iter_mut().enumerate() {
                let prepared = self.native_object_list_elements_in(original, strings);
                if let Err(error) = prepared {
                    return Tick::Return(crate::command::completion_from_cmd_error(
                        self,
                        error.into(),
                    ));
                }
                if original.native_object_is_shared() {
                    match original.native_list_copy(strings) {
                        Ok(copy) => *original = copy,
                        Err(error) => {
                            return Tick::Return(crate::command::completion_from_cmd_error(
                                self,
                                error.into(),
                            ));
                        }
                    }
                }
                let items = match self.native_object_list_elements_in(original, strings) {
                    Ok(items) => items,
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            error.into(),
                        ));
                    }
                };
                iter_max = iter_max.max(items.len().div_ceil(groups[index].len()));
                lists.push(items);
            }
        } else {
            if auxiliary.temporaries.len() != groups.len() + 1 {
                return Tick::Return(
                    self.refuse_host_command("compiled foreach temporary geometry".into()),
                );
            }
            let slot = auxiliary.temporaries[groups.len()];
            // The anonymous counter has no name, observers, or alias lookup.
            if let Err(completion) =
                self.initialise_native_compiled_counter(slot, auxiliary.version)
            {
                return Tick::Return(completion);
            }
        }
        let start = frame.pc - 1;
        let Some(&step) = frame.foreach_pairs.get(&start) else {
            return Tick::Return(self.refuse_host_command("compiled foreach step pairing".into()));
        };
        frame.last_options = Value::empty();
        let native_accumulator = if collect {
            let Some(accumulator) = frame.stack.pop() else {
                return Tick::Return(
                    self.refuse_host_command("compiled lmap accumulator is unavailable".into()),
                );
            };
            Some(accumulator)
        } else {
            None
        };
        frame.foreach_stack.push(ForeachState {
            native: Some(auxiliary.clone()),
            _native_iterator_headers: (storage == NativeCompiledEachStorage::StackLists)
                .then(|| [Value::empty(), Value::empty()]),
            native_accumulator,
            var_groups: groups,
            lists,
            _list_roots: roots,
            iter_num: 0,
            iter_max,
            body_idx: frame.pc,
            collect,
            accum: Vec::new(),
        });
        frame.pc = step;
        Tick::Continue
    }

    pub(super) fn native_compiled_each_step(&mut self, frame: &mut Frame) -> Tick {
        let state = frame
            .foreach_stack
            .last_mut()
            .expect("selected compiled foreach");
        let auxiliary = state.native.as_ref().expect("original compiled auxiliary");
        let storage = native_compiled_each_storage(auxiliary.version);
        let strings = NativeStringProtocol::C(auxiliary.version);
        let iteration = state.iter_num;
        if storage != NativeCompiledEachStorage::StackLists {
            let counter = match self
                .native_compiled_temporary_value(auxiliary.temporaries[auxiliary.variables.len()])
            {
                Ok(counter) => counter,
                Err(completion) => return Tick::Return(completion),
            };
            let Ok(count) = i64::try_from(iteration) else {
                return Tick::Return(
                    self.refuse_host_command("compiled foreach counter overflow".into()),
                );
            };
            if let Err(error) = counter
                .value()
                .set_native_loop_counter(count, auxiliary.version)
            {
                return Tick::Return(crate::command::completion_from_cmd_error(
                    self,
                    error.into(),
                ));
            }
            state.iter_max = 0;
            for (index, group) in auxiliary.variables.iter().enumerate() {
                let original =
                    match self.native_compiled_temporary_value(auxiliary.temporaries[index]) {
                        Ok(original) => original,
                        Err(completion) => return Tick::Return(completion),
                    };
                let items = match self.native_object_list_elements_in(original.value(), strings) {
                    Ok(items) => items,
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            error.into(),
                        ));
                    }
                };
                state.iter_max = state.iter_max.max(items.len().div_ceil(group.len()));
            }
        }
        if iteration >= state.iter_max {
            return Tick::Continue;
        }
        state.iter_num += 1;
        for (group_index, group) in auxiliary.variables.iter().enumerate() {
            let group_copy = if storage == NativeCompiledEachStorage::LocalGroupCopy {
                let original = match self
                    .native_compiled_temporary_value(auxiliary.temporaries[group_index])
                {
                    Ok(original) => original,
                    Err(completion) => return Tick::Return(completion),
                };
                match original.value().native_list_copy(strings) {
                    Ok(copy) => Some(copy.into_native_unowned_lifetime()),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            error.into(),
                        ));
                    }
                }
            } else {
                None
            };
            let copied_items = match &group_copy {
                Some(copy) => match self.native_object_list_elements_in(copy, strings) {
                    Ok(items) => Some(items),
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            error.into(),
                        ));
                    }
                },
                None => None,
            };
            for (member, slot) in group.iter().copied().enumerate() {
                let position = iteration * group.len() + member;
                let assigned = match storage {
                    NativeCompiledEachStorage::StackLists => {
                        state.lists[group_index].elements().map(|items| {
                            items.get(position).map_or_else(Value::empty, |value| {
                                value.native_lifetime_lease().into_value()
                            })
                        })
                    }
                    NativeCompiledEachStorage::LocalGroupCopy => copied_items
                        .as_ref()
                        .expect("original group copy")
                        .elements()
                        .map(|items| {
                            items.get(position).map_or_else(Value::empty, |value| {
                                value.native_lifetime_lease().into_value()
                            })
                        }),
                    NativeCompiledEachStorage::LocalRefetch => {
                        let original = match self
                            .native_compiled_temporary_value(auxiliary.temporaries[group_index])
                        {
                            Ok(original) => original,
                            Err(completion) => return Tick::Return(completion),
                        };
                        match self.native_object_list_elements_in(original.value(), strings) {
                            Ok(items) => items.elements().map(|items| {
                                items.get(position).map_or_else(Value::empty, |value| {
                                    value.native_lifetime_lease().into_value()
                                })
                            }),
                            Err(error) => {
                                return Tick::Return(self.refuse_host_command(format!(
                                    "native Tcl 8.4 foreach member refetch fatal boundary: {error}"
                                )));
                            }
                        }
                    }
                };
                let assigned = match assigned {
                    Ok(value) => value,
                    Err(error) => {
                        return Tick::Return(crate::command::completion_from_cmd_error(
                            self,
                            error.into(),
                        ));
                    }
                };
                if let Err(completion) = self.native_compiled_each_assign(slot, assigned, storage) {
                    return Tick::Return(completion);
                }
            }
        }
        frame.last_options = Value::empty();
        frame.pc = state.body_idx;
        Tick::Continue
    }
}

#[cfg(test)]
mod tests {
    use crate::Value;
    use tcl_core_types::Code;

    mod inputs {
        include!("../../../tcl-registry/tests/data/native_each_try_compilation/cases.rs");
    }
    fn bytes(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    #[test]
    fn original_compiled_each_matches_95_native_completions() {
        let engines = [
            (
                "tcl8.4",
                include_str!(
                    "../../../tcl-registry/tests/data/native_each_try_compilation/8.4.20.tsv"
                ),
            ),
            (
                "tcl8.5",
                include_str!(
                    "../../../tcl-registry/tests/data/native_each_try_compilation/8.5.19.tsv"
                ),
            ),
            (
                "tcl8.6",
                include_str!(
                    "../../../tcl-registry/tests/data/native_each_try_compilation/8.6.18.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../tcl-registry/tests/data/native_each_try_compilation/9.0.4.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../tcl-registry/tests/data/native_each_try_compilation/9.1.0.tsv"
                ),
            ),
        ];
        let mut compared = 0;
        for (profile, fixture) in engines {
            let profile = tcl_dialect::DialectProfile::find(profile).unwrap();
            for row in fixture.lines().skip(1).take(19) {
                let fields = row.split('\t').collect::<Vec<_>>();
                let index = fields[0].parse::<usize>().unwrap();
                let (label, original) = inputs::CASES[index];
                let mut vm = crate::native_fixture::interpreter(profile);
                let defined = vm
                    .try_invoke_command(
                        "proc",
                        &[
                            Value::new_native_string_bytes(b"p".as_slice()),
                            Value::empty(),
                            Value::new_native_string_bytes(original),
                        ],
                    )
                    .unwrap_or_else(|error| {
                        panic!("{}/{label} definition: {error:?}", profile.name)
                    });
                assert_eq!(
                    defined.code,
                    Code::Ok,
                    "{}/{label}: {defined:?}",
                    profile.name
                );
                let completion = vm.try_invoke_command("p", &[]).unwrap_or_else(|error| {
                    panic!("{}/{label} activation: {error:?}", profile.name)
                });
                assert_eq!(
                    completion.code,
                    Code::from_int(fields[1].parse().unwrap()),
                    "{}/{label}: {completion:?}",
                    profile.name
                );
                let strings = vm
                    .actual_native_invocation_dialect()
                    .native_string_protocol()
                    .unwrap();
                assert_eq!(
                    completion
                        .result
                        .native_string_bytes(strings)
                        .unwrap()
                        .as_ref(),
                    bytes(fields[2]),
                    "{}/{label}",
                    profile.name
                );
                compared += 1;
            }
        }
        assert_eq!(compared, 95);
    }
}
