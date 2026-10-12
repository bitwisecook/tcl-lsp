// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Tcl value-representation effects declared by command specifications.
//!
//! Tcl values may carry both a string representation and a typed internal
//! representation. They are also reference-counted: a container mutation
//! duplicates a shared value before changing it. Registry consumers need
//! these facts for shimmer and copy-on-write diagnostics without recognising
//! command spellings.

/// A command form's effect on a Tcl value's representation or sharing.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum RepresentationEffect {
    /// No representation effect has been declared.
    #[default]
    None,
    /// Mutating the named container duplicates it when its Tcl object is
    /// shared.
    ///
    /// Both indices are relative to the effective command or subcommand's
    /// arguments. `minimum_arguments` distinguishes a mutation from a
    /// read-only or replacement spelling of the same command.
    CopyOnWriteContainerMutation {
        /// Argument containing the variable name whose value is mutated.
        variable_arg: u8,
        /// Minimum number of arguments required for this form to mutate the
        /// container in place.
        minimum_arguments: u8,
    },
    /// Parse/compile expression source values and coerce values read by the
    /// expression. Internal representations can change without changing
    /// string bytes or variable cells, including through a shared object.
    CoerceExpressionValues {
        /// First expression-source argument, relative to the selected form.
        arguments_from: u8,
    },
    /// Native scalar numeric conversions of original math value operands.
    /// Unknown object updaters and intrep destructors remain effect obligations;
    /// this descriptor grants neither a numeric cache nor a normal result.
    CoerceNumericValues {
        /// First original value argument, relative to the selected form.
        arguments_from: u8,
    },
    /// The normal native list-length conversion of an ordinary container.
    /// Abstract lists and unproved object representations are excluded.
    CoerceOrdinaryList {
        /// Value operand relative to the selected command or member.
        operand: u8,
    },
    /// The normal native dictionary-size conversion of an ordinary container.
    CoerceOrdinaryDictionary {
        /// Value operand relative to the selected command or member.
        operand: u8,
    },
    /// Native foreach list-input conversion, independently of body execution.
    CoerceOrdinaryListPairs {
        /// First variable-list word, then alternating values and a final body.
        variables_from: u8,
    },
    /// Native list indexing, including C's single grouped-index-list form.
    CoerceOrdinaryListIndices {
        /// Original list operand.
        operand: u8,
        /// First index operand; all remaining arguments are indices.
        indices_from: u8,
    },
    /// Native list range selection with two independent index operands.
    CoerceOrdinaryListRange {
        /// Original list operand.
        operand: u8,
        /// First selected index.
        first: u8,
        /// Last selected index.
        last: u8,
    },
}

impl crate::InvocationFacts {
    /// Original scalar operands whose numeric conversion may invoke native
    /// object hooks. Exact selected layout and native axis are required; this
    /// inventories effect obligations without proving conversion or a result.
    #[must_use]
    pub fn numeric_object_conversion_arguments(
        &self,
        arguments: crate::InvocationArguments<'_>,
    ) -> Option<Vec<usize>> {
        let RepresentationEffect::CoerceNumericValues { arguments_from } =
            self.representation_effect
        else {
            return None;
        };
        let dialect = arguments.dialect()?;
        // Engine consistency alone is required. This is not a primitive
        // getter result, numeric acceptance or current cache proof.
        dialect.native_scalar_getter_protocol()?;
        crate::mathfunc::native_function_dispatch(dialect)?;
        let count = arguments.exact_argv_len()?;
        if self.arity_accepts_frozen_arguments() != Some(true)
            || self.frozen_argument_count != Some(count)
        {
            return None;
        }
        let first = self
            .argument_offset
            .checked_add(usize::from(arguments_from))?;
        (first <= count).then(|| (first..count).collect())
    }
}

/// Independently proved ordinary object representations covered by conversion.
/// A semantic list/dictionary type or C 9 abstract-list capability is insufficient.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OrdinaryContainerRepresentation {
    /// Native ordinary list internal representation.
    List,
    /// Native ordinary dictionary internal representation.
    Dictionary,
}

/// Selected successful conversion, requiring independent ordinary-object proof.
/// It grants neither a result representation nor a fresh object or opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrdinaryContainerCoercion {
    argument: usize,
    target: OrdinaryContainerRepresentation,
    preserves_empty: bool,
    may_skip: bool,
}

