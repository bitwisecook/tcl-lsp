// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original String range operand and explicit stack-result disposition.

use super::*;
use tcl_registry::native_string_range_compilation::{
    NativeStringRangeInstruction, NativeStringRangeOperation,
};

pub(super) struct StringRangeOperation {
    operand: NamespaceOperand,
    operation: NativeStringRangeOperation,
    empty_literal: Option<usize>,
    pub(super) prepared_words: HashMap<usize, WordInstruction>,
}

impl Builder<'_> {
    pub(super) fn string_range_operation(
        &mut self,
        words: &NativeCompilerWords<'_>,
        recipe: &NativeStringRangeInstruction,
        depth: u32,
    ) -> Result<StringRangeOperation, tcl_cmd_core::CmdError> {
        let mut prepared_words = HashMap::new();
        let operand = self.namespace_operand(words, &recipe.operand, &mut prepared_words, depth)?;
        let empty_literal = (recipe.operation == NativeStringRangeOperation::Empty)
            .then(|| self.literals.intern_bytes(b""));
        Ok(StringRangeOperation {
            operand,
            empty_literal,
            operation: recipe.operation,
            prepared_words,
        })
    }
}

impl Interp {
    pub(super) fn execute_body_string_range(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        recipe: &StringRangeOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let original = super::native_list_conversion::BodyObjectOperation::capture(self)
            .map_err(|error| self.report_cmd_error(error))?;
        let actual = self.clone();
        let current = || original.check(&actual);
        let mut subject = None;
        let mut created = None;
        let reached = (|| -> Result<(), Code> {
            subject =
                Some(self.body_namespace_operand(artifact, command, &recipe.operand, execution)?);
            current().map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
            if execution.done {
                return Ok(());
            }
            match recipe.operation {
                NativeStringRangeOperation::Empty => {
                    self.retire_body_range_subject(&mut subject, &original)?;
                    let index = recipe.empty_literal.expect("compiled empty PUSH literal");
                    created = Some(self.body_namespace_operand(
                        artifact,
                        command,
                        &NamespaceOperand::Literal(index),
                        execution,
                    )?);
                    self.publish_body_range_created(
                        &mut subject,
                        created.as_ref().expect("original literal PUSH"),
                        &original,
                    )?;
                }
                NativeStringRangeOperation::Immediate(coordinates) => {
                    let result = tcl_cmd_core::string::compiled_immediate_range(
                        self,
                        &subject
                            .as_ref()
                            .expect("prepared original subject")
                            .as_ptr(),
                        coordinates,
                        artifact.stamp.physical,
                    )
                    .map_err(|error| {
                        self.report_cmd_error_using(original.retain(error), Some(&current))
                    })?;
                    match result {
                        tcl_cmd_core::string::NativeCompiledRangeResult::Borrowed(value) => {
                            self.set_result(value)
                        }
                        tcl_cmd_core::string::NativeCompiledRangeResult::Created(value) => {
                            created = Some(obj::Owned::fresh(value));
                            self.publish_body_range_created(
                                &mut subject,
                                created.as_ref().expect("reached result PUSH"),
                                &original,
                            )?;
                        }
                    }
                }
            }
            Ok(())
        })();
        self.finish_body_range_stack(&original, created, subject, reached.map(|()| Code::Ok))
    }

    fn finish_body_range_stack(
        &mut self,
        original: &Rc<super::native_list_conversion::BodyObjectOperation>,
        created: Option<obj::Owned>,
        subject: Option<obj::Owned>,
        code: Result<Code, Code>,
    ) -> Result<Code, Code> {
        let actual = self.clone();
        let current = || original.check(&actual);
        let first = current().map_err(|error| original.retain(error));
        drop(created);
        drop(subject);
        let settled = current();
        first
            .and(settled)
            .map_err(|error| self.report_cmd_error_using(error, Some(&current)))?;
        code
    }

    fn retire_body_range_subject(
        &mut self,
        subject: &mut Option<obj::Owned>,
        original: &Rc<super::native_list_conversion::BodyObjectOperation>,
    ) -> Result<(), Code> {
        let actual = self.clone();
        let current = || original.check(&actual);
        drop(subject.take());
        current()
            .map_err(|error| self.report_cmd_error_using(original.retain(error), Some(&current)))
    }

