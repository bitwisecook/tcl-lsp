// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Authored native result dependencies, separate from completion and purity.

use crate::InvocationArguments;

/// Internal numeric shape produced by a successful native store, before write
/// callbacks. This carries no number, object identity or folding permission.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeNumericStoreProduction {
    /// Increment's selected integer tower produces an integer representation.
    Increment {
        /// Actual engine arithmetic, including its overflow representation.
        arithmetic: tcl_dialect::NativeArithmetic,
    },
}

/// Numeric representation of an actual normal native result, without a value,
/// folding permission, producer-effect closure or object allocation proof.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeNumericResultProduction {
    /// Actual scalar native math result has a double representation.
    Double,
    /// Native count result in the selected engine's integer tower.
    Integer {
        /// Independently selected native arithmetic.
        arithmetic: tcl_dialect::NativeArithmetic,
    },
}

/// Audited native list-method provider on a reached normal result object.
/// This proves only the listed method effects, independently of bytes, list
/// representation, object allocation freshness and the producing call's effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeListMethodProvider {
    /// Native C Tcl 9 arithmetic sequence result: an ordinary empty object or
    /// an arithmetic-series object whose length method reads its stored size.
    ArithmeticSequence,
    /// Actual generic native zero-argument list result. Only root list-method
    /// effects are closed; this is no `StringAccess`, cache or freshness proof.
    EmptyListRoot,
}

impl NativeListMethodProvider {
    /// Interpreter-world effects of an audited method on this current provider.
    /// Receiver/cache mutation and representation lifetime remain independent.
    #[must_use]
    pub const fn world_is_closed_for(
        self,
        method: crate::list_object_methods::NativeListMethod,
    ) -> bool {
        use crate::list_object_methods::NativeListMethod;
        matches!(
            (self, method),
            (
                Self::ArithmeticSequence | Self::EmptyListRoot,
                NativeListMethod::Duplicate
                    | NativeListMethod::Length
                    | NativeListMethod::Index
                    | NativeListMethod::Slice
                    | NativeListMethod::Elements
            )
        )
    }

    /// Whether the provider's native length method has no mutable-world effects.
    /// The original object and current provider lifetime must be proved separately.
    #[must_use]
    pub const fn length_is_read_only(self) -> bool {
        matches!(self, Self::ArithmeticSequence)
    }
}

/// Point at which a variable supplies the native invocation result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum VariableResultPhase {
    /// Successful variable read, after any native read observers have run.
    AfterRead,
    /// Successful store, after write observers and intrinsic hooks have run.
    /// This is not an assertion that a subsequent Tcl read trace executes.
    AfterWrite,
}

impl VariableResultPhase {
    /// Result when the originally selected cell has no contents after observers.
    /// Never re-resolve its spelling: callbacks may have allocated a different cell.
    #[must_use]
    pub fn missing_cell_result(self, dialect: crate::InvocationDialect) -> NativeResultSelection {
        match self {
            Self::AfterRead => NativeResultSelection::InvalidArguments,
            Self::AfterWrite
                if dialect.family() != Some(tcl_dialect::model::Family::Jim)
                    && dialect.tcl_version.is_some() =>
            {
                NativeResultSelection::EmptyString
            }
            Self::AfterWrite => NativeResultSelection::Unknown,
        }
    }
}

