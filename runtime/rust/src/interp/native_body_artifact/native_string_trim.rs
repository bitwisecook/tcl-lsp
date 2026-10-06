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

//! Original trim operand preparation and same-subject no-cut publication.
use super::*;
use tcl_registry::native_string_trim_compilation::{
    NativeStringTrimInstruction, NativeStringTrimOperation,
};

pub(super) struct StringTrimOperation {
    subject: NamespaceOperand,
    characters: NamespaceOperand,
    operation: NativeStringTrimOperation,
    pub(super) prepared_words: HashMap<usize, WordInstruction>,
}

impl Builder<'_> {
    pub(super) fn string_trim_operation(
        &mut self,
        words: &NativeCompilerWords<'_>,
        recipe: NativeStringTrimInstruction,
        depth: u32,
    ) -> Result<StringTrimOperation, ValueError> {
        let mut prepared_words = HashMap::new();
        let subject = self.namespace_operand(words, &recipe.subject, &mut prepared_words, depth)?;
        let characters = match recipe.characters {
            Some(original) => {
                self.namespace_operand(words, &original, &mut prepared_words, depth)?
            }
            None => NamespaceOperand::Literal(self.literals.intern_bytes(
                tcl_registry::native_string_trim_compilation::default_trim_set(self.stamp.physical),
            )),
        };
        Ok(StringTrimOperation {
            subject,
            characters,
            operation: recipe.operation,
            prepared_words,
        })
    }
}
impl Interp {
    pub(super) fn execute_body_string_trim(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        trim: &StringTrimOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let subject = self.body_namespace_operand(artifact, command, &trim.subject, execution)?;
        if execution.done {
            return Ok(Code::Ok);
        }
        let characters =
            self.body_namespace_operand(artifact, command, &trim.characters, execution)?;
        if execution.done {
            return Ok(Code::Ok);
        }
        let (left, right) = trim.operation.ends();
        let result = tcl_cmd_core::string::compiled_trim(
            self,
            &subject.as_ptr(),
            &characters.as_ptr(),
            artifact.stamp.physical,
            left,
            right,
        )
        .map_err(|error| self.report_cmd_error(error))?;
        self.set_result(result);
        Ok(Code::Ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    include!("../../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/cases.rs");
    fn trim_interpreter(engine: &str) -> Interp {
        if engine != "jim" {
            return super::super::tests::interpreter(engine);
        }
        let interp = Interp::with_native_core(
            default_host(),
            crate::environment::profile_for_dialect(engine),
            tcl_registry::special_vars::NativeBootstrapInputs::default(),
        )
        .expect("the original Jim trim fixture requires its genuine core");
        assert!(interp.native_compiler_cache_epochs(GLOBAL).is_none());
        assert!(interp.source_string_protocol().is_some());
        interp
    }

    #[test]
    fn original_string_trim_matches_all_216_native_header_windows() {
        let table = include_str!(
            "../../../../../rust/tcl-registry/tests/data/native_string_trim_compilation/windows.tsv"
        );
        let mut compared = 0;
        for engine in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let mut interp = trim_interpreter(engine);
            for row in table.lines().skip(1) {
                let f: Vec<_> = row.split('|').collect();
                if f[0] != engine {
                    continue;
                }
                let mode = f[1];
                let member = f[2];
                let case = f[3].parse::<usize>().unwrap();
                let source = format!(
                    "proc p {{s c}} {{string {member} $s{}}}",
                    if case == 5 { " $c" } else { "" }
                );
                assert_eq!(interp.eval_str(source.as_bytes()), Code::Ok, "{row}");
                let subject = obj::Owned::fresh(if engine == "jim" {
                    obj::new_string_bytes(TRIM_INPUTS[case])
                } else {
                    crate::bytearray::new_byte_array(
                        TRIM_INPUTS[case],
                        interp
                            .native_invocation_dialect()
                            .byte_array_string_recipe(None)
                            .unwrap(),
                    )
                });
                let characters = obj::Owned::fresh(obj::new_string_bytes(b"-"));
                let head = obj::Owned::fresh(obj::new_string_bytes(if mode == "1" {
                    b"p"
                } else {
                    b"string"
                }));
                let member = obj::Owned::fresh(obj::new_string_bytes(member.as_bytes()));
                let argv = if mode == "1" {
                    vec![head.as_ptr(), subject.as_ptr(), characters.as_ptr()]
                } else {
                    let mut args = vec![head.as_ptr(), member.as_ptr(), subject.as_ptr()];
                    if case == 5 {
                        args.push(characters.as_ptr());
                    }
                    args
                };
                let code = interp.eval_original_object_vector(&argv);
                assert!(
                    !interp.host_refusal_pending(),
                    "{row}: {:?}",
                    interp.native_access_refusal()
                );
                assert_eq!(code, Code::from_int(f[4].parse().unwrap()), "{row}");
                let result = interp.result_obj();
                // Result and original subject are genuinely live interpreter/embedding
                // roles. Inspect fields without acquiring an observer reference.
                let result_refs = unsafe { usize::try_from((*result).ref_count).unwrap() };
                let subject_refs =
                    unsafe { usize::try_from((*subject.as_ptr()).ref_count).unwrap() };
                assert_eq!(
                    trim_header(&obj::native_object_snapshot(result).unwrap(), result_refs),
                    f[5],
                    "{row}"
                );
                assert_eq!(
                    usize::from(result == subject.as_ptr()).to_string(),
                    f[6],
                    "{row}"
                );
                assert_eq!(
                    trim_header(
                        &obj::native_object_snapshot(subject.as_ptr()).unwrap(),
                        subject_refs
                    ),
                    f[7],
                    "{row}"
                );
                assert_eq!(trim_hex(&interp.result_bytes()), f[9], "{row}");
                compared += 1;
            }
        }
        assert_eq!(compared, 216);
    }
}
