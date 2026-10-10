// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reached Boolean operand conversions, independently of expression evaluation.
//!
//! A recipe describes conversion of the original operand. It supplies no
//! native object, invocation, source, result-normalisation or effect authority.
//! An expression API or command condition supplies its evaluated result through
//! its separate producer before using the corresponding conversion purpose.

use tcl_dialect::TclVersion;

use crate::number::Number;
use crate::scalar_getter::{NativeScalarCache, NativeScalarGetterKind, NativeScalarGetterProtocol};

/// Actual reached conversion site, independent of mathematical value type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NativeBooleanTruthPurpose {
    /// The operand of a reached logical-not instruction.
    LogicalNot = 0,
    /// A reached expression conditional jump, including a ternary condition.
    ConditionalJump = 1,
    /// The short-circuit branch of a conjunction expression.
    LogicalAnd = 2,
    /// The short-circuit branch of a disjunction expression.
    LogicalOr = 3,
    /// Boolean extraction after the command's expression result producer.
    CommandCondition = 4,
    /// Boolean extraction after the public expression API's result producer.
    ExpressionApiResult = 5,
    /// C8.4's reached non-short-circuit logical-and instruction.
    LogicalAndInstruction = 6,
    /// C8.4's reached non-short-circuit logical-or instruction.
    LogicalOrInstruction = 7,
}

impl NativeBooleanTruthPurpose {
    /// Decode a counted ABI tag without selecting a default conversion.
    #[must_use]
    pub const fn from_abi(tag: i32) -> Option<Self> {
        match tag {
            0 => Some(Self::LogicalNot),
            1 => Some(Self::ConditionalJump),
            2 => Some(Self::LogicalAnd),
            3 => Some(Self::LogicalOr),
            4 => Some(Self::CommandCondition),
            5 => Some(Self::ExpressionApiResult),
            6 => Some(Self::LogicalAndInstruction),
            7 => Some(Self::LogicalOrInstruction),
            _ => None,
        }
    }

    /// Whether the caller supplies an independently evaluated expression result.
    #[must_use]
    pub const fn requires_expression_result(self) -> bool {
        matches!(self, Self::CommandCondition | Self::ExpressionApiResult)
    }
}

/// Interpreter result effect of a reached unsuccessful primitive getter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBooleanTruthGetterEffects {
    /// The original producer passed NULL or used JimGetWideNoErr.
    ErrorNeutral,
    /// The actual getter reports its diagnostic even before a later success.
    ReportGuestFailure,
}

/// A reached operand probe. Numeric parsing remains with the scalar owner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBooleanTruthProbe {
    /// The selected existing scalar getter and its actual effects contract.
    Scalar {
        /// Primitive conversion reached on the original operand.
        kind: NativeScalarGetterKind,
        /// Whether a failure updates actual interpreter state immediately.
        effects: NativeBooleanTruthGetterEffects,
    },
    /// C8.4 GET_WIDE_OR_INT; retain its native-long versus wide cache recipe.
    ExpressionInteger84,
    /// C8.4 LNOT converts a stringless Boolean primary to native long in place.
    ExpressionWordBoolean84(bool),
}

/// The genuine final failure producer after all reached conversion probes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBooleanTruthFailureProducer {
    /// Keep the last reached primitive's own diagnostic and state update.
    ScalarGetter,
    /// C IllegalExprOperandType, independently of primitive getter wording.
    LogicalOperand,
}

/// Selected conversion sequence after independently inspecting the original.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeBooleanTruthPreparation {
    /// Read an existing native numeric/word cache without string generation.
    Complete(bool),
    /// The original C8.4 spelling must be read before selecting its first probe.
    OriginalStringRequired,
    /// Execute probes in order until one succeeds; apply every reached cache
    /// and required diagnostic update before continuing or returning.
    Probes {
        /// Exact selected conversion stages.
        stages: Vec<NativeBooleanTruthProbe>,
        /// Actual final failure producer, if no stage succeeds.
        failure: NativeBooleanTruthFailureProducer,
    },
}

/// Pure selected engine/purpose recipe; it never attests an original operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeBooleanTruthProtocol {
    scalar: NativeScalarGetterProtocol,
    purpose: NativeBooleanTruthPurpose,
}

