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

//! Type propagation over the SSA graph.
//!
//! Computes a `TypeLattice` for every SSA value by iterating
//! to a fixed point over the CFG in SCCP-executable-block order.
//!
//! The pass is intentionally conservative: it only assigns a `Known`
//! type when the assignment is unambiguous (constant, `incr`, pure var
//! ref, or command with a declared `return_type`). Everything else
//! remains `Unknown` or widens to `Overdefined`.
//!
//! ## Inputs
//!
//! - `cfg` — control-flow graph.
//! - `ssa` — SSA form with phi nodes and statement use/def maps.
//! - `sccp` — SCCP result providing `executable_blocks` and
//!   `executable_edges` so unreachable branches don't widen types.
//! - `registry` — command registry for return-type look-ups.
//!
//! ## Output
//!
//! A `HashMap<ValueKey, TypeLattice>` mapping each `(name, version)`
//! pair to its inferred type. Values not present in the map are
//! implicitly `Unknown`.

use std::collections::HashMap;
use std::collections::HashSet;

use tcl_dialect::NumberSyntax;
use tcl_registry::{CommandRegistry, ReturnElements, TclType, VarElementsEffect, VarWriteTyping};
use tcl_syntax::number::{Number, ParseFlags, parse_whole_with};

use crate::analyses::{ConstValue, LatticeValue};
use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
#[cfg(test)]
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::ExprNode;
#[cfg(test)]
use crate::expr_ast::{BinOp, UnaryOp};
use crate::ir::Statement;
use crate::sccp::SccpResult;
use crate::shimmer::hints::is_pure_intrep;
use crate::ssa::{SsaFunction, Symbol, ValueKey};
use crate::types::{
    Elements, MAX_EXACT_ELEMENTS, TypeKind, TypeLattice, TypeShape, join_elements, shape_join,
    type_join,
};
use crate::var_resolve::{VariableCellKey, VariableCellTable};

// Float literal pattern: requires a decimal point so that forms like `1e3`
// (no `.`) are NOT classified as floats.
fn looks_like_float(s: &str) -> bool {
    let s = s.trim();
    s.contains('.') && s.parse::<f64>().is_ok()
}

/// Explicit standalone numeric policy for compatibility helpers.
/// Actual function typing uses the independent retained source grammar in
/// [`TypePropagationMetadata`]. No ambient numeric state participates.
#[must_use]
fn numbers_of(registry: &CommandRegistry) -> NumberSyntax {
    registry.numbers()
}

/// The integer-tower shape of a Tcl integer literal under `numbers` —
/// [`TypeShape::Int`] when the value fits a wide (`i64`), [`TypeShape::Bignum`]
/// beyond it (`9223372036854775808`, `0xFFFFFFFFFFFFFFFF` — promotion is by
/// magnitude, never wrapping; see docs type-tracking.md). `None` when
/// the text is not, in its entirety, an integer numeral of the target release.
///
/// The grammar is the shared one ([`tcl_syntax::number`]), so which spellings
/// exist follows the release: `0x` always, `0o`/`0b` from 8.5, `0d` and `_`
/// separators from 9.0, and a bare leading zero is octal up to 8.6 but decimal
/// from 9.0. A prefix the release lacks is not a prefix (`0o17` is a bareword
/// under 8.4), and radix-invalid digits (`0o8`, `0xZZ`) are never a numeral.
/// `integer_only` mirrors `TCL_PARSE_INTEGER_ONLY`: a fractional part is
/// trailing junk, so `1.5` yields `None` here and is classified as a double by
/// the caller.
#[must_use]
fn int_literal_shape(s: &str, numbers: NumberSyntax) -> Option<TypeShape> {
    let flags = ParseFlags {
        integer_only: true,
        ..ParseFlags::for_syntax(numbers)
    };
    match parse_whole_with(s, flags)? {
        Number::Int(_) => Some(TypeShape::Int),
        Number::Big { .. } => Some(TypeShape::Bignum),
        Number::Double(_) | Number::Nan { .. } => None,
    }
}

/// Whether `s` carries the `0o`/`0O` octal prefix — the one integer spelling
/// the **set-statement** classifier deliberately holds back as `String` while
/// the expr classifier calls it an `Int`.
///
/// The divergence is intentional and pinned by tests (`0o17` is `String` after
/// `set x 0o17`, `Int` inside `expr {0o17}`): a `set` stores a pure string whose
/// canonical stringified intrep (`15`) differs from the source text, whereas the
/// expr lexer tokenises the word as a number outright. Only the *spelling* is
/// held back here; whether that spelling is a numeral at all is still the
/// release's business (under 8.4 there is no `0o` prefix, so such a word is a
/// bareword and would be `String` anyway).
#[must_use]
fn has_octal_prefix(s: &str) -> bool {
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    // number-drift-ok: this recognises the `0o` *spelling* to apply the
    // intrep divergence above, not to parse a numeral — the value and its
    // validity still come from `int_literal_shape`'s dialect-aware parse. A
    // release that lacks the prefix classifies such a word `String` either way,
    // so the answer does not depend on the grammar.
    body.starts_with("0o") || body.starts_with("0O")
}

/// Classify a literal string as its Tcl intrep type (set-statement context),
/// reading numerals under `numbers` — the target release's grammar.
fn literal_type(text: &str, numbers: NumberSyntax) -> TypeLattice {
    let s = text.trim();
    if !has_octal_prefix(s)
        && let Some(shape) = int_literal_shape(s, numbers)
    {
        return TypeLattice::of_shape(shape);
    }
    if looks_like_float(s) {
        return TypeLattice::of(TclType::Double);
    }
    // Case-insensitive boolean check (full spellings — the literal
    // classifier's would-commit convention; prefixes commit only at use).
    if tcl_syntax::boolean::is_boolean_full_word(s) {
        return TypeLattice::of(TclType::Boolean);
    }
    TypeLattice::of(TclType::String)
}

/// Classify a literal's type in **expr context**, reading numerals under
/// `numbers`.
///
/// The expr parser tokenises every integer spelling the release has — decimal,
/// hex (`0xff`), octal (`0o15`), binary (`0b1010`), `0d` decimal and
/// `_`-separated forms in 9.0 — as an integer (`Tcl_GetInt` accepts them), so
/// they all map to `Int`. This is the key divergence from the set-statement
/// [`literal_type`], where a `0o…` spelling stays `String` (its canonical
/// stringified intrep differs from the source text). A literal that is not a
/// number of this release degrades to `Numeric` — an `expr` always yields a
/// number — rather than to `String`.
#[cfg(test)]
fn expr_literal_type(text: &str, numbers: NumberSyntax) -> TypeLattice {
    let s = text.trim();
    // Boolean first (full spellings — see `literal_type`).
    if tcl_syntax::boolean::is_boolean_full_word(s) {
        return TypeLattice::of(TclType::Boolean);
    }
    // One pass of the shared grammar covers the whole numeric tower: the
    // integer rungs (wide / bignum), doubles, and `Inf`/`NaN`. Anything the
    // release does not read as a number at all — `08` under the 8.x octal
    // rule, `0d99` before 9.0 — is `Numeric`, not a wrongly-narrowed `Double`.
    match parse_whole_with(s, ParseFlags::for_syntax(numbers)) {
        Some(Number::Int(_)) => TypeLattice::of(TclType::Int),
        Some(Number::Big { .. }) => TypeLattice::of_shape(TypeShape::Bignum),
        Some(Number::Double(_) | Number::Nan { .. }) => TypeLattice::of(TclType::Double),
        None => TypeLattice::of(TclType::Numeric),
    }
}

/// The enclosing namespace of a (possibly qualified) function name —
/// `"::ns::Foo"` → `"::ns"`, `"::Foo"` / `"::top"` → `"::"`.  Used to resolve
/// a relative constructor head against its call-site namespace.
fn function_namespace(qname: &str) -> String {
    match qname.rsplit_once("::") {
        Some((ns, _)) if !ns.is_empty() => ns.to_string(),
        _ => "::".to_string(),
    }
}

/// Type a registry-declared class constructor call as `OBJECT(class)` when
/// its head resolves
/// to a known class, else `OVERDEFINED`.  The relative head is resolved as-is,
/// `::`-prefixed, and against the call-site `namespace` (so `[Foo new]` inside
/// `namespace eval ns` types as `OBJECT(::ns::Foo)`).
///
/// `known_classes` carries **no deletion or lifetime information** — it is
/// the raw signature-scan class set, and this module (a flow-insensitive
/// per-symbol type inference reached from three internal sites, none of
/// which has a call-site offset to hand) has nowhere to put one. A class
/// this file later `rename`s away therefore still types its constructor
/// result here. Consumers that turn an `OBJECT(class)` into a *diagnostic*
/// must apply the liveness gate themselves — `var_command.rs`'s
/// `aggregate_object_types` does, at file-end granularity, for W308.
/// Consumers that only use the type internally (SCCP shape
/// selection, codegen hints) are unaffected: a dead class's constructor
/// raises at runtime, so no correct program reaches them.
fn constructor_object_type<S: std::hash::BuildHasher>(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
    known_classes: &HashSet<String, S>,
    namespace: &str,
) -> TypeLattice {
    let is_ctor_spelling = args
        .first()
        .is_some_and(|word| registry.is_possible_class_construction_word(word));
    if is_ctor_spelling && !known_classes.is_empty() {
        if known_classes.contains(command) {
            return TypeLattice::object_of(command);
        }
        let qualified = if command.starts_with("::") {
            command.to_string()
        } else {
            format!("::{command}")
        };
        if known_classes.contains(&qualified) {
            return TypeLattice::object_of(qualified);
        }
        if namespace != "::" && !command.starts_with("::") {
            let ns_qualified =
                crate::naming::normalise_qualified_name(&format!("{namespace}::{command}"));
            if known_classes.contains(&ns_qualified) {
                return TypeLattice::object_of(ns_qualified);
            }
        }
    }
    // Do not infer `object_of` from the `new` spelling alone.
    TypeLattice::overdefined()
}

/// Return the type produced by a known command's return value.
///
/// Checks the command spec's `return_type` field, with subcommand
/// support.  ``pub(crate)`` rather than ``pub`` so the helper
/// stays an internal API surface — only the analyser-side
/// W307 / W308 emitter consumes it today.
#[must_use]
pub(crate) fn return_type_for_command<S: std::hash::BuildHasher>(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
    known_classes: &HashSet<String, S>,
    namespace: &str,
) -> TypeLattice {
    let Some(spec) = registry.get(command) else {
        // Not a registered built-in — recognise a TclOO / snit constructor
        // (`Foo new` / `Foo create x` / `Foo %AUTO%` / `Widget .path`) whose
        // head names a known class, typing it `OBJECT(::ns::Foo)` — but not
        // `object_of` from the `new` spelling alone.
        return constructor_object_type(registry, command, args, known_classes, namespace);
    };

    // Resolved against this call, not just the command: `regexp -inline`
    // returns the matched substrings as a list where a bare `regexp` returns
    // a match count.  Subcommand dispatch and the per-form
    // refinement both live in `return_type_for_call`, so the taint sanitiser
    // test and the shimmer byte-array check cannot disagree with this.
    match spec.return_type_for_call(args) {
        Some(t) => TypeLattice::of(t),
        None => TypeLattice::overdefined(),
    }
    // NB: a registry naming-factory (`struct::graph ?name?`) is deliberately
    // *not* typed `OBJECT(class)` in the SSA lattice here.  The W307/W308
    // object-dispatch checks aggregate `fu.types` object-insensitively across
    // procs (`var_command::aggregate_object_types`), so lattice-typing a
    // factory result would leak a handle's class from one proc to a same-named
    // untyped var in another (regressing FP-OBJ-04).  The factory-return
    // provenance lives instead in `object_types::object_handle_classes` (a
    // highlight/callback-only, imprecision-tolerant map), which harvests these
    // factories syntactically without feeding the diagnostic aggregate.
}

/// Infer the type produced by an expression AST node.
///
/// Numeric operators always produce a numeric type; string comparison
/// operators produce boolean; variable references look up the known
/// type from `var_types`.
#[must_use]
#[cfg(test)]
fn infer_expr_type(
    node: &ExprNode,
    var_types: &HashMap<(String, u32, u32), TypeLattice>,
    depth: u32,
    numbers: NumberSyntax,
) -> TypeLattice {
    // Native-stack safety net: `ExprNode` operator trees are
    // walked with one native frame per nesting level. Past the cap, give up
    // conservatively with `overdefined` — the same "any type / can't tell"
    // top this function already returns for opaque `Command`/`Raw` nodes, so
    // no caller narrows a type it shouldn't.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return TypeLattice::overdefined();
    }
    match node {
        ExprNode::Literal { text, .. } => expr_literal_type(text, numbers),

        ExprNode::String { .. } | ExprNode::CompiledWord { .. } => TypeLattice::of(TclType::String),

        ExprNode::Var {
            text, start, end, ..
        } => var_types
            .get(&(text.clone(), *start, *end))
            .cloned()
            .unwrap_or_else(TypeLattice::unknown),

        ExprNode::Binary {
            op, left, right, ..
        } => {
            match op {
                // BITWISE / shift → always Int (Tcl `expr` coerces the
                // operands to integers), never the weaker Numeric that
                // arithmetic yields.
                BinOp::LShift | BinOp::RShift | BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => {
                    TypeLattice::of(TclType::Int)
                }

                // LOGICAL / COMPARISON → always Boolean.  The
                // six iRules string predicates (`contains` / `starts_with`
                // / `ends_with` / `equals` / `matches_glob` /
                // `matches_regex`) plus the word-logical `and` / `or` used
                // to fall through `_ => overdefined()`.
                BinOp::And
                | BinOp::Or
                | BinOp::WordAnd
                | BinOp::WordOr
                | BinOp::Eq
                | BinOp::Ne
                | BinOp::Lt
                | BinOp::Le
                | BinOp::Gt
                | BinOp::Ge
                | BinOp::StrEq
                | BinOp::StrNe
                | BinOp::StrLt
                | BinOp::StrLe
                | BinOp::StrGt
                | BinOp::StrGe
                | BinOp::In
                | BinOp::Ni
                | BinOp::Contains
                | BinOp::StartsWith
                | BinOp::EndsWith
                | BinOp::StrEquals
                | BinOp::Matches
                | BinOp::MatchesGlob
                | BinOp::MatchesRegex => TypeLattice::of(TclType::Boolean),

                // ARITHMETIC / DIVISION → `_arithmetic_result` over the
                // operand types, but only when both are `Known`;
                // otherwise Numeric.
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Pow => {
                    let lt = infer_expr_type(left, var_types, depth + 1, numbers);
                    let rt = infer_expr_type(right, var_types, depth + 1, numbers);
                    if lt.kind() == TypeKind::Known && rt.kind() == TypeKind::Known {
                        arithmetic_result(&lt, &rt)
                    } else {
                        TypeLattice::of(TclType::Numeric)
                    }
                }
            }
        }

        ExprNode::Unary { op, operand, .. } => match op {
            // Arithmetic sign is identity (same intrep as the operand);
            // bitwise NOT always coerces to `Int` (`~$double` → Int);
            // logical NOT yields `Boolean`.'
            // `UnaryOpKind` BITWISE arm (`~` was grouped with
            // the identity ops and leaked the operand's `Double`).
            UnaryOp::Neg | UnaryOp::Pos => infer_expr_type(operand, var_types, depth + 1, numbers),
            UnaryOp::BitNot => TypeLattice::of(TclType::Int),
            UnaryOp::Not | UnaryOp::WordNot => TypeLattice::of(TclType::Boolean),
        },

        ExprNode::Ternary {
            true_branch,
            false_branch,
            ..
        } => {
            let tt = infer_expr_type(true_branch, var_types, depth + 1, numbers);
            let ft = infer_expr_type(false_branch, var_types, depth + 1, numbers);
            type_join(&tt, &ft)
        }

        // Math-function calls resolve through the expr-function table
        // — `sqrt($x)` is Double, `int(...)` is Int, etc.
        ExprNode::Call { function, args, .. } => {
            expr_call_type(function, args, var_types, depth, numbers)
        }

        // Command substitutions and raw/unrecognised expression text
        // need the registry (or runtime context) to resolve. Without it
        // we over-approximate to overdefined; the outer
        // evaluate_type_def handles command-sub type resolution where the
        // registry is in scope.
        ExprNode::Command { .. } | ExprNode::Raw { .. } => TypeLattice::overdefined(),
    }
}

/// INT op INT → INT
/// (boolean counts as int), DOUBLE anywhere → DOUBLE, otherwise
/// NUMERIC.  Callers guarantee both operand types are `Known`.
#[cfg(test)]
fn arithmetic_result(lt: &TypeLattice, rt: &TypeLattice) -> TypeLattice {
    match (lt.tcl_type(), rt.tcl_type()) {
        (Some(TclType::Int | TclType::Boolean), Some(TclType::Int | TclType::Boolean)) => {
            TypeLattice::of(TclType::Int)
        }
        (Some(TclType::Double), _) | (_, Some(TclType::Double)) => TypeLattice::of(TclType::Double),
        _ => TypeLattice::of(TclType::Numeric),
    }
}

/// Resolve a Tcl `expr` math-function call to its result type.
///
/// `abs` is identity
/// (preserves its operand's type), `max` / `min` join their operand
/// types, every other built-in returns its declared type, and an
/// unknown function is conservatively `Numeric` (an `expr` function
/// always yields a number).
#[cfg(test)]
fn expr_call_type(
    function: &str,
    args: &[ExprNode],
    var_types: &HashMap<(String, u32, u32), TypeLattice>,
    depth: u32,
    numbers: NumberSyntax,
) -> TypeLattice {
    // `depth` is the level of the enclosing `Call` node; its args are one
    // level deeper; `infer_expr_type` guards the cap itself.
    // Identity: `abs` preserves the operand type (Int fallback).
    if function == "abs" {
        return match args.first() {
            Some(a) => infer_expr_type(a, var_types, depth + 1, numbers),
            None => TypeLattice::of(TclType::Int),
        };
    }
    // Variadic join: `max` / `min` join all operand types.
    if function == "max" || function == "min" {
        let mut it = args.iter();
        return match it.next() {
            Some(first) => {
                let mut acc = infer_expr_type(first, var_types, depth + 1, numbers);
                for a in it {
                    acc = type_join(&acc, &infer_expr_type(a, var_types, depth + 1, numbers));
                }
                acc
            }
            None => TypeLattice::of(TclType::Numeric),
        };
    }
    match function {
        // Integer-returning conversions. NB: `ceil`/`floor` are NOT here — they
        // return a *double* in Tcl (`expr {ceil(3.14)}` → 4.0, `string is
        // integer 4.0` → 0), unlike `round`/`int`/`entier` which round to an
        // integer. Verified against tclsh8.6/9.0.
        "int" | "round" | "isqrt" | "wide" | "entier" => TypeLattice::of(TclType::Int),
        // Double-returning math (incl. ceil/floor, which yield N.0).  The
        // Tcl 9.1 C99 additions (TIP 745, verified against tmp/tcl9.1-src) are
        // all double-valued except the `signbit` predicate below.
        "double" | "ceil" | "floor" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan"
        | "atan2" | "sinh" | "cosh" | "tanh" | "sqrt" | "exp" | "log" | "log10" | "pow"
        | "hypot" | "fmod" | "rand" | "srand" | "acosh" | "asinh" | "atanh" | "cbrt"
        | "copysign" | "dim" | "erf" | "erfc" | "exp2" | "expm1" | "fma" | "gamma" | "ldexp"
        | "lgamma" | "log1p" | "log2" | "logb" | "nextafter" | "remainder" | "trunc" => {
            TypeLattice::of(TclType::Double)
        }
        // Boolean-returning predicates.  `signbit` yields 0/1 (Tcl 9.1, TIP 745).
        "bool" | "isnan" | "isinf" | "signbit" => TypeLattice::of(TclType::Boolean),
        // Unknown function — conservative.
        _ => TypeLattice::of(TclType::Numeric),
    }
}

