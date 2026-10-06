// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Native numeric coercion diagnostics, separate from numeric acceptance.

/// Actual native presentation of an unparsable integer's original spelling.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeIntegerErrorPresentation {
    /// Quote the spelling; legacy cores limit the number of appended bytes.
    Quoted {
        /// Maximum spelling bytes appended; absent means retain the full spelling.
        limit: Option<usize>,
    },
    /// Jim 0.84 quotes the original bytes through its NUL-terminated `%s` door.
    Jim084Quoted,
    /// Modern C cores describe well-formed multiple-element values as a list.
    DescribeValue,
}

/// The actual failed conversion stage of a native expression operand.
///
/// This is selected by the reached operator's conversion protocol, not by
/// classifying an already formatted error message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeExpressionOperandStage {
    /// Integer-only conversion, including bitwise operands.
    Integer,
    /// Floating-point fallback in a numeric binary operator.
    FloatingPoint,
    /// Boolean conversion, including Jim's final numeric-unary fallback.
    Boolean,
}

/// Selected native expression conversion error formatting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeExpressionOperandErrorPresentation {
    /// Jim 0.84's measured conversion-stage diagnostics and NONE error code.
    Jim084,
}

/// Error-code policy for a reached expression operand conversion, independently
/// of primitive getter formatting, expression parsing and numeric acceptance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeExpressionOperandErrorCodePolicy {
    /// Legacy C8.4 and Jim supply the unstructured native NONE code.
    None,
    /// Later C cores retain the operand/type owner's structured native code.
    Structured,
}

/// Reached expression invalid-type update, independent of primitive getters.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeExpressionErrorCodeUpdate {
    /// No actual interpreter error-code store at this stage.
    Unchanged,
    /// Store this exact native code list.
    Set(Vec<u8>),
}

/// Actual invalid-type direct API failure and subsequent Eval propagation.
/// Arithmetic, NaN, domain, range and parser failures cannot use this record.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeExpressionInvalidTypeErrorCode {
    version: tcl_dialect::TclVersion,
    stage: NativeExpressionOperandStage,
    direct: NativeExpressionErrorCodeUpdate,
    eval: NativeExpressionErrorCodeUpdate,
}

impl NativeExpressionInvalidTypeErrorCode {
    /// Actual independently selected C release retained by this stage record.
    #[must_use]
    pub const fn tcl_version(&self) -> tcl_dialect::TclVersion {
        self.version
    }

    /// Independently reached operand conversion stage.
    #[must_use]
    pub const fn stage(&self) -> NativeExpressionOperandStage {
        self.stage
    }

    /// Update before a direct expression API returns its failure.
    #[must_use]
    pub const fn direct_update(&self) -> &NativeExpressionErrorCodeUpdate {
        &self.direct
    }

    /// Separate update when interpreter Eval propagates this failure.
    #[must_use]
    pub const fn eval_update(&self) -> &NativeExpressionErrorCodeUpdate {
        &self.eval
    }
}

impl NativeExpressionOperandErrorCodePolicy {
    /// Select metadata for a proved invalid scalar spelling or incompatible
    /// operand type, after its existing expression-stage diagnosis. Arithmetic,
    /// range, domain, NaN and parser errors must not use this projection.
    /// Messages and conversion results remain unchanged.
    #[must_use]
    pub fn select_invalid_type_code(self, structured: &[u8]) -> &[u8] {
        match self {
            Self::None => b"NONE",
            Self::Structured => structured,
        }
    }
}

impl crate::InvocationDialect {
    /// Retain an actual reached C invalid-type stage and its propagation update.
    /// The supplied structured code must come from that selected expression
    /// failure owner. This is neither a primitive getter nor a parser/NaN code
    /// rewrite. Jim's independent direct-API protocol is not authored here.
    #[must_use]
    pub fn expression_invalid_type_error_code(
        self,
        stage: NativeExpressionOperandStage,
        structured: &[u8],
    ) -> Option<NativeExpressionInvalidTypeErrorCode> {
        let version = self.native_scalar_getter_protocol()?.tcl_version()?;
        let (direct, eval) = if version == tcl_dialect::TclVersion::V8_4 {
            (
                NativeExpressionErrorCodeUpdate::Unchanged,
                NativeExpressionErrorCodeUpdate::Set(b"NONE".to_vec()),
            )
        } else {
            (
                NativeExpressionErrorCodeUpdate::Set(structured.to_vec()),
                NativeExpressionErrorCodeUpdate::Set(structured.to_vec()),
            )
        };
        Some(NativeExpressionInvalidTypeErrorCode {
            version,
            stage,
            direct,
            eval,
        })
    }

