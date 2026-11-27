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

//! Numeric objects produced by actual native expression execution.
//!
//! Source allocation families are not live runtime object tokens. They prove an
//! already numeric representation at a retained read, never unique aliasing.

use std::sync::Arc;

use tcl_registry::value_transfer::ExactValue;

use crate::command_binding::SourceExpressionPreparation;
use crate::tcl_expr_eval::TclValue;

/// Exact numeric contents, with bit-preserving double equality.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum SourceNativeNumber {
    /// Native wide integer.
    Integer(i64),
    /// Native arbitrary precision integer, in canonical decimal form.
    BigInteger(Arc<str>),
    /// Native double bits, including signed zero.
    Double(u64),
}

impl SourceNativeNumber {
    /// Preserve the selected native evaluator's result representation.
    #[must_use]
    pub fn from_value(value: &TclValue) -> Self {
        match value {
            TclValue::Int(value) => Self::Integer(*value),
            TclValue::Big(value) => Self::BigInteger(value.to_string().into()),
            TclValue::Float(value) => Self::Double(value.to_bits()),
        }
    }

    /// Numeric contents only; this does not establish an execution operand.
    #[must_use]
    pub fn value(&self) -> Option<TclValue> {
        Some(match self {
            Self::Integer(value) => TclValue::Int(*value),
            Self::BigInteger(value) => TclValue::Big(value.parse().ok()?),
            Self::Double(bits) => TclValue::Float(f64::from_bits(*bits)),
        })
    }
}

/// One source-produced numeric object family under an exact prepared tree.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SourceNativeNumericObject {
    /// Actual selected native numeric, result and representation protocols.
    pub dialect: tcl_native_model::InvocationDialect,
    /// Original native preparation and allocation instruction.
    pub preparation: Arc<SourceExpressionPreparation>,
    /// Actual native numeric result, independently from rendered bytes.
    pub number: SourceNativeNumber,
}

/// Closed integer contents accepted by the actual C bignum expression and
/// Increment protocols. This proves conversion/effects only, never a current
/// intrep, concrete number, unique object or frozen operand. Fixed-width C and
/// Jim require separate range/getter proofs and cannot construct this domain.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceIntegerConvertibleContents {
    dialect: tcl_native_model::InvocationDialect,
}

impl SourceIntegerConvertibleContents {
    fn for_dialect(dialect: tcl_native_model::InvocationDialect) -> Option<Self> {
        (dialect.family() == Some(tcl_dialect::model::Family::Tcl)
            && dialect.arithmetic() == Some(tcl_dialect::NativeArithmetic::TclBignum))
        .then_some(Self { dialect })
    }

    fn agrees_with(self, mut dialect: tcl_native_model::InvocationDialect) -> bool {
        dialect.lexer_grammar = self.dialect.lexer_grammar;
        dialect.word_values = self.dialect.word_values;
        dialect == self.dialect
    }
}

/// Actual numeric representation produced by a normal native store. Unknown
/// bytes cannot supply a number, native object identity or frozen operand.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceNativeNumericShape {
    dialect: tcl_native_model::InvocationDialect,
    production: SourceNumericShapeProduction,
    weakened_category: bool,
    integer_inputs: bool,
    integer_contents: Option<SourceIntegerConvertibleContents>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SourceNumericShapeProduction {
    Store(tcl_registry::native_result::NativeNumericStoreProduction),
    Result(tcl_registry::native_result::NativeNumericResultProduction),
    Expression(tcl_registry::runtime_expr_validation::NativeExpressionNumericResultProduction),
    Conversion(tcl_registry::native_numeric_conversion::NativeOperandNumericCacheProduction),
    Joined {
        category: tcl_registry::TclType,
        normalises_string: bool,
    },
}

impl SourceNativeNumericShape {
    pub(crate) const fn from_selected(
        dialect: tcl_native_model::InvocationDialect,
        production: tcl_registry::native_result::NativeNumericStoreProduction,
    ) -> Self {
        Self {
            dialect,
            production: SourceNumericShapeProduction::Store(production),
            weakened_category: false,
            integer_inputs: false,
            integer_contents: None,
        }
    }

    pub(crate) const fn from_result(
        dialect: tcl_native_model::InvocationDialect,
        production: tcl_registry::native_result::NativeNumericResultProduction,
    ) -> Self {
        Self {
            dialect,
            production: SourceNumericShapeProduction::Result(production),
            weakened_category: false,
            integer_inputs: false,
            integer_contents: None,
        }
    }
    pub(crate) const fn from_expression(
        dialect: tcl_native_model::InvocationDialect,
        production: tcl_registry::runtime_expr_validation::NativeExpressionNumericResultProduction,
    ) -> Self {
        Self {
            dialect,
            production: SourceNumericShapeProduction::Expression(production),
            weakened_category: false,
            integer_inputs: false,
            integer_contents: None,
        }
    }

    pub(crate) const fn from_conversion(
        dialect: tcl_native_model::InvocationDialect,
        production: tcl_registry::native_numeric_conversion::NativeOperandNumericCacheProduction,
    ) -> Self {
        Self {
            dialect,
            production: SourceNumericShapeProduction::Conversion(production),
            weakened_category: false,
            integer_inputs: false,
            integer_contents: None,
        }
    }

    pub(crate) fn with_integer_contents(
        mut self,
        proof: Option<SourceIntegerConvertibleContents>,
    ) -> Self {
        self.integer_contents = proof;
        self
    }

    pub(crate) fn with_integer_inputs(mut self) -> Self {
        self.integer_inputs = true;
        self
    }