/// True when `command` (with `args`) creates a scope alias — see
/// [`crate::var_scoping::is_scope_alias_call`], the shared recogniser.
///
/// Such a statement imports an externally-determined variable whose intrep
/// lives in another scope, so its def must widen to `Overdefined` rather
/// than take the command's nominal (`String`) return type — otherwise a
/// use-site / merge shimmer check fires on a nominally-`String`-typed
/// alias.
fn is_scope_alias_call(registry: &CommandRegistry, command: &str, args: &[String]) -> bool {
    crate::var_scoping::is_scope_alias_call(registry, command, args)
}

/// Retain executable type provenance only when no widening fact owns the key.
fn type_version(
    ssa: &SsaFunction,
    symbol: Symbol,
    version: u32,
    types: &HashMap<ValueKey, TypeLattice>,
) -> u32 {
    if types.contains_key(&(symbol, version)) {
        version
    } else {
        ssa.binding_version(symbol, version)
    }
}

/// Conditional normal result types keyed by original implementation allocation.
pub(crate) type NormalProcedureResultTypes =
    HashMap<crate::command_binding::CommandAllocation, TypeLattice>;

/// Read-only word-shape context at one retained program point.
struct WordTypingCtx<'a, S: std::hash::BuildHasher> {
    preparations: &'a [crate::command_binding::SourceExpressionPreparation],
    tokens: Option<&'a crate::ir::CommandTokens>,
    source: crate::ssa::SsaSourceView<'a>,
    uses: &'a HashMap<Symbol, u32>,
    /// Complete retained availability; this supplies no executed handler.
    context: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    /// Exact lexical and list policy of the original source.
    config: tcl_lexer::LexerConfig,
    types: &'a HashMap<ValueKey, TypeLattice>,
    /// SCCP constants at this point — purity evidence (a numeric-typed
    /// literal is still a pure string) and constant list/index values for
    /// element facts.
    values: &'a HashMap<ValueKey, LatticeValue>,
    registry: &'a CommandRegistry,
    known_classes: &'a HashSet<String, S>,
    namespace: &'a str,
    ssa: &'a SsaFunction,
    /// The actual source's numeric grammar, independently of the command store.
    numbers: NumberSyntax,
    /// Active provenance write points; cycles cannot establish a value type.
    active_writes: &'a [(BlockId, usize)],
    normal_results: Option<&'a NormalProcedureResultTypes>,
}

impl<S: std::hash::BuildHasher> WordTypingCtx<'_, S> {
    fn is_variable_word(&self, spelling: &str) -> bool {
        tcl_lexer::word_parts::whole_var_ref(spelling.as_bytes(), self.config)
            .is_ok_and(|reference| reference.is_some())
    }

    fn variable_key(&self, spelling: &str) -> Option<VariableCellKey> {
        self.source
            .symbol(spelling)
            .map(|symbol| self.ssa.cell_key(symbol).clone())
    }

    fn variable_type(&self, spelling: &str) -> Option<TypeLattice> {
        if self.source.is_positioned() {
            let read = self.source.read_spelling(spelling)?;
            if let Some(version) = read.version {
                let version = type_version(self.ssa, read.symbol, version, self.types);
                return self.types.get(&(read.symbol, version)).cloned();
            }
            return self
                .source
                .read_spelling_contents(spelling, self.registry)
                .map(|contents| type_of_read_contents(self, contents));
        }
        let symbol = self.source.symbol(spelling)?;
        let version = type_version(self.ssa, symbol, *self.uses.get(&symbol)?, self.types);
        (version != 0)
            .then(|| self.types.get(&(symbol, version)).cloned())
            .flatten()
    }

    /// Expression types are keyed by original occurrence, independently of
    /// base-name dependency labels and conflicting same-spelling reads.
    fn expression_variable_types(
        &self,
        expression: &ExprNode,
        expression_base: Option<u32>,
    ) -> HashMap<(String, u32, u32), TypeLattice> {
        expression
            .variable_nodes()
            .into_iter()
            .filter_map(|node| {
                let ExprNode::Var {
                    text, start, end, ..
                } = node
                else {
                    return None;
                };
                let value = self
                    .source
                    .read_expression_variable(node, expression_base)
                    .and_then(|read| {
                        let version =
                            type_version(self.ssa, read.symbol, read.version?, self.types);
                        self.types.get(&(read.symbol, version)).cloned()
                    })
                    .or_else(|| {
                        self.source
                            .read_expression_variable_contents(node, expression_base, self.registry)
                            .map(|contents| type_of_read_contents(self, contents))
                    })?;
                Some(((text.clone(), *start, *end), value))
            })
            .collect()
    }

    /// Written substitutions select their original read; decoded names never
    /// pass through a second substitution decoder.
    fn written_variable_type(
        &self,
        spelling: &str,
        retained: Option<&crate::ir::WordExpr>,
    ) -> Option<TypeLattice> {
        if let Some(word) = retained {
            return Some(retained_contents_type(self, word));
        }
        let mut selected = None;
        for word in self
            .tokens?
            .words()
            .iter()
            .filter(|word| written_variable_word_matches(word, spelling))
        {
            let value = retained_contents_type(self, word);
            if selected.as_ref().is_some_and(|previous| *previous != value) {
                return None;
            }
            selected = Some(value);
        }
        selected
    }

    /// Contents read by an admitted native named-cell update. The operand is
    /// an evaluated variable name, rather than a substitution from argv.
    fn named_cell_type(&self, name: &str) -> Option<TypeLattice> {
        let symbol = self.source.symbol(name)?;
        let version = type_version(self.ssa, symbol, *self.uses.get(&symbol)?, self.types);
        if version == 0 {
            return None;
        }
        self.types.get(&(symbol, version)).cloned()
    }

    fn named_cell_constant(&self, name: &str) -> Option<&LatticeValue> {
        let symbol = self.source.symbol(name)?;
        self.values.get(&(symbol, *self.uses.get(&symbol)?))
    }

    fn with_tokens<'b>(&'b self, tokens: &'b crate::ir::CommandTokens) -> WordTypingCtx<'b, S> {
        WordTypingCtx {
            preparations: self.preparations,
            tokens: Some(tokens),
            source: self.source,
            uses: self.uses,
            context: self.context,
            config: self.config,
            types: self.types,
            values: self.values,
            registry: self.registry,
            known_classes: self.known_classes,
            namespace: self.namespace,
            ssa: self.ssa,
            numbers: self.numbers,
            active_writes: self.active_writes,
            normal_results: self.normal_results,
        }
    }

    /// Constant contents from the original written substitution reads.
    fn constant_of_word(&self, spelling: &str) -> Option<&LatticeValue> {
        let mut selected = None;
        for word in self
            .tokens?
            .words()
            .iter()
            .filter(|word| written_variable_word_matches(word, spelling))
        {
            let read = self.source.read_word(word)?;
            let value = self.values.get(&(read.symbol, read.version?))?;
            if selected.is_some_and(|previous| previous != value) {
                return None;
            }
            selected = Some(value);
        }
        selected
    }
}

fn written_variable_word_matches(word: &crate::ir::WordExpr, spelling: &str) -> bool {
    word.sole_variable_substitution()
        .is_some_and(|(original, _)| original == spelling || word.legacy_text() == spelling)
}

/// The shape a builder argument *word* contributes as a container element,
/// or `None` when no shape claim is defensible for downstream checks.
///
/// Faithful to the runtime's by-reference elements (see docs
/// type-tracking.md): a container shares its argument objects, so a
/// **committed** source keeps its intrep inside the container —
/// `[list [expr {2**20}] x]` genuinely holds an int, `[list [C new]]` an
/// object. A **pure** source (a literal word, an interpolation, a
/// still-pure variable, a string-returning command) contributes a pure
/// string whose downstream conversion validity depends on its *value* — a
/// fact the type lattice cannot carry per element — so those positions stay
/// agnostic rather than invite the FP classes either concrete claim brings
/// (FP-SH-17 pins `lassign {1 2 3}` targets silent in both arithmetic and
/// string comparisons). The purity split is [`is_pure_intrep`] — the same
/// predicate the shimmer suppression uses.
fn element_word_shape<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    word: &str,
    retained: Option<&crate::ir::WordExpr>,
) -> Option<TypeShape> {
    let stripped = word.trim();
    if stripped.starts_with("{*}") {
        return None;
    }
    if ctx.is_variable_word(stripped) {
        let t = ctx.written_variable_type(stripped, retained)?;
        let shape = t.single_shape()?.clone();
        let constant = retained.map_or_else(
            || ctx.constant_of_word(stripped),
            |word| {
                let read = ctx.source.read_word(word)?;
                ctx.values.get(&(read.symbol, read.version?))
            },
        );
        if is_pure_intrep(shape.coarse(), constant) {
            return None;
        }
        return Some(shape);
    }
    if stripped.starts_with('[') && stripped.ends_with(']') {
        let shape = value_word_type(ctx, stripped, retained)
            .single_shape()?
            .clone();
        // A string-returning command (`string trim`, `format`) yields a pure
        // result — no committed intrep enters the container.
        if is_pure_intrep(shape.coarse(), None) {
            return None;
        }
        return Some(shape);
    }
    // Bare literal or interpolation: a fresh pure string object of known
    // spelling but per-element-untracked value.
    None
}

/// Element facts for a builder's result, per the registry's
/// [`ReturnElements`] declaration. `None` when the fact does not apply to
/// this call shape (a multi-level `dict get`, an unknown container).
fn return_elements_lattice<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    fact: ReturnElements,
    args: &[&str],
    retained: Option<&[crate::ir::WordExpr]>,
) -> Option<TypeLattice> {
    match fact {
        ReturnElements::ListOfArgs { from } => {
            let words = args.get(usize::from(from)..).unwrap_or(&[]);
            Some(TypeLattice::of_shape(TypeShape::List(
                build_exact_elements(
                    ctx,
                    words,
                    retained.and_then(|words| words.get(usize::from(from)..)),
                ),
            )))
        }
        ReturnElements::DictOfPairs { from } => {
            let words = args.get(usize::from(from)..).unwrap_or(&[]);
            // Values are the odd offsets of the key/value pairs.
            let value_words: Vec<&str> = words.iter().skip(1).step_by(2).copied().collect();
            let retained_values = retained
                .and_then(|words| words.get(usize::from(from)..))
                .map(|words| words.iter().skip(1).step_by(2).cloned().collect::<Vec<_>>());
            Some(TypeLattice::of_shape(TypeShape::Dict(uniform_elements_of(
                ctx,
                &value_words,
                retained_values.as_deref(),
            ))))
        }
        ReturnElements::ElementOf { container_arg } => {
            // Single-step retrieval only: exactly one index/key word after
            // the container (`lindex $l $i`, `dict get $d $k`).
            let container_idx = usize::from(container_arg);
            if args.len() != container_idx + 2 {
                return None;
            }
            let container = args.get(container_idx)?;
            if !ctx.is_variable_word(container) {
                return None;
            }
            let t = ctx.written_variable_type(
                container,
                retained.and_then(|words| words.get(container_idx)),
            )?;
            let elements = t.elements()?;
            // A constant integer index resolves an Exact position; any
            // other index falls back to the uniform bound.
            let index_word = args.get(container_idx + 1)?;
            let shape = constant_index(ctx, index_word)
                .and_then(|i| elements.shape_at(i).cloned())
                .or_else(|| elements.uniform_shape())?;
            Some(TypeLattice::of_shape(shape))
        }
        ReturnElements::SubListOf { container_arg } => {
            let container = args.get(usize::from(container_arg))?;
            if !ctx.is_variable_word(container) {
                return None;
            }
            let elements = ctx
                .written_variable_type(
                    container,
                    retained.and_then(|words| words.get(usize::from(container_arg))),
                )?
                .elements()?
                .uniform_shape()
                .map_or(Elements::Unknown, |u| Elements::Uniform(Box::new(u)));
            Some(TypeLattice::of_shape(TypeShape::List(elements)))
        }
    }
}

/// `Exact` per-position elements for a builder's argument words, widening to
/// a uniform bound (or no facts) past [`MAX_EXACT_ELEMENTS`] or on an
/// `{*}`-expansion word (whose element count is unknown).
fn build_exact_elements<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    words: &[&str],
    retained: Option<&[crate::ir::WordExpr]>,
) -> Elements {
    if words.iter().any(|w| w.trim().starts_with("{*}")) {
        return uniform_elements_of(ctx, words, retained);
    }
    if words.len() > MAX_EXACT_ELEMENTS {
        return uniform_elements_of(ctx, words, retained);
    }
    Elements::Exact(
        words
            .iter()
            .enumerate()
            .map(|(index, w)| {
                element_word_shape(ctx, w, retained.and_then(|words| words.get(index)))
            })
            .collect(),
    )
}

/// The uniform element bound of a word set: the single-shape join of every
/// word's shape, or no facts when any word is shapeless / unjoinable.
fn uniform_elements_of<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    words: &[&str],
    retained: Option<&[crate::ir::WordExpr]>,
) -> Elements {
    let mut acc: Option<TypeShape> = None;
    for (index, word) in words.iter().enumerate() {
        // `{*}$expansion` contributes its *list's* element shapes, which are
        // unknown here — no uniform claim survives.
        let word = word.trim();
        let shape = if let Some(expanded) = word.strip_prefix("{*}") {
            let Some(TypeShape::List(elements)) = (if ctx.is_variable_word(expanded) {
                ctx.written_variable_type(expanded, None)
                    .and_then(|t| t.single_shape().cloned())
            } else {
                None
            }) else {
                return Elements::Unknown;
            };
            match elements.uniform_shape() {
                Some(u) => u,
                None => return Elements::Unknown,
            }
        } else {
            match element_word_shape(ctx, word, retained.and_then(|words| words.get(index))) {
                Some(s) => s,
                None => return Elements::Unknown,
            }
        };
        acc = Some(match acc {
            None => shape,
            Some(prev) => match shape_join(&prev, &shape) {
                Some(joined) => joined,
                None => return Elements::Unknown,
            },
        });
    }
    match acc {
        Some(shape) => Elements::Uniform(Box::new(shape)),
        None => Elements::Unknown,
    }
}

/// The constant element index a retrieval word denotes: a literal integer
/// (`lindex $l 0`) or a `$var` whose SCCP constant is an integer.
fn constant_index<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    word: &str,
) -> Option<usize> {
    let word = word.trim();
    if let Ok(i) = word.parse::<usize>() {
        return Some(i);
    }
    if ctx.is_variable_word(word)
        && let Some(LatticeValue::Const(ConstValue::Int(i))) = ctx.constant_of_word(word)
    {
        return usize::try_from(*i).ok();
    }
    None
}

/// The evolved container shape after an in-place element write —
/// `lappend var v…` / `dict set var … v` — per the registry's
/// [`VarElementsEffect`]. Generalises the old object-only element-class
/// harvesting: every element shape flows, so `lappend l [C new]` still
/// yields `List<OBJECT(C)*>` and `lappend l [list 1]` now yields real
/// element facts too.
fn var_elements_effect_lattice<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    effect: VarElementsEffect,
    target: &str,
    args: &[&str],
    base: usize,
    retained: Option<&[crate::ir::WordExpr]>,
) -> TypeLattice {
    let (container_ctor, value_words): (fn(Elements) -> TypeShape, &[&str]) = match effect {
        VarElementsEffect::SetsArrayElementsFromList { .. } => return TypeLattice::overdefined(),
        VarElementsEffect::AppendsListElements { values_from } => (
            TypeShape::List,
            args.get(base + usize::from(values_from)..).unwrap_or(&[]),
        ),
        VarElementsEffect::SetsDictValue => (
            TypeShape::Dict,
            args.len()
                .checked_sub(1)
                .map_or(&[][..], |last| &args[last..]),
        ),
        VarElementsEffect::ListifiesDictValue => (TypeShape::Dict, &[]),
        VarElementsEffect::ExtendsDictValuesByName { values_from } => (
            TypeShape::Dict,
            args.get(base + usize::from(values_from)..).unwrap_or(&[]),
        ),
    };

    let retained_values =
        retained.and_then(|words| words.get(args.len().saturating_sub(value_words.len())..));
    let prior = prior_container_elements(ctx, target);
    let evolved = match effect {
        VarElementsEffect::SetsArrayElementsFromList { .. } => return TypeLattice::overdefined(),
        VarElementsEffect::AppendsListElements { .. } => {
            append_list_elements(ctx, prior, value_words, retained_values)
        }
        // Dict values: join the new value shape into the prior uniform
        // bound — per-key tracking is out of scope (type-tracking.md).
        // Only the single-key `dict set var key value` carries the leaf
        // word's shape; a nested path stores a *dict* under the first key
        // (`dict get $d outer` is a dict — tclsh-verified), so the
        // contribution is a Dict wrapper with unknown structure.
        VarElementsEffect::SetsDictValue => {
            let single_key = args.len().saturating_sub(base) == 3;
            let incoming = if single_key {
                uniform_elements_of(ctx, value_words, retained_values)
            } else {
                Elements::Uniform(Box::new(TypeShape::Dict(Elements::Unknown)))
            };
            match prior {
                Some(prior) => join_elements(&prior, &incoming),
                None => incoming,
            }
        }
        // `dict append` concatenates: value intreps do not survive, but an
        // object's dispatch identity (the objref text) does — only
        // object-class shapes flow into the bound (the collection-of-objects
        // pattern); anything else contributes no
        // element fact.
        VarElementsEffect::ExtendsDictValuesByName { .. } => {
            let incoming = uniform_elements_of(ctx, value_words, retained_values);
            let object_only = match &incoming {
                Elements::Uniform(shape) if matches!(**shape, TypeShape::Object(_)) => {
                    Some(incoming.clone())
                }
                _ => None,
            };
            match (prior, object_only) {
                (Some(prior), Some(inc)) => join_elements(&prior, &inc),
                (Some(prior), None) => prior,
                (None, Some(inc)) => inc,
                (None, None) => Elements::Unknown,
            }
        }
        // `dict lappend`: the key's value becomes a list (a prior scalar
        // becomes element 0), so the wrapper is the fact and the element
        // shapes stay unknown.
        VarElementsEffect::ListifiesDictValue => {
            let incoming = Elements::Uniform(Box::new(TypeShape::List(Elements::Unknown)));
            match prior {
                Some(prior) => join_elements(&prior, &incoming),
                None => incoming,
            }
        }
    };
    TypeLattice::of_shape(container_ctor(evolved))
}

