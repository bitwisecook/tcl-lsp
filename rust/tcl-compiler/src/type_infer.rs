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
    // `1.5`, `1e3`, `Inf`, `-Inf`: the shared grammar's floating spellings.
    if matches!(
        parse_whole_with(s, ParseFlags::for_syntax(numbers)),
        Some(Number::Double(_) | Number::Nan { .. })
    ) {
        return TypeLattice::of(TclType::Double);
    }
    // Case-insensitive boolean check (full spellings — the literal
    // classifier's would-commit convention; prefixes commit only at use).
    if tcl_syntax::boolean::is_boolean_full_word(s) {
        return TypeLattice::of(TclType::Boolean);
    }
    TypeLattice::of(TclType::String)
}

/// Decoded contents of one unchanged original literal argument.
/// This carries no object class, header, representation, SSA value or admission.
#[doc(hidden)]
pub struct OriginalLiteralArgumentContents {
    value: String,
    numbers: NumberSyntax,
}

impl OriginalLiteralArgumentContents {
    #[doc(hidden)]
    #[must_use]
    pub fn value(&self) -> &str {
        &self.value
    }

    /// Existing contents-type convention under the actual source's numerals.
    /// A numeric shape is not a physical Native integer representation.
    pub(crate) fn contents_type(&self) -> TypeLattice {
        literal_type(&self.value, self.numbers)
    }
}

