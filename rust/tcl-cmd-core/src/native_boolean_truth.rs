// SPDX-License-Identifier: AGPL-3.0-or-later
//! Ordered original operand conversion shared by physical runtimes.
//!
//! The pure recipe supplies stage selection. An adapter separately retains the
//! actual object, selected interpreter, checked getter and publication owner.
//! Every reached effect completes before another stage begins; a successful
//! fallback retains an earlier getter's actual interpreter diagnostic.

use tcl_syntax::{
    native_boolean_truth::{
        NativeBooleanTruthFailureProducer, NativeBooleanTruthGetterEffects,
        NativeBooleanTruthPreparation, NativeBooleanTruthProbe, NativeBooleanTruthProtocol,
    },
    scalar_getter::{
        NativeScalarCache, NativeScalarGetterFailure, NativeScalarGetterKind,
        NativeScalarGetterValue,
    },
    value::ValueError,
};

use crate::CmdError;

/// Actual original object and interpreter operations. Implementations inspect
/// the current first Host cause without resetting the entry or Tcl state.
pub trait NativeBooleanTruthOps {
    /// A retained original operand, independently of its scalar cache.
    type Value;

    /// Refuse a pending first Host cause before reading or publishing anything.
    /// # Errors
    /// Returns the unchanged actual Host cause.
    fn check_host_refusal(&mut self) -> Result<(), CmdError>;

    /// Inspect a genuine live compatible header before any cache fast path.
    /// # Errors
    /// Refuses foreign, retired or unsupported original cache identities.
    fn inspect_original(
        &mut self,
        value: &Self::Value,
        protocol: NativeBooleanTruthProtocol,
    ) -> Result<(Option<NativeScalarCache>, bool), CmdError>;

    /// Install the actual evaluated original term as interpreter result when
    /// the independently selected Jim conversion producer requires it.
    /// # Errors
    /// Refuses a missing publication owner or preserves the original first Host.
    fn publish_original_operand(
        &mut self,
        value: &Self::Value,
        protocol: NativeBooleanTruthProtocol,
    ) -> Result<(), CmdError>;

    /// Run the selected original String getter, preserving counted bytes.
    /// # Errors
    /// Returns the original checked getter's full operational refusal.
    fn original_string(
        &mut self,
        value: &Self::Value,
        protocol: NativeBooleanTruthProtocol,
    ) -> Result<Vec<u8>, CmdError>;

    /// Execute exactly one reached probe and its original cache transitions.
    /// This operation does not render or publish an unsuccessful guest getter.
    /// # Errors
    /// Refuses unavailable original conversion or host environment facts.
    fn probe(
        &mut self,
        value: &Self::Value,
        protocol: NativeBooleanTruthProtocol,
        stage: NativeBooleanTruthProbe,
    ) -> Result<Result<NativeScalarGetterValue, NativeScalarGetterFailure>, CmdError>;

    /// Retain the selected primitive's full diagnostic and metadata. Only its
    /// declared rendering input obligation may reach another String getter.
    /// # Errors
    /// Returns the checked rendering getter's original Host refusal.
    fn getter_failure(
        &mut self,
        value: &Self::Value,
        protocol: NativeBooleanTruthProtocol,
        kind: NativeScalarGetterKind,
        failure: NativeScalarGetterFailure,
    ) -> Result<CmdError, CmdError>;

    /// Publish a reached intermediate failed primitive before its successor.
    /// The last failed stage is returned to the caller for one final publication.
    /// # Errors
    /// Returns the first Host cause produced by publication or its callbacks.
    fn publish_intermediate_failure(&mut self, failure: CmdError) -> Result<(), CmdError>;

    /// Retain C's original instruction-specific operand failure, independently
    /// of the neutral primitive's diagnostic. Original classification and list
    /// getters remain with the selected physical adapter and shared owners.
    /// # Errors
    /// Refuses any unavailable original diagnostic/classification operation.
    fn logical_operand_failure(
        &mut self,
        value: &Self::Value,
        protocol: NativeBooleanTruthProtocol,
        failures: &[(NativeBooleanTruthProbe, NativeScalarGetterFailure)],
    ) -> Result<CmdError, CmdError>
    where
        Self: Sized,
    {
        original_logical_operand_failure(value, protocol, failures, self)
    }

