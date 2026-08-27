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
use crate::depth_guard::MAX_EXPR_NODE_DEPTH;
use crate::expr_ast::{BinOp, ExprNode, UnaryOp};
use crate::ir::Statement;
use crate::naming::normalise_var_name;
use crate::sccp::SccpResult;
use crate::shimmer::hints::is_pure_intrep;
use crate::ssa::{SsaFunction, Symbol, ValueKey};
use crate::types::{
    Elements, MAX_EXACT_ELEMENTS, TypeKind, TypeLattice, TypeShape, join_elements, shape_join,
    type_join,
};
use crate::value_shapes::{is_pure_var_ref, parse_command_substitution};

// Float literal pattern: requires a decimal point so that forms like `1e3`
// (no `.`) are NOT classified as floats.
fn looks_like_float(s: &str) -> bool {
    let s = s.trim();
    s.contains('.') && s.parse::<f64>().is_ok()
}

/// The numeric-literal grammar of `registry`'s dialect profile.
///
/// The registry the lattice pipeline runs under is the dialect-selected one
/// (`registry_for_dialect`), so its profile is where the numeral grammar comes
/// from — the same source [`crate::tcl_expr_eval::FoldPolicy::from_registry`]
/// reads. A hand-assembled registry with no profile (unit tests, dialect-free
/// helpers) falls back to `Tcl90`, matching
/// [`crate::codegen::CodegenCtx::numbers`].
///
/// Explicit, never `tcl_syntax::number`'s thread-local ambient: the LSP types
/// documents of several dialects in one process, and ambient state would let one
/// document's grammar decide another's literals.
#[must_use]
fn numbers_of(registry: &CommandRegistry) -> NumberSyntax {
    registry.numbers()
}