    /// Actual expression invalid/incompatible numeric or boolean operand type
    /// metadata. This selects neither arithmetic/range/domain/NaN failures nor
    /// primitive getter messages or parser error diagnostics.
    #[must_use]
    pub fn expression_operand_error_code_policy(
        self,
    ) -> Option<NativeExpressionOperandErrorCodePolicy> {
        use NativeExpressionOperandErrorCodePolicy as Policy;
        let protocol = self.native_scalar_getter_protocol()?;
        Some(
            if protocol.is_jim084() || protocol.tcl_version() == Some(tcl_dialect::TclVersion::V8_4)
            {
                Policy::None
            } else {
                Policy::Structured
            },
        )
    }

    /// Select expression-stage formatting independently of numeric acceptance.
    /// C cores retain their own operand/type presentation owner; an unknown
    /// native core point supplies no Jim formatting authority.
    #[must_use]
    pub fn expression_operand_error_presentation(
        self,
    ) -> Option<NativeExpressionOperandErrorPresentation> {
        (self.tcl_version.is_none()
            && self
                .core_point
                .is_some_and(|point| point.release() == tcl_dialect::model::Release::JIM_0_84))
        .then_some(NativeExpressionOperandErrorPresentation::Jim084)
    }

    /// Measured integer diagnostic protocol; it grants no numeric conversion.
    #[must_use]
    pub fn integer_error_presentation(self) -> Option<NativeIntegerErrorPresentation> {
        use NativeIntegerErrorPresentation as Presentation;
        self.tcl_version
            .map(|version| match version {
                tcl_dialect::TclVersion::V8_4 | tcl_dialect::TclVersion::V8_5 => {
                    Presentation::Quoted { limit: Some(50) }
                }
                tcl_dialect::TclVersion::V8_6 => Presentation::Quoted { limit: None },
                tcl_dialect::TclVersion::V9_0 | tcl_dialect::TclVersion::V9_1 => {
                    Presentation::DescribeValue
                }
            })
            .or_else(|| {
                self.core_point
                    .is_some_and(|point| point.release() == tcl_dialect::model::Release::JIM_0_84)
                    .then_some(Presentation::Jim084Quoted)
            })
    }
}

impl NativeIntegerErrorPresentation {
    /// Present a proved integer conversion failure, preserving native clipping.
    #[must_use]
    pub fn message(self, error: &tcl_syntax::value::ValueError) -> Vec<u8> {
        let bytes = match error {
            tcl_syntax::value::ValueError::NotInteger(spelling) => spelling.as_bytes(),
            tcl_syntax::value::ValueError::NotIntegerBytes(bytes) => bytes,
            _ => return error.message_bytes(),
        };
        let end = match self {
            Self::Quoted { limit } => limit.unwrap_or(bytes.len()).min(bytes.len()),
            Self::Jim084Quoted => bytes
                .iter()
                .position(|byte| *byte == 0)
                .unwrap_or(bytes.len()),
            Self::DescribeValue => return error.message_bytes(),
        };
        let mut message = b"expected integer but got \"".to_vec();
        message.extend_from_slice(&bytes[..end]);
        message.push(b'"');
        message
    }
}

impl NativeExpressionOperandErrorPresentation {
    /// Jim's numeric-unary rejection after the actual operand successfully
    /// converted to boolean. This supplies wording, never conversion evidence.
    #[must_use]
    pub fn non_numeric_unary_message(
        self,
        operation: tcl_syntax::expr::UnaryOp,
    ) -> Option<Vec<u8>> {
        let symbol = match operation {
            tcl_syntax::expr::UnaryOp::Pos => '+',
            tcl_syntax::expr::UnaryOp::Neg => '-',
            _ => return None,
        };
        match self {
            Self::Jim084 => Some(
                format!("can't use non-numeric string as operand of \"{symbol}\"").into_bytes(),
            ),
        }
    }

