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

use tcl_registry::{CommandRegistry, TclType, Traits};

use crate::cfg::{Function as CfgFunction, Terminator};
use crate::expr_ast::{BinOp, ExprNode, UnaryOp};
use crate::ir::Statement;
use crate::naming::normalise_var_name;
use crate::sccp::SccpResult;
use crate::ssa::{SsaFunction, ValueKey};
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
fn constructor_object_type(
    command: &str,
    args: &[&str],
    known_classes: &HashSet<String>,
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
pub(crate) fn return_type_for_command(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
    known_classes: &HashSet<String>,
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
            && let Some(sub) = spec.subcommand(sub_name)
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
        // Integer-returning conversions.
        "int" | "round" | "ceil" | "floor" | "isqrt" | "wide" | "entier" => {
            TypeLattice::of(TclType::Int)
        }
        // Double-returning math.
        "double" | "sin" | "cos" | "tan" | "asin" | "acos" | "atan" | "atan2" | "sinh" | "cosh"
        | "tanh" | "sqrt" | "exp" | "log" | "log10" | "pow" | "hypot" | "fmod" | "rand"
        | "srand" => TypeLattice::of(TclType::Double),
        // Boolean-returning predicates.
        "bool" | "isnan" | "isinf" => TypeLattice::of(TclType::Boolean),
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
        .and_then(|sub| spec.subcommand(sub))
        .is_some_and(|sub| sub.creates_scope_alias)
}

/// Infer the type produced by `stmt` under the current `types` map.
#[must_use]
fn evaluate_type_def(
    stmt: &Statement,
    uses: &HashMap<String, u32>,
    types: &HashMap<ValueKey, TypeLattice>,
    registry: &CommandRegistry,
    known_classes: &HashSet<String>,
    namespace: &str,
) -> TypeLattice {
    match stmt {
        Statement::AssignConst { value, .. } => literal_type(value),

        Statement::AssignExpr { expr, .. } => {
            // Build a name→TypeLattice map for variables used in the expression.
            let var_types: HashMap<String, TypeLattice> = uses
                .iter()
                .filter_map(|(name, &ver)| {
                    if ver == 0 {
                        return None;
                    }
                    let t = types.get(&(name.clone(), ver))?;
                    Some((name.clone(), t.clone()))
                })
                .collect();
            infer_expr_type(expr, &var_types)
        }

        Statement::AssignValue { value, .. } => {
            let stripped = value.trim();
            // Pure variable reference: inherit source type.
            if is_pure_var_ref(stripped) {
                let name = normalise_var_name(stripped);
                if let Some(&ver) = uses.get(name)
                    && ver > 0
                {
                    return types
                        .get(&(name.to_owned(), ver))
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
                return return_type_for_command(
                    registry,
                    &cmd,
                    &arg_refs,
                    known_classes,
                    namespace,
                );
            }
            // String interpolation or complex value.
            if value.contains('$') || value.contains('[') {
                return TypeLattice::of(TclType::String);
            }
            literal_type(value)
        }

        Statement::Incr { .. } => TypeLattice::of(TclType::Int),

        Statement::Call {
            command,
            args,
            defs,
            ..
        } if !defs.is_empty() => {
            let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
            return_type_for_command(registry, command, &arg_refs, known_classes, namespace)
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
#[must_use]
#[allow(clippy::implicit_hasher)] // `known_classes` is always the default-hasher set built by the CU.
pub fn propagate_types(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    sccp: &SccpResult,
    registry: &CommandRegistry,
    known_classes: &HashSet<String>,
) -> HashMap<ValueKey, TypeLattice> {
    let preds = cfg.predecessors();
    let order = crate::sccp::cfg_order(cfg);
    // Constructor heads written `[Foo new]` inside this function resolve
    // relative names against the function's own namespace.
    let namespace = function_namespace(&cfg.name);

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
            if bn != &cfg.entry {
                let mut exec_preds: Vec<&str> = preds
                    .get(bn)
                    .map(|ps| {
                        ps.iter()
                            .filter(|p| {
                                sccp.executable_edges
                                    .contains(&((*p).to_owned(), bn.clone()))
                            })
                            .map(String::as_str)
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
                        let ver = phi.incoming.get(*pred).copied().unwrap_or(0);
                        if ver == 0 {
                            continue;
                        }
                        let t = types
                            .get(&(phi.name.clone(), ver))
                            .cloned()
                            .unwrap_or_else(TypeLattice::unknown);
                        phi_type = type_join(&phi_type, &t);
                    }
                    let key = (phi.name.clone(), phi.version);
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
            for ssa_stmt in &ssa_block.statements {
                let stmt = &ssa_stmt.statement;
                // A barrier widens every def to OVERDEFINED (it may have
                // mutated them arbitrarily); a scope-alias declaration
                // (`global`/`variable`/`upvar`/`namespace upvar`) likewise
                // widens its defs — the imported variable's intrep is
                // external and unknown.'s
                // barrier + `alias_cmds` arms.  Every def of one statement
                // gets the same inferred type, so compute it once.
                let inferred = match stmt {
                    Statement::Barrier { .. } => TypeLattice::overdefined(),
                    Statement::Call {
                        command,
                        args,
                        defs,
                        ..
                    } if !defs.is_empty() && is_scope_alias_call(registry, command, args) => {
                        TypeLattice::overdefined()
                    }
                    _ => evaluate_type_def(
                        stmt,
                        &ssa_stmt.uses,
                        &types,
                        registry,
                        known_classes,
                        &namespace,
                    ),
                };
                for (var, &ver) in &ssa_stmt.defs {
                    let key = (var.clone(), ver);
                    let old = types
                        .get(&key)
                        .cloned()
                        .unwrap_or_else(TypeLattice::unknown);
                    let merged = type_join(&old, &inferred);
                    if merged != old {
                        types.insert(key, merged);
                        changed = true;
                    }
                }
            }
        }
    }

    types
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
pub(crate) fn infer_function_return_type(
    cfg: &CfgFunction,
    sccp: &SccpResult,
    types: &HashMap<ValueKey, TypeLattice>,
    registry: &CommandRegistry,
    known_classes: &HashSet<String>,
) -> TypeLattice {
    let namespace = function_namespace(&cfg.name);
    // Collapse the versioned type map to a name-keyed map by joining
    // every version of each name — the over-approximation noted above.
    let mut var_types: HashMap<String, TypeLattice> = HashMap::new();
    for ((name, _ver), t) in types {
        var_types
            .entry(name.clone())
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
fn infer_return_value_type(
    value: &str,
    var_types: &HashMap<String, TypeLattice>,
    registry: &CommandRegistry,
    known_classes: &HashSet<String>,
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

    fn empty_sccp(blocks: &[&str]) -> SccpResult {
        SccpResult {
            values: HashMap::new(),
            executable_blocks: blocks.iter().copied().map(String::from).collect(),
            executable_edges: HashSet::default(),
            constant_branches: Vec::new(),
        }
    }

    fn assign_const(name: &str, value: &str) -> Statement {
        Statement::AssignConst {
            span: Span::new(0, 0),
            name: name.to_owned(),
            value: value.to_owned(),
        }
    }

    fn make_ssa_stmt(stmt: Statement, defs: &[(&str, u32)]) -> SsaStatement {
        SsaStatement {
            statement: stmt,
            uses: HashMap::new(),
            defs: defs.iter().map(|&(n, v)| (String::from(n), v)).collect(),
        }
    }

    #[test]
    fn integer_literal_infers_int() {
        let mut f = Function::new("::top", "entry");
        f.blocks
            .get_mut("entry")
            .unwrap()
            .statements
            .push(assign_const("x", "42"));

        let sccp = empty_sccp(&["entry"]);
        let mut ssa = SsaFunction {
            name: "::top".into(),
            entry: "entry".into(),
            blocks: HashMap::new(),
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
        };
        ssa.blocks.insert(
            "entry".into(),
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: vec![make_ssa_stmt(assign_const("x", "42"), &[("x", 1)])],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );
        let types = propagate_types(&f, &ssa, &sccp, &registry(), &HashSet::new());
        assert_eq!(
            types.get(&("x".to_owned(), 1)),
            Some(&TypeLattice::of(TclType::Int))
        );
    }

    #[test]
    fn incr_infers_int() {
        let stmt = Statement::Incr {
            span: Span::new(0, 0),
            name: "n".to_owned(),
            amount: None,
            safe_on_uninit: false,
        };
        let t = evaluate_type_def(
            &stmt,
            &HashMap::new(),
            &HashMap::new(),
            &registry(),
            &HashSet::new(),
            "::",
        );
        assert_eq!(t, TypeLattice::of(TclType::Int));
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
        cfg.blocks.insert("exit".into(), Block::new("exit"));
        cfg.blocks.get_mut("entry").unwrap().terminator = Some(crate::cfg::Terminator::Goto {
            target: "exit".into(),
            span: None,
        });

        let phi = Phi {
            name: "x".into(),
            version: 2,
            incoming: [("entry".into(), 1u32)].into_iter().collect(),
        };
        let entry_stmt = make_ssa_stmt(assign_const("x", "10"), &[("x", 1)]);
        let mut ssa = SsaFunction {
            name: "::top".into(),
            entry: "entry".into(),
            blocks: HashMap::new(),
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
        };
        ssa.blocks.insert(
            "entry".into(),
            SsaBlock {
                name: "entry".into(),
                phis: Vec::new(),
                statements: vec![entry_stmt],
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );
        ssa.blocks.insert(
            "exit".into(),
            SsaBlock {
                name: "exit".into(),
                phis: vec![phi],
                statements: Vec::new(),
                entry_versions: HashMap::new(),
                exit_versions: HashMap::new(),
            },
        );

        let mut sccp = empty_sccp(&["entry", "exit"]);
        sccp.executable_edges
            .insert(("entry".into(), "exit".into()));

        let types = propagate_types(&cfg, &ssa, &sccp, &registry(), &HashSet::new());
        // x@1 (entry) should be Int.
        assert_eq!(
            types.get(&("x".to_owned(), 1)),
            Some(&TypeLattice::of(TclType::Int))
        );
        // x@2 (phi in exit) should propagate Int from entry.
        assert_eq!(
            types.get(&("x".to_owned(), 2)),
            Some(&TypeLattice::of(TclType::Int))
        );
    }

    /// `AssignValue` with a pure variable reference inherits the source type.
    #[test]
    fn assign_value_pure_var_ref_inherits_type() {
        use crate::compilation_unit::CompilationUnit;
        let cu = CompilationUnit::build_for("set x 42\nset y $x", &registry(), false);
        let fu = cu.function("::top").unwrap();
        // x should be Int; y (which copies x) should also be Int.
        let x_is_int = fu
            .types
            .iter()
            .any(|((name, _), t)| name == "x" && t.tcl_type == Some(TclType::Int));
        let y_is_int = fu
            .types
            .iter()
            .any(|((name, _), t)| name == "y" && t.tcl_type == Some(TclType::Int));
        assert!(x_is_int, "expected x to be Int");
        assert!(y_is_int, "expected y to inherit Int type from x");
    }

    /// `AssignValue` with a command substitution uses the command's return type.
    #[test]
    fn assign_value_command_sub_uses_return_type() {
        use crate::compilation_unit::CompilationUnit;
        // `llength` returns Int per the registry.
        let cu =
            CompilationUnit::build_for("set lst {a b c}\nset n [llength $lst]", &registry(), false);
        let fu = cu.function("::top").unwrap();
        let n_is_int = fu
            .types
            .iter()
            .any(|((name, _), t)| name == "n" && t.tcl_type == Some(TclType::Int));
        assert!(n_is_int, "expected n to be Int (llength return type)");
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
        // A plain command (and `namespace eval`) is not a scope alias.
        assert!(!is_scope_alias_call(&reg, "set", &["x".into(), "1".into()]));
        assert!(!is_scope_alias_call(
            &reg,
            "namespace",
            &["eval".into(), "ns".into(), "body".into()]
        ));
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
            .any(|((n, _), t)| n == "counter" && t.kind == TypeKind::Overdefined);
        assert!(
            widened,
            "scope-aliased 'counter' should be OVERDEFINED: {:?}",
            fu.types
        );
    }
}
