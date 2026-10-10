// SPDX-License-Identifier: AGPL-3.0-or-later
//! Original namespace string operands and counted compiler result ownership.

use super::*;
use tcl_registry::native_namespace_string_compilation::{
    NativeNamespaceStringInstruction, NativeNamespaceStringOperation,
};

pub(super) struct NamespaceStringOperation {
    operand: NamespaceOperand,
    separator: usize,
    operation: NativeNamespaceStringOperation,
    pub(super) prepared_words: HashMap<usize, WordInstruction>,
}

impl Builder<'_> {
    pub(super) fn namespace_string_operation(
        &mut self,
        words: &NativeCompilerWords<'_>,
        recipe: NativeNamespaceStringInstruction,
        depth: u32,
    ) -> Result<NamespaceStringOperation, ValueError> {
        let mut prepared_words = HashMap::new();
        let operand = self.namespace_operand(words, &recipe.operand, &mut prepared_words, depth)?;
        let separator = self.literals.intern_bytes(b"::");
        Ok(NamespaceStringOperation {
            operand,
            separator,
            operation: recipe.operation,
            prepared_words,
        })
    }
}

impl Interp {
    // Native proof: naming.namespace.original-counted-tail-compiler-and-runtime
    // docs/design/analysis/name-resolution-proofs/original-counted-tail-compiler-and-runtime.md
    pub(super) fn execute_body_namespace_string(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        recipe: &NamespaceStringOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let subject = self.body_namespace_operand(artifact, command, &recipe.operand, execution)?;
        if execution.done {
            return Ok(Code::Ok);
        }
        let separator = self.body_namespace_operand(
            artifact,
            command,
            &NamespaceOperand::Literal(recipe.separator),
            execution,
        )?;
        let result = match recipe.operation {
            NativeNamespaceStringOperation::Tail => tcl_cmd_core::string::compiled_tail(
                self,
                &subject.as_ptr(),
                &separator.as_ptr(),
                artifact.stamp.physical,
            ),
            NativeNamespaceStringOperation::Qualifiers => {
                tcl_cmd_core::string::compiled_qualifiers(
                    self,
                    &subject.as_ptr(),
                    &separator.as_ptr(),
                    artifact.stamp.physical,
                )
            }
        }
        .map_err(|error| self.report_cmd_error(error))?;
        let result = obj::Owned::fresh(result);
        self.set_result(result.as_ptr());
        Ok(Code::Ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_syntax::value::ValueOps;
    const INPUTS: [&[u8]; 10] = [
        b"a::tail",
        b"no_separator",
        b"a:::tail",
        b"a::",
        b"a::\xed\xa0\x80",
        b"a::\xed\xa0\x81",
        b"a::\xff",
        b"a\0::tail",
        b"",
        b":",
    ];
    fn unhex(bytes: &str) -> Vec<u8> {
        bytes
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    fn primary(original: *mut obj::TclObj) -> String {
        let descriptor = obj::obj_type_ptr(original);
        if descriptor.is_null() {
            "NULL".into()
        } else {
            // SAFETY: original argv and interpreter result ownership keep the header live.
            unsafe { std::ffi::CStr::from_ptr((*descriptor).name) }
                .to_str()
                .unwrap()
                .into()
        }
    }
    #[test]
    // Native proof: naming.namespace.original-counted-tail-compiler-and-runtime
    // docs/design/analysis/name-resolution-proofs/original-counted-tail-compiler-and-runtime.md
    fn original_namespace_tail_matches_one_hundred_counted_native_windows() {
        let mut windows = 0;
        for (engine, table) in [
            (
                "tcl8.4",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/8.4.20/stdout.tsv"
                ),
            ),
            (
                "tcl8.5",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/8.5.19/stdout.tsv"
                ),
            ),
            (
                "tcl8.6",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/8.6.18/stdout.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/9.0.4/stdout.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_namespace_string_compilation/9.1.0/stdout.tsv"
                ),
            ),
        ] {
            let mut interp = super::super::tests::interpreter(engine);
            assert_eq!(
                interp.eval_str(b"proc originalTail {value} {namespace tail $value}"),
                Code::Ok
            );
            for row in table.lines().filter(|row| row.starts_with("R|")) {
                let fields: Vec<_> = row.split('|').collect();
                let form = fields[1];
                let case = fields[2].parse::<usize>().unwrap();
                let input = obj::Owned::fresh(obj::new_string_bytes(INPUTS[case]));
                let head = obj::Owned::fresh(obj::new_string_bytes(if form == "1" {
                    b"originalTail".as_slice()
                } else {
                    b"namespace".as_slice()
                }));
                let selector = obj::Owned::fresh(obj::new_string_bytes(b"tail"));
                let argv = if form == "1" {
                    vec![head.as_ptr(), input.as_ptr()]
                } else {
                    vec![head.as_ptr(), selector.as_ptr(), input.as_ptr()]
                };
                let code = interp.eval_original_object_vector(&argv);
                assert!(
                    !interp.host_refusal_pending(),
                    "{engine}/{form}/{case}: {:?}",
                    interp.native_access_refusal()
                );
                assert_eq!(
                    code.as_int().to_string(),
                    fields[3],
                    "{engine}/{form}/{case}"
                );
                let result = interp.get_obj_result();
                assert_eq!(
                    primary(result),
                    fields[4],
                    "{engine}/{form}/{case}: result primary"
                );
                assert_eq!(
                    usize::from(result == input.as_ptr()).to_string(),
                    fields[5],
                    "{engine}/{form}/{case}: result identity"
                );
                assert_eq!(
                    interp.native_string_bytes(&result).unwrap().as_ref(),
                    unhex(fields[7]),
                    "{engine}/{form}/{case}: counted result"
                );
                let original = table
                    .lines()
                    .find(|row| row.starts_with(&format!("I|{form}|{case}|")))
                    .unwrap()
                    .split('|')
                    .collect::<Vec<_>>();
                assert_eq!(
                    primary(input.as_ptr()),
                    original[3],
                    "{engine}/{form}/{case}: original primary"
                );
                assert_eq!(
                    interp
                        .native_string_bytes(&input.as_ptr())
                        .unwrap()
                        .as_ref(),
                    unhex(original[5]),
                    "{engine}/{form}/{case}: original counted bytes"
                );
                windows += 1;
            }
        }
        assert_eq!(windows, 100);
    }
    const QUALIFIERS_INPUTS: [&[u8]; 20] = [
        b"a::tail",
        b"no_separator",
        b"a:::tail",
        b"a::",
        b"a::\xed\xa0\x80",
        b"a::\xed\xa0\x81",
        b"a::\xff",
        b"a\0::tail",
        b"",
        b":",
        b"::a",
        b":::a",
        b"::::",
        b"a::::b",
        b"a::b::c",
        b"::a::b",
        b"\xff::tail",
        b"\xed\xa0\x80::tail",
        b"\xed\xa0\x81::tail",
        b"a::b\0::tail",
    ];

