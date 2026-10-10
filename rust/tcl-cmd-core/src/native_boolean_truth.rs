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
        failure: NativeScalarGetterFailure,
    ) -> Result<CmdError, CmdError>;
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
    for (index, stage) in stages.into_iter().enumerate() {
        ops.check_host_refusal()?;
        let outcome = ops.probe(value, protocol, stage);
        ops.check_host_refusal()?;
        let outcome = outcome?;
        let failure = match outcome {
            Ok(result) => return returned_truth(stage, result),
            Err(failure) => failure,
        };
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
            let failure = ops.logical_operand_failure(value, protocol, failure);
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
            _: NativeScalarGetterFailure,
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