    fn publish_body_range_created(
        &mut self,
        subject: &mut Option<obj::Owned>,
        result: &obj::Owned,
        original: &Rc<super::native_list_conversion::BodyObjectOperation>,
    ) -> Result<(), Code> {
        self.retire_body_range_subject(subject, original)?;
        self.set_result(result.as_ptr());
        original
            .check(self)
            .map_err(|error| self.report_cmd_error(original.retain(error)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_cmd_core::string::{compiled_immediate_range, NativeCompiledRangeResult};
    use tcl_syntax::native_compiled_index::{NativeCompiledListIndex, NativeCompiledListRange};

    #[test]
    fn original_c9_immediate_range_retains_empty_subject_and_getrange_result() {
        let mut interp = super::super::tests::interpreter("tcl9.0");
        let coordinates = NativeCompiledListRange {
            first: NativeCompiledListIndex::from_encoded(2),
            last: NativeCompiledListIndex::from_encoded(-2),
        };
        let empty = obj::Owned::fresh(obj::new_string_bytes(b""));
        let result = compiled_immediate_range(
            &mut interp,
            &empty.as_ptr(),
            coordinates,
            tcl_dialect::TclVersion::V9_0,
        )
        .unwrap();
        assert!(
            matches!(result, NativeCompiledRangeResult::Borrowed(value) if value == empty.as_ptr())
        );
        let subject = obj::Owned::fresh(obj::new_string_bytes(b"abcdef"));
        let NativeCompiledRangeResult::Created(range) = compiled_immediate_range(
            &mut interp,
            &subject.as_ptr(),
            coordinates,
            tcl_dialect::TclVersion::V9_0,
        )
        .unwrap() else {
            panic!("reached GetRange constructor");
        };
        let range = obj::Owned::fresh(range);
        assert_eq!(obj_bytes(range.as_ptr()), b"cdef");
        assert!(compiled_immediate_range(
            &mut interp,
            &subject.as_ptr(),
            coordinates,
            tcl_dialect::TclVersion::V8_6
        )
        .is_err());
    }

    #[test]
    fn original_c9_range_recipe_executes_through_the_selected_body_artifact() {
        let mut interp = super::super::tests::interpreter("tcl9.0");
        assert_eq!(
            interp
                .eval_str(b"proc range_subject {s} {string range $s 2 end}; range_subject abcdef"),
            Code::Ok
        );
        assert_eq!(interp.result_bytes(), b"cdef");
        let procedure = interp.proc_def(b"range_subject").unwrap();
        assert!(
            super::super::cache(procedure.body.as_ptr()).is_some(),
            "original native body artifact"
        );
        assert_eq!(
            interp.eval_str(b"proc empty_range {s} {string range $s 0 -1}; empty_range abc"),
            Code::Ok
        );
        let first = obj::Owned::retain(interp.result_obj());
        assert_eq!(interp.eval_str(b"empty_range abc"), Code::Ok);
        assert_eq!(
            interp.result_obj(),
            first.as_ptr(),
            "same original compiled empty literal"
        );
    }

    #[test]
    fn original_range_getter_refusal_keeps_first_cause_and_never_adopts_new_cache() {
        use crate::cmd_control::original_list_conversion_tests as fixture;
        use std::cell::Cell;
        for mode in [1, 2] {
            let mut interp = fixture::native("tcl9.0");
            let _current = fixture::install(&mut interp, mode);
            let subject = fixture::original(b"abcdef");
            interp.set_result_bytes(b"PRIOR");
            let prior = interp.get_obj_result();
            let error = compiled_immediate_range(
                &mut interp,
                &subject.as_ptr(),
                NativeCompiledListRange {
                    first: NativeCompiledListIndex::from_encoded(2),
                    last: NativeCompiledListIndex::from_encoded(-2),
                },
                tcl_dialect::TclVersion::V9_0,
            )
            .unwrap_err();
            assert_eq!(
                error.native_execution_refusal(),
                interp.native_execution_refusal().as_ref()
            );
            if mode == 1 {
                fixture::host(&interp.native_execution_refusal().unwrap());
            }
            assert_eq!(interp.get_obj_result(), prior);
            assert_eq!(fixture::FREES.with(Cell::get), 1);
            assert_eq!(fixture::REFS.with(Cell::get), 1);
            assert_eq!(unsafe { (*subject.as_ptr()).ref_count }, 1);
            assert!(!matches!(
                obj::native_object_snapshot(subject.as_ptr()).unwrap().cache,
                tcl_syntax::native_object::NativeObjectCacheSnapshot::String { .. }
            ));
        }
    }

    thread_local! { static CREATED_FREES: std::cell::Cell<usize> = const { std::cell::Cell::new(0) }; }
    extern "C" fn free_created(_original: *mut TclObj) {
        CREATED_FREES.with(|count| count.set(count.get() + 1));
    }
    static CREATED: obj::TclObjType = obj::TclObjType {
        name: c"originalRangeCreatedRetirement".as_ptr(),
        free_int_rep_proc: Some(free_created),
        dup_int_rep_proc: None,
        update_string_proc: None,
        set_from_any_proc: None,
    };

    #[test]
    fn original_range_subject_retirement_precedes_publication_and_retires_created_result_on_refusal(
    ) {
        use crate::cmd_control::original_list_conversion_tests as fixture;
        use std::cell::Cell;
        let mut interp = fixture::native("tcl9.0");
        let _current = fixture::install(&mut interp, 3);
        interp.frames.borrow_mut().push(crate::namespace::GLOBAL);
        let original =
            super::super::native_list_conversion::BodyObjectOperation::capture(&interp).unwrap();
        let frame = interp.frames.borrow().current_activation();
        let mut subject = Some(fixture::original(b"SUBJECT"));
        let created = obj::Owned::fresh(obj::alloc_typed(&CREATED, 0));
        CREATED_FREES.with(|count| count.set(0));
        interp.set_result_bytes(b"PRIOR");
        let prior = interp.get_obj_result();
        let code = interp
            .publish_body_range_created(&mut subject, &created, &original)
            .map(|()| Code::Ok);
        assert!(code.is_err());
        assert!(subject.is_none());
        assert_ne!(interp.frames.borrow().current_activation(), frame);
        assert_eq!(
            interp.get_obj_result(),
            prior,
            "retirement refusal prevents result publication"
        );
        assert_eq!(fixture::FREES.with(Cell::get), 1);
        assert_eq!(fixture::REFS.with(Cell::get), 0);
        assert_eq!(
            CREATED_FREES.with(Cell::get),
            0,
            "result PUSH remains held before required cleanup"
        );
        let first = interp.native_execution_refusal().unwrap();
        assert!(interp
            .finish_body_range_stack(&original, Some(created), subject, code)
            .is_err());
        assert_eq!(CREATED_FREES.with(Cell::get), 1);
        assert_eq!(interp.native_execution_refusal(), Some(first));
        assert_eq!(interp.get_obj_result(), prior);
    }

    #[test]
    fn original_empty_subject_is_borrowed_without_a_discard_or_new_native_hold() {
        use crate::cmd_control::original_list_conversion_tests as fixture;
        use std::cell::Cell;
        let mut interp = fixture::native("tcl9.0");
        let _current = fixture::install(&mut interp, 0);
        let subject = fixture::original(b"");
        let result = compiled_immediate_range(
            &mut interp,
            &subject.as_ptr(),
            NativeCompiledListRange {
                first: NativeCompiledListIndex::from_encoded(2),
                last: NativeCompiledListIndex::from_encoded(-2),
            },
            tcl_dialect::TclVersion::V9_0,
        )
        .unwrap();
        assert!(
            matches!(result, NativeCompiledRangeResult::Borrowed(value) if value == subject.as_ptr())
        );
        assert_eq!(fixture::FREES.with(Cell::get), 0);
        assert_eq!(unsafe { (*subject.as_ptr()).ref_count }, 1);
    }
}
