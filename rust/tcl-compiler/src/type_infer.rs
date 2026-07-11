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

use tcl_registry::dialects::DialectSet;
use tcl_registry::{CommandRegistry, TclType, Traits, VarWriteTyping};

use crate::cfg::{BlockId, Function as CfgFunction, Terminator};
use crate::expr_ast::{BinOp, ExprNode, UnaryOp};
use crate::ir::Statement;
use crate::naming::normalise_var_name;
use crate::sccp::SccpResult;
use crate::ssa::{SsaFunction, Symbol, ValueKey};
use crate::types::{TypeKind, TypeLattice, type_join};
use crate::value_shapes::{is_pure_var_ref, parse_command_substitution};

// Float literal pattern: requires a decimal point so that forms like `1e3`
// (no `.`) are NOT classified as floats.
fn looks_like_float(s: &str) -> bool {
    let s = s.trim();
    s.contains('.') && s.parse::<f64>().is_ok()
}

const BOOL_LITERALS: &[&str] = &["true", "false", "yes", "no", "on", "off"];

/// True when `s` is a Tcl integer literal: decimal, or a `0x`/`0X` hex or
/// `0b`/`0B` binary form (each optionally signed).  Hex/binary store an INT
/// intrep (`set n 0x80; incr n` is one clean parse, not per-iteration
/// shimmer), while `0o` octal stays STRING (the set-statement classifier
/// excludes it).
#[must_use]
fn is_tcl_int_literal(s: &str) -> bool {
    if s.parse::<i64>().is_ok() {
        return true;
    }
    let body = s.strip_prefix(['+', '-']).unwrap_or(s);
    if let Some(h) = body.strip_prefix("0x").or_else(|| body.strip_prefix("0X")) {
        return !h.is_empty() && h.bytes().all(|c| c.is_ascii_hexdigit());
    }
    if let Some(b) = body.strip_prefix("0b").or_else(|| body.strip_prefix("0B")) {
        return !b.is_empty() && b.bytes().all(|c| matches!(c, b'0' | b'1'));
    }
    false
}

/// Classify a literal string as its Tcl intrep type (set-statement context).
fn literal_type(text: &str) -> TypeLattice {
    let s = text.trim();
    if is_tcl_int_literal(s) {
        return TypeLattice::of(TclType::Int);
    }
    if looks_like_float(s) {
        return TypeLattice::of(TclType::Double);
    }
    // Case-insensitive boolean check.
    if BOOL_LITERALS.contains(&s.to_ascii_lowercase().as_str()) {
        return TypeLattice::of(TclType::Boolean);
    }
    TypeLattice::of(TclType::String)
}

