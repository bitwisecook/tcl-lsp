// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native list-object callback obligations, separate from object mutation.

use crate::native_compilation::{NativeCompilationSelection, SuccessfulHandlerSpec};
use crate::{IntrinsicId, InvocationArguments, InvocationFacts, SemanticOperationId};

/// Native object method whose interpreter-world effects need independent proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeListMethod {
    /// Materialize an index or variable name, possibly through nested strings.
    StringAccess,
    /// Duplicate the object's internal representation.
    Duplicate,
    /// Read the object's length.
    Length,
    /// Select an indexed element.
    Index,
    /// Select a range, potentially changing the receiver's representation/cache.
    Slice,
    /// Materialize an element array, potentially changing the receiver's cache.
    Elements,
}

/// A possible method dispatch on one original frozen argv object.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeListMethodRequirement {
    /// Absolute post-head argv position, retaining member and alias offsets.
    pub argument: usize,
    /// Native method whose interpreter-world effects remain an obligation.
    pub method: NativeListMethod,
}

/// Ordered conservative runtime object-method obligations of a selected native invocation.
/// This grants no completion, result, compiler admission or representation proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeListObjectProtocol {
    /// The selected operation does not access or convert its input object.
    Ordinary,
    /// A stock list conversion can invoke custom string or intrep hooks.
    Conversion {
        /// Conservative original-operand conversion footprint.
        requirements: Vec<NativeListMethodRequirement>,
        /// A grouped index, nested element or preparation route remains unbounded.
        unresolved: bool,
    },
    /// Tcl 9 may invoke native methods on the captured original operands.
    Abstract {
        /// Conservative original-operand method footprint in preparation order.
        /// Alternative native branches need not invoke every listed method.
        requirements: Vec<NativeListMethodRequirement>,
        /// A grouped index, nested element or preparation route remains unbounded.
        unresolved: bool,
    },
    /// Actual engine, argv cardinality or native preparation is unproved.
    Unknown,
}

impl InvocationFacts {
    /// Runtime object-method obligations under the retained compiler selection.
    /// The caller separately proves actual handler identity and current objects;
    /// method closure says nothing about receiver/cache representation changes.
    /// Numeric index positions require independent current object-class proof;
    /// they supply neither contents, index validity nor a conversion guarantee.
    #[must_use]
    pub fn native_list_method_requirements(
        &self,
        arguments: InvocationArguments<'_>,
        selection: NativeCompilationSelection,
        proved_native_numeric_indices: &[usize],
    ) -> Option<NativeListObjectProtocol> {
        use crate::hooks::LoweringHookId;
        let iteration = matches!(
            self.operation,
            SemanticOperationId::StructuredLowering(LoweringHookId::Foreach | LoweringHookId::Lmap)
        );
        let leaf = matches!(
            self.operation,
            SemanticOperationId::Intrinsic(
                IntrinsicId::ListLength | IntrinsicId::ListIndex | IntrinsicId::ListRange
            )
        );
        if (!iteration && !leaf)
            || self.successful_handler
                != Some(if iteration {
                    SuccessfulHandlerSpec::PossibleBodies
                } else {
                    SuccessfulHandlerSpec::Leaf
                })
        {
            return None;
        }
        if self.arity_accepts_frozen_arguments() != Some(true)
            || self.frozen_argument_count != arguments.exact_argv_len()
        {
            return Some(NativeListObjectProtocol::Unknown);
        }
        let Some(abstract_dispatch) = abstract_list_dispatch(arguments.dialect()) else {
            return Some(NativeListObjectProtocol::Unknown);
        };
        let protocol =
            selected_requirements(self, arguments, selection, proved_native_numeric_indices)?;
        Some(match protocol {
            NativeListObjectProtocol::Abstract {
                requirements,
                unresolved,
            } if !abstract_dispatch => NativeListObjectProtocol::Conversion {
                requirements,
                unresolved,
            },
            protocol => protocol,
        })
    }
}

fn abstract_list_dispatch(dialect: Option<crate::InvocationDialect>) -> Option<bool> {
    use tcl_dialect::{
        TclVersion,
        model::{Family, Release},
    };
    let dialect = dialect?;
    match (dialect.family(), dialect.tcl_version) {
        (Some(Family::Tcl), Some(version)) => Some(version >= TclVersion::V9_0),
        (Some(Family::Jim), _)
            if dialect
                .core_point
                .is_some_and(|point| point.release() == Release::JIM_0_84) =>
        {
            Some(false)
        }
        _ => None,
    }
}