impl OrdinaryContainerCoercion {
    /// Absolute post-head argv operand, including aliases/member offsets.
    #[must_use]
    pub const fn argument(self) -> usize {
        self.argument
    }

    /// Ordinary representation installed if the conversion is performed.
    #[must_use]
    pub const fn target(self) -> OrdinaryContainerRepresentation {
        self.target
    }

    /// Whether actual bytes leave a possible branch preserving the input.
    /// Empty bytes do not prove a resident canonical-empty string pointer.
    #[must_use]
    pub fn may_preserve_input_for_bytes(self, bytes: Option<&[u8]>) -> bool {
        self.may_skip || (self.preserves_empty && bytes.is_none_or(<[u8]>::is_empty))
    }
}

/// Native object dispatch used by an independently selected list-length handler.
/// This identifies an effect obligation, not a value, completion or opcode.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ListLengthObjectProtocol {
    /// This native engine parses ordinary lists without abstract-list methods.
    Ordinary {
        /// Absolute post-head list operand, preserving member/alias offsets.
        argument: usize,
    },
    /// Tcl 9 can invoke the operand's registered native length method.
    /// Ordinary List/Dictionary or an audited read-only provider is required
    /// before preserving the surrounding mutable world.
    AbstractLength {
        /// Absolute post-head operand whose native object type supplies the method.
        argument: usize,
    },
    /// The actual engine object protocol has not been established.
    Unknown,
}

/// Actual argument layout for a native representation-coercion phase.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RepresentationCoercionSelection {
    /// This invocation declares no operand representation coercion.
    None,
    /// These argv values supply the expression source. Expression values
    /// are additionally coerced when lazy evaluation actually reads them.
    Expression {
        /// Absolute post-head argv indices; aliases/subcommands keep offsets.
        source_arguments: std::ops::Range<usize>,
    },
    /// Selected argv objects may have their existing internal representation
    /// replaced, including objects also held by other variables.
    Operands {
        /// Absolute post-head argv indices under the selected form's layout.
        arguments: Vec<usize>,
    },
    /// Expansion or unavailable argv shape prevents exact operand selection.
    Unknown,
}

impl RepresentationEffect {
    /// Declare a shared-container mutation.
    #[must_use]
    pub const fn copy_on_write_container(variable_arg: u8, minimum_arguments: u8) -> Self {
        Self::CopyOnWriteContainerMutation {
            variable_arg,
            minimum_arguments,
        }
    }

    /// Return the effective argument index carrying the mutated variable.
    ///
    /// `argument_offset` is one for a subcommand and zero for a top-level
    /// command. The invocation remains unclassified when it does not meet the
    /// descriptor's mutation arity floor.
    #[must_use]
    pub fn mutation_target_index(
        self,
        effective_argument_count: usize,
        argument_offset: usize,
    ) -> Option<usize> {
        match self {
            Self::CopyOnWriteContainerMutation {
                variable_arg,
                minimum_arguments,
            } if effective_argument_count >= usize::from(minimum_arguments) => {
                Some(argument_offset + usize::from(variable_arg))
            }
            Self::None
            | Self::CopyOnWriteContainerMutation { .. }
            | Self::CoerceExpressionValues { .. }
            | Self::CoerceNumericValues { .. }
            | Self::CoerceOrdinaryList { .. }
            | Self::CoerceOrdinaryDictionary { .. }
            | Self::CoerceOrdinaryListPairs { .. }
            | Self::CoerceOrdinaryListIndices { .. }
            | Self::CoerceOrdinaryListRange { .. } => None,
        }
    }