/// Classify a literal's type in **expr context**.
///
/// The expr parser tokenises every integer spelling — decimal, hex
/// (`0xff`), octal (`0o15`), and binary (`0b1010`) — as an integer
/// (`Tcl_GetInt` accepts them), so they all map to `Int`. This is the key
/// divergence from the set-statement [`literal_type`], where `0o…` stays
/// `String` (its canonical stringified intrep differs from the source
/// text). An unrecognised literal degrades to `Numeric` — an `expr`
/// always yields a number — rather than to `String`.
fn expr_literal_type(text: &str) -> TypeLattice {
    let s = text.trim();
    let low = s.to_ascii_lowercase();
    // Boolean first.
    if BOOL_LITERALS.contains(&low.as_str()) {
        return TypeLattice::of(TclType::Boolean);
    }
    // Every integer-form spelling tokenises to int in expr context.
    if low.starts_with("0x") || low.starts_with("0o") || low.starts_with("0b") {
        return TypeLattice::of(TclType::Int);
    }
    if s.parse::<i64>().is_ok() {
        return TypeLattice::of(TclType::Int);
    }
    if s.parse::<f64>().is_ok() {
        return TypeLattice::of(TclType::Double);
    }
    TypeLattice::of(TclType::Numeric)
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

/// Type a `TclOO` / snit constructor call (`Foo new` / `Foo create x` /
/// `Foo %AUTO%` / `Widget .path`) as `OBJECT(class)` when its head resolves
/// to a known class, else `OVERDEFINED`.  The relative head is resolved as-is,
/// `::`-prefixed, and against the call-site `namespace` (so `[Foo new]` inside
/// `namespace eval ns` types as `OBJECT(::ns::Foo)`).
fn constructor_object_type<S: std::hash::BuildHasher>(
    command: &str,
    args: &[&str],
    known_classes: &HashSet<String, S>,
    namespace: &str,
) -> TypeLattice {
    let is_ctor_spelling = args
        .first()
        .is_some_and(|a| matches!(*a, "new" | "create") || *a == "%AUTO%" || a.starts_with('.'));
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
        return constructor_object_type(command, args, known_classes, namespace);
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
fn infer_expr_type(node: &ExprNode, var_types: &HashMap<String, TypeLattice>) -> TypeLattice {
    match node {
        ExprNode::Literal { text, .. } => expr_literal_type(text),

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
                | BinOp::MatchesGlob
                | BinOp::MatchesRegex => TypeLattice::of(TclType::Boolean),

                // ARITHMETIC / DIVISION → `_arithmetic_result` over the
                // operand types, but only when both are `Known`;
                // otherwise Numeric.
                BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Mod | BinOp::Pow => {
                    let lt = infer_expr_type(left, var_types);
                    let rt = infer_expr_type(right, var_types);
                    if lt.kind == TypeKind::Known && rt.kind == TypeKind::Known {
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
            UnaryOp::Neg | UnaryOp::Pos => infer_expr_type(operand, var_types),
            UnaryOp::BitNot => TypeLattice::of(TclType::Int),
            UnaryOp::Not | UnaryOp::WordNot => TypeLattice::of(TclType::Boolean),
        },

        ExprNode::Ternary {
            true_branch,
            false_branch,
            ..
        } => {
            let tt = infer_expr_type(true_branch, var_types);
            let ft = infer_expr_type(false_branch, var_types);
            type_join(&tt, &ft)
        }

        // Math-function calls resolve through the expr-function table
        // — `sqrt($x)` is Double, `int(...)` is Int, etc.,
        // where they previously degraded to overdefined.
        ExprNode::Call { function, args, .. } => expr_call_type(function, args, var_types),

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
    match (lt.tcl_type, rt.tcl_type) {
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
) -> TypeLattice {
    // Identity: `abs` preserves the operand type (Int fallback).
    if function == "abs" {
        return match args.first() {
            Some(a) => infer_expr_type(a, var_types),
            None => TypeLattice::of(TclType::Int),
        };
    }
    // Variadic join: `max` / `min` join all operand types.
    if function == "max" || function == "min" {
        let mut it = args.iter();
        return match it.next() {
            Some(first) => {
                let mut acc = infer_expr_type(first, var_types);
                for a in it {
                    acc = type_join(&acc, &infer_expr_type(a, var_types));
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

/// True when `command` (with `args`) creates a scope alias — `global`,
/// `variable`, `upvar`, or the `namespace upvar` compound.
///
/// Such a statement imports an externally-determined variable whose intrep
/// lives in another scope, so its def must widen to `Overdefined` rather
/// than take the command's nominal (`String`) return type — otherwise a
/// use-site / merge shimmer check fires on a nominally-`String`-typed
/// alias.  Derived from the registry's `CREATES_SCOPE_ALIAS` trait
/// (top-level commands) and the per-subcommand `creates_scope_alias` flag
/// (`namespace upvar`).
fn is_scope_alias_call(registry: &CommandRegistry, command: &str, args: &[String]) -> bool {
    let Some(spec) = registry.get(command) else {
        return false;
    };
    if spec.traits.contains(Traits::CREATES_SCOPE_ALIAS) {
        return true;
    }
    args.first()
        .and_then(|sub| spec.resolve_subcommand(sub))
        .is_some_and(|sub| sub.creates_scope_alias)
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

/// The object class an argument *text* provably denotes: a
/// `[Class new|create …]` constructor, or a `$var` whose tracked type is
/// `OBJECT(class)`.  `None` when the text is not provably a single object —
/// the signal that harvests the element class of an object collection.
fn arg_object_class<S: std::hash::BuildHasher>(
    text: &str,
    uses: &HashMap<Symbol, u32>,
    types: &HashMap<ValueKey, TypeLattice>,
    ssa: &SsaFunction,
    registry: &CommandRegistry,
    known_classes: &HashSet<String, S>,
    namespace: &str,
) -> Option<String> {
    let stripped = text.trim();
    if is_pure_var_ref(stripped) {
        let t = lookup_var_type(normalise_var_name(stripped), uses, types, ssa)?;
        return (t.tcl_type == Some(TclType::Object))
            .then_some(t.class_name)
            .flatten();
    }
    if stripped.starts_with('[')
        && stripped.ends_with(']')
        && let Some((cmd, args)) = parse_command_substitution(stripped)
    {
        let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
        let t = return_type_for_command(registry, &cmd, &arg_refs, known_classes, namespace);
        if t.tcl_type == Some(TclType::Object)
            && let Some(class) = t.class_name
        {
            return Some(class);
        }
        // A registry-modelled `[Class new|create]` factory is not typed through
        // `return_type_for_command` (the class is a registered command, not a
        // `known_classes` name), so resolve its declared object class directly.
        if args.first().is_some_and(|a| a == "new" || a == "create") {
            return registry
                .object_class(&cmd)
                .map(|c| c.class_name.to_string());
        }
    }
    None
}

/// The element class a `dict set`/`dict append`/`dict lappend`/`lappend`
/// statement leaves on its target collection: the object class of the appended
/// value(s), joined with the collection's prior element class so a mixed-class
/// or non-object write widens the container back to a plain `List`/`Dict`.
///
/// `value_args` are the words that become element values (the value(s) after
/// the key for `dict set/append`, or after the var for `lappend`).  `target`
/// is the collection variable's own name, consulted for its prior element
/// class (monotone: an unknown-typed write carries the prior class forward
/// rather than dropping it — the fixpoint's phi joins handle genuine merges).
#[allow(clippy::too_many_arguments)] // mirrors the type-inference context threaded through this module
fn collection_element_class<S: std::hash::BuildHasher>(
    target: &str,
    value_args: &[&str],
    container: TclType,
    uses: &HashMap<Symbol, u32>,
    types: &HashMap<ValueKey, TypeLattice>,
    ssa: &SsaFunction,
    registry: &CommandRegistry,
    known_classes: &HashSet<String, S>,
    namespace: &str,
) -> TypeLattice {
    let prior = lookup_var_type(target, uses, types, ssa)
        .and_then(|t| t.element_class().map(str::to_owned));
    let mut elem = prior;
    for value in value_args {
        let value_class =
            arg_object_class(value, uses, types, ssa, registry, known_classes, namespace);
        match (&elem, value_class) {
            // No provable object class for this write — no new evidence for or
            // against homogeneity; carry the prior class forward.
            (_, None) => {}
            (None, Some(v)) => elem = Some(v),
            (Some(p), Some(v)) if *p == v => {}
            // A different concrete class: the collection is not homogeneous.
            (Some(_), Some(_)) => return TypeLattice::of(container),
        }
    }
    match elem {
        Some(c) => TypeLattice::collection_of(container, c),
        None => TypeLattice::of(container),
    }
}

/// The object type a container *retrieval* yields — `dict get $coll k` or
/// `lindex $coll i` on a collection tracked as object-homogeneous → an
/// `OBJECT(element_class)`.  `None` for any other shape, so the caller falls
/// back to the command's declared return type.  Only a single-level `dict get`
/// (one key) is modelled; a nested-path `dict get` is left untyped.
fn container_retrieval_object_type(
    command: &str,
    args: &[&str],
    uses: &HashMap<Symbol, u32>,
    types: &HashMap<ValueKey, TypeLattice>,
    ssa: &SsaFunction,
) -> Option<TypeLattice> {
    // Single-level retrieval only: `dict get $coll $key` (exactly one key) or
    // `lindex $coll $idx`.  A nested-path `dict get` (4+ args) does not match.
    let coll = match (command, args) {
        ("dict", [sub, coll, _key]) if *sub == "get" => *coll,
        ("lindex", [coll, _idx]) => *coll,
        _ => return None,
    };
    if !is_pure_var_ref(coll) {
        return None;
    }
    let class = lookup_var_type(normalise_var_name(coll), uses, types, ssa)?
        .element_class()
        .map(str::to_owned)?;
    Some(TypeLattice::object_of(class))
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
    value: &str,
    uses: &HashMap<Symbol, u32>,
    types: &HashMap<ValueKey, TypeLattice>,
    registry: &CommandRegistry,
    known_classes: &HashSet<String, S>,
    namespace: &str,
    ssa: &SsaFunction,
) -> TypeLattice {
    let stripped = value.trim();
    // Pure variable reference: inherit source type.
    if is_pure_var_ref(stripped) {
        let name = normalise_var_name(stripped);
        if let Some(&ver) = ssa.var_symbol(name).and_then(|s| uses.get(&s))
            && ver > 0
        {
            return ssa
                .var_symbol(name)
                .and_then(|s| types.get(&(s, ver)))
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
        // A `dict get $coll k` / `lindex $coll i` on an object-homogeneous
        // collection yields an `OBJECT(element_class)` — resolved before
        // the command's declared (`String`/`Overdefined`) return type.
        if let Some(t) = container_retrieval_object_type(&cmd, &arg_refs, uses, types, ssa) {
            return t;
        }
        return return_type_for_command(registry, &cmd, &arg_refs, known_classes, namespace);
    }
    // String interpolation or complex value.
    if value.contains('$') || value.contains('[') {
        return TypeLattice::of(TclType::String);
    }
    literal_type(value)
}

/// Infer the type produced by `stmt` under the current `types` map.
#[must_use]
fn evaluate_type_def<S: std::hash::BuildHasher>(
    stmt: &Statement,
    uses: &HashMap<Symbol, u32>,
    types: &HashMap<ValueKey, TypeLattice>,
    registry: &CommandRegistry,
    known_classes: &HashSet<String, S>,
    namespace: &str,
    ssa: &SsaFunction,
) -> TypeLattice {
    match stmt {
        Statement::AssignConst { value, .. } => literal_type(value),

        Statement::AssignExpr { expr, .. } => {
            // Build a name→TypeLattice map for variables used in the expression.
            let var_types: HashMap<String, TypeLattice> = uses
                .iter()
                .filter_map(|(&sym, &ver)| {
                    if ver == 0 {
                        return None;
                    }
                    let t = types.get(&(sym, ver))?;
                    Some((ssa.var_name(sym).to_owned(), t.clone()))
                })
                .collect();
            infer_expr_type(expr, &var_types)
        }

        Statement::AssignValue { value, .. } => {
            value_word_type(value, uses, types, registry, known_classes, namespace, ssa)
        }

        Statement::Incr { .. } => TypeLattice::of(TclType::Int),

        Statement::Call {
            command,
            canonical_command,
            args,
            defs,
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
                && registry.get(canon).and_then(|s| s.lowering_hook)
                    == Some(tcl_registry::hooks::LoweringHookId::Set)
            {
                return value_word_type(
                    arg_refs[1],
                    uses,
                    types,
                    registry,
                    known_classes,
                    namespace,
                    ssa,
                );
            }
            // A `dict set/append/lappend VAR …` or `lappend VAR …` that stores
            // object handles types VAR as a collection *of* that class, so a
            // later `[dict get $VAR k] method …` retrieval resolves the element.
            // `dict set VAR ?key…? value` takes a single trailing value; the
            // `*append`/`lappend` forms take every word after the key/var.
            if let Some((container, target, value_args)) = match (canon, &arg_refs[..]) {
                ("dict", ["set", target, rest @ ..]) if !rest.is_empty() => {
                    Some((TclType::Dict, *target, &rest[rest.len() - 1..]))
                }
                ("dict", ["append" | "lappend", target, _key, values @ ..])
                    if !values.is_empty() =>
                {
                    Some((TclType::Dict, *target, values))
                }
                ("lappend", [target, values @ ..]) if !values.is_empty() => {
                    Some((TclType::List, *target, values))
                }
                _ => None,
            } {
                return collection_element_class(
                    target,
                    value_args,
                    container,
                    uses,
                    types,
                    ssa,
                    registry,
                    known_classes,
                    namespace,
                );
            }
            // How a command types the variable(s) it *writes* is a distinct
            // fact from the value it *returns*.  A destructuring writer
            // (`lassign`, `scan`, `regexp`, `binary scan`) returns a leftover
            // list or a match/convert count while writing element-wise pieces;
            // `gets` returns the character count while writing a text line;
            // `lpop` returns the popped element while leaving a shortened list.
            // The registry declares this per command / subcommand
            // (`VarWriteTyping`), so the compiler never keys on the command
            // name.  The former `defs.len() > 1` heuristic guessed it from the
            // write count and mistyped every single-target destructure — a
            // `lassign $l x` target wrongly typed `List`, a `regexp … capture`
            // wrongly typed `Int` (issue #867).  Resolved through `canon`, not
            // the source spelling, so an aliased / renamed destructuring writer
            // (`rename lassign mylassign`) still resolves to the real command's
            // `VarWriteTyping` — the same canonical-command indirection the
            // value-passthrough store above and the collection-element typing
            // use (FP-SH-15).
            let typing = registry
                .resolve_call(canon, &arg_refs, DialectSet::empty())
                .map_or(VarWriteTyping::ReturnValue, |r| r.var_write_typing());
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
                VarWriteTyping::ReturnValue if defs.len() > 1 => TypeLattice::overdefined(),
                VarWriteTyping::ReturnValue => {
                    return_type_for_command(registry, canon, &arg_refs, known_classes, namespace)
                }
                VarWriteTyping::Fixed(t) => TypeLattice::of(t),
                VarWriteTyping::Destructured => TypeLattice::overdefined(),
            }
        }

        // `ExprEval`, `Barrier`, and structured statements that survive as
        // statements (before CFG construction in some paths) all lack a
        // resolvable result type here — treat them conservatively as
        // overdefined.
        _ => TypeLattice::overdefined(),
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
    let ctx = StatementTypingCtx {
        ssa,
        registry,
        known_classes,
        namespace: &namespace,
        escaping: &escaping,
        has_dynamic_variable_trace: trace_facts.has_dynamic_variable_trace,
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
    registry: &'a CommandRegistry,
    known_classes: &'a HashSet<String, S>,
    namespace: &'a str,
    /// Names [`crate::sccp::is_externally_mutable`] should treat as
    /// unconditionally aliased/escaping (per-function `analyse_var_observability`
    /// union'd with the caller's whole-module `extra_global_escaping` and
    /// `trace_facts.traced_variables`).
    escaping: &'a HashSet<String>,
    has_dynamic_variable_trace: bool,
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
        // imported variable's intrep is external and unknown. Every def of
        // one statement gets the same inferred type, so compute it once.
        let inferred = match stmt {
            Statement::Barrier { .. } => TypeLattice::overdefined(),
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
                TypeLattice::overdefined()
            }
            _ => evaluate_type_def(
                stmt,
                &ssa_stmt.uses,
                types,
                ctx.registry,
                ctx.known_classes,
                ctx.namespace,
                ctx.ssa,
            ),
        };
        for (&var, &ver) in &ssa_stmt.defs {
            let key = (var, ver);
            let old = types
                .get(&key)
                .cloned()
                .unwrap_or_else(TypeLattice::unknown);
            let def_type = if crate::sccp::is_externally_mutable(
                ctx.ssa.var_name(var),
                ctx.escaping,
                ctx.has_dynamic_variable_trace,
            ) {
                TypeLattice::overdefined()
            } else {
                inferred.clone()
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
                    infer_expr_type(expr, &var_types)
                } else if let Some(value) = value {
                    infer_return_value_type(value, &var_types, registry, known_classes, &namespace)
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
    literal_type(value)
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
        }
    }

    fn make_ssa_stmt(ssa: &mut SsaFunction, stmt: Statement, defs: &[(&str, u32)]) -> SsaStatement {
        SsaStatement {
            statement: stmt,
            uses: HashMap::new(),
            defs: defs.iter().map(|&(n, v)| (ssa.intern_var(n), v)).collect(),
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
        let t = evaluate_type_def(
            &stmt,
            &HashMap::new(),
            &HashMap::new(),
            &registry(),
            &HashSet::new(),
            "::",
            &ssa,
        );
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
        let t = evaluate_type_def(
            &stmt,
            &HashMap::new(),
            &HashMap::new(),
            &registry(),
            &HashSet::new(),
            "::",
            &ssa,
        );
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
        let t = evaluate_type_def(
            &stmt,
            &HashMap::new(),
            &HashMap::new(),
            &registry(),
            &HashSet::new(),
            "::",
            &ssa,
        );
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
        let t = evaluate_type_def(
            &stmt,
            &HashMap::new(),
            &HashMap::new(),
            &registry(),
            &HashSet::new(),
            "::",
            &ssa,
        );
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
        let t = evaluate_type_def(
            &stmt,
            &HashMap::new(),
            &HashMap::new(),
            &registry(),
            &HashSet::new(),
            "::",
            &ssa,
        );
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
                    && lat.kind == TypeKind::Known
                    && lat.tcl_type == Some(t)
            })
        }
        // Helper: is every version of `name` non-Known (OVERDEFINED/UNKNOWN)?
        fn none_known(fu: &crate::compilation_unit::FunctionUnit, name: &str) -> bool {
            fu.types
                .iter()
                .filter(|((sym, _), _)| fu.ssa.var_name(*sym) == name)
                .all(|(_, lat)| lat.kind != TypeKind::Known)
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
        let t = literal_type("3.14");
        assert_eq!(t, TypeLattice::of(TclType::Double));
    }

    #[test]
    fn bool_literal_infers_boolean() {
        let t = literal_type("true");
        assert_eq!(t, TypeLattice::of(TclType::Boolean));
        let t2 = literal_type("false");
        assert_eq!(t2, TypeLattice::of(TclType::Boolean));
    }

    #[test]
    fn string_literal_infers_string() {
        let t = literal_type("hello");
        assert_eq!(t, TypeLattice::of(TclType::String));
    }

    #[test]
    fn hex_and_binary_literals_infer_int() {
        // Hex / binary literals store an INT intrep (`set n 0x80; incr n` is
        // one clean parse, not per-iteration shimmer).
        for lit in ["0x80", "0X1f", "-0xFF", "0b1010", "0B1", "+0x0"] {
            assert_eq!(
                literal_type(lit),
                TypeLattice::of(TclType::Int),
                "{lit} should be INT"
            );
        }
        // Octal `0o…` and non-integer hex stay STRING (set-statement classifier).
        assert_eq!(literal_type("0o17"), TypeLattice::of(TclType::String));
        assert_eq!(literal_type("0xZZ"), TypeLattice::of(TclType::String));
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
            fu.ssa.var_name(*name) == "x" && t.tcl_type == Some(TclType::Int)
        });
        let y_is_int = fu.types.iter().any(|((name, _), t)| {
            fu.ssa.var_name(*name) == "y" && t.tcl_type == Some(TclType::Int)
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
            fu.ssa.var_name(*name) == "x" && t.tcl_type == Some(TclType::Int)
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
            fu.ssa.var_name(*name) == "n" && t.tcl_type == Some(TclType::Int)
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
                && t.tcl_type == Some(TclType::Object)
                && t.class_name.as_deref() == Some("::Pin")
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
                && t.tcl_type == Some(TclType::Object)
                && t.class_name.as_deref() == Some("::Pin")
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
    /// context — variable refs stay `Unknown`).
    fn infer_str(src: &str) -> TypeLattice {
        infer_str_dialect(src, None)
    }

    /// As [`infer_str`] but parses under `dialect` (the iRules string
    /// predicates only tokenise as operators in the iRules dialect).
    fn infer_str_dialect(src: &str, dialect: Option<&str>) -> TypeLattice {
        let node = crate::parse_expr(src, dialect);
        infer_expr_type(&node, &HashMap::new())
    }

    #[test]
    fn math_function_calls_infer_their_return_type() {
        // (a) sqrt → Double, int → Int, bool → Boolean.
        assert_eq!(infer_str("sqrt(2.0)").tcl_type, Some(TclType::Double));
        assert_eq!(infer_str("sin($x)").tcl_type, Some(TclType::Double));
        assert_eq!(infer_str("int($x)").tcl_type, Some(TclType::Int));
        assert_eq!(infer_str("wide($x)").tcl_type, Some(TclType::Int));
        assert_eq!(infer_str("isnan($x)").tcl_type, Some(TclType::Boolean));
        // abs is identity: abs(2) keeps the operand's Int type.
        assert_eq!(infer_str("abs(2)").tcl_type, Some(TclType::Int));
        // max/min join operands: max(1, 2) stays Int.
        assert_eq!(infer_str("max(1, 2)").tcl_type, Some(TclType::Int));
        // Tcl 9.1 C99 functions (TIP 745): double-valued, except `signbit`.
        for f in [
            "acosh", "cbrt", "exp2", "log2", "trunc", "erf", "expm1", "logb",
        ] {
            assert_eq!(
                infer_str(&format!("{f}($x)")).tcl_type,
                Some(TclType::Double),
                "{f} should infer Double",
            );
        }
        assert_eq!(infer_str("signbit($x)").tcl_type, Some(TclType::Boolean));
        // Unknown function → Numeric (conservative).
        assert_eq!(infer_str("nope($x)").tcl_type, Some(TclType::Numeric));
    }

    #[test]
    fn bitwise_and_shift_ops_infer_int() {
        // (c) bitwise / shift force Int even with untyped operands.
        for src in ["$x & $y", "$x | $y", "$x ^ $y", "$x << 2", "$x >> 2"] {
            assert_eq!(
                infer_str(src).tcl_type,
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
            let t = infer_str_dialect(src, Some("f5-irules"));
            assert_eq!(
                t.tcl_type,
                Some(TclType::Boolean),
                "expected Boolean for `{src}`, got {t:?}",
            );
        }
    }

    #[test]
    fn arithmetic_promotes_double() {
        // `_arithmetic_result`: int + double → Double (was Numeric).
        assert_eq!(infer_str("3 + 2.0").tcl_type, Some(TclType::Double));
        // int + int → Int.
        assert_eq!(infer_str("3 + 4").tcl_type, Some(TclType::Int));
    }

    #[test]
    fn expr_context_literal_typing() {
        // Every integer spelling tokenises to Int in expr context — including
        // octal `0o…`, which the set-statement classifier keeps as String.
        assert_eq!(infer_str("0o17").tcl_type, Some(TclType::Int));
        assert_eq!(infer_str("0xff").tcl_type, Some(TclType::Int));
        assert_eq!(infer_str("0b1010").tcl_type, Some(TclType::Int));
        assert_eq!(infer_str("42").tcl_type, Some(TclType::Int));
        assert_eq!(infer_str("3.14").tcl_type, Some(TclType::Double));
        // The set-statement classifier still keeps octal as String.
        assert_eq!(literal_type("0o17"), TypeLattice::of(TclType::String));
        // An unrecognised literal degrades to Numeric in expr context
        // (the set-statement classifier would say String).
        assert_eq!(expr_literal_type("nope").tcl_type, Some(TclType::Numeric));
    }

    #[test]
    fn bitnot_coerces_to_int() {
        // `~$x` always yields Int, regardless of the operand's type
        // (was leaking the operand type via the identity arm).
        assert_eq!(infer_str("~$x").tcl_type, Some(TclType::Int));
        assert_eq!(infer_str("~3.5").tcl_type, Some(TclType::Int));
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
        let widened = fu
            .types
            .iter()
            .any(|((n, _), t)| fu.ssa.var_name(*n) == "counter" && t.kind == TypeKind::Overdefined);
        assert!(
            widened,
            "scope-aliased 'counter' should be OVERDEFINED: {:?}",
            fu.types
        );
    }

    #[test]
    fn conditionally_assigned_param_merges_to_overdefined() {
        use crate::compilation_unit::CompilationUnit;
        // RUST_ISSUE_066: a parameter assigned in only one arm must NOT be
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
            .any(|((n, _), t)| fu.ssa.var_name(*n) == "x" && t.kind == TypeKind::Overdefined);
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
                fu.ssa.var_name(*n) == "x" && matches!(t.tcl_type, Some(TclType::Int))
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
        // RUST_ISSUE_066 (FP guard): an *unconditionally* reassigned local is
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
            .all(|(_, t)| matches!(t.tcl_type, Some(TclType::Int)) || t.kind == TypeKind::Unknown);
        assert!(
            all_int,
            "unconditionally-assigned 'y' should stay Int on every path: {:?}",
            fu.types
        );
    }
}
