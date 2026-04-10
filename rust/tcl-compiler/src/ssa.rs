// These algorithms always use the default RandomState hasher; making
// them generic over BuildHasher adds complexity for no real benefit.
#![allow(clippy::implicit_hasher)]
//! Static Single-Assignment (SSA) construction over CFG blocks.
//!
//! SSA is a variable-naming discipline where every variable is assigned
//! exactly once. When control flow merges (e.g. after an `if`), a
//! synthetic *phi node* is inserted to select the correct version of a
//! variable depending on which predecessor block was executed.
//!
//! This module provides:
//!
//! 1. SSA data structures: [`Phi`], [`SsaStatement`], [`SsaBlock`],
//!    [`SsaFunction`].
//! 2. **Dominator** computation: [`compute_dominators`] and
//!    [`compute_idom`] for immediate dominators.
//! 3. **Dominance frontier**: [`compute_dominance_frontier`].
//! 4. **Phi placement**: [`compute_phi_vars`] using the iterated
//!    dominance frontier algorithm.
//! 5. **Variable definition extraction**: [`defs_of`] extracts variable
//!    names defined by an IR statement.
//!
//! The full SSA rename pass (`build_ssa`) is deferred until the
//! variable-use scanner (`_uses`) is ported — it depends on the
//! command registry and `VarReferenceScanner` which are not yet in
//! Rust.

use std::collections::{HashMap, HashSet};

use crate::cfg;
use crate::ir::Statement;
use crate::naming::normalise_var_name;

/// SSA version number — each definition of a variable gets a unique version.
pub type Version = u32;

/// Key identifying a specific SSA value: `(variable_name, version)`.
pub type ValueKey = (String, Version);

// SSA data structures

/// A phi node merging variable versions at a control-flow join.
///
/// `incoming` maps each predecessor block name to the variable
/// version that flows in from that edge.
#[derive(Debug, Clone, PartialEq)]
pub struct Phi {
    /// Variable name.
    pub name: String,
    /// SSA version assigned by this phi.
    pub version: Version,
    /// Predecessor block → incoming version.
    pub incoming: HashMap<String, Version>,
}

/// An IR statement annotated with SSA version numbers.
///
/// `uses` maps each variable name read by the statement to the
/// SSA version in scope. `defs` maps each variable name written
/// to its newly assigned version.
#[derive(Debug, Clone, PartialEq)]
pub struct SsaStatement {
    /// The underlying IR statement.
    pub statement: Statement,
    /// Variables read: name → SSA version.
    pub uses: HashMap<String, Version>,
    /// Variables written: name → SSA version.
    pub defs: HashMap<String, Version>,
}

/// A CFG basic block in SSA form.
///
/// `entry_versions` / `exit_versions` record which SSA version
/// of each variable is live at the start and end of the block.
#[derive(Debug, Clone, PartialEq)]
pub struct SsaBlock {
    /// Block name.
    pub name: String,
    /// Phi nodes at the start of this block.
    pub phis: Vec<Phi>,
    /// SSA-annotated statements.
    pub statements: Vec<SsaStatement>,
    /// Variable versions at block entry.
    pub entry_versions: HashMap<String, Version>,
    /// Variable versions at block exit.
    pub exit_versions: HashMap<String, Version>,
}

/// Complete SSA representation of one Tcl procedure or top-level script.
///
/// Includes the dominator tree and dominance frontier so that
/// downstream passes (SCCP, liveness) do not need to recompute them.
#[derive(Debug, Clone, PartialEq)]
pub struct SsaFunction {
    /// Procedure name.
    pub name: String,
    /// Entry block name.
    pub entry: String,
    /// SSA blocks keyed by block name.
    pub blocks: HashMap<String, SsaBlock>,
    /// Immediate dominator: block → parent (None for entry).
    pub idom: HashMap<String, Option<String>>,
    /// Dominance frontier: block → frontier blocks.
    pub dominance_frontier: HashMap<String, Vec<String>>,
    /// Dominator tree: block → children.
    pub dominator_tree: HashMap<String, Vec<String>>,
}

// Variable definition extraction

