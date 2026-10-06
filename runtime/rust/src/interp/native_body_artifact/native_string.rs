// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original stack operands for the authenticated C StringMatch compiler.

use super::*;
use tcl_registry::native_string_compilation::{
    NativeStringMatchInstruction, NativeStringMatchOperation,
};

pub(super) struct StringMatchOperation {
    pattern: NamespaceOperand,
    subject: NamespaceOperand,
    operation: NativeStringMatchOperation,
    pub(super) prepared_words: HashMap<usize, WordInstruction>,
}

impl Builder<'_> {
    pub(super) fn string_match_operation(
        &mut self,
        words: &NativeCompilerWords<'_>,
        recipe: NativeStringMatchInstruction,
        depth: u32,
    ) -> Result<StringMatchOperation, ValueError> {
        let mut prepared_words = HashMap::new();
        let pattern = self.namespace_operand(words, &recipe.pattern, &mut prepared_words, depth)?;
        let subject = self.namespace_operand(words, &recipe.subject, &mut prepared_words, depth)?;
        Ok(StringMatchOperation {
            pattern,
            subject,
            operation: recipe.operation,
            prepared_words,
        })
    }
}

impl Interp {
    pub(super) fn execute_body_string_match(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        matcher: &StringMatchOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let pattern =
            self.body_namespace_operand(artifact, command, &matcher.pattern, execution)?;
        if execution.done {
            return Ok(Code::Ok);
        }
        let subject =
            self.body_namespace_operand(artifact, command, &matcher.subject, execution)?;
        if execution.done {
            return Ok(Code::Ok);
        }
        let matched = match matcher.operation {
            NativeStringMatchOperation::Equal => tcl_cmd_core::switch::compiled_equal(
                self,
                &pattern.as_ptr(),
                &subject.as_ptr(),
                artifact.stamp.physical,
            ),
            NativeStringMatchOperation::Glob { nocase } => tcl_cmd_core::switch::compiled_glob(
                self,
                &pattern.as_ptr(),
                &subject.as_ptr(),
                artifact.stamp.physical,
                nocase,
            ),
        }
        .map_err(|error| self.report_cmd_error(error))?;
        drop(subject);
        let result = self
            .native_compiled_match_result(
                pattern,
                matched,
                artifact.stamp.physical,
                matcher.operation,
            )
            .map_err(|error| self.report_cmd_error(error.into()))?;
        self.set_result(result.as_ptr());
        Ok(Code::Ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compiled_string_match_headers_match_33_original_native_windows() {
        // Keep the complete native sequence: result string getters prime the
        // genuine execution constants used by later admitted instructions.
        let cases: [&[u8]; 12] = [
            b"string match * $subject",
            b"string match A $subject",
            b"string match {]} $subject",
            b"string match -n A $subject",
            b"string match - A $subject",
            b"string match -nocaseX A $subject",
            b"string match $flag A $subject",
            b"string match $pattern $subject",
            b"string match \\-n A $subject",
            b"string match {*}\"A\" $subject",
            b"string match {*}{* X}",
            b"string match A",
        ];
        let table = include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_string_compilation/windows.tsv"
        );
        let mut executed = 0;
        let mut compared = 0;
        for (version, profile) in [
            ("8.4.20", "tcl8.4"),
            ("8.5.19", "tcl8.5"),
            ("8.6.18", "tcl8.6"),
            ("9.0.4", "tcl9.0"),
            ("9.1.0", "tcl9.1"),
        ] {
            let mut interp = super::super::tests::interpreter(profile);
            for row in table.lines().skip(1) {
                let fields = row.split('\t').collect::<Vec<_>>();
                if fields[0] != version {
                    continue;
                }
                let case = fields[1].parse::<usize>().unwrap();
                // The native probe defines each procedure through Tcl_EvalEx,
                // then invokes it through Tcl_EvalObjv on the original arguments.
                let mut definition = b"proc p {pattern subject flag} {".to_vec();
                definition.extend_from_slice(cases[case]);
                definition.push(b'}');
                assert_eq!(
                    interp.eval_str(&definition),
                    Code::Ok,
                    "{version}/{case} definition: {:?}",
                    interp.native_access_refusal(),
                );
                assert!(
                    !interp.host_refusal_pending(),
                    "{version}/{case} native definition: {:?}",
                    interp.native_access_refusal(),
                );
                let original = [b"p".as_slice(), b"*", b"A", b"-n"]
                    .map(|bytes| obj::Owned::fresh(new_string(bytes)));
                let argv = original.each_ref().map(obj::Owned::as_ptr);
                let code = interp.eval_original_object_vector(&argv);
                assert!(
                    !interp.host_refusal_pending(),
                    "{version}/{case} native invocation: {:?}",
                    interp.native_access_refusal(),
                );
                assert_eq!(
                    code,
                    Code::from_int(fields[2].parse().unwrap()),
                    "{version}/{case}"
                );
                let inline = fields[8]
                    .split(',')
                    .any(|opcode| matches!(opcode, "streq" | "strmatch"));
                if inline {
                    let result = interp.result_obj();
                    let descriptor = obj::obj_type_ptr(result);
                    assert!(!descriptor.is_null(), "{version}/{case} result primary");
                    // The interpreter owns this live result. Observe its native
                    // header before any getter, without acquiring a reference.
                    let (class, references) = unsafe {
                        (
                            core::ffi::CStr::from_ptr((*descriptor).name)
                                .to_str()
                                .unwrap(),
                            (*result).ref_count,
                        )
                    };
                    assert_eq!(class, fields[3], "{version}/{case} primary");
                    assert_eq!(
                        usize::from(obj::has_string_rep(result)),
                        fields[4].parse::<usize>().unwrap(),
                        "{version}/{case} resident"
                    );
                    assert_eq!(
                        references,
                        fields[5].parse().unwrap(),
                        "{version}/{case} refs"
                    );
                    assert_eq!(
                        usize::from(result == argv[1]),
                        fields[6].parse::<usize>().unwrap(),
                        "{version}/{case} original pattern"
                    );
                    let procedure = interp.proc_def(b"p").unwrap();
                    let artifact =
                        cache(procedure.body.as_ptr()).expect("actual original artifact");
                    assert!(artifact.scripts.values().any(|script| script
                        .commands
                        .iter()
                        .any(|command| matches!(command.operation, Operation::StringMatch(_)))));
                    compared += 1;
                }
                assert_eq!(
                    interp.result_bytes(),
                    fields[7].as_bytes(),
                    "{version}/{case} result"
                );
                executed += 1;
            }
        }
        assert_eq!(executed, 60);
        assert_eq!(compared, 33);
    }

    #[test]
    fn original_match_artifact_preserves_renamed_registration_and_withdraws_replaced_target() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = super::super::tests::interpreter(profile);
            assert_eq!(
                interp.eval_str(b"rename string saved; proc p {x} {saved match * $x}; p A"),
                Code::Ok,
                "{profile}"
            );
            assert_eq!(interp.result_bytes(), b"1", "{profile}");
            let procedure = interp.proc_def(b"p").unwrap();
            let artifact = cache(procedure.body.as_ptr()).expect("same renamed original compiler");
            assert!(
                artifact.scripts.values().any(|script| script
                    .commands
                    .iter()
                    .any(|command| matches!(command.operation, Operation::StringMatch(_)))),
                "{profile}"
            );
            assert_eq!(
                interp.eval_str(b"proc saved args {return REPLACED}; p A"),
                Code::Ok,
                "{profile}"
            );
            assert_eq!(interp.result_bytes(), b"REPLACED", "{profile}");
            let current = interp.proc_def(b"p").unwrap();
            if let Some(artifact) = cache(current.body.as_ptr()) {
                assert!(
                    artifact.scripts.values().all(|script| script
                        .commands
                        .iter()
                        .all(|command| !matches!(command.operation, Operation::StringMatch(_)))),
                    "{profile}: replacement cannot donate the original matcher"
                );
            }
        }
    }

    #[test]
    fn compiled_match_results_retain_actual_pattern_or_execution_constant_owners() {
        for (profile, version) in [
            ("tcl8.4", tcl_dialect::TclVersion::V8_4),
            ("tcl8.5", tcl_dialect::TclVersion::V8_5),
            ("tcl8.6", tcl_dialect::TclVersion::V8_6),
            ("tcl9.0", tcl_dialect::TclVersion::V9_0),
            ("tcl9.1", tcl_dialect::TclVersion::V9_1),
        ] {
            let mut interp = super::super::tests::interpreter(profile);
            let pattern = obj::Owned::fresh(new_string(b"*"));
            let original = pattern.as_ptr();
            let result = interp
                .native_compiled_match_result(
                    pattern,
                    true,
                    version,
                    NativeStringMatchOperation::Glob { nocase: false },
                )
                .unwrap();
            if version == tcl_dialect::TclVersion::V8_4 {
                assert_eq!(result.as_ptr(), original, "original unshared pattern");
                assert_eq!(
                    obj::native_scalar_cache(result.as_ptr()).unwrap(),
                    Some(tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(1))
                );
            } else {
                assert_ne!(result.as_ptr(), original);
                assert_eq!(
                    interp.native_execution_boolean_constant(true).unwrap(),
                    result.as_ptr()
                );
            }
            assert!(obj::native_object_snapshot(result.as_ptr())
                .unwrap()
                .resident
                .is_none());
            drop(result);

            let pattern = obj::Owned::fresh(new_string(b"*"));
            let external = pattern.clone();
            let result = interp
                .native_compiled_match_result(
                    pattern,
                    false,
                    version,
                    NativeStringMatchOperation::Glob { nocase: false },
                )
                .unwrap();
            assert_ne!(
                result.as_ptr(),
                external.as_ptr(),
                "shared original is preserved"
            );
            assert_eq!(obj_bytes(external.as_ptr()), b"*");
            drop(result);
            let pattern = obj::Owned::fresh(new_string(b"A"));
            let original = pattern.as_ptr();
            let result = interp
                .native_compiled_match_result(
                    pattern,
                    true,
                    version,
                    NativeStringMatchOperation::Equal,
                )
                .unwrap();
            assert_ne!(
                result.as_ptr(),
                original,
                "STR_EQ never mutates the original pattern"
            );
        }
    }

    #[test]
    fn original_string_match_artifacts_preserve_operand_order_and_selected_matcher() {
        for profile in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = super::super::tests::interpreter(profile);
            for (body, expected) in [
                (b"string match * $subject".as_slice(), b"1".as_slice()),
                (b"string match A $subject".as_slice(), b"1".as_slice()),
                (b"string match -n a $subject".as_slice(), b"1".as_slice()),
                (
                    b"string match [set pattern *] [set reached $subject]".as_slice(),
                    b"1".as_slice(),
                ),
            ] {
                let mut source = b"proc p {subject} {".to_vec();
                source.extend_from_slice(body);
                source.extend_from_slice(b"}");
                assert_eq!(interp.eval_str(&source), Code::Ok, "{profile}");
                assert_eq!(interp.eval_str(b"p A"), Code::Ok, "{profile}");
                assert_eq!(interp.result_bytes(), expected, "{profile}");
                let procedure = interp.proc_def(b"p").unwrap();
                let artifact =
                    cache(procedure.body.as_ptr()).expect("authentic StringMatch artifact");
                assert!(artifact.scripts.values().any(|script| script
                    .commands
                    .iter()
                    .any(|command| matches!(command.operation, Operation::StringMatch(_)))));
            }
            assert_eq!(interp.eval_str(b"set order {}; proc first {} {lappend ::order pattern; return *}; proc second {} {lappend ::order subject; return A}; proc order_test {} {string match [first] [second]}; order_test"), Code::Ok, "{profile}");
            assert_eq!(interp.result_bytes(), b"1");
            assert_eq!(
                obj_bytes(interp.var_get_at(b"::order", 0).unwrap()),
                b"pattern subject"
            );
        }
    }
}
