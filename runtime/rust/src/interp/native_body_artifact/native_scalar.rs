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

//! Retained original scalar operands and selected native getters/result owners.

use super::*;
use tcl_registry::native_scalar_compilation::{NativeScalarInstruction, NativeScalarOperation};
use tcl_syntax::value::ValueOps;

pub(super) struct ScalarOperation {
    operands: Vec<NamespaceOperand>,
    operation: NativeScalarOperation,
    pub(super) prepared_words: HashMap<usize, WordInstruction>,
}

impl Builder<'_> {
    pub(super) fn scalar_operation(
        &mut self,
        words: &NativeCompilerWords<'_>,
        recipe: NativeScalarInstruction,
        depth: u32,
    ) -> Result<ScalarOperation, ValueError> {
        if recipe.operands.len() != recipe.operation.arity() {
            return Err(ValueError::CommandProtocolUnavailable(
                "native scalar operand geometry",
            ));
        }
        let mut prepared_words = HashMap::new();
        let operands = recipe
            .operands
            .iter()
            .map(|operand| self.namespace_operand(words, operand, &mut prepared_words, depth))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(ScalarOperation {
            operands,
            operation: recipe.operation,
            prepared_words,
        })
    }
}

impl Interp {
    pub(super) fn execute_body_scalar(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        scalar: &ScalarOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let mut operands = Vec::with_capacity(scalar.operands.len());
        for operand in &scalar.operands {
            operands.push(self.body_namespace_operand(artifact, command, operand, execution)?);
            if execution.done {
                return Ok(Code::Ok);
            }
        }
        let version = artifact.stamp.physical;
        let result = match scalar.operation {
            NativeScalarOperation::StringEqual => {
                let matched = tcl_cmd_core::switch::compiled_equal(
                    self,
                    &operands[0].as_ptr(),
                    &operands[1].as_ptr(),
                    version,
                )
                .map_err(|error| self.report_cmd_error(error))?;
                let right = operands.pop().expect("validated second scalar operand");
                drop(right);
                self.native_compiled_match_result(
                    operands.pop().expect("validated first scalar operand"),
                    matched,
                    version,
                    tcl_registry::native_string_compilation::NativeStringMatchOperation::Equal,
                )
                .map_err(|error| self.report_cmd_error(error.into()))?
            }
            operation => {
                let input = operands[0].as_ptr();
                let length = match operation {
                    NativeScalarOperation::StringLength => self.native_char_len(&input),
                    NativeScalarOperation::ListLength => self.list_len(&input),
                    NativeScalarOperation::StringEqual => unreachable!("equality handled above"),
                }
                .map_err(|error| self.report_cmd_error(error.into()))?;
                let result = self
                    .native_scalar_length_result(length, version)
                    .map_err(|error| self.report_cmd_error(error.into()))?;
                drop(operands);
                result
            }
        };
        self.set_result(result.as_ptr());
        Ok(Code::Ok)
    }