    /// Reach the original neutral Number primitive for C's instruction error.
    /// # Errors
    /// Preserves unavailable getters and actual Host failures.
    fn logical_number_probe(
        &mut self,
        _value: &Self::Value,
        _protocol: NativeBooleanTruthProtocol,
    ) -> Result<Result<tcl_syntax::number::Number, NativeScalarGetterFailure>, CmdError> {
        Err(unavailable())
    }
    /// Inspect a genuine original Dictionary cache without generating String.
    /// # Errors
    /// Refuses unavailable original header/backing inspection.
    fn logical_dictionary_size(
        &mut self,
        _value: &Self::Value,
        _protocol: NativeBooleanTruthProtocol,
    ) -> Result<Option<usize>, CmdError> {
        Err(unavailable())
    }
    /// Run an actual reached C9 object length hook, independently of spelling.
    /// # Errors
    /// Retains the original hook's operational refusal.
    fn logical_length_hook(
        &mut self,
        _value: &Self::Value,
        _protocol: NativeBooleanTruthProtocol,
    ) -> Result<Option<usize>, CmdError> {
        Err(unavailable())
    }
    /// Enter the selected original List getter with NULL guest diagnostics.
    /// Rejected ordinary guest grammar is false; original Host remains Err.
    /// # Errors
    /// Retains the actual original List getter refusal.
    fn logical_list_probe(
        &mut self,
        _value: &Self::Value,
        _protocol: NativeBooleanTruthProtocol,
    ) -> Result<bool, CmdError> {
        Err(unavailable())
    }
    /// Probe a fresh spelling object for C8.4's diagnostic classification.
    /// Its cache belongs to that fresh header, independently of the operand.
    /// # Errors
    /// Retains the actual constructor/getter/host failure.
    fn logical_spelling_double_probe(
        &mut self,
        _original: &[u8],
        _protocol: NativeBooleanTruthProtocol,
    ) -> Result<bool, CmdError> {
        Err(unavailable())
    }
}

fn legacy_logical_class<O: NativeBooleanTruthOps>(
    value: &O::Value,
    protocol: NativeBooleanTruthProtocol,
    failures: &[(NativeBooleanTruthProbe, NativeScalarGetterFailure)],
    ops: &mut O,
) -> Result<tcl_syntax::native_boolean_truth::NativeBooleanLogicalOperandClass, CmdError> {
    use tcl_syntax::native_boolean_truth::NativeBooleanLogicalOperandClass as Class;
    let observation = ops.inspect_original(value, protocol);
    let (_, resident) = settled(ops, observation)?;
    if !resident {
        return Ok(Class::EmptyString);
    }
    let original = ops.original_string(value, protocol);
    let original = settled(ops, original)?;
    if original.is_empty() {
        return Ok(Class::EmptyString);
    }
    if original.eq_ignore_ascii_case(b"nan") {
        return Ok(Class::NonNumericFloatingPoint);
    }
    if original.eq_ignore_ascii_case(b"inf") {
        return Ok(Class::InfiniteFloatingPoint);
    }
    if failures
        .iter()
        .any(|(_, failure)| *failure == NativeScalarGetterFailure::InvalidOctal)
    {
        return Ok(Class::InvalidOctal);
    }
    if failures.iter().any(|(_, failure)| {
        matches!(
            failure,
            NativeScalarGetterFailure::IntegerOverflow
                | NativeScalarGetterFailure::IntWidthOverflow
        )
    }) {
        return Ok(Class::IntegerOverflow);
    }
    let double = ops.logical_spelling_double_probe(&original, protocol);
    Ok(if settled(ops, double)? {
        Class::FloatingPoint
    } else {
        Class::NonNumericString
    })
}

