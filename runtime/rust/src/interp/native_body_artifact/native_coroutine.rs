// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Original coroutine stack preparation and retained List/value handoff.

use super::*;
use tcl_registry::native_coroutine_compilation::{NativeCoroutineInstruction, NativeCoroutineStep};

pub(super) struct CoroutineOperation {
    steps: Vec<CoroutineStep>,
    pub(super) prepared_words: HashMap<usize, WordInstruction>,
}

enum CoroutineStep {
    Operand(NamespaceOperand),
    CurrentNamespace,
    List(usize),
    Concat,
    Tailcall { count: usize, legacy: bool },
    TailcallList,
    Yield,
    YieldTo,
    Name,
}

impl CoroutineOperation {
    pub(super) fn compaction_hazards(
        &self,
    ) -> Vec<tcl_registry::native_compiler_pass::NativeCompilerPassHazard> {
        use tcl_registry::native_compiler_pass::NativeCompilerPassHazard as Hazard;
        self.steps
            .iter()
            .filter_map(|step| match step {
                CoroutineStep::Yield => Some(Hazard::Yield),
                CoroutineStep::YieldTo => Some(Hazard::YieldTo),
                _ => None,
            })
            .collect()
    }
}

impl Builder<'_> {
    pub(super) fn coroutine_operation(
        &mut self,
        words: &NativeCompilerWords<'_>,
        recipe: NativeCoroutineInstruction,
        depth: u32,
    ) -> Result<CoroutineOperation, ValueError> {
        let mut prepared_words = HashMap::new();
        let mut steps = Vec::with_capacity(recipe.steps.len());
        for step in recipe.steps {
            steps.push(match step {
                NativeCoroutineStep::Word(operand) => CoroutineStep::Operand(
                    self.namespace_operand(words, &operand, &mut prepared_words, depth)?,
                ),
                NativeCoroutineStep::CommandWord { bytes, .. } => CoroutineStep::Operand(
                    NamespaceOperand::Literal(self.selected_command_literal(&bytes)?),
                ),
                NativeCoroutineStep::Empty => CoroutineStep::Operand(NamespaceOperand::Literal(
                    self.literals.intern_bytes(b""),
                )),
                NativeCoroutineStep::CurrentNamespace => CoroutineStep::CurrentNamespace,
                NativeCoroutineStep::List(count) => CoroutineStep::List(count),
                NativeCoroutineStep::Concat => CoroutineStep::Concat,
                NativeCoroutineStep::Tailcall { count, legacy } => {
                    CoroutineStep::Tailcall { count, legacy }
                }
                NativeCoroutineStep::TailcallList => CoroutineStep::TailcallList,
                NativeCoroutineStep::Yield => CoroutineStep::Yield,
                NativeCoroutineStep::YieldTo => CoroutineStep::YieldTo,
                NativeCoroutineStep::Name => CoroutineStep::Name,
            });
        }
        Ok(CoroutineOperation {
            steps,
            prepared_words,
        })
    }
}