    #[test]
    // Native proof: naming.namespace.original-counted-qualifiers-compiler-and-runtime
    // docs/design/analysis/name-resolution-proofs/original-counted-qualifiers-compiler-and-runtime.md
    fn original_namespace_qualifiers_matches_two_hundred_forty_counted_native_windows() {
        // naming.namespace.original-counted-qualifiers-compiler-and-runtime
        // docs/design/analysis/name-resolution-proofs/original-counted-qualifiers-compiler-and-runtime.md
        let mut windows = 0;
        for (engine, table) in [
            (
                "tcl8.4",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_namespace_qualifiers_compilation/8.4.20/stdout.tsv"
                ),
            ),
            (
                "tcl8.5",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_namespace_qualifiers_compilation/8.5.19/stdout.tsv"
                ),
            ),
            (
                "tcl8.6",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_namespace_qualifiers_compilation/8.6.18/stdout.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_namespace_qualifiers_compilation/9.0.4/stdout.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_namespace_qualifiers_compilation/9.1.0/stdout.tsv"
                ),
            ),
            (
                "jim",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_namespace_qualifiers_compilation/jim/stdout.tsv"
                ),
            ),
        ] {
            let mut interp = super::super::tests::interpreter(engine);
            assert_eq!(
                interp.eval_str(b"proc originalQualifiers {value} {namespace qualifiers $value}"),
                Code::Ok
            );
            for row in table.lines().filter(|row| row.starts_with("R|")) {
                let fields: Vec<_> = row.split('|').collect();
                let form = fields[1];
                let case = fields[2].parse::<usize>().unwrap();
                let input = obj::Owned::fresh(obj::new_string_bytes(QUALIFIERS_INPUTS[case]));
                let head = obj::Owned::fresh(obj::new_string_bytes(if form == "1" {
                    b"originalQualifiers".as_slice()
                } else {
                    b"namespace".as_slice()
                }));
                let selector = obj::Owned::fresh(obj::new_string_bytes(b"qualifiers"));
                let argv = if form == "1" {
                    vec![head.as_ptr(), input.as_ptr()]
                } else {
                    vec![head.as_ptr(), selector.as_ptr(), input.as_ptr()]
                };
                let code = interp.eval_original_object_vector(&argv);
                assert!(
                    !interp.host_refusal_pending(),
                    "{engine}/{form}/{case}: {:?}",
                    interp.native_access_refusal()
                );
                assert_eq!(
                    code.as_int().to_string(),
                    fields[3],
                    "{engine}/{form}/{case}"
                );
                let result = interp.get_obj_result();
                assert_eq!(
                    primary(result),
                    fields[4],
                    "{engine}/{form}/{case}: result primary"
                );
                assert_eq!(
                    usize::from(result == input.as_ptr()).to_string(),
                    fields[5],
                    "{engine}/{form}/{case}: result identity"
                );
                assert_eq!(
                    interp.native_string_bytes(&result).unwrap().as_ref(),
                    unhex(fields[7]),
                    "{engine}/{form}/{case}: counted result"
                );
                let original = table
                    .lines()
                    .find(|row| row.starts_with(&format!("I|{form}|{case}|")))
                    .unwrap()
                    .split('|')
                    .collect::<Vec<_>>();
                assert_eq!(
                    primary(input.as_ptr()),
                    original[3],
                    "{engine}/{form}/{case}: original primary"
                );
                assert_eq!(
                    interp
                        .native_string_bytes(&input.as_ptr())
                        .unwrap()
                        .as_ref(),
                    unhex(original[5]),
                    "{engine}/{form}/{case}: original counted bytes"
                );
                windows += 1;
            }
        }
        assert_eq!(windows, 240);
    }
}