/// The target variable's element facts *before* this write: its tracked
/// container elements, or — for a pure list-shaped constant (`set l {a b};
/// lappend l c`) — the parsed literal's per-position pure-string elements.
/// `None` when nothing is known (including an uninitialised accumulator).
fn prior_container_elements<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    target: &str,
) -> Option<Elements> {
    if let Some(t) = ctx.named_cell_type(target) {
        if t.kind() == crate::types::TypeKind::Overdefined {
            return Some(Elements::Unknown);
        }
        if let Some(e) = t.elements() {
            return Some(e.clone());
        }
        // A pure string constant parses to a known element count of pure
        // strings — `lappend` on `{a b}` starts from two `String` elements.
        let rules = tcl_syntax::word_rules::WordValueRules::from_config(&ctx.config);
        if t.tcl_type() == Some(TclType::String)
            && let Some(LatticeValue::Const(ConstValue::String(text))) =
                ctx.named_cell_constant(target)
            && let Ok(parsed) = rules.split_list(text)
        {
            if parsed.len() > MAX_EXACT_ELEMENTS {
                return Some(Elements::Uniform(Box::new(TypeShape::String)));
            }
            return Some(Elements::Exact(
                parsed.iter().map(|_| Some(TypeShape::String)).collect(),
            ));
        }
    }
    None
}

/// Append `value_words` as new elements after `prior`, keeping `Exact`
/// arity when both sides pin one (the phi joins collapse mismatched
/// loop-carried arities — see [`join_elements`] — so the fixpoint
/// terminates).
fn append_list_elements<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    prior: Option<Elements>,
    value_words: &[&str],
    retained: Option<&[crate::ir::WordExpr]>,
) -> Elements {
    let appended = build_exact_elements(ctx, value_words, retained);
    match (prior, appended) {
        (None, appended) => appended,
        (Some(Elements::Exact(existing)), Elements::Exact(new)) => {
            let mut merged = existing.into_vec();
            merged.extend(new.into_vec());
            if merged.len() > MAX_EXACT_ELEMENTS {
                let shapes: Vec<TypeShape> = merged.into_iter().flatten().collect();
                return uniform_bound_of_shapes(&shapes);
            }
            Elements::Exact(merged.into_boxed_slice())
        }
        (Some(prior), appended) => {
            // No exact arity on one side: fall back to the joint uniform
            // bound when both sides admit one.
            match (prior.uniform_shape(), appended.uniform_shape()) {
                (Some(a), Some(b)) => match shape_join(&a, &b) {
                    Some(joined) => Elements::Uniform(Box::new(joined)),
                    None => Elements::Unknown,
                },
                _ => Elements::Unknown,
            }
        }
    }
}

/// The uniform bound over a shape list, or no facts when empty / unjoinable.
fn uniform_bound_of_shapes(shapes: &[TypeShape]) -> Elements {
    let mut acc: Option<TypeShape> = None;
    for shape in shapes {
        acc = Some(match acc {
            None => shape.clone(),
            Some(prev) => match shape_join(&prev, shape) {
                Some(joined) => joined,
                None => return Elements::Unknown,
            },
        });
    }
    match acc {
        Some(shape) => Elements::Uniform(Box::new(shape)),
        None => Elements::Unknown,
    }
}

/// Successful result representation is independent of permission to emit a
/// native opcode. Nested operand types retain their exact read provenance.
fn normal_result_type<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    tokens: &crate::ir::CommandTokens,
) -> TypeLattice {
    if let Some(target) = tokens
        .source_binding
        .as_ref()
        .and_then(crate::command_binding::SourceInvocationBinding::proved_handler_target)
        && target.kind == crate::command_binding::BindingKind::Proc
        && let Some(allocation) = &target.implementation_allocation
        && let Some(result) = ctx
            .normal_results
            .and_then(|results| results.get(allocation))
    {
        return result.clone();
    }
    if let Some(result) = prepared_expression_result_type(ctx, tokens) {
        return result;
    }
    let Some(invocation) =
        crate::registry_invocation::normal_representation_invocation_with_metadata_context(
            ctx.registry,
            ctx.context,
            tokens,
        )
    else {
        return TypeLattice::overdefined();
    };
    if let Some(elements) = invocation.result_elements()
        && let Some(words) = invocation
            .effective_words()
            .words
            .get(1 + invocation.argument_offset()..)
    {
        let arguments = words
            .iter()
            .map(crate::ir::WordExpr::legacy_text)
            .collect::<Vec<_>>();
        let references = arguments.iter().map(String::as_str).collect::<Vec<_>>();
        if let Some(result) =
            return_elements_lattice(&ctx.with_tokens(tokens), elements, &references, Some(words))
        {
            return result;
        }
    }
    invocation
        .result_representation_type()
        .map_or_else(TypeLattice::overdefined, TypeLattice::of)
}

/// Conditional representation of an authenticated original expression.
/// Operand normalisation and current authored pool evidence are separate from
/// arithmetic category, physical object evidence and executable admission.
fn prepared_expression_result_type<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    tokens: &crate::ir::CommandTokens,
) -> Option<TypeLattice> {
    if let Some(advice) =
        crate::registry_invocation::original_expression_operand_advice(ctx.registry, tokens)
    {
        let variables =
            ctx.expression_variable_types(&advice.expression, Some(advice.expression_base));
        let receipt = tokens
            .source_binding
            .as_ref()?
            .conditional_expression_evaluation(ctx.registry, tokens)?;
        return receipt
            .normal_result_representation(|node| {
                let ExprNode::Var {
                    text, start, end, ..
                } = node
                else {
                    return None;
                };
                variables.get(&(text.clone(), *start, *end))?.tcl_type()
            })
            .map(TypeLattice::of);
    }
    let preparation = tokens
        .source_binding
        .as_ref()?
        .expression_preparation(ctx.preparations)?;
    let production = preparation.witness.normal_numeric_result_production()?;
    Some(TypeLattice::of(production.result_type()))
}

/// A lowered expression retains its original sole child invocation. Analysis
/// IR topology alone does not establish a current expression result intrep.
fn lowered_expression_result_type<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    tokens: Option<&crate::ir::CommandTokens>,
    expression: &ExprNode,
    base: Option<u32>,
) -> TypeLattice {
    let Some(tokens) = tokens else {
        return TypeLattice::overdefined();
    };
    let config = ctx.config;
    let Some(calls) = crate::word_subst::checked_lifted_calls(tokens, config) else {
        return TypeLattice::overdefined();
    };
    let mut result = None;
    for call in calls {
        let Some(child) = call.tokens else {
            continue;
        };
        let Some(advice) =
            crate::registry_invocation::original_expression_operand_advice(ctx.registry, &child)
        else {
            continue;
        };
        if advice.expression != *expression || Some(advice.expression_base) != base {
            continue;
        }
        if result.is_some() {
            return TypeLattice::overdefined();
        }
        result = prepared_expression_result_type(ctx, &child);
    }
    result.unwrap_or_else(TypeLattice::overdefined)
}

/// Infer the intrep a `set`-style value *word* stores.
///
/// The shared body of the [`Statement::AssignValue`] typing and the
/// value-passthrough typing of a canonically-`set` [`Statement::Call`] (an
/// aliased or renamed `set`).  A pure `$var` reference inherits the source
/// version's type, a `[cmd …]` command substitution takes the command's
/// declared return type (or an object-collection retrieval), an
/// interpolated / otherwise-complex word is `String`, and a bare literal is
/// classified by its Tcl intrep.
fn value_word_type<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    value: &str,
    retained: Option<&crate::ir::WordExpr>,
) -> TypeLattice {
    use crate::ir::{WordExpr, WordPart};
    let config = ctx.config;
    let parsed = retained
        .is_none()
        .then(|| crate::value_shapes::value_word_with_config(value, config))
        .flatten();
    let Some(word) = retained.or(parsed.as_ref()) else {
        return TypeLattice::overdefined();
    };
    let variable = match word {
        WordExpr::Variable { spelling, .. } => Some(spelling.as_str()),
        WordExpr::Template { parts, .. } => match parts.as_slice() {
            [WordPart::Variable { spelling, .. }] => Some(spelling.as_str()),
            _ => None,
        },
        _ => None,
    };
    if let Some(spelling) = variable {
        if ctx.source.is_positioned() {
            return retained_contents_type(ctx, word);
        }
        let Some(reference) = tcl_lexer::word_parts::whole_var_ref(spelling.as_bytes(), config)
            .ok()
            .flatten()
        else {
            return TypeLattice::overdefined();
        };
        let Ok(name) = core::str::from_utf8(reference.name) else {
            return TypeLattice::overdefined();
        };
        return ctx
            .variable_type(name)
            .unwrap_or_else(TypeLattice::overdefined);
    }
    if word.sole_command_substitution().is_some() {
        return command_word_result_type(ctx, word);
    }
    match word {
        WordExpr::Literal { text, .. } | WordExpr::BracedLiteral { text, .. } => {
            literal_type(text, ctx.numbers)
        }
        WordExpr::Template { .. } => TypeLattice::of(TclType::String),
        WordExpr::Expand { .. }
        | WordExpr::Opaque { .. }
        | WordExpr::Variable { .. }
        | WordExpr::CommandSubstitution { .. } => TypeLattice::overdefined(),
    }
}

/// Type the original sole nested command using the same source metadata.
fn command_word_result_type<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    word: &crate::ir::WordExpr,
) -> TypeLattice {
    let config = ctx.config;
    let Some(mut commands) =
        crate::value_shapes::command_substitution_tokens(word, ctx.tokens, config)
    else {
        return TypeLattice::overdefined();
    };
    // Earlier commands may replace the last implementation or return
    // abruptly. Their composed result is not a nominal string value.
    if commands.len() != 1 {
        return TypeLattice::overdefined();
    }
    let tokens = commands.remove(0);
    if let Some(class_name) = tokens
        .source_binding
        .as_ref()
        .and_then(|binding| binding.proved_construction_result(ctx.registry))
    {
        return TypeLattice::object_of(class_name);
    }
    let Some(invocation) =
        crate::registry_invocation::resolved_tokens_invocation_with_metadata_context(
            ctx.registry,
            ctx.context,
            &tokens,
        )
    else {
        return normal_result_type(ctx, &tokens);
    };
    if let Some(result) = prepared_expression_result_type(ctx, &tokens) {
        return result;
    }
    let arguments = invocation
        .arguments
        .iter()
        .map(Option::as_deref)
        .collect::<Option<Vec<_>>>();
    if let Some(fact) = invocation.facts.return_elements
        && let Some(arguments) = arguments.as_ref()
    {
        let elements = arguments
            .get(invocation.facts.argument_offset..)
            .unwrap_or(&[]);
        if let Some(value) = return_elements_lattice(
            &ctx.with_tokens(&tokens),
            fact,
            elements,
            invocation
                .effective
                .words
                .get(1 + invocation.facts.argument_offset..),
        ) {
            return value;
        }
    }
    invocation
        .facts
        .return_type
        .map_or_else(TypeLattice::overdefined, TypeLattice::of)
}

/// Join the actual stores reaching a read whose memory contents do not have
/// one scalar SSA version. The shared read owner proves root/lifetime overlap;
/// this adapter only supplies the value lattice of those retained statements.
fn retained_contents_type<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    word: &crate::ir::WordExpr,
) -> TypeLattice {
    if let Some(read) = ctx.source.read_word(word)
        && let Some(version) = read.version
    {
        let version = type_version(ctx.ssa, read.symbol, version, ctx.types);
        return ctx
            .types
            .get(&(read.symbol, version))
            .cloned()
            .unwrap_or_else(TypeLattice::overdefined);
    }
    let Some(contents) = ctx.source.read_word_contents(word, ctx.registry) else {
        return TypeLattice::overdefined();
    };
    type_of_read_contents(ctx, contents)
}

fn type_of_read_contents<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    contents: crate::ssa::SsaReadContents,
) -> TypeLattice {
    if contents.unknown_residual {
        return TypeLattice::overdefined();
    }
    if let Some(version) = contents.reference.version {
        let version = type_version(ctx.ssa, contents.reference.symbol, version, ctx.types);
        return ctx
            .types
            .get(&(contents.reference.symbol, version))
            .cloned()
            .unwrap_or_else(TypeLattice::overdefined);
    }
    let mut result = if contents.includes_incoming {
        TypeLattice::overdefined()
    } else {
        TypeLattice::unknown()
    };
    for (block, index) in contents.writes {
        if ctx.active_writes.contains(&(block, index)) {
            return TypeLattice::overdefined();
        }
        let Some(statement) = ctx
            .ssa
            .blocks
            .get(&block)
            .and_then(|block| block.statements.get(index))
        else {
            return TypeLattice::overdefined();
        };
        let mut active_writes = ctx.active_writes.to_vec();
        active_writes.push((block, index));
        let write_ctx = WordTypingCtx {
            preparations: ctx.preparations,
            tokens: statement.statement.tokens(),
            source: crate::ssa::SsaSourceView::at_statement(ctx.ssa, block, index),
            uses: &statement.uses,
            context: ctx.context,
            config: ctx.config,
            types: ctx.types,
            values: ctx.values,
            registry: ctx.registry,
            known_classes: ctx.known_classes,
            namespace: ctx.namespace,
            ssa: ctx.ssa,
            numbers: ctx.numbers,
            active_writes: &active_writes,
            normal_results: ctx.normal_results,
        };
        let value = match evaluate_type_def(&statement.statement, &write_ctx) {
            DefTyping::Uniform(value) => value,
            DefTyping::PerDef(values) => values
                .get(ctx.ssa.cell_key(contents.reference.symbol))
                .cloned()
                .unwrap_or_else(TypeLattice::overdefined),
        };
        result = type_join(&result, &value);
    }
    if result.kind() == TypeKind::Unknown {
        TypeLattice::overdefined()
    } else {
        result
    }
}

/// Read an empty class declaration through registry manufacturer descriptors.
fn empty_source_class(
    command: &str,
    args: &[String],
    tokens: &crate::ir::CommandTokens,
    registry: &CommandRegistry,
) -> Option<String> {
    let spec = registry.get(command)?;
    if !spec.traits.contains(tcl_registry::Traits::IS_OO_METACLASS)
        || spec.definition_body?.family != tcl_registry::definer::DefinerFamily::TclOo
    {
        return None;
    }
    let method = spec
        .manufacturer_methods
        .iter()
        .find(|method| args.first().is_some_and(|arg| arg == method.keyword))?;
    let body_index = usize::from(method.definition_body_at?);
    let name_index = usize::from(method.names_instance_at?);
    let words = tokens.words();
    let class =
        crate::registry_invocation::invocation_word(words.get(name_index + 1)?).literal()?;
    let body = crate::registry_invocation::invocation_word(words.get(body_index + 1)?).literal()?;
    (body.trim().is_empty() && args.len() == body_index + 1)
        .then(|| crate::naming::normalise_qualified_name(class))
}

fn caller_safe_factory_invocation(
    metadata: TypePropagationMetadata<'_>,
    head: &str,
    args: &[&str],
    classes: &HashSet<String>,
) -> bool {
    let registry = metadata.registry;
    let grammar = &tcl_registry::definer::TCLOO_GRAMMAR;
    if classes.contains(&crate::naming::normalise_qualified_name(head)) {
        return args
            .first()
            .and_then(|word| grammar.manufacturer(word))
            .is_some_and(|method| {
                method.names_instance_at.is_none()
                    && method.visibility == tcl_registry::definer::MemberVisibility::Exported
                    && args.len() == usize::from(method.constructor_args_from)
            });
    }
    let Some(resolved) = tcl_registry::model::resolve_invocation_in_context(
        registry,
        metadata
            .context
            .map(crate::registry_invocation::InvocationMetadataContext::context),
        head,
        args,
    ) else {
        return false;
    };
    !resolved.semantics.traits.intersects(
        tcl_registry::Traits::EVALUATES_CODE
            | tcl_registry::Traits::CREATES_BARRIER
            | tcl_registry::Traits::CREATES_DYNAMIC_BARRIER,
    ) && !resolved.semantics.state_transitions.is_declared()
}

/// Prove the narrow straight-line case of empty source class factories.
/// Any other opaque invocation, binding transition, class body, or control
/// edge withdraws the proof for the whole function.
fn has_only_caller_safe_factories<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    metadata: TypePropagationMetadata<'_>,
    known_classes: &HashSet<String, S>,
) -> bool {
    let registry = metadata.registry;
    if cfg.name != "::top"
        || known_classes.is_empty()
        || !cfg.exception_edges.is_empty()
        || cfg.blocks.iter().any(|(id, block)| {
            *id != cfg.entry && (!block.statements.is_empty() || block.terminator.is_some())
        })
        || !matches!(
            cfg.blocks[&cfg.entry].terminator,
            None | Some(Terminator::Goto { .. })
        )
    {
        return false;
    }
    let mut classes = HashSet::new();
    for stmt in &cfg.blocks[&cfg.entry].statements {
        if let Some(marker) = stmt.synthetic_marker() {
            if !matches!(
                marker,
                crate::ir::SyntheticMarker::RegistryBarrier
                    | crate::ir::SyntheticMarker::GlobalFrameScript
            ) || !cfg.blocks[&cfg.entry]
                .statements
                .iter()
                .any(|host| host.synthetic_marker().is_none() && host.span() == stmt.span())
            {
                return false;
            }
            continue;
        }
        match stmt {
            Statement::Call {
                command,
                args,
                tokens,
                ..
            }
            | Statement::Barrier {
                command,
                args,
                tokens,
                ..
            } => {
                let Some(tokens) = tokens else {
                    return false;
                };
                let words = tokens.words();
                if words
                    .first()
                    .and_then(|word| crate::registry_invocation::invocation_word(word).literal())
                    .is_none()
                {
                    return false;
                }
                let empty_class = empty_source_class(command, args, tokens, registry);
                if let Some(class) = empty_class {
                    if !known_classes.contains(&class) || !classes.insert(class) {
                        return false;
                    }
                    continue;
                }
                if !caller_safe_factory_invocation(
                    metadata,
                    command,
                    &args.iter().map(String::as_str).collect::<Vec<_>>(),
                    &classes,
                ) {
                    return false;
                }
            }
            Statement::AssignConst { .. } | Statement::AssignValue { .. } => {}
            _ => return false,
        }
        if !substitutions_have_only_caller_safe_factories(stmt, metadata, &classes) {
            return false;
        }
    }
    !classes.is_empty()
}

/// Check nested operands without selecting a fresh context or lexer policy.
fn substitutions_have_only_caller_safe_factories(
    stmt: &Statement,
    metadata: TypePropagationMetadata<'_>,
    classes: &HashSet<String>,
) -> bool {
    let registry = metadata.registry;
    let embedded = crate::ir_helpers::evaluated_command_substitutions_with_metadata_context(
        stmt,
        registry,
        metadata.context,
        metadata.config,
    );
    if embedded.opaque {
        return false;
    }
    for words in embedded.all_commands() {
        let Some(head) = words
            .first()
            .and_then(crate::ir_helpers::CommandWord::literal)
        else {
            return false;
        };
        let args: Vec<_> = words
            .iter()
            .skip(1)
            .map(|word| word.literal().unwrap_or("<dynamic>"))
            .collect();
        if !caller_safe_factory_invocation(metadata, head, &args, classes) {
            return false;
        }
    }
    true
}

/// How one statement types the variable(s) it defines.
enum DefTyping {
    /// One lattice applied to every def of the statement.
    Uniform(TypeLattice),
    /// Positional element typing (`lassign`, `foreach` element vars): each
    /// def takes its own lattice; a def absent from the map widens to
    /// `Overdefined`.
    PerDef(VariableCellTable<TypeLattice>),
}