    /// Unknown current contents may still be disjoint from a literal pool
    /// object when an actual result setter normalised their string bytes.
    /// This grants no exact bytes, unique identity or frozen operand.
    fn normalises_string(self) -> bool {
        match self.production {
            SourceNumericShapeProduction::Store(_) | SourceNumericShapeProduction::Result(_) => {
                true
            }
            SourceNumericShapeProduction::Expression(production) => {
                production.normalises_result_string()
            }
            SourceNumericShapeProduction::Conversion(production) => {
                production.normalises_original_string()
            }
            SourceNumericShapeProduction::Joined {
                normalises_string, ..
            } => normalises_string,
        }
    }

    pub(crate) fn string_bytes_may_equal(self, bytes: &[u8]) -> bool {
        use tcl_syntax::number::{Number, ParseFlags, parse_whole_with};
        let parsed = |numbers| {
            std::str::from_utf8(bytes)
                .ok()
                .and_then(|text| parse_whole_with(text, ParseFlags::for_syntax(numbers)))
        };
        // Integer contents, proved or held as an integer, are bytes that
        // parse as an integer.
        let integer_syntax = self
            .integer_contents_proof()
            .map(|proof| proof.dialect.numbers)
            .or_else(|| {
                (self.category() == tcl_registry::TclType::Int).then_some(self.dialect.numbers)
            });
        if let Some(numbers) = integer_syntax {
            return matches!(parsed(numbers), Some(Number::Int(_) | Number::Big { .. }));
        }
        if !self.normalises_string() {
            return true;
        }
        // A normalised numeric string is a whole number spelling, including
        // the platform's non-finite forms.
        tcl_syntax::number::canonical_numeric_bytes_may_equal(bytes)
            && (parsed(self.dialect.numbers).is_some()
                || std::str::from_utf8(bytes).is_ok_and(|text| {
                    matches!(
                        text.to_ascii_lowercase().as_str(),
                        "nan"
                            | "-nan"
                            | "+nan"
                            | "inf"
                            | "-inf"
                            | "+inf"
                            | "infinity"
                            | "-infinity"
                    )
                }))
    }

    fn joined(self, other: Self) -> Option<Self> {
        (self.dialect == other.dialect).then(|| Self {
            dialect: self.dialect,
            production: SourceNumericShapeProduction::Joined {
                category: if self.category() == other.category() {
                    self.category()
                } else {
                    tcl_registry::TclType::Numeric
                },
                normalises_string: self.normalises_string() && other.normalises_string(),
            },
            weakened_category: false,
            integer_inputs: false,
            integer_contents: self
                .integer_contents_proof()
                .filter(|proof| Some(*proof) == other.integer_contents_proof()),
        })
    }

    fn integer_contents_proof(self) -> Option<SourceIntegerConvertibleContents> {
        self.integer_contents.or_else(|| {
            (self.category() == tcl_registry::TclType::Int)
                .then(|| SourceIntegerConvertibleContents::for_dialect(self.dialect))
                .flatten()
        })
    }

    fn category(self) -> tcl_registry::TclType {
        use tcl_registry::{TclType, native_result::NativeNumericResultProduction};
        if self.weakened_category {
            return TclType::Numeric;
        }
        if self.integer_inputs {
            return TclType::Int;
        }
        match self.production {
            SourceNumericShapeProduction::Store(_)
            | SourceNumericShapeProduction::Result(NativeNumericResultProduction::Integer {
                ..
            }) => TclType::Int,
            SourceNumericShapeProduction::Result(NativeNumericResultProduction::Double) => {
                TclType::Double
            }
            SourceNumericShapeProduction::Expression(production) => production.result_type(),
            SourceNumericShapeProduction::Conversion(production) => production.category(),
            SourceNumericShapeProduction::Joined { category, .. } => category,
        }
    }
}

/// Frozen object representation valid until the next possible shared coercion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct FrozenSourceNumeric {
    pub object: Arc<SourceNativeNumericObject>,
    pub epoch: u64,
}

/// A closed set of ordinary String/List/Dict representations for conversion
/// advice. A stock string converts through the ordinary parser without object
/// callbacks. It cannot prove one current representation or erase any operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ClosedContainerRepresentations {
    string: bool,
    list: bool,
    dictionary: bool,
}

impl ClosedContainerRepresentations {
    pub(crate) fn of(representation: tcl_syntax::value::ValueRepresentation) -> Option<Self> {
        use tcl_syntax::value::ValueRepresentation;
        let only = |string, list, dictionary| {
            Some(Self {
                string,
                list,
                dictionary,
            })
        };
        match representation {
            ValueRepresentation::String => only(true, false, false),
            ValueRepresentation::List => only(false, true, false),
            ValueRepresentation::Dict => only(false, false, true),
            ValueRepresentation::Unknown => None,
        }
    }

    pub(crate) const fn joined(self, other: Self) -> Self {
        Self {
            string: self.string || other.string,
            list: self.list || other.list,
            dictionary: self.dictionary || other.dictionary,
        }
    }

    pub(crate) fn stored(self) -> StoredNativeRepresentation {
        use tcl_syntax::value::ValueRepresentation;
        match (self.string, self.list, self.dictionary) {
            (true, false, false) => StoredNativeRepresentation::Known(ValueRepresentation::String),
            (false, true, false) => StoredNativeRepresentation::Known(ValueRepresentation::List),
            (false, false, true) => StoredNativeRepresentation::Known(ValueRepresentation::Dict),
            _ => StoredNativeRepresentation::ContainerAlternatives(self),
        }
    }

    /// Whether a represented normal path may have this ordinary container type.
    #[must_use]
    pub const fn contains(self, representation: tcl_syntax::value::ValueRepresentation) -> bool {
        use tcl_syntax::value::ValueRepresentation;
        match representation {
            ValueRepresentation::String => self.string,
            ValueRepresentation::List => self.list,
            ValueRepresentation::Dict => self.dictionary,
            ValueRepresentation::Unknown => false,
        }
    }
}