    /// Project native coercion independently of result type and container
    /// mutation. Operands can shimmer before an eventual evaluation error;
    /// lazy expression branches do not license coercing unvisited values.
    #[must_use]
    pub fn coercion_selection(
        self,
        arguments: crate::InvocationArguments<'_>,
        argument_offset: usize,
    ) -> RepresentationCoercionSelection {
        if let Some((operand, indices)) = self.indexed_list_arguments(arguments, argument_offset) {
            let mut selected = indices;
            if self.indexed_list_input_is_used(arguments, argument_offset) != Some(false) {
                selected.insert(0, operand);
            }
            return if selected.is_empty() {
                RepresentationCoercionSelection::None
            } else {
                RepresentationCoercionSelection::Operands {
                    arguments: selected,
                }
            };
        }
        if matches!(
            self,
            Self::CoerceOrdinaryListIndices { .. } | Self::CoerceOrdinaryListRange { .. }
        ) {
            return RepresentationCoercionSelection::Unknown;
        }
        if let Self::CoerceOrdinaryListPairs { variables_from } = self {
            return ordinary_list_pair_arguments(arguments, argument_offset, variables_from)
                .map_or(RepresentationCoercionSelection::Unknown, |arguments| {
                    RepresentationCoercionSelection::Operands { arguments }
                });
        }
        if let Self::CoerceOrdinaryList { operand } | Self::CoerceOrdinaryDictionary { operand } =
            self
        {
            return arguments
                .exact_argv_len()
                .zip(argument_offset.checked_add(usize::from(operand)))
                .map_or(
                    RepresentationCoercionSelection::Unknown,
                    |(count, index)| {
                        if index < count {
                            RepresentationCoercionSelection::Operands {
                                arguments: vec![index],
                            }
                        } else {
                            RepresentationCoercionSelection::None
                        }
                    },
                );
        }
        let Self::CoerceExpressionValues { arguments_from } = self else {
            return RepresentationCoercionSelection::None;
        };
        let Some(count) = arguments.exact_argv_len() else {
            return RepresentationCoercionSelection::Unknown;
        };
        let Some(from) = argument_offset.checked_add(usize::from(arguments_from)) else {
            return RepresentationCoercionSelection::Unknown;
        };
        if from >= count {
            return RepresentationCoercionSelection::None;
        }
        RepresentationCoercionSelection::Expression {
            source_arguments: from..count,
        }
    }

    /// Select an actual native normal conversion for independently proved
    /// ordinary List/Dictionary objects. Unknown/abstract objects must not use
    /// this contract. C 9 abstract-list length callbacks can preserve another
    /// representation; ordinary Dict has no such callback in the pinned cores.
    #[must_use]
    pub fn successful_ordinary_container_coercion(
        self,
        arguments: crate::InvocationArguments<'_>,
        argument_offset: usize,
    ) -> Option<OrdinaryContainerCoercion> {
        use tcl_dialect::{
            TclVersion,
            model::{Family, Release},
        };
        if matches!(self, Self::CoerceOrdinaryListIndices { .. })
            && self.indexed_list_input_is_used(arguments, argument_offset) == Some(false)
        {
            return None;
        }
        let (operand, target) = match self {
            Self::CoerceOrdinaryList { operand }
            | Self::CoerceOrdinaryListIndices { operand, .. }
            | Self::CoerceOrdinaryListRange { operand, .. } => {
                (operand, OrdinaryContainerRepresentation::List)
            }
            Self::CoerceOrdinaryDictionary { operand } => {
                (operand, OrdinaryContainerRepresentation::Dictionary)
            }
            _ => return None,
        };
        let argument = argument_offset.checked_add(usize::from(operand))?;
        if argument >= arguments.exact_argv_len()? {
            return None;
        }
        let dialect = arguments.dialect()?;
        let preserves_empty = match dialect.family()? {
            Family::Tcl => {
                let version = dialect.tcl_version?;
                if target == OrdinaryContainerRepresentation::Dictionary
                    && version == TclVersion::V8_4
                {
                    return None;
                }
                target == OrdinaryContainerRepresentation::List && version >= TclVersion::V8_5
            }
            Family::Jim
                if dialect
                    .core_point
                    .is_some_and(|point| point.release() == Release::JIM_0_84) =>
            {
                false
            }
            _ => return None,
        };
        Some(OrdinaryContainerCoercion {
            argument,
            target,
            preserves_empty,
            may_skip: matches!(self, Self::CoerceOrdinaryListIndices { .. })
                && self.indexed_list_input_is_used(arguments, argument_offset) != Some(true),
        })
    }

    /// Bound the authored native list-length protocol after independently
    /// proving its original operand is an ordinary List/Dictionary object.
    /// The caller must validate that proof before operand coercion or observers.
    /// Semantic container types and C 9 abstract lists cannot use this query.
    /// Errors remain possible; no result, store or successful execution is proved.
    #[must_use]
    pub(crate) fn ordinary_list_length_completion(
        self,
        arguments: crate::InvocationArguments<'_>,
        argument_offset: usize,
        proved_argument: usize,
        representation: OrdinaryContainerRepresentation,
    ) -> Option<crate::completion_route::InvocationCompletionRoute> {
        use crate::{completion::CompletionCode, completion_route::InvocationCompletionRoute};
        let Self::CoerceOrdinaryList { .. } = self else {
            return None;
        };
        let conversion = self.successful_ordinary_container_coercion(arguments, argument_offset)?;
        if conversion.argument() != proved_argument
            || arguments.exact_argv_len()? != proved_argument.checked_add(1)?
        {
            return None;
        }
        match representation {
            OrdinaryContainerRepresentation::List | OrdinaryContainerRepresentation::Dictionary => {
                Some(InvocationCompletionRoute::TclAlternatives(&[
                    CompletionCode::Ok,
                    CompletionCode::Error,
                ]))
            }
        }
    }