/// Infer the type produced by `stmt` under the current `types` map.
#[must_use]
fn evaluate_type_def<S: std::hash::BuildHasher>(
    stmt: &Statement,
    ctx: &WordTypingCtx<'_, S>,
) -> DefTyping {
    match stmt {
        Statement::AssignConst { value, .. } => {
            DefTyping::Uniform(literal_type(value, ctx.numbers))
        }

        Statement::AssignExpr {
            expr, expr_base, ..
        } => DefTyping::Uniform(lowered_expression_result_type(
            ctx, ctx.tokens, expr, *expr_base,
        )),

        Statement::AssignValue { value, tokens, .. } => DefTyping::Uniform(value_word_type(
            ctx,
            value,
            tokens.as_ref().and_then(|tokens| tokens.words().get(2)),
        )),

        Statement::Incr { .. } => DefTyping::Uniform(TypeLattice::of(TclType::Int)),

        // A physical array-element write can have no scalar SSA definition.
        // Its retained memory write still contributes a value at later reads.
        Statement::Call { .. } => evaluate_call_type_def(stmt, ctx),

        // `ExprEval`, `Barrier`, and structured statements that survive as
        // statements (before CFG construction in some paths) all lack a
        // resolvable result type here — treat them conservatively as
        // overdefined.
        _ => DefTyping::Uniform(TypeLattice::overdefined()),
    }
}

fn evaluate_call_type_def<S: std::hash::BuildHasher>(
    stmt: &Statement,
    ctx: &WordTypingCtx<'_, S>,
) -> DefTyping {
    let Statement::Call {
        command,
        canonical_command,
        args,
        defs,
        ..
    } = stmt
    else {
        return DefTyping::Uniform(TypeLattice::overdefined());
    };
    if stmt.tokens().is_some_and(|tokens| {
        matches!(
            tokens.synthetic,
            Some(crate::ir::SyntheticMarker::IterationBindings(_))
        )
    }) {
        let Statement::Call { foreach_groups, .. } = stmt else {
            unreachable!()
        };
        let arguments = args.iter().map(String::as_str).collect::<Vec<_>>();
        return elements_of_def_typing(ctx, &arguments, defs, foreach_groups.as_deref(), 0);
    }
    if let Some(tokens) = stmt.tokens()
        && let Some(binding) = tokens.source_binding.as_ref()
        && let Some(normal) =
            crate::registry_invocation::normal_transfer_invocation_with_metadata_context(
                ctx.registry,
                ctx.context,
                tokens,
            )
    {
        if let Some((value, word)) =
            normal.stored_value_operand(&binding.variable_context, ctx.registry)
        {
            return DefTyping::Uniform(value_word_type(ctx, value, Some(word)));
        }
        if let Some(write) = normal.container_element_write(&binding.variable_context, ctx.registry)
        {
            let args = write
                .arguments
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            return DefTyping::Uniform(var_elements_effect_lattice(
                ctx,
                write.effect,
                &write.target,
                &args,
                write.argument_offset,
                Some(write.words),
            ));
        }
    }
    // Resolve the source spelling through the lowerer's
    // `canonical_command` snapshot (an `interp alias` / `rename`
    // target) so a renamed or aliased builtin — `rename set myset` /
    // `interp alias {} myset {} set` — is typed by the *real* command's
    // registry spec, not left as an unknown `Call` (OVERDEFINED).
    let proven = stmt.tokens().map(|_| {
        crate::registry_invocation::resolved_statement_invocation_with_metadata_context(
            ctx.registry,
            ctx.context,
            stmt,
        )
    });
    if matches!(proven, Some(None)) {
        return DefTyping::Uniform(TypeLattice::overdefined());
    }
    let proven = proven.flatten();
    let canon = proven.as_ref().map_or_else(
        || canonical_command.as_deref().unwrap_or(command),
        |invocation| invocation.facts.canonical_command.as_str(),
    );
    let presented;
    let args = if let Some(invocation) = &proven {
        let Some(arguments) = invocation
            .arguments
            .iter()
            .cloned()
            .collect::<Option<Vec<_>>>()
        else {
            return DefTyping::Uniform(TypeLattice::overdefined());
        };
        presented = arguments;
        presented.as_slice()
    } else {
        args.as_slice()
    };
    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
    // A canonically-`set` store keeps its runtime `Call` shape for
    // codegen — an aliased / renamed command is not inline-foldable,
    // its binding may change by call time — but for the type lattice
    // its stored cell takes the *value word's* intrep verbatim, exactly
    // as the un-aliased [`Statement::AssignValue`] path does. Keyed off
    // the canonical command's `Set` lowering hook (the registry's own
    // "this is a value-passthrough store" fact), never the source
    // spelling. Effective argv fixes the setter value position, including
    // composed aliases. A dynamic array index has a physical store without
    // a scalar definition, so scalar def cardinality cannot gate this fact.
    if arg_refs.len() == 2
        && proven.as_ref().map_or_else(
            || {
                ctx.registry.get(canon).and_then(|spec| spec.lowering_hook)
                    == Some(tcl_registry::hooks::LoweringHookId::Set)
            },
            |invocation| {
                invocation.facts.operation
                    == tcl_registry::SemanticOperationId::StructuredLowering(
                        tcl_registry::hooks::LoweringHookId::Set,
                    )
            },
        )
    {
        return DefTyping::Uniform(value_word_type(
            ctx,
            arg_refs[1],
            proven
                .as_ref()
                .and_then(|invocation| invocation.effective.words.get(2)),
        ));
    }

    written_call_type(stmt, ctx, canon, &arg_refs, proven.as_ref())
}

fn written_call_type<S: std::hash::BuildHasher>(
    stmt: &Statement,
    ctx: &WordTypingCtx<'_, S>,
    canon: &str,
    arg_refs: &[&str],
    proven: Option<&crate::registry_invocation::ResolvedStatementInvocation>,
) -> DefTyping {
    let Statement::Call {
        defs,
        foreach_groups,
        ..
    } = stmt
    else {
        return DefTyping::Uniform(TypeLattice::overdefined());
    };
    let resolved = tcl_registry::model::resolve_invocation_in_context(
        ctx.registry,
        ctx.context
            .map(crate::registry_invocation::InvocationMetadataContext::context),
        canon,
        arg_refs,
    );

    // An in-place element write (`lappend VAR v…`, `dict set VAR … v`)
    // evolves the target's container elements — the registry's
    // `VarElementsEffect` fact, generalising the old object-only
    // element-class harvesting to every element shape.
    if let Some(effect) = proven.map_or_else(
        || {
            resolved
                .as_ref()
                .and_then(|resolved| resolved.semantics.var_elements_effect)
        },
        |invocation| invocation.facts.var_elements_effect,
    ) {
        let base = proven.map_or_else(
            || {
                usize::from(
                    resolved
                        .as_ref()
                        .is_some_and(|resolved| resolved.subcommand.is_resolved()),
                )
            },
            |invocation| invocation.facts.argument_offset,
        );
        if let Some(target) = arg_refs.get(base) {
            return DefTyping::Uniform(var_elements_effect_lattice(
                ctx,
                effect,
                target,
                arg_refs,
                base,
                proven.and_then(|invocation| invocation.effective.words.get(1..)),
            ));
        }
    }

    // How a command types the variable(s) it *writes* is a distinct
    // fact from the value it *returns*.  A destructuring writer
    // (`scan`, `regexp`, `binary scan`) returns a match/convert count
    // while writing element-wise pieces; `gets` returns the character
    // count while writing a text line; `lassign` writes its
    // container's *elements* positionally.  The registry declares this
    // per command / subcommand (`VarWriteTyping`), so the compiler
    // never keys on the command name.  Resolved through `canon`, not
    // the source spelling, so an aliased / renamed destructuring
    // writer (`rename lassign mylassign`) still resolves to the real
    // command's `VarWriteTyping` (FP-SH-15).
    let typing = proven.map_or_else(
        || {
            resolved
                .as_ref()
                .map_or(VarWriteTyping::ReturnValue, |resolved| {
                    resolved.semantics.var_write_typing
                })
        },
        |invocation| invocation.facts.var_write_typing,
    );
    match typing {
        // The default typing stores the command's *return value* in the
        // target — meaningful only for a single-target writer (`append`,
        // `lappend`). A call that writes *several* variables under the
        // default (no override) is not a single-value writer: the
        // synthetic `catch {body} resultVar optionsVar` / `try …` calls
        // carry the body's writes plus the result / options vars as defs
        // while `catch` / `try` return an Int status code, none of which
        // is that status. Broadcasting the return type onto all of them
        // would mistype every such variable, so stay conservative — the
        // old `defs.len() > 1` fallback, now scoped to the default arm
        // rather than a blanket heuristic (a registry `Destructured` /
        // `Fixed` override still applies at any def count).
        VarWriteTyping::ReturnValue if defs.len() > 1 => {
            DefTyping::Uniform(TypeLattice::overdefined())
        }
        VarWriteTyping::ReturnValue => DefTyping::Uniform(proven.map_or_else(
            || {
                return_type_for_command(
                    ctx.registry,
                    canon,
                    arg_refs,
                    ctx.known_classes,
                    ctx.namespace,
                )
            },
            |invocation| {
                invocation
                    .facts
                    .return_type
                    .map_or_else(TypeLattice::overdefined, TypeLattice::of)
            },
        )),
        VarWriteTyping::Fixed(t) => DefTyping::Uniform(TypeLattice::of(t)),
        VarWriteTyping::Destructured => DefTyping::Uniform(TypeLattice::overdefined()),
        VarWriteTyping::ElementsOf { container_arg } => elements_of_def_typing(
            ctx,
            arg_refs,
            defs,
            foreach_groups.as_deref(),
            container_arg,
        ),
    }
}

/// Positional element typing for a [`VarWriteTyping::ElementsOf`] writer.
///
/// Two shapes reach here:
/// - **`lassign $l a b …`** (no `foreach_groups`): target `i` takes element
///   `i` of the container. A tracked `Exact` container types each target
///   from its position — a position past the container's arity is the empty
///   string `lassign` pads with (`String`), an untracked position widens to
///   `Overdefined`.
/// - **`foreach` / `lmap` / `dict for` headers** (`foreach_groups`
///   present): `defs` is the flattened per-group element vars and `args`
///   holds one container word per group. A single-var list group's element
///   var is typed `String` (list elements stringify — the established
///   convention the shimmer suite pins) unless the container tracks a
///   uniform element shape; multi-var groups type var `j` from the join of
///   the `Exact` positions congruent to `j` mod the group size, else
///   `Overdefined` (the old multi-def behaviour). A dict group types its
///   value var from the dict's tracked value shape when one exists.
fn elements_of_def_typing<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    args: &[&str],
    defs: &[String],
    foreach_groups: Option<&[usize]>,
    container_arg: u8,
) -> DefTyping {
    let mut per_def = VariableCellTable::default();

    if let Some(groups) = foreach_groups {
        let mut def_cursor = 0usize;
        for (group, &nvars) in groups.iter().enumerate() {
            let group_defs = defs.get(def_cursor..def_cursor + nvars).unwrap_or(&[]);
            def_cursor += nvars;
            let container_shape = args.get(group).and_then(|w| container_word_shape(ctx, w));
            for (j, def) in group_defs.iter().enumerate() {
                per_def.insert(
                    ctx.variable_key(def).unwrap_or_else(|| def.clone().into()),
                    foreach_var_lattice(container_shape.as_ref(), nvars, j),
                );
            }
        }
        return DefTyping::PerDef(per_def);
    }

    // lassign: targets follow the container word.
    let container = args.get(usize::from(container_arg));
    let elements = container
        .filter(|w| ctx.is_variable_word(w))
        .and_then(|w| ctx.written_variable_type(w, None))
        .and_then(|t| t.elements().cloned());
    for (i, def) in defs.iter().enumerate() {
        let lattice = match &elements {
            Some(Elements::Exact(shapes)) => {
                if i >= shapes.len() {
                    // Past the container's arity: `lassign` pads with the
                    // empty string.
                    TypeLattice::of(TclType::String)
                } else {
                    shapes[i]
                        .clone()
                        .map_or_else(TypeLattice::overdefined, TypeLattice::of_shape)
                }
            }
            _ => TypeLattice::overdefined(),
        };
        per_def.insert(
            ctx.variable_key(def).unwrap_or_else(|| def.clone().into()),
            lattice,
        );
    }
    DefTyping::PerDef(per_def)
}

/// The container shape a `foreach` group's list word denotes, when tracked.
fn container_word_shape<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    word: &str,
) -> Option<TypeShape> {
    let word = word.trim();
    if ctx.is_variable_word(word) {
        return ctx
            .written_variable_type(word, None)
            .and_then(|t| t.single_shape().cloned());
    }
    if word.starts_with('[') && word.ends_with(']') {
        return value_word_type(ctx, word, None).single_shape().cloned();
    }
    None
}

/// The lattice for one `foreach` element variable — var `j` of an
/// `nvars`-wide group iterating `container_shape`.
fn foreach_var_lattice(container_shape: Option<&TypeShape>, nvars: usize, j: usize) -> TypeLattice {
    match container_shape {
        Some(TypeShape::List(elements)) => {
            if nvars == 1 {
                match elements.uniform_shape() {
                    Some(u) => TypeLattice::of_shape(u),
                    // List elements stringify — the established convention.
                    None => TypeLattice::of(TclType::String),
                }
            } else if let Elements::Exact(shapes) = elements {
                // Var `j` sees positions j, j+nvars, j+2·nvars, …
                let mut acc: Option<TypeShape> = None;
                for shape in shapes.iter().skip(j).step_by(nvars) {
                    let Some(shape) = shape else {
                        return TypeLattice::overdefined();
                    };
                    acc = Some(match acc {
                        None => shape.clone(),
                        Some(prev) => match shape_join(&prev, shape) {
                            Some(joined) => joined,
                            None => return TypeLattice::overdefined(),
                        },
                    });
                }
                acc.map_or_else(
                    // No positions at this stride: the var gets the empty
                    // string `foreach` pads with.
                    || TypeLattice::of(TclType::String),
                    TypeLattice::of_shape,
                )
            } else {
                TypeLattice::overdefined()
            }
        }
        // A dict group (`dict for {k v} $d`): the key stringifies, the value
        // takes the dict's tracked value shape. Without a tracked value
        // shape both stay conservative (the old multi-def behaviour).
        Some(TypeShape::Dict(elements)) => {
            if nvars == 2 && j == 1 {
                elements
                    .uniform_shape()
                    .map_or_else(TypeLattice::overdefined, TypeLattice::of_shape)
            } else if nvars == 2 && j == 0 && elements.uniform_shape().is_some() {
                TypeLattice::of(TclType::String)
            } else {
                TypeLattice::overdefined()
            }
        }
        // Untracked container: a single-var group keeps the established
        // `String` element convention; wider groups stay conservative.
        _ => {
            if nvars == 1 {
                TypeLattice::of(TclType::String)
            } else {
                TypeLattice::overdefined()
            }
        }
    }
}

/// Run type propagation with an explicitly supplied standalone command store.
/// Actual function/unit consumers use [`propagate_types_with_metadata_context`]
/// with their retained source metadata and configuration.
///
/// Returns a map from `(variable_name, ssa_version)` to inferred
/// `TypeLattice`. Values absent from the map are implicitly `Unknown`.
///
/// Every def of a name [`crate::ssa::SsaSourceView::externally_mutable_by`] considers
/// externally mutable — fully namespace-qualified, aliased via `global`/
/// `variable`/`upvar`/traced *within this function* ([`crate::var_observability::
/// analyse_var_observability`]), named by `extra_global_escaping` (the
/// whole-module `global`-declaration scan for the *top-level* unit — see
/// [`crate::var_observability::scan_module_global_names`]), or traced
/// *anywhere in the module* (`trace_facts`) — is forced `Overdefined` here,
/// unless the source owner retains the exact successful store's value in the
/// same physical cell after callbacks. That receipt types this definition;
/// later reads still require their own reached contents proof.
/// reusing the exact predicate [`crate::sccp::sccp_with_extra_escaping`] and
/// [`crate::optimiser::propagation`]'s O102 load-forwarding already apply to
/// their own (separate) lattices, rather than re-deriving a third,
/// potentially-divergent notion of "externally mutable" for this one. A
/// `set`-only view of such a name's literal types is not sound: a callee's
/// `global NAME; set NAME …`, a top-level name no procedure declares
/// `global` itself but another's `global NAME` can still reach, or a write
/// trace's callback, can all change what the name actually holds — reporting
/// a shimmer purely from the visible literals could be a false positive (or
/// miss the real one). `extra_global_escaping` is empty and `trace_facts` is
/// [`crate::compilation_unit::ModuleTraceFacts::none()`] for the
/// module-context-free callers ([`Self`] unit tests, isolated single-function
/// rebuilds with no module to scan).
#[must_use]
pub fn propagate_types<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    sccp: &SccpResult,
    registry: &CommandRegistry,
    known_classes: &HashSet<String, S>,
    extra_global_escaping: &HashSet<String, S>,
    trace_facts: crate::compilation_unit::ModuleTraceFacts<'_>,
) -> HashMap<ValueKey, TypeLattice> {
    let context = registry
        .profile()
        .map(tcl_registry::model::semantic::SemanticContext::for_profile)
        .map(Into::into);
    propagate_types_with_metadata_context(
        cfg,
        ssa,
        sccp,
        TypePropagationMetadata {
            registry,
            context,
            config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
            numbers: numbers_of(registry),
        },
        known_classes,
        extra_global_escaping,
        trace_facts,
    )
}

/// Exact source metadata and lexical/numeric grammar used by the type solver.
#[derive(Clone, Copy)]
pub struct TypePropagationMetadata<'a> {
    /// Actual command store used by this graph.
    pub registry: &'a CommandRegistry,
    /// Supplied complete availability, independent of an executing interpreter.
    pub context: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    /// Exact original source lexical and list policy.
    pub config: tcl_lexer::LexerConfig,
    /// Exact source numeric grammar.
    pub numbers: NumberSyntax,
}

impl<'a> TypePropagationMetadata<'a> {
    /// Borrow all typing premises from one genuine function input.
    /// Missing input, a foreign store or inconsistent source policy declines.
    #[must_use]
    pub fn for_function(
        function: &'a crate::compilation_unit::FunctionUnit,
        registry: &'a CommandRegistry,
    ) -> Option<Self> {
        let input = function.source_metadata_input()?;
        let context = function.invocation_metadata_context(registry)?;
        let config = function.source_lexer_config();
        if config.normalized() != input.lexer_config().normalized() {
            return None;
        }
        Some(Self {
            registry,
            context: Some(context),
            config,
            numbers: input.unit_profile().grammar.numbers,
        })
    }
}

