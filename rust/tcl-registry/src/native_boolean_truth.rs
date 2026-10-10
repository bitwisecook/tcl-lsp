// SPDX-License-Identifier: AGPL-3.0-or-later
//! Authentic selected Boolean conversion sites, separately from source grammar.

pub use tcl_syntax::native_boolean_truth::{
    NativeBooleanExpressionResultProduction, NativeBooleanExpressionResultProtocol,
    NativeBooleanTruthProtocol, NativeBooleanTruthPurpose,
};

impl crate::InvocationDialect {
    /// Select the actual expression-result conversion and API copy recipe.
    /// The physical executor must perform this producer before it grants an
    /// owned Boolean operand receipt; a profile or public tag cannot do so.
    #[must_use]
    pub fn native_boolean_expression_result_protocol(
        self,
        production: NativeBooleanExpressionResultProduction,
    ) -> Option<NativeBooleanExpressionResultProtocol> {
        self.native_scalar_getter_protocol().map(|scalar| {
            NativeBooleanExpressionResultProtocol::for_scalar_getter(scalar, production)
        })
    }

    /// Select the actual retained engine's reached Boolean operand conversion.
    /// Source compatibility, catalogue availability and supplied unknown points
    /// cannot donate a physical getter or expression-result producer.
    #[must_use]
    pub fn native_boolean_truth_protocol(
        self,
        purpose: NativeBooleanTruthPurpose,
    ) -> Option<NativeBooleanTruthProtocol> {
        self.native_scalar_getter_protocol()
            .map(|scalar| NativeBooleanTruthProtocol::for_scalar_getter(scalar, purpose))
    }

    /// The original compiler's final logical operand instruction. The first
    /// operand's short-circuit jump has a separate conversion site. This pure
    /// selection requires the independently admitted original compiler route.
    #[must_use]
    pub fn native_logical_final_operand_purpose(
        self,
        conjunction: bool,
    ) -> Option<NativeBooleanTruthPurpose> {
        let scalar = self.native_scalar_getter_protocol()?;
        Some(
            if scalar.tcl_version() == Some(tcl_dialect::TclVersion::V8_4) {
                if conjunction {
                    NativeBooleanTruthPurpose::LogicalAndInstruction
                } else {
                    NativeBooleanTruthPurpose::LogicalOrInstruction
                }
            } else if conjunction {
                NativeBooleanTruthPurpose::LogicalAnd
            } else {
                NativeBooleanTruthPurpose::LogicalOr
            },
        )
    }
}