fn modern_logical_class<O: NativeBooleanTruthOps>(
    value: &O::Value,
    protocol: NativeBooleanTruthProtocol,
    ops: &mut O,
) -> Result<tcl_syntax::native_boolean_truth::NativeBooleanLogicalOperandClass, CmdError> {
    use tcl_syntax::{
        native_boolean_truth::NativeBooleanLogicalOperandClass as Class, number::Number,
    };
    let conversion = ops.logical_number_probe(value, protocol);
    let outcome = settled(ops, conversion)?;
    let version = protocol
        .scalar_protocol()
        .tcl_version()
        .ok_or_else(unavailable)?;
    Ok(match outcome {
        Ok(Number::Nan { .. }) => Class::NonNumericFloatingPoint,
        Ok(Number::Double(number)) if number.is_nan() => Class::NonNumericFloatingPoint,
        Ok(Number::Double(_)) => Class::FloatingPoint,
        Ok(_) => return Err(unavailable()),
        Err(failure) => {
            if version >= tcl_dialect::TclVersion::V9_0 {
                let size = ops.logical_dictionary_size(value, protocol);
                if settled(ops, size)?.is_some_and(|size| size > 0) {
                    return Ok(Class::List);
                }
                let length = ops.logical_length_hook(value, protocol);
                if settled(ops, length)?.is_some_and(|length| length > 1) {
                    return Ok(Class::List);
                }
            }
            let original = ops.original_string(value, protocol);
            let original = settled(ops, original)?;
            if version >= tcl_dialect::TclVersion::V9_0
                && tcl_syntax::list::max_list_length_bytes(&original) > 1
            {
                let parsed = ops.logical_list_probe(value, protocol);
                if settled(ops, parsed)? {
                    return Ok(Class::List);
                }
            }
            if original.is_empty() {
                Class::EmptyString
            } else if failure == NativeScalarGetterFailure::InvalidOctal {
                Class::InvalidOctal
            } else {
                Class::NonNumericString
            }
        }
    })
}

/// Reach C's one instruction-specific diagnostic classification and producer.
/// All Number, length, String and List effects are owned by the physical adapter
/// and settled immediately; no primitive diagnostic text is reparsed.
/// # Errors
/// Preserves first Host before any later getter or guest result publication.
pub fn original_logical_operand_failure<O: NativeBooleanTruthOps>(
    value: &O::Value,
    protocol: NativeBooleanTruthProtocol,
    failures: &[(NativeBooleanTruthProbe, NativeScalarGetterFailure)],
    ops: &mut O,
) -> Result<CmdError, CmdError> {
    ops.check_host_refusal()?;
    let version = protocol
        .scalar_protocol()
        .tcl_version()
        .ok_or_else(unavailable)?;
    let class = if version == tcl_dialect::TclVersion::V8_4 {
        legacy_logical_class(value, protocol, failures, ops)?
    } else {
        modern_logical_class(value, protocol, ops)?
    };
    let original = if version >= tcl_dialect::TclVersion::V9_0
        && class != tcl_syntax::native_boolean_truth::NativeBooleanLogicalOperandClass::List
    {
        let spelling = ops.original_string(value, protocol);
        Some(settled(ops, spelling)?)
    } else {
        None
    };
    let diagnostic = protocol
        .logical_failure_presentation(class, original.as_deref())
        .ok_or_else(unavailable)?;
    Ok(
        CmdError::with_error_code_bytes(diagnostic.message, diagnostic.error_code)
            .with_native_string_result(tcl_syntax::native_string::NativeStringProtocol::C(version)),
    )
}

fn unavailable() -> CmdError {
    ValueError::CommandProtocolUnavailable("original Boolean conversion stage").into()
}

fn returned_truth(
    stage: NativeBooleanTruthProbe,
    value: NativeScalarGetterValue,
) -> Result<bool, CmdError> {
    match (stage, value) {
        (
            NativeBooleanTruthProbe::ExpressionInteger84
            | NativeBooleanTruthProbe::ExpressionWordBoolean84(_),
            NativeScalarGetterValue::Wide(integer),
        )
        | (
            NativeBooleanTruthProbe::Scalar {
                kind: NativeScalarGetterKind::Wide | NativeScalarGetterKind::Long,
                ..
            },
            NativeScalarGetterValue::Wide(integer),
        ) => Ok(integer != 0),
        (
            NativeBooleanTruthProbe::Scalar {
                kind: NativeScalarGetterKind::Double,
                ..
            },
            NativeScalarGetterValue::Double(double),
        ) => Ok(double != 0.0),
        (
            NativeBooleanTruthProbe::Scalar {
                kind: NativeScalarGetterKind::Boolean,
                ..
            },
            NativeScalarGetterValue::Boolean(boolean),
        ) => Ok(boolean.is_true()),
        _ => Err(unavailable()),
    }
}