impl Interp {
    pub(super) fn execute_body_coroutine(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        coroutine: &CoroutineOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let mut stack = Vec::<obj::Owned>::new();
        for step in &coroutine.steps {
            match step {
                CoroutineStep::Operand(operand) => {
                    stack.push(self.body_namespace_operand(artifact, command, operand, execution)?);
                    if execution.done {
                        return Ok(Code::Ok);
                    }
                }
                CoroutineStep::CurrentNamespace => {
                    stack.push(obj::Owned::fresh(
                        tcl_cmd_core::namespace::current_original(self)
                            .map_err(|error| self.report_cmd_error(error))?,
                    ));
                }
                CoroutineStep::List(count) => {
                    let values = stack.split_off(
                        stack
                            .len()
                            .checked_sub(*count)
                            .expect("native coroutine List stack"),
                    );
                    let pointers = values.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                    stack.push(obj::Owned::fresh(crate::list::new_list_obj_native(
                        &pointers,
                        artifact.stamp.source_protocol,
                    )));
                }
                CoroutineStep::Concat => {
                    let source = stack.pop().expect("native coroutine List source");
                    let target = stack.pop().expect("native coroutine List target");
                    stack.push(
                        crate::list::concatenate_native_lists(
                            target.as_ptr(),
                            source.as_ptr(),
                            artifact.stamp.source_protocol,
                        )
                        .map_err(|error| self.report_cmd_error(error.into()))?,
                    );
                }
                CoroutineStep::Tailcall { count, legacy } => {
                    if !self.in_proc() {
                        return Ok(self
                            .error(b"tailcall can only be called from a proc, lambda or method"));
                    }
                    let mut values = stack.split_off(
                        stack
                            .len()
                            .checked_sub(*count)
                            .expect("native tailcall stack"),
                    );
                    if *legacy {
                        values[0] = obj::Owned::fresh(
                            tcl_cmd_core::namespace::current_original(self)
                                .map_err(|error| self.report_cmd_error(error))?,
                        );
                    }
                    let pointers = values.iter().map(obj::Owned::as_ptr).collect::<Vec<_>>();
                    let original = obj::Owned::fresh(crate::list::new_list_obj_native(
                        &pointers,
                        artifact.stamp.source_protocol,
                    ));
                    return Ok(self.schedule_original_tailcall_list(original));
                }
                CoroutineStep::TailcallList => {
                    return Ok(self.schedule_original_tailcall_list(
                        stack.pop().expect("native tailcall List"),
                    ));
                }
                CoroutineStep::Yield => {
                    return Ok(crate::cmd_coro::yield_original(
                        self,
                        stack.pop().expect("native yielded value"),
                    ));
                }
                CoroutineStep::Name => {
                    self.set_result_bytes(&crate::cmd_coro::current_coroutine());
                    return Ok(Code::Ok);
                }
                CoroutineStep::YieldTo => {
                    return Ok(crate::cmd_coro::yieldto_original(
                        self,
                        stack.pop().expect("native yielded invocation List"),
                    ));
                }
            }
        }
        unreachable!("native coroutine recipe has a terminal instruction")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    thread_local! {
        static RETIRING_FRAMES: std::cell::Cell<*const std::cell::RefCell<crate::frame::FrameStack>> = const { std::cell::Cell::new(std::ptr::null()) };
        static RETIRED_WITHOUT_FRAME_BORROW: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    extern "C" fn observe_tailcall_retirement(_original: *mut TclObj) {
        RETIRING_FRAMES.with(|frames| {
            let pointer = frames.get();
            if !pointer.is_null() {
                // The fixture owns this frame cell throughout the synchronous free callback.
                let available = unsafe { (*pointer).try_borrow_mut().is_ok() };
                RETIRED_WITHOUT_FRAME_BORROW.with(|result| result.set(available));
            }
        });
    }

    static RETIREMENT_TYPE: obj::TclObjType = obj::TclObjType {
        name: c"tailcall-retirement-control".as_ptr(),
        free_int_rep_proc: Some(observe_tailcall_retirement),
        dup_int_rep_proc: None,
        update_string_proc: None,
        set_from_any_proc: None,
    };

    #[test]
    fn tailcall_cancellation_releases_original_headers_before_continuation() {
        for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = super::super::tests::interpreter(profile);
            interp.frames.borrow_mut().push(GLOBAL);
            let namespace =
                obj::Owned::fresh(tcl_cmd_core::namespace::current_original(&mut interp).unwrap());
            let target = obj::Owned::fresh(obj::new_string_bytes(b"unentered_target"));
            let payload = obj::Owned::fresh(obj::alloc_typed(&RETIREMENT_TYPE, 0));
            let original = obj::Owned::fresh(crate::list::new_list_obj(&[
                namespace.as_ptr(),
                target.as_ptr(),
                payload.as_ptr(),
            ]));
            assert_eq!(
                interp.schedule_original_tailcall_list(original),
                Code::Return
            );
            RETIRING_FRAMES.with(|frames| frames.set(std::ptr::from_ref(&interp.frames)));
            RETIRED_WITHOUT_FRAME_BORROW.with(|result| result.set(false));
            drop(payload);
            assert_eq!(interp.schedule_tailcall(&[]), Code::Return);
            assert!(
                RETIRED_WITHOUT_FRAME_BORROW.with(std::cell::Cell::get),
                "{profile}: original free hook must run outside the frame borrow"
            );
            assert!(interp.frames.borrow_mut().take_tailcall().is_none());
            RETIRING_FRAMES.with(|frames| frames.set(std::ptr::null()));

            let cancellation = obj::Owned::fresh(crate::list::new_list_obj(&[namespace.as_ptr()]));
            assert_eq!(unsafe { (*namespace.as_ptr()).ref_count }, 2);
            assert_eq!(
                interp.schedule_original_tailcall_list(cancellation),
                Code::Return
            );
            assert_eq!(
                unsafe { (*namespace.as_ptr()).ref_count },
                1,
                "{profile}: namespace-only opcode operand must retire now"
            );
            assert!(interp.frames.borrow_mut().take_tailcall().is_none());
        }
    }

    fn unhex(hex: &str) -> Vec<u8> {
        hex.as_bytes()
            .chunks_exact(2)
            .map(|pair| u8::from_str_radix(std::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    #[test]
    fn original_coroutine_artifacts_match_native_namespace_and_completion_controls() {
        let mut compared = 0;
        for (profile, table) in [
            (
                "tcl8.6",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_coroutine_compilation/runtime-8.6.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_coroutine_compilation/runtime-9.0.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_coroutine_compilation/runtime-9.1.tsv"
                ),
            ),
        ] {
            for row in table.lines() {
                let fields = row.split('\t').collect::<Vec<_>>();
                let mut interp = super::super::tests::interpreter(profile);
                let code = interp.eval_str(&unhex(fields[1]));
                assert_eq!(
                    code.as_int().to_string(),
                    fields[2],
                    "{profile}/{}",
                    fields[0]
                );
                assert_eq!(
                    interp.result_bytes(),
                    unhex(fields[3]),
                    "{profile}/{}",
                    fields[0]
                );
                // An error during relay can leave a genuinely suspended coroutine.
                // Delete only this test's command, retaining normal unwind callbacks.
                interp.eval_str(b"catch {rename c {}}");
                compared += 1;
            }
        }
        assert_eq!(compared, 39);
    }

    #[test]
    fn original_coroutine_channel_retains_yield_resume_and_creation_headers() {
        for profile in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = super::super::tests::interpreter(profile);
            assert_eq!(
                interp.eval_str(
                    b"proc generator value {set resumed [yield $value]; return $resumed}"
                ),
                Code::Ok
            );
            let payload = obj::Owned::fresh(crate::list::new_list_obj(&[
                obj::new_string_bytes(b"A"),
                obj::new_string_bytes(b"B"),
            ]));
            let head = obj::Owned::fresh(obj::new_string_bytes(b"coroutine"));
            let name = obj::Owned::fresh(obj::new_string_bytes(b"c"));
            let target = obj::Owned::fresh(obj::new_string_bytes(b"generator"));
            assert_eq!(
                interp.dispatch(&[
                    head.as_ptr(),
                    name.as_ptr(),
                    target.as_ptr(),
                    payload.as_ptr()
                ]),
                Code::Ok,
                "{profile}"
            );
            assert_eq!(
                interp.result_obj(),
                payload.as_ptr(),
                "{profile}: original creation argv/yield"
            );
            let resume = obj::Owned::fresh(crate::list::new_list_obj(&[
                obj::new_string_bytes(b"C"),
                obj::new_string_bytes(b"D"),
            ]));
            assert_eq!(
                interp.dispatch(&[name.as_ptr(), resume.as_ptr()]),
                Code::Ok,
                "{profile}"
            );
            assert_eq!(
                interp.result_obj(),
                resume.as_ptr(),
                "{profile}: original resume/return"
            );
        }
    }
}