/// Result contract of an independently proved native implementation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeResultContract {
    /// Audited scalar math implementation, independently of lookup and effects.
    ScalarMath(crate::mathfunc::NativeScalarMathOperation),
    /// Successful native list length constructs an integer result, independently
    /// of input bytes and native object-method effects.
    ListLength,
    /// Return the originally selected cell's actual contents at the authored phase.
    /// Capture its identity as native variable lookup begins, before observers.
    VariableValue {
        /// Variable-name operand, relative to the selected form/subcommand.
        variable_at: u8,
        /// Observer boundary that precedes the returned value.
        phase: VariableResultPhase,
    },
    /// Return whether the selected cell has defined contents after native read observers.
    VariableExistence {
        /// Variable-name operand, relative to the selected form/subcommand.
        variable_at: u8,
    },
    /// Return a list value retaining each argument as one element.
    /// Internal representation requires the separate construction policy.
    ListArguments {
        /// First retained operand, relative to the selected form/subcommand.
        from: u8,
    },
    /// Capture one script as a namespace-scoped command prefix on normal return.
    /// This identifies an input dependency, not result bytes or future entry.
    NamespaceCommandPrefix,
    /// Return a selected list range; its representation needs independently
    /// proved ordinary input and a nonempty native selection.
    ListRange {
        /// List operand relative to the selected form.
        list_at: u8,
        /// First-index operand relative to the selected form.
        first_at: u8,
        /// Last-index operand relative to the selected form.
        last_at: u8,
    },
    /// Construct a dictionary from retained key/value operands. Duplicate keys
    /// keep their first position and last value through the shared dict owner.
    DictionaryArguments {
        /// First key operand, relative to the selected form/subcommand.
        from: u8,
    },
    /// Return an existing dictionary value selected by a nonempty key path.
    /// This does not promise a new object or a representation for that value.
    DictionaryValue {
        /// Dictionary operand, relative to the selected subcommand.
        dictionary_at: u8,
        /// First key operand, relative to the selected subcommand.
        keys_from: u8,
    },
    /// Every successful handler continuation returns empty bytes, independently
    /// of argv content. This does not promise a fresh string object representation.
    EmptyString,
    /// Select the result using the shared dialect-sensitive return grammar.
    ReturnResult,
    /// Successful native C Tcl 9 sequence construction returns an ordinary
    /// empty object or the audited arithmetic-series list-method provider.
    /// Operand conversion remains a separate effect obligation.
    ArithmeticSequence,
    /// Increment publishes a numeric object into its captured receiver before
    /// write callbacks. The eventual returned value needs separate proof.
    IncrementStore,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoped_prefix_capture_retains_only_the_exact_native_input_dependency() {
        let contract = NativeResultContract::NamespaceCommandPrefix;
        let operands = [
            crate::InvocationWord::Literal("code"),
            crate::InvocationWord::Dynamic,
        ];
        for version in tcl_dialect::TclVersion::ALL {
            let args = InvocationArguments::structured(&operands)
                .with_dialect(crate::InvocationDialect::for_version(version));
            assert_eq!(contract.normal_scoped_prefix_operand(args, 1), Some(1));
            assert_eq!(contract.normal_scoped_prefix_operand(args, 0), None);
            assert_eq!(contract.select(args, 1), NativeResultSelection::Unknown);
        }
        assert_eq!(
            contract.normal_scoped_prefix_operand(InvocationArguments::structured(&operands), 1),
            None
        );
        let jim = crate::model::ingress::static_context_for("jim");
        let args = InvocationArguments::structured(&operands).with_dialect(
            crate::InvocationDialect::of_profile(jim.commands().profile().unwrap()),
        );
        assert_eq!(contract.normal_scoped_prefix_operand(args, 1), None);
        let expanded = [
            crate::InvocationWord::Literal("code"),
            crate::InvocationWord::Expanded,
        ];
        let args = InvocationArguments::structured(&expanded).with_dialect(
            crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1),
        );
        assert_eq!(contract.normal_scoped_prefix_operand(args, 1), None);
    }

    #[test]
    fn list_intrinsic_code_retains_unknown_effects_and_requires_native_cardinality() {
        // Source-inspection proof: naming.list.constructor-intrinsic-return-code
        // docs/design/analysis/name-resolution-proofs/list-constructor-intrinsic-return-code.md
        let contract = NativeResultContract::ListArguments { from: 0 };
        let operands = [
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::KnownBytes(b"\xff\0"),
        ];
        let mut dialects = tcl_dialect::TclVersion::ALL
            .map(crate::InvocationDialect::for_version)
            .to_vec();
        dialects.push(crate::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        ));
        for dialect in dialects {
            for words in [&[][..], operands.as_slice()] {
                let args = InvocationArguments::structured(words).with_dialect(dialect);
                assert_eq!(
                    contract.list_constructor_completion_code(args, 0),
                    Some(crate::completion::CompletionCode::Ok)
                );
                assert_eq!(contract.list_constructor_completion_code(args, 1), None);
                assert_eq!(
                    NativeResultContract::ListArguments { from: 1 }
                        .list_constructor_completion_code(args, 0),
                    None
                );
            }
            for words in [
                [crate::InvocationWord::Expanded],
                [crate::InvocationWord::Opaque],
            ] {
                let args = InvocationArguments::structured(&words).with_dialect(dialect);
                assert_eq!(contract.list_constructor_completion_code(args, 0), None);
            }
        }
        assert_eq!(
            contract
                .list_constructor_completion_code(InvocationArguments::structured(&operands), 0),
            None
        );
        let hosted = crate::model::ingress::static_context_for("f5-irules").commands();
        let args = InvocationArguments::structured(&operands).with_dialect(
            crate::InvocationDialect::of_profile(hosted.profile().unwrap()),
        );
        assert_eq!(contract.list_constructor_completion_code(args, 0), None);
    }

    #[test]
    fn arithmetic_sequence_provider_requires_actual_native_result_axes() {
        let contract = NativeResultContract::ArithmeticSequence;
        let operands = [crate::InvocationWord::Dynamic];
        for version in tcl_dialect::TclVersion::ALL {
            let args = InvocationArguments::structured(&operands)
                .with_dialect(crate::InvocationDialect::for_version(version));
            assert_eq!(
                contract.normal_list_method_provider(args, 0),
                (version >= tcl_dialect::TclVersion::V9_0)
                    .then_some(NativeListMethodProvider::ArithmeticSequence)
            );
            assert_eq!(contract.select(args, 0), NativeResultSelection::Unknown);
            assert_eq!(contract.normal_list_method_provider(args, 1), None);
        }
        let jim = InvocationArguments::structured(&operands).with_dialect(
            crate::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
                tcl_dialect::model::Release::JIM_0_84,
            )),
        );
        assert_eq!(contract.normal_list_method_provider(jim, 0), None);
        assert_eq!(
            contract.normal_list_method_provider(InvocationArguments::structured(&operands), 0),
            None
        );
        assert_eq!(
            NativeResultContract::ListArguments { from: 0 }.normal_list_method_provider(jim, 0),
            None
        );
        assert!(NativeListMethodProvider::ArithmeticSequence.length_is_read_only());
    }

    #[test]
    fn increment_store_shape_requires_selected_arithmetic_without_result_donation() {
        use tcl_dialect::{NativeArithmetic, TclVersion};
        let contract = NativeResultContract::IncrementStore;
        let words = [
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Dynamic,
        ];
        for (dialect, arithmetic) in [
            (
                crate::InvocationDialect::for_version(TclVersion::V8_4),
                NativeArithmetic::Tcl84Wide,
            ),
            (
                crate::InvocationDialect::for_version(TclVersion::V8_6),
                NativeArithmetic::TclBignum,
            ),
            (
                crate::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
                    tcl_dialect::model::Release::JIM_0_84,
                )),
                NativeArithmetic::JimWide,
            ),
        ] {
            let args = InvocationArguments::structured(&words).with_dialect(dialect);
            assert_eq!(
                contract.normal_numeric_store_production(args, 0),
                Some(NativeNumericStoreProduction::Increment { arithmetic })
            );
            assert_eq!(contract.select(args, 0), NativeResultSelection::Unknown);
            assert_eq!(
                contract.normal_numeric_store_production(
                    InvocationArguments::structured(&words[..1]).with_dialect(dialect),
                    0,
                ),
                Some(NativeNumericStoreProduction::Increment { arithmetic })
            );
            assert_eq!(
                contract.normal_numeric_store_production(
                    InvocationArguments::structured(&[crate::InvocationWord::Expanded])
                        .with_dialect(dialect),
                    0,
                ),
                None
            );
            assert_eq!(
                NativeResultContract::ListArguments { from: 0 }
                    .normal_numeric_store_production(args, 0),
                None
            );
            for literals in [Vec::new(), vec!["x", "1", "extra"]] {
                assert_eq!(
                    contract.normal_numeric_store_production(
                        InvocationArguments::literals(&literals).with_dialect(dialect),
                        0,
                    ),
                    None
                );
            }
        }
        assert_eq!(
            contract.normal_numeric_store_production(InvocationArguments::structured(&words), 0),
            None
        );
    }

    #[test]
    fn native_range_shape_requires_known_nonempty_ordinary_selection() {
        use crate::representation::OrdinaryContainerRepresentation as Ordinary;
        use tcl_syntax::value::ValueRepresentation;
        let contract = NativeResultContract::ListRange {
            list_at: 0,
            first_at: 1,
            last_at: 2,
        };
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            let args = InvocationArguments::literals(&["prefix", "a b c", "0", "end"])
                .with_dialect(dialect);
            let selected = contract.select(args, 1);
            assert_eq!(
                selected,
                NativeResultSelection::ListRange {
                    list_at: 1,
                    first_at: 2,
                    last_at: 3,
                }
            );
            assert_eq!(
                selected.ordinary_list_range_representation(args, Ordinary::List, 3),
                Some(ValueRepresentation::List)
            );
            assert_eq!(
                selected.ordinary_list_range_representation(args, Ordinary::Dictionary, 0),
                None
            );
            assert_eq!(
                selected.created_representation(
                    dialect,
                    crate::native_compilation::NativeCompilationContext::default(),
                ),
                ValueRepresentation::Unknown
            );
            let empty = InvocationArguments::literals(&["prefix", "a b c", "3", "end"])
                .with_dialect(dialect);
            assert_eq!(
                selected.ordinary_list_range_representation(empty, Ordinary::List, 3),
                None
            );
            let dynamic = [
                crate::InvocationWord::Literal("a b c"),
                crate::InvocationWord::Dynamic,
                crate::InvocationWord::Literal("end"),
            ];
            let unknown = InvocationArguments::structured(&dynamic).with_dialect(dialect);
            assert_eq!(
                contract
                    .select(unknown, 0)
                    .ordinary_list_range_representation(unknown, Ordinary::List, 3),
                None
            );
        }
        let jim = crate::InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        let jim_args = InvocationArguments::literals(&["a b c", "0", "end"]).with_dialect(jim);
        assert_eq!(
            contract
                .select(jim_args, 0)
                .ordinary_list_range_representation(jim_args, Ordinary::List, 3),
            Some(ValueRepresentation::List)
        );
        let args = InvocationArguments::literals(&["a b c", "0", "end"]);
        assert_eq!(
            contract
                .select(args, 0)
                .ordinary_list_range_representation(args, Ordinary::List, 3),
            None
        );
        assert_eq!(
            contract.select(InvocationArguments::literals(&["a b c", "0"]), 0),
            NativeResultSelection::InvalidArguments
        );
        let expanded = [crate::InvocationWord::Expanded];
        assert_eq!(
            contract.select(InvocationArguments::structured(&expanded), 0),
            NativeResultSelection::Unknown
        );
    }

    #[test]
    fn ordinary_range_bytes_require_selected_native_serialization() {
        use crate::representation::OrdinaryContainerRepresentation as Ordinary;
        let contract = NativeResultContract::ListRange {
            list_at: 0,
            first_at: 1,
            last_at: 2,
        };
        let mut dialects = tcl_dialect::TclVersion::ALL
            .into_iter()
            .map(crate::InvocationDialect::for_version)
            .collect::<Vec<_>>();
        dialects.push(crate::InvocationDialect::of_point(
            tcl_dialect::model::DialectPoint::canonical(tcl_dialect::model::Release::JIM_0_84),
        ));
        for dialect in dialects {
            for (list, first, last, length, expected) in [
                ("  a   b  c  ", "0", "end", 3, Some("a b c")),
                ("  a   b  c  ", "-100", "100", 3, Some("a b c")),
                ("\"a\" {b} c", "0", "1", 3, Some("a b")),
                ("7", "0", "1", 1, Some("7")),
                ("a b c", "3", "end", 3, None),
                ("a b c", "0", "1", 4, None),
                (
                    "x #value end",
                    "1",
                    "1",
                    3,
                    Some(
                        if dialect.tcl_version == Some(tcl_dialect::TclVersion::V8_4) {
                            "#value"
                        } else {
                            "{#value}"
                        },
                    ),
                ),
                ("x {a b} end", "1", "1", 3, Some("{a b}")),
                ("x é end", "1", "1", 3, Some("é")),
            ] {
                let literals = [list, first, last];
                let arguments = InvocationArguments::literals(&literals).with_dialect(dialect);
                assert_eq!(
                    contract
                        .select(arguments, 0)
                        .ordinary_range_literal_result(arguments, Ordinary::List, length,)
                        .as_deref(),
                    expected,
                    "{dialect:?}: {literals:?}"
                );
            }
        }
        let arguments = InvocationArguments::literals(&["a b", "0", "1"]);
        assert_eq!(
            contract.select(arguments, 0).ordinary_range_literal_result(
                arguments,
                Ordinary::List,
                2
            ),
            None
        );
        let words = [
            crate::InvocationWord::Dynamic,
            crate::InvocationWord::Literal("0"),
            crate::InvocationWord::Literal("1"),
        ];
        let arguments = InvocationArguments::structured(&words).with_dialect(
            crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
        );
        assert_eq!(
            contract.select(arguments, 0).ordinary_range_literal_result(
                arguments,
                Ordinary::List,
                2
            ),
            None
        );
    }

    #[test]
    fn list_construction_keeps_literal_pool_reuse_separate_from_native_list() {
        use crate::native_compilation::{
            NativeCompilationContext, NativeCompilationFrame, NativeCompilationGuard,
            NativeCompilationMode, NativeCompilationSelection as Selection,
            NativeCompilationWordShape as Shape,
        };
        use ListResultConstruction as Construction;
        let context = NativeCompilationContext {
            mode: NativeCompilationMode::BytecodeObject,
            frame: NativeCompilationFrame::ProcedureCode,
            ..Default::default()
        };
        let selected = Selection::Inline {
            operation: crate::SemanticOperationId::Invoke,
            guard: NativeCompilationGuard::BeforeArguments,
        };
        let result = NativeResultSelection::ListArguments { from: 0, len: 2 };
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            let literal = result.list_construction(
                dialect,
                context,
                selected,
                &[Shape::Literal, Shape::BackslashLiteral],
            );
            assert_eq!(
                literal,
                if version < tcl_dialect::TclVersion::V8_6 {
                    Construction::Fresh
                } else {
                    Construction::SharedLiteral
                }
            );
            assert_eq!(
                result.list_construction(
                    dialect,
                    context,
                    selected,
                    &[Shape::Substituted, Shape::Literal]
                ),
                Construction::Fresh
            );
            assert_eq!(
                result.list_construction(dialect, context, Selection::Generic, &[]),
                Construction::Fresh
            );
            for shape in [Shape::Expanded, Shape::Opaque] {
                assert_eq!(
                    result.list_construction(dialect, context, selected, &[shape, Shape::Literal]),
                    Construction::Unknown
                );
            }
            assert_eq!(
                result.list_construction(
                    dialect,
                    context,
                    Selection::Unknown,
                    &[Shape::Substituted, Shape::Literal]
                ),
                Construction::Unknown
            );
        }
    }

    #[test]
    fn direct_list_construction_requires_the_actual_native_family() {
        use tcl_syntax::value::ValueRepresentation;
        let result = NativeResultSelection::ListArguments { from: 0, len: 1 };
        let context = crate::native_compilation::NativeCompilationContext {
            mode: crate::native_compilation::NativeCompilationMode::Direct,
            ..Default::default()
        };
        let actual = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        assert_eq!(
            result.created_representation(actual, context),
            ValueRepresentation::List
        );
        for family in [None, Some(tcl_dialect::model::Family::F5Irules)] {
            let assist = crate::InvocationDialect {
                core_point: None,
                native_family: family,
                ..actual
            };
            assert_eq!(
                result.created_representation(assist, context),
                ValueRepresentation::Unknown
            );
            assert_eq!(
                result.list_construction(
                    assist,
                    context,
                    crate::native_compilation::NativeCompilationSelection::Generic,
                    &[],
                ),
                ListResultConstruction::Unknown
            );
        }
    }

    #[test]
    fn dictionary_value_result_uses_native_keys_without_representation_donation() {
        let contract = NativeResultContract::DictionaryValue {
            dictionary_at: 0,
            keys_from: 1,
        };
        let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        for (words, expected) in [
            (
                vec!["get", "nested {y OLD y NEW}", "nested", "y"],
                Some("NEW"),
            ),
            (vec!["get", "nested {y NEW}", "nested", "missing"], None),
            (vec!["get", "nested {y}", "nested", "y"], None),
        ] {
            let arguments = InvocationArguments::literals(&words).with_dialect(dialect);
            let selection = contract.select(arguments, 1);
            assert_eq!(
                selection.dictionary_literal_result(arguments).as_deref(),
                expected
            );
            assert_eq!(
                selection.created_representation(
                    dialect,
                    crate::native_compilation::NativeCompilationContext::default(),
                ),
                tcl_syntax::value::ValueRepresentation::Unknown
            );
        }
        assert_eq!(
            contract.select(InvocationArguments::literals(&["get", "x 1"]), 1),
            NativeResultSelection::Unknown
        );
        let dynamic = [
            crate::InvocationWord::Literal("get"),
            crate::InvocationWord::Literal("x 1"),
            crate::InvocationWord::Dynamic,
        ];
        let arguments = InvocationArguments::structured(&dynamic).with_dialect(dialect);
        assert!(
            contract
                .select(arguments, 1)
                .dictionary_literal_result(arguments)
                .is_none()
        );
        let unexpanded = [
            crate::InvocationWord::Literal("get"),
            crate::InvocationWord::Expanded,
        ];
        assert_eq!(
            contract.select(InvocationArguments::structured(&unexpanded), 1),
            NativeResultSelection::Unknown
        );
        let without_dialect = InvocationArguments::literals(&["get", "x 1", "x"]);
        assert!(
            contract
                .select(without_dialect, 1)
                .dictionary_literal_result(without_dialect)
                .is_none()
        );
    }

    #[test]
    fn constant_success_result_does_not_invent_completion_or_object_representation() {
        let arguments = [crate::InvocationWord::Expanded];
        assert_eq!(
            NativeResultContract::EmptyString
                .select(InvocationArguments::structured(&arguments), 0),
            NativeResultSelection::EmptyString
        );
        assert_eq!(
            NativeResultSelection::EmptyString.created_representation(
                crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6),
                crate::native_compilation::NativeCompilationContext::default()
            ),
            tcl_syntax::value::ValueRepresentation::Unknown
        );
    }

    #[test]
    fn return_result_positions_share_native_option_validation() {
        for version in tcl_dialect::TclVersion::ALL {
            let dialect = crate::InvocationDialect::for_version(version);
            for (words, expected) in [
                (vec![], NativeResultSelection::EmptyString),
                (vec!["value"], NativeResultSelection::Argument(0)),
                (vec!["-code"], NativeResultSelection::Argument(0)),
                (vec!["-code", "ok"], NativeResultSelection::EmptyString),
                (
                    vec!["-code", "ok", "value"],
                    NativeResultSelection::Argument(2),
                ),
                (vec!["one", "two"], NativeResultSelection::InvalidArguments),
                (
                    vec!["-code", "wrong", "value"],
                    NativeResultSelection::InvalidArguments,
                ),
            ] {
                let args = InvocationArguments::literals(&words).with_dialect(dialect);
                assert_eq!(
                    NativeResultContract::ReturnResult.select(args, 0),
                    expected,
                    "{version:?}: {words:?}"
                );
            }
        }
        assert_eq!(
            NativeResultContract::ReturnResult.select(InvocationArguments::literals(&["value"]), 0),
            NativeResultSelection::Unknown
        );
    }

    #[test]
    fn existence_result_retains_subcommand_offset_and_exact_arity() {
        let contract = NativeResultContract::VariableExistence { variable_at: 0 };
        assert_eq!(
            contract.select(InvocationArguments::literals(&["exists", "x"]), 1),
            NativeResultSelection::VariableExistence { variable_at: 1 }
        );
        for words in [&["exists"][..], &["exists", "x", "y"][..]] {
            assert_eq!(
                contract.select(InvocationArguments::literals(words), 1),
                NativeResultSelection::InvalidArguments
            );
        }
    }

    #[test]
    fn dictionary_results_keep_offsets_duplicate_keys_and_unknown_values() {
        let words = ["create", "a", "1", "b", "2", "a", "3"];
        let arguments = InvocationArguments::literals(&words);
        let contract = NativeResultContract::DictionaryArguments { from: 0 };
        let selected = contract.select(arguments, 1);
        assert_eq!(
            selected,
            NativeResultSelection::DictionaryArguments { from: 1, len: 6 }
        );
        assert_eq!(
            selected.dictionary_literal_result(arguments).as_deref(),
            Some("a 3 b 2")
        );
        assert_eq!(
            contract.select(InvocationArguments::literals(&["create", "a"]), 1),
            NativeResultSelection::InvalidArguments
        );
        let unknown = [
            crate::InvocationWord::Literal("a"),
            crate::InvocationWord::Dynamic,
        ];
        let arguments = InvocationArguments::structured(&unknown);
        assert!(
            contract
                .select(arguments, 0)
                .dictionary_literal_result(arguments)
                .is_none()
        );
        let expanded = [crate::InvocationWord::Expanded];
        assert_eq!(
            contract.select(InvocationArguments::structured(&expanded), 0),
            NativeResultSelection::Unknown
        );
    }

    #[test]
    fn list_result_keeps_structural_offsets_and_missing_cell_policy_is_explicit() {
        let args = InvocationArguments::literals(&["subcommand", "prefix", "value"]);
        assert_eq!(
            NativeResultContract::ListArguments { from: 1 }.select(args, 1),
            NativeResultSelection::ListArguments { from: 2, len: 1 }
        );
        let dialect = crate::InvocationDialect::for_version(tcl_dialect::TclVersion::V8_6);
        assert_eq!(
            VariableResultPhase::AfterWrite.missing_cell_result(dialect),
            NativeResultSelection::EmptyString
        );
        assert_eq!(
            VariableResultPhase::AfterRead.missing_cell_result(dialect),
            NativeResultSelection::InvalidArguments
        );
    }
}