/// The actual result producer before Boolean conversion, independently of the
/// source operator or a public caller's assertion that a value is normalised.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(i32)]
pub enum NativeBooleanExpressionResultProduction {
    /// The selected expression compiler's reached result conversion.
    InlineExpression = 0,
    /// The evaluated result returned through the public expression API.
    PublicExpressionApi = 1,
}

impl NativeBooleanExpressionResultProduction {
    /// Decode the requested producer; an unknown tag selects no operation.
    #[must_use]
    pub const fn from_abi(tag: i32) -> Option<Self> {
        match tag {
            0 => Some(Self::InlineExpression),
            1 => Some(Self::PublicExpressionApi),
            _ => None,
        }
    }
}

/// Selected original result conversion and public API copy policy. The caller
/// performs these operations on a genuine retained result before truth extraction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeBooleanExpressionResultProtocol {
    scalar: NativeScalarGetterProtocol,
    production: NativeBooleanExpressionResultProduction,
}

impl NativeBooleanExpressionResultProtocol {
    /// Retain the independently selected engine and genuine reached producer.
    #[must_use]
    pub const fn for_scalar_getter(
        scalar: NativeScalarGetterProtocol,
        production: NativeBooleanExpressionResultProduction,
    ) -> Self {
        Self { scalar, production }
    }

    /// Original scalar/number owner of the selected result conversion.
    #[must_use]
    pub const fn scalar_protocol(self) -> NativeScalarGetterProtocol {
        self.scalar
    }

    /// Exact retained producer, independently of normalisation outcome.
    #[must_use]
    pub const fn production(self) -> NativeBooleanExpressionResultProduction {
        self.production
    }

    /// Whether the selected outer Boolean result producer retains TRY_NUM.
    /// C8.6+ peephole compilation removes it before conditional jumps; the
    /// public expression API still evaluates its independent expression program.
    /// Jim evaluates its original term without borrowing either C operation.
    #[must_use]
    pub fn converts_numeric_result(self) -> bool {
        self.scalar.tcl_version().is_some_and(|version| {
            self.production == NativeBooleanExpressionResultProduction::PublicExpressionApi
                || version < TclVersion::V8_6
        })
    }

    /// C8.6+ public Tcl_ExprObj copies the evaluated result through
    /// ExprObjCallback/TclSetDuplicateObj, after successful evaluation, including a nonnumeric result.
    /// An inline compiled expression does not perform this API copy.
    #[must_use]
    pub fn copies_api_result(self) -> bool {
        self.production == NativeBooleanExpressionResultProduction::PublicExpressionApi
            && self
                .scalar
                .tcl_version()
                .is_some_and(|version| version >= TclVersion::V8_6)
    }

    /// The actual Boolean conversion site reached after this producer.
    #[must_use]
    pub const fn truth_purpose(self) -> NativeBooleanTruthPurpose {
        match self.production {
            NativeBooleanExpressionResultProduction::InlineExpression => {
                NativeBooleanTruthPurpose::ConditionalJump
            }
            NativeBooleanExpressionResultProduction::PublicExpressionApi => {
                NativeBooleanTruthPurpose::ExpressionApiResult
            }
        }
    }
}

impl NativeBooleanTruthProtocol {
    /// Retain the independently selected scalar engine and the reached purpose.
    #[must_use]
    pub const fn for_scalar_getter(
        scalar: NativeScalarGetterProtocol,
        purpose: NativeBooleanTruthPurpose,
    ) -> Self {
        Self { scalar, purpose }
    }

    /// Primitive owner used by the selected sequence; this grants no getter call.
    #[must_use]
    pub const fn scalar_protocol(self) -> NativeScalarGetterProtocol {
        self.scalar
    }

    /// Exact retained conversion site.
    #[must_use]
    pub const fn purpose(self) -> NativeBooleanTruthPurpose {
        self.purpose
    }

    /// JimExprGetTermBoolean installs the evaluated original term before
    /// ExprBool. Numeric unary logical-not has its separate original path.
    #[must_use]
    pub const fn publishes_original_operand(self) -> bool {
        self.scalar.is_jim084() && !matches!(self.purpose, NativeBooleanTruthPurpose::LogicalNot)
    }

