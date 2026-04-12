//! Intrep shimmer analysis — detect places where a variable's
//! Tcl-value intrep (list/dict/int/…) is converted at a use site.
//!
//! Ported from `core/compiler/shimmer.py` (C27d). This strip lands
//! the diagnostic types (`ShimmerWarning`, `ThunkingWarning`),
//! supporting lattice/graph helpers (`loop_body_blocks`,
//! `blocks_reaching`), and a stub `find_shimmer_warnings` entry
//! point that returns an empty result. The full use-site / phi /
//! expression-context shimmer detection is a follow-up that
//! depends on the richer `TypeLattice` machinery.

#![allow(clippy::implicit_hasher)]

use std::collections::{HashMap, HashSet};

use tcl_lexer::Span;

use crate::cfg::{Function as CfgFunction, Terminator};
use tcl_registry::TclType;

// ---------------------------------------------------------------------------
// Diagnostic types
// ---------------------------------------------------------------------------

/// A use-site where a variable's intrep is converted.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShimmerWarning {
    /// Source span of the use.
    pub span: Span,
    /// Variable name.
    pub variable: String,
    /// Source intrep kind.
    pub from_type: TclType,
    /// Target intrep kind.
    pub to_type: TclType,
    /// Command that triggered the conversion.
    pub command: String,
    /// Whether the use is inside a loop body.
    pub in_loop: bool,
    /// Diagnostic code (`"S100"` / `"S101"`).
    pub code: String,
    /// Formatted message.
    pub message: String,
    /// Related spans + labels for diagnostic context.
    pub related: Vec<(Span, String)>,
}

/// A variable that oscillates between two types across loop
/// iterations.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThunkingWarning {
    /// Source span.
    pub span: Span,
    /// Variable name.
    pub variable: String,
    /// First observed type.
    pub type_a: TclType,
    /// Second observed type.
    pub type_b: TclType,
    /// Diagnostic code (`"S102"`).
    pub code: String,
    /// Formatted message.
    pub message: String,
    /// Related spans.
    pub related: Vec<(Span, String)>,
}

/// Human-readable name for a Tcl intrep type.
#[must_use]
pub fn type_name(t: TclType) -> String {
    format!("{t:?}").to_ascii_lowercase()
}

// ---------------------------------------------------------------------------
// CFG helpers
// ---------------------------------------------------------------------------

/// Return the set of blocks that are inside a loop body.
///
/// A block is "in a loop" if it is on a cycle — i.e. can reach
/// itself via CFG successor edges.
#[must_use]
pub fn loop_body_blocks(cfg: &CfgFunction) -> HashSet<String> {
    let succs = build_successors(cfg);
    let mut loop_blocks: HashSet<String> = HashSet::new();
    for start in cfg.blocks.keys() {
        if loop_blocks.contains(start) {
            continue;
        }
        let mut visited: HashSet<String> = HashSet::new();
        let initial = succs.get(start).cloned().unwrap_or_default();
        let mut frontier = initial;
        let mut found = false;
        while let Some(bn) = frontier.pop() {
            if bn == *start {
                found = true;
                break;
            }
            if !visited.insert(bn.clone()) {
                continue;
            }
            if let Some(next) = succs.get(&bn) {
                frontier.extend(next.iter().cloned());
            }
        }
        if found {
            // A block is on the cycle iff it was visited on the
            // forward BFS AND it can reach `start` — otherwise
            // the BFS merely passed through it on a dead-end
            // branch (e.g. the exit block).
            let reaching = blocks_reaching(&succs, start);
            loop_blocks.insert(start.clone());
            for name in visited {
                if reaching.contains(&name) {
                    loop_blocks.insert(name);
                }
            }
        }
    }
    loop_blocks
}

/// Return the set of blocks that can reach `target` via `succs`.
#[must_use]
pub fn blocks_reaching(succs: &HashMap<String, Vec<String>>, target: &str) -> HashSet<String> {
    // Build reverse edges and BFS back from target.
    let mut preds: HashMap<String, Vec<String>> = HashMap::new();
    for (src, sx) in succs {
        for dst in sx {
            preds.entry(dst.clone()).or_default().push(src.clone());
        }
    }
    let mut visited: HashSet<String> = HashSet::new();
    let mut frontier: Vec<String> = vec![target.to_owned()];
    while let Some(bn) = frontier.pop() {
        if !visited.insert(bn.clone()) {
            continue;
        }
        if let Some(ps) = preds.get(&bn) {
            frontier.extend(ps.iter().cloned());
        }
    }
    visited
}

