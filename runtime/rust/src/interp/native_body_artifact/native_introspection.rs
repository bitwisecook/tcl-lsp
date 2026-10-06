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

//! Native introspection over original command/frame receivers and scoped List children.

use super::*;
use tcl_registry::native_introspection_compilation::{
    NativeIntrospectionInstruction, NativeIntrospectionKind as Kind,
};

pub(super) struct IntrospectionOperation {
    kind: Kind,
    operands: Vec<NamespaceOperand>,
    code_prefix: Option<[usize; 2]>,
    pub(super) prepared_words: HashMap<usize, WordInstruction>,
}

impl Builder<'_> {
    pub(super) fn introspection_operation(
        &mut self,
        words: &NativeCompilerWords<'_>,
        recipe: NativeIntrospectionInstruction,
        depth: u32,
    ) -> Result<IntrospectionOperation, ValueError> {
        let code_prefix = (recipe.kind == Kind::NamespaceCode).then(|| {
            [
                self.literals.intern_bytes(b"::namespace"),
                self.literals.intern_bytes(b"inscope"),
            ]
        });
        let mut prepared_words = HashMap::new();
        let operands = recipe
            .operands
            .iter()
            .map(|operand| self.namespace_operand(words, operand, &mut prepared_words, depth))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(IntrospectionOperation {
            kind: recipe.kind,
            operands,
            code_prefix,
            prepared_words,
        })
    }
}

