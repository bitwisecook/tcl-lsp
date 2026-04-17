//! Type propagation over the SSA graph.
//!
//! Ported from `core/compiler/core_analyses.py::_type_propagation`
//! (C26). Computes a `TypeLattice` for every SSA value by iterating
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

#![allow(clippy::implicit_hasher)]

use std::collections::HashMap;

use tcl_registry::{CommandRegistry, TclType};

use crate::cfg::Function as CfgFunction;
use crate::expr_ast::{BinOp, ExprNode, UnaryOp};
use crate::ir::Statement;
use crate::naming::normalise_var_name;
use crate::sccp::SccpResult;
use crate::ssa::{SsaFunction, ValueKey};
use crate::types::{type_join, TypeKind, TypeLattice};
use crate::value_shapes::{is_pure_var_ref, parse_command_substitution};

// Float literal pattern matching Python's `_FLOAT_RE`: requires a decimal
// point so that forms like `1e3` (no `.`) are NOT classified as floats.
fn looks_like_float(s: &str) -> bool {
    let s = s.trim();
    s.contains('.') && s.parse::<f64>().is_ok()
}

const BOOL_LITERALS: &[&str] = &["true", "false", "yes", "no", "on", "off"];

/// Classify a literal string as its Tcl intrep type.
#[must_use]
fn literal_type(text: &str) -> TypeLattice {
    let s = text.trim();
    if s.parse::<i64>().is_ok() {
        return TypeLattice::of(TclType::Int);
    }
    if looks_like_float(s) {
        return TypeLattice::of(TclType::Double);
    }
    // Case-insensitive boolean check, matching Python's behaviour.
    if BOOL_LITERALS.contains(&s.to_ascii_lowercase().as_str()) {
        return TypeLattice::of(TclType::Boolean);
    }
    TypeLattice::of(TclType::String)
}

/// Return the type produced by a known command's return value.
///
/// Checks the command spec's `return_type` field, with subcommand
/// support.
#[must_use]
fn return_type_for_command(
    registry: &CommandRegistry,
    command: &str,
    args: &[&str],
) -> TypeLattice {
    let Some(spec) = registry.get(command) else {
        return TypeLattice::overdefined();
    };

    // Subcommand commands: check sub's return_type.
    if !spec.subcommands.is_empty() {
        if let Some(sub_name) = args.first() {
            if let Some(sub) = spec.subcommand(sub_name) {
                return match sub.return_type {
                    Some(t) => TypeLattice::of(t),
                    None => TypeLattice::overdefined(),
                };
            }
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
        ExprNode::Literal { text, .. } => literal_type(text),

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
            let lt = infer_expr_type(left, var_types);
            let rt = infer_expr_type(right, var_types);
            match op {
                // Numeric arithmetic → promote operand types.
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
                | BinOp::BitXor => {
                    // Prefer Int when both operands are integrally typed.
                    let both_int = matches!(
                        (&lt, &rt),
                        (t, u)
                        if t.kind == TypeKind::Known && u.kind == TypeKind::Known
                            && matches!(t.tcl_type, Some(TclType::Int | TclType::Boolean))
                            && matches!(u.tcl_type, Some(TclType::Int | TclType::Boolean))
                    );
                    if both_int {
                        TypeLattice::of(TclType::Int)
                    } else {
                        TypeLattice::of(TclType::Numeric)
                    }
                }
                // Boolean comparisons.
                BinOp::And
                | BinOp::Or
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
                | BinOp::Ni => TypeLattice::of(TclType::Boolean),
                // iRules string predicates.
                _ => TypeLattice::overdefined(),
            }
        }

        ExprNode::Unary { op, operand, .. } => match op {
            UnaryOp::Neg | UnaryOp::Pos | UnaryOp::BitNot => infer_expr_type(operand, var_types),
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

        // Command substitutions, raw/unrecognised expression text, and
        // function calls inside expressions all need the registry (or
        // runtime context) to resolve. Without it we over-approximate to
        // overdefined; the outer evaluate_type_def handles command-sub
        // type resolution where the registry is in scope.
        ExprNode::Command { .. } | ExprNode::Raw { .. } | ExprNode::Call { .. } => {
            TypeLattice::overdefined()
        }
    }
}

/// Infer the type produced by `stmt` under the current `types` map.
#[must_use]
fn evaluate_type_def(
    stmt: &Statement,
    uses: &HashMap<String, u32>,
    types: &HashMap<ValueKey, TypeLattice>,
    registry: &CommandRegistry,
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
                if let Some(&ver) = uses.get(name) {
                    if ver > 0 {
                        return types
                            .get(&(name.to_owned(), ver))
                            .cloned()
                            .unwrap_or_else(TypeLattice::unknown);
                    }
                }
                return TypeLattice::unknown();
            }
            // Command substitution: [cmd ...].
            if stripped.starts_with('[') && stripped.ends_with(']') {
                if let Some((cmd, args)) = parse_command_substitution(stripped) {
                    let arg_refs: Vec<&str> = args.iter().map(String::as_str).collect();
                    return return_type_for_command(registry, &cmd, &arg_refs);
                }
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
            return_type_for_command(registry, command, &arg_refs)
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
pub fn propagate_types(
    cfg: &CfgFunction,
    ssa: &SsaFunction,
    sccp: &SccpResult,
    registry: &CommandRegistry,
) -> HashMap<ValueKey, TypeLattice> {
    let preds = cfg.predecessors();
    let order = crate::sccp::cfg_order(cfg);

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
                let exec_preds: Vec<&str> = preds
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
                match stmt {
                    Statement::Barrier { .. } => {
                        for (var, &ver) in &ssa_stmt.defs {
                            let key = (var.clone(), ver);
                            let old = types
                                .get(&key)
                                .cloned()
                                .unwrap_or_else(TypeLattice::unknown);
                            let merged = type_join(&old, &TypeLattice::overdefined());
                            if merged != old {
                                types.insert(key, merged);
                                changed = true;
                            }
                        }
                    }
                    _ => {
                        for (var, &ver) in &ssa_stmt.defs {
                            let inferred =
                                evaluate_type_def(stmt, &ssa_stmt.uses, &types, registry);
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
        }
    }

    types
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
        let types = propagate_types(&f, &ssa, &sccp, &registry());
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
        let t = evaluate_type_def(&stmt, &HashMap::new(), &HashMap::new(), &registry());
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

        let types = propagate_types(&cfg, &ssa, &sccp, &registry());
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
}