/// Structural result dependency after the exact native argv shape is selected.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeResultSelection {
    /// Contents of a cell after its actual observer boundary.
    VariableValue {
        /// Effective post-head variable-name index.
        variable_at: usize,
        /// Point at which to consult the cell state.
        phase: VariableResultPhase,
    },
    /// Defined contents of the originally selected cell after read observers.
    VariableExistence {
        /// Effective post-head variable-name index.
        variable_at: usize,
    },
    /// A native list containing a contiguous range of retained argument values.
    ListArguments {
        /// Effective post-head first element index.
        from: usize,
        /// Number of retained element values.
        len: usize,
    },
    /// A selected native list range, without an unconditional result shape.
    ListRange {
        /// Effective post-head list operand index.
        list_at: usize,
        /// Effective post-head first-index operand index.
        first_at: usize,
        /// Effective post-head last-index operand index.
        last_at: usize,
    },
    /// A native dictionary retaining a contiguous even key/value operand range.
    DictionaryArguments {
        /// Effective post-head first key index.
        from: usize,
        /// Number of retained key/value operands.
        len: usize,
    },
    /// An existing dictionary value selected using the native list grammar.
    DictionaryValue {
        /// Effective post-head dictionary operand index.
        dictionary_at: usize,
        /// Effective post-head first key index.
        keys_from: usize,
        /// Number of selected key operands.
        keys_len: usize,
    },
    /// An existing operand value, retaining its representation.
    Argument(usize),
    /// The command's omitted result is the empty string.
    EmptyString,
    /// Native argument validation fails; no successful result is promised.
    InvalidArguments,
    /// Argument shape or native grammar remains unresolved.
    Unknown,
}