/// Decode only a literal written argument under its retained Module and point
/// input. Missing ownership, substitutions, opaque bytes and NUL decline.
#[doc(hidden)]
pub fn original_literal_argument_contents(
    registry: &CommandRegistry,
    module: &crate::ir::Module,
    tokens: &crate::ir::CommandTokens,
    argument: usize,
) -> Option<OriginalLiteralArgumentContents> {
    if tokens.synthetic.is_some() || !tokens.words_align_with_argv_text() {
        return None;
    }
    let binding = tokens.source_binding.as_ref()?;
    let metadata = binding.original_invocation_metadata_for_module(tokens, module, registry)?;
    let input = metadata.source_analysis_input()?;
    let value = crate::registry_invocation::original_literal_argument_data_for_module(
        registry, module, tokens, argument,
    )?;
    Some(OriginalLiteralArgumentContents {
        value,
        numbers: input.unit_profile().grammar.numbers,
    })
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

/// Function namespace label for compatibility constructor advice.
fn function_namespace(qname: &str) -> String {
    tcl_syntax::naming::reported_qname_namespace(
        qname,
        tcl_syntax::naming::ReportedQNameNamespacePurpose::LiteralPrefix,
    )
}

/// Compatibility constructor advice in shared current/global candidate order.
/// The class roster supplies no competing-command, path or lifetime proof.
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
    if is_ctor_spelling
        && let Some(class) =
            crate::naming::resolve_command_with::<&str, _>(namespace, &[], command, |candidate| {
                known_classes.contains(candidate)
            })
    {
        return TypeLattice::object_of(class);
    }
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
    use tcl_syntax::expr::mathfunc::{MathResultClass, result_class};
    match result_class(function) {
        MathResultClass::Int => TypeLattice::of(TclType::Int),
        MathResultClass::Float => TypeLattice::of(TclType::Double),
        MathResultClass::Bool => TypeLattice::of(TclType::Boolean),
        // `depth` is the level of the enclosing `Call` node; its args are one
        // level deeper; `infer_expr_type` guards the cap itself.
        MathResultClass::Numeric => {
            let mut operands = args
                .iter()
                .map(|arg| infer_expr_type(arg, var_types, depth + 1, numbers));
            match operands.next() {
                Some(first) => operands.fold(first, |acc, next| type_join(&acc, &next)),
                None => TypeLattice::of(TclType::Numeric),
            }
        }
        MathResultClass::Any => TypeLattice::of(TclType::Numeric),
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
#[cfg(test)]
fn is_scope_alias_call(registry: &CommandRegistry, command: &str, args: &[String]) -> bool {
    crate::var_scoping::is_scope_alias_call(registry, command, args)
}

/// Widening advice from the same original selected invocation. Missing or
/// foreign metadata supplies no effect-free alias declaration or nominal type.
fn original_scope_alias_call(
    registry: &CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    statement: &Statement,
) -> Option<bool> {
    let metadata = metadata.filter(|metadata| metadata.matches_registry(registry))?;
    let Statement::Call {
        command,
        canonical_command,
        args,
        tokens,
        ..
    } = statement
    else {
        return Some(false);
    };
    // Explicit standalone source compatibility carries no Native receiver.
    if metadata.is_standalone()
        && tokens
            .as_ref()
            .is_none_or(|tokens| tokens.source_binding.is_none())
    {
        return Some(crate::var_scoping::is_scope_alias_call(
            registry,
            canonical_command.as_deref().unwrap_or(command),
            args,
        ));
    }
    let tokens = tokens.as_ref()?;
    let binding = tokens.source_binding.as_ref()?;
    let purpose = binding.original_invocation_source_purpose(tokens, metadata, registry)?;
    let selected = match purpose {
        crate::command_binding::OriginalInvocationSourcePurpose::Logical => {
            crate::registry_invocation::original_logical_operation_invocation_with_metadata_context(
                registry, metadata, tokens,
            )
        }
        crate::command_binding::OriginalInvocationSourcePurpose::Execution
        | crate::command_binding::OriginalInvocationSourcePurpose::Standalone
        | crate::command_binding::OriginalInvocationSourcePurpose::AuthoredSimulation => {
            crate::registry_invocation::resolved_handler_invocation_with_metadata_context(
                registry,
                Some(metadata),
                tokens,
            )
        }
    }?;
    Some(
        selected
            .facts
            .traits
            .contains(tcl_registry::Traits::CREATES_SCOPE_ALIAS),
    )
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
    /// Reached math-function dispatches of the same source inventory.
    math: &'a [crate::command_binding::SourceMathInvocation],
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
            .and_then(|symbol| self.ssa.cell_key(symbol).cloned())
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
            math: self.math,
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
        let math = crate::math_function_binding::ExpressionMathBindings::for_origin(
            ctx.math,
            Some(receipt.source()),
            Some(advice.expression_base),
        );
        return receipt
            .normal_result_representation(
                |node| {
                    let ExprNode::Var {
                        text, start, end, ..
                    } = node
                    else {
                        return None;
                    };
                    variables.get(&(text.clone(), *start, *end))?.tcl_type()
                },
                // Only a call whose dispatch reached a registry function
                // with no alias prefix has that function's result class.
                |node| {
                    let ExprNode::Call {
                        function, start, ..
                    } = node
                    else {
                        return None;
                    };
                    let call = math.resolved_call_for_value_analysis(function, *start)?;
                    call.prepended
                        .is_empty()
                        .then(|| tcl_syntax::expr::mathfunc::result_class(call.function))
                },
            )
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

/// A `[Class new …]` / `[Class create …]` word whose head names a class this
/// unit declares, typed as that class's object. This is a candidate type from
/// the written words, as a top-level constructor call's result is; it is not
/// the construction proof, which only `proved_construction_result` gives. A
/// registry command and a head bound to a builtin are never candidates.
fn source_class_construction_candidate<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    tokens: &crate::ir::CommandTokens,
) -> TypeLattice {
    fn literal(word: Option<&crate::ir::WordExpr>) -> Option<&str> {
        match word? {
            crate::ir::WordExpr::Literal { text, .. }
            | crate::ir::WordExpr::BracedLiteral { text, .. } => Some(text.as_str()),
            _ => None,
        }
    }
    let words = tokens.words();
    let (Some(head), Some(first)) = (literal(words.first()), literal(words.get(1))) else {
        return TypeLattice::overdefined();
    };
    if ctx.known_classes.is_empty()
        || ctx.registry.get(head).is_some()
        || tokens
            .source_binding
            .as_ref()
            .and_then(crate::command_binding::SourceInvocationBinding::proved_target)
            .is_some_and(|target| target.registry_backed)
    {
        return TypeLattice::overdefined();
    }
    constructor_object_type(
        ctx.registry,
        head,
        &[first],
        ctx.known_classes,
        ctx.namespace,
    )
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
    let candidate = source_class_construction_candidate(ctx, &tokens);
    if candidate.class_name().is_some() {
        return candidate;
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
    if contents.unknown_residual || ctx.ssa.cell_key(contents.reference.symbol).is_none() {
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
            math: ctx.math,
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
            DefTyping::PerDef(values) => ctx
                .ssa
                .cell_key(contents.reference.symbol)
                .and_then(|cell| values.get(cell))
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

/// Account for an empty declaration through its actual original source schema.
/// The descriptive roster checks inclusion only; it cannot select a factory.
fn empty_source_class(
    metadata: TypePropagationMetadata<'_>,
    tokens: &crate::ir::CommandTokens,
) -> Option<String> {
    let context = metadata.context?;
    let selected =
        crate::registry_invocation::original_structured_invocation_with_metadata_context(
            metadata.registry,
            context,
            tokens,
        )?;
    let spec = context
        .context()
        .resolve_spec(metadata.registry, &selected.facts.canonical_command)?;
    if !spec.traits.contains(tcl_registry::Traits::IS_OO_METACLASS)
        || spec.definition_body?.family != tcl_registry::definer::DefinerFamily::TclOo
    {
        return None;
    }
    let selector =
        crate::registry_invocation::invocation_word(selected.effective.words.get(1)?).literal()?;
    let method = CommandRegistry::manufacturer_method_for_descriptor(spec, selector)?;
    let body_index = usize::from(method.definition_body_at?);
    let name_index = usize::from(method.names_instance_at?);
    let name_word = selected.effective.written_argument(name_index)?;
    let body_word = selected.effective.written_argument(body_index)?;
    let words = tokens.words();
    let class = crate::registry_invocation::invocation_word(words.get(name_word + 1)?).literal()?;
    let body = crate::registry_invocation::invocation_word(words.get(body_word + 1)?).literal()?;
    (body.trim().is_empty() && selected.effective.words.len() == body_index + 2)
        .then(|| crate::naming::normalise_qualified_name(class))
}

fn caller_safe_factory_invocation(
    metadata: TypePropagationMetadata<'_>,
    tokens: &crate::ir::CommandTokens,
    _head: &str,
    _args: &[&str],
) -> bool {
    let registry = metadata.registry;
    let Some(context) = metadata
        .context
        .filter(|context| context.matches_registry(registry))
    else {
        return false;
    };
    let Some(binding) = tokens.source_binding.as_ref() else {
        return false;
    };
    let Some(original_config) = binding.original_lexer_config_for_tokens(tokens) else {
        return false;
    };
    if original_config.nested().normalized() != metadata.config.nested().normalized() {
        return false;
    }
    if let Some(input) = context.source_analysis_input() {
        if metadata.config.normalized() != input.lexer_config().normalized()
            || binding
                .original_invocation_metadata_for_input(tokens, input, registry)
                .is_none()
        {
            return false;
        }
    } else if !context.is_standalone()
        || binding
            .original_invocation_metadata_at_point(tokens, registry)
            .is_err()
    {
        return false;
    }
    if let Some(shape) = binding.original_default_manufacture_shape(tokens, registry) {
        return shape.is_argument_free_anonymous_shape();
    }
    // An actual class target lacking the default owner cannot be repaired
    // through its reporting name or an ordinary TclOO default grammar.
    if binding.proved_target().is_some_and(|target| {
        target.kind == crate::command_binding::BindingKind::Class || !target.registry_backed
    }) {
        return false;
    }
    // Metadata ownership retains unknown lookup as a separate axis. Select
    // the actual original invocation before reading its Registry facts; a
    // written reporting head cannot repair a missing or clobbered binding.
    let Some(selected) =
        crate::registry_invocation::original_structured_invocation_with_metadata_context(
            registry, context, tokens,
        )
    else {
        return false;
    };
    !selected.facts.traits.intersects(
        tcl_registry::Traits::EVALUATES_CODE
            | tcl_registry::Traits::CREATES_BARRIER
            | tcl_registry::Traits::CREATES_DYNAMIC_BARRIER,
    ) && !selected.facts.state_transitions.is_declared()
}

/// Prove the narrow straight-line case of empty source class factories.
/// Any other opaque invocation, binding transition, class body, or control
/// edge withdraws the proof for the whole function.
fn has_only_caller_safe_factories<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    metadata: TypePropagationMetadata<'_>,
    known_classes: &HashSet<String, S>,
) -> bool {
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
                crate::ir::SyntheticMarker::UnseenCall
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
                let empty_class = empty_source_class(metadata, tokens);
                if let Some(class) = empty_class {
                    if !known_classes.contains(&class) || !classes.insert(class) {
                        return false;
                    }
                    continue;
                }
                if !caller_safe_factory_invocation(
                    metadata,
                    tokens,
                    command,
                    &args.iter().map(String::as_str).collect::<Vec<_>>(),
                ) {
                    return false;
                }
            }
            Statement::AssignConst { .. } | Statement::AssignValue { .. } => {}
            _ => return false,
        }
        if !substitutions_have_only_caller_safe_factories(stmt, metadata) {
            return false;
        }
    }
    !classes.is_empty()
}

/// Check authentic nested operands without replacing their source owner.
fn substitutions_have_only_caller_safe_factories(
    stmt: &Statement,
    metadata: TypePropagationMetadata<'_>,
) -> bool {
    let Some(tokens) = stmt.tokens() else {
        // A typed statement without substitutions needs no nested factory
        // query. Recovered commands without original tokens cannot supply one.
        let inventory = crate::ir_helpers::evaluated_command_substitutions_with_metadata_context(
            stmt,
            metadata.registry,
            metadata.context,
            metadata.config,
        );
        return !inventory.opaque && inventory.all_commands().next().is_none();
    };
    let Some(context) = metadata
        .context
        .filter(|context| context.matches_registry(metadata.registry))
    else {
        return false;
    };
    let calls = if context.source_analysis_input().is_some() {
        crate::word_subst::checked_original_lifted_calls_with_metadata_context(
            tokens,
            metadata.config,
            metadata.registry,
            context,
        )
    } else if context.is_standalone() {
        crate::word_subst::checked_lifted_calls(tokens, metadata.config)
    } else {
        None
    };
    let Some(calls) = calls else {
        return false;
    };
    calls.into_iter().all(|call| {
        let Some(tokens) = call.tokens.as_ref() else {
            return false;
        };
        caller_safe_factory_invocation(
            metadata,
            tokens,
            &call.command,
            &call.args.iter().map(String::as_str).collect::<Vec<_>>(),
        )
    })
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

        // The fused `set` keeps its original words beside the statement, not on it.
        Statement::AssignExpr {
            expr, expr_base, ..
        } => DefTyping::Uniform(lowered_expression_result_type(
            ctx,
            ctx.tokens.or_else(|| ctx.source.source_tokens()),
            expr,
            *expr_base,
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

fn normal_store_type_def<S: std::hash::BuildHasher>(
    stmt: &Statement,
    ctx: &WordTypingCtx<'_, S>,
) -> Option<DefTyping> {
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
            return Some(DefTyping::Uniform(value_word_type(ctx, value, Some(word))));
        }
        if let Some(write) = normal.container_element_write(&binding.variable_context, ctx.registry)
        {
            let args = write
                .arguments
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>();
            return Some(DefTyping::Uniform(var_elements_effect_lattice(
                ctx,
                write.effect,
                &write.target,
                &args,
                write.argument_offset,
                Some(write.words),
            )));
        }
    }
    None
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
    if let Some(typing) = normal_store_type_def(stmt, ctx) {
        return typing;
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
/// *anywhere in the module* (`trace_facts`), or written by a callback script
/// of the module — is forced `Overdefined` here,
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

fn seed_clobbered_types<S: std::hash::BuildHasher>(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    sccp: &SccpResult,
    metadata: TypePropagationMetadata<'_>,
    known_classes: &HashSet<String, S>,
) -> HashMap<ValueKey, TypeLattice> {
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
    types
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
    escaping.extend(trace_facts.deferred_writes.names.iter().cloned());
    // Constructor heads written `[Foo new]` inside this function resolve
    // relative names against the function's own namespace.
    let namespace = function_namespace(&cfg.name);
    // Every nested query borrows the same complete source availability and
    // lexical policy; neither is reconstructed from the registry profile.
    let ctx = StatementTypingCtx {
        preparations: &cfg.expression_preparations,
        math: &cfg.implicit_math_invocations,
        ssa,
        context: Some(context),
        config: metadata.config,
        registry,
        known_classes,
        namespace: &namespace,
        values: &sccp.values,
        folded: &sccp.folded_types,
        escaping: &escaping,
        has_dynamic_variable_trace: trace_facts.has_dynamic_variable_trace
            || trace_facts.deferred_writes.any,
        numbers: metadata.numbers,
    };

    let mut types = seed_clobbered_types(cfg, ssa, sccp, metadata, known_classes);
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
                let mut exec_preds: Vec<BlockId> = preds.get(bn).map_or_default(|ps| {
                    ps.iter()
                        .filter(|p| sccp.executable_edges.contains(&(**p, *bn)))
                        .copied()
                        .collect()
                });
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

            // Statements, under the type refinements in force at the block.
            if type_refined_statements(&mut types, (sccp, *bn), ssa_block, &ctx) {
                changed = true;
            }
        }
    }

    types
}

/// Type one block's statements under the type refinements in force there
/// ([`types_at`]): a version a `string is` test or a numeric `==` proved a
/// type of reads as that type in the block, and as its own past it. Whether
/// a type moved.
fn type_refined_statements<S: std::hash::BuildHasher>(
    types: &mut HashMap<ValueKey, TypeLattice>,
    (sccp, block): (&SccpResult, BlockId),
    ssa_block: &crate::ssa::SsaBlock,
    ctx: &StatementTypingCtx<'_, S>,
) -> bool {
    let narrowed: Vec<(ValueKey, Option<TypeLattice>)> = types_at(types, sccp, block)
        .into_iter()
        .map(|(key, ty)| (key, types.insert(key, ty)))
        .collect();
    let changed = type_infer_process_statements(types, block, ssa_block, ctx);
    for (key, before) in narrowed {
        match before {
            Some(ty) => {
                types.insert(key, ty);
            }
            None => {
                types.remove(&key);
            }
        }
    }
    changed
}

/// The type each version a type refinement in force at `block` names holds
/// there ([`SccpResult::refinements_in`]): the type the test proved, or the
/// version's own where that is the proved one or narrower
/// ([`refined_type`]), the refinements of one version applied in turn. The
/// test's own conversion is why a type holds: `string is integer -strict
/// $x` and `$x == 1` leave `x` with a numeric representation whatever it
/// had.
fn types_at(
    types: &HashMap<ValueKey, TypeLattice>,
    sccp: &SccpResult,
    block: BlockId,
) -> HashMap<ValueKey, TypeLattice> {
    let mut narrowed: HashMap<ValueKey, TypeLattice> = HashMap::new();
    for refinement in sccp.refinements_in(block) {
        let tcl_registry::value_transfer::FactView::Domain(
            tcl_registry::value_transfer::DomainFact::Type {
                intrep: Some(proved),
                ..
            },
        ) = &refinement.fact
        else {
            continue;
        };
        // A version-0 key is what the frame entered with, a parameter's
        // argument among them, which no definition types: the live-in
        // root, as a φ reads it.
        let own = narrowed
            .get(&refinement.key)
            .or_else(|| types.get(&refinement.key))
            .cloned()
            .unwrap_or_else(|| {
                if refinement.key.1 == 0 {
                    TypeLattice::overdefined()
                } else {
                    TypeLattice::unknown()
                }
            });
        let refined = refined_type(&own, *proved);
        if refined != own {
            narrowed.insert(refinement.key, refined);
        }
    }
    narrowed
}

/// Per block, the type each version a type refinement in force there names
/// holds there ([`types_at`]), over the function's settled `types`: what a
/// use in the block reads, where it differs from the version's own type.
#[must_use]
pub(crate) fn types_in_force(
    types: &HashMap<ValueKey, TypeLattice>,
    sccp: &SccpResult,
) -> HashMap<(BlockId, ValueKey), TypeLattice> {
    sccp.refinements_at
        .keys()
        .flat_map(|&block| {
            types_at(types, sccp, block)
                .into_iter()
                .map(move |(key, ty)| ((block, key), ty))
        })
        .collect()
}

/// What a version typed `own` is typed where a refinement proved `proved`:
/// its own type where that is the proved one or narrower (an integer where
/// a number is proved), the proved type otherwise, and the optimistic
/// bottom for a version not typed yet.
fn refined_type(own: &TypeLattice, proved: TclType) -> TypeLattice {
    let within = |ty: TclType| {
        ty == proved || (proved == TclType::Numeric && matches!(ty, TclType::Int | TclType::Double))
    };
    match own.kind() {
        TypeKind::Unknown => own.clone(),
        TypeKind::Known if own.tcl_type().is_some_and(within) => own.clone(),
        _ => TypeLattice::of(proved),
    }
}

/// Shared, read-only context for [`type_infer_process_statements`].
struct StatementTypingCtx<'a, S: std::hash::BuildHasher> {
    preparations: &'a [crate::command_binding::SourceExpressionPreparation],
    /// Reached math-function dispatches of the same source inventory.
    math: &'a [crate::command_binding::SourceMathInvocation],
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
    /// Types produced by the selected shared route, independent of source-name lookup.
    folded: &'a HashMap<ValueKey, crate::value_transfer::FoldedType>,
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
            Statement::Call { defs, .. }
                if !defs.is_empty()
                    && original_scope_alias_call(ctx.registry, ctx.context, stmt)
                        != Some(false) =>
            {
                DefTyping::Uniform(TypeLattice::overdefined())
            }
            _ => {
                let word_ctx = WordTypingCtx {
                    preparations: ctx.preparations,
                    math: ctx.math,
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
            let def_type = if cell.is_none()
                || ssa_stmt.destruction_defs.contains(&var)
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
                            DefTyping::PerDef(map) => cell
                                .and_then(|cell| map.get(cell))
                                .cloned()
                                .unwrap_or_else(TypeLattice::overdefined),
                        };
                        type_join(&prev, &written)
                    }
                    None => TypeLattice::overdefined(),
                }
            } else {
                let inferred = match &inferred {
                    DefTyping::Uniform(t) => t.clone(),
                    // Positional element typing: a def the map does not
                    // name widens to Overdefined.
                    DefTyping::PerDef(map) => cell
                        .and_then(|cell| map.get(cell))
                        .cloned()
                        .unwrap_or_else(TypeLattice::overdefined),
                };
                refined_by_folded_type(inferred, ctx.folded.get(&key))
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

/// A definition's static type, refined by the folded type its evaluation
/// states: where the static typing knows nothing — a command with no
/// declared return type, a destructured target — the type the evaluation
/// proved for this call (its stated intrep, or the one the route
/// constructed) is the definition's. A known static type stands: it can
/// carry element facts the folded type does not, and the evaluation's own
/// type is the same fact for every shipped route.
fn refined_by_folded_type(
    inferred: TypeLattice,
    folded: Option<&crate::value_transfer::FoldedType>,
) -> TypeLattice {
    if inferred.kind() == TypeKind::Known {
        return inferred;
    }
    let proved = folded.and_then(|folded| {
        folded.intrep.or(match folded.representation {
            tcl_registry::value_transfer::RepresentationEvidence::Constructed(built) => Some(built),
            tcl_registry::value_transfer::RepresentationEvidence::Unknown => None,
        })
    });
    proved.map_or(inferred, TypeLattice::of)
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
                    math: &cfg.implicit_math_invocations,
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
mod tests;

#[cfg(test)]
mod reported_namespace_tests;

#[cfg(test)]
mod scope_alias_metadata_tests;