/// The integer-tower shape of a Tcl integer literal under `numbers` —
/// [`TypeShape::Int`] when the value fits a wide (`i64`), [`TypeShape::Bignum`]
/// beyond it (`9223372036854775808`, `0xFFFFFFFFFFFFFFFF` — promotion is by
/// magnitude, never wrapping; oracle corpus in type-tracking.md). `None` when
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
/// `aggregate_object_types` does, at file-end granularity, for W308 (issue
/// #1013). Consumers that only use the type internally (SCCP shape
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

    // Subcommand commands: check sub's return_type.
    if !spec.subcommands.is_empty() {
        if let Some(sub_name) = args.first()
            && let Some(sub) = spec.resolve_subcommand(sub_name)
        {
            return match sub.return_type {
                Some(t) => TypeLattice::of(t),
                None => TypeLattice::overdefined(),
            };
        }
        // Unknown subcommand.
        return TypeLattice::overdefined();
    }

    match spec.return_type {
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
fn infer_expr_type(
    node: &ExprNode,
    var_types: &HashMap<String, TypeLattice>,
    depth: u32,
    numbers: NumberSyntax,
) -> TypeLattice {
    // Native-stack safety net (issue #996): `ExprNode` operator trees are
    // walked with one native frame per nesting level. Past the cap, give up
    // conservatively with `overdefined` — the same "any type / can't tell"
    // top this function already returns for opaque `Command`/`Raw` nodes, so
    // no caller narrows a type it shouldn't.
    if MAX_EXPR_NODE_DEPTH.exceeded(depth) {
        return TypeLattice::overdefined();
    }
    match node {
        ExprNode::Literal { text, .. } => expr_literal_type(text, numbers),

        ExprNode::String { .. } => TypeLattice::of(TclType::String),

        ExprNode::Var { name, .. } => {
            let base = normalise_var_name(name);
            var_types
                .get(base)
                .cloned()
                .unwrap_or_else(TypeLattice::unknown)
        }

        ExprNode::Binary {
            op, left, right, ..
        } => {
            match op {
                // BITWISE / shift → always Int (Tcl `expr` coerces the
                // operands to integers). Previously these were grouped
                // with arithmetic and degraded to Numeric.
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
        // — `sqrt($x)` is Double, `int(...)` is Int, etc.,
        // where they previously degraded to overdefined.
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
fn expr_call_type(
    function: &str,
    args: &[ExprNode],
    var_types: &HashMap<String, TypeLattice>,
    depth: u32,
    numbers: NumberSyntax,
) -> TypeLattice {
    // `depth` is the level of the enclosing `Call` node; its args are one
    // level deeper (issue #996 — `infer_expr_type` guards the cap itself).
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

/// The tracked [`TypeLattice`] of the SSA value `name` reads at this site, or
/// `None` when it is unversioned / untyped.
fn lookup_var_type(
    name: &str,
    uses: &HashMap<Symbol, u32>,
    types: &HashMap<ValueKey, TypeLattice>,
    ssa: &SsaFunction,
) -> Option<TypeLattice> {
    let sym = ssa.var_symbol(name)?;
    let ver = *uses.get(&sym)?;
    if ver == 0 {
        return None;
    }
    types.get(&(sym, ver)).cloned()
}

/// Shared, read-only context for the word-shape / element-inference helpers —
/// everything a value word's type depends on at one program point.
struct WordTypingCtx<'a, S: std::hash::BuildHasher> {
    uses: &'a HashMap<Symbol, u32>,
    /// The resolved context this function's registry answers under, when
    /// the registry carries a profile — the I4 binding proof for the
    /// spec-fact specialisations below (`None` for a hand-assembled,
    /// profile-less registry: the obligation is `NotRequired`).
    context: Option<&'a tcl_registry::model::ResolvedContext>,
    types: &'a HashMap<ValueKey, TypeLattice>,
    /// SCCP constants at this point — purity evidence (a numeric-typed
    /// literal is still a pure string) and constant list/index values for
    /// element facts.
    values: &'a HashMap<ValueKey, LatticeValue>,
    registry: &'a CommandRegistry,
    known_classes: &'a HashSet<String, S>,
    namespace: &'a str,
    ssa: &'a SsaFunction,
    /// The target release's numeric-literal grammar, derived once from the
    /// dialect-selected registry in [`propagate_types`] and threaded to every
    /// literal classifier — see [`numbers_of`].
    numbers: NumberSyntax,
}

impl<S: std::hash::BuildHasher> WordTypingCtx<'_, S> {
    /// The SCCP constant of `name` at this site, when tracked.
    fn const_of(&self, name: &str) -> Option<&LatticeValue> {
        let sym = self.ssa.var_symbol(name)?;
        let ver = *self.uses.get(&sym)?;
        if ver == 0 {
            return None;
        }
        self.values.get(&(sym, ver))
    }
}

/// The shape a builder argument *word* contributes as a container element,
/// or `None` when no shape claim is defensible for downstream checks.
///
/// Faithful to the runtime's by-reference elements (oracle corpus,
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
) -> Option<TypeShape> {
    let stripped = word.trim();
    if stripped.starts_with("{*}") {
        return None;
    }
    if is_pure_var_ref(stripped) {
        let name = normalise_var_name(stripped);
        let t = lookup_var_type(name, ctx.uses, ctx.types, ctx.ssa)?;
        let shape = t.single_shape()?.clone();
        if is_pure_intrep(shape.coarse(), ctx.const_of(name)) {
            return None;
        }
        return Some(shape);
    }
    if stripped.starts_with('[') && stripped.ends_with(']') {
        let shape = value_word_type(ctx, stripped).single_shape()?.clone();
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
) -> Option<TypeLattice> {
    match fact {
        ReturnElements::ListOfArgs { from } => {
            let words = args.get(usize::from(from)..).unwrap_or(&[]);
            Some(TypeLattice::of_shape(TypeShape::List(
                build_exact_elements(ctx, words),
            )))
        }
        ReturnElements::DictOfPairs { from } => {
            let words = args.get(usize::from(from)..).unwrap_or(&[]);
            // Values are the odd offsets of the key/value pairs.
            let value_words: Vec<&str> = words.iter().skip(1).step_by(2).copied().collect();
            Some(TypeLattice::of_shape(TypeShape::Dict(uniform_elements_of(
                ctx,
                &value_words,
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
            if !is_pure_var_ref(container) {
                return None;
            }
            let name = normalise_var_name(container);
            let t = lookup_var_type(name, ctx.uses, ctx.types, ctx.ssa)?;
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
            if !is_pure_var_ref(container) {
                return None;
            }
            let name = normalise_var_name(container);
            let elements = lookup_var_type(name, ctx.uses, ctx.types, ctx.ssa)?
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
) -> Elements {
    if words.iter().any(|w| w.trim().starts_with("{*}")) {
        return uniform_elements_of(ctx, words);
    }
    if words.len() > MAX_EXACT_ELEMENTS {
        return uniform_elements_of(ctx, words);
    }
    Elements::Exact(words.iter().map(|w| element_word_shape(ctx, w)).collect())
}

/// The uniform element bound of a word set: the single-shape join of every
/// word's shape, or no facts when any word is shapeless / unjoinable.
fn uniform_elements_of<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    words: &[&str],
) -> Elements {
    let mut acc: Option<TypeShape> = None;
    for word in words {
        // `{*}$expansion` contributes its *list's* element shapes, which are
        // unknown here — no uniform claim survives.
        let word = word.trim();
        let shape = if let Some(expanded) = word.strip_prefix("{*}") {
            let Some(TypeShape::List(elements)) = (if is_pure_var_ref(expanded) {
                lookup_var_type(normalise_var_name(expanded), ctx.uses, ctx.types, ctx.ssa)
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
            match element_word_shape(ctx, word) {
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
    if is_pure_var_ref(word)
        && let Some(LatticeValue::Const(ConstValue::Int(i))) =
            ctx.const_of(normalise_var_name(word))
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
) -> TypeLattice {
    let (container_ctor, value_words): (fn(Elements) -> TypeShape, &[&str]) = match effect {
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

    let prior = prior_container_elements(ctx, target);
    let evolved = match effect {
        VarElementsEffect::AppendsListElements { .. } => {
            append_list_elements(ctx, prior, value_words)
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
                uniform_elements_of(ctx, value_words)
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
        // object-class shapes flow into the bound (issue #797's
        // collection-of-objects pattern); anything else contributes no
        // element fact.
        VarElementsEffect::ExtendsDictValuesByName { .. } => {
            let incoming = uniform_elements_of(ctx, value_words);
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
    if let Some(t) = lookup_var_type(target, ctx.uses, ctx.types, ctx.ssa) {
        if let Some(e) = t.elements() {
            return Some(e.clone());
        }
        // A pure string constant parses to a known element count of pure
        // strings — `lappend` on `{a b}` starts from two `String` elements.
        if t.tcl_type() == Some(TclType::String)
            && let Some(LatticeValue::Const(ConstValue::String(text))) = ctx.const_of(target)
            && let Ok(parsed) = tcl_syntax::list::split_list(text)
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
) -> Elements {
    let appended = build_exact_elements(ctx, value_words);
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
) -> TypeLattice {
    let stripped = value.trim();
    // Pure variable reference: inherit source type.
    if is_pure_var_ref(stripped) {
        let name = normalise_var_name(stripped);
        if let Some(&ver) = ctx.ssa.var_symbol(name).and_then(|s| ctx.uses.get(&s))
            && ver > 0
        {
            return ctx
                .ssa
                .var_symbol(name)
                .and_then(|s| ctx.types.get(&(s, ver)))
                .cloned()
                .unwrap_or_else(TypeLattice::unknown);
        }
        return TypeLattice::unknown();
    }
    // Command substitution: [cmd ...].
    if stripped.starts_with('[')
        && stripped.ends_with(']')
        && let Some((cmd, args)) = parse_command_substitution(stripped)
    {
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        // The registry's result↔element fact refines the declared return
        // type: `[list …]` builds per-position elements, `[lindex $l 0]` /
        // `[dict get $d k]` retrieves the container's tracked element shape
        // (subsuming the old object-only collection retrieval — an
        // `Object(class)` element shape IS the `OBJECT(class)` result).
        if let Some(resolved) = tcl_registry::model::resolve_invocation_in_context(
            ctx.registry,
            ctx.context,
            &cmd,
            &arg_refs,
        ) && let Some(fact) = resolved.semantics.return_elements
        {
            // The fact's indices are relative to after the subcommand word
            // when one matched (`dict get $d k` counts from `$d`).
            let elem_args = if resolved.subcommand.is_resolved() {
                arg_refs.get(1..).unwrap_or(&[])
            } else {
                &arg_refs[..]
            };
            if let Some(t) = return_elements_lattice(ctx, fact, elem_args) {
                return t;
            }
        }
        return return_type_for_command(
            ctx.registry,
            &cmd,
            &arg_refs,
            ctx.known_classes,
            ctx.namespace,
        );
    }
    // String interpolation or complex value.
    if value.contains('$') || value.contains('[') {
        return TypeLattice::of(TclType::String);
    }
    literal_type(value, ctx.numbers)
}

/// How one statement types the variable(s) it defines.
enum DefTyping {
    /// One lattice applied to every def of the statement.
    Uniform(TypeLattice),
    /// Positional element typing (`lassign`, `foreach` element vars): each
    /// def takes its own lattice; a def absent from the map widens to
    /// `Overdefined`.
    PerDef(HashMap<String, TypeLattice>),
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

        Statement::AssignExpr { expr, .. } => {
            // Build a name→TypeLattice map for variables used in the expression.
            let var_types: HashMap<String, TypeLattice> = ctx
                .uses
                .iter()
                .filter_map(|(&sym, &ver)| {
                    if ver == 0 {
                        return None;
                    }
                    let t = ctx.types.get(&(sym, ver))?;
                    Some((ctx.ssa.var_name(sym).to_owned(), t.clone()))
                })
                .collect();
            DefTyping::Uniform(infer_expr_type(expr, &var_types, 0, ctx.numbers))
        }

        Statement::AssignValue { value, .. } => DefTyping::Uniform(value_word_type(ctx, value)),

        Statement::Incr { .. } => DefTyping::Uniform(TypeLattice::of(TclType::Int)),

        Statement::Call {
            command,
            canonical_command,
            args,
            defs,
            foreach_groups,
            ..
        } if !defs.is_empty() => {
            // Resolve the source spelling through the lowerer's
            // `canonical_command` snapshot (an `interp alias` / `rename`
            // target) so a renamed or aliased builtin — `rename set myset` /
            // `interp alias {} myset {} set` — is typed by the *real* command's
            // registry spec, not left as an unknown `Call` (OVERDEFINED).
            let canon = canonical_command.as_deref().unwrap_or(command);
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            // A canonically-`set` store keeps its runtime `Call` shape for
            // codegen — an aliased / renamed command is not inline-foldable,
            // its binding may change by call time — but for the type lattice
            // its single def takes the *value word's* intrep verbatim, exactly
            // as the un-aliased [`Statement::AssignValue`] path does. Keyed off
            // the canonical command's `Set` lowering hook (the registry's own
            // "this is a value-passthrough store" fact), never the source
            // spelling. The two-arg / single-def guard restricts this to the
            // `set VAR VALUE` setter shape (no `interp alias` prepended args
            // shifting the value word out of `args[1]`, and not the one-arg
            // getter, which has no def).
            if defs.len() == 1
                && arg_refs.len() == 2
                && ctx.registry.get(canon).and_then(|s| s.lowering_hook)
                    == Some(tcl_registry::hooks::LoweringHookId::Set)
            {
                return DefTyping::Uniform(value_word_type(ctx, arg_refs[1]));
            }

            let resolved = tcl_registry::model::resolve_invocation_in_context(
                ctx.registry,
                ctx.context,
                canon,
                &arg_refs,
            );

            // An in-place element write (`lappend VAR v…`, `dict set VAR … v`)
            // evolves the target's container elements — the registry's
            // `VarElementsEffect` fact, generalising the old object-only
            // element-class harvesting to every element shape.
            if let Some(effect) = resolved
                .as_ref()
                .and_then(|resolved| resolved.semantics.var_elements_effect)
            {
                let base = usize::from(
                    resolved
                        .as_ref()
                        .is_some_and(|resolved| resolved.subcommand.is_resolved()),
                );
                if let Some(target) = arg_refs.get(base) {
                    return DefTyping::Uniform(var_elements_effect_lattice(
                        ctx, effect, target, &arg_refs, base,
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
            let typing = resolved
                .as_ref()
                .map_or(VarWriteTyping::ReturnValue, |resolved| {
                    resolved.semantics.var_write_typing
                });
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
                VarWriteTyping::ReturnValue => DefTyping::Uniform(return_type_for_command(
                    ctx.registry,
                    canon,
                    &arg_refs,
                    ctx.known_classes,
                    ctx.namespace,
                )),
                VarWriteTyping::Fixed(t) => DefTyping::Uniform(TypeLattice::of(t)),
                VarWriteTyping::Destructured => DefTyping::Uniform(TypeLattice::overdefined()),
                VarWriteTyping::ElementsOf { container_arg } => elements_of_def_typing(
                    ctx,
                    &arg_refs,
                    defs,
                    foreach_groups.as_deref(),
                    container_arg,
                ),
            }
        }

        // `ExprEval`, `Barrier`, and structured statements that survive as
        // statements (before CFG construction in some paths) all lack a
        // resolvable result type here — treat them conservatively as
        // overdefined.
        _ => DefTyping::Uniform(TypeLattice::overdefined()),
    }
}

/// Positional element typing for a [`VarWriteTyping::ElementsOf`] writer.
///
/// Two shapes reach here:
/// - **`lassign $l a b …`** (no `foreach_groups`): target `i` takes element
///   `i` of the container. A tracked `Exact` container types each target
///   from its position — a position past the container's arity is the empty
///   string `lassign` pads with (`String`), an untracked position widens to
///   `Overdefined` (the pre-element-tracking behaviour, issue #867).
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
    let mut per_def: HashMap<String, TypeLattice> = HashMap::new();

    if let Some(groups) = foreach_groups {
        let mut def_cursor = 0usize;
        for (group, &nvars) in groups.iter().enumerate() {
            let group_defs = defs.get(def_cursor..def_cursor + nvars).unwrap_or(&[]);
            def_cursor += nvars;
            let container_shape = args.get(group).and_then(|w| container_word_shape(ctx, w));
            for (j, def) in group_defs.iter().enumerate() {
                per_def.insert(
                    def.clone(),
                    foreach_var_lattice(container_shape.as_ref(), nvars, j),
                );
            }
        }
        return DefTyping::PerDef(per_def);
    }

    // lassign: targets follow the container word.
    let container = args.get(usize::from(container_arg));
    let elements = container
        .filter(|w| is_pure_var_ref(w))
        .and_then(|w| lookup_var_type(normalise_var_name(w), ctx.uses, ctx.types, ctx.ssa))
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
        per_def.insert(def.clone(), lattice);
    }
    DefTyping::PerDef(per_def)
}

/// The container shape a `foreach` group's list word denotes, when tracked.
fn container_word_shape<S: std::hash::BuildHasher>(
    ctx: &WordTypingCtx<'_, S>,
    word: &str,
) -> Option<TypeShape> {
    let word = word.trim();
    if is_pure_var_ref(word) {
        return lookup_var_type(normalise_var_name(word), ctx.uses, ctx.types, ctx.ssa)
            .and_then(|t| t.single_shape().cloned());
    }
    if word.starts_with('[') && word.ends_with(']') {
        return value_word_type(ctx, word).single_shape().cloned();
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

/// Run type propagation over one SSA function.
///
/// Returns a map from `(variable_name, ssa_version)` to inferred
/// `TypeLattice`. Values absent from the map are implicitly `Unknown`.
///
/// Every def of a name [`crate::sccp::is_externally_mutable`] considers
/// externally mutable — fully namespace-qualified, aliased via `global`/
/// `variable`/`upvar`/traced *within this function* ([`crate::var_observability::
/// analyse_var_observability`]), named by `extra_global_escaping` (the
/// whole-module `global`-declaration scan for the *top-level* unit — see
/// [`crate::var_observability::scan_module_global_names`]), or traced
/// *anywhere in the module* (`trace_facts`) — is forced `Overdefined` here,
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
    let preds = cfg.predecessors();
    let order = crate::sccp::cfg_order(cfg);
    let mut escaping =
        crate::var_observability::analyse_var_observability(cfg, registry).escaping_var_names();
    if !extra_global_escaping.is_empty() {
        escaping.extend(extra_global_escaping.iter().cloned());
    }
    escaping.extend(trace_facts.traced_variables.iter().cloned());
    // Constructor heads written `[Foo new]` inside this function resolve
    // relative names against the function's own namespace.
    let namespace = function_namespace(&cfg.name);
    // The I4 binding proof for spec-fact specialisation: the dialect-
    // selected registry's own environment context (a profile-less
    // registry carries no obligation).
    let generation = registry
        .profile()
        .map(crate::environment_ingress::context_for_profile);
    let ctx = StatementTypingCtx {
        ssa,
        context: generation
            .as_deref()
            .map(tcl_registry::model::ContextRegistry::context),
        registry,
        known_classes,
        namespace: &namespace,
        values: &sccp.values,
        escaping: &escaping,
        has_dynamic_variable_trace: trace_facts.has_dynamic_variable_trace,
        numbers: numbers_of(registry),
    };

    let mut types: HashMap<ValueKey, TypeLattice> = HashMap::new();

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
                        let ver = phi.incoming.get(pred).copied().unwrap_or(0);
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
            if type_infer_process_statements(&mut types, ssa_block, &ctx) {
                changed = true;
            }
        }
    }

    types
}

/// Shared, read-only context for [`type_infer_process_statements`].
struct StatementTypingCtx<'a, S: std::hash::BuildHasher> {
    ssa: &'a SsaFunction,
    /// See [`WordTypingCtx::context`].
    context: Option<&'a tcl_registry::model::ResolvedContext>,
    registry: &'a CommandRegistry,
    known_classes: &'a HashSet<String, S>,
    namespace: &'a str,
    /// SCCP constants — purity evidence and constant list/index values for
    /// the element-inference helpers (see [`WordTypingCtx::values`]).
    values: &'a HashMap<ValueKey, LatticeValue>,
    /// Names [`crate::sccp::is_externally_mutable`] should treat as
    /// unconditionally aliased/escaping (per-function `analyse_var_observability`
    /// union'd with the caller's whole-module `extra_global_escaping` and
    /// `trace_facts.traced_variables`).
    escaping: &'a HashSet<String>,
    has_dynamic_variable_trace: bool,
    /// The target release's numeric-literal grammar — see [`numbers_of`].
    numbers: NumberSyntax,
}

/// Evaluate each statement's defs for one block, forcing every def of a name
/// [`crate::sccp::is_externally_mutable`] considers externally mutable to
/// `Overdefined` regardless of what its own literal/expression would
/// otherwise infer. Returns `true` if any lattice value changed. Extracted
/// from [`propagate_types`], mirroring [`crate::sccp::sccp_process_statements`]'s
/// shape for the (separate) constant-folding lattice.
fn type_infer_process_statements<S: std::hash::BuildHasher>(
    types: &mut HashMap<ValueKey, TypeLattice>,
    ssa_block: &crate::ssa::SsaBlock,
    ctx: &StatementTypingCtx<'_, S>,
) -> bool {
    let mut changed = false;
    for ssa_stmt in &ssa_block.statements {
        let stmt = &ssa_stmt.statement;
        // A barrier widens every def to OVERDEFINED (it may have mutated
        // them arbitrarily); a scope-alias declaration (`global`/`variable`/
        // `upvar`/`namespace upvar`) likewise widens its defs — the
        // imported variable's intrep is external and unknown.
        let inferred = match stmt {
            Statement::Barrier { .. } => DefTyping::Uniform(TypeLattice::overdefined()),
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
                    uses: &ssa_stmt.uses,
                    context: ctx.context,
                    types,
                    values: ctx.values,
                    registry: ctx.registry,
                    known_classes: ctx.known_classes,
                    namespace: ctx.namespace,
                    ssa: ctx.ssa,
                    numbers: ctx.numbers,
                };
                evaluate_type_def(stmt, &word_ctx)
            }
        };
        // An element write's base def carries no value type of its own —
        // it refreshes `arr` for whole-array readers only (mirrors SCCP).
        let element_write_base = match &ssa_stmt.statement {
            Statement::AssignConst { name, .. }
            | Statement::AssignExpr { name, .. }
            | Statement::AssignValue { name, .. }
            | Statement::Incr { name, .. }
                if name.contains('(') =>
            {
                ctx.ssa.var_symbol(crate::naming::normalise_var_name(name))
            }
            _ => None,
        };
        for (&var, &ver) in &ssa_stmt.defs {
            let key = (var, ver);
            let old = types
                .get(&key)
                .cloned()
                .unwrap_or_else(TypeLattice::unknown);
            let name = ctx.ssa.var_name(var);
            let def_type = if crate::sccp::is_externally_mutable(
                name,
                ctx.escaping,
                ctx.has_dynamic_variable_trace,
            ) || element_write_base == Some(var)
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
                                .get(name)
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
                        .get(name)
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
/// SSA reaching-defs aren't tracked at terminators, so a `return $x`
/// (or `return [expr {$x}]`) joins over *every* known version of each
/// name — a sound over-approximation.
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
    registry: &CommandRegistry,
    known_classes: &HashSet<String, S>,
    ssa: &SsaFunction,
) -> TypeLattice {
    let namespace = function_namespace(&cfg.name);
    // The registry is the dialect-selected one, so it carries the numeral
    // grammar the return literals must be read under (see [`numbers_of`]).
    let numbers = numbers_of(registry);
    // Collapse the versioned type map to a name-keyed map by joining
    // every version of each name — the over-approximation noted above.
    let mut var_types: HashMap<String, TypeLattice> = HashMap::new();
    for ((sym, _ver), t) in types {
        var_types
            .entry(ssa.var_name(*sym).to_owned())
            .and_modify(|acc| *acc = type_join(acc, t))
            .or_insert_with(|| t.clone());
    }

    let mut result: Option<TypeLattice> = None;
    for (bn, block) in &cfg.blocks {
        if !sccp.executable_blocks.contains(bn) {
            continue;
        }
        let t = match &block.terminator {
            Some(Terminator::Return { value, expr, .. }) => {
                if let Some(expr) = expr {
                    infer_expr_type(expr, &var_types, 0, numbers)
                } else if let Some(value) = value {
                    infer_return_value_type(
                        value,
                        &var_types,
                        registry,
                        known_classes,
                        &namespace,
                        numbers,
                    )
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

/// Infer the type of a `return`'s textual value, following the
/// `Statement::AssignValue` arm of [`evaluate_type_def`] but keyed on
/// the version-collapsed `var_types` map.
fn infer_return_value_type<S: std::hash::BuildHasher>(
    value: &str,
    var_types: &HashMap<String, TypeLattice>,
    registry: &CommandRegistry,
    known_classes: &HashSet<String, S>,
    namespace: &str,
    numbers: NumberSyntax,
) -> TypeLattice {
    let stripped = value.trim();
    // Pure variable reference: inherit the source type.
    if is_pure_var_ref(stripped) {
        let name = normalise_var_name(stripped);
        return var_types
            .get(name)
            .cloned()
            .unwrap_or_else(TypeLattice::unknown);
    }
    // Command substitution: `[cmd ...]`.
    if stripped.starts_with('[')
        && stripped.ends_with(']')
        && let Some((cmd, args)) = parse_command_substitution(stripped)
    {
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        return return_type_for_command(registry, &cmd, &arg_refs, known_classes, namespace);
    }
    // String interpolation or other complex value.
    if value.contains('$') || value.contains('[') {
        return TypeLattice::of(TclType::String);
    }
    literal_type(value, numbers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::{Block, Function};
    use crate::ir::Statement;
    use crate::ssa::{Phi, SsaBlock, SsaFunction, SsaStatement};
    use std::collections::HashSet;
    use tcl_lexer::Span;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default()
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
            uses: &uses,
            context: None,
            types: &types,
            values: &values,
            registry,
            known_classes: &known_classes,
            namespace: "::",
            ssa,
            numbers: numbers_of(registry),
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
            quoted_uses: std::collections::HashSet::new(),
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
        // Issue #867 core regression: a *single*-target `lassign $l x` was the
        // case the old `defs.len() > 1` heuristic missed — one write fell
        // through to `lassign`'s `List` return type, so `x` was typed LIST and
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
        // Regression (PR #885 review): a call that writes SEVERAL variables
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
    /// distinct from its return type (issue #867).
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
        let cu = CompilationUnit::build_for("gets $ch line", &registry(), false);
        let fu = cu.function("::top").unwrap();
        assert!(
            any_known(fu, "line", TclType::String),
            "gets target must be Known String: {:?}",
            fu.types
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
    /// as `set x 5` would, so the renamed/aliased store is no longer an opaque
    /// OVERDEFINED `Call`.
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

    /// A `dict set VAR k [Class new]` collection retrieved by `dict get` types
    /// the element as the class (issue #797 `SpiceGenTcl` `Pins` shape).
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
        assert!(widened, "mixed-class dict must drop its element class");
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
    fn infer_str_dialect(src: &str, dialect: Option<&tcl_dialect::DialectProfile>) -> TypeLattice {
        let node = crate::parse_expr(src, dialect.map(|profile| profile.name));
        infer_expr_type(
            &node,
            &HashMap::new(),
            0,
            crate::intervals::numbers_for_dialect(dialect),
        )
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

    /// Regression coverage for issue #996: `infer_expr_type` (and its
    /// mutually-recursive helper `expr_call_type`) recurse once per
    /// `ExprNode` operator-tree level, with no depth cap before this fix.
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
    fn scope_alias_def_widens_to_overdefined() {
        use crate::compilation_unit::CompilationUnit;
        // `variable counter` imports an externally-determined variable; its
        // def must be OVERDEFINED, not the nominal return type of `variable`.
        let cu = CompilationUnit::build_for(
            "proc ::f {} { variable counter\n return $counter }",
            &registry(),
            false,
        );
        let fu = cu.function("::f").unwrap();
        let widened = fu.types.iter().any(|((n, _), t)| {
            fu.ssa.var_name(*n) == "counter" && t.kind() == TypeKind::Overdefined
        });
        assert!(
            widened,
            "scope-aliased 'counter' should be OVERDEFINED: {:?}",
            fu.types
        );
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

    // --- P3: registry-driven container element inference (type-tracking.md) ---

    /// Helper: the joined lattice of every version of `var` in `func`.
    fn type_of(
        cu: &crate::compilation_unit::CompilationUnit,
        func: &str,
        var: &str,
    ) -> TypeLattice {
        let fu = cu.function(func).unwrap();
        let mut acc = TypeLattice::unknown();
        for ((sym, ver), t) in fu.types.iter() {
            if *ver > 0 && fu.ssa.var_name(*sym) == var {
                acc = type_join(&acc, t);
            }
        }
        acc
    }

    /// Oracle: elements are shared objects — a committed computed element
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
    /// carries its own type (the oracle's "array elements behave as
    /// independent scalars"), and the conflated base claims nothing.
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
        let final_is_int = fu.types.iter().any(|((sym, ver), t)| {
            *ver == 2 && fu.ssa.var_name(*sym) == "a(x)" && t.tcl_type() == Some(TclType::Int)
        });
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
        let widened = fu.types.iter().any(|((sym, ver), t)| {
            *ver == 2 && fu.ssa.var_name(*sym) == "a(x)" && t.tcl_type() == Some(TclType::Int)
        });
        assert!(
            !widened,
            "INT ⊔ STRING must not stay a single INT claim: {:?}",
            fu.types
        );
    }

    /// Review-pinned dict-value semantics (tclsh 8.6/9.0 verified):
    /// a multi-key `dict set` stores a *dict* under the first key; `dict
    /// lappend` stores a *list*; `dict append` concatenates (intreps do
    /// not survive), with only object dispatch-identity flowing (#797).
    #[test]
    fn dict_value_effects_match_oracle() {
        let reg = tcl_registry::CommandRegistry::build_default();
        let elements_of = |src: &str, qname: &str, var: &str| -> Vec<Option<TclType>> {
            let cu = crate::compilation_unit::CompilationUnit::build_for(src, &reg, false);
            let fu = cu.function(qname).unwrap();
            fu.types
                .iter()
                .filter(|((sym, ver), _)| *ver > 0 && fu.ssa.var_name(*sym) == var)
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