/// Existing per-cell representation owner, including source numeric provenance.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum StoredNativeRepresentation {
    /// Original stock literal object class; native intrep is independently tracked.
    StockLiteralObject(crate::command_binding::SourceStockLiteralObject),
    /// Original Normal producer closes `StringAccess` effects only.
    NativeStringAccess(Arc<crate::command_binding::SourceNativeStringAccess>),
    /// Representation proved independently by a selected handler.
    Known(tcl_syntax::value::ValueRepresentation),
    /// Closed possible ordinary container types; no single intrep is proved.
    ContainerAlternatives(ClosedContainerRepresentations),
    /// Actual expression-produced numeric object, before any shared coercion.
    Numeric(Arc<SourceNativeNumericObject>),
    /// Current numeric intrep only, independently of its unknown contents.
    NumericShape(SourceNativeNumericShape),
    /// Closed integer contents/conversion alternatives; strict intrep is unknown.
    IntegerConvertibleContents(SourceIntegerConvertibleContents),
    /// Current audited object-method provider; no ordinary representation or bytes.
    ReadOnlyListMethodProvider(tcl_registry::native_result::NativeListMethodProvider),
}

impl From<tcl_syntax::value::ValueRepresentation> for StoredNativeRepresentation {
    fn from(value: tcl_syntax::value::ValueRepresentation) -> Self {
        Self::Known(value)
    }
}

impl StoredNativeRepresentation {
    pub(crate) fn numeric_shape(&self) -> Option<SourceNativeNumericShape> {
        match self {
            Self::NumericShape(shape) => Some(*shape),
            Self::Numeric(object) => Some(SourceNativeNumericShape {
                dialect: object.dialect,
                production: SourceNumericShapeProduction::Joined {
                    category: match &object.number {
                        SourceNativeNumber::Integer(_) | SourceNativeNumber::BigInteger(_) => {
                            tcl_registry::TclType::Int
                        }
                        SourceNativeNumber::Double(_) => tcl_registry::TclType::Double,
                    },
                    normalises_string: false,
                },
                weakened_category: false,
                integer_inputs: false,
                integer_contents: None,
            }),
            _ => None,
        }
    }

    pub(crate) fn joined_current_numeric(&self, other: &Self) -> Option<Self> {
        Some(Self::NumericShape(
            self.numeric_shape()?.joined(other.numeric_shape()?)?,
        ))
    }

    /// Existing nonnumeric compatibility projection; numeric proof is separate.
    #[must_use]
    pub const fn representation(&self) -> tcl_syntax::value::ValueRepresentation {
        match self {
            Self::StockLiteralObject(object) => object.representation(),
            Self::Known(value) => *value,
            Self::NativeStringAccess(_)
            | Self::Numeric(_)
            | Self::NumericShape(_)
            | Self::IntegerConvertibleContents(_)
            | Self::ReadOnlyListMethodProvider(_)
            | Self::ContainerAlternatives(_) => tcl_syntax::value::ValueRepresentation::Unknown,
        }
    }

    pub(crate) fn container_alternatives(&self) -> Option<ClosedContainerRepresentations> {
        match self {
            Self::StockLiteralObject(object) => object.container_alternatives(),
            Self::Known(value) => ClosedContainerRepresentations::of(*value),
            Self::ContainerAlternatives(values) => Some(*values),
            Self::NativeStringAccess(_)
            | Self::Numeric(_)
            | Self::NumericShape(_)
            | Self::IntegerConvertibleContents(_)
            | Self::ReadOnlyListMethodProvider(_) => None,
        }
    }

    pub(crate) fn integer_contents_in(
        &self,
        bytes: Option<&str>,
        dialect: Option<tcl_native_model::InvocationDialect>,
    ) -> Option<SourceIntegerConvertibleContents> {
        let dialect = dialect?;
        let proof = SourceIntegerConvertibleContents::for_dialect(dialect)?;
        match self {
            Self::IntegerConvertibleContents(old) => old.agrees_with(dialect).then_some(*old),
            // A source literal's numeric reading is a function of its string
            // on every release measured, whatever read the shared object
            // first: tclsh 8.4.20 to 9.1.0 print 2 for `set a 1; format %f $a;
            // expr {$a + 1}` and after `format %e`, `lsort -real`,
            // `binary format d`, `expr {double($a)}`, `string is double` and
            // `incr a 0` (`expr_operand_literal_reading` in the optimiser
            // tests holds the transcript).
            Self::StockLiteralObject(object) => (object.accepts_integer_contents(bytes?, dialect)
                || matches!(
                    tcl_syntax::number::parse_whole_with(
                        bytes?,
                        tcl_syntax::number::ParseFlags::for_syntax(dialect.numbers)
                    ),
                    Some(
                        tcl_syntax::number::Number::Int(_) | tcl_syntax::number::Number::Big { .. }
                    )
                ))
            .then_some(proof),
            Self::NumericShape(shape) => shape
                .integer_contents
                .filter(|retained| retained.agrees_with(dialect))
                .or_else(|| {
                    (shape.category() == tcl_registry::TclType::Int
                        && proof.agrees_with(shape.dialect))
                    .then_some(proof)
                }),
            Self::Numeric(number) => (matches!(
                number.number,
                SourceNativeNumber::Integer(_) | SourceNativeNumber::BigInteger(_)
            ) && proof.agrees_with(number.dialect))
            .then_some(proof),
            _ => None,
        }
    }