fn build_successors(cfg: &CfgFunction) -> HashMap<String, Vec<String>> {
    let mut out: HashMap<String, Vec<String>> = HashMap::new();
    for (bn, block) in &cfg.blocks {
        let mut s: Vec<String> = Vec::new();
        match &block.terminator {
            Some(Terminator::Goto { target, .. }) => {
                if cfg.blocks.contains_key(target) {
                    s.push(target.clone());
                }
            }
            Some(Terminator::Branch {
                true_target,
                false_target,
                ..
            }) => {
                if cfg.blocks.contains_key(true_target) {
                    s.push(true_target.clone());
                }
                if cfg.blocks.contains_key(false_target) {
                    s.push(false_target.clone());
                }
            }
            _ => {}
        }
        out.insert(bn.clone(), s);
    }
    out
}

// ---------------------------------------------------------------------------
// Entry points (stubbed — full analysis deferred)
// ---------------------------------------------------------------------------

/// Find intrep-shimmer warnings for a single function.
///
/// **Current status**: returns an empty vector. The full
/// use-site shimmer detection requires the type lattice and
/// per-command argument-type hints from the registry, plus the
/// phi-shimmer and thunking passes. This stub lets downstream
/// callers integrate the API now; a follow-up strip will flesh
/// out the analysis.
#[must_use]
pub fn find_shimmer_warnings(_cfg: &CfgFunction) -> Vec<ShimmerWarning> {
    Vec::new()
}

/// Find thunking warnings for a single function.
///
/// **Current status**: stub returning an empty vector.
#[must_use]
pub fn find_thunking_warnings(_cfg: &CfgFunction) -> Vec<ThunkingWarning> {
    Vec::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::{Block, Function, Terminator};
    use crate::expr_ast::ExprNode;

    fn branch(cond: ExprNode, tt: &str, ft: &str) -> Terminator {
        Terminator::Branch {
            condition: cond,
            true_target: tt.into(),
            false_target: ft.into(),
            span: None,
        }
    }

    fn goto(target: &str) -> Terminator {
        Terminator::Goto {
            target: target.into(),
            span: None,
        }
    }

    #[test]
    fn loop_body_blocks_detects_simple_cycle() {
        // entry → body → entry (back edge)
        let mut f = Function::new("::top", "entry");
        f.blocks.insert("body".into(), Block::new("body"));
        f.blocks.get_mut("entry").unwrap().terminator = Some(branch(
            ExprNode::Literal {
                text: "1".into(),
                start: 0,
                end: 1,
            },
            "body",
            "exit",
        ));
        f.blocks.insert("exit".into(), Block::new("exit"));
        f.blocks.get_mut("body").unwrap().terminator = Some(goto("entry"));
        f.blocks.get_mut("exit").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        let lb = loop_body_blocks(&f);
        assert!(lb.contains("entry"));
        assert!(lb.contains("body"));
        assert!(!lb.contains("exit"));
    }

    #[test]
    fn loop_body_blocks_linear_chain_is_empty() {
        let mut f = Function::new("::top", "entry");
        f.blocks.insert("next".into(), Block::new("next"));
        f.blocks.get_mut("entry").unwrap().terminator = Some(goto("next"));
        f.blocks.get_mut("next").unwrap().terminator = Some(Terminator::Return {
            value: None,
            span: None,
            expr: None,
            braced: false,
        });
        assert!(loop_body_blocks(&f).is_empty());
    }

    #[test]
    fn blocks_reaching_walks_predecessors() {
        let mut succs: HashMap<String, Vec<String>> = HashMap::new();
        succs.insert("a".into(), vec!["b".into()]);
        succs.insert("b".into(), vec!["c".into()]);
        succs.insert("c".into(), Vec::new());
        let r = blocks_reaching(&succs, "c");
        assert!(r.contains("a"));
        assert!(r.contains("b"));
        assert!(r.contains("c"));
    }

    #[test]
    fn find_shimmer_warnings_is_empty_stub() {
        let f = Function::new("::top", "entry");
        assert!(find_shimmer_warnings(&f).is_empty());
        assert!(find_thunking_warnings(&f).is_empty());
    }

    #[test]
    fn type_name_is_lowercase() {
        // `format!("{:?}", TclType::Int)` renders "Int" →
        // lowercased to "int".
        assert_eq!(type_name(TclType::Int), "int");
    }
}
