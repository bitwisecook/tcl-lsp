// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Metadata-only selector layout for separately callable ensemble workers.

use super::CodegenCtx;

impl CodegenCtx<'_> {
    pub(super) fn with_native_hook_operands<T>(
        &mut self,
        command: &str,
        arguments: &[(String, bool)],
        emit: impl FnOnce(&mut Self, &[(String, bool)]) -> T,
    ) -> T {
        // A statement hook already scoped this layout before its inline bridge.
        if self.native_hook_layout.is_some() {
            return emit(self, arguments);
        }
        let prefix = self
            .invocation_tokens
            .as_deref()
            .and_then(|tokens| {
                crate::registry_invocation::admitted_native_compiler_invocation(
                    self.registry,
                    None,
                    tokens,
                )
                .map(|invocation| invocation.hook_operand_prefix())
            })
            .unwrap_or(&[]);
        if prefix.is_empty() {
            // Quoting belongs to the frozen operands even when this worker
            // adds no selector. Value-position bridges share the statement
            // emitter and must not inherit another command's bracing flags.
            let previous_bracing = std::mem::replace(
                &mut self.cmd_arg_braced,
                arguments.iter().map(|(_, braced)| *braced).collect(),
            );
            let result = emit(self, arguments);
            self.cmd_arg_braced = previous_bracing;
            return result;
        }
        let mut logical = prefix
            .iter()
            .map(|selector| ((*selector).to_owned(), true))
            .collect::<Vec<_>>();
        logical.extend_from_slice(arguments);
        let previous = self
            .native_hook_layout
            .replace((command.to_owned(), prefix.len()));
        let previous_bracing = std::mem::replace(
            &mut self.cmd_arg_braced,
            logical.iter().map(|(_, braced)| *braced).collect(),
        );
        let result = emit(self, &logical);
        self.cmd_arg_braced = previous_bracing;
        self.native_hook_layout = previous;
        result
    }

    pub(super) fn original_hook_argument(&self, logical: usize) -> Option<usize> {
        logical.checked_sub(
            self.native_hook_layout
                .as_ref()
                .map_or(0, |(_, count)| *count),
        )
    }

    pub(super) fn original_hook_operands<'a>(
        &self,
        command: &str,
        arguments: &'a [(String, bool)],
    ) -> &'a [(String, bool)] {
        if let Some((original, count)) = &self.native_hook_layout
            && original == command
        {
            return arguments.get(*count..).unwrap_or(&[]);
        }
        arguments
    }
}

#[cfg(test)]
mod tests {
    use tcl_runtime_api::CompileService;

    #[test]
    fn stack_level_hook_emits_the_original_zero_or_one_operand() {
        for (source, expected) in [
            ("return [info level]", tcl_bytecode::Op::INFO_LEVEL_NUM),
            (
                "return [info level [set n 0]]",
                tcl_bytecode::Op::INFO_LEVEL_ARGS,
            ),
            (
                "return [::tcl::info::level 0]",
                tcl_bytecode::Op::INFO_LEVEL_ARGS,
            ),
        ] {
            let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
            let service = crate::compile_service::BytecodeCompileService::for_profile(profile);
            let (_native_owner, entry) =
                crate::environment_ingress::captured_native_entry_with_owner(profile);
            let module = service
                .compile_procedure_with_entry(
                    tcl_runtime_api::ProcedureCompileTarget {
                        source,
                        namespace: "::",
                        parameters: &[],
                    },
                    profile,
                    &entry,
                    tcl_runtime_api::ProcedureDispatch::Optimised,
                )
                .unwrap();
            assert!(
                module
                    .top_level
                    .instructions
                    .iter()
                    .any(|instruction| instruction.op == expected),
                "{source}"
            );
        }
    }

    #[test]
    fn admitted_lappend_result_hooks_preserve_protected_and_value_contexts() {
        for dialect in ["tcl9.0", "tcl9.1"] {
            let profile = tcl_dialect::DialectProfile::find(dialect).unwrap();
            let service = crate::compile_service::BytecodeCompileService::for_profile(profile);
            for source in [
                "set a(k) OLD; catch {lappend a(k)} r",
                "set a(k) OLD; return [lappend a(k)]",
                "set a(k) OLD; try {error bad} on error {message options} {lappend a(k)}",
            ] {
                let module = service
                    .compile_procedure_for_profile(
                        tcl_runtime_api::ProcedureCompileTarget {
                            source,
                            namespace: "::",
                            parameters: &[],
                        },
                        profile,
                        tcl_runtime_api::ProcedureDispatch::Optimised,
                    )
                    .unwrap();
                assert_eq!(
                    module.top_level.instructions.iter().any(|instruction| {
                        instruction.op == tcl_bytecode::Op::LAPPEND_LIST_ARRAY
                    }),
                    dialect == "tcl9.1",
                    "{dialect}: {source}",
                );
            }
        }
    }

    #[test]
    fn private_string_worker_uses_the_logical_selector_without_emitting_it() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let service = crate::compile_service::BytecodeCompileService::for_profile(profile);
        let module = service
            .compile("set result [::tcl::string::equal {a b} {a b}]")
            .unwrap();
        assert!(
            module
                .top_level
                .instructions
                .iter()
                .any(|instruction| { instruction.op == tcl_bytecode::Op::STR_EQ })
        );
    }

    #[test]
    fn private_array_worker_maps_its_original_local_operand() {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let service = crate::compile_service::BytecodeCompileService::for_profile(profile);
        let (_native_owner, entry) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let parameters = [];
        let module = service
            .compile_procedure_with_entry(
                tcl_runtime_api::ProcedureCompileTarget {
                    source: "return [::tcl::array::exists a]",
                    namespace: "::",
                    parameters: &parameters,
                },
                profile,
                &entry,
                tcl_runtime_api::ProcedureDispatch::Optimised,
            )
            .unwrap();
        assert!(
            module
                .top_level
                .instructions
                .iter()
                .any(|instruction| { instruction.op == tcl_bytecode::Op::ARRAY_EXISTS_IMM })
        );
        assert!(
            module
                .top_level
                .lvt
                .entries()
                .iter()
                .any(|name| name == "a")
        );
    }
}