    pub(crate) fn joined(&self, other: &Self) -> Option<Self> {
        use tcl_registry::native_result::NativeListMethodProvider::EmptyListRoot;

        if let (Self::StockLiteralObject(left), Self::StockLiteralObject(right)) = (self, other) {
            return Some(Self::StockLiteralObject(left.joined(*right)));
        }
        if self == other {
            return Some(self.clone());
        }
        if matches!(
            (self, other),
            (
                Self::ReadOnlyListMethodProvider(EmptyListRoot),
                Self::Known(tcl_syntax::value::ValueRepresentation::List)
            ) | (
                Self::Known(tcl_syntax::value::ValueRepresentation::List),
                Self::ReadOnlyListMethodProvider(EmptyListRoot)
            )
        ) {
            // Intersect root effects only. The joined object need not be empty,
            // have a List cache, or close member StringAccess callbacks.
            return Some(Self::ReadOnlyListMethodProvider(EmptyListRoot));
        }
        if let Some(joined) = self.joined_current_numeric(other) {
            return Some(joined);
        }
        Some(Self::ContainerAlternatives(
            self.container_alternatives()?
                .joined(other.container_alternatives()?),
        ))
    }
}

/// Numeric expression topology without callbacks, string conversion or list
/// interpretation. Literal syntax may be analysed mathematically, but requires
/// a separate pool/conversion proof before it counts as already numeric.
/// Variables require independently retained numeric objects.
pub(crate) fn numeric_tree(
    tree: &crate::expr_ast::ExprNode,
    literals_are_numeric: bool,
    mut variable: impl FnMut(&crate::expr_ast::ExprNode) -> bool,
) -> bool {
    use crate::expr_ast::ExprNode;
    let mut pending = vec![tree];
    while let Some(node) = pending.pop() {
        match node {
            ExprNode::Literal { .. } if literals_are_numeric => {}
            ExprNode::Var { .. } if variable(node) => {}
            ExprNode::Unary { operand, .. } => pending.push(operand),
            ExprNode::Binary { left, right, op } => {
                use tcl_syntax::expr::BinOp;
                if !matches!(
                    op,
                    BinOp::Add
                        | BinOp::Sub
                        | BinOp::Mul
                        | BinOp::Div
                        | BinOp::Mod
                        | BinOp::Pow
                        | BinOp::LShift
                        | BinOp::RShift
                        | BinOp::BitAnd
                        | BinOp::BitOr
                        | BinOp::BitXor
                        | BinOp::And
                        | BinOp::Or
                        | BinOp::Eq
                        | BinOp::Ne
                        | BinOp::Lt
                        | BinOp::Le
                        | BinOp::Gt
                        | BinOp::Ge
                ) {
                    return false;
                }
                pending.extend([left.as_ref(), right.as_ref()]);
            }
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                pending.extend([
                    condition.as_ref(),
                    true_branch.as_ref(),
                    false_branch.as_ref(),
                ]);
            }
            _ => return false,
        }
    }
    true
}

impl crate::var_resolve::ResolveContext {
    /// Object hooks of already selected contents, after a captured native read
    /// has completed. The lease owner checks liveness/generation; this helper
    /// deliberately does not request a second read or rerun its observers.
    pub(crate) fn selected_contents_numeric_hooks_closed(
        &self,
        place: &tcl_names::place::Place,
    ) -> bool {
        if self.binding_identity != crate::var_resolve::BindingIdentity::Bound {
            return false;
        }
        let Some(key) = crate::var_resolve::canonical_binding_value_key(place) else {
            return false;
        };
        match self.value_representations.get(&key) {
            Some(
                StoredNativeRepresentation::StockLiteralObject(_)
                | StoredNativeRepresentation::Known(tcl_syntax::value::ValueRepresentation::String),
            ) => true,
            Some(StoredNativeRepresentation::IntegerConvertibleContents(proof)) => self
                .invocation_dialect
                .is_some_and(|dialect| proof.agrees_with(dialect)),
            Some(value) => value.numeric_shape().is_some_and(|shape| {
                self.invocation_dialect.is_some_and(|mut actual| {
                    actual.lexer_grammar = shape.dialect.lexer_grammar;
                    actual.word_values = shape.dialect.word_values;
                    actual == shape.dialect
                })
            }),
            None => false,
        }
    }