    /// Prepare from actual current cache/String observations. Supplying bytes
    /// means the original checked String getter has already completed; they
    /// cannot substitute for a cache, source or normalisation receipt.
    #[must_use]
    pub fn prepare(
        self,
        cache: Option<&NativeScalarCache>,
        string_resident: bool,
        original: Option<&[u8]>,
    ) -> NativeBooleanTruthPreparation {
        use NativeBooleanTruthFailureProducer::{LogicalOperand, ScalarGetter};
        use NativeBooleanTruthGetterEffects::{ErrorNeutral, ReportGuestFailure};
        use NativeBooleanTruthPreparation::{Complete, OriginalStringRequired, Probes};
        use NativeBooleanTruthPurpose as Purpose;
        use NativeScalarGetterKind::{Boolean, Double, Long, Wide};

        let scalar = |kind, effects| NativeBooleanTruthProbe::Scalar { kind, effects };
        if self.scalar.is_jim084() {
            if self.purpose == Purpose::LogicalNot {
                let mut stages = Vec::with_capacity(3);
                if string_resident
                    || !matches!(
                        cache,
                        Some(
                            NativeScalarCache::Number(Number::Double(_) | Number::Nan { .. })
                                | NativeScalarCache::JimCoercedInteger(_)
                        )
                    )
                {
                    stages.push(scalar(Wide, ErrorNeutral));
                }
                stages.extend([
                    scalar(Double, ReportGuestFailure),
                    scalar(Boolean, ReportGuestFailure),
                ]);
                return Probes {
                    stages,
                    failure: ScalarGetter,
                };
            }
            return Probes {
                stages: vec![
                    scalar(Long, ReportGuestFailure),
                    scalar(Double, ReportGuestFailure),
                    scalar(Boolean, ReportGuestFailure),
                ],
                failure: ScalarGetter,
            };
        }

        let logical_instruction = matches!(
            self.purpose,
            Purpose::LogicalNot | Purpose::LogicalAndInstruction | Purpose::LogicalOrInstruction
        );
        if self.scalar.tcl_version() == Some(TclVersion::V8_4) {
            if let Some(truth) = cached_truth84(cache, string_resident, self.purpose) {
                return Complete(truth);
            }
            if logical_instruction {
                if self.purpose == Purpose::LogicalNot
                    && !string_resident
                    && let Some(NativeScalarCache::WordBoolean(boolean)) = cache
                {
                    return Probes {
                        stages: vec![NativeBooleanTruthProbe::ExpressionWordBoolean84(*boolean)],
                        failure: LogicalOperand,
                    };
                }
                let Some(original) = original else {
                    return OriginalStringRequired;
                };
                let integer = self.scalar.expression_integer_spelling84(original);
                let mut stages = Vec::with_capacity(2);
                if integer {
                    stages.push(NativeBooleanTruthProbe::ExpressionInteger84);
                } else if self.purpose == Purpose::LogicalNot {
                    stages.push(scalar(Double, ErrorNeutral));
                }
                if !integer || self.purpose == Purpose::LogicalNot {
                    stages.push(scalar(Boolean, ErrorNeutral));
                }
                return Probes {
                    stages,
                    failure: LogicalOperand,
                };
            }
        }
        Probes {
            stages: vec![scalar(
                Boolean,
                if logical_instruction {
                    ErrorNeutral
                } else {
                    ReportGuestFailure
                },
            )],
            failure: if logical_instruction {
                LogicalOperand
            } else {
                ScalarGetter
            },
        }
    }
}

fn cached_truth84(
    cache: Option<&NativeScalarCache>,
    string_resident: bool,
    purpose: NativeBooleanTruthPurpose,
) -> Option<bool> {
    let evaluated = purpose.requires_expression_result();
    let not_needs_spelling = purpose == NativeBooleanTruthPurpose::LogicalNot && string_resident;
    match cache? {
        NativeScalarCache::Tcl84Long(integer) => Some(*integer != 0),
        NativeScalarCache::Number(Number::Int(integer)) if !evaluated => Some(*integer != 0),
        NativeScalarCache::Number(Number::Double(double)) if !not_needs_spelling => {
            Some(*double != 0.0)
        }
        NativeScalarCache::Number(Number::Nan { .. }) if !not_needs_spelling => Some(true),
        NativeScalarCache::WordBoolean(boolean)
            if matches!(
                purpose,
                NativeBooleanTruthPurpose::LogicalAndInstruction
                    | NativeBooleanTruthPurpose::LogicalOrInstruction
            ) =>
        {
            Some(*boolean)
        }
        NativeScalarCache::Number(
            Number::Int(_) | Number::Big { .. } | Number::Double(_) | Number::Nan { .. },
        )
        | NativeScalarCache::JimCoercedInteger(_)
        | NativeScalarCache::WordBoolean(_) => None,
    }
}