/// Convert one genuinely retained operand at its actual reached site.
/// Expression evaluation and result normalisation are independent producers.
/// # Errors
/// Preserves the original first Host or actual final guest diagnostic; no
/// unsuccessful getter becomes false, an empty result or another guest error.
pub fn original_boolean_truth<O: NativeBooleanTruthOps>(
    value: &O::Value,
    protocol: NativeBooleanTruthProtocol,
    ops: &mut O,
) -> Result<bool, CmdError> {
    ops.check_host_refusal()?;
    let observed = ops.inspect_original(value, protocol);
    ops.check_host_refusal()?;
    let (cache, string_resident) = observed?;
    if protocol.publishes_original_operand() {
        let publication = ops.publish_original_operand(value, protocol);
        ops.check_host_refusal()?;
        publication?;
    }
    let preparation = protocol.prepare(cache.as_ref(), string_resident, None);
    let preparation = if preparation == NativeBooleanTruthPreparation::OriginalStringRequired {
        let original = ops.original_string(value, protocol);
        ops.check_host_refusal()?;
        let original = original?;
        protocol.prepare(cache.as_ref(), string_resident, Some(&original))
    } else {
        preparation
    };
    let (stages, producer) = match preparation {
        NativeBooleanTruthPreparation::Complete(truth) => return Ok(truth),
        NativeBooleanTruthPreparation::Probes { stages, failure } => (stages, failure),
        NativeBooleanTruthPreparation::OriginalStringRequired => return Err(unavailable()),
    };
    let count = stages.len();
    let mut failures = Vec::with_capacity(count);
    for (index, stage) in stages.into_iter().enumerate() {
        ops.check_host_refusal()?;
        let outcome = ops.probe(value, protocol, stage);
        ops.check_host_refusal()?;
        let outcome = outcome?;
        let failure = match outcome {
            Ok(result) => return returned_truth(stage, result),
            Err(failure) => failure,
        };
        failures.push((stage, failure));
        let (kind, effects) = match stage {
            NativeBooleanTruthProbe::Scalar { kind, effects } => (kind, effects),
            NativeBooleanTruthProbe::ExpressionInteger84
            | NativeBooleanTruthProbe::ExpressionWordBoolean84(_) => (
                NativeScalarGetterKind::Long,
                NativeBooleanTruthGetterEffects::ErrorNeutral,
            ),
        };
        let last = index + 1 == count;
        if last && producer == NativeBooleanTruthFailureProducer::LogicalOperand {
            let failure = ops.logical_operand_failure(value, protocol, &failures);
            ops.check_host_refusal()?;
            let failure = failure?;
            return Err(failure);
        }
        if last || effects == NativeBooleanTruthGetterEffects::ReportGuestFailure {
            let failure = ops.getter_failure(value, protocol, kind, failure);
            ops.check_host_refusal()?;
            let failure = failure?;
            if last {
                return Err(failure);
            }
            let publication = ops.publish_intermediate_failure(failure);
            ops.check_host_refusal()?;
            publication?;
        }
    }
    Err(unavailable())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;
    use tcl_dialect::model::{DialectPoint, Release};
    use tcl_runtime_api::NativeExecutionError;
    use tcl_syntax::{
        native_boolean_truth::NativeBooleanTruthPurpose, raw_string::NativeValueAccessRefusal,
        scalar_getter::NativeScalarGetterProtocol,
    };

    struct PublicationOwner {
        calls: Vec<&'static str>,
        outcomes: VecDeque<Result<NativeScalarGetterValue, NativeScalarGetterFailure>>,
        result: Vec<u8>,
        first: Option<NativeExecutionError>,
        refuse_publication: bool,
        refuse_rendering: bool,
    }

    impl NativeBooleanTruthOps for PublicationOwner {
        type Value = ();
        fn check_host_refusal(&mut self) -> Result<(), CmdError> {
            self.first
                .clone()
                .map_or(Ok(()), |first| Err(CmdError::from_execution_refusal(first)))
        }
        fn inspect_original(
            &mut self,
            _: &(),
            _: NativeBooleanTruthProtocol,
        ) -> Result<(Option<NativeScalarCache>, bool), CmdError> {
            Ok((None, true))
        }
        fn publish_original_operand(
            &mut self,
            _: &(),
            _: NativeBooleanTruthProtocol,
        ) -> Result<(), CmdError> {
            self.calls.push("term");
            self.result = b"original operand".to_vec();
            Ok(())
        }
        fn original_string(
            &mut self,
            _: &(),
            _: NativeBooleanTruthProtocol,
        ) -> Result<Vec<u8>, CmdError> {
            panic!("this stage has no String selection obligation")
        }
        fn probe(
            &mut self,
            _: &(),
            _: NativeBooleanTruthProtocol,
            stage: NativeBooleanTruthProbe,
        ) -> Result<Result<NativeScalarGetterValue, NativeScalarGetterFailure>, CmdError> {
            self.calls.push(match stage {
                NativeBooleanTruthProbe::Scalar {
                    kind: NativeScalarGetterKind::Long,
                    ..
                } => "long",
                NativeBooleanTruthProbe::Scalar {
                    kind: NativeScalarGetterKind::Double,
                    ..
                } => "double",
                _ => panic!("unexpected reached primitive"),
            });
            Ok(self
                .outcomes
                .pop_front()
                .expect("each reached probe has a supplied outcome"))
        }
        fn getter_failure(
            &mut self,
            _: &(),
            _: NativeBooleanTruthProtocol,
            _: NativeScalarGetterKind,
            _: NativeScalarGetterFailure,
        ) -> Result<CmdError, CmdError> {
            if self.refuse_rendering {
                self.first = Some(original_cause());
            }
            Ok(CmdError::new_bytes(
                b"retained primitive diagnostic".to_vec(),
            ))
        }
        fn publish_intermediate_failure(&mut self, error: CmdError) -> Result<(), CmdError> {
            self.calls.push("publish");
            self.result = error.message_bytes().to_vec();
            if self.refuse_publication {
                self.first = Some(original_cause());
            }
            Ok(())
        }
        fn logical_operand_failure(
            &mut self,
            _: &(),
            _: NativeBooleanTruthProtocol,
            _: &[(NativeBooleanTruthProbe, NativeScalarGetterFailure)],
        ) -> Result<CmdError, CmdError> {
            panic!("Jim stages retain their actual primitive failure")
        }
    }

    fn original_cause() -> NativeExecutionError {
        NativeExecutionError::ValueAccessRefusal(
            NativeValueAccessRefusal::CommandProtocolUnavailable("original publication callback"),
        )
    }
    fn owner() -> PublicationOwner {
        PublicationOwner {
            calls: Vec::new(),
            outcomes: [
                Err(NativeScalarGetterFailure::CachedNonInteger),
                Ok(NativeScalarGetterValue::Double(17.0)),
            ]
            .into(),
            result: b"prior result".to_vec(),
            first: None,
            refuse_publication: false,
            refuse_rendering: false,
        }
    }
    fn protocol() -> NativeBooleanTruthProtocol {
        NativeBooleanTruthProtocol::for_scalar_getter(
            NativeScalarGetterProtocol::for_point(DialectPoint::canonical(Release::JIM_0_84))
                .unwrap(),
            NativeBooleanTruthPurpose::ConditionalJump,
        )
    }

    #[test]
    fn intermediate_failure_is_published_before_success_and_remains_observable() {
        // Source/API control only: supplied stage outcomes are not original Jim objects.
        // naming.numeric.original-primitive-boolean-vs-expression-truth
        // docs/design/analysis/name-resolution-proofs/numeric-original-primitive-boolean-vs-expression-truth.md
        let mut owner = owner();
        assert!(original_boolean_truth(&(), protocol(), &mut owner).unwrap());
        assert_eq!(owner.calls, ["term", "long", "publish", "double"]);
        assert_eq!(owner.result, b"retained primitive diagnostic");
        assert!(owner.first.is_none());
    }

    #[test]
    fn first_host_from_publication_stops_every_later_probe() {
        // Source/API control; this tests the shared settlement owner, not native conversion.
        // naming.numeric.original-primitive-boolean-vs-expression-truth
        // docs/design/analysis/name-resolution-proofs/numeric-original-primitive-boolean-vs-expression-truth.md
        let mut owner = owner();
        owner.refuse_publication = true;
        let error = original_boolean_truth(&(), protocol(), &mut owner).unwrap_err();
        assert_eq!(error.native_execution_refusal(), Some(&original_cause()));
        assert_eq!(owner.calls, ["term", "long", "publish"]);
        assert_eq!(owner.outcomes.len(), 1);
        assert_eq!(owner.result, b"retained primitive diagnostic");
    }

    #[test]
    fn first_host_from_rendering_wins_over_a_returned_guest_record() {
        // Source/API control; no original external provider outcome is inferred.
        // naming.numeric.original-primitive-boolean-vs-expression-truth
        // docs/design/analysis/name-resolution-proofs/numeric-original-primitive-boolean-vs-expression-truth.md
        let mut owner = owner();
        owner.refuse_rendering = true;
        let error = original_boolean_truth(&(), protocol(), &mut owner).unwrap_err();
        assert_eq!(error.native_execution_refusal(), Some(&original_cause()));
        assert_eq!(owner.calls, ["term", "long"]);
        assert_eq!(owner.result, b"original operand");
    }
}