    /// Original list-input positions in the authored pair layout. This selects
    /// operands only; it proves no object class, list value or callback closure.
    #[must_use]
    pub fn ordinary_list_pair_input_arguments(
        self,
        arguments: crate::InvocationArguments<'_>,
        argument_offset: usize,
    ) -> Option<Vec<usize>> {
        let Self::CoerceOrdinaryListPairs { variables_from } = self else {
            return None;
        };
        ordinary_list_pair_arguments(arguments, argument_offset, variables_from)
    }

    /// Every normal list-input conversion in an exact native foreach layout.
    /// Unknown expansion cardinality or malformed pairs retain uncertainty.
    #[must_use]
    pub fn successful_ordinary_container_coercions(
        self,
        arguments: crate::InvocationArguments<'_>,
        argument_offset: usize,
    ) -> Option<Vec<OrdinaryContainerCoercion>> {
        if matches!(
            self,
            Self::CoerceOrdinaryListIndices { .. } | Self::CoerceOrdinaryListRange { .. }
        ) {
            self.indexed_list_arguments(arguments, argument_offset)?;
            if self.indexed_list_input_is_used(arguments, argument_offset) == Some(false) {
                return Some(Vec::new());
            }
        }
        let Self::CoerceOrdinaryListPairs { variables_from } = self else {
            return self
                .successful_ordinary_container_coercion(arguments, argument_offset)
                .map(|coercion| vec![coercion]);
        };
        ordinary_list_pair_arguments(arguments, argument_offset, variables_from)?
            .into_iter()
            .map(|argument| {
                Self::CoerceOrdinaryList { operand: 0 }
                    .successful_ordinary_container_coercion(arguments, argument)
            })
            .collect()
    }

    /// Actual native index objects whose coercion footprint is independent of
    /// the original ordinary list. Numeric-object proof may exclude sharing
    /// with a current ordinary container, but does not prove index contents or
    /// preserve that numeric representation through a grouped-index path.
    #[must_use]
    pub fn ordinary_container_index_arguments(
        self,
        arguments: crate::InvocationArguments<'_>,
        argument_offset: usize,
    ) -> Option<Vec<usize>> {
        Self::CoerceOrdinaryList { operand: 0 }
            .successful_ordinary_container_coercion(arguments, argument_offset)?;
        self.indexed_list_arguments(arguments, argument_offset)
            .map(|(_, indices)| indices)
    }

    fn indexed_list_arguments(
        self,
        arguments: crate::InvocationArguments<'_>,
        offset: usize,
    ) -> Option<(usize, Vec<usize>)> {
        let count = arguments.exact_argv_len()?;
        let (operand, indices) = match self {
            Self::CoerceOrdinaryListIndices {
                operand,
                indices_from,
            } => {
                let first = offset.checked_add(usize::from(indices_from))?;
                if first > count {
                    return None;
                }
                (operand, (first..count).collect())
            }
            Self::CoerceOrdinaryListRange {
                operand,
                first,
                last,
            } => {
                let indices = vec![
                    offset.checked_add(usize::from(first))?,
                    offset.checked_add(usize::from(last))?,
                ];
                if indices.iter().any(|&index| index >= count) {
                    return None;
                }
                (operand, indices)
            }
            _ => return None,
        };
        let operand = offset.checked_add(usize::from(operand))?;
        (operand < count).then_some((operand, indices))
    }