/// Original instruction failure class, separately from primitive getter text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeBooleanLogicalOperandClass {
    /// Reached nonnumeric counted spelling.
    NonNumericString,
    /// C8.4–8.6's distinct physical empty-spelling diagnostic.
    EmptyString,
    /// C8.4–8.6's independently observed bad-octal spelling.
    InvalidOctal,
    /// C8.4's reached integer range failure.
    IntegerOverflow,
    /// The original numeric NaN primary or exact selected classification.
    NonNumericFloatingPoint,
    /// C8.4's exact infinite-spelling classification.
    InfiniteFloatingPoint,
    /// An original rejected finite floating-point value.
    FloatingPoint,
    /// C9's actually reached original List classification.
    List,
}

/// Original instruction error bytes; these grant no publication or input read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeBooleanLogicalOperandDiagnostic {
    /// Exact selected instruction result.
    pub message: Vec<u8>,
    /// Exact selected error-code store, independently of rendered wording.
    pub error_code: Vec<u8>,
}

impl NativeBooleanTruthProtocol {
    /// Render only an actually classified failed C instruction operand.
    /// Original bytes are required where the original message reads them;
    /// absent unused bytes remain distinct from a fabricated empty spelling.
    #[must_use]
    pub fn logical_failure_presentation(
        self,
        class: NativeBooleanLogicalOperandClass,
        original: Option<&[u8]>,
    ) -> Option<NativeBooleanLogicalOperandDiagnostic> {
        use NativeBooleanLogicalOperandClass as Class;
        use NativeBooleanTruthPurpose as Purpose;
        let version = self.scalar.tcl_version()?;
        let operator = match self.purpose {
            Purpose::LogicalNot => b"!".as_slice(),
            Purpose::LogicalAndInstruction if version == TclVersion::V8_4 => b"&&",
            Purpose::LogicalOrInstruction if version == TclVersion::V8_4 => b"||",
            _ => return None,
        };
        let class = if version >= TclVersion::V9_0
            && matches!(class, Class::EmptyString | Class::InvalidOctal)
        {
            Class::NonNumericString
        } else {
            class
        };
        let description = match class {
            Class::NonNumericString => b"non-numeric string".as_slice(),
            Class::EmptyString => b"empty string",
            Class::InvalidOctal => b"invalid octal number",
            Class::IntegerOverflow if version == TclVersion::V8_4 => {
                b"integer value too large to represent"
            }
            Class::NonNumericFloatingPoint => b"non-numeric floating-point value",
            Class::InfiniteFloatingPoint if version == TclVersion::V8_4 => {
                b"infinite floating-point value"
            }
            Class::FloatingPoint => b"floating-point value",
            Class::List if version >= TclVersion::V9_0 => b"list",
            Class::IntegerOverflow | Class::InfiniteFloatingPoint | Class::List => return None,
        };
        let mut message = if version >= TclVersion::V9_0 {
            b"cannot use ".to_vec()
        } else {
            b"can't use ".to_vec()
        };
        if class == Class::List {
            message.extend_from_slice(b"a list");
        } else {
            message.extend_from_slice(description);
        }
        if version >= TclVersion::V9_0 && class != Class::List {
            message.extend_from_slice(b" \"");
            message.extend_from_slice(tcl_core_types::c_string_extent(original?));
            message.push(b'"');
        }
        message.extend_from_slice(b" as operand of \"");
        message.extend_from_slice(operator);
        message.push(b'"');
        let error_code = if version == TclVersion::V8_4 {
            if class == Class::IntegerOverflow {
                b"ARITH IOVERFLOW {integer value too large to represent}".to_vec()
            } else {
                b"NONE".to_vec()
            }
        } else {
            let mut code = b"ARITH DOMAIN ".to_vec();
            if description.contains(&b' ') {
                code.push(b'{');
                code.extend_from_slice(description);
                code.push(b'}');
            } else {
                code.extend_from_slice(description);
            }
            code
        };
        Some(NativeBooleanLogicalOperandDiagnostic {
            message,
            error_code,
        })
    }
}