    /// Format a proved conversion failure from its original object bytes.
    /// Jim's native formatter preserves high bytes but its `%s` door stops at
    /// the first NUL. This does not parse or evaluate the spelling.
    #[must_use]
    pub fn message(self, stage: NativeExpressionOperandStage, spelling: &[u8]) -> Vec<u8> {
        match self {
            Self::Jim084 => {
                let expected: &[u8] = match stage {
                    NativeExpressionOperandStage::Integer => b"integer",
                    NativeExpressionOperandStage::FloatingPoint => b"floating-point number",
                    NativeExpressionOperandStage::Boolean => b"boolean",
                };
                let mut message = b"expected ".to_vec();
                message.extend_from_slice(expected);
                message.extend_from_slice(b" but got \"");
                let end = spelling
                    .iter()
                    .position(|byte| *byte == 0)
                    .unwrap_or(spelling.len());
                message.extend_from_slice(&spelling[..end]);
                message.push(b'"');
                message
            }
        }
    }

    /// Guest error metadata supplied by this measured conversion protocol.
    /// Operational host refusals never consume this guest presenter.
    #[must_use]
    pub const fn error_code(self) -> &'static [u8] {
        match self {
            Self::Jim084 => b"NONE",
        }
    }
}

#[cfg(test)]
mod tests {
    use tcl_syntax::value::ValueError;

    #[test]
    fn direct_invalid_type_error_state_is_distinct_from_eval_propagation() {
        use super::{
            NativeExpressionErrorCodeUpdate as Update, NativeExpressionOperandStage as Stage,
        };
        use tcl_dialect::TclVersion as V;
        let record = crate::InvocationDialect::for_version(V::V8_4)
            .expression_invalid_type_error_code(Stage::FloatingPoint, b"TCL VALUE NUMBER")
            .unwrap();
        assert_eq!(record.tcl_version(), V::V8_4);
        assert_eq!(record.direct_update(), &Update::Unchanged);
        assert_eq!(record.eval_update(), &Update::Set(b"NONE".to_vec()));
        for version in [V::V8_5, V::V8_6, V::V9_0, V::V9_1] {
            let record = crate::InvocationDialect::for_version(version)
                .expression_invalid_type_error_code(Stage::Boolean, b"TCL VALUE NUMBER")
                .unwrap();
            assert_eq!(
                record.direct_update(),
                &Update::Set(b"TCL VALUE NUMBER".to_vec())
            );
            assert_eq!(record.direct_update(), record.eval_update());
        }
        let jim = crate::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        assert_eq!(
            jim.expression_invalid_type_error_code(Stage::Integer, b"NONE"),
            None
        );
    }

    #[test]
    fn seeded_expression_native_stages_retain_ninety_observations() {
        let fixtures = [
            include_str!(
                "../../tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/8.4.20.txt"
            ),
            include_str!(
                "../../tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/8.5.19.txt"
            ),
            include_str!(
                "../../tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/8.6.18.txt"
            ),
            include_str!(
                "../../tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/9.0.4.txt"
            ),
            include_str!(
                "../../tcl-syntax/tests/data/native_scalar_getters/errors/expression_stage/9.1.0.txt"
            ),
        ];
        assert_eq!(
            fixtures
                .iter()
                .map(|fixture| fixture.lines().count())
                .sum::<usize>(),
            90
        );
        for (release, fixture) in fixtures.iter().enumerate() {
            for row in fixture.lines() {
                let field = |name| {
                    row.split_whitespace()
                        .find_map(|part| {
                            part.split_once('=')
                                .filter(|(key, _)| *key == name)
                                .map(|(_, value)| value)
                        })
                        .unwrap()
                };
                if field("code") != "1" || field("case") == "3" {
                    continue;
                }
                if release == 0 {
                    assert_eq!(
                        field("insideCode"),
                        if field("route") == "2" {
                            "4e4f4e45"
                        } else {
                            "50524f4245204245464f5245"
                        }
                    );
                    assert_eq!(field("propagatedCode"), "4e4f4e45");
                } else {
                    assert_eq!(field("insideOptionsCode"), field("propagatedCode"));
                }
            }
        }
    }