fn selected_requirements(
    facts: &InvocationFacts,
    arguments: InvocationArguments<'_>,
    selection: NativeCompilationSelection,
    proved_native_numeric_indices: &[usize],
) -> Option<NativeListObjectProtocol> {
    use NativeListMethod::{Duplicate, Elements, Index, Length, Slice, StringAccess};

    let dialect = arguments.dialect()?;
    let iteration = matches!(
        facts.operation,
        SemanticOperationId::StructuredLowering(
            crate::hooks::LoweringHookId::Foreach | crate::hooks::LoweringHookId::Lmap
        )
    );
    let count = arguments
        .exact_argv_len()?
        .checked_sub(facts.argument_offset)?;
    let mut requirements = Vec::new();
    let mut unresolved = false;
    let operand = facts.argument_offset;
    let mut add = |argument, methods: &[NativeListMethod]| {
        requirements.extend(
            methods
                .iter()
                .map(|&method| NativeListMethodRequirement { argument, method }),
        );
    };
    match facts.operation {
        SemanticOperationId::Intrinsic(IntrinsicId::ListLength) if count == 1 => {
            add(operand, &[Length]);
        }
        SemanticOperationId::Intrinsic(IntrinsicId::ListRange) if count == 3 => {
            for index in operand + 1..operand + 3 {
                if !proved_native_numeric_indices.contains(&index) {
                    add(index, &[StringAccess]);
                }
            }
            add(operand, &[Length, Slice]);
            if !matches!(selection, NativeCompilationSelection::Generic) {
                add(operand, &[Elements]);
            }
        }
        SemanticOperationId::Intrinsic(IntrinsicId::ListIndex) if count >= 1 => {
            if count == 1 {
                return Some(NativeListObjectProtocol::Ordinary);
            }
            for index in operand + 1..operand + count {
                if !proved_native_numeric_indices.contains(&index) {
                    add(index, &[StringAccess]);
                }
            }
            if count == 2 {
                let scalar = proved_native_numeric_indices.contains(&(operand + 1))
                    || arguments.literal_at(operand + 1).is_some_and(|text| {
                        dialect.index_syntax().is_some_and(|syntax| {
                            tcl_cmd_core::index::resolve_opt_in(text, 1, syntax).is_some()
                        })
                    });
                if !scalar {
                    add(operand + 1, &[Duplicate, Elements]);
                    unresolved = arguments
                        .literal_at(operand + 1)
                        .and_then(|text| {
                            tcl_syntax::list::split_list_bytes_in(
                                text.as_bytes(),
                                dialect.word_values.list,
                                dialect.lexer_grammar.escapes,
                            )
                            .ok()
                        })
                        .is_none_or(|elements| elements.len() > 1);
                }
            } else {
                unresolved = true;
            }
            add(operand, &[Length, Index]);
            if !matches!(selection, NativeCompilationSelection::Generic) {
                add(operand, &[Elements]);
            }
        }
        _ if iteration && count >= 3 && count % 2 == 1 => {
            let compiled = matches!(selection, NativeCompilationSelection::Inline {
                    operation, .. } if operation == facts.operation);
            if !compiled && !matches!(selection, NativeCompilationSelection::Generic) {
                unresolved = true;
            }
            for variable in (operand..operand + count - 1).step_by(2) {
                if !compiled {
                    add(variable, &[Duplicate, Length, Elements, StringAccess]);
                }
                add(variable + 1, &[Duplicate, Length, Index, Elements]);
            }
        }
        _ => return Some(NativeListObjectProtocol::Unknown),
    }
    Some(NativeListObjectProtocol::Abstract {
        requirements,
        unresolved,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_compilation::NativeCompilationGuard;
    use crate::{InvocationDialect, InvocationWord, InvocationWords};

    fn selected(
        head: &'static str,
        operands: &[InvocationWord<'_>],
        environment: &str,
    ) -> (InvocationFacts, InvocationDialect) {
        let context = crate::model::ingress::static_context_for(environment);
        let dialect = InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment(environment).unit_profile(),
        );
        let words = InvocationWords::structured(InvocationWord::Literal(head), operands)
            .with_dialect(dialect);
        let facts = context
            .commands()
            .resolve_structured_invocation(words, dialect.authoring_query())
            .resolved()
            .expect("selected native descriptor")
            .facts();
        (facts, dialect)
    }

    fn abstract_requirements(
        protocol: NativeListObjectProtocol,
    ) -> (Vec<NativeListMethodRequirement>, bool) {
        let NativeListObjectProtocol::Abstract {
            requirements,
            unresolved,
        } = protocol
        else {
            panic!("expected selected C9 object protocol");
        };
        (requirements, unresolved)
    }

    #[test]
    fn custom_scalar_conversion_requirements_apply_before_zero_trip_on_every_engine() {
        let operands = [
            InvocationWord::Literal("x"),
            InvocationWord::Dynamic,
            InvocationWord::Literal(""),
        ];
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let (facts, dialect) = selected("foreach", &operands, environment);
            let protocol = facts
                .native_list_method_requirements(
                    InvocationArguments::structured(&operands).with_dialect(dialect),
                    NativeCompilationSelection::Generic,
                    &[],
                )
                .unwrap();
            let (requirements, unresolved) = match protocol {
                NativeListObjectProtocol::Abstract {
                    requirements,
                    unresolved,
                }
                | NativeListObjectProtocol::Conversion {
                    requirements,
                    unresolved,
                } => (requirements, unresolved),
                other => panic!("{environment}: {other:?}"),
            };
            assert!(!unresolved);
            assert!(
                requirements
                    .iter()
                    .any(|item| item.argument == 1 && item.method == NativeListMethod::Length)
            );
            assert!(requirements.iter().any(|item| item.argument == 0 && item.method == NativeListMethod::StringAccess));
        }
    }

    #[test]
    fn native_method_footprint_requires_actual_family_operation_and_cardinality() {
        let operands = [InvocationWord::Dynamic];
        for environment in ["tcl8.4", "tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1", "jim"] {
            let (mut facts, dialect) = selected("llength", &operands, environment);
            let args = InvocationArguments::structured(&operands).with_dialect(dialect);
            let protocol = facts.native_list_method_requirements(
                args,
                NativeCompilationSelection::Generic,
                &[],
            );
            if dialect
                .tcl_version
                .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0)
            {
                assert_eq!(
                    protocol,
                    Some(NativeListObjectProtocol::Abstract {
                        requirements: vec![NativeListMethodRequirement {
                            argument: 0,
                            method: NativeListMethod::Length
                        }],
                        unresolved: false,
                    })
                );
            } else {
                assert_eq!(
                    protocol,
                    Some(NativeListObjectProtocol::Conversion {
                        requirements: vec![NativeListMethodRequirement {
                            argument: 0,
                            method: NativeListMethod::Length
                        }],
                        unresolved: false,
                    })
                );
            }
            assert_eq!(
                facts.native_list_method_requirements(
                    InvocationArguments::structured(&operands),
                    NativeCompilationSelection::Generic,
                    &[]
                ),
                Some(NativeListObjectProtocol::Unknown)
            );
            facts.operation = SemanticOperationId::Invoke;
            assert_eq!(
                facts.native_list_method_requirements(
                    args,
                    NativeCompilationSelection::Generic,
                    &[]
                ),
                None
            );
        }
        let (facts, dialect) = selected("llength", &operands, "tcl9.1");
        let expanded = [InvocationWord::Expanded];
        assert_eq!(
            facts.native_list_method_requirements(
                InvocationArguments::structured(&expanded).with_dialect(dialect),
                NativeCompilationSelection::Generic,
                &[]
            ),
            Some(NativeListObjectProtocol::Unknown)
        );
    }

    #[test]
    fn range_and_index_method_footprints_keep_compiler_and_nested_residuals() {
        use NativeListMethod::{Duplicate, Elements, Index, Length, Slice};
        let operands = [
            InvocationWord::Dynamic,
            InvocationWord::Literal("0"),
            InvocationWord::Literal("1"),
        ];
        let (facts, dialect) = selected("lrange", &operands, "tcl9.1");
        let args = InvocationArguments::structured(&operands).with_dialect(dialect);
        let (generic, unresolved) = abstract_requirements(
            facts
                .native_list_method_requirements(args, NativeCompilationSelection::Generic, &[])
                .unwrap(),
        );
        assert!(!unresolved);
        assert_eq!(
            generic.iter().map(|item| item.method).collect::<Vec<_>>(),
            [
                NativeListMethod::StringAccess,
                NativeListMethod::StringAccess,
                Length,
                Slice
            ]
        );
        let (compiled, _) = abstract_requirements(
            facts
                .native_list_method_requirements(
                    args,
                    NativeCompilationSelection::Inline {
                        operation: facts.operation,
                        guard: NativeCompilationGuard::BeforeArguments,
                    },
                    &[],
                )
                .unwrap(),
        );
        assert!(compiled.iter().any(|item| item.method == Elements));

        for (index, nested) in [
            (InvocationWord::Literal("0"), false),
            (InvocationWord::Literal("0 1"), true),
            (InvocationWord::Dynamic, true),
        ] {
            let values = [InvocationWord::Dynamic, index];
            let (facts, dialect) = selected("lindex", &values, "tcl9.1");
            let (requirements, residual) = abstract_requirements(
                facts
                    .native_list_method_requirements(
                        InvocationArguments::structured(&values).with_dialect(dialect),
                        NativeCompilationSelection::Generic,
                        &[],
                    )
                    .unwrap(),
            );
            assert_eq!(residual, nested);
            assert!(
                requirements
                    .iter()
                    .any(|item| item.argument == 0 && item.method == Index)
            );
            assert_eq!(
                requirements
                    .iter()
                    .any(|item| item.argument == 1 && item.method == Duplicate),
                nested
            );
        }
    }

    #[test]
    fn foreach_runtime_requirements_do_not_fabricate_compiled_variable_list_objects() {
        use NativeListMethod::{Duplicate, Elements, Index, Length};
        let operands = [
            InvocationWord::Literal("x"),
            InvocationWord::Dynamic,
            InvocationWord::Literal(""),
        ];
        let (facts, dialect) = selected("foreach", &operands, "tcl9.1");
        let args = InvocationArguments::structured(&operands).with_dialect(dialect);
        let (generic, residual) = abstract_requirements(
            facts
                .native_list_method_requirements(args, NativeCompilationSelection::Generic, &[])
                .unwrap(),
        );
        assert!(!residual);
        assert_eq!(
            generic
                .iter()
                .filter(|item| item.argument == 0)
                .map(|item| item.method)
                .collect::<Vec<_>>(),
            [Duplicate, Length, Elements, NativeListMethod::StringAccess]
        );
        let (compiled, residual) = abstract_requirements(
            facts
                .native_list_method_requirements(
                    args,
                    NativeCompilationSelection::Inline {
                        operation: facts.operation,
                        guard: NativeCompilationGuard::BeforeArguments,
                    },
                    &[],
                )
                .unwrap(),
        );
        assert!(!residual);
        assert!(compiled.iter().all(|item| item.argument == 1));
        assert_eq!(
            compiled.iter().map(|item| item.method).collect::<Vec<_>>(),
            [Duplicate, Length, Index, Elements]
        );
        let (_, residual) = abstract_requirements(
            facts
                .native_list_method_requirements(args, NativeCompilationSelection::Unknown, &[])
                .unwrap(),
        );
        assert!(residual);
    }

    #[test]
    fn native_numeric_index_object_proof_closes_only_its_own_grouped_path() {
        let operands = [InvocationWord::Dynamic, InvocationWord::Dynamic];
        let (facts, dialect) = selected("lindex", &operands, "tcl9.1");
        let arguments = InvocationArguments::structured(&operands).with_dialect(dialect);
        for (proved, residual) in [(&[1][..], false), (&[0][..], true), (&[][..], true)] {
            let (requirements, unresolved) = abstract_requirements(
                facts
                    .native_list_method_requirements(
                        arguments,
                        NativeCompilationSelection::Generic,
                        proved,
                    )
                    .unwrap(),
            );
            assert_eq!(unresolved, residual);
            assert_eq!(requirements.iter().any(|item| item.argument == 1), residual);
        }
        for method in [
            NativeListMethod::Duplicate,
            NativeListMethod::Length,
            NativeListMethod::Index,
            NativeListMethod::Slice,
            NativeListMethod::Elements,
        ] {
            assert!(
                crate::native_result::NativeListMethodProvider::ArithmeticSequence
                    .world_is_closed_for(method)
            );
        }
    }
}