impl Interp {
    pub(super) fn execute_body_introspection(
        &mut self,
        artifact: &NativeBodyArtifact,
        command: &CommandInstruction,
        recipe: &IntrospectionOperation,
        execution: &mut BodyExecution,
    ) -> Result<Code, Code> {
        let mut operands = Vec::with_capacity(recipe.operands.len());
        for operand in &recipe.operands {
            operands.push(self.body_namespace_operand(artifact, command, operand, execution)?);
            if execution.done {
                return Ok(Code::Ok);
            }
        }
        let original = match recipe.kind {
            Kind::NamespaceCurrent => obj::Owned::fresh(
                tcl_cmd_core::namespace::current_original(self)
                    .map_err(|error| self.report_cmd_error(error))?,
            ),
            Kind::InfoLevel => {
                let number = operands.first().map(obj::Owned::as_ptr);
                obj::Owned::fresh(
                    self.native_info_level(number.as_ref())
                        .map_err(|error| self.report_cmd_error(error))?,
                )
            }
            Kind::NamespaceOrigin => {
                let input = operands[0].as_ptr();
                let bytes = self
                    .native_namespace_origin(input)
                    .map_err(|error| self.report_cmd_error(error.into()))?;
                let Some(bytes) = bytes else {
                    return Err(self.native_namespace_origin_failure(input));
                };
                self.native_namespace_origin_result(&bytes)
                    .map_err(|error| self.report_cmd_error(error.into()))?
            }
            Kind::NamespaceCode => {
                let namespace = u32::try_from(self.current_ns())
                    .map(tcl_runtime_api::NsId)
                    .map_err(|_| {
                        self.report_cmd_error(
                            unavailable("native scoped namespace token width").into(),
                        )
                    })?;
                let current=obj::Owned::fresh(tcl_cmd_core::namespace::NamespaceObjectBackend::produce_namespace_object(self,namespace,tcl_syntax::native_namespace_name::NativeNamespaceObjectProducer::CodeContext).map_err(|error|self.report_cmd_error(error.into()))?);
                let prefix = recipe
                    .code_prefix
                    .expect("selected scoped List original prefix");
                let children = [
                    artifact
                        .literals
                        .original(prefix[0])
                        .expect("registered namespace prefix"),
                    artifact
                        .literals
                        .original(prefix[1])
                        .expect("registered inscope prefix"),
                    current.as_ptr(),
                    operands[0].as_ptr(),
                ];
                obj::Owned::fresh(self.new_list_object(&children))
            }
        };
        self.set_result(original.as_ptr());
        Ok(Code::Ok)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    include!(
        "../../../../../rust/tcl-registry/tests/data/native_introspection_compilation/cases.rs"
    );

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
            return "none".into();
        }
        // SAFETY: interpreter or original argument ownership keeps this header live.
        unsafe { std::ffi::CStr::from_ptr((*descriptor).name) }
            .to_str()
            .unwrap()
            .into()
    }
    #[test]
    fn compiled_introspection_preserves_thirty_three_native_original_header_windows() {
        original_windows(false);
    }

    #[test]
    fn compiled_arrays_preserve_forty_eight_native_original_header_and_local_windows() {
        original_windows(true);
    }

    #[test]
    fn compiled_array_make_preserves_the_native_write_array_error_category() {
        for engine in ["tcl8.6", "tcl9.0", "tcl9.1"] {
            let mut interp = super::super::tests::interpreter(engine);
            assert_eq!(
                interp.eval_str(
                    b"proc p {} {set a scalar; array set a {}}; catch {p} message; set ::errorCode",
                ),
                Code::Ok,
                "{engine}"
            );
            assert!(!interp.host_refusal_pending(), "{engine}");
            let result = interp.get_obj_result();
            assert_eq!(
                interp.native_object_string_bytes(result).unwrap().as_ref(),
                b"TCL WRITE ARRAY",
                "{engine}"
            );
        }
    }

    fn original_windows(arrays: bool) {
        let mut windows = 0;
        for (engine, table) in [
            (
                "tcl8.6",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_introspection_compilation/8.6.18.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_introspection_compilation/9.0.4.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_introspection_compilation/9.1.0.tsv"
                ),
            ),
        ] {
            let mut interp = super::super::tests::interpreter(engine);
            assert_eq!(interp.eval_str(b"namespace eval ::N {}; namespace eval ::source {proc target {} {}; namespace export target}; namespace eval ::N {namespace import ::source::target}"),Code::Ok);
            for row in table.lines() {
                let fields: Vec<_> = row.split('\t').collect();
                let case = fields[0].parse::<usize>().unwrap();
                if if arrays {
                    case < 15
                } else {
                    !matches!(case, 0 | 2 | 3 | 4 | 6 | 8 | 9 | 10 | 11 | 12 | 13)
                } {
                    continue;
                }
                assert_eq!(
                    interp.eval_str(
                        format!("proc ::N::p {{left right}} {{{}}}", CASES[case]).as_bytes()
                    ),
                    Code::Ok
                );
                let left = match case {
                    2 => b"target".as_slice(),
                    10 => b"0".as_slice(),
                    _ => b"a".as_slice(),
                };
                let words = [b"::N::p".as_slice(), left, b"k V j W".as_slice()]
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
                // SAFETY: interpreter and external argument owners remain live.
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
                if arrays {
                    let procedure = interp
                        .proc_def(b"::N::p")
                        .expect("actual original array procedure");
                    let original = procedure.body.checked_ptr().unwrap();
                    let artifact = super::super::cache(original).expect("original array Bytecode");
                    let layout = artifact
                        .compiled_local_layout()
                        .expect("actual original procedure local layout");
                    assert_eq!(
                        layout.names.len().to_string(),
                        fields[10],
                        "{engine}/{case}: compiled locals"
                    );
                    let names: Vec<_> = layout
                        .names
                        .iter()
                        .map(|name| {
                            name.as_ref()
                                .map_or_else(Vec::new, |name| name.as_bytes().to_vec())
                        })
                        .collect();
                    assert_eq!(
                        names,
                        fields[11].split(',').map(unhex).collect::<Vec<_>>(),
                        "{engine}/{case}: original local names"
                    );
                }
                windows += 1;
            }
        }
        assert_eq!(windows, if arrays { 48 } else { 33 });
    }
}