/// Object construction selected by the native list compiler or handler.
/// This proves a result representation, never private element object identities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListResultConstruction {
    /// The reached operation creates a list object for this result.
    Fresh,
    /// The compiler pushes a reusable literal whose representation may have changed.
    SharedLiteral,
    /// Original operands or selected construction protocol are not proved.
    Unknown,
}

impl NativeResultSelection {
    /// Representation proved by native result construction alone.
    /// Variable/argument results retain their input evidence instead. Compiled
    /// C constant lists may reuse interned literal objects, and empty C lists
    /// may reuse a shared empty object, so neither promises a list representation.
    #[must_use]
    pub fn created_representation(
        self,
        dialect: crate::InvocationDialect,
        compilation: crate::native_compilation::NativeCompilationContext,
    ) -> tcl_syntax::value::ValueRepresentation {
        use tcl_syntax::value::ValueRepresentation;
        if matches!(self, Self::DictionaryArguments { .. }) {
            return if dialect.core_point.is_some_and(|point| {
                point.family() == tcl_dialect::model::Family::Jim
                    && point.release() == tcl_dialect::model::Release::JIM_0_84
            }) || (dialect.family() == Some(tcl_dialect::model::Family::Tcl)
                && dialect
                    .tcl_version
                    .is_some_and(|version| version >= tcl_dialect::TclVersion::V8_5))
            {
                ValueRepresentation::Dict
            } else {
                ValueRepresentation::Unknown
            };
        }
        let Self::ListArguments { len, .. } = self else {
            return ValueRepresentation::Unknown;
        };
        if dialect.core_point.is_some_and(|point| {
            point.family() == tcl_dialect::model::Family::Jim
                && point.release() == tcl_dialect::model::Release::JIM_0_84
        }) || (len > 0
            && dialect.family() == Some(tcl_dialect::model::Family::Tcl)
            && dialect.tcl_version.is_some()
            && compilation.mode == crate::native_compilation::NativeCompilationMode::Direct)
        {
            ValueRepresentation::List
        } else {
            ValueRepresentation::Unknown
        }
    }