    fn indexed_list_input_is_used(
        self,
        arguments: crate::InvocationArguments<'_>,
        offset: usize,
    ) -> Option<bool> {
        let (_, indices) = self.indexed_list_arguments(arguments, offset)?;
        if indices.is_empty() {
            return Some(false);
        }
        if !matches!(self, Self::CoerceOrdinaryListIndices { .. }) || indices.len() > 1 {
            return Some(true);
        }
        let dialect = arguments.dialect()?;
        if dialect.family()? == tcl_dialect::model::Family::Jim {
            return Some(true);
        }
        if dialect.family()? != tcl_dialect::model::Family::Tcl {
            return None;
        }
        let index = arguments.literal_at(indices[0])?;
        tcl_syntax::list::split_list_bytes_in(
            index.as_bytes(),
            dialect.word_values.list,
            dialect.lexer_grammar.escapes,
        )
        .ok()
        // The generic C handler returns an empty grouped path unchanged;
        // LIST_INDEX bytecode can convert the original list before falling
        // back to that handler. Bytes alone cannot select the compilation path.
        .and_then(|elements| (!elements.is_empty()).then_some(true))
    }
}

fn ordinary_list_pair_arguments(
    arguments: crate::InvocationArguments<'_>,
    argument_offset: usize,
    variables_from: u8,
) -> Option<Vec<usize>> {
    let first = argument_offset.checked_add(usize::from(variables_from))?;
    let count = arguments.exact_argv_len()?.checked_sub(first)?;
    if count < 3 || count % 2 != 1 {
        return None;
    }
    Some((first + 1..first + count - 1).step_by(2).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numeric_conversion_effects_require_the_authored_original_layout() {
        use crate::{InvocationArguments, InvocationDialect, InvocationWord, InvocationWords};
        let operands = [InvocationWord::Dynamic];
        for environment in ["tcl8.5", "tcl8.6", "tcl9.0", "tcl9.1"] {
            let context = crate::model::ingress::static_context_for(environment);
            let dialect = InvocationDialect::of_profile(context.commands().profile().unwrap());
            let words = InvocationWords::structured(
                InvocationWord::Literal("::tcl::mathfunc::sin"),
                &operands,
            )
            .with_dialect(dialect);
            let facts = context
                .commands()
                .resolve_structured_invocation(words, dialect.authoring_query())
                .resolved()
                .unwrap()
                .facts();
            assert_eq!(
                facts.numeric_object_conversion_arguments(words.arguments()),
                Some(vec![0])
            );
            assert!(facts.native_result.is_none());
            let mut conflict = dialect;
            conflict.core_point = Some(tcl_dialect::model::DialectPoint::canonical(
                tcl_dialect::model::Release::JIM_0_84,
            ));
            assert!(
                facts
                    .numeric_object_conversion_arguments(
                        InvocationArguments::structured(&operands).with_dialect(conflict),
                    )
                    .is_none()
            );
            assert!(
                facts
                    .numeric_object_conversion_arguments(InvocationArguments::structured(&operands))
                    .is_none()
            );
            let expanded = [InvocationWord::Expanded];
            assert!(
                facts
                    .numeric_object_conversion_arguments(
                        InvocationArguments::structured(&expanded).with_dialect(dialect)
                    )
                    .is_none()
            );
            let mut unrelated = facts.clone();
            unrelated.representation_effect = RepresentationEffect::None;
            assert!(
                unrelated
                    .numeric_object_conversion_arguments(words.arguments())
                    .is_none()
            );
        }
    }

    #[test]
    fn ordinary_list_length_completion_requires_its_native_operand_contract() {
        use crate::{InvocationArguments, InvocationDialect, InvocationWord};
        use tcl_dialect::TclVersion;
        let effect = RepresentationEffect::CoerceOrdinaryList { operand: 0 };
        let words = [InvocationWord::Dynamic];
        for version in TclVersion::ALL {
            let arguments = InvocationArguments::structured(&words)
                .with_dialect(InvocationDialect::for_version(version));
            for representation in [
                OrdinaryContainerRepresentation::List,
                OrdinaryContainerRepresentation::Dictionary,
            ] {
                let route = effect
                    .ordinary_list_length_completion(arguments, 0, 0, representation)
                    .expect("ordinary native list-length route");
                assert!(route.normal_possible());
                assert!(route.abrupt_possible());
                assert_eq!(route.alternatives().len(), 2);
                assert!(route.alternatives().iter().all(|route| matches!(
                    route,
                    crate::completion_route::InvocationCompletionRoute::Tcl(
                        crate::completion::CompletionCode::Ok
                            | crate::completion::CompletionCode::Error
                    )
                )));
            }
            assert!(
                effect
                    .ordinary_list_length_completion(
                        arguments,
                        0,
                        1,
                        OrdinaryContainerRepresentation::List
                    )
                    .is_none()
            );
            assert!(
                RepresentationEffect::CoerceOrdinaryListRange {
                    operand: 0,
                    first: 1,
                    last: 2,
                }
                .ordinary_list_length_completion(
                    arguments,
                    0,
                    0,
                    OrdinaryContainerRepresentation::List
                )
                .is_none()
            );
        }
        assert!(
            effect
                .ordinary_list_length_completion(
                    InvocationArguments::structured(&words),
                    0,
                    0,
                    OrdinaryContainerRepresentation::List
                )
                .is_none()
        );
    }

    #[test]
    fn c_indexed_list_footprints_keep_grouped_empty_and_numeric_index_axes_separate() {
        use crate::{InvocationArguments, InvocationDialect, InvocationWord};
        use tcl_dialect::TclVersion;
        let effect = RepresentationEffect::CoerceOrdinaryListIndices {
            operand: 0,
            indices_from: 1,
        };
        let unknown = [InvocationWord::Dynamic, InvocationWord::Dynamic];
        for version in TclVersion::ALL {
            let dialect = InvocationDialect::for_version(version);
            let args = InvocationArguments::structured(&unknown).with_dialect(dialect);
            assert_eq!(
                effect.ordinary_container_index_arguments(args, 0),
                Some(vec![1])
            );
            let input = effect
                .successful_ordinary_container_coercions(args, 0)
                .unwrap();
            assert_eq!(input[0].argument(), 0);
            assert!(input[0].may_preserve_input_for_bytes(Some(b"a 1")));
            let empty = InvocationArguments::literals(&["malformed {", ""]).with_dialect(dialect);
            assert!(
                effect
                    .successful_ordinary_container_coercions(empty, 0)
                    .unwrap()[0]
                    .may_preserve_input_for_bytes(Some(b"a 1"))
            );
            assert_eq!(
                effect.coercion_selection(empty, 0),
                RepresentationCoercionSelection::Operands {
                    arguments: vec![0, 1]
                }
            );
            let no_index = InvocationArguments::literals(&["malformed {"]).with_dialect(dialect);
            assert_eq!(
                effect.coercion_selection(no_index, 0),
                RepresentationCoercionSelection::None
            );
            let nested = InvocationArguments::literals(&["a 1", "0", "1"]).with_dialect(dialect);
            assert_eq!(
                effect.ordinary_container_index_arguments(nested, 0),
                Some(vec![1, 2])
            );
        }
    }

    #[test]
    fn jim_indexed_list_footprints_keep_native_index_and_range_axes_separate() {
        use crate::{InvocationArguments, InvocationDialect, InvocationWord};
        use tcl_dialect::model::{DialectPoint, Release};
        let effect = RepresentationEffect::CoerceOrdinaryListIndices {
            operand: 0,
            indices_from: 1,
        };
        let unknown = [InvocationWord::Dynamic, InvocationWord::Dynamic];
        let jim = InvocationDialect::of_point(DialectPoint::canonical(Release::JIM_0_84));
        let args = InvocationArguments::structured(&unknown).with_dialect(jim);
        assert!(
            !effect
                .successful_ordinary_container_coercions(args, 0)
                .unwrap()[0]
                .may_preserve_input_for_bytes(Some(b"a 1"))
        );
        assert!(
            effect
                .ordinary_container_index_arguments(InvocationArguments::structured(&unknown), 0)
                .is_none()
        );
        let expanded = [InvocationWord::Dynamic, InvocationWord::Expanded];
        assert_eq!(
            effect.coercion_selection(
                InvocationArguments::structured(&expanded).with_dialect(jim),
                0
            ),
            RepresentationCoercionSelection::Unknown
        );
        let range = RepresentationEffect::CoerceOrdinaryListRange {
            operand: 0,
            first: 1,
            last: 2,
        };
        let args = InvocationArguments::literals(&["a 1", "0", "1"]).with_dialect(jim);
        assert_eq!(
            range.ordinary_container_index_arguments(args, 0),
            Some(vec![1, 2])
        );
        assert!(
            !range
                .successful_ordinary_container_coercions(args, 0)
                .unwrap()[0]
                .may_preserve_input_for_bytes(None)
        );
    }

    #[test]
    fn ordinary_coercion_keeps_native_empty_and_unknown_policies() {
        use crate::{InvocationArguments, InvocationDialect};
        use tcl_dialect::TclVersion;
        for version in TclVersion::ALL {
            let args = InvocationArguments::literals(&["prefix", "value"])
                .with_dialect(InvocationDialect::for_version(version));
            let list = RepresentationEffect::CoerceOrdinaryList { operand: 0 }
                .successful_ordinary_container_coercion(args, 1)
                .unwrap();
            assert_eq!(list.argument(), 1);
            assert_eq!(list.target(), OrdinaryContainerRepresentation::List);
            assert!(!list.may_preserve_input_for_bytes(Some(b"a 1")));
            assert_eq!(
                list.may_preserve_input_for_bytes(Some(b"")),
                version >= TclVersion::V8_5
            );
            assert_eq!(
                list.may_preserve_input_for_bytes(None),
                version >= TclVersion::V8_5
            );
            let dict = RepresentationEffect::CoerceOrdinaryDictionary { operand: 0 }
                .successful_ordinary_container_coercion(args, 1);
            assert_eq!(dict.is_some(), version >= TclVersion::V8_5);
            if let Some(dict) = dict {
                assert_eq!(dict.target(), OrdinaryContainerRepresentation::Dictionary);
                assert!(!dict.may_preserve_input_for_bytes(None));
            }
        }
        let jim = InvocationDialect::of_point(tcl_dialect::model::DialectPoint::canonical(
            tcl_dialect::model::Release::JIM_0_84,
        ));
        let list = RepresentationEffect::CoerceOrdinaryList { operand: 0 }
            .successful_ordinary_container_coercion(
                InvocationArguments::literals(&["x"]).with_dialect(jim),
                0,
            )
            .unwrap();
        assert!(!list.may_preserve_input_for_bytes(None));
        assert!(
            RepresentationEffect::CoerceOrdinaryList { operand: 0 }
                .successful_ordinary_container_coercion(InvocationArguments::literals(&["x"]), 0)
                .is_none()
        );
    }

    #[test]
    fn ordinary_foreach_coercion_selects_only_exact_list_values() {
        use crate::{InvocationArguments, InvocationDialect, InvocationWord};
        let dialect = InvocationDialect::for_version(tcl_dialect::TclVersion::V9_1);
        let effect = RepresentationEffect::CoerceOrdinaryListPairs { variables_from: 0 };
        let values = [
            InvocationWord::Literal("prefix"),
            InvocationWord::Literal("a"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("b"),
            InvocationWord::Dynamic,
            InvocationWord::Literal("body"),
        ];
        let selected = effect
            .successful_ordinary_container_coercions(
                InvocationArguments::structured(&values).with_dialect(dialect),
                1,
            )
            .unwrap();
        assert_eq!(
            selected
                .iter()
                .map(|item| item.argument())
                .collect::<Vec<_>>(),
            [2, 4]
        );
        assert!(
            effect
                .successful_ordinary_container_coercions(
                    InvocationArguments::structured(&[InvocationWord::Expanded])
                        .with_dialect(dialect),
                    0
                )
                .is_none()
        );
        assert!(
            effect
                .successful_ordinary_container_coercions(
                    InvocationArguments::literals(&["a", "value", "b", "body"])
                        .with_dialect(dialect),
                    0
                )
                .is_none()
        );
    }

    #[test]
    fn copy_on_write_effect_applies_only_to_mutating_arity() {
        let effect = RepresentationEffect::copy_on_write_container(0, 2);
        assert_eq!(effect.mutation_target_index(1, 0), None);
        assert_eq!(effect.mutation_target_index(2, 0), Some(0));
        assert_eq!(effect.mutation_target_index(2, 1), Some(1));
    }

    #[test]
    fn expression_coercion_tracks_effective_argv_without_mutating_a_container() {
        let effect = RepresentationEffect::CoerceExpressionValues { arguments_from: 0 };
        let arguments = crate::InvocationArguments::literals(&["expr", "known"]);
        assert_eq!(
            effect.coercion_selection(arguments, 1),
            RepresentationCoercionSelection::Expression {
                source_arguments: 1..2
            }
        );
        assert_eq!(effect.mutation_target_index(2, 1), None);
        assert_eq!(
            effect.coercion_selection(crate::InvocationArguments::literals(&[]), 0),
            RepresentationCoercionSelection::None
        );
        assert_eq!(
            effect.coercion_selection(
                crate::InvocationArguments::structured(&[crate::InvocationWord::Expanded]),
                0,
            ),
            RepresentationCoercionSelection::Unknown
        );
    }
}
