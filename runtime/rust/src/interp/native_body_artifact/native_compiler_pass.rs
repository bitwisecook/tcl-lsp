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

//! Whole-body compiler compaction retains the procedure local table across actual passes.

use super::*;
use tcl_registry::native_compiler_pass::{
    NativeCompilerPassHazard as Hazard, native_compiler_replays,
};
use tcl_runtime_api::native_compiler_pass::{
    NativeCompilerPassEnvironment, NativeCompilerPassOwner, NativeCompilerPassProcedure,
};

impl Interp {
    pub(super) fn capture_body_compiler_pass_environment(
        &self,
        namespace: NsId,
        procedure: Option<&Rc<ProcDef>>,
    ) -> Option<NativeCompilerPassEnvironment> {
        self.native_invocation_dialect()
            .native_error_log_protocol()?;
        let namespaces = self.namespaces.borrow();
        let procedure = procedure.filter(|procedure| procedure.namespace() == namespace);
        let command_owned = procedure.is_some_and(|procedure| {
            namespaces.command_names(namespace).into_iter().any(|name| {
                matches!(namespaces.command_in(namespace, name),
                    Some(Command::Proc(original)) if Rc::ptr_eq(&original.declaration(), procedure))
            })
        });
        let command_owned = command_owned || procedure.is_some_and(|procedure| {
            self.hidden.borrow().values().any(|binding| {
                matches!(&binding.command, Command::Proc(original) if Rc::ptr_eq(&original.declaration(), procedure))
            })
        });
        let procedure = if command_owned {
            Some(NativeCompilerPassProcedure {
                namespace: tcl_runtime_api::native_compilation::NativeNamespaceContext {
                    interpreter: self.native_command_interpreter,
                    token: u64::try_from(namespace).ok()?,
                    path: namespaces.native_context_path(namespace)?,
                },
                namespace_full_name: tcl_runtime_api::NameBytes::from(
                    namespaces.qualified_name(namespace).as_slice(),
                ),
            })
        } else {
            None
        };
        let limits = self.limits.borrow();
        let owner = self
            .native_compiler_pass_owner
            .get_or_init(|| NativeCompilerPassOwner::new(self.native_command_interpreter));
        Some(owner.capture(
            !self.native_child_interpreter.get(),
            limits.cmd_value.is_some(),
            limits.time_value.is_some(),
            procedure,
        ))
    }
}

impl Builder<'_> {
    pub(super) fn finish_compactible_body(
        &mut self,
        region: Span,
        original: *mut TclObj,
    ) -> Result<Rc<NativeRuntimeLiteralArray>, ValueError> {
        let first_pass = self.materialize_body_literals(original)?;
        let procedure = self.procedure.as_ref().and_then(Weak::upgrade);
        let current = self
            .interp
            .native_body_stamp(self.stamp.namespace, procedure.as_ref())
            .ok_or_else(|| unavailable("native compiler context after callbacks"))?;
        if current != self.stamp {
            return Err(unavailable(
                "native compiler context changed during preparation",
            ));
        }
        // Limits select this reached compaction attempt; changing them does
        // not invalidate an already installed native Bytecode artifact.
        let environment = self
            .interp
            .capture_body_compiler_pass_environment(self.stamp.namespace, procedure.as_ref());
        let hazards = self
            .scripts
            .values()
            .flat_map(|script| {
                script
                    .commands
                    .iter()
                    .flat_map(|command| command.operation.compaction_hazards())
            })
            .collect::<Vec<_>>();
        if !native_compiler_replays(
            self.stamp.physical,
            self.stamp.interpreter,
            environment.as_ref(),
            hazards,
        ) {
            return Ok(first_pass);
        }
        // TclFreeCompileEnv releases the first executable/literal/auxiliary
        // owners, while SAME procPtr retains every completed local declaration.
        drop(first_pass);
        self.scripts.clear();
        self.literals = LiteralTable::new();
        self.private_objects.clear();
        self.parse_failure = None;
        self.compilation_failure = None;
        self.script(region, 0)?;
        self.materialize_body_literals(original)
    }

    fn materialize_body_literals(
        &mut self,
        original: *mut TclObj,
    ) -> Result<Rc<NativeRuntimeLiteralArray>, ValueError> {
        let entries = self
            .literals
            .entries()
            .iter()
            .enumerate()
            .map(|(index, literal)| {
                materialize_literal(literal, index, &self.stamp, &mut self.private_objects)
            })
            .collect::<Result<Vec<_>, ValueError>>()?;
        let actions: Vec<_> = self
            .literals
            .native_actions()
            .iter()
            .map(|action| match action {
                NativeLiteralAction::RetainSyntaxErrorInfo { options, message } => {
                    NativeRuntimeLiteralAction::RetainSyntaxErrorInfo {
                        options: *options,
                        message: *message,
                    }
                }
                NativeLiteralAction::Register(index) => {
                    NativeRuntimeLiteralAction::Register(*index)
                }
                NativeLiteralAction::AdoptExpressionNumber {
                    index,
                    version,
                    value,
                } => NativeRuntimeLiteralAction::AdoptExpressionNumber {
                    index: *index,
                    version: *version,
                    value: value.clone(),
                },
                NativeLiteralAction::Hide(index) => NativeRuntimeLiteralAction::Hide(*index),
                NativeLiteralAction::PrimeExpressionBoolean84(index) => {
                    NativeRuntimeLiteralAction::PrimeExpressionBoolean84(*index)
                }
                NativeLiteralAction::PrimeCommandName { index, receipt } => {
                    NativeRuntimeLiteralAction::PrimeCommandName {
                        index: *index,
                        receipt: receipt.clone(),
                    }
                }
            })
            .collect();
        self.interp
            .create_native_literal_array_with_actions(original, &entries, &actions)
    }
}