    /// Select list construction from original compiler operands, not frozen argv bytes.
    /// `TclCompileListCmd` in C8.4/8.5 emits LIST for nonempty operands;
    /// C8.6–9.1 pools compile-time-known lists and emits LIST for ordinary
    /// dynamic operands. Expanded recipes remain unknown here. A Generic
    /// selected native handler independently creates its nonempty result.
    #[must_use]
    pub fn list_construction(
        self,
        dialect: crate::InvocationDialect,
        compilation: crate::native_compilation::NativeCompilationContext,
        selected: crate::native_compilation::NativeCompilationSelection,
        shapes: &[crate::native_compilation::NativeCompilationWordShape],
    ) -> ListResultConstruction {
        use crate::native_compilation::{
            NativeCompilationSelection as Selection, NativeCompilationWordShape as Shape,
        };
        use ListResultConstruction as Construction;
        let Self::ListArguments { from: 0, len } = self else {
            return Construction::Unknown;
        };
        if self.created_representation(dialect, compilation)
            == tcl_syntax::value::ValueRepresentation::List
        {
            return Construction::Fresh;
        }
        let Some(version) = dialect
            .tcl_version
            .filter(|_| dialect.family() == Some(tcl_dialect::model::Family::Tcl))
        else {
            return Construction::Unknown;
        };
        if len == 0 {
            return Construction::Unknown;
        }
        match selected {
            Selection::Generic => Construction::Fresh,
            Selection::Inline {
                operation: crate::SemanticOperationId::Invoke,
                ..
            } if compilation.mode
                == crate::native_compilation::NativeCompilationMode::BytecodeObject
                && shapes.len() == len
                && !shapes
                    .iter()
                    .any(|shape| matches!(shape, Shape::Opaque | Shape::Expanded)) =>
            {
                if version < tcl_dialect::TclVersion::V8_6 || shapes.contains(&Shape::Substituted) {
                    Construction::Fresh
                } else {
                    Construction::SharedLiteral
                }
            }
            _ => Construction::Unknown,
        }
    }