/// Extract variable names defined by an IR statement.
///
/// Handles assignments (`set`, `incr`), call defs, `trace add variable`,
/// and `dict for`/`dict map` barriers. This is the Rust equivalent of
/// the Python `_defs()` function in `ssa.py`.
#[must_use]
pub fn defs_of(stmt: &Statement) -> Vec<String> {
    match stmt {
        Statement::AssignConst { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::Incr { name, .. } => {
            vec![normalise_var_name(name).to_owned()]
        }
        Statement::Call { defs, .. } if !defs.is_empty() => defs.clone(),
        Statement::Barrier { command, args, .. } => {
            // trace add variable $var → defines $var
            if command == "trace" && args.len() >= 3 && args[0] == "add" && args[1] == "variable" {
                return vec![normalise_var_name(&args[2]).to_owned()];
            }
            // dict for/map barriers: extract iteration variable names
            if (command.ends_with("::for") || command.ends_with("::map")) && !args.is_empty() {
                return args[0].split_whitespace().map(String::from).collect();
            }
            Vec::new()
        }
        _ => Vec::new(),
    }
}

// Dominator algorithms

/// Compute the dominator sets for all blocks in a CFG function.
///
/// Uses the iterative dataflow algorithm. Returns a map from block
/// name to the set of blocks that dominate it.
#[must_use]
pub fn compute_dominators(func: &cfg::Function) -> HashMap<String, HashSet<String>> {
    let reachable = func.reachable_blocks();
    let mut dom: HashMap<String, HashSet<String>> = HashMap::new();

    for name in func.blocks.keys() {
        if !reachable.contains(name.as_str()) || *name == func.entry {
            dom.insert(name.clone(), HashSet::from([name.clone()]));
        } else {
            dom.insert(name.clone(), reachable.clone());
        }
    }

    let preds = func.predecessors();
    let mut changed = true;
    while changed {
        changed = false;
        for name in &reachable {
            if *name == func.entry {
                continue;
            }
            let bn_preds: Vec<&String> = preds
                .get(name)
                .map(|p| {
                    p.iter()
                        .filter(|p| reachable.contains(p.as_str()))
                        .collect()
                })
                .unwrap_or_default();

            let new_dom = if bn_preds.is_empty() {
                HashSet::from([name.clone()])
            } else {
                let mut inter = dom[bn_preds[0]].clone();
                for p in &bn_preds[1..] {
                    inter = inter.intersection(&dom[*p]).cloned().collect();
                }
                inter.insert(name.clone());
                inter
            };

            if new_dom != dom[name] {
                dom.insert(name.clone(), new_dom);
                changed = true;
            }
        }
    }
    dom
}

/// Compute immediate dominators from dominator sets.
///
/// The immediate dominator of a block is the closest strict dominator
/// (the one with the largest dominator set).
#[must_use]
pub fn compute_idom(
    func: &cfg::Function,
    dom: &HashMap<String, HashSet<String>>,
) -> HashMap<String, Option<String>> {
    let reachable = func.reachable_blocks();
    let mut idom: HashMap<String, Option<String>> = HashMap::new();

    for name in func.blocks.keys() {
        idom.insert(name.clone(), None);
    }

    for name in &reachable {
        if *name == func.entry {
            continue;
        }
        let strict: HashSet<&String> = dom[name].iter().filter(|d| *d != name).collect();
        if strict.is_empty() {
            continue;
        }
        // The idom is the strict dominator with the largest dom set.
        let best = strict.iter().max_by_key(|d| dom[**d].len()).unwrap();
        idom.insert(name.clone(), Some((*best).clone()));
    }
    idom
}

/// Compute the dominance frontier for each block.
///
/// A block `b` is in the dominance frontier of block `a` if `a`
/// dominates a predecessor of `b` but does not strictly dominate `b`.
#[must_use]
pub fn compute_dominance_frontier(
    func: &cfg::Function,
    idom: &HashMap<String, Option<String>>,
) -> HashMap<String, HashSet<String>> {
    let reachable = func.reachable_blocks();
    let preds = func.predecessors();
    let mut df: HashMap<String, HashSet<String>> = HashMap::new();

    for name in func.blocks.keys() {
        df.insert(name.clone(), HashSet::new());
    }

    for name in &reachable {
        let bn_preds: Vec<&String> = preds
            .get(name)
            .map(|p| {
                p.iter()
                    .filter(|p| reachable.contains(p.as_str()))
                    .collect()
            })
            .unwrap_or_default();

        if bn_preds.len() < 2 {
            continue;
        }

        for p in &bn_preds {
            let mut runner = Some((*p).clone());
            while let Some(ref r) = runner {
                if idom.get(name).and_then(|i| i.as_ref()) == Some(r) {
                    break;
                }
                df.entry(r.clone()).or_default().insert(name.clone());
                runner = idom.get(r).cloned().flatten();
            }
        }
    }
    df
}

/// Build the dominator tree from immediate dominators.
///
/// Returns a map from each block to its children in the dominator tree.
#[must_use]
pub fn build_dom_tree(idom: &HashMap<String, Option<String>>) -> HashMap<String, Vec<String>> {
    let mut tree: HashMap<String, Vec<String>> = HashMap::new();
    for name in idom.keys() {
        tree.entry(name.clone()).or_default();
    }
    for (name, parent) in idom {
        if let Some(p) = parent {
            tree.entry(p.clone()).or_default().push(name.clone());
        }
    }
    for children in tree.values_mut() {
        children.sort();
    }
    tree
}

/// Compute which variables need phi nodes in each block.
///
/// Uses the iterated dominance frontier algorithm: for each variable,
/// starting from blocks where it is defined, propagate phi nodes to
/// the dominance frontier until convergence.
#[must_use]
pub fn compute_phi_vars(
    func: &cfg::Function,
    df: &HashMap<String, HashSet<String>>,
) -> HashMap<String, HashSet<String>> {
    let reachable = func.reachable_blocks();

    // Collect definition sites for each variable.
    let mut defsites: HashMap<String, HashSet<String>> = HashMap::new();
    for name in &reachable {
        if let Some(block) = func.blocks.get(name) {
            for stmt in &block.statements {
                for var in defs_of(stmt) {
                    defsites.entry(var).or_default().insert(name.clone());
                }
            }
        }
    }

    // Place phis using iterated dominance frontier.
    let mut phi: HashMap<String, HashSet<String>> = HashMap::new();
    for name in func.blocks.keys() {
        phi.insert(name.clone(), HashSet::new());
    }

    for (var, sites) in &defsites {
        let mut work: Vec<String> = sites.iter().cloned().collect();
        work.sort();
        let mut has_phi: HashSet<String> = HashSet::new();

        while let Some(nb) = work.pop() {
            for fb in df.get(&nb).into_iter().flatten() {
                if has_phi.insert(fb.clone()) {
                    phi.entry(fb.clone()).or_default().insert(var.clone());
                    if !sites.contains(fb) {
                        work.push(fb.clone());
                    }
                }
            }
        }
    }
    phi
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::{Block, Function, Terminator};
    use crate::expr_ast::ExprNode;
    use tcl_lexer::Span;

    fn make_goto(target: &str) -> Terminator {
        Terminator::Goto {
            target: target.into(),
            span: None,
        }
    }

    fn make_branch(cond: &str, t: &str, f: &str) -> Terminator {
        Terminator::Branch {
            condition: ExprNode::Raw { text: cond.into() },
            true_target: t.into(),
            false_target: f.into(),
            span: None,
        }
    }

    fn make_return() -> Terminator {
        Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        }
    }

    /// Build a diamond CFG: entry → branch → then/else → end → return
    fn diamond_cfg() -> Function {
        let mut func = Function::new("::test", "entry");
        func.blocks.get_mut("entry").unwrap().terminator = Some(make_branch("$x", "then", "else"));
        func.blocks.insert("then".into(), Block::new("then"));
        func.blocks.get_mut("then").unwrap().terminator = Some(make_goto("end"));
        func.blocks.insert("else".into(), Block::new("else"));
        func.blocks.get_mut("else").unwrap().terminator = Some(make_goto("end"));
        func.blocks.insert("end".into(), Block::new("end"));
        func.blocks.get_mut("end").unwrap().terminator = Some(make_return());
        func
    }

    /// Build a loop CFG: entry → header → branch → body → header / end
    fn loop_cfg() -> Function {
        let mut func = Function::new("::test", "entry");
        func.blocks.get_mut("entry").unwrap().terminator = Some(make_goto("header"));
        func.blocks.insert("header".into(), Block::new("header"));
        func.blocks.get_mut("header").unwrap().terminator =
            Some(make_branch("$i < 10", "body", "end"));
        func.blocks.insert("body".into(), Block::new("body"));
        func.blocks.get_mut("body").unwrap().terminator = Some(make_goto("header"));
        func.blocks.insert("end".into(), Block::new("end"));
        func.blocks.get_mut("end").unwrap().terminator = Some(make_return());
        func
    }

    // Data structure tests

    #[test]
    fn phi_construction() {
        let phi = Phi {
            name: "x".into(),
            version: 3,
            incoming: HashMap::from([("then".into(), 1), ("else".into(), 2)]),
        };
        assert_eq!(phi.name, "x");
        assert_eq!(phi.version, 3);
        assert_eq!(phi.incoming.len(), 2);
    }

    #[test]
    fn ssa_statement_construction() {
        let stmt = SsaStatement {
            statement: Statement::AssignConst {
                span: Span::new(0, 10),
                name: "x".into(),
                value: "1".into(),
            },
            uses: HashMap::new(),
            defs: HashMap::from([("x".into(), 1)]),
        };
        assert_eq!(stmt.defs["x"], 1);
        assert!(stmt.uses.is_empty());
    }

    #[test]
    fn ssa_block_construction() {
        let block = SsaBlock {
            name: "entry".into(),
            phis: vec![],
            statements: vec![],
            entry_versions: HashMap::new(),
            exit_versions: HashMap::from([("x".into(), 1)]),
        };
        assert_eq!(block.name, "entry");
        assert!(block.phis.is_empty());
    }

    #[test]
    fn ssa_function_construction() {
        let func = SsaFunction {
            name: "::test".into(),
            entry: "entry".into(),
            blocks: HashMap::new(),
            idom: HashMap::new(),
            dominance_frontier: HashMap::new(),
            dominator_tree: HashMap::new(),
        };
        assert_eq!(func.name, "::test");
    }

    // defs_of tests

    #[test]
    fn defs_of_assign_const() {
        let stmt = Statement::AssignConst {
            span: Span::new(0, 10),
            name: "$x".into(),
            value: "1".into(),
        };
        assert_eq!(defs_of(&stmt), vec!["x"]);
    }

    #[test]
    fn defs_of_incr() {
        let stmt = Statement::Incr {
            span: Span::new(0, 10),
            name: "i".into(),
            amount: None,
            safe_on_uninit: false,
        };
        assert_eq!(defs_of(&stmt), vec!["i"]);
    }

    #[test]
    fn defs_of_call_with_defs() {
        let stmt = Statement::Call {
            span: Span::new(0, 20),
            command: "lappend".into(),
            args: vec!["list".into(), "item".into()],
            defs: vec!["list".into()],
            reads: vec![],
            reads_own_defs: true,
            safe_on_uninit: false,
            tokens: None,
        };
        assert_eq!(defs_of(&stmt), vec!["list"]);
    }

    #[test]
    fn defs_of_call_no_defs() {
        let stmt = Statement::Call {
            span: Span::new(0, 10),
            command: "puts".into(),
            args: vec!["hello".into()],
            defs: vec![],
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
        };
        assert!(defs_of(&stmt).is_empty());
    }

    #[test]
    fn defs_of_return() {
        let stmt = Statement::Return {
            span: Span::new(0, 10),
            value: Some("1".into()),
            expr: None,
            braced: false,
        };
        assert!(defs_of(&stmt).is_empty());
    }

    #[test]
    fn defs_of_barrier_trace() {
        let stmt = Statement::Barrier {
            span: Span::new(0, 30),
            reason: "trace".into(),
            command: "trace".into(),
            args: vec!["add".into(), "variable".into(), "$x".into()],
            tokens: None,
        };
        assert_eq!(defs_of(&stmt), vec!["x"]);
    }

    #[test]
    fn defs_of_barrier_dict_for() {
        let stmt = Statement::Barrier {
            span: Span::new(0, 30),
            reason: "dict for".into(),
            command: "dict::for".into(),
            args: vec!["k v".into(), "$d".into()],
            tokens: None,
        };
        assert_eq!(defs_of(&stmt), vec!["k", "v"]);
    }

    // Dominator tests

    #[test]
    fn dominators_linear() {
        // entry → b1 → b2 → return
        let mut func = Function::new("::test", "entry");
        func.blocks.get_mut("entry").unwrap().terminator = Some(make_goto("b1"));
        func.blocks.insert("b1".into(), Block::new("b1"));
        func.blocks.get_mut("b1").unwrap().terminator = Some(make_goto("b2"));
        func.blocks.insert("b2".into(), Block::new("b2"));
        func.blocks.get_mut("b2").unwrap().terminator = Some(make_return());

        let dom = compute_dominators(&func);
        assert_eq!(dom["entry"], HashSet::from(["entry".into()]));
        assert_eq!(dom["b1"], HashSet::from(["entry".into(), "b1".into()]));
        assert_eq!(
            dom["b2"],
            HashSet::from(["entry".into(), "b1".into(), "b2".into()])
        );
    }

    #[test]
    fn dominators_diamond() {
        let func = diamond_cfg();
        let dom = compute_dominators(&func);

        // entry dominates everything
        for name in func.blocks.keys() {
            assert!(dom[name].contains("entry"));
        }
        // then and else are not dominated by each other
        assert!(!dom["then"].contains("else"));
        assert!(!dom["else"].contains("then"));
        // end is dominated by entry but not by then or else
        assert!(dom["end"].contains("entry"));
        assert!(!dom["end"].contains("then"));
        assert!(!dom["end"].contains("else"));
    }

    #[test]
    fn idom_diamond() {
        let func = diamond_cfg();
        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);

        assert_eq!(idom["entry"], None);
        assert_eq!(idom["then"], Some("entry".into()));
        assert_eq!(idom["else"], Some("entry".into()));
        assert_eq!(idom["end"], Some("entry".into()));
    }

    #[test]
    fn idom_loop() {
        let func = loop_cfg();
        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);

        assert_eq!(idom["entry"], None);
        assert_eq!(idom["header"], Some("entry".into()));
        assert_eq!(idom["body"], Some("header".into()));
        assert_eq!(idom["end"], Some("header".into()));
    }

    #[test]
    fn dominance_frontier_diamond() {
        let func = diamond_cfg();
        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let df = compute_dominance_frontier(&func, &idom);

        // then and else have "end" in their dominance frontier
        assert!(df["then"].contains("end"));
        assert!(df["else"].contains("end"));
        // entry has no dominance frontier
        assert!(df["entry"].is_empty());
    }

    #[test]
    fn dominance_frontier_loop() {
        let func = loop_cfg();
        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let df = compute_dominance_frontier(&func, &idom);

        // body has "header" in its dominance frontier (back edge)
        assert!(df["body"].contains("header"));
        // entry strictly dominates header, so header is NOT in entry's DF
        assert!(
            df["entry"].is_empty(),
            "entry's DF should be empty; got {:?}",
            df["entry"]
        );
    }

    #[test]
    fn dom_tree_diamond() {
        let func = diamond_cfg();
        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let tree = build_dom_tree(&idom);

        // entry's children include then, else, end (all directly dominated)
        let entry_children = &tree["entry"];
        assert!(entry_children.contains(&"else".to_string()));
        assert!(entry_children.contains(&"end".to_string()));
        assert!(entry_children.contains(&"then".to_string()));
    }

    // Phi placement tests

    #[test]
    fn phi_vars_diamond_with_defs() {
        // x defined in both then and else → phi needed at end
        let mut func = diamond_cfg();
        func.blocks
            .get_mut("then")
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(10, 20),
                name: "x".into(),
                value: "1".into(),
            });
        func.blocks
            .get_mut("else")
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(30, 40),
                name: "x".into(),
                value: "2".into(),
            });

        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let df = compute_dominance_frontier(&func, &idom);
        let phi = compute_phi_vars(&func, &df);

        assert!(phi["end"].contains("x"), "x should need a phi at 'end'");
        assert!(
            !phi["entry"].contains("x"),
            "x should not need a phi at entry"
        );
    }

    #[test]
    fn phi_vars_single_def_no_phi() {
        // x defined only in entry → no phi needed anywhere
        let mut func = diamond_cfg();
        func.blocks
            .get_mut("entry")
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(0, 10),
                name: "x".into(),
                value: "1".into(),
            });

        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let df = compute_dominance_frontier(&func, &idom);
        let phi = compute_phi_vars(&func, &df);

        for (_, vars) in &phi {
            assert!(!vars.contains("x"), "x should not need a phi anywhere");
        }
    }

    #[test]
    fn phi_vars_loop_def() {
        // i defined in entry and body → phi at header
        let mut func = loop_cfg();
        func.blocks
            .get_mut("entry")
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(0, 10),
                name: "i".into(),
                value: "0".into(),
            });
        func.blocks
            .get_mut("body")
            .unwrap()
            .statements
            .push(Statement::Incr {
                span: Span::new(30, 40),
                name: "i".into(),
                amount: None,
                safe_on_uninit: false,
            });

        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let df = compute_dominance_frontier(&func, &idom);
        let phi = compute_phi_vars(&func, &df);

        assert!(
            phi["header"].contains("i"),
            "i should need a phi at 'header'"
        );
    }

    #[test]
    fn phi_vars_no_defs_no_phis() {
        let func = diamond_cfg();
        let dom = compute_dominators(&func);
        let idom = compute_idom(&func, &dom);
        let df = compute_dominance_frontier(&func, &idom);
        let phi = compute_phi_vars(&func, &df);

        for (_, vars) in &phi {
            assert!(vars.is_empty(), "no defs → no phis");
        }
    }
}
