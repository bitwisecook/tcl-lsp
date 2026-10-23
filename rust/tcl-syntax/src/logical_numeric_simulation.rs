// SPDX-License-Identifier: AGPL-3.0-or-later
//! Explicit logical simulation grammar, independent of native getter evidence.
//!
//! An embedding host supplies this capability deliberately. The registry checks
//! its logical dialect axes; it does not infer the capability from a missing
//! native getter, a compatible release, or the host's physical compiler engine.

use crate::number::{Number, NumberSyntax, ParseFlags};

/// An authored simulation contract, carrying no native execution attestation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AuthoredLogicalNumericSimulation {
    /// Simulate the documented Tcl 8.4 numeric core for an F5 logical dialect.
    Tcl84Core,
}

/// Numeric parsing stage; integer width and arithmetic remain separate owners.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalNumericInputStage {
    /// Recognize a complete numeric value, preserving its category.
    Number,
    /// Recognize integer syntax only, preserving the exact magnitude.
    Integer,
}

/// Boolean simulation stages must not borrow primitive native Boolean rules.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalBooleanInputStage {
    /// A command option or value conversion rejects a NaN numeric value.
    BooleanValue,
    /// Tcl 8.4 logical expression truth treats NaN as nonzero.
    NumericTruth,
}

/// Logical origin retained for the adapter's separate simulation cache policy.
#[derive(Debug, Clone, PartialEq)]
pub enum LogicalBooleanOrigin {
    /// A boolean word or unambiguous prefix, without numeric cache authority.
    Word,
    /// Numeric truth retains the parsed category and exact integer magnitude.
    Numeric(Number),
}

/// Successful logical truth and the conversion origin, without native intrep proof.
#[derive(Debug, Clone, PartialEq)]
pub struct LogicalBooleanValue {
    value: bool,
    origin: LogicalBooleanOrigin,
}

impl LogicalBooleanValue {
    /// The logical result alone; this grants no native cache conversion.
    #[must_use]
    pub const fn value(&self) -> bool {
        self.value
    }

    /// Consume the logical result while retaining its simulation cache origin.
    #[must_use]
    pub fn into_parts(self) -> (bool, LogicalBooleanOrigin) {
        (self.value, self.origin)
    }
}

/// A logical simulation failure, never a primitive native error-state receipt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogicalNumericSimulationFailure {
    /// Complete numeric syntax was not recognized.
    InvalidNumber,
    /// Complete integer syntax was not recognized.
    InvalidInteger,
    /// Neither a boolean word nor complete numeric syntax was recognized.
    InvalidBoolean,
    /// The `BooleanValue` stage rejects NaN; `NumericTruth` remains independent.
    NaNBooleanValue,
}

impl AuthoredLogicalNumericSimulation {
    /// Interpret an independently current numeric cache under the authored
    /// logical stage. Both Tcl 8.4 stages treat cached NaN as nonzero; fresh
    /// `BooleanValue` parsing remains separately fallible. This supplies no
    /// native cache transition, string preparation or primitive error receipt.
    #[must_use]
    pub fn current_number_boolean(
        self,
        number: Number,
        stage: LogicalBooleanInputStage,
    ) -> LogicalBooleanValue {
        let value = match (self, stage) {
            (
                Self::Tcl84Core,
                LogicalBooleanInputStage::BooleanValue | LogicalBooleanInputStage::NumericTruth,
            ) => number_truth(&number),
        };
        LogicalBooleanValue {
            value,
            origin: LogicalBooleanOrigin::Numeric(number),
        }
    }

    /// Parse complete original bytes using the authored grammar. This does not
    /// truncate at NUL, materialize an object, or publish a native cache/error.
    pub fn parse_number(
        self,
        original: &[u8],
        stage: LogicalNumericInputStage,
    ) -> Result<Number, LogicalNumericSimulationFailure> {
        let failure = match stage {
            LogicalNumericInputStage::Number => LogicalNumericSimulationFailure::InvalidNumber,
            LogicalNumericInputStage::Integer => LogicalNumericSimulationFailure::InvalidInteger,
        };
        let text = std::str::from_utf8(original).map_err(|_| failure)?;
        let mut flags = ParseFlags::for_syntax(match self {
            Self::Tcl84Core => NumberSyntax::Tcl84,
        });
        flags.integer_only = stage == LogicalNumericInputStage::Integer;
        let number = crate::number::parse_whole_with(text, flags).ok_or(failure)?;
        if stage == LogicalNumericInputStage::Integer
            && !matches!(number, Number::Int(_) | Number::Big { .. })
        {
            return Err(failure);
        }
        Ok(number)
    }