    pub(crate) fn contents_stock_literal_object_at(
        &self,
        place: &tcl_names::place::Place,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<crate::command_binding::SourceStockLiteralObject> {
        self.read_produces_value(place, registry).then_some(())?;
        let key = crate::var_resolve::canonical_binding_value_key(place)?;
        match self.value_representations.get(&key)? {
            StoredNativeRepresentation::StockLiteralObject(object) => Some(*object),
            _ => None,
        }
    }
    /// An actual successful live read whose every contents alternative closes
    /// both C-bignum expression-number and Increment-integer conversion. This
    /// purpose query does not establish a numeric intrep or concrete value.
    pub(crate) fn contents_integer_increment_conversion_at(
        &self,
        place: &tcl_names::place::Place,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<SourceIntegerConvertibleContents> {
        // The sealed closed domain records all producer alternatives. Their
        // exact write origin may differ at a join; no value/operand is exposed.
        if self.binding_identity != crate::var_resolve::BindingIdentity::Bound
            || !self.read_produces_value(place, registry)
        {
            return None;
        }
        let key = crate::var_resolve::canonical_binding_value_key(place)?;
        self.value_representations.get(&key)?.integer_contents_in(
            self.constant_values.get(&key).map(String::as_str),
            self.invocation_dialect,
        )
    }

    /// Closed integer spelling for the numeric branch of a reached C number
    /// getter. A pre-existing Double cache may be retained, so this supplies
    /// neither Integer cache subtype nor Increment acceptance or a value.
    pub(crate) fn contents_integer_number_branch_at(
        &self,
        place: &tcl_names::place::Place,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        if self.binding_identity != crate::var_resolve::BindingIdentity::Bound
            || !self.read_produces_value(place, registry)
        {
            return false;
        }
        if self.contents_native_numeric_category_at(place, registry)
            == Some(tcl_registry::TclType::Int)
            || self
                .contents_integer_increment_conversion_at(place, registry)
                .is_some()
        {
            return true;
        }
        let Some(key) = crate::var_resolve::canonical_binding_value_key(place) else {
            return false;
        };
        let Some(dialect) = self.invocation_dialect else {
            return false;
        };
        matches!(
            self.value_representations.get(&key),
            Some(StoredNativeRepresentation::StockLiteralObject(_))
        ) && self.constant_values.get(&key).is_some_and(|bytes| {
            matches!(
                tcl_syntax::number::parse_whole_with(
                    bytes,
                    tcl_syntax::number::ParseFlags::for_syntax(dialect.numbers)
                ),
                Some(tcl_syntax::number::Number::Int(_) | tcl_syntax::number::Number::Big { .. })
            )
        })
    }

    /// The current original scalar can expose its string without a custom
    /// updater/free callback. Closed integer contents alternatives may prove
    /// this effect obligation while their current cache category is unknown.
    /// This licenses no getter acceptance, value, cache mutation or operand.
    #[must_use]
    pub fn contents_native_string_access_closed_at(
        &self,
        place: &tcl_names::place::Place,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        if self.binding_identity != crate::var_resolve::BindingIdentity::Bound {
            return false;
        }
        self.contents_native_string_access_receipt_at(place, registry)
            .is_some()
            || self.contents_already_native_numeric_at(place, registry)
            || self
                .contents_stock_literal_object_at(place, registry)
                .is_some()
            || self
                .contents_integer_increment_conversion_at(place, registry)
                .is_some()
    }

    pub(crate) fn contents_native_string_result_input_closed_at(
        &self,
        place: &tcl_names::place::Place,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        self.contents_native_string_access_receipt_at(place, registry)
            .is_some()
            || self
                .contents_stock_literal_object_at(place, registry)
                .is_some()
            || self.contents_already_native_numeric_at(place, registry)
    }

    pub(crate) fn contents_native_string_access_receipt_at(
        &self,
        place: &tcl_names::place::Place,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<&Arc<crate::command_binding::SourceNativeStringAccess>> {
        if self.binding_identity != crate::var_resolve::BindingIdentity::Bound
            || !self.read_produces_value(place, registry)
        {
            return None;
        }
        let key = crate::var_resolve::canonical_binding_value_key(place)?;
        match self.value_representations.get(&key)? {
            StoredNativeRepresentation::NativeStringAccess(receipt) if receipt.is_current() => {
                Some(receipt)
            }
            _ => None,
        }
    }

    /// Current numeric subtype from an actual normal native producer or exact
    /// numeric object receipt. It supplies neither contents nor an operand.
    #[must_use]
    pub fn contents_native_numeric_category_at(
        &self,
        place: &tcl_names::place::Place,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<tcl_registry::TclType> {
        self.contents_already_native_numeric_at(place, registry)
            .then_some(())?;
        let key = crate::var_resolve::canonical_binding_value_key(place)?;
        match self.value_representations.get(&key)? {
            StoredNativeRepresentation::NumericShape(shape) => Some(shape.category()),
            StoredNativeRepresentation::Numeric(numeric) => Some(match &numeric.number {
                SourceNativeNumber::Integer(_) | SourceNativeNumber::BigInteger(_) => {
                    tcl_registry::TclType::Int
                }
                SourceNativeNumber::Double(_) => tcl_registry::TclType::Double,
            }),
            _ => None,
        }
    }

    pub(crate) fn invalidate_math_numeric_operand_representations(
        &mut self,
        policy: tcl_registry::mathfunc::NativeMathNumericOperandPolicy,
    ) {
        self.invalidate_numeric_conversion_representations(None);
        if policy == tcl_registry::mathfunc::NativeMathNumericOperandPolicy::WeakenToNumeric {
            for value in self.value_representations.values_mut() {
                if let StoredNativeRepresentation::NumericShape(shape) = value
                    && shape.category() == tcl_registry::TclType::Int
                {
                    shape.weakened_category = true;
                }
            }
        }
    }

    pub(crate) fn contents_native_numeric_shape_at(
        &self,
        place: &tcl_names::place::Place,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<SourceNativeNumericShape> {
        self.contents_already_native_numeric_at(place, registry)
            .then_some(())?;
        let key = crate::var_resolve::canonical_binding_value_key(place)?;
        match self.value_representations.get(&key)? {
            StoredNativeRepresentation::NumericShape(shape) => Some(*shape),
            _ => None,
        }
    }

    /// Already numeric at this actual read, without supplying a numeric value
    /// or an object receipt. This can exclude simultaneous ordinary List/Dict
    /// identity from a coercion footprint even when store bytes are unknown.
    #[must_use]
    pub fn contents_already_native_numeric_at(
        &self,
        place: &tcl_names::place::Place,
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        if self.binding_identity != crate::var_resolve::BindingIdentity::Bound
            || !self.read_produces_value(place, registry)
        {
            return false;
        }
        let Some(key) = crate::var_resolve::canonical_binding_value_key(place) else {
            return false;
        };
        let dialect = match self.value_representations.get(&key) {
            Some(StoredNativeRepresentation::Numeric(object)) => {
                if !matches!(
                    self.read_contents_origin(place, registry),
                    crate::var_resolve::ContentsOrigin::WrittenAt(_)
                        | crate::var_resolve::ContentsOrigin::Incoming
                ) {
                    return false;
                }
                object.dialect
            }
            Some(StoredNativeRepresentation::NumericShape(shape)) => shape.dialect,
            _ => return false,
        };
        let Some(mut actual) = self.invocation_dialect else {
            return false;
        };
        actual.lexer_grammar = dialect.lexer_grammar;
        actual.word_values = dialect.word_values;
        actual == dialect
    }

    /// Actual numeric-only operand conversion preserves an already numeric
    /// shape. It still retires overlapping concrete receipts and advances the
    /// shared epoch; list, index, callback and unknown coercions cannot use it.
    pub(crate) fn invalidate_numeric_conversion_representations(
        &mut self,
        values: Option<&[String]>,
    ) {
        let shapes = self
            .value_representations
            .iter()
            .filter_map(|(key, value)| match value {
                StoredNativeRepresentation::NumericShape(_)
                | StoredNativeRepresentation::IntegerConvertibleContents(_) => {
                    Some((key.clone(), value.clone()))
                }
                StoredNativeRepresentation::StockLiteralObject(object) => Some((
                    key.clone(),
                    StoredNativeRepresentation::StockLiteralObject(
                        object.converted(tcl_syntax::value::ValueRepresentation::Unknown),
                    ),
                )),
                _ => None,
            })
            .collect::<Vec<_>>();
        self.invalidate_representations_for_values(values);
        for (key, shape) in shapes {
            self.value_representations.entry(key).or_insert(shape);
        }
    }

    /// A selected numeric index can change its own intrep, but cannot be an
    /// object currently represented as an ordinary container. All numeric
    /// receipts are retired because other cells may share that index object.
    pub(crate) fn invalidate_numeric_index_representations(
        &mut self,
        indices: &[tcl_names::place::Place],
        registry: &tcl_registry::CommandRegistry,
    ) -> bool {
        if indices.is_empty() {
            return true;
        }
        if !indices
            .iter()
            .all(|index| self.contents_already_native_numeric_at(index, registry))
        {
            // Converting an index replaces its intrep, never its bytes: a
            // stock literal it may share stays a stock-class object.
            self.coerce_representations_for_values(None);
            return false;
        }
        // Converting the index never changes bytes, so a stock literal stays a
        // stock-class object whatever intrep the conversion leaves it.
        let ordinary = self
            .value_representations
            .iter()
            .filter_map(|(key, value)| match value {
                StoredNativeRepresentation::StockLiteralObject(object) => Some((
                    key.clone(),
                    StoredNativeRepresentation::StockLiteralObject(
                        object.converted(tcl_syntax::value::ValueRepresentation::Unknown),
                    ),
                )),
                _ => value
                    .container_alternatives()
                    .is_some()
                    .then(|| (key.clone(), value.clone())),
            })
            .collect::<Vec<_>>();
        self.invalidate_shared_representations();
        self.value_representations.extend(ordinary);
        true
    }

    /// Copy an ordinary Value formal's actual frozen object receipt. Logical
    /// parameter contents or default/rest/reference binding cannot mint it.
    pub(crate) fn retain_incoming_native_numeric(
        &mut self,
        slot: &str,
        receipt: &FrozenSourceNumeric,
        registry: &tcl_registry::CommandRegistry,
    ) {
        let place = crate::var_resolve::resolve_literal_access(
            slot,
            self,
            false,
            registry,
            tcl_registry::TraceOperation::Write,
        );
        let Some(cell) = &place.cell else {
            return;
        };
        if self.representation_epoch != Some(receipt.epoch)
            || place.observed
            || place.dynamic
            || place.kind != tcl_names::place::PlaceKind::Scalar
            || place.index.is_some()
            || !matches!(&cell.owner, tcl_names::place::CellOwner::Activation(owner) if self.activation.as_ref() == Some(owner))
            || self.contents_presence(&place) != crate::var_resolve::ContentsPresence::Defined
            || self.contents_origin(&place) != crate::var_resolve::ContentsOrigin::Incoming
        {
            return;
        }
        if let Some(key) = crate::var_resolve::canonical_binding_value_key(&place) {
            self.value_representations.insert(
                key,
                StoredNativeRepresentation::Numeric(Arc::clone(&receipt.object)),
            );
        }
    }

    /// Already numeric contents of this exact live cell, before read observers.
    #[must_use]
    pub fn contents_native_numeric_at(
        &self,
        place: &tcl_names::place::Place,
        registry: &tcl_registry::CommandRegistry,
    ) -> Option<Arc<SourceNativeNumericObject>> {
        use crate::var_resolve::{BindingIdentity, ContentsOrigin, ContentsPresence};
        let cell = place.cell.as_ref()?;
        if self.binding_identity != BindingIdentity::Bound
            || place.observed
            || place.dynamic
            || place.kind != tcl_names::place::PlaceKind::Scalar
            || place.index.is_some()
            || self.dynamic_bindings
            || self.dynamic_traces
            || self.contents_presence(place) != ContentsPresence::Defined
            || self.store_would_error(place)
            || cell.generation == tcl_names::place::CellGeneration::Unknown
            || cell.generation
                != self
                    .generations
                    .get(&crate::var_resolve::cell_key(place))
                    .copied()
                    .unwrap_or_default()
            || !matches!(
                self.read_contents_origin(place, registry),
                ContentsOrigin::WrittenAt(_) | ContentsOrigin::Incoming
            )
        {
            return None;
        }
        let key = crate::var_resolve::canonical_binding_value_key(place)?;
        let StoredNativeRepresentation::Numeric(object) = self.value_representations.get(&key)?
        else {
            return None;
        };
        let mut actual = self.invocation_dialect?;
        actual.lexer_grammar = object.dialect.lexer_grammar;
        actual.word_values = object.dialect.word_values;
        (actual == object.dialect).then(|| Arc::clone(object))
    }
}

/// Closed original read of an already native numeric source-produced object.
/// Mathematical contents never construct this proof; every physical context
/// must retain the same producer and native representation without observers.
pub(crate) fn source_read_operand(
    access: &crate::command_binding::SourceVariableAccess,
    registry: &tcl_registry::CommandRegistry,
) -> Option<crate::tcl_expr_eval::RetainedNativeOperandProof> {
    use crate::command_binding::SourceVariableReadResidual;
    if access.context_residual() != SourceVariableReadResidual::Closed {
        return None;
    }
    let mut producer = None;
    let mut cells = Vec::new();
    for context in access.context_alternatives() {
        let place = access.place_in_context(context, registry);
        let object = context.contents_native_numeric_at(&place, registry)?;
        if producer
            .as_ref()
            .is_some_and(|previous| previous != &object)
        {
            return None;
        }
        producer = Some(object);
        let cell = place.cell?;
        if !cells.contains(&cell) {
            cells.push(cell);
        }
    }
    let producer = producer?;
    Some(crate::tcl_expr_eval::RetainedNativeOperandProof {
        dialect: producer.dialect,
        value: producer.number.value()?,
        existing_string: None,
        identity: crate::tcl_expr_eval::NativeOperandObjectIdentity::Source {
            producer,
            read: access.source.clone(),
            cells,
        },
    })
}

/// Exact source read whose independently closed integer contents are accepted
/// by the selected number conversion. It does not describe a current numeric
/// representation or a unique native object.
#[derive(Debug, Clone)]
pub(crate) struct SourceIntegerContentsRead {
    dialect: tcl_native_model::InvocationDialect,
    recipe: tcl_registry::native_numeric_conversion::NativeStockIntegerContentsProtocol,
    contents: ExactValue<String>,
    number: TclValue,
    cells: Vec<tcl_names::place::CellIdentity>,
}

impl SourceIntegerContentsRead {
    pub(crate) fn contents(&self) -> &str {
        &self.contents.bytes
    }
    pub(crate) fn number(&self) -> &TclValue {
        &self.number
    }
    pub(crate) fn agrees_with(&self, other: &Self) -> bool {
        self.dialect == other.dialect
            && self.recipe == other.recipe
            && self.contents == other.contents
            && self.cells == other.cells
    }
    pub(crate) fn accepts_number(
        &self,
        dialect: Option<tcl_native_model::InvocationDialect>,
        contents: &str,
    ) -> bool {
        dialect == Some(self.dialect)
            && contents == self.contents.bytes
            && tcl_registry::native_numeric_conversion::NativeStockIntegerContentsProtocol::select(
                self.dialect,
            ) == Some(self.recipe)
    }
}

pub(crate) type SourceIntegerContentsReads =
    std::collections::HashMap<String, SourceIntegerContentsRead>;

/// Value and conversion are independent premises. Every original live read
/// must supply the same closed integer contents and physical alternatives.
/// A semantic integer, current Numeric shape or unknown incoming value cannot
/// construct this receipt by itself.
pub(crate) fn source_read_integer_contents(
    access: &crate::command_binding::SourceVariableAccess,
    registry: &tcl_registry::CommandRegistry,
) -> Option<SourceIntegerContentsRead> {
    use crate::command_binding::SourceVariableReadResidual;
    if access.context_residual() != SourceVariableReadResidual::Closed
        || access.context_alternatives().is_empty()
    {
        return None;
    }
    let mut selected = None;
    let mut cells = Vec::new();
    for context in access.context_alternatives() {
        let place = access.place_in_context(context, registry);
        let proof = context.contents_integer_increment_conversion_at(&place, registry)?;
        let dialect = context.invocation_dialect?;
        if !proof.agrees_with(dialect) {
            return None;
        }
        let recipe =
            tcl_registry::native_numeric_conversion::NativeStockIntegerContentsProtocol::select(
                dialect,
            )?;
        let contents = context.literal_contents_at(&place, registry)?;
        if !recipe.accepts_integer_contents(contents) {
            return None;
        }
        if selected
            .as_ref()
            .is_some_and(|(old_dialect, old_recipe, old_contents)| {
                *old_dialect != dialect || *old_recipe != recipe || old_contents != contents
            })
        {
            return None;
        }
        selected = Some((dialect, recipe, contents.to_owned()));
        let cell = place.cell?;
        if !cells.contains(&cell) {
            cells.push(cell);
        }
    }
    let (dialect, recipe, contents) = selected?;
    let number = match tcl_syntax::number::parse_whole_with(
        &contents,
        tcl_syntax::number::ParseFlags::for_syntax(dialect.numbers),
    )? {
        tcl_syntax::number::Number::Int(value) => TclValue::Int(value),
        tcl_syntax::number::Number::Big {
            negative,
            radix,
            digits,
        } => {
            let mut value = num_bigint::BigInt::parse_bytes(digits.as_bytes(), radix as u32)?;
            if negative {
                value = -value;
            }
            TclValue::from_big(value)
        }
        _ => return None,
    };
    Some(SourceIntegerContentsRead {
        dialect,
        recipe,
        contents: ExactValue::utf8(contents),
        number,
        cells,
    })
}

/// Scoped numeric environment and representation receipts for an unchanged
/// callback-free expression. Repeated spellings must retain the same producer
/// and every physical alternative, never merely the same mathematical value.
pub(crate) fn expression_operands(
    tree: &crate::expr_ast::ExprNode,
    read: impl FnMut(
        &crate::expr_ast::ExprNode,
    ) -> Option<crate::tcl_expr_eval::RetainedNativeOperandProof>,
) -> Option<(
    crate::tcl_expr_eval::Env,
    crate::tcl_expr_eval::NativeOperandProofs,
)> {
    expression_operands_with_calls(tree, read, |_| false)
}

/// Collect the same original operand proofs while independently validating
/// every function through its retained reached-call owner. This projects no
/// implementation or completion authority from a function spelling.
pub(crate) fn expression_operands_with_calls(
    tree: &crate::expr_ast::ExprNode,
    read: impl FnMut(
        &crate::expr_ast::ExprNode,
    ) -> Option<crate::tcl_expr_eval::RetainedNativeOperandProof>,
    call: impl FnMut(&crate::expr_ast::ExprNode) -> bool,
) -> Option<(
    crate::tcl_expr_eval::Env,
    crate::tcl_expr_eval::NativeOperandProofs,
)> {
    expression_operands_with_integer_contents(tree, read, |_| None, call)
        .map(|(environment, operands, _)| (environment, operands))
}

/// Read already-numeric objects and accepted integer conversions through
/// independent source receipts. The latter never enter the numeric-object map.
pub(crate) fn expression_operands_with_integer_contents(
    tree: &crate::expr_ast::ExprNode,
    mut read: impl FnMut(
        &crate::expr_ast::ExprNode,
    ) -> Option<crate::tcl_expr_eval::RetainedNativeOperandProof>,
    mut integer_read: impl FnMut(&crate::expr_ast::ExprNode) -> Option<SourceIntegerContentsRead>,
    mut call: impl FnMut(&crate::expr_ast::ExprNode) -> bool,
) -> Option<(
    crate::tcl_expr_eval::Env,
    crate::tcl_expr_eval::NativeOperandProofs,
    SourceIntegerContentsReads,
)> {
    use crate::tcl_expr_eval::{Env, EnvValue, NativeOperandObjectIdentity, NativeOperandProofs};
    let mut environment = Env::new();
    let mut operands = NativeOperandProofs::new();
    let mut integer_contents = SourceIntegerContentsReads::new();
    let closed = retained_operand_tree_with_calls(
        tree,
        |node| {
            let crate::expr_ast::ExprNode::Var { text, .. } = node else {
                return false;
            };
            if let Some(proof) = read(node) {
                let Ok(place) = crate::native_lowering::cells::variable_reference_place(
                    text,
                    tcl_lexer::LexerConfig::from_grammar(proof.dialect.lexer_grammar),
                ) else {
                    return false;
                };
                let name = place.spelling();
                if integer_contents.contains_key(&name) {
                    return false;
                }
                if let Some(previous) = operands.get(&name) {
                    let (
                        NativeOperandObjectIdentity::Source {
                            producer: a,
                            cells: ac,
                            ..
                        },
                        NativeOperandObjectIdentity::Source {
                            producer: b,
                            cells: bc,
                            ..
                        },
                    ) = (&previous.identity, &proof.identity)
                    else {
                        return false;
                    };
                    if a != b || ac != bc {
                        return false;
                    }
                }
                let value = match &proof.value {
                    number @ (TclValue::Int(_) | TclValue::Float(_)) => {
                        EnvValue::Number(number.clone())
                    }
                    TclValue::Big(value) => EnvValue::text(value.to_string()),
                };
                environment.insert(name.clone(), value);
                operands.insert(name, proof);
            } else {
                let Some(proof) = integer_read(node) else {
                    return false;
                };
                let Ok(place) = crate::native_lowering::cells::variable_reference_place(
                    text,
                    tcl_lexer::LexerConfig::from_grammar(proof.dialect.lexer_grammar),
                ) else {
                    return false;
                };
                let name = place.spelling();
                if operands.contains_key(&name)
                    || integer_contents
                        .get(&name)
                        .is_some_and(|old| !proof.agrees_with(old))
                {
                    return false;
                }
                environment.insert(name.clone(), EnvValue::Str(proof.contents.clone()));
                integer_contents.insert(name, proof);
            }
            true
        },
        &mut call,
    );
    closed.then_some((environment, operands, integer_contents))
}

/// Operand evidence is independent of the result's numeric topology. A fixed
/// comparison string needs no variable read; a substituting string cannot
/// borrow this environment without its own ordered read evidence.
#[cfg(test)]
fn retained_operand_tree(
    tree: &crate::expr_ast::ExprNode,
    read: impl FnMut(&crate::expr_ast::ExprNode) -> bool,
) -> bool {
    retained_operand_tree_with_calls(tree, read, |_| false)
}

fn retained_operand_tree_with_calls(
    tree: &crate::expr_ast::ExprNode,
    mut read: impl FnMut(&crate::expr_ast::ExprNode) -> bool,
    mut call: impl FnMut(&crate::expr_ast::ExprNode) -> bool,
) -> bool {
    use crate::expr_ast::ExprNode;
    let mut pending = vec![tree];
    while let Some(node) = pending.pop() {
        match node {
            ExprNode::Literal { .. } => {}
            ExprNode::String { text, .. }
                if tcl_syntax::expr::fixed_string_operand(text).is_some() => {}
            ExprNode::Var { .. } if read(node) => {}
            ExprNode::Call { args, .. } if call(node) => pending.extend(args),
            ExprNode::Unary { operand, .. } => pending.push(operand),
            ExprNode::Binary { left, right, .. } => pending.extend([left.as_ref(), right.as_ref()]),
            ExprNode::Ternary {
                condition,
                true_branch,
                false_branch,
            } => {
                pending.extend([
                    condition.as_ref(),
                    true_branch.as_ref(),
                    false_branch.as_ref(),
                ]);
            }
            _ => return false,
        }
    }
    true
}

#[cfg(test)]
mod tests;