    #[test]
    fn expression_operand_codes_do_not_borrow_primitive_getter_or_modern_codes() {
        use tcl_dialect::TclVersion as V;
        for version in [V::V8_4, V::V8_5, V::V8_6, V::V9_0, V::V9_1] {
            let policy = crate::InvocationDialect::for_version(version)
                .expression_operand_error_code_policy()
                .unwrap();
            assert_eq!(
                policy.select_invalid_type_code(b"TCL VALUE NUMBER"),
                if version == V::V8_4 {
                    b"NONE".as_slice()
                } else {
                    b"TCL VALUE NUMBER"
                }
            );
        }
        let jim = crate::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        assert_eq!(
            jim.expression_operand_error_code_policy()
                .unwrap()
                .select_invalid_type_code(b"TCL VALUE NUMBER"),
            b"NONE"
        );
        let mut unknown = jim;
        unknown.core_point = None;
        assert_eq!(unknown.expression_operand_error_code_policy(), None);
    }

    #[test]
    fn jim_expression_conversion_stages_preserve_native_raw_formatting() {
        use super::NativeExpressionOperandStage as Stage;
        let jim = crate::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        let presentation = jim.expression_operand_error_presentation().unwrap();
        for (stage, expected) in [
            (
                Stage::Integer,
                b"expected integer but got \"\xff\"".as_slice(),
            ),
            (
                Stage::FloatingPoint,
                b"expected floating-point number but got \"\xff\"".as_slice(),
            ),
            (
                Stage::Boolean,
                b"expected boolean but got \"\xff\"".as_slice(),
            ),
        ] {
            assert_eq!(presentation.message(stage, b"\xff"), expected);
            assert_eq!(presentation.message(stage, b"\xff\0after"), expected);
        }
        assert_eq!(presentation.error_code(), b"NONE");
        for version in [
            tcl_dialect::TclVersion::V8_4,
            tcl_dialect::TclVersion::V8_5,
            tcl_dialect::TclVersion::V8_6,
            tcl_dialect::TclVersion::V9_0,
            tcl_dialect::TclVersion::V9_1,
        ] {
            assert_eq!(
                crate::InvocationDialect::for_version(version)
                    .expression_operand_error_presentation(),
                None,
            );
        }
        let mut unknown = jim;
        unknown.core_point = None;
        assert_eq!(unknown.expression_operand_error_presentation(), None);
    }

    #[test]
    fn native_integer_failure_presentation_uses_actual_release() {
        use tcl_dialect::TclVersion as V;
        for version in [V::V8_4, V::V8_5, V::V8_6, V::V9_0, V::V9_1] {
            let policy = crate::InvocationDialect::for_version(version)
                .integer_error_presentation()
                .unwrap();
            let message = policy.message(&ValueError::NotInteger("k 10".into()));
            assert_eq!(
                message,
                if version < V::V9_0 {
                    b"expected integer but got \"k 10\"".as_slice()
                } else {
                    b"expected integer but got a list".as_slice()
                }
            );
            let long = policy.message(&ValueError::NotInteger("A".repeat(70)));
            assert_eq!(long.len(), if version == V::V8_6 { 97 } else { 77 });
        }
        let jim = crate::InvocationDialect::of_profile(
            crate::model::ingress::resolve_environment("jim").unit_profile(),
        );
        assert_eq!(
            jim.integer_error_presentation()
                .unwrap()
                .message(&ValueError::NotInteger("k 10".into())),
            b"expected integer but got \"k 10\""
        );
        let raw = ValueError::NotIntegerBytes(b"\xff\0after".to_vec());
        assert_eq!(
            jim.integer_error_presentation().unwrap().message(&raw),
            b"expected integer but got \"\xff\"",
        );
        assert_eq!(
            crate::InvocationDialect::for_version(V::V8_6)
                .integer_error_presentation()
                .unwrap()
                .message(&raw),
            b"expected integer but got \"\xff\0after\"",
        );
    }
}