    /// Nonempty native range shape for an independently proved ordinary input.
    /// `length` must be its captured actual native list length. Abstract-list
    /// objects, semantic types, unknown indices and empty results give no proof.
    /// This promises neither a fresh object nor an unshared result.
    #[must_use]
    pub fn ordinary_list_range_representation(
        self,
        arguments: InvocationArguments<'_>,
        input: crate::representation::OrdinaryContainerRepresentation,
        length: usize,
    ) -> Option<tcl_syntax::value::ValueRepresentation> {
        self.ordinary_range_selection(arguments, input, length)?;
        Some(tcl_syntax::value::ValueRepresentation::List)
    }

    /// Exact normal range bytes under a bounded native serialization recipe.
    /// The caller separately proves a current original ordinary input before
    /// coercion and publishes only after the actual native normal successor.
    /// The selected native list-object renderer owns quoting and spacing.
    /// Unrepresented byte output declines the Unicode projection. This supplies neither freshness
    /// nor object identity, representation or erasure permission.
    #[must_use]
    pub fn ordinary_range_literal_result(
        self,
        arguments: InvocationArguments<'_>,
        input: crate::representation::OrdinaryContainerRepresentation,
        length: usize,
    ) -> Option<String> {
        let (first, last) = self.ordinary_range_selection(arguments, input, length)?;
        let Self::ListRange { list_at, .. } = self else {
            return None;
        };
        let dialect = arguments.dialect()?;
        let elements = tcl_syntax::list::split_list_bytes_in(
            arguments.literal_at(list_at)?.as_bytes(),
            dialect.word_values.list,
            dialect.lexer_grammar.escapes,
        )
        .ok()?;
        if elements.len() != length {
            return None;
        }
        let selected = &elements[first..=last];
        let bytes = dialect.list_result_serialization()?.render(selected);
        String::from_utf8(bytes).ok()
    }