fn materialize_literal(
    literal: &tcl_bytecode::NativeStringLiteral,
    index: usize,
    stamp: &CacheStamp,
    private_objects: &mut HashMap<usize, obj::Owned>,
) -> Result<NativeRuntimeLiteral, ValueError> {
    Ok(match literal.allocation() {
        NativeLiteralAllocation::PrivateLogicalBoolean85(value) => {
            NativeRuntimeLiteral::PrivateLogicalBoolean85(*value)
        }
        NativeLiteralAllocation::PrivateInteger(value) => {
            NativeRuntimeLiteral::UnsharedOriginal(obj::Owned::fresh(obj::new_wide_int_obj(*value)))
        }
        NativeLiteralAllocation::PrivateExpressionNumber { version, value } => {
            NativeRuntimeLiteral::UnsharedOriginal(materialize_expression_number(
                stamp, *version, value,
            )?)
        }
        NativeLiteralAllocation::PrivateConstantList { members, protocol } => {
            NativeRuntimeLiteral::PrivateConstantList {
                members: members.clone(),
                protocol: *protocol,
            }
        }
        NativeLiteralAllocation::Unshared => {
            NativeRuntimeLiteral::UnsharedBytes(literal.bytes().to_vec())
        }
        NativeLiteralAllocation::PrivateConcatString => {
            NativeRuntimeLiteral::PrivateConcatString(literal.bytes().to_vec())
        }
        NativeLiteralAllocation::PrivateReturnOptions(recipe) => {
            NativeRuntimeLiteral::UnsharedOriginal(
                crate::native_return_merge::manufacture(recipe).map_err(|error| {
                    let _ = error;
                    unavailable("native private Return literal manufacture")
                })?,
            )
        }
        NativeLiteralAllocation::PrivateOriginal => {
            NativeRuntimeLiteral::UnsharedOriginal(private_objects.remove(&index).ok_or_else(
                || unavailable("native private original literal lacks supplied owner"),
            )?)
        }
        NativeLiteralAllocation::RegisteredNativeCommand {
            context,
            fully_qualified,
        } => NativeRuntimeLiteral::RegisteredBytes {
            bytes: literal.bytes().to_vec(),
            namespace: tcl_runtime_api::native_literal::command_literal_partition(
                stamp.source_protocol,
                tcl_core_types::NsId(
                    u32::try_from(context.namespace_token)
                        .map_err(|_| unavailable("native literal namespace partition"))?,
                ),
                *fully_qualified,
            ),
        },
        NativeLiteralAllocation::RegisteredData => NativeRuntimeLiteral::RegisteredBytes {
            bytes: literal.bytes().to_vec(),
            namespace: None,
        },
        NativeLiteralAllocation::RegisteredCommand { .. } => {
            return Err(unavailable("native literal lacks original namespace token"));
        }
    })
}