/// Infer source values under retained metadata without recreating availability.
#[must_use]
pub fn propagate_types_with_metadata_context<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    sccp: &SccpResult,
    metadata: TypePropagationMetadata<'_>,
    known_classes: &HashSet<String, S>,
    extra_global_escaping: &HashSet<String, S>,
    trace_facts: crate::compilation_unit::ModuleTraceFacts<'_>,
) -> HashMap<ValueKey, TypeLattice> {
    let registry = metadata.registry;
    let Some(context) = metadata
        .context
        .filter(|context| context.matches_registry(registry))
    else {
        return HashMap::new();
    };
    let preds = cfg.predecessors();
    let order = crate::sccp::cfg_order(cfg);
    let mut escaping = crate::var_observability::analyse_var_observability_with_metadata_context(
        cfg,
        registry,
        Some(context),
    )
    .escaping_var_names();
    if !extra_global_escaping.is_empty() {
        escaping.extend(extra_global_escaping.iter().cloned());
    }
    escaping.extend(trace_facts.traced_variables.iter().cloned());
    // Constructor heads written `[Foo new]` inside this function resolve
    // relative names against the function's own namespace.
    let namespace = function_namespace(&cfg.name);
    // Every nested query borrows the same complete source availability and
    // lexical policy; neither is reconstructed from the registry profile.
    let ctx = StatementTypingCtx {
        preparations: &cfg.expression_preparations,
        ssa,
        context: Some(context),
        config: metadata.config,
        registry,
        known_classes,
        namespace: &namespace,
        values: &sccp.values,
        escaping: &escaping,
        has_dynamic_variable_trace: trace_facts.has_dynamic_variable_trace,
        numbers: metadata.numbers,
    };

    let mut types: HashMap<ValueKey, TypeLattice> = HashMap::new();
    let caller_safe_factories = has_only_caller_safe_factories(cfg, metadata, known_classes);
    for (block, markers) in &ssa.value_clobbers {
        if sccp.executable_blocks.contains(block) {
            for versions in markers.values() {
                for (&symbol, &(_, fresh)) in versions {
                    if !caller_safe_factories {
                        types.insert((symbol, fresh), TypeLattice::overdefined());
                    }
                }
            }
        }
    }
    let mut changed = true;
    while changed {
        changed = false;
        for bn in &order {
            if !sccp.executable_blocks.contains(bn) {
                continue;
            }
            let Some(ssa_block) = ssa.blocks.get(bn) else {
                continue;
            };

            // Phi nodes at non-entry blocks.
            if *bn != cfg.entry {
                let mut exec_preds: Vec<BlockId> = preds
                    .get(bn)
                    .map(|ps| {
                        ps.iter()
                            .filter(|p| sccp.executable_edges.contains(&(**p, *bn)))
                            .copied()
                            .collect()
                    })
                    .unwrap_or_default();
                // `predecessors()` yields a `HashSet`, so the fold order below is
                // nondeterministic — and `type_join` is *not* order-independent
                // for a 3+-way shimmer merge (it records only a `(from, to)`
                // pair), so an unsorted fold makes the S101 message name
                // different types run-to-run.  Sort for a stable join order.
                exec_preds.sort_unstable();

                for phi in &ssa_block.phis {
                    if exec_preds.is_empty() {
                        continue;
                    }
                    let mut phi_type = TypeLattice::unknown();
                    for pred in &exec_preds {
                        let ver = type_version(
                            ssa,
                            phi.name,
                            phi.incoming.get(pred).copied().unwrap_or(0),
                            &types,
                        );
                        // A version-0 incoming is the entry / live-in root (a
                        // proc parameter, global, or other caller-supplied
                        // value). Its runtime type is unknown at compile time,
                        // so it joins in as OVERDEFINED — never skipped.
                        // Skipping it would let a phi that merges a live-in
                        // with a defined-arm type collapse to that arm's type,
                        // so a conditionally-assigned parameter
                        // (`proc p {c x} { if {$c} { set x 5 } … $x }`) would
                        // be typed solely from the assigned arm and the S101 /
                        // W307 / W308 consumers would report facts false for
                        // the not-taken path. Mirrors `sccp_process_phis`.
                        let t = if ver == 0 {
                            TypeLattice::overdefined()
                        } else {
                            types
                                .get(&(phi.name, ver))
                                .cloned()
                                .unwrap_or_else(TypeLattice::unknown)
                        };
                        phi_type = type_join(&phi_type, &t);
                    }
                    let key = (phi.name, phi.version);
                    let old = types
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(TypeLattice::unknown);
                    let merged = type_join(&old, &phi_type);
                    if merged != old {
                        types.insert(key, merged);
                        changed = true;
                    }
                }
            }

            // Statements.
            if type_infer_process_statements(&mut types, *bn, ssa_block, &ctx) {
                changed = true;
            }
        }
    }

    types
}

/// Shared, read-only context for [`type_infer_process_statements`].
struct StatementTypingCtx<'a, S: std::hash::BuildHasher> {
    preparations: &'a [crate::command_binding::SourceExpressionPreparation],
    ssa: &'a SsaFunction,
    /// Same retained metadata as each original word query.
    context: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
    config: tcl_lexer::LexerConfig,
    registry: &'a CommandRegistry,
    known_classes: &'a HashSet<String, S>,
    namespace: &'a str,
    /// SCCP constants — purity evidence and constant list/index values for
    /// the element-inference helpers (see [`WordTypingCtx::values`]).
    values: &'a HashMap<ValueKey, LatticeValue>,
    /// Names [`crate::ssa::SsaSourceView::externally_mutable_by`] should treat as
    /// unconditionally aliased/escaping (per-function `analyse_var_observability`
    /// union'd with the caller's whole-module `extra_global_escaping` and
    /// `trace_facts.traced_variables`).
    escaping: &'a HashSet<String>,
    has_dynamic_variable_trace: bool,
    /// The actual source's numeric-literal grammar.
    numbers: NumberSyntax,
}

/// Evaluate each statement's defs for one block, forcing every def of a name
/// [`crate::ssa::SsaSourceView::externally_mutable_by`] considers externally mutable to
/// `Overdefined` unless its exact normal-store contents receipt survives.
/// Returns `true` if any lattice value changed. Extracted
/// from [`propagate_types`], mirroring [`crate::sccp::sccp_process_statements`]'s
/// shape for the (separate) constant-folding lattice.
fn type_infer_process_statements<S: std::hash::BuildHasher>(
    types: &mut HashMap<ValueKey, TypeLattice>,
    block: BlockId,
    ssa_block: &crate::ssa::SsaBlock,
    ctx: &StatementTypingCtx<'_, S>,
) -> bool {
    let mut changed = false;
    for (index, ssa_stmt) in ssa_block.statements.iter().enumerate() {
        let stmt = &ssa_stmt.statement;
        let source = crate::ssa::SsaSourceView::at_statement(ctx.ssa, block, index);
        // A barrier widens every def to OVERDEFINED (it may have mutated
        // them arbitrarily); a scope-alias declaration (`global`/`variable`/
        // `upvar`/`namespace upvar`) likewise widens its defs — the
        // imported variable's intrep is external and unknown.
        let inferred = match stmt {
            Statement::Barrier { .. } | Statement::NativeCall { .. } => {
                DefTyping::Uniform(TypeLattice::overdefined())
            }
            Statement::Call {
                command,
                canonical_command,
                args,
                defs,
                ..
            } if !defs.is_empty()
                && is_scope_alias_call(
                    ctx.registry,
                    canonical_command.as_deref().unwrap_or(command),
                    args,
                ) =>
            {
                DefTyping::Uniform(TypeLattice::overdefined())
            }
            _ => {
                let word_ctx = WordTypingCtx {
                    preparations: ctx.preparations,
                    tokens: stmt.tokens(),
                    source,
                    uses: &ssa_stmt.uses,
                    context: ctx.context,
                    config: ctx.config,
                    types,
                    values: ctx.values,
                    registry: ctx.registry,
                    known_classes: ctx.known_classes,
                    namespace: ctx.namespace,
                    ssa: ctx.ssa,
                    numbers: ctx.numbers,
                    active_writes: &[],
                    normal_results: None,
                };
                evaluate_type_def(stmt, &word_ctx)
            }
        };
        for (&var, &ver) in &ssa_stmt.defs {
            let key = (var, ver);
            let old = types
                .get(&key)
                .cloned()
                .unwrap_or_else(TypeLattice::unknown);
            let cell = ctx.ssa.cell_key(var);
            let externally_mutable = source.externally_mutable_by(
                var,
                ctx.escaping,
                ctx.has_dynamic_variable_trace,
                ctx.registry,
            ) != Some(false);
            let def_type = if ssa_stmt.destruction_defs.contains(&var)
                || (externally_mutable
                    && !source.normal_store_contents_preserved(var, ctx.registry))
                || ctx.ssa.is_array_root_refresh_version(var, ver)
            {
                TypeLattice::overdefined()
            } else if ssa_stmt.may_defs.contains(&var) {
                // A synthetic may-def: the write may or may not have hit
                // this element, so its type is the JOIN of the prior
                // version's type (recorded as a use) and the written type
                // — `set arr($k) 9` over an INT `arr(a)` stays INT, over a
                // STRING one widens. No prior use (a base refresh from a
                // Call def) → Overdefined.
                match ssa_stmt.uses.get(&var) {
                    Some(prev_ver) => {
                        let prev = types
                            .get(&(var, *prev_ver))
                            .cloned()
                            .unwrap_or_else(TypeLattice::overdefined);
                        let written = match &inferred {
                            DefTyping::Uniform(t) => t.clone(),
                            DefTyping::PerDef(map) => map
                                .get(cell)
                                .cloned()
                                .unwrap_or_else(TypeLattice::overdefined),
                        };
                        type_join(&prev, &written)
                    }
                    None => TypeLattice::overdefined(),
                }
            } else {
                match &inferred {
                    DefTyping::Uniform(t) => t.clone(),
                    // Positional element typing: a def the map does not
                    // name widens to Overdefined.
                    DefTyping::PerDef(map) => map
                        .get(cell)
                        .cloned()
                        .unwrap_or_else(TypeLattice::overdefined),
                }
            };
            let merged = type_join(&old, &def_type);
            if merged != old {
                types.insert(key, merged);
                changed = true;
            }
        }
    }
    changed
}

/// Infer a function's overall return type by joining the result types
/// of every executable exit — explicit `Return` terminators *and*
/// fall-through exits.
///
/// `types` is the [`propagate_types`] result for the same function.
/// Value returns use the terminator's reaching SSA versions and retained
/// source invocation context. Expression-only returns currently join each
/// name's known versions conservatively.
///
/// A reachable block with no terminator is a *fall-through* exit:
/// control runs off the end of the body and Tcl returns the result of
/// the last command executed (e.g. the empty string of an
/// else-less `if`, or `set`'s value).  That result is not modelled
/// here, so a fall-through contributes `Overdefined` to the join
/// rather than being skipped — without this, a partial-return proc
/// like `if {$c} { return 1 }` would report an overconfident `Int`.
/// Returns `Unknown` only when the function has no executable exit at
/// all.
#[must_use]
pub(crate) fn infer_function_return_type<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    sccp: &SccpResult,
    types: &HashMap<ValueKey, TypeLattice>,
    metadata: TypePropagationMetadata<'_>,
    known_classes: &HashSet<String, S>,
    ssa: &SsaFunction,
) -> TypeLattice {
    infer_function_return_type_with_results(cfg, sccp, types, metadata, known_classes, ssa, None)
}