    /// Selected range bytes for a constant-folder's ordinary-list model.
    /// Successful parsing and rendering supply values only; actual object
    /// conversion, callbacks, completion and erasure require separate proof.
    #[must_use]
    pub fn constant_range_literal_result(
        self,
        arguments: InvocationArguments<'_>,
    ) -> Option<String> {
        let Self::ListRange { list_at, .. } = self else {
            return None;
        };
        let dialect = arguments.dialect()?;
        let elements = tcl_syntax::list::split_list_bytes_in(
            arguments.literal_at(list_at)?.as_bytes(),
            dialect.word_values.list,
            dialect.lexer_grammar.escapes,
        )
        .ok()?;
        let (first, last) = self.ordinary_range_indices(
            arguments,
            crate::representation::OrdinaryContainerRepresentation::List,
            elements.len(),
        )?;
        let Some((first, last)) = crate::const_fold::clamp_range(first, last, elements.len())
        else {
            return Some(String::new());
        };
        String::from_utf8(
            dialect
                .list_result_serialization()?
                .render(&elements[first..=last]),
        )
        .ok()
    }

    fn ordinary_range_selection(
        self,
        arguments: InvocationArguments<'_>,
        input: crate::representation::OrdinaryContainerRepresentation,
        length: usize,
    ) -> Option<(usize, usize)> {
        let (first, last) = self.ordinary_range_indices(arguments, input, length)?;
        crate::const_fold::clamp_range(first, last, length)
    }

    fn ordinary_range_indices(
        self,
        arguments: InvocationArguments<'_>,
        input: crate::representation::OrdinaryContainerRepresentation,
        length: usize,
    ) -> Option<(i64, i64)> {
        let Self::ListRange {
            list_at,
            first_at,
            last_at,
        } = self
        else {
            return None;
        };
        if !(list_at < first_at && first_at < last_at)
            || last_at.checked_add(1) != arguments.exact_argv_len()
        {
            return None;
        }
        let dialect = arguments.dialect()?;
        match dialect.family()? {
            tcl_dialect::model::Family::Tcl if dialect.tcl_version.is_some() => {
                if input == crate::representation::OrdinaryContainerRepresentation::Dictionary
                    && dialect.tcl_version == Some(tcl_dialect::TclVersion::V8_4)
                {
                    return None;
                }
            }
            tcl_dialect::model::Family::Jim
                if dialect.core_point.is_some_and(|point| {
                    point.release() == tcl_dialect::model::Release::JIM_0_84
                }) => {}
            _ => return None,
        }
        let syntax = dialect.index_syntax()?;
        let first =
            tcl_cmd_core::index::resolve_opt_in(arguments.literal_at(first_at)?, length, syntax)?;
        let last =
            tcl_cmd_core::index::resolve_opt_in(arguments.literal_at(last_at)?, length, syntax)?;
        Some((first, last))
    }

    /// Exact constructor bytes when every selected dictionary operand already
    /// has proved bytes. Unknown value slots are never replaced by empty text.
    #[must_use]
    pub fn dictionary_literal_result(self, arguments: InvocationArguments<'_>) -> Option<String> {
        match self {
            Self::DictionaryArguments { from, len } => {
                let operands = (from..from.checked_add(len)?)
                    .map(|index| arguments.literal_at(index))
                    .collect::<Option<Vec<_>>>()?;
                crate::const_fold::fold_dict_create(&operands)
            }
            Self::DictionaryValue {
                dictionary_at,
                keys_from,
                keys_len,
            } => {
                let dialect = arguments.dialect()?;
                let rules =
                    tcl_syntax::word_rules::WordValueRules::from_grammar(&dialect.lexer_grammar);
                let mut value = arguments.literal_at(dictionary_at)?.to_owned();
                for index in keys_from..keys_from.checked_add(keys_len)? {
                    let key = arguments.literal_at(index)?;
                    let elements = rules.split_list(&value).ok()?;
                    if !elements.len().is_multiple_of(2) {
                        return None;
                    }
                    let slots = tcl_syntax::value::canonical_dict_slots(
                        elements.iter().step_by(2).map(AsRef::as_ref),
                    );
                    let (_, selected) = slots
                        .into_iter()
                        .find(|(candidate, _)| elements[candidate * 2].as_ref() == key)?;
                    value = elements[selected * 2 + 1].to_string();
                }
                Some(value)
            }
            _ => None,
        }
    }
}

impl NativeResultContract {
    /// Intrinsic code of the selected stock list constructor after argv
    /// evaluation. Pointer retention and result publication return OK in the
    /// selected C/Jim handler. This grants no release-effect closure, observer
    /// closure, successful entry, Normal certificate or list serialization.
    #[must_use]
    pub fn list_constructor_completion_code(
        self,
        args: InvocationArguments<'_>,
        argument_offset: usize,
    ) -> Option<crate::completion::CompletionCode> {
        // Source-inspection proof: naming.list.constructor-intrinsic-return-code
        // docs/design/analysis/name-resolution-proofs/list-constructor-intrinsic-return-code.md
        if self != (Self::ListArguments { from: 0 }) || argument_offset != 0 {
            return None;
        }
        args.exact_argv_len()?;
        args.dialect()?.native_string_protocol()?;
        Some(crate::completion::CompletionCode::Ok)
    }

    /// Original operand captured by the selected native namespace wrapper.
    /// Actual namespace/receiver ownership and normal completion are separate
    /// prerequisites; this grants no callback execution or visibility proof.
    #[must_use]
    pub fn normal_scoped_prefix_operand(
        self,
        args: InvocationArguments<'_>,
        argument_offset: usize,
    ) -> Option<usize> {
        (self == Self::NamespaceCommandPrefix
            && args.dialect()?.family() == Some(tcl_dialect::model::Family::Tcl)
            && args.dialect()?.tcl_version.is_some()
            && argument_offset.checked_add(1) == args.exact_argv_len())
        .then_some(argument_offset)
    }

