// SPDX-License-Identifier: AGPL-3.0-or-later
//! Reached native operand conversions, independently of result setters and
//! primitive Wide extraction. Every recipe is conditional on an actual normal
//! integer-only numeric conversion of the original operand object.

use crate::{
    InvocationArguments, InvocationFacts, TclType,
    intrinsic::IntrinsicId,
    native_compilation::{
        NativeCompilationSelection, NativeCompilationWordShape, SuccessfulHandlerSpec,
    },
    runtime_expr_validation::PreparedExpressionWitness,
    semantic_operation::SemanticOperationId,
};
use tcl_dialect::TclVersion;
use tcl_syntax::expr::{BinOp, ExprNode, parser::NativeExprSyntax};

/// Selected stock-object integer contents conversion. Ingress must separately
/// prove a fresh ordinary literal lineage whose caches remain consistent with
/// its counted string. This grants no current cache, value or object identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeStockIntegerContentsProtocol {
    version: TclVersion,
}

impl NativeStockIntegerContentsProtocol {
    /// Authenticate the C bignum parser/getter recipe independently of lexer
    /// overrides. C8.4 and Jim require their own contents/range proofs.
    #[must_use]
    pub fn select(dialect: crate::InvocationDialect) -> Option<Self> {
        let version = dialect.native_scalar_getter_protocol()?.tcl_version()?;
        (version != TclVersion::V8_4 && dialect.numbers == version.number_syntax())
            .then_some(Self { version })
    }

    /// Whether the entire original spelling is an integer accepted by the
    /// selected stock expression and Increment conversions. Cache consistency
    /// is an independent prerequisite, never inferred by this parser.
    #[must_use]
    pub fn accepts_integer_contents(self, text: &str) -> bool {
        matches!(
            tcl_syntax::number::parse_whole_with(
                text,
                tcl_syntax::number::ParseFlags::for_syntax(self.version.number_syntax()),
            ),
            Some(tcl_syntax::number::Number::Int(_) | tcl_syntax::number::Number::Big { .. })
        )
    }

    /// Actual selected C release, independently of source pool provenance.
    #[must_use]
    pub const fn tcl_version(self) -> TclVersion {
        self.version
    }
}

/// Independently proved cache class of the original native operand object.
/// This is not a semantic type, string spelling, or class of an extracted
/// child. Source ingress must retain the exact current object evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeNumericOperandClass {
    /// Original object has no internal representation and retains a string.
    String,
    /// Original object has the ordinary native List representation.
    List,
    /// Original object has the ordinary native Dict representation.
    Dict,
    /// Original object has the native `ByteArray` representation.
    ByteArray,
    /// Original object has a current native integer representation.
    Integer,
    /// Original object has a current native Double representation.
    Double,
    /// Original object is numeric, but its subtype is not independently known.
    Numeric,
    /// No current original cache class is proved.
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ConversionStage {
    ExpressionGetNumber,
    ScalarIntegerIndex,
}

/// Conditional current cache from an authenticated native operand conversion.
/// This is not an actual read, successful conversion, concrete value, effects
/// proof or canonical-string producer. Those obligations remain with ingress.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NativeOperandNumericCacheProduction {
    stage: ConversionStage,
    version: TclVersion,
    non_list_input: bool,
    category: TclType,
    normalises_string: bool,
}

impl NativeOperandNumericCacheProduction {
    /// Refine a reached expression conversion using an independently proved
    /// original cache class. The caller still proves successful integer-only
    /// numeric conversion, current object identity and closed conversion
    /// effects. A normal relational result alone can use string fallback.
    /// Existing Double caches stay Double even with integer-looking strings.
    /// Unknown and Dict inputs retain the conservative Numeric category;
    /// ordinary Dict comparison can succeed through string fallback without
    /// converting its original cache. Index recipes keep their separate stage.
    #[must_use]
    pub const fn with_original_cache_class(mut self, class: NativeNumericOperandClass) -> Self {
        if matches!(self.stage, ConversionStage::ExpressionGetNumber) {
            self.category = match class {
                NativeNumericOperandClass::String
                | NativeNumericOperandClass::List
                | NativeNumericOperandClass::ByteArray
                | NativeNumericOperandClass::Integer => TclType::Int,
                NativeNumericOperandClass::Double => TclType::Double,
                NativeNumericOperandClass::Dict
                | NativeNumericOperandClass::Numeric
                | NativeNumericOperandClass::Unknown => TclType::Numeric,
            };
        }
        self
    }

    /// Current numeric category; Int includes native C bignum representations.
    #[must_use]
    pub const fn category(self) -> TclType {
        self.category
    }

