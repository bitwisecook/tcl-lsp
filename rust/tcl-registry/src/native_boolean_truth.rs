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

//! Authentic selected Boolean conversion sites, separately from source grammar.

pub use tcl_syntax::native_boolean_truth::{
    NativeBooleanExpressionResultProduction, NativeBooleanExpressionResultProtocol,
    NativeBooleanTruthProtocol, NativeBooleanTruthPurpose, NativeExpressionResultProducer,
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

    /// Select an actually reached C numeric-result instruction, independent of
    /// a Boolean condition's peephole removal or a public expression API copy.
    #[must_use]
    pub fn native_numeric_instruction_result_protocol(
        self,
    ) -> Option<NativeBooleanExpressionResultProtocol> {
        self.native_scalar_getter_protocol()
            .and_then(NativeBooleanExpressionResultProtocol::for_numeric_instruction)
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