/// Physical operations for the actual expression-result producer. The shared
/// executor owns probe selection, numeric COW, failure order and API copying;
/// adapters supply only their checked original object operations.
pub trait NativeBooleanExpressionResultOps: NativeBooleanTruthOps {
    /// An actual owned expression result, independently of a public ABI tag.
    type ResultValue;
    /// C's actual Number primitive, with its reached original cache changes.
    /// # Errors
    /// Refuses unavailable original object or native Number purpose.
    fn original_number_probe(
        &mut self,
        value: &Self::Value,
        protocol: tcl_syntax::native_boolean_truth::NativeBooleanExpressionResultProtocol,
    ) -> Result<(), CmdError>;
    /// Inspect the original native reference count before acquiring output ownership.
    /// # Errors
    /// Refuses a retired or foreign original header.
    fn original_is_shared(&mut self, value: &Self::Value) -> Result<bool, CmdError>;
    /// Create the reached fresh absent-String numeric result under its original
    /// selected constructor, without materialising or changing the input.
    /// # Errors
    /// Refuses unsupported selected numeric constructor/header recipes.
    fn copy_numeric_result(
        &mut self,
        value: &Self::Value,
        cache: NativeScalarCache,
        protocol: tcl_syntax::native_boolean_truth::NativeBooleanExpressionResultProtocol,
    ) -> Result<Self::ResultValue, CmdError>;
    /// Withdraw String from the genuinely unshared original numeric result.
    /// # Errors
    /// Refuses unavailable original ownership or numeric representation.
    fn invalidate_numeric_string(
        &mut self,
        value: &Self::Value,
        protocol: tcl_syntax::native_boolean_truth::NativeBooleanExpressionResultProtocol,
    ) -> Result<(), CmdError>;
    /// Retain the same original result only after conversion/COW decisions.
    /// # Errors
    /// Refuses a retired or foreign original header.
    fn retain_original_result(
        &mut self,
        value: &Self::Value,
    ) -> Result<Self::ResultValue, CmdError>;
    /// Copy the actual successful evaluated result through its API result producer.
    /// # Errors
    /// Refuses unavailable original selected object duplication.
    fn copy_api_result(
        &mut self,
        value: &Self::ResultValue,
        protocol: tcl_syntax::native_boolean_truth::NativeBooleanExpressionResultProtocol,
    ) -> Result<Self::ResultValue, CmdError>;
    /// Inspect the reached nonfinite expression failure at its selected point.
    /// C8.4's actual host errno is independently required for infinity.
    /// # Errors
    /// Refuses missing actual numeric environment facts outside guest completion.
    fn nonfinite_expression_failure(
        &mut self,
        value: f64,
        protocol: tcl_syntax::native_boolean_truth::NativeBooleanExpressionResultProtocol,
    ) -> Result<Option<CmdError>, CmdError>;
}