    /// Actual native engine release of this independently selected stage.
    #[must_use]
    pub const fn tcl_version(self) -> TclVersion {
        self.version
    }

    /// Whether caller must exclude a current grouped List index object.
    /// Its scalar-looking bytes do not prove that the original object, rather
    /// than a child extracted from that List, reached the native getter.
    #[must_use]
    pub const fn requires_non_list_input(self) -> bool {
        self.non_list_input
    }

    /// Initial lindex recipe requires an independently current integer cache.
    /// A cached Double with integer-looking bytes may fail scalar conversion
    /// and then convert a grouped child while retaining the parent List.
    #[must_use]
    pub const fn requires_original_integer_cache(self) -> bool {
        self.non_list_input
    }

    /// These getters retain the original spelling, including whitespace and
    /// radix prefixes. They do not license canonical-numeric byte disjointness.
    #[must_use]
    pub const fn normalises_original_string(self) -> bool {
        self.normalises_string
    }
}

impl PreparedExpressionWitness {
    /// Conditional original-operand cache for the prepared C numeric `<`
    /// branch. Caller separately proves this exact operand's successful read,
    /// closed integer-only contents, stock conversion effects and normal
    /// numeric branch. Normal comparison alone may instead use string fallback.
    /// C8.4 and Jim abstain: their independent preparation protocols cannot
    /// borrow later C `GetNumber` cache behavior or primitive Wide semantics.
    #[must_use]
    pub fn integer_relational_operand_conversion(
        &self,
        operand: &ExprNode,
    ) -> Option<NativeOperandNumericCacheProduction> {
        let NativeExprSyntax::Tcl(version) = self.context().native_syntax else {
            return None;
        };
        if version == TclVersion::V8_4 {
            return None;
        }
        let ExprNode::Binary {
            op: BinOp::Lt,
            left,
            right,
        } = self.tree()
        else {
            return None;
        };
        if !matches!(operand, ExprNode::Var { .. })
            || (operand != left.as_ref() && operand != right.as_ref())
        {
            return None;
        }
        Some(NativeOperandNumericCacheProduction {
            stage: ConversionStage::ExpressionGetNumber,
            version,
            non_list_input: false,
            category: TclType::Numeric,
            normalises_string: false,
        })
    }
}

impl InvocationFacts {
    /// Conditional integer-branch cache for a selected runtime scalar list
    /// index. Caller proves actual original-object mapping, successful getter
    /// continuation, integer-only contents and closed conversion effects.
    /// For lindex it also closes `requires_original_integer_cache` (and hence
    /// `requires_non_list_input`); a grouped index's
    /// child conversion cannot publish a cache on its original parent object.
    /// Inline literal indices, nested layouts and unknown selection abstain.
    #[must_use]
    pub fn integer_index_operand_conversion(
        &self,
        arguments: InvocationArguments<'_>,
        selection: NativeCompilationSelection,
        index: usize,
        original_shape: NativeCompilationWordShape,
    ) -> Option<NativeOperandNumericCacheProduction> {
        if self.successful_handler != Some(SuccessfulHandlerSpec::Leaf)
            || self.arity_accepts_frozen_arguments() != Some(true)
            || self.frozen_argument_count != arguments.exact_argv_len()
        {
            return None;
        }
        let version = arguments
            .dialect()?
            .native_scalar_getter_protocol()?
            .tcl_version()?;
        match selection {
            NativeCompilationSelection::Inline { operation, .. }
                if operation == self.operation
                    && original_shape == NativeCompilationWordShape::Substituted => {}
            NativeCompilationSelection::Generic
            | NativeCompilationSelection::NamedInvocation { .. }
                if !matches!(
                    original_shape,
                    NativeCompilationWordShape::Expanded | NativeCompilationWordShape::Opaque
                ) => {}
            _ => return None,
        }
        let count = arguments
            .exact_argv_len()?
            .checked_sub(self.argument_offset)?;
        let relative = index.checked_sub(self.argument_offset)?;
        let non_list_input = match self.operation {
            SemanticOperationId::Intrinsic(IntrinsicId::ListRange)
                if count == 3 && (1..=2).contains(&relative) =>
            {
                false
            }
            SemanticOperationId::Intrinsic(IntrinsicId::ListIndex)
                if count == 2 && relative == 1 =>
            {
                true
            }
            _ => return None,
        };
        Some(NativeOperandNumericCacheProduction {
            stage: ConversionStage::ScalarIntegerIndex,
            version,
            non_list_input,
            category: TclType::Int,
            normalises_string: false,
        })
    }
}

#[cfg(test)]
mod tests;