    /// Representation of the actual normal result. Native identity and normal
    /// completion must be proved separately; no operand effects are discharged.
    #[must_use]
    pub fn normal_numeric_result_production(
        self,
        args: InvocationArguments<'_>,
        argument_offset: usize,
    ) -> Option<NativeNumericResultProduction> {
        if let Self::ScalarMath(operation) = self {
            return crate::mathfunc::NativeScalarMathProtocol::for_invocation(
                operation,
                args,
                argument_offset,
            )
            .map(crate::mathfunc::NativeScalarMathProtocol::result_production);
        }
        if self != Self::ListLength || argument_offset.checked_add(1) != args.exact_argv_len() {
            return None;
        }
        Some(NativeNumericResultProduction::Integer {
            arithmetic: args.dialect()?.arithmetic()?,
        })
    }

    /// Select the representation produced at the successful store boundary.
    /// The caller must prove the actual handler and reached normal store, then
    /// publish this only into its captured cell before executing write callbacks.
    /// Unknown cardinality or arithmetic supplies no shape evidence.
    #[must_use]
    pub fn normal_numeric_store_production(
        self,
        args: InvocationArguments<'_>,
        argument_offset: usize,
    ) -> Option<NativeNumericStoreProduction> {
        if self != Self::IncrementStore
            || !matches!(
                args.exact_argv_len()?.checked_sub(argument_offset),
                Some(1 | 2)
            )
        {
            return None;
        }
        Some(NativeNumericStoreProduction::Increment {
            arithmetic: args.dialect()?.arithmetic()?,
        })
    }

    /// Audited list-method provider on the actual normal result. This query
    /// requires an independently proved native handler and normal return after
    /// operand evaluation; it does not preserve the producer's mutable world.
    #[must_use]
    pub fn normal_list_method_provider(
        self,
        args: InvocationArguments<'_>,
        argument_offset: usize,
    ) -> Option<NativeListMethodProvider> {
        if self != Self::ArithmeticSequence
            || !(1..=5).contains(&args.exact_argv_len()?.checked_sub(argument_offset)?)
        {
            return None;
        }
        let dialect = args.dialect()?;
        (dialect.family() == Some(tcl_dialect::model::Family::Tcl)
            && dialect
                .tcl_version
                .is_some_and(|version| version >= tcl_dialect::TclVersion::V9_0))
        .then_some(NativeListMethodProvider::ArithmeticSequence)
    }

    /// Select positions without guessing values, completion or trace behavior.
    /// Consumers must first prove native identity and the appropriate completion.
    #[must_use]
    pub fn select(
        self,
        args: InvocationArguments<'_>,
        argument_offset: usize,
    ) -> NativeResultSelection {
        if self == Self::EmptyString {
            return NativeResultSelection::EmptyString;
        }
        let Some(count) = args.exact_argv_len() else {
            return NativeResultSelection::Unknown;
        };
        match self {
            Self::EmptyString => NativeResultSelection::EmptyString,
            Self::VariableValue { variable_at, phase } => {
                let variable_at = argument_offset + usize::from(variable_at);
                if variable_at < count {
                    NativeResultSelection::VariableValue { variable_at, phase }
                } else {
                    NativeResultSelection::InvalidArguments
                }
            }
            Self::VariableExistence { variable_at } => {
                let variable_at = argument_offset + usize::from(variable_at);
                if variable_at.checked_add(1) == Some(count) {
                    NativeResultSelection::VariableExistence { variable_at }
                } else {
                    NativeResultSelection::InvalidArguments
                }
            }
            Self::ListArguments { from } => {
                let from = argument_offset + usize::from(from);
                count
                    .checked_sub(from)
                    .map_or(NativeResultSelection::InvalidArguments, |len| {
                        NativeResultSelection::ListArguments { from, len }
                    })
            }
            Self::ListRange {
                list_at,
                first_at,
                last_at,
            } => select_list_range(count, argument_offset, [list_at, first_at, last_at]),
            Self::DictionaryArguments { from } => {
                let from = argument_offset + usize::from(from);
                match count.checked_sub(from) {
                    Some(len) if len.is_multiple_of(2) => {
                        NativeResultSelection::DictionaryArguments { from, len }
                    }
                    _ => NativeResultSelection::InvalidArguments,
                }
            }
            Self::DictionaryValue {
                dictionary_at,
                keys_from,
            } => {
                let dictionary_at = argument_offset + usize::from(dictionary_at);
                let keys_from = argument_offset + usize::from(keys_from);
                match count.checked_sub(keys_from) {
                    Some(keys_len) if keys_len > 0 && dictionary_at < keys_from => {
                        NativeResultSelection::DictionaryValue {
                            dictionary_at,
                            keys_from,
                            keys_len,
                        }
                    }
                    Some(0) if dictionary_at < count => NativeResultSelection::Unknown,
                    _ => NativeResultSelection::InvalidArguments,
                }
            }
            Self::ReturnResult if argument_offset == 0 => {
                crate::registry::native_return_result_selection(args)
            }
            // The numeric shape exists before observers, whereas this query
            // describes the invocation result after observers have run.
            Self::ReturnResult
            | Self::IncrementStore
            | Self::ArithmeticSequence
            | Self::ListLength
            | Self::NamespaceCommandPrefix
            | Self::ScalarMath(_) => NativeResultSelection::Unknown,
        }
    }
}

fn select_list_range(count: usize, offset: usize, positions: [u8; 3]) -> NativeResultSelection {
    match positions.map(|position| offset.checked_add(usize::from(position))) {
        [Some(input_at), Some(first_at), Some(last_at)]
            if input_at < first_at
                && first_at < last_at
                && last_at.checked_add(1) == Some(count) =>
        {
            NativeResultSelection::ListRange {
                list_at: input_at,
                first_at,
                last_at,
            }
        }
        _ => NativeResultSelection::InvalidArguments,
    }
}