fn settled<O: NativeBooleanTruthOps, T>(
    ops: &mut O,
    result: Result<T, CmdError>,
) -> Result<T, CmdError> {
    ops.check_host_refusal()?;
    result
}

fn legacy_result_probe<O: NativeBooleanTruthOps>(
    value: &O::Value,
    truth: NativeBooleanTruthProtocol,
    current: Option<NativeScalarCache>,
    resident: bool,
    ops: &mut O,
) -> Result<(), CmdError> {
    use tcl_syntax::number::Number;
    let scalar = truth.scalar_protocol();
    let known = matches!(
        current,
        Some(NativeScalarCache::Tcl84Long(_) | NativeScalarCache::Number(Number::Int(_)))
    ) || !resident
        && matches!(
            current,
            Some(NativeScalarCache::Number(
                Number::Double(_) | Number::Nan { .. }
            ))
        );
    if !known {
        let stage = if !resident && let Some(NativeScalarCache::WordBoolean(boolean)) = current {
            NativeBooleanTruthProbe::ExpressionWordBoolean84(boolean)
        } else {
            let original = ops.original_string(value, truth);
            let original = settled(ops, original)?;
            if scalar.expression_integer_spelling84(&original) {
                NativeBooleanTruthProbe::ExpressionInteger84
            } else {
                NativeBooleanTruthProbe::Scalar {
                    kind: NativeScalarGetterKind::Double,
                    effects: NativeBooleanTruthGetterEffects::ErrorNeutral,
                }
            }
        };
        let conversion = ops.probe(value, truth, stage);
        // TRY_NUM's ordinary guest rejection preserves its reached cache
        // and same nonnumeric header; its Host refusal remains terminal.
        let _outcome = settled(ops, conversion)?;
    }
    Ok(())
}