/// Conditional normal result inference with independently retained callee body
/// types. Only the actual positioned handler allocation can select a result.
pub(crate) fn infer_function_return_type_with_results<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    sccp: &SccpResult,
    types: &HashMap<ValueKey, TypeLattice>,
    metadata: TypePropagationMetadata<'_>,
    known_classes: &HashSet<String, S>,
    ssa: &SsaFunction,
    normal_results: Option<&NormalProcedureResultTypes>,
) -> TypeLattice {
    let registry = metadata.registry;
    let Some(context) = metadata
        .context
        .filter(|context| context.matches_registry(registry))
    else {
        return TypeLattice::unknown();
    };
    let namespace = function_namespace(&cfg.name);
    let numbers = metadata.numbers;
    let mut result: Option<TypeLattice> = None;
    for (bn, block) in &cfg.blocks {
        if !sccp.executable_blocks.contains(bn) {
            continue;
        }
        let t = match &block.terminator {
            Some(Terminator::Return {
                value,
                value_word,
                tokens,
                expr,
                expr_base,
                ..
            }) => {
                let Some(ssa_block) = ssa.blocks.get(bn) else {
                    continue;
                };
                let context = WordTypingCtx {
                    preparations: &cfg.expression_preparations,
                    tokens: tokens.as_deref(),
                    source: crate::ssa::SsaSourceView::at_terminator(ssa, *bn),
                    uses: &ssa_block.exit_versions,
                    context: Some(context),
                    config: metadata.config,
                    types,
                    values: &sccp.values,
                    registry,
                    known_classes,
                    namespace: &namespace,
                    ssa,
                    numbers,
                    active_writes: &[],
                    normal_results,
                };
                if let Some(expr) = expr {
                    lowered_expression_result_type(&context, tokens.as_deref(), expr, *expr_base)
                } else if let Some(value) = value {
                    value_word_type(&context, value, value_word.as_ref())
                } else {
                    // Bare `return` yields the empty string.
                    TypeLattice::of(TclType::String)
                }
            }
            // Fall-through exit (last-command result, not modelled).
            None => TypeLattice::overdefined(),
            // `Goto` / `Branch` have successors — not exits.
            Some(_) => continue,
        };
        result = Some(match result {
            Some(acc) => type_join(&acc, &t),
            None => t,
        });
    }

    result.unwrap_or_else(TypeLattice::unknown)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::{Block, Function};
    use crate::ir::Statement;
    use crate::ssa::{Phi, SsaBlock, SsaFunction, SsaStatement};
    use std::collections::HashSet;
    use tcl_lexer::Span;

    #[test]
    fn original_type_metadata_retains_actual_availability_and_independent_source_grammar() {
        // naming.compiler.retained-representation-metadata
        // docs/design/analysis/name-resolution-proofs/retained-representation-metadata.md
        use crate::compilation_unit::{CompilationUnit, UnitBuildOptions};
        use std::sync::Arc;

        let base =
            tcl_registry::model::ingress::resolve_environment("tcl8.4").default_context_registry();
        let profile = tcl_dialect::DialectProfile::plain_tcl();
        let mut commands = base
            .commands()
            .project_for_profile(base.commands().profile().unwrap());
        commands.insert_ambient_package("example", "1.0");
        let actual = Arc::new(base.with_command_store(Arc::new(commands)));
        let registry = actual.commands();
        let config = tcl_lexer::LexerConfig {
            braced_var: tcl_dialect::BracedVarStyle::FirstClose,
            list_parse: tcl_dialect::ListParse::Lenient,
            ..tcl_lexer::LexerConfig::for_profile(Some(profile))
        };
        let unit = CompilationUnit::build_with_context_registry(
            "proc f {lst} {return [llength $lst]}",
            UnitBuildOptions {
                registry,
                dialect: Some(profile),
                defer_top_level: false,
                config,
                declared_commands: None,
                external_call_sites: None,
            },
            None,
            Arc::clone(&actual),
        );
        let function = unit.function("::f").unwrap();
        let metadata = TypePropagationMetadata::for_function(function, registry).unwrap();
        assert!(core::ptr::eq(
            metadata.context.unwrap().context(),
            actual.context()
        ));
        assert!(
            metadata
                .context
                .unwrap()
                .context()
                .ambient_package("example")
        );
        assert_eq!(metadata.config, config);
        assert_eq!(metadata.numbers, profile.grammar.numbers);
        assert_ne!(metadata.numbers, registry.numbers());
        assert!(
            tcl_syntax::word_rules::WordValueRules::from_config(&metadata.config)
                .split_list("{a")
                .is_ok()
        );
        assert!(
            tcl_syntax::word_rules::WordValueRules::TCL
                .split_list("{a")
                .is_err()
        );
        assert_eq!(
            tcl_lexer::word_parts::whole_var_ref(b"${a{b}", metadata.config)
                .unwrap()
                .unwrap()
                .name,
            b"a{b",
        );
        assert!(
            tcl_lexer::word_parts::whole_var_ref(
                b"${a{b}",
                tcl_lexer::LexerConfig::for_profile(Some(profile)),
            )
            .is_err()
        );
        let mut stale = function.clone();
        stale.source_config.braced_var = tcl_dialect::BracedVarStyle::Tcl9Nesting;
        assert!(TypePropagationMetadata::for_function(&stale, registry).is_none());
    }

    #[test]
    fn original_type_consumers_keep_positive_normal_results_and_refuse_missing_foreign_metadata() {
        // naming.compiler.retained-representation-metadata
        // docs/design/analysis/name-resolution-proofs/retained-representation-metadata.md
        use crate::compilation_unit::CompilationUnit;

        let registry = registry();
        let unit = CompilationUnit::build_for(
            "proc f {lst} {set x [llength $lst]; return $x}",
            &registry,
            false,
        );
        let function = unit.function("::f").unwrap();
        let classes: HashSet<String> = HashSet::new();
        let escaping: HashSet<String> = HashSet::new();
        let metadata = TypePropagationMetadata::for_function(function, &registry).unwrap();
        let infer = |metadata| {
            infer_function_return_type(
                &function.cfg,
                &function.sccp,
                &function.types,
                metadata,
                &classes,
                &function.ssa,
            )
        };
        let propagate = |metadata| {
            propagate_types_with_metadata_context(
                &function.cfg,
                &function.ssa,
                &function.sccp,
                metadata,
                &classes,
                &escaping,
                crate::compilation_unit::ModuleTraceFacts::none(),
            )
        };
        assert_eq!(infer(metadata).tcl_type(), Some(TclType::Int));
        assert!(!propagate(metadata).is_empty());
        let missing = TypePropagationMetadata {
            context: None,
            ..metadata
        };
        assert_eq!(infer(missing).kind(), TypeKind::Unknown);
        assert!(propagate(missing).is_empty());
        let mut foreign = tcl_registry::CommandRegistry::build_default();
        let mut changed = foreign.get("llength").unwrap().clone();
        changed.return_type = Some(TclType::String);
        foreign.insert(changed);
        let foreign_metadata = TypePropagationMetadata {
            registry: &foreign,
            ..metadata
        };
        assert_eq!(infer(foreign_metadata).kind(), TypeKind::Unknown);
        assert!(propagate(foreign_metadata).is_empty());
        assert!(TypePropagationMetadata::for_function(function, &foreign).is_none());
        let mut absent = function.clone();
        absent.source_metadata_input = None;
        assert!(TypePropagationMetadata::for_function(&absent, &registry).is_none());
        let replacement = CompilationUnit::build_for(
            "proc llength {lst} {return other}; proc f {lst} {set x [llength $lst]; return $x}",
            &registry,
            false,
        );
        let replaced = replacement.function("::f").unwrap();
        let replacement_metadata =
            TypePropagationMetadata::for_function(replaced, &registry).unwrap();
        let replacement_result = infer_function_return_type(
            &replaced.cfg,
            &replaced.sccp,
            &replaced.types,
            replacement_metadata,
            &classes,
            &replaced.ssa,
        );
        assert_ne!(replacement_result.tcl_type(), Some(TclType::Int));
    }

    #[test]
    fn positional_type_facts_retain_native_cell_keys_without_display_lookup() {
        use crate::command_binding::SourceNamespaceKey;
        use crate::var_resolve::VariableProofRelocation;
        use tcl_runtime_api::native_compilation::{
            NativeInterpreterIdentity, NativeNamespaceContext,
        };

        let registry = registry();
        let interpreter = NativeInterpreterIdentity {
            owner: NativeInterpreterIdentity::fresh_owner(),
            interpreter: 0,
        };
        let key = |token| VariableCellKey::Namespace {
            identity: SourceNamespaceKey::Native(NativeNamespaceContext {
                interpreter,
                token,
                path: tcl_core_types::ByteNamespacePath::from_segments(["N"]),
            }),
            simple: "x".into(),
        };
        let original = key(1);
        let replacement = key(2);
        let mut ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
        ssa.intern_var("x");
        ssa.relocate_variable_proofs(&VariableProofRelocation {
            storage_keys: HashMap::from([("x".into(), original.clone())]),
            ..Default::default()
        });
        let uses = HashMap::new();
        let types = HashMap::new();
        let values = HashMap::new();
        let known_classes: HashSet<String> = HashSet::new();
        let context = WordTypingCtx {
            preparations: &[],
            tokens: None,
            source: crate::ssa::SsaSourceView::unpositioned(&ssa),
            uses: &uses,
            context: registry
                .profile()
                .map(tcl_registry::model::semantic::SemanticContext::for_profile)
                .map(Into::into),
            config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
            types: &types,
            values: &values,
            registry: &registry,
            known_classes: &known_classes,
            namespace: "::",
            ssa: &ssa,
            numbers: numbers_of(&registry),
            active_writes: &[],
            normal_results: None,
        };
        let DefTyping::PerDef(facts) =
            elements_of_def_typing(&context, &["item"], &["x".into()], Some(&[1]), 0)
        else {
            panic!("positional definition types");
        };
        assert_eq!(
            facts.get(&original),
            Some(&TypeLattice::of(TclType::String))
        );
        assert_eq!(facts.get(&replacement), None);
        assert_eq!(facts.get(&original.compatibility_name()), None);
        assert_eq!(facts.get("x"), None);
    }

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default().project_for_profile(
            tcl_dialect::DialectProfile::find("tcl9.0").expect("native Tcl fixture"),
        )
    }

    /// Evaluate one statement's def typing against empty context pieces and
    /// return the lattice a def named `def_name` receives (`DefTyping::PerDef`
    /// defs absent from the map widen to `Overdefined`, matching the
    /// propagation pass).
    fn eval_def(
        stmt: &Statement,
        registry: &CommandRegistry,
        ssa: &SsaFunction,
        def_name: &str,
    ) -> TypeLattice {
        let uses = HashMap::new();
        let types = HashMap::new();
        let values = HashMap::new();
        let known_classes: HashSet<String> = HashSet::new();
        let ctx = WordTypingCtx {
            preparations: &[],
            tokens: stmt.tokens(),
            source: crate::ssa::SsaSourceView::unpositioned(ssa),
            uses: &uses,
            context: registry
                .profile()
                .map(tcl_registry::model::semantic::SemanticContext::for_profile)
                .map(Into::into),
            config: tcl_lexer::LexerConfig::for_profile(registry.profile()),
            types: &types,
            values: &values,
            registry,
            known_classes: &known_classes,
            namespace: "::",
            ssa,
            numbers: numbers_of(registry),
            active_writes: &[],
            normal_results: None,
        };
        match evaluate_type_def(stmt, &ctx) {
            DefTyping::Uniform(t) => t,
            DefTyping::PerDef(map) => map
                .get(def_name)
                .cloned()
                .unwrap_or_else(TypeLattice::overdefined),
        }
    }

    fn empty_sccp(f: &Function, blocks: &[&str]) -> SccpResult {
        SccpResult {
            required_math_invocations: Vec::new(),
            required_expression_preparations: Vec::new(),
            values: HashMap::new(),
            executable_blocks: blocks
                .iter()
                .map(|n| f.block_id(n).expect("interned block"))
                .collect(),
            executable_edges: HashSet::default(),
            constant_branches: Vec::new(),
        }
    }

    fn assign_const(name: &str, value: &str) -> Statement {
        Statement::AssignConst {
            span: Span::new(0, 0),
            name: name.to_owned(),
            value: value.to_owned(),
            name_braced: false,
            value_span: None,
        }
    }

    fn make_ssa_stmt(ssa: &mut SsaFunction, stmt: Statement, defs: &[(&str, u32)]) -> SsaStatement {
        SsaStatement {
            statement: stmt,
            uses: HashMap::new(),
            defs: defs.iter().map(|&(n, v)| (ssa.intern_var(n), v)).collect(),
            may_defs: std::collections::HashSet::new(),
            destruction_defs: std::collections::HashSet::new(),
            quoted_uses: std::collections::HashSet::new(),
            name_only_uses: std::collections::HashSet::new(),
        }
    }

    #[test]
    fn integer_literal_infers_int() {
        let mut f = Function::new("::top", "entry");
        let entry = f.entry;
        f.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(assign_const("x", "42"));

        let sccp = empty_sccp(&f, &["entry"]);
        let mut ssa = SsaFunction::trivial("::top", entry, f.block_names().to_vec());
        let stmt = make_ssa_stmt(&mut ssa, assign_const("x", "42"), &[("x", 1)]);
        ssa.blocks.insert(
            entry,
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: vec![stmt],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );
        let x = ssa.var_symbol("x").unwrap();
        let types = propagate_types(
            &f,
            &ssa,
            &sccp,
            &registry(),
            &HashSet::new(),
            &HashSet::new(),
            crate::compilation_unit::ModuleTraceFacts::none(),
        );
        assert_eq!(types.get(&(x, 1)), Some(&TypeLattice::of(TclType::Int)));
    }

    #[test]
    fn incr_infers_int() {
        let stmt = Statement::Incr {
            span: Span::new(0, 0),
            name: "n".to_owned(),
            name_braced: false,
            amount: None,
            safe_on_uninit: false,
        };
        let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
        let t = eval_def(&stmt, &registry(), &ssa, "__def__");
        assert_eq!(t, TypeLattice::of(TclType::Int));
    }

    #[test]
    fn lassign_destructure_defs_are_overdefined_not_command_return_type() {
        // TP: `lassign $pipe a b` writes destructured list *elements*;
        // `lassign`'s own `return_type` (List — the *leftover* elements) must
        // not be broadcast onto the targets. Pre-fix, both `a` and `b` were
        // typed LIST, so a later channel-position use (`puts $a ...`) would
        // falsely fire W126 ("has type LIST, not CHANNEL") — see FP-STY-04.
        // The typing now comes from the registry's `VarWriteTyping` for
        // `lassign` (`Destructured`), not a def-count heuristic.
        let stmt = Statement::Call {
            span: Span::new(0, 0),
            command: "lassign".to_owned(),
            canonical_command: None,
            args: vec!["$pipe".to_owned(), "a".to_owned(), "b".to_owned()],
            defs: vec!["a".to_owned(), "b".to_owned()],
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
        let t = eval_def(&stmt, &registry(), &ssa, "__def__");
        assert_eq!(t, TypeLattice::overdefined());
    }

    #[test]
    fn lassign_single_destructure_def_is_overdefined_not_list() {
        // A *single*-target `lassign $l x` is the case a `defs.len() > 1`
        // heuristic misses — one write falls
        // through to `lassign`'s `List` return type, so `x` is typed LIST and
        // `expr {$x + 1}` fired a bogus S100. The registry's `Destructured`
        // typing widens it to OVERDEFINED regardless of target count.
        let stmt = Statement::Call {
            span: Span::new(0, 0),
            command: "lassign".to_owned(),
            canonical_command: None,
            args: vec!["$point".to_owned(), "x".to_owned()],
            defs: vec!["x".to_owned()],
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
        let t = eval_def(&stmt, &registry(), &ssa, "__def__");
        assert_eq!(t, TypeLattice::overdefined());
    }

    #[test]
    fn single_def_command_still_uses_its_declared_return_type() {
        // TN control: a command that writes exactly one variable (`append`)
        // legitimately shares its return value with that variable — its
        // `VarWriteTyping` is the default `ReturnValue`, so it must keep taking
        // the declared return type (`String`), not widen to OVERDEFINED.
        let stmt = Statement::Call {
            span: Span::new(0, 0),
            command: "append".to_owned(),
            canonical_command: None,
            args: vec!["result".to_owned(), "x".to_owned()],
            defs: vec!["result".to_owned()],
            reads: Vec::new(),
            reads_own_defs: true,
            safe_on_uninit: true,
            tokens: None,
            foreach_groups: None,
        };
        let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
        let t = eval_def(&stmt, &registry(), &ssa, "__def__");
        assert_eq!(t, TypeLattice::of(TclType::String));
    }

    #[test]
    fn unannotated_multi_def_call_stays_overdefined_not_return_type() {
        // A call that writes SEVERAL variables
        // under the default `ReturnValue` typing must not broadcast its
        // return type onto all of them. The synthetic `catch {body} resultVar
        // optionsVar` call `emit_opaque_catch` builds carries the body's
        // writes plus the result/options vars as defs, while `catch` returns
        // an Int status code and declares no `VarWriteTyping` override — typing
        // `msg`/`result`/`opts` as that Int would wrongly fire S100/W126. The
        // default arm's multi-def guard keeps them OVERDEFINED.
        let stmt = Statement::Call {
            span: Span::new(0, 0),
            command: "catch".to_owned(),
            canonical_command: None,
            args: vec![
                "{set msg hello}".to_owned(),
                "result".to_owned(),
                "opts".to_owned(),
            ],
            defs: vec!["msg".to_owned(), "result".to_owned(), "opts".to_owned()],
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        };
        let ssa = SsaFunction::trivial("::top", BlockId(0), vec!["entry".into()]);
        let t = eval_def(&stmt, &registry(), &ssa, "__def__");
        assert_eq!(t, TypeLattice::overdefined());
    }

    /// End-to-end lattice checks for the registry-driven `VarWriteTyping`:
    /// each destructuring writer types its side-effect target correctly,
    /// distinct from its return type.
    #[test]
    fn var_write_typing_shapes_destructure_target_types() {
        use crate::compilation_unit::CompilationUnit;

        // Helper: does any version of `name` carry a KNOWN type `t`?
        fn any_known(fu: &crate::compilation_unit::FunctionUnit, name: &str, t: TclType) -> bool {
            fu.types.iter().any(|((sym, _), lat)| {
                fu.ssa.var_name(*sym) == name
                    && lat.kind() == TypeKind::Known
                    && lat.tcl_type() == Some(t)
            })
        }
        // Helper: is every version of `name` non-Known (OVERDEFINED/UNKNOWN)?
        fn none_known(fu: &crate::compilation_unit::FunctionUnit, name: &str) -> bool {
            fu.types
                .iter()
                .filter(|((sym, _), _)| fu.ssa.var_name(*sym) == name)
                .all(|(_, lat)| lat.kind() != TypeKind::Known)
        }

        // `lassign` element target — OVERDEFINED, never List.
        let cu = CompilationUnit::build_for("set p [list 1 2 3]\nlassign $p x", &registry(), false);
        let fu = cu.function("::top").unwrap();
        assert!(
            none_known(fu, "x"),
            "lassign target must not carry a Known type: {:?}",
            fu.types
        );

        // `regexp` capture — OVERDEFINED, never Int (the match count).
        let cu = CompilationUnit::build_for("regexp {(.)} abc c", &registry(), false);
        let fu = cu.function("::top").unwrap();
        assert!(none_known(fu, "c"), "regexp capture must not be Known Int");

        // `scan` target — OVERDEFINED (format-dependent), never Int.
        let cu = CompilationUnit::build_for("scan hello %s word", &registry(), false);
        let fu = cu.function("::top").unwrap();
        assert!(none_known(fu, "word"), "scan target must not be Known Int");

        // `binary scan` target (subcommand-level typing) — OVERDEFINED.
        let cu = CompilationUnit::build_for("binary scan $d a3 chars", &registry(), false);
        let fu = cu.function("::top").unwrap();
        assert!(
            none_known(fu, "chars"),
            "binary scan target must not be Known Int"
        );

        // `gets chan line` — Fixed(String): the target is the read line, a
        // String, not the character count the two-arg form returns.
        let cu = CompilationUnit::build_for("set ch stdin; gets $ch ::line", &registry(), false);
        let fu = cu.function("::top").unwrap();
        assert!(
            any_known(fu, "::line", TclType::String),
            "gets target must be Known String: {:?}; escaping {:?}; operations {:?}",
            fu.types,
            crate::var_observability::analyse_var_observability(&fu.cfg, &registry())
                .escaping_var_names(),
            fu.ssa
                .blocks
                .values()
                .flat_map(|block| &block.statements)
                .map(|statement| (
                    statement
                        .defs
                        .keys()
                        .map(|symbol| fu.ssa.var_name(*symbol))
                        .collect::<Vec<_>>(),
                    &statement.may_defs,
                    crate::registry_invocation::resolved_statement_invocation(
                        &registry(),
                        None,
                        &statement.statement,
                    )
                    .map(|invocation| invocation.facts.operation),
                ))
                .collect::<Vec<_>>()
        );

        // `lpop listVar` — Fixed(List): the variable is left holding the
        // shortened list, not the popped element (String) it returns.
        let cu = CompilationUnit::build_for("set l [list 1 2 3]\nlpop l", &registry(), false);
        let fu = cu.function("::top").unwrap();
        assert!(
            any_known(fu, "l", TclType::List),
            "lpop target must be Known List, not the element return type: {:?}",
            fu.types
        );
    }

    #[test]
    fn float_literal_infers_double() {
        let t = literal_type("3.14", NumberSyntax::Tcl90);
        assert_eq!(t, TypeLattice::of(TclType::Double));
    }

    #[test]
    fn bool_literal_infers_boolean() {
        let t = literal_type("true", NumberSyntax::Tcl90);
        assert_eq!(t, TypeLattice::of(TclType::Boolean));
        let t2 = literal_type("false", NumberSyntax::Tcl90);
        assert_eq!(t2, TypeLattice::of(TclType::Boolean));
    }

    #[test]
    fn string_literal_infers_string() {
        let t = literal_type("hello", NumberSyntax::Tcl90);
        assert_eq!(t, TypeLattice::of(TclType::String));
    }

    #[test]
    fn hex_and_binary_literals_infer_int() {
        // Hex / binary literals store an INT intrep (`set n 0x80; incr n` is
        // one clean parse, not per-iteration shimmer).
        for lit in ["0x80", "0X1f", "-0xFF", "0b1010", "0B1", "+0x0"] {
            assert_eq!(
                literal_type(lit, NumberSyntax::Tcl90),
                TypeLattice::of(TclType::Int),
                "{lit} should be INT"
            );
        }
        // Octal `0o…` and non-integer hex stay STRING (set-statement classifier).
        assert_eq!(
            literal_type("0o17", NumberSyntax::Tcl90),
            TypeLattice::of(TclType::String)
        );
        assert_eq!(
            literal_type("0xZZ", NumberSyntax::Tcl90),
            TypeLattice::of(TclType::String)
        );
    }

    /// The literal classifiers read numerals under the *target release's*
    /// grammar, and the set/expr divergence on `0o…` survives it.
    ///
    /// The matrix (from the C release sources): `0x` in every release; `0o`/`0b`
    /// from 8.5; `0d` and `_` separators from 9.0; a bare leading zero is octal
    /// up to 8.6 and decimal from 9.0. A spelling the release does not have is
    /// not a numeral — a bareword in expr context (`Numeric`, since an `expr`
    /// still yields a number) and a plain `String` after `set`.
    #[test]
    fn literal_classifiers_follow_the_release_numeral_matrix() {
        use NumberSyntax::{Tcl84, Tcl85, Tcl90};
        let set_type = |lit: &str, n| literal_type(lit, n).tcl_type();
        let expr_type = |lit: &str, n| expr_literal_type(lit, n).tcl_type();

        // `0o17`: the deliberate divergence. From 8.5 it is an `Int` in expr
        // context but stays `String` after `set` (its canonical intrep, `15`,
        // differs from the source text). Under 8.4 there is no `0o` prefix at
        // all, so it is a bareword either way.
        for syntax in [Tcl85, Tcl90] {
            assert_eq!(
                set_type("0o17", syntax),
                Some(TclType::String),
                "{syntax:?}"
            );
            assert_eq!(expr_type("0o17", syntax), Some(TclType::Int), "{syntax:?}");
        }
        assert_eq!(set_type("0o17", Tcl84), Some(TclType::String));
        assert_eq!(expr_type("0o17", Tcl84), Some(TclType::Numeric));

        // `0d99` — the 9.0-only decimal prefix.
        assert_eq!(set_type("0d99", Tcl90), Some(TclType::Int));
        assert_eq!(expr_type("0d99", Tcl90), Some(TclType::Int));
        for syntax in [Tcl84, Tcl85] {
            assert_eq!(
                set_type("0d99", syntax),
                Some(TclType::String),
                "{syntax:?}"
            );
            assert_eq!(
                expr_type("0d99", syntax),
                Some(TclType::Numeric),
                "{syntax:?}"
            );
        }

        // `1_000` — 9.0-only digit separators.
        assert_eq!(set_type("1_000", Tcl90), Some(TclType::Int));
        assert_eq!(expr_type("1_000", Tcl90), Some(TclType::Int));
        for syntax in [Tcl84, Tcl85] {
            assert_eq!(
                set_type("1_000", syntax),
                Some(TclType::String),
                "{syntax:?}"
            );
            assert_eq!(
                expr_type("1_000", syntax),
                Some(TclType::Numeric),
                "{syntax:?}"
            );
        }

        // `0b101` — 8.5 onward.
        assert_eq!(set_type("0b101", Tcl84), Some(TclType::String));
        assert_eq!(expr_type("0b101", Tcl84), Some(TclType::Numeric));
        for syntax in [Tcl85, Tcl90] {
            assert_eq!(set_type("0b101", syntax), Some(TclType::Int), "{syntax:?}");
            assert_eq!(expr_type("0b101", syntax), Some(TclType::Int), "{syntax:?}");
        }

        // A radix-invalid digit is never a numeral, in any release.
        for syntax in [Tcl84, Tcl85, Tcl90] {
            for bad in ["0o8", "0xZZ"] {
                assert_eq!(
                    set_type(bad, syntax),
                    Some(TclType::String),
                    "{bad} under {syntax:?}"
                );
                assert_eq!(
                    expr_type(bad, syntax),
                    Some(TclType::Numeric),
                    "{bad} under {syntax:?}"
                );
            }
        }

        // A bare leading zero is an integer in every release (only its *value*
        // differs: 493 up to 8.6, 755 from 9.0 — see `intervals`), while `08`
        // is a broken octal up to 8.6 and plain decimal 8 from 9.0.
        for syntax in [Tcl84, Tcl85, Tcl90] {
            assert_eq!(set_type("0755", syntax), Some(TclType::Int), "{syntax:?}");
            assert_eq!(expr_type("0755", syntax), Some(TclType::Int), "{syntax:?}");
        }
        for syntax in [Tcl84, Tcl85] {
            assert_eq!(set_type("08", syntax), Some(TclType::String), "{syntax:?}");
            assert_eq!(
                expr_type("08", syntax),
                Some(TclType::Numeric),
                "{syntax:?}"
            );
        }
        assert_eq!(set_type("08", Tcl90), Some(TclType::Int));
        assert_eq!(expr_type("08", Tcl90), Some(TclType::Int));
    }

    #[test]
    fn phi_joins_types_from_executable_preds() {
        // A minimal two-block CFG: entry → exit.
        // The phi in exit merges version 1 (INT) from entry.
        let mut cfg = Function::new("::top", "entry");
        let entry = cfg.entry;
        let exit = cfg.intern_block("exit");
        cfg.blocks.insert(exit, Block::new("exit"));
        cfg.blocks.get_mut(&entry).unwrap().terminator = Some(crate::cfg::Terminator::Goto {
            target: exit,
            span: None,
        });

        let mut ssa = SsaFunction::trivial("::top", entry, cfg.block_names().to_vec());
        let x = ssa.intern_var("x");
        let phi = Phi {
            name: x,
            version: 2,
            incoming: [(entry, 1u32)].into_iter().collect(),
        };
        let entry_stmt = make_ssa_stmt(&mut ssa, assign_const("x", "10"), &[("x", 1)]);
        ssa.blocks.insert(
            entry,
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: vec![entry_stmt],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );
        ssa.blocks.insert(
            exit,
            SsaBlock {
                name: "exit".into(),
                phis: vec![phi],
                statements: Vec::new(),
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );

        let mut sccp = empty_sccp(&cfg, &["entry", "exit"]);
        sccp.executable_edges.insert((entry, exit));

        let types = propagate_types(
            &cfg,
            &ssa,
            &sccp,
            &registry(),
            &HashSet::new(),
            &HashSet::new(),
            crate::compilation_unit::ModuleTraceFacts::none(),
        );
        // x@1 (entry) should be Int.
        assert_eq!(types.get(&(x, 1)), Some(&TypeLattice::of(TclType::Int)));
        // x@2 (phi in exit) should propagate Int from entry.
        assert_eq!(types.get(&(x, 2)), Some(&TypeLattice::of(TclType::Int)));
    }

    /// `AssignValue` with a pure variable reference inherits the source type.
    #[test]
    fn assign_value_pure_var_ref_inherits_type() {
        use crate::compilation_unit::CompilationUnit;
        let cu = CompilationUnit::build_for("set x 42\nset y $x", &registry(), false);
        let fu = cu.function("::top").unwrap();
        // x should be Int; y (which copies x) should also be Int.
        let x_is_int = fu.types.iter().any(|((name, _), t)| {
            fu.ssa.var_name(*name) == "x" && t.tcl_type() == Some(TclType::Int)
        });
        let y_is_int = fu.types.iter().any(|((name, _), t)| {
            fu.ssa.var_name(*name) == "y" && t.tcl_type() == Some(TclType::Int)
        });
        assert!(x_is_int, "expected x to be Int");
        assert!(y_is_int, "expected y to inherit Int type from x");
    }

    /// An aliased `set` (`interp alias {} myset {} set`) keeps its runtime
    /// `Call` shape, but its single def takes the *value word's* intrep — the
    /// canonical-command value-passthrough. `myset x 5` types x as Int, exactly
    /// as `set x 5` would, rather than an opaque OVERDEFINED `Call`.
    #[test]
    fn aliased_set_call_types_def_from_value() {
        use crate::compilation_unit::CompilationUnit;
        let cu = CompilationUnit::build_for(
            "interp alias {} myset {} set\nproc ::g {} { myset x 5\n return $x }",
            &registry(),
            false,
        );
        let fu = cu.function("::g").unwrap();
        let x_is_int = fu.types.iter().any(|((name, _), t)| {
            fu.ssa.var_name(*name) == "x" && t.tcl_type() == Some(TclType::Int)
        });
        assert!(
            x_is_int,
            "aliased `myset x 5` should type x as Int (value passthrough): {:?}",
            fu.types
                .iter()
                .map(|((n, v), t)| (fu.ssa.var_name(*n), *v, t.to_string()))
                .collect::<Vec<_>>()
        );
    }

    /// `AssignValue` with a command substitution uses the command's return type.
    #[test]
    fn assign_value_command_sub_uses_return_type() {
        use crate::compilation_unit::CompilationUnit;
        // `llength` returns Int per the registry.
        let cu =
            CompilationUnit::build_for("set lst {a b c}\nset n [llength $lst]", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let n_is_int = fu.types.iter().any(|((name, _), t)| {
            fu.ssa.var_name(*name) == "n" && t.tcl_type() == Some(TclType::Int)
        });
        assert!(n_is_int, "expected n to be Int (llength return type)");
    }

    #[test]
    fn normal_result_representation_does_not_require_an_opcode() {
        use crate::compilation_unit::CompilationUnit;
        let registry = CommandRegistry::build_default()
            .project_for_profile(tcl_dialect::DialectProfile::irules());
        let source = "when HTTP_REQUEST {set input {a b}; set size [llength $input]}";
        let cu = CompilationUnit::build_for_profile(
            source,
            &registry,
            false,
            tcl_dialect::DialectProfile::irules(),
        );
        let function = cu.function("::when::HTTP_REQUEST").expect("handler");
        assert!(
            function.types.iter().any(|((symbol, _), value)| {
                function.ssa.var_name(*symbol) == "size" && value.tcl_type() == Some(TclType::Int)
            }),
            "successful producer: {:?}",
            function.types
        );
        let shadow = CompilationUnit::build_for_profile(
            "proc llength {args} {return arbitrary}; when HTTP_REQUEST {set input {a b}; set size [llength $input]}",
            &registry,
            false,
            tcl_dialect::DialectProfile::irules(),
        );
        let shadow = shadow.function("::when::HTTP_REQUEST").expect("handler");
        assert!(
            !shadow.types.iter().any(|((symbol, _), value)| {
                shadow.ssa.var_name(*symbol) == "size" && value.tcl_type() == Some(TclType::Int)
            }),
            "custom producer: {:?}",
            shadow.types
        );
    }

    /// A `dict set VAR k [Class new]` collection retrieved by `dict get` types
    /// the element as the class (the collection-of-objects shape).
    #[test]
    fn dict_of_objects_retrieval_types_element() {
        use crate::compilation_unit::CompilationUnit;
        let src = "oo::class create Pin { method cfg {args} {} }\n\
                   dict set pins a [Pin new]\n\
                   set p [dict get $pins a]\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").expect("top level");
        let pins_ok = fu.types.iter().any(|((name, _), t)| {
            fu.ssa.var_name(*name) == "pins" && t.element_class() == Some("::Pin")
        });
        assert!(
            pins_ok,
            "pins should be Dict<OBJECT(::Pin)>; got {:?}",
            fu.types
                .iter()
                .map(|((n, v), t)| (fu.ssa.var_name(*n), *v, t.to_string()))
                .collect::<Vec<_>>()
        );
        let p_ok = fu.types.iter().any(|((name, _), t)| {
            fu.ssa.var_name(*name) == "p"
                && t.tcl_type() == Some(TclType::Object)
                && t.class_name() == Some("::Pin")
        });
        assert!(p_ok, "p (dict get) should be OBJECT(::Pin)");
    }

    /// A `lappend VAR [Class new]` list retrieved by `lindex` types the element.
    #[test]
    fn list_of_objects_lindex_types_element() {
        use crate::compilation_unit::CompilationUnit;
        let src = "oo::class create Pin {}\n\
                   lappend pins [Pin new]\n\
                   set p [lindex $pins 0]\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").expect("top level");
        let p_ok = fu.types.iter().any(|((name, _), t)| {
            fu.ssa.var_name(*name) == "p"
                && t.tcl_type() == Some(TclType::Object)
                && t.class_name() == Some("::Pin")
        });
        assert!(p_ok, "p (lindex) should be OBJECT(::Pin)");
    }

    /// A collection written with two *different* object classes is not
    /// homogeneous, so the element class widens away (retrieval stays untyped).
    #[test]
    fn heterogeneous_object_collection_drops_element_class() {
        use crate::compilation_unit::CompilationUnit;
        let src = "oo::class create A {}\noo::class create B {}\n\
                   dict set d k1 [A new]\n\
                   dict set d k2 [B new]\n";
        let cu = CompilationUnit::build_for(src, &registry(), false);
        let fu = cu.function("::top").expect("top level");
        // The latest `d` version must not claim a single element class.
        let widened = fu
            .types
            .iter()
            .filter(|((name, _), _)| fu.ssa.var_name(*name) == "d")
            .all(|((_, ver), t)| *ver < 2 || t.element_class().is_none());
        assert!(
            widened,
            "mixed-class dict must drop its element class; types {:?}; operations {:?}",
            fu.types,
            fu.ssa
                .blocks
                .iter()
                .map(|(id, block)| (
                    id,
                    block
                        .statements
                        .iter()
                        .enumerate()
                        .map(|(index, statement)| (
                            &statement.defs,
                            &statement.uses,
                            crate::ssa::SsaSourceView::at_statement(&fu.ssa, *id, index)
                                .symbol("d"),
                        ))
                        .collect::<Vec<_>>()
                ))
                .collect::<Vec<_>>()
        );
    }

    // expr type-inference precision

    /// Infer the type of a standalone expression string (no SSA
    /// context — variable refs stay `Unknown`), under the Tcl 9.0 numeral
    /// grammar.
    fn infer_str(src: &str) -> TypeLattice {
        infer_str_dialect(src, None)
    }

    /// As [`infer_str`] but parses under `dialect` (the iRules string
    /// predicates only tokenise as operators in the iRules dialect) and reads
    /// numerals under that dialect's grammar.
    fn infer_str_dialect(
        src: &str,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> TypeLattice {
        let node = crate::parse_expr_for_profile(src, dialect);
        infer_expr_type(
            &node,
            &HashMap::new(),
            0,
            crate::intervals::numbers_for_dialect(dialect),
        )
    }

    #[test]
    fn decoded_expression_names_keep_literal_sigils_and_element_identity() {
        let profile = tcl_registry::model::ingress::static_context_for("tcl8.6")
            .commands()
            .profile();
        let types = HashMap::from([
            ("b".to_owned(), TypeLattice::of(TclType::List)),
            ("$b".to_owned(), TypeLattice::of(TclType::Int)),
            ("a".to_owned(), TypeLattice::of(TclType::Dict)),
            ("a(k)".to_owned(), TypeLattice::of(TclType::Double)),
        ]);
        for (source, expected) in [("${$b}", TclType::Int), ("${a(k)}", TclType::Double)] {
            let expression = crate::parse_expr_for_profile(source, profile);
            let reference = expression
                .variable_reference(tcl_lexer::LexerConfig::from_grammar(
                    profile.unwrap().grammar,
                ))
                .unwrap()
                .unwrap();
            assert!(reference.index.is_none());
            let ExprNode::Var { start, end, .. } = &expression else {
                panic!("variable");
            };
            let occurrence_types = HashMap::from([(
                (source.to_owned(), *start, *end),
                types[std::str::from_utf8(reference.name).unwrap()].clone(),
            )]);
            assert_eq!(
                infer_expr_type(
                    &expression,
                    &occurrence_types,
                    0,
                    NumberSyntax::of_profile(profile)
                )
                .tcl_type(),
                Some(expected),
                "{source}"
            );
        }
    }

    #[test]
    fn expression_contents_types_use_original_element_and_literal_name_reads() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (source, expected) in [
            (
                "proc f {} {set {$b} [expr {7<<1}]; set b [list x]; return [expr {${$b}}]}; f",
                TclType::Int,
            ),
            (
                "proc f {} {set a(k) [expr {7.5}]; return [expr {${a(k)}}]}; f",
                TclType::Double,
            ),
            (
                "proc f {} {set k k; set a(k) [expr {7.5}]; return [expr {$a($k)}]}; f",
                TclType::Double,
            ),
        ] {
            let unit = crate::compilation_unit::CompilationUnit::build_for(source, registry, false);
            assert_eq!(
                unit.function("::f").unwrap().return_type.tcl_type(),
                Some(expected),
                "{source}"
            );
        }
    }

    #[test]
    fn written_container_reads_keep_raw_and_braced_variable_identity() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for word in ["$items", "${items}"] {
            let source = format!(
                "proc f {{raw}} {{set items [list [expr {{$raw<<1}}]]; return [lindex {word} 0]}}; f 7"
            );
            let unit =
                crate::compilation_unit::CompilationUnit::build_for(&source, registry, false);
            assert_eq!(
                unit.function("::f").unwrap().return_type.tcl_type(),
                Some(TclType::Int),
                "{source}"
            );
        }
    }

    #[test]
    fn math_function_calls_infer_their_return_type() {
        // (a) sqrt → Double, int → Int, bool → Boolean.
        assert_eq!(infer_str("sqrt(2.0)").tcl_type(), Some(TclType::Double));
        assert_eq!(infer_str("sin($x)").tcl_type(), Some(TclType::Double));
        assert_eq!(infer_str("int($x)").tcl_type(), Some(TclType::Int));
        assert_eq!(infer_str("wide($x)").tcl_type(), Some(TclType::Int));
        assert_eq!(infer_str("isnan($x)").tcl_type(), Some(TclType::Boolean));
        // abs is identity: abs(2) keeps the operand's Int type.
        assert_eq!(infer_str("abs(2)").tcl_type(), Some(TclType::Int));
        // max/min join operands: max(1, 2) stays Int.
        assert_eq!(infer_str("max(1, 2)").tcl_type(), Some(TclType::Int));
        // Tcl 9.1 C99 functions (TIP 745): double-valued, except `signbit`.
        for f in [
            "acosh", "cbrt", "exp2", "log2", "trunc", "erf", "expm1", "logb",
        ] {
            assert_eq!(
                infer_str(&format!("{f}($x)")).tcl_type(),
                Some(TclType::Double),
                "{f} should infer Double",
            );
        }
        assert_eq!(infer_str("signbit($x)").tcl_type(), Some(TclType::Boolean));
        // Unknown function → Numeric (conservative).
        assert_eq!(infer_str("nope($x)").tcl_type(), Some(TclType::Numeric));
    }

    /// `infer_expr_type` (and its
    /// mutually-recursive helper `expr_call_type`) recurse once per
    /// `ExprNode` operator-tree level, so both need a depth cap.
    /// A long left-associative `1+1+1+…` chain parses *iteratively* (the
    /// Pratt parser's own `MAX_EXPR_DEPTH` never trips — it builds the
    /// left-nested `Binary` spine in a loop), so it hands this walker an
    /// arbitrarily deep tree the parser cap does not bound. Empirically that
    /// unguarded walk overflowed the native stack (SIGABRT) in the low
    /// thousands of levels on a 2 MiB thread (`cargo test`'s per-test
    /// default). 3000 is comfortably past both that crash range and
    /// `MAX_EXPR_NODE_DEPTH` (256); the assertion is that inference returns
    /// at all, not what it returns. Also exercises `expr_call_type` via the
    /// nested-`min(…)` variant.
    #[test]
    fn deeply_nested_expr_type_inference_survives() {
        let chain = format!("1{}", "+1".repeat(3000));
        // Returns without overflowing the stack; the value is unspecified
        // past the cap (a conservative `overdefined`/`numeric`).
        let _ = infer_str(&chain);

        // Drive the `expr_call_type` side of the same recursion cluster with
        // a tree built directly — nested `min(min(…))` *source* would trip
        // the Pratt parser's own `MAX_EXPR_DEPTH` first (function-call
        // nesting recurses the parser, unlike the iterative binary chain
        // above), so build the deep `Call` spine by hand to reach this
        // walker unbounded.
        let mut node = ExprNode::Literal {
            text: "1".to_owned(),
            start: 0,
            end: 1,
        };
        for _ in 0..3000 {
            node = ExprNode::Call {
                function: "abs".to_owned(),
                args: vec![node],
                start: 0,
                end: 1,
            };
        }
        let _ = infer_expr_type(&node, &HashMap::new(), 0, NumberSyntax::Tcl90);
    }

    /// Companion to `deeply_nested_expr_type_inference_survives`: moderate
    /// nesting (well under `MAX_EXPR_NODE_DEPTH`) must still infer exactly as
    /// before — the cap changes nothing for realistic input. A 100-deep
    /// all-integer additive chain is unambiguously `Int`.
    #[test]
    fn moderate_depth_expr_type_inference_unchanged() {
        let chain = format!("1{}", "+1".repeat(100));
        assert_eq!(infer_str(&chain).tcl_type(), Some(TclType::Int));
    }

    #[test]
    fn bitwise_and_shift_ops_infer_int() {
        // (c) bitwise / shift force Int even with untyped operands.
        for src in ["$x & $y", "$x | $y", "$x ^ $y", "$x << 2", "$x >> 2"] {
            assert_eq!(
                infer_str(src).tcl_type(),
                Some(TclType::Int),
                "expected Int for `{src}`",
            );
        }
    }

    #[test]
    fn irules_string_predicates_infer_boolean() {
        // (b) the iRules string predicates were falling through to
        // overdefined; they are Boolean.
        for src in [
            "$s contains \"x\"",
            "$s starts_with \"x\"",
            "$s ends_with \"x\"",
            "$s equals \"x\"",
            "$s matches_glob \"x*\"",
            "$s matches_regex \"x.\"",
        ] {
            let t = infer_str_dialect(src, Some(tcl_dialect::DialectProfile::irules()));
            assert_eq!(
                t.tcl_type(),
                Some(TclType::Boolean),
                "expected Boolean for `{src}`, got {t:?}",
            );
        }
    }

    #[test]
    fn arithmetic_promotes_double() {
        // `_arithmetic_result`: int + double → Double (was Numeric).
        assert_eq!(infer_str("3 + 2.0").tcl_type(), Some(TclType::Double));
        // int + int → Int.
        assert_eq!(infer_str("3 + 4").tcl_type(), Some(TclType::Int));
    }

    #[test]
    fn expr_context_literal_typing() {
        // Every integer spelling tokenises to Int in expr context — including
        // octal `0o…`, which the set-statement classifier keeps as String.
        assert_eq!(infer_str("0o17").tcl_type(), Some(TclType::Int));
        assert_eq!(infer_str("0xff").tcl_type(), Some(TclType::Int));
        assert_eq!(infer_str("0b1010").tcl_type(), Some(TclType::Int));
        assert_eq!(infer_str("42").tcl_type(), Some(TclType::Int));
        assert_eq!(infer_str("3.14").tcl_type(), Some(TclType::Double));
        // The set-statement classifier still keeps octal as String.
        assert_eq!(
            literal_type("0o17", NumberSyntax::Tcl90),
            TypeLattice::of(TclType::String)
        );
        // An unrecognised literal degrades to Numeric in expr context
        // (the set-statement classifier would say String).
        assert_eq!(
            expr_literal_type("nope", NumberSyntax::Tcl90).tcl_type(),
            Some(TclType::Numeric)
        );
    }

    #[test]
    fn bitnot_coerces_to_int() {
        // `~$x` always yields Int, regardless of the operand's type
        // (was leaking the operand type via the identity arm).
        assert_eq!(infer_str("~$x").tcl_type(), Some(TclType::Int));
        assert_eq!(infer_str("~3.5").tcl_type(), Some(TclType::Int));
    }

    #[test]
    fn scope_alias_commands_detected() {
        let reg = registry();
        let one = |s: &str| vec![s.to_owned()];
        assert!(is_scope_alias_call(&reg, "global", &one("g")));
        assert!(is_scope_alias_call(&reg, "variable", &one("v")));
        assert!(is_scope_alias_call(
            &reg,
            "upvar",
            &["0".into(), "x".into(), "y".into()]
        ));
        assert!(is_scope_alias_call(
            &reg,
            "namespace",
            &["upvar".into(), "ns".into(), "x".into(), "y".into()]
        ));
        assert!(is_scope_alias_call(
            &reg,
            "my",
            &["variable".into(), "count".into()]
        ));
        // A plain command (and `namespace eval`) is not a scope alias.
        assert!(!is_scope_alias_call(&reg, "set", &["x".into(), "1".into()]));
        assert!(!is_scope_alias_call(
            &reg,
            "namespace",
            &["eval".into(), "ns".into(), "body".into()]
        ));
        // `my`'s other subcommands (an arbitrary method name) are not scope
        // aliases — only the reserved `variable` word is.
        assert!(!is_scope_alias_call(&reg, "my", &["touch".into()]));
    }

    #[test]
    fn scope_alias_declaration_preserves_unknown_contents() {
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc ::f {} { variable counter; set observed $counter; return $observed }",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        assert!(
            fu.types
                .iter()
                .all(|((name, _), _)| fu.ssa.var_name(*name) != "::counter"),
            "a declaration alone cannot write namespace contents"
        );
        assert_eq!(
            fu.return_type.kind(),
            TypeKind::Overdefined,
            "an external cell's unknown contents are not the declaration's return value; actual return {}",
            fu.return_type
        );
    }

    #[test]
    fn actual_traced_alias_store_preserves_only_a_closed_unchanged_value() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        for (callback, preserves_dictionary) in [
            ("", true),
            // A callback can coerce the same value without changing its bytes
            // or dictionary contents. Current representation is a separate
            // read receipt consumed by representation diagnostics.
            ("upvar 1 dictionary destination; llength $destination", true),
            (
                "upvar 1 dictionary destination; set destination altered",
                false,
            ),
        ] {
            let source = format!(
                "proc observe args {{{callback}}}; proc f {{}} {{set dictionary [dict create k 1]; upvar 0 dictionary view; trace add variable view write observe; set dictionary [dict create k 2]; llength $view}}"
            );
            let unit =
                crate::compilation_unit::CompilationUnit::build_for(&source, registry, false);
            let function = unit.function("::f").unwrap();
            let mut writes = Vec::new();
            for (&block, body) in &function.ssa.blocks {
                for (index, statement) in body.statements.iter().enumerate() {
                    if matches!(&statement.statement, Statement::AssignValue { name, .. } if name == "dictionary")
                    {
                        for (&symbol, &version) in &statement.defs {
                            writes.push((
                                statement.statement.span().start(),
                                function.types.get(&(symbol, version)),
                                crate::ssa::SsaSourceView::at_statement(
                                    &function.ssa,
                                    block,
                                    index,
                                )
                                .normal_store_contents_preserved(symbol, registry),
                            ));
                        }
                    }
                }
            }
            writes.sort_by_key(|(offset, _, _)| *offset);
            assert_eq!(writes.len(), 2);
            assert_eq!(writes[1].2, preserves_dictionary, "callback {callback:?}");
            assert_eq!(
                writes[1].1.unwrap().tcl_type() == Some(TclType::Dict),
                preserves_dictionary,
                "callback {callback:?}: {:?}",
                writes[1].1,
            );
        }
    }

    #[test]
    fn expression_types_follow_actual_alias_spellings_at_each_read() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let unit = crate::compilation_unit::CompilationUnit::build_for(
            "proc f {} {set original [expr {3.5}]; upvar 0 original view; set result [expr {$original}]; return [expr {$view}]}",
            registry,
            false,
        );
        let function = unit.function("::f").unwrap();
        assert_eq!(function.return_type.tcl_type(), Some(TclType::Double));
        let writes = function
            .ssa
            .blocks
            .values()
            .flat_map(|body| &body.statements);
        let result = writes
            .filter(|statement| matches!(&statement.statement, Statement::AssignExpr { name, .. } if name == "result"))
            .flat_map(|statement| &statement.defs)
            .filter_map(|(&symbol, &version)| function.types.get(&(symbol, version)))
            .collect::<Vec<_>>();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].tcl_type(), Some(TclType::Double));
    }

    #[test]
    fn conditionally_assigned_param_merges_to_overdefined() {
        use crate::compilation_unit::CompilationUnit;
        // A parameter assigned in only one arm must NOT be
        // typed from that arm alone. The merge phi joins the live-in
        // (caller-supplied, unknown → OVERDEFINED) with the assigned-arm Int,
        // so the merged type is OVERDEFINED — never Known Int.
        let cu = CompilationUnit::build_for(
            "proc ::p {c x} { if {$c} { set x 5 }\n return $x }",
            &registry(),
            false,
        );
        let fu = cu.function("::p").unwrap();
        // (TP) The merge phi for `x` is OVERDEFINED.
        let has_overdefined = fu
            .types
            .iter()
            .any(|((n, _), t)| fu.ssa.var_name(*n) == "x" && t.kind() == TypeKind::Overdefined);
        assert!(
            has_overdefined,
            "conditionally-assigned param 'x' should merge to OVERDEFINED: {:?}",
            fu.types
        );
        // (FP guard) The assigned arm (`set x 5`) is *still* typed Known Int —
        // the fix widens only the merge, not the definite assignment. Exactly
        // one `x` version is Known Int (the assigned arm) and one is
        // OVERDEFINED (the phi); pre-fix, the phi would also be Known Int.
        let known_int_count = fu
            .types
            .iter()
            .filter(|((n, _), t)| {
                fu.ssa.var_name(*n) == "x" && matches!(t.tcl_type(), Some(TclType::Int))
            })
            .count();
        assert_eq!(
            known_int_count, 1,
            "only the assigned arm of 'x' should be Known Int (not the phi): {:?}",
            fu.types
        );
    }

    #[test]
    fn unconditionally_assigned_param_still_typed() {
        use crate::compilation_unit::CompilationUnit;
        // (FP guard): an *unconditionally* reassigned local is
        // still typed from its definition — the version-0 join only applies
        // when a live-in genuinely reaches the merge. Here `y` is set on every
        // path, so its post-assignment type stays Known Int.
        let cu = CompilationUnit::build_for(
            "proc ::q {c} { set y 5\n if {$c} { set y 7 } else { set y 9 }\n return $y }",
            &registry(),
            false,
        );
        let fu = cu.function("::q").unwrap();
        let all_int = fu
            .types
            .iter()
            .filter(|((n, _), _)| fu.ssa.var_name(*n) == "y")
            .all(|(_, t)| {
                matches!(t.tcl_type(), Some(TclType::Int)) || t.kind() == TypeKind::Unknown
            });
        assert!(
            all_int,
            "unconditionally-assigned 'y' should stay Int on every path: {:?}",
            fu.types
        );
    }

    // P3: registry-driven container element inference (type-tracking.md).

    /// Helper: the joined lattice of every version of `var` in `func`.
    fn type_of(
        cu: &crate::compilation_unit::CompilationUnit,
        func: &str,
        var: &str,
    ) -> TypeLattice {
        let fu = cu.function(func).unwrap();
        let mut symbols = HashSet::new();
        symbols.extend(fu.ssa.var_symbol(var));
        for (&block, cfg_block) in &fu.cfg.blocks {
            for index in (0..cfg_block.statements.len()).chain(std::iter::once(usize::MAX)) {
                symbols.extend(
                    crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).symbol(var),
                );
            }
        }
        let mut acc = TypeLattice::unknown();
        for ((sym, ver), t) in fu.types.iter() {
            if *ver > 0 && symbols.contains(sym) {
                acc = type_join(&acc, t);
            }
        }
        acc
    }

    /// Elements are shared objects — a committed computed element
    /// keeps its intrep inside the list, and `lindex` retrieves it.
    #[test]
    fn list_builder_tracks_committed_element_and_lindex_retrieves_it() {
        use crate::compilation_unit::CompilationUnit;
        let cu = CompilationUnit::build_for(
            "set l [list [expr {2**20}] other]\nset first [lindex $l 0]",
            &registry(),
            false,
        );
        let l = type_of(&cu, "::top", "l");
        assert_eq!(l.tcl_type(), Some(TclType::List), "{l}");
        let elements = l.elements().expect("tracked elements").clone();
        assert_eq!(elements.known_len(), Some(2), "{l}");
        assert!(
            matches!(elements.shape_at(0), Some(s) if s.is_numeric_family()),
            "committed element 0 keeps its numeric intrep: {l}"
        );
        assert_eq!(
            elements.shape_at(1),
            None,
            "a literal word stays agnostic: {l}"
        );
        let first = type_of(&cu, "::top", "first");
        assert!(
            matches!(first.single_shape(), Some(s) if s.is_numeric_family()),
            "lindex retrieves the element's shape: {first}"
        );
    }

    /// Arity is a fact even with unknown element values: `[list $a $b]`
    /// provably has two elements.
    #[test]
    fn list_builder_tracks_arity_of_unknown_words() {
        use crate::compilation_unit::CompilationUnit;
        let cu = CompilationUnit::build_for(
            "proc f {a b} { set l [list $a $b]\n return $l }",
            &registry(),
            false,
        );
        let l = type_of(&cu, "::f", "l");
        assert_eq!(
            l.elements().and_then(Elements::known_len),
            Some(2),
            "arity of [list $a $b] is exactly 2: {l}"
        );
    }

    /// The old object-collection behaviour, now through general elements:
    /// `lappend`ing constructed objects yields `List<OBJECT(C)*>` and a
    /// `lassign` / `lindex` retrieval resolves the class.
    #[test]
    fn lassign_of_committed_object_list_types_targets() {
        use crate::compilation_unit::CompilationUnit;
        let cu = CompilationUnit::build_for(
            "oo::class create Foo {}\nset l [list [Foo new] [Foo new]]\nlassign $l a b",
            &registry(),
            false,
        );
        let a = type_of(&cu, "::top", "a");
        assert_eq!(a.tcl_type(), Some(TclType::Object), "{a}");
        assert_eq!(a.class_name(), Some("::Foo"), "{a}");
        let b = type_of(&cu, "::top", "b");
        assert_eq!(b.class_name(), Some("::Foo"), "{b}");
    }

    /// FP-SH-17 parity: `lassign` targets from a *pure-literal* list stay
    /// Overdefined — no shape claim is defensible for value-dependent
    /// conversion checks.
    #[test]
    fn lassign_of_literal_list_targets_stay_overdefined() {
        use crate::compilation_unit::CompilationUnit;
        let cu = CompilationUnit::build_for(
            "set point [list 1 2 3]\nlassign $point x y z",
            &registry(),
            false,
        );
        let x = type_of(&cu, "::top", "x");
        assert_eq!(x.kind(), TypeKind::Overdefined, "{x}");
    }

    /// A `lassign` target past the container's arity receives the empty
    /// string the command pads with.
    #[test]
    fn lassign_past_arity_is_empty_string() {
        use crate::compilation_unit::CompilationUnit;
        let cu = CompilationUnit::build_for(
            "set l [list [expr {1+1}]]\nlassign $l a b",
            &registry(),
            false,
        );
        let b = type_of(&cu, "::top", "b");
        assert_eq!(b.tcl_type(), Some(TclType::String), "{b}");
    }

    /// `dict create` of constructed objects → `Dict<OBJECT(C)*>`; `dict get`
    /// resolves the element class (the old object retrieval, generalised).
    #[test]
    fn dict_create_and_get_track_object_values() {
        use crate::compilation_unit::CompilationUnit;
        let cu = CompilationUnit::build_for(
            "oo::class create Pin {}\nset pins [dict create p1 [Pin new] p2 [Pin new]]\nset p [dict get $pins p1]",
            &registry(),
            false,
        );
        let pins = type_of(&cu, "::top", "pins");
        assert_eq!(pins.element_class(), Some("::Pin"), "{pins}");
        let p = type_of(&cu, "::top", "p");
        assert_eq!(p.class_name(), Some("::Pin"), "{p}");
    }

    /// `lappend var [C new]` evolves the variable's elements — the old
    /// `collection_element_class` behaviour through the registry's
    /// `VarElementsEffect`, aliased spellings included.
    #[test]
    fn lappend_object_evolves_element_class_through_rename() {
        use crate::compilation_unit::CompilationUnit;
        let cu = CompilationUnit::build_for(
            "oo::class create Foo {}\nset acc [list]\nlappend acc [Foo new]\nlappend acc [Foo new]",
            &registry(),
            false,
        );
        let acc = type_of(&cu, "::top", "acc");
        assert_eq!(acc.tcl_type(), Some(TclType::List), "{acc}");
        assert_eq!(acc.element_class(), Some("::Foo"), "{acc}");
    }

    /// A `foreach` element var over a tracked homogeneous list takes the
    /// element shape at its def (the loop-header phi separately joins the
    /// live-in as Overdefined, by the conditionally-assigned rule).
    #[test]
    fn foreach_var_takes_uniform_element_shape() {
        use crate::compilation_unit::CompilationUnit;
        let cu = CompilationUnit::build_for(
            "oo::class create Foo {}\nset l [list [Foo new] [Foo new]]\nforeach o $l { puts $o }",
            &registry(),
            false,
        );
        let fu = cu.function("::top").unwrap();
        let has_object_def = fu.types.iter().any(|((sym, ver), t)| {
            *ver > 0 && fu.ssa.var_name(*sym) == "o" && t.class_name() == Some("::Foo")
        });
        assert!(
            has_object_def,
            "some version of 'o' must carry the element class: {:?}",
            fu.types
        );
    }

    /// P5: constant-keyed array elements are independent variables — each
    /// carries its own type ("array elements behave as independent
    /// scalars"), and the conflated base claims nothing.
    #[test]
    fn array_elements_type_independently() {
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc f {} {\n    set arr(a) 5\n    set arr(b) \"hello world\"\n    return $arr(a)\n}",
            &tcl_registry::CommandRegistry::build_default(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let type_of = |name: &str| -> Vec<Option<TclType>> {
            fu.types
                .iter()
                .filter(|((sym, ver), _)| *ver > 0 && fu.ssa.var_name(*sym) == name)
                .map(|(_, t)| t.tcl_type())
                .collect()
        };
        assert!(
            type_of("arr(a)").iter().all(|t| *t == Some(TclType::Int)),
            "arr(a) is INT in every version: {:?}",
            fu.types
        );
        assert!(
            type_of("arr(b)")
                .iter()
                .all(|t| *t == Some(TclType::String)),
            "arr(b) is STRING independently: {:?}",
            fu.types
        );
        assert!(
            type_of("arr").iter().all(Option::is_none),
            "the base symbol claims no value type: {:?}",
            fu.types
        );
    }

    /// P5: a dynamic-key write is a may-write over every known element —
    /// the element's type JOINS with the written type (INT ⊔ INT stays
    /// INT; INT ⊔ STRING widens) instead of trusting either side.
    #[test]
    fn dynamic_key_write_joins_element_types() {
        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc f {k} {\n    set a(x) 5\n    set a($k) 9\n    return $a(x)\n}",
            &tcl_registry::CommandRegistry::build_default(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let final_is_int = fu.return_type.tcl_type() == Some(TclType::Int);
        assert!(
            final_is_int,
            "INT ⊔ INT across the may-write stays INT: {:?}",
            fu.types
        );

        let cu = crate::compilation_unit::CompilationUnit::build_for(
            "proc g {k} {\n    set a(x) 5\n    set a($k) \"s t\"\n    return $a(x)\n}",
            &tcl_registry::CommandRegistry::build_default(),
            false,
        );
        let fu = cu.function("::g").unwrap();
        let widened = fu.return_type.tcl_type() == Some(TclType::Int);
        assert!(
            !widened,
            "INT ⊔ STRING must not stay a single INT claim: {:?}",
            fu.types
        );
    }

    /// Dict-value semantics, verified on tclsh 8.6/9.0:
    /// a multi-key `dict set` stores a *dict* under the first key; `dict
    /// lappend` stores a *list*; `dict append` concatenates (intreps do
    /// not survive), with only object dispatch-identity flowing.
    #[test]
    fn dict_value_effects_match_oracle() {
        for profile in ["tcl8.6", "tcl9.0"] {
            assert_dict_value_effects(
                tcl_dialect::DialectProfile::find(profile).expect("native Tcl fixture"),
            );
        }
    }

    fn assert_dict_value_effects(profile: &'static tcl_dialect::DialectProfile) {
        let reg = tcl_registry::CommandRegistry::build_default().project_for_profile(profile);
        let elements_of = |src: &str, qname: &str, var: &str| -> Vec<Option<TclType>> {
            let cu = crate::compilation_unit::CompilationUnit::build_for(src, &reg, false);
            let fu = cu.function(qname).unwrap();
            let mut symbols = HashSet::new();
            symbols.extend(fu.ssa.var_symbol(var));
            for (&block, cfg_block) in &fu.cfg.blocks {
                for index in (0..cfg_block.statements.len()).chain(std::iter::once(usize::MAX)) {
                    symbols.extend(
                        crate::ssa::SsaSourceView::at_statement(&fu.ssa, block, index).symbol(var),
                    );
                }
            }
            fu.types
                .iter()
                .filter(|((sym, ver), _)| *ver > 0 && symbols.contains(sym))
                .map(|(_, t)| {
                    t.elements()
                        .and_then(Elements::uniform_shape)
                        .map(|s| s.coarse())
                })
                .collect()
        };
        // Multi-key set: the top-level value is a DICT, never the leaf shape.
        let multi = elements_of(
            "proc f {} {\n    set d {}\n    dict set d outer inner [expr {1}]\n    return $d\n}",
            "::f",
            "d",
        );
        assert!(
            multi.iter().flatten().all(|t| *t == TclType::Dict),
            "multi-key dict set stores a nested dict: {multi:?}"
        );
        // Single-key set keeps the leaf's shape.
        let single = elements_of(
            "proc f {} {\n    set d {}\n    dict set d k [expr {1}]\n    return $d\n}",
            "::f",
            "d",
        );
        assert!(
            single
                .iter()
                .any(|t| matches!(t, Some(TclType::Int | TclType::Numeric))),
            "single-key dict set keeps the leaf shape: {single:?}"
        );
        // lappend: the value is a LIST (elements unknown — the prior scalar
        // becomes element 0).
        let lap = elements_of(
            "proc f {} {\n    set d {}\n    dict lappend d k [expr {1}]\n    return $d\n}",
            "::f",
            "d",
        );
        assert!(
            lap.iter().flatten().all(|t| *t == TclType::List),
            "dict lappend listifies the value: {lap:?}"
        );
        // append: concatenation — a numeric argument's intrep must NOT
        // become the value's element fact.
        let app = elements_of(
            "proc f {} {\n    set d {}\n    dict append d k [expr {1}]\n    return $d\n}",
            "::f",
            "d",
        );
        assert!(
            !app.iter()
                .flatten()
                .any(|t| matches!(t, TclType::Int | TclType::Numeric)),
            "dict append must not claim a numeric value intrep: {app:?}"
        );
    }

    /// Bignum literals classify on the tower: a beyond-wide decimal is
    /// `Bignum` (coarse `Int`), never wrapped and never `String`.
    #[test]
    fn bignum_literal_classifies_on_the_tower() {
        assert_eq!(
            literal_type("18446744073709551616", NumberSyntax::Tcl90).single_shape(),
            Some(&TypeShape::Bignum)
        );
        assert_eq!(
            literal_type("0xFFFFFFFFFFFFFFFF", NumberSyntax::Tcl90).single_shape(),
            Some(&TypeShape::Bignum)
        );
        assert_eq!(
            literal_type("5", NumberSyntax::Tcl90).single_shape(),
            Some(&TypeShape::Int)
        );
        assert_eq!(
            literal_type("-9223372036854775808", NumberSyntax::Tcl90).single_shape(),
            Some(&TypeShape::Int),
            "i64::MIN still fits a wide"
        );
    }
}
