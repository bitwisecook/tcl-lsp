// These algorithms always use the default RandomState hasher; making
// them generic over BuildHasher adds complexity for no real benefit.
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
//! The full SSA rename pass ([`build_ssa`]) and variable-use scanner
//! ([`uses_of`]) are now implemented, completing the SSA construction
//! pipeline. The rename pass walks the dominator tree, assigns SSA
//! versions to variable definitions and uses, and fills in phi-node
//! incoming edges.

use std::collections::{BTreeSet, HashMap, HashSet};

use tcl_registry::CommandRegistry;

use crate::cfg;
use crate::ir::{CommandTokens, Statement};
use crate::naming::normalise_var_name;
use crate::var_refs::{vars_in_expr, VarReferenceScanner, VarScanOptions};

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
pub(crate) fn compute_idom(
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
pub(crate) fn compute_dominance_frontier(
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
pub(crate) fn build_dom_tree(
    idom: &HashMap<String, Option<String>>,
) -> HashMap<String, Vec<String>> {
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
pub(crate) fn compute_phi_vars(
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

// Variable-use extraction
//
// These functions determine which variables an IR statement reads.
// They are the Rust port of Python's `_uses()` function in `ssa.py`.

/// Return `true` when argument at `arg_index` is a braced literal
/// (single-token STR word).
///
/// When token info is unavailable, returns `false` so unknown
/// arguments are still scanned as ordinary inputs. We only exclude
/// bodies when we can positively identify them as single-token
/// braced literals.
fn is_braced_arg(tokens: Option<&CommandTokens>, arg_index: usize) -> bool {
    let Some(tokens) = tokens else {
        return false;
    };
    // tokens.argv includes the command name at index 0; args are 1-based.
    let tok_index = arg_index + 1;
    if tok_index >= tokens.single_token_word.len() {
        return false;
    }
    if !tokens.single_token_word[tok_index] {
        return false;
    }
    // A single-token word from a VAR or CMD token is not braced.
    if let Some(text) = tokens.argv_texts.get(tok_index) {
        !text.starts_with("${") && !text.starts_with('[')
    } else {
        true
    }
}

/// Return BODY arg indices that should be excluded from local statement uses.
///
/// We only exclude handler-style bodies that are lowered/analysed separately.
/// Dynamic evaluation commands like `eval` still need their args treated as
/// ordinary dataflow inputs (for taint and read-before-set tracking).
/// Body-carrying options for `tcltest::test`.
const TCLTEST_BODY_OPTIONS: &[&str] = &["-setup", "-body", "-cleanup"];

fn structural_body_indices(
    command: &str,
    args: &[String],
    tokens: Option<&CommandTokens>,
    registry: &CommandRegistry,
) -> HashSet<usize> {
    use tcl_registry::ArgRole;

    if command == "when" || command == "proc" {
        let arg_strs: Vec<&str> = args.iter().map(String::as_str).collect();
        let candidates = registry.arg_indices_for_role(command, &arg_strs, ArgRole::Body);
        return candidates
            .into_iter()
            .filter(|&idx| idx < args.len() && is_braced_arg(tokens, idx))
            .collect();
    }

    if command == "test" || command == "tcltest::test" {
        let mut indices = HashSet::new();
        let mut has_body_option = false;
        let mut i = 2;
        while i + 1 < args.len() {
            if TCLTEST_BODY_OPTIONS.contains(&args[i].as_str()) {
                let value_idx = i + 1;
                has_body_option = true;
                if is_braced_arg(tokens, value_idx) {
                    indices.insert(value_idx);
                }
            }
            i += 2;
        }
        // Legacy positional form: test name desc ?constraints? body result
        if !has_body_option && args.len() >= 4 {
            let body_index = args.len() - 2;
            if is_braced_arg(tokens, body_index) {
                indices.insert(body_index);
            }
        }
        return indices;
    }

    HashSet::new()
}

/// Extract variable names used (read) by an IR statement.
///
/// This is the Rust port of Python's `_uses()` in `ssa.py`. It uses
/// a [`VarReferenceScanner`] to find variable references in word texts
/// and expression trees.
///
/// Returns sorted variable names, excluding variables that are defined
/// by this statement (unless they exhibit read-before-write semantics).
pub fn uses_of(
    stmt: &Statement,
    scanner: &mut VarReferenceScanner,
    registry: &CommandRegistry,
) -> Vec<String> {
    let mut vars_found: BTreeSet<String> = BTreeSet::new();
    let mut reads_own_def: BTreeSet<String> = BTreeSet::new();

    match stmt {
        Statement::ExprEval { expr, .. } => {
            vars_found.extend(vars_in_expr(expr));
        }

        Statement::AssignExpr { name, expr, .. } => {
            vars_found.extend(vars_in_expr(expr));
            let norm = normalise_var_name(name);
            if !norm.is_empty() && vars_found.contains(norm) {
                reads_own_def.insert(norm.to_owned());
            }
        }

        Statement::AssignValue { name, value, .. } => {
            vars_found.extend(scanner.scan_word(value, registry));
            let norm = normalise_var_name(name);
            if !norm.is_empty() && vars_found.contains(norm) {
                reads_own_def.insert(norm.to_owned());
            }
        }

        Statement::Incr { name, amount, .. } => {
            let norm = normalise_var_name(name);
            if !norm.is_empty() {
                vars_found.insert(norm.to_owned());
                reads_own_def.insert(norm.to_owned());
            }
            if let Some(amt) = amount {
                vars_found.extend(scanner.scan_word(amt, registry));
            }
        }

        Statement::Call {
            command,
            args,
            defs,
            reads,
            reads_own_defs,
            tokens,
            ..
        } => {
            vars_found.extend(scanner.scan_word(command, registry));
            let body_indices = structural_body_indices(command, args, tokens.as_ref(), registry);
            for (idx, arg) in args.iter().enumerate() {
                if body_indices.contains(&idx) {
                    continue;
                }
                vars_found.extend(scanner.scan_word(arg, registry));
            }
            for name in reads {
                if !name.is_empty() {
                    vars_found.insert(name.clone());
                }
            }
            if *reads_own_defs {
                for name in defs {
                    vars_found.insert(name.clone());
                    reads_own_def.insert(name.clone());
                }
            }
        }

        Statement::Return { value, expr, .. } => {
            if let Some(v) = value {
                vars_found.extend(scanner.scan_word(v, registry));
            }
            if let Some(e) = expr {
                vars_found.extend(vars_in_expr(e));
            }
        }

        Statement::Barrier {
            command,
            args,
            tokens,
            ..
        } => {
            vars_found.extend(scanner.scan_word(command, registry));
            let body_indices = structural_body_indices(command, args, tokens.as_ref(), registry);
            for (idx, arg) in args.iter().enumerate() {
                if body_indices.contains(&idx) {
                    continue;
                }
                vars_found.extend(scanner.scan_word(arg, registry));
            }
            // dict with/update: the dict variable name is a plain string,
            // not a $-substitution, so scan_word misses it.
            if command == "dict" && args.len() >= 2 && (args[0] == "with" || args[0] == "update") {
                let dict_var = normalise_var_name(&args[1]);
                if !dict_var.is_empty() {
                    vars_found.insert(dict_var.to_owned());
                }
            }
        }

        // Structured IR statements (If, For, While, etc.) are not
        // present in CFG blocks — they're flattened by the CFG builder
        // before SSA construction.
        _ => {}
    }

    // Exclude variables defined by this statement, unless they're
    // read-before-write.
    let defs: HashSet<String> = defs_of(stmt).into_iter().collect();
    vars_found
        .into_iter()
        .filter(|v| !v.is_empty() && (!defs.contains(v) || reads_own_def.contains(v)))
        .collect()
}

/// Build SSA with dominator-based phi placement and renaming.
///
/// This is the Rust port of Python's `build_ssa()` in `ssa.py`.
/// It computes dominators, places phi nodes, then walks the dominator
/// tree to assign SSA version numbers to every variable definition
/// and use.
/// Frame for the iterative rename walk (avoids deep recursion).
struct RenameFrame {
    block_name: String,
    child_index: usize,
    pushed_vars: Vec<String>,
    phase: RenamePhase,
}

/// Phase within a single rename frame.
enum RenamePhase {
    /// Process phi nodes and statements for this block.
    Enter,
    /// Iterate over dominator-tree children.
    ProcessChildren,
}

/// Build SSA with dominator-based phi placement and renaming.
///
/// Computes dominators, places phi nodes, then walks the dominator
/// tree to assign SSA version numbers to every variable definition
/// and use.
///
/// This function is inherently long because the rename walk couples
/// version counters, stacks, phi versions, incoming edges, and
/// per-statement use/def maps — splitting it would just scatter the
/// state across many parameters.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn build_ssa(func: &cfg::Function, registry: &CommandRegistry) -> SsaFunction {
    // 1. Compute dominance information.
    let dom = compute_dominators(func);
    let idom = compute_idom(func, &dom);
    let df = compute_dominance_frontier(func, &idom);
    let tree = build_dom_tree(&idom);
    let phi_vars = compute_phi_vars(func, &df);

    // 2. Set up rename state.
    let mut version_counter: HashMap<String, Version> = HashMap::new();
    let mut stacks: HashMap<String, Vec<Version>> = HashMap::new();

    let top = |stacks: &HashMap<String, Vec<Version>>, var: &str| -> Version {
        stacks.get(var).and_then(|s| s.last().copied()).unwrap_or(0)
    };

    let push_new = |version_counter: &mut HashMap<String, Version>,
                    stacks: &mut HashMap<String, Vec<Version>>,
                    var: &str|
     -> Version {
        let vn = version_counter.get(var).copied().unwrap_or(0) + 1;
        version_counter.insert(var.to_owned(), vn);
        stacks.entry(var.to_owned()).or_default().push(vn);
        vn
    };

    // Per-block state collected during the rename walk.
    let mut phi_versions: HashMap<String, HashMap<String, Version>> = HashMap::new();
    let mut phi_incoming: HashMap<String, HashMap<String, HashMap<String, Version>>> =
        HashMap::new();
    let mut entry_versions: HashMap<String, HashMap<String, Version>> = HashMap::new();
    let mut exit_versions: HashMap<String, HashMap<String, Version>> = HashMap::new();
    let mut stmt_infos: HashMap<String, Vec<SsaStatement>> = HashMap::new();

    for name in func.blocks.keys() {
        phi_versions.insert(name.clone(), HashMap::new());
        phi_incoming.insert(name.clone(), HashMap::new());
        entry_versions.insert(name.clone(), HashMap::new());
        exit_versions.insert(name.clone(), HashMap::new());
        stmt_infos.insert(name.clone(), Vec::new());
    }

    // Create a scanner for variable-use extraction.
    let mut scanner = VarReferenceScanner::new(VarScanOptions {
        include_var_read_roles: true,
        recurse_cmd_substitutions: true,
    });

    // 3. Rename walk — iterative using an explicit stack to avoid
    //    deep recursion on large dominator trees.
    let mut stack: Vec<RenameFrame> = Vec::new();

    if func.blocks.contains_key(&func.entry) {
        stack.push(RenameFrame {
            block_name: func.entry.clone(),
            child_index: 0,
            pushed_vars: Vec::new(),
            phase: RenamePhase::Enter,
        });
    }

    while let Some(frame) = stack.last_mut() {
        match frame.phase {
            RenamePhase::Enter => {
                let bn = frame.block_name.clone();

                // Process phi nodes — push new versions.
                let mut phi_var_list: Vec<String> = phi_vars
                    .get(&bn)
                    .map(|s| s.iter().cloned().collect())
                    .unwrap_or_default();
                phi_var_list.sort();

                for var in &phi_var_list {
                    let ver = push_new(&mut version_counter, &mut stacks, var);
                    frame.pushed_vars.push(var.clone());
                    phi_versions.get_mut(&bn).unwrap().insert(var.clone(), ver);
                    phi_incoming
                        .get_mut(&bn)
                        .unwrap()
                        .entry(var.clone())
                        .or_default();
                }

                // Record entry versions.
                let mut visible_vars: BTreeSet<String> = stacks.keys().cloned().collect();
                visible_vars.extend(phi_versions[&bn].keys().cloned());
                let ev: HashMap<String, Version> = visible_vars
                    .iter()
                    .filter_map(|v| {
                        let t = top(&stacks, v);
                        if t > 0 {
                            Some((v.clone(), t))
                        } else {
                            None
                        }
                    })
                    .collect();
                *entry_versions.get_mut(&bn).unwrap() = ev;

                // Process statements.
                if let Some(block) = func.blocks.get(&bn) {
                    let stmts: Vec<Statement> = block.statements.clone();
                    for stmt in &stmts {
                        let uses_list = uses_of(stmt, &mut scanner, registry);
                        let mut uses_map: HashMap<String, Version> = HashMap::new();
                        for var in &uses_list {
                            uses_map.insert(var.clone(), top(&stacks, var));
                        }

                        let mut defs_map: HashMap<String, Version> = HashMap::new();
                        for var in defs_of(stmt) {
                            let ver = push_new(&mut version_counter, &mut stacks, &var);
                            frame.pushed_vars.push(var.clone());
                            defs_map.insert(var, ver);
                        }

                        stmt_infos.get_mut(&bn).unwrap().push(SsaStatement {
                            statement: stmt.clone(),
                            uses: uses_map,
                            defs: defs_map,
                        });
                    }
                }

                // Record exit versions.
                let mut visible_vars: BTreeSet<String> = stacks.keys().cloned().collect();
                visible_vars.extend(phi_versions[&bn].keys().cloned());
                let xv: HashMap<String, Version> = visible_vars
                    .iter()
                    .filter_map(|v| {
                        let t = top(&stacks, v);
                        if t > 0 {
                            Some((v.clone(), t))
                        } else {
                            None
                        }
                    })
                    .collect();
                *exit_versions.get_mut(&bn).unwrap() = xv;

                // Fill in phi incoming edges for successors.
                if let Some(block) = func.blocks.get(&bn) {
                    if let Some(ref term) = block.terminator {
                        for succ in term.successors() {
                            if !func.blocks.contains_key(succ) {
                                continue;
                            }
                            let succ_phis: Vec<String> = phi_vars
                                .get(succ)
                                .map(|s| {
                                    let mut v: Vec<String> = s.iter().cloned().collect();
                                    v.sort();
                                    v
                                })
                                .unwrap_or_default();
                            for var in &succ_phis {
                                phi_incoming
                                    .get_mut(succ)
                                    .unwrap()
                                    .entry(var.clone())
                                    .or_default()
                                    .insert(bn.clone(), top(&stacks, var));
                            }
                        }
                    }
                }

                frame.phase = RenamePhase::ProcessChildren;
            }

            RenamePhase::ProcessChildren => {
                let bn = frame.block_name.clone();
                let children = tree.get(&bn).cloned().unwrap_or_default();
                let idx = frame.child_index;

                if idx < children.len() {
                    frame.child_index += 1;
                    let child = children[idx].clone();
                    stack.push(RenameFrame {
                        block_name: child,
                        child_index: 0,
                        pushed_vars: Vec::new(),
                        phase: RenamePhase::Enter,
                    });
                } else {
                    // Pop versions pushed in this block.
                    let pushed = frame.pushed_vars.clone();
                    for var in pushed.iter().rev() {
                        if let Some(s) = stacks.get_mut(var) {
                            s.pop();
                            if s.is_empty() {
                                stacks.remove(var);
                            }
                        }
                    }
                    stack.pop();
                }
            }
        }
    }

    // 4. Assemble SSA blocks.
    let mut ssa_blocks: HashMap<String, SsaBlock> = HashMap::new();
    for (bn, block) in &func.blocks {
        let mut phis: Vec<Phi> = Vec::new();
        let mut phi_var_list: Vec<String> = phi_vars
            .get(bn)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default();
        phi_var_list.sort();

        for var in &phi_var_list {
            phis.push(Phi {
                name: var.clone(),
                version: phi_versions
                    .get(bn)
                    .and_then(|m| m.get(var))
                    .copied()
                    .unwrap_or(0),
                incoming: phi_incoming
                    .get(bn)
                    .and_then(|m| m.get(var))
                    .cloned()
                    .unwrap_or_default(),
            });
        }

        ssa_blocks.insert(
            bn.clone(),
            SsaBlock {
                name: bn.clone(),
                phis,
                statements: stmt_infos.remove(bn).unwrap_or_default(),
                entry_versions: entry_versions.remove(bn).unwrap_or_default(),
                exit_versions: exit_versions.remove(bn).unwrap_or_default(),
            },
        );
        let _ = block; // used only for iteration
    }

    SsaFunction {
        name: func.name.clone(),
        entry: func.entry.clone(),
        blocks: ssa_blocks,
        idom,
        dominance_frontier: df
            .into_iter()
            .map(|(k, v)| {
                let mut sorted: Vec<String> = v.into_iter().collect();
                sorted.sort();
                (k, sorted)
            })
            .collect(),
        dominator_tree: tree,
    }
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

        for vars in phi.values() {
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

        for vars in phi.values() {
            assert!(vars.is_empty(), "no defs → no phis");
        }
    }

    // uses_of tests

    fn default_registry() -> CommandRegistry {
        CommandRegistry::build_default()
    }

    #[test]
    fn uses_of_assign_const() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let stmt = Statement::AssignConst {
            span: Span::new(0, 10),
            name: "x".into(),
            value: "1".into(),
        };
        let uses = uses_of(&stmt, &mut scanner, &reg);
        assert!(uses.is_empty(), "constant assignment reads nothing");
    }

    #[test]
    fn uses_of_assign_value_with_var() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions {
            include_var_read_roles: true,
            recurse_cmd_substitutions: true,
        });
        let stmt = Statement::AssignValue {
            span: Span::new(0, 15),
            name: "y".into(),
            value: "$x".into(),
            value_needs_backsubst: false,
            tokens: None,
        };
        let uses = uses_of(&stmt, &mut scanner, &reg);
        assert!(uses.contains(&"x".to_string()), "should read $x");
        assert!(
            !uses.contains(&"y".to_string()),
            "should not read $y (it's defined)"
        );
    }

    #[test]
    fn uses_of_incr() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let stmt = Statement::Incr {
            span: Span::new(0, 10),
            name: "i".into(),
            amount: None,
            safe_on_uninit: false,
        };
        let uses = uses_of(&stmt, &mut scanner, &reg);
        // incr reads and writes — reads_own_def
        assert!(uses.contains(&"i".to_string()), "incr reads the variable");
    }

    #[test]
    fn uses_of_return_with_value() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let stmt = Statement::Return {
            span: Span::new(0, 15),
            value: Some("$result".into()),
            expr: None,
            braced: false,
        };
        let uses = uses_of(&stmt, &mut scanner, &reg);
        assert!(uses.contains(&"result".to_string()));
    }

    #[test]
    fn uses_of_expr_eval() {
        let reg = default_registry();
        let mut scanner = VarReferenceScanner::new(VarScanOptions::default());
        let stmt = Statement::ExprEval {
            span: Span::new(0, 20),
            expr: ExprNode::Binary {
                op: crate::expr_ast::BinOp::Add,
                left: Box::new(ExprNode::Var {
                    text: "$a".into(),
                    name: "a".into(),
                    start: 0,
                    end: 2,
                }),
                right: Box::new(ExprNode::Var {
                    text: "$b".into(),
                    name: "b".into(),
                    start: 5,
                    end: 7,
                }),
            },
        };
        let uses = uses_of(&stmt, &mut scanner, &reg);
        assert!(uses.contains(&"a".to_string()));
        assert!(uses.contains(&"b".to_string()));
    }

    // build_ssa tests

    #[test]
    fn build_ssa_linear() {
        let reg = default_registry();
        // entry: set x 1; set y $x; return
        let mut func = Function::new("::test", "entry");
        func.blocks.get_mut("entry").unwrap().statements = vec![
            Statement::AssignConst {
                span: Span::new(0, 7),
                name: "x".into(),
                value: "1".into(),
            },
            Statement::AssignValue {
                span: Span::new(8, 16),
                name: "y".into(),
                value: "$x".into(),
                value_needs_backsubst: false,
                tokens: None,
            },
        ];
        func.blocks.get_mut("entry").unwrap().terminator = Some(make_return());

        let ssa = build_ssa(&func, &reg);
        assert_eq!(ssa.name, "::test");
        assert_eq!(ssa.entry, "entry");

        let entry = &ssa.blocks["entry"];
        // First statement (set x 1): defs x=1
        assert_eq!(entry.statements[0].defs.get("x"), Some(&1));
        // Second statement (set y $x): uses x=1, defs y=1
        assert_eq!(entry.statements[1].uses.get("x"), Some(&1));
        assert_eq!(entry.statements[1].defs.get("y"), Some(&1));
    }

    #[test]
    fn build_ssa_diamond_phi() {
        let reg = default_registry();
        // entry → branch on $x → then: set x 1 → end
        //                       → else: set x 2 → end
        let mut func = diamond_cfg();

        // Define x in entry first so it's used in the condition.
        func.blocks
            .get_mut("entry")
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(0, 7),
                name: "x".into(),
                value: "0".into(),
            });

        func.blocks
            .get_mut("then")
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(10, 18),
                name: "x".into(),
                value: "1".into(),
            });
        func.blocks
            .get_mut("else")
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(20, 28),
                name: "x".into(),
                value: "2".into(),
            });

        let ssa = build_ssa(&func, &reg);

        // x is defined in both then and else, so end block should have a phi for x.
        let end_block = &ssa.blocks["end"];
        assert!(
            end_block.phis.iter().any(|phi| phi.name == "x"),
            "end block should have a phi for x"
        );

        // The phi should have incoming edges from then and else.
        if let Some(phi) = end_block.phis.iter().find(|p| p.name == "x") {
            assert!(
                phi.incoming.contains_key("then"),
                "phi should have incoming from then"
            );
            assert!(
                phi.incoming.contains_key("else"),
                "phi should have incoming from else"
            );
        }
    }

    #[test]
    fn build_ssa_loop() {
        let reg = default_registry();
        // entry: set i 0 → header: branch $i<10 → body: incr i → header
        //                                        → end: return
        let mut func = loop_cfg();

        func.blocks
            .get_mut("entry")
            .unwrap()
            .statements
            .push(Statement::AssignConst {
                span: Span::new(0, 8),
                name: "i".into(),
                value: "0".into(),
            });
        func.blocks
            .get_mut("body")
            .unwrap()
            .statements
            .push(Statement::Incr {
                span: Span::new(20, 28),
                name: "i".into(),
                amount: None,
                safe_on_uninit: false,
            });

        let ssa = build_ssa(&func, &reg);

        // header should have a phi for i (from entry and body).
        let header = &ssa.blocks["header"];
        assert!(
            header.phis.iter().any(|p| p.name == "i"),
            "header should have a phi for i"
        );
    }

    #[test]
    fn build_ssa_empty_function() {
        let reg = default_registry();
        let mut func = Function::new("::empty", "entry");
        func.blocks.get_mut("entry").unwrap().terminator = Some(make_return());

        let ssa = build_ssa(&func, &reg);
        assert_eq!(ssa.blocks.len(), 1);
        assert!(ssa.blocks["entry"].phis.is_empty());
        assert!(ssa.blocks["entry"].statements.is_empty());
    }
}