    fn native_scalar_length_result(
        &self,
        length: usize,
        version: tcl_dialect::TclVersion,
    ) -> Result<obj::Owned, ValueError> {
        let dialect = self.native_invocation_dialect();
        if dialect.native_error_log_protocol().is_none() || dialect.tcl_version != Some(version) {
            return Err(ValueError::CommandProtocolUnavailable(
                "native scalar length result issuer",
            ));
        }
        let length = i64::try_from(length)
            .map_err(|_| ValueError::CommandProtocolUnavailable("native scalar length width"))?;
        if version != tcl_dialect::TclVersion::V8_4 {
            return Ok(obj::Owned::fresh(obj::new_wide_int_obj(length)));
        }
        let protocol = dialect
            .native_scalar_getter_protocol()
            .ok_or(ValueError::ScalarNumericInputUnavailable)?;
        let result = obj::Owned::fresh(obj::new_obj());
        obj::invalidate_string(result.as_ptr());
        obj::adopt_native_scalar_cache(
            result.as_ptr(),
            tcl_syntax::scalar_getter::NativeScalarCache::Tcl84Long(length),
            protocol,
        )?;
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    include!("../../../../../rust/tcl-registry/tests/data/native_scalar_compilation/cases.rs");

    fn primary(original: *mut obj::TclObj) -> String {
        let descriptor = obj::obj_type_ptr(original);
        if descriptor.is_null() {
            "none".into()
        } else {
            // SAFETY: genuine argv or interpreter ownership keeps this header live.
            unsafe { std::ffi::CStr::from_ptr((*descriptor).name) }
                .to_str()
                .unwrap()
                .into()
        }
    }
    fn unhex(bytes: &str) -> Vec<u8> {
        bytes
            .as_bytes()
            .as_chunks::<2>()
            .0
            .iter()
            .map(|pair| u8::from_str_radix(core::str::from_utf8(pair).unwrap(), 16).unwrap())
            .collect()
    }

    #[test]
    fn compiled_scalars_preserve_forty_one_native_original_header_windows() {
        let mut windows = 0;
        for (engine, table) in [
            (
                "tcl8.4",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_scalar_compilation/8.4.20.tsv"
                ),
            ),
            (
                "tcl8.5",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_scalar_compilation/8.5.19.tsv"
                ),
            ),
            (
                "tcl8.6",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_scalar_compilation/8.6.18.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_scalar_compilation/9.0.4.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_scalar_compilation/9.1.0.tsv"
                ),
            ),
        ] {
            let mut interp = super::super::tests::interpreter(engine);
            for row in table.lines() {
                let fields: Vec<_> = row.split('\t').collect();
                if !fields[9]
                    .split(',')
                    .any(|op| matches!(op, "streq" | "strlen" | "listlength" | "listLength"))
                {
                    continue;
                }
                let case = fields[0].parse::<usize>().unwrap();
                assert_eq!(
                    interp
                        .eval_str(format!("proc p {{left right}} {{{}}}", CASES[case]).as_bytes()),
                    Code::Ok
                );
                let left = if matches!(case, 7..=9) {
                    b"a b".as_slice()
                } else {
                    b"A\0x".as_slice()
                };
                let words = [b"p".as_slice(), left, b"A\0y".as_slice()]
                    .map(|bytes| obj::Owned::fresh(obj::new_string_bytes(bytes)));
                let argv = words.each_ref().map(obj::Owned::as_ptr);
                let code = interp.eval_original_object_vector(&argv);
                assert!(
                    !interp.host_refusal_pending(),
                    "{engine}/{case}: {:?}",
                    interp.native_access_refusal()
                );
                assert_eq!(code.as_int().to_string(), fields[1], "{engine}/{case}");
                let result = interp.get_obj_result();
                assert_eq!(primary(result), fields[2], "{engine}/{case}");
                // SAFETY: interpreter result and genuine external argv owners are live.
                unsafe {
                    assert_eq!(
                        usize::from(!(*result).bytes.is_null()).to_string(),
                        fields[3],
                        "{engine}/{case}"
                    );
                    assert_eq!(
                        (*result).ref_count.to_string(),
                        fields[4],
                        "{engine}/{case}"
                    );
                    assert_eq!(
                        usize::from(!(*argv[1]).bytes.is_null()).to_string(),
                        fields[6],
                        "{engine}/{case}"
                    );
                }
                assert_eq!(primary(argv[1]), fields[5], "{engine}/{case}");
                assert_eq!(
                    usize::from(result == argv[1]).to_string(),
                    fields[7],
                    "{engine}/{case}"
                );
                assert_eq!(
                    interp.native_object_string_bytes(result).unwrap().as_ref(),
                    unhex(fields[8]),
                    "{engine}/{case}"
                );
                windows += 1;
            }
        }
        assert_eq!(windows, 41);
    }
}