fn materialize_expression_number(
    stamp: &CacheStamp,
    version: tcl_dialect::TclVersion,
    value: &tcl_bytecode::NativeExpressionNumberLiteral,
) -> Result<obj::Owned, ValueError> {
    if version != stamp.physical || version < tcl_dialect::TclVersion::V8_5 {
        return Err(unavailable("native folded-number literal issuer"));
    }
    let original = obj::Owned::fresh(obj::new_string_bytes(b""));
    let protocol = tcl_registry::InvocationDialect::for_version(version)
        .native_scalar_getter_protocol()
        .ok_or_else(|| unavailable("native folded-number literal producer"))?;
    obj::adopt_native_scalar_cache(
        original.as_ptr(),
        tcl_syntax::scalar_getter::NativeScalarCache::Number(value.number()),
        protocol,
    )?;
    obj::invalidate_string(original.as_ptr());
    Ok(original)
}

impl Operation {
    fn compaction_hazards(&self) -> Vec<Hazard> {
        match self {
            Self::Invoke | Self::NamedInvocation(_) => vec![Hazard::Invocation],
            Self::Uplevel(..) => vec![Hazard::ScriptEvaluation],
            Self::Upvar(_) => vec![Hazard::Upvar],
            Self::NamespaceBindings(recipe) => vec![match recipe.kind {
                tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingKind::Variable => Hazard::Variable,
                tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingKind::Global => Hazard::NamespaceUpvar,
                tcl_registry::native_namespace_binding_compilation::NativeNamespaceBindingKind::Upvar => Hazard::Upvar,
            }],
            Self::DictionaryScope(_) => vec![Hazard::ScriptEvaluation],
            Self::Array(recipe) => recipe.compaction_hazards(),
            Self::Coroutine(recipe) => recipe.compaction_hazards(),
            Self::Expression(recipe) => recipe.compaction_hazards(),
            Self::Control(recipe) => recipe.compaction_hazards(),
            Self::Try(recipe) => recipe.compaction_hazards(),
            // Arithmetic stack steps add no compaction hazard. Substitutions in
            // actually visited operands are already collected from self.scripts.
            Self::MathOperator(_) => Vec::new(),
            Self::Scalar(_) | Self::Introspection(_) | Self::ListIndex(_) | Self::ListOperations(_) | Self::StringTrim(_) | Self::StringMatch(_) | Self::Error(_) | Self::DictionaryLookup(_) | Self::DictionaryMutation(_) | Self::Break | Self::Continue | Self::Each(_) | Self::InfoExists(_) | Self::Switch(_) | Self::TclOoHelper(..) | Self::List(..) | Self::Concat{..} | Self::Unset(_) | Self::Load(_) | Self::Store(..) | Self::Increment(..) | Self::Append(..) | Self::SelectedReturn(..) => Vec::new(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enabling_native_limit_preserves_warm_original_bytecode_and_local_table() {
        for (engine, native) in [
            (
                "tcl8.6",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/8.6.18.tsv"
                ),
            ),
            (
                "tcl9.0",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/9.0.4.tsv"
                ),
            ),
            (
                "tcl9.1",
                include_str!(
                    "../../../../../rust/tcl-registry/tests/data/native_compiler_pass/warm_limit/9.1.0.tsv"
                ),
            ),
        ] {
            let native: Vec<_> = native.trim_end().split('\t').collect();
            let mut interp = super::super::tests::interpreter(engine);
            assert_eq!(
                interp.eval_str(b"proc p {} {array set a {k V}}; p"),
                Code::Ok
            );
            let procedure = interp.proc_def(b"p").unwrap();
            let original = procedure.body.checked_ptr().unwrap();
            let before = super::super::cache(original).expect("actual warm procedure Bytecode");
            assert_eq!(
                before.compiled_local_layout().unwrap().names.len(),
                native[3].parse::<usize>().unwrap(),
                "{engine}"
            );
            let option = obj::Owned::fresh(obj::new_string_bytes(b"-value"));
            let value = obj::Owned::fresh(obj::new_string_bytes(b"1000000"));
            let result = interp
                .limit_apply(b"commands", &[option.as_ptr(), value.as_ptr()])
                .expect("actual native command limit receiver");
            drop(obj::Owned::fresh(result));
            let reached = interp
                .capture_body_compiler_pass_environment(procedure.namespace(), Some(&procedure))
                .unwrap();
            assert!(reached.has_enabled_limits());
            assert!(!native_compiler_replays(
                interp.native_invocation_dialect().tcl_version.unwrap(),
                interp.native_command_interpreter,
                Some(&reached),
                [],
            ));
            assert!(
                interp.original_procedure_artifact_is_current(&procedure),
                "{engine}"
            );
            assert_eq!(
                interp.eval_str(b"p").as_int().to_string(),
                native[0],
                "{engine}"
            );
            let after = super::super::cache(original).expect("same warm procedure Bytecode");
            assert_eq!(
                usize::from(Rc::ptr_eq(&before, &after)).to_string(),
                native[5],
                "{engine}"
            );
            assert_eq!(
                after.compiled_local_layout().unwrap().names.len(),
                native[4].parse::<usize>().unwrap(),
                "{engine}"
            );
            assert!(!interp.host_refusal_pending(), "{engine}");
        }
    }

    #[test]
    fn compiler_pass_environment_uses_actual_parent_limits_and_command_owner() {
        let mut interp = super::super::tests::interpreter("tcl8.6");
        assert_eq!(
            interp.eval_str(b"proc ::tcl::pass {} {array set a {k V}}"),
            Code::Ok
        );
        let procedure = interp.proc_def(b"::tcl::pass").unwrap();
        let environment = interp
            .capture_body_compiler_pass_environment(procedure.namespace(), Some(&procedure))
            .unwrap();
        assert!(environment.is_current_for(interp.native_command_interpreter));
        assert!(environment.is_root());
        assert!(!environment.has_enabled_limits());
        assert_eq!(
            environment
                .procedure()
                .unwrap()
                .namespace_full_name
                .as_bytes(),
            b"::tcl"
        );
        assert!(native_compiler_replays(
            tcl_dialect::TclVersion::V8_6,
            interp.native_command_interpreter,
            Some(&environment),
            [Hazard::Invocation]
        ));
        let script_environment = interp
            .capture_body_compiler_pass_environment(procedure.namespace(), None)
            .unwrap();
        assert!(script_environment.procedure().is_none());
        assert!(!native_compiler_replays(
            tcl_dialect::TclVersion::V8_6,
            interp.native_command_interpreter,
            Some(&script_environment),
            [Hazard::Invocation]
        ));
        interp.create_child(Some(b"child".to_vec()));
        let child_environment = interp
            .with_child(b"child", |child| {
                child
                    .capture_body_compiler_pass_environment(GLOBAL, None)
                    .unwrap()
            })
            .unwrap();
        assert!(!child_environment.is_root());
        let retained_child = interp
            .children
            .borrow()
            .get(b"child".as_slice())
            .unwrap()
            .clone();
        let outside_evaluation = retained_child
            .capture_body_compiler_pass_environment(GLOBAL, None)
            .unwrap();
        assert!(!outside_evaluation.is_root());
        assert_eq!(
            interp.eval_str(b"interp limit child commands -value 100"),
            Code::Ok
        );
        let limited_environment = interp
            .with_child(b"child", |child| {
                child
                    .capture_body_compiler_pass_environment(GLOBAL, None)
                    .unwrap()
            })
            .unwrap();
        assert!(limited_environment.has_enabled_limits());
        assert_ne!(child_environment, limited_environment);
        assert_eq!(interp.eval_str(b"rename ::tcl::pass {}"), Code::Ok);
        let retired_command = interp
            .capture_body_compiler_pass_environment(procedure.namespace(), Some(&procedure))
            .unwrap();
        assert!(retired_command.procedure().is_none());
        assert!(!native_compiler_replays(
            tcl_dialect::TclVersion::V8_6,
            interp.native_command_interpreter,
            Some(&retired_command),
            [Hazard::Invocation]
        ));
    }
}