/// Perform the selected outer result producer on a genuine evaluated operand.
/// A public tag selects an operation; this function actually performs it before
/// returning its owned result. No extra retained input changes the COW decision.
/// # Errors
/// Retains first Host failures and reached nonfinite expression diagnostics;
/// ordinary unsuccessful TRY_NUM probes retain their original nonnumeric value.
pub fn original_boolean_expression_result<O: NativeBooleanExpressionResultOps>(
    value: &O::Value,
    protocol: tcl_syntax::native_boolean_truth::NativeBooleanExpressionResultProtocol,
    ops: &mut O,
) -> Result<O::ResultValue, CmdError> {
    use tcl_syntax::{native_boolean_truth::NativeBooleanTruthProtocol, number::Number};
    ops.check_host_refusal()?;
    let scalar = protocol.scalar_protocol();
    let truth = NativeBooleanTruthProtocol::for_scalar_getter(scalar, protocol.truth_purpose());
    let observation = ops.inspect_original(value, truth);
    let (current, resident) = settled(ops, observation)?;
    let mut output = None;
    if protocol.converts_numeric_result() {
        let legacy = scalar.tcl_version() == Some(tcl_dialect::TclVersion::V8_4);
        if legacy {
            legacy_result_probe(value, truth, current, resident, ops)?;
        } else {
            let conversion = ops.original_number_probe(value, protocol);
            settled(ops, conversion)?;
        }
        let observation = ops.inspect_original(value, truth);
        let (cache, resident) = settled(ops, observation)?;
        let numeric = cache.filter(|cache| {
            matches!(
                cache,
                NativeScalarCache::Tcl84Long(_) | NativeScalarCache::Number(_)
            )
        });
        if let Some(cache) = numeric {
            let double = match &cache {
                NativeScalarCache::Number(Number::Double(double)) => Some(*double),
                NativeScalarCache::Number(Number::Nan { .. }) => Some(f64::NAN),
                _ => None,
            };
            if !legacy && let Some(double) = double {
                let failure = ops.nonfinite_expression_failure(double, protocol);
                if let Some(failure) = settled(ops, failure)? {
                    return Err(failure);
                }
            }
            let shared = ops.original_is_shared(value);
            if settled(ops, shared)? && resident {
                let duplicate = ops.copy_numeric_result(value, cache, protocol);
                output = Some(settled(ops, duplicate)?);
            } else if resident {
                let invalidation = ops.invalidate_numeric_string(value, protocol);
                settled(ops, invalidation)?;
            }
            // C8.4 checks nonfinite arithmetic after its original numeric COW.
            if legacy && let Some(double) = double {
                let failure = ops.nonfinite_expression_failure(double, protocol);
                if let Some(failure) = settled(ops, failure)? {
                    return Err(failure);
                }
            }
        }
    }
    let output = if let Some(output) = output {
        output
    } else {
        let retained = ops.retain_original_result(value);
        settled(ops, retained)?
    };
    if protocol.copies_api_result() {
        let copied = ops.copy_api_result(&output, protocol);
        settled(ops, copied)
    } else {
        Ok(output)
    }
}