    /// Select the logical boolean stage and retain Word-versus-Numeric origin.
    /// Invalid bytes are a logical spelling failure, with the original object
    /// and full diagnostic bytes left to the simulation completion owner.
    pub fn parse_boolean(
        self,
        original: &[u8],
        stage: LogicalBooleanInputStage,
    ) -> Result<LogicalBooleanValue, LogicalNumericSimulationFailure> {
        let text = std::str::from_utf8(original)
            .map_err(|_| LogicalNumericSimulationFailure::InvalidBoolean)?;
        if let Some(value) = crate::boolean::parse_boolean_word(text) {
            return Ok(LogicalBooleanValue {
                value,
                origin: LogicalBooleanOrigin::Word,
            });
        }
        let number = self
            .parse_number(original, LogicalNumericInputStage::Number)
            .map_err(|_| LogicalNumericSimulationFailure::InvalidBoolean)?;
        if stage == LogicalBooleanInputStage::BooleanValue
            && (matches!(&number, Number::Nan { .. })
                || matches!(&number, Number::Double(value) if value.is_nan()))
        {
            return Err(LogicalNumericSimulationFailure::NaNBooleanValue);
        }
        Ok(self.current_number_boolean(number, stage))
    }
}

fn number_truth(number: &Number) -> bool {
    match number {
        Number::Int(value) => *value != 0,
        Number::Big { .. } | Number::Nan { .. } => true,
        Number::Double(value) => *value != 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn logical_number_grammar_keeps_full_bytes_and_exact_magnitude() {
        let protocol = AuthoredLogicalNumericSimulation::Tcl84Core;
        let stage = LogicalNumericInputStage::Number;
        assert_eq!(protocol.parse_number(b"077", stage), Ok(Number::Int(63)));
        for bytes in [
            b"08".as_slice(),
            b"0b10",
            b"0o10",
            b"0d10",
            b"1\0suffix",
            b"1\xff",
        ] {
            assert_eq!(
                protocol.parse_number(bytes, stage),
                Err(LogicalNumericSimulationFailure::InvalidNumber)
            );
        }
        assert!(matches!(
            protocol.parse_number(b"18446744073709551616", LogicalNumericInputStage::Integer),
            Ok(Number::Big { .. })
        ));
        for bytes in [b"1.5".as_slice(), b"NaN", b"Inf"] {
            assert_eq!(
                protocol.parse_number(bytes, LogicalNumericInputStage::Integer),
                Err(LogicalNumericSimulationFailure::InvalidInteger)
            );
        }
    }

    #[test]
    fn logical_boolean_stage_retains_word_numeric_and_nan_distinctions() {
        let protocol = AuthoredLogicalNumericSimulation::Tcl84Core;
        let stage = LogicalBooleanInputStage::BooleanValue;
        assert_eq!(
            protocol.parse_boolean(b"tru", stage).unwrap().into_parts(),
            (true, LogicalBooleanOrigin::Word)
        );
        assert_eq!(
            protocol.parse_boolean(b"2", stage).unwrap().into_parts(),
            (true, LogicalBooleanOrigin::Numeric(Number::Int(2)))
        );
        assert_eq!(
            protocol.parse_boolean(b"NaN", stage),
            Err(LogicalNumericSimulationFailure::NaNBooleanValue)
        );
        assert!(
            protocol
                .parse_boolean(b"NaN", LogicalBooleanInputStage::NumericTruth)
                .unwrap()
                .value()
        );
        for bytes in [b"o".as_slice(), b"true\0suffix", b"\xff"] {
            assert_eq!(
                protocol.parse_boolean(bytes, stage),
                Err(LogicalNumericSimulationFailure::InvalidBoolean)
            );
        }
    }

    #[test]
    fn current_numeric_boolean_is_independent_of_fresh_spelling_and_native_cache() {
        let protocol = AuthoredLogicalNumericSimulation::Tcl84Core;
        for stage in [
            LogicalBooleanInputStage::BooleanValue,
            LogicalBooleanInputStage::NumericTruth,
        ] {
            let result = protocol.current_number_boolean(Number::Double(f64::NAN), stage);
            assert!(result.value());
            assert!(
                matches!(result.into_parts(), (true, LogicalBooleanOrigin::Numeric(Number::Double(value))) if value.is_nan())
            );
            assert!(
                !protocol
                    .current_number_boolean(Number::Int(0), stage)
                    .value()
            );
            assert!(
                protocol
                    .current_number_boolean(Number::Double(1.5), stage)
                    .value()
            );
        }
        assert_eq!(
            protocol.parse_boolean(b"NaN", LogicalBooleanInputStage::BooleanValue),
            Err(LogicalNumericSimulationFailure::NaNBooleanValue)
        );
    }
}
