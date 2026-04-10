//! Control-flow graph (CFG) representation for Tcl procedures.
//!
//! A CFG represents a procedure as a set of *basic blocks* —
//! straight-line sequences of IR statements with no internal
//! branching. Each block ends with a *terminator* that transfers
//! control:
//!
//! - [`Terminator::Goto`] — unconditional jump to one successor.
//! - [`Terminator::Branch`] — conditional jump to one of two successors.
//! - [`Terminator::Return`] — procedure exit.
//!
//! Structured IR constructs ([`Statement::If`], [`Statement::For`],
//! [`Statement::Switch`], etc.) are flattened into this graph form so
//! that SSA and dataflow analyses can reason about all possible
//! execution paths.
//!
//! **Architectural note:** terminators and blocks carry [`Span`]s (not
//! inline position pairs), matching the span-first architecture
//! established in the lexer and IR crates. The [`ExprNode`] in a
//! [`Terminator::Branch`] condition is the same type produced by the
//! expression parser (C1).

use std::collections::{HashMap, HashSet, VecDeque};

use tcl_lexer::Span;

use crate::expr_ast::ExprNode;
use crate::ir::Statement;

// Terminators

/// How control leaves a basic block.
#[derive(Debug, Clone, PartialEq)]
pub enum Terminator {
    /// Unconditional jump to a single successor.
    Goto {
        /// Target block name.
        target: String,
        /// Source span of the jump (e.g. closing brace of a body).
        span: Option<Span>,
    },

    /// Conditional jump: evaluate `condition`, then jump to
    /// `true_target` or `false_target`.
    Branch {
        /// The condition expression.
        condition: ExprNode,
        /// Block name when condition is true.
        true_target: String,
        /// Block name when condition is false.
        false_target: String,
        /// Source span of the condition.
        span: Option<Span>,
    },

    /// Procedure exit.
    Return {
        /// Return value text, if any.
        value: Option<String>,
        /// Source span of the return statement.
        span: Option<Span>,
        /// Parsed return expression, if any.
        expr: Option<ExprNode>,
        /// Whether the return value was braced.
        braced: bool,
    },
}

impl Terminator {
    /// Return the names of all successor blocks.
    #[must_use]
    pub fn successors(&self) -> Vec<&str> {
        match self {
            Self::Goto { target, .. } => vec![target],
            Self::Branch {
                true_target,
                false_target,
                ..
            } => vec![true_target, false_target],
            Self::Return { .. } => vec![],
        }
    }

    /// Return the source span, if any.
    #[must_use]
    pub fn span(&self) -> Option<Span> {
        match self {
            Self::Goto { span, .. } | Self::Branch { span, .. } | Self::Return { span, .. } => {
                *span
            }
        }
    }
}

// Basic block

/// A basic block: a named sequence of IR statements ending with
/// an optional terminator.
///
/// Blocks are the fundamental unit of the control-flow graph.
/// Statements within a block execute sequentially; control flow
/// between blocks is explicit via terminators.
#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    /// Unique block name (e.g. `"entry_1"`, `"if_then_2"`).
    pub name: String,
    /// Statements in execution order.
    pub statements: Vec<Statement>,
    /// How control leaves this block (`None` for unreachable/incomplete blocks).
    pub terminator: Option<Terminator>,
}

impl Block {
    /// Create a new empty block with the given name.
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            statements: Vec::new(),
            terminator: None,
        }
    }

    /// Return the names of all successor blocks.
    #[must_use]
    pub fn successors(&self) -> Vec<&str> {
        match &self.terminator {
            Some(t) => t.successors(),
            None => vec![],
        }
    }
}

// Function-level CFG

/// Metadata about a loop in the CFG.
///
/// Maps from the loop's exit block name to its entry block name and
/// the original `IRFor` statement. Used by loop analyses and the
/// bottom-tested loop rewriter in codegen.
#[derive(Debug, Clone, PartialEq)]
pub struct LoopNode {
    /// Name of the loop header/entry block.
    pub entry_block: String,
    /// Source span of the original `for` statement.
    pub span: Span,
}

/// A complete control-flow graph for a single procedure or top-level script.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// Fully qualified procedure name (e.g. `"::top"`, `"::ns::proc"`).
    pub name: String,
    /// Name of the entry block.
    pub entry: String,
    /// All blocks in the function, keyed by block name.
    pub blocks: HashMap<String, Block>,
    /// Loop metadata: exit block → loop info.
    pub loop_nodes: HashMap<String, LoopNode>,
}

impl Function {
    /// Create a new function with a single empty entry block.
    #[must_use]
    pub fn new(name: impl Into<String>, entry: impl Into<String>) -> Self {
        let entry = entry.into();
        let mut blocks = HashMap::new();
        blocks.insert(entry.clone(), Block::new(entry.clone()));
        Self {
            name: name.into(),
            entry,
            blocks,
            loop_nodes: HashMap::new(),
        }
    }

    /// Compute the predecessor map: block name → set of predecessor block names.
    #[must_use]
    pub fn predecessors(&self) -> HashMap<String, HashSet<String>> {
        let mut preds: HashMap<String, HashSet<String>> = HashMap::new();
        for name in self.blocks.keys() {
            preds.entry(name.clone()).or_default();
        }
        for (name, block) in &self.blocks {
            for succ in block.successors() {
                preds
                    .entry(succ.to_owned())
                    .or_default()
                    .insert(name.clone());
            }
        }
        preds
    }

    /// Return the set of block names reachable from the entry block.
    #[must_use]
    pub fn reachable_blocks(&self) -> HashSet<String> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(self.entry.clone());
        while let Some(name) = queue.pop_front() {
            if !visited.insert(name.clone()) {
                continue;
            }
            if let Some(block) = self.blocks.get(&name) {
                for succ in block.successors() {
                    if !visited.contains(succ) {
                        queue.push_back(succ.to_owned());
                    }
                }
            }
        }
        visited
    }

    /// Return block names in reverse post-order (RPO) from the entry.
    ///
    /// RPO is the standard traversal order for forward dataflow
    /// analyses: every block is visited after all its non-back-edge
    /// predecessors.
    #[must_use]
    pub fn reverse_postorder(&self) -> Vec<String> {
        let mut visited = HashSet::new();
        let mut postorder = Vec::new();
        self.dfs_postorder(&self.entry, &mut visited, &mut postorder);
        postorder.reverse();
        postorder
    }

    /// Depth-first postorder traversal helper.
    fn dfs_postorder(
        &self,
        name: &str,
        visited: &mut HashSet<String>,
        postorder: &mut Vec<String>,
    ) {
        if !visited.insert(name.to_owned()) {
            return;
        }
        if let Some(block) = self.blocks.get(name) {
            for succ in block.successors() {
                self.dfs_postorder(succ, visited, postorder);
            }
        }
        postorder.push(name.to_owned());
    }
}

// Module-level CFG

/// A complete CFG for a module: top-level script + all procedures.
#[derive(Debug, Clone, PartialEq)]
pub struct CfgModule {
    /// CFG for the top-level script.
    pub top_level: Function,
    /// CFGs for named procedures, keyed by qualified name.
    pub procedures: HashMap<String, Function>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_goto(target: &str) -> Terminator {
        Terminator::Goto {
            target: target.into(),
            span: None,
        }
    }

    fn make_branch(cond_text: &str, t: &str, f: &str) -> Terminator {
        Terminator::Branch {
            condition: ExprNode::Raw {
                text: cond_text.into(),
            },
            true_target: t.into(),
            false_target: f.into(),
            span: None,
        }
    }

    fn make_return(value: Option<&str>) -> Terminator {
        Terminator::Return {
            value: value.map(String::from),
            span: None,
            expr: None,
            braced: false,
        }
    }

    // Terminator tests

    #[test]
    fn goto_successors() {
        let t = make_goto("block_2");
        assert_eq!(t.successors(), vec!["block_2"]);
    }

    #[test]
    fn branch_successors() {
        let t = make_branch("$x", "then", "else");
        let mut succs = t.successors();
        succs.sort_unstable();
        assert_eq!(succs, vec!["else", "then"]);
    }

    #[test]
    fn return_successors() {
        let t = make_return(Some("1"));
        assert!(t.successors().is_empty());
    }

    #[test]
    fn terminator_span_none() {
        let t = make_goto("b");
        assert!(t.span().is_none());
    }

    #[test]
    fn terminator_span_some() {
        let t = Terminator::Goto {
            target: "b".into(),
            span: Some(Span::new(0, 5)),
        };
        assert_eq!(t.span(), Some(Span::new(0, 5)));
    }

    // Block tests

    #[test]
    fn empty_block() {
        let block = Block::new("entry");
        assert_eq!(block.name, "entry");
        assert!(block.statements.is_empty());
        assert!(block.terminator.is_none());
        assert!(block.successors().is_empty());
    }

    #[test]
    fn block_with_goto() {
        let mut block = Block::new("b1");
        block.terminator = Some(make_goto("b2"));
        assert_eq!(block.successors(), vec!["b2"]);
    }

    #[test]
    fn block_with_statements() {
        let mut block = Block::new("b1");
        block.statements.push(Statement::AssignConst {
            span: Span::new(0, 10),
            name: "x".into(),
            value: "1".into(),
        });
        block.terminator = Some(make_return(None));
        assert_eq!(block.statements.len(), 1);
        assert!(block.successors().is_empty());
    }

    // Function tests

    #[test]
    fn new_function_has_entry() {
        let func = Function::new("::test", "entry_1");
        assert_eq!(func.name, "::test");
        assert_eq!(func.entry, "entry_1");
        assert!(func.blocks.contains_key("entry_1"));
    }

    #[test]
    fn predecessors_simple() {
        // entry → b1 → b2
        let mut func = Function::new("::test", "entry");
        func.blocks.get_mut("entry").unwrap().terminator = Some(make_goto("b1"));
        func.blocks.insert("b1".into(), Block::new("b1"));
        func.blocks.get_mut("b1").unwrap().terminator = Some(make_goto("b2"));
        func.blocks.insert("b2".into(), Block::new("b2"));
        func.blocks.get_mut("b2").unwrap().terminator = Some(make_return(None));

        let preds = func.predecessors();
        assert!(preds["entry"].is_empty());
        assert_eq!(preds["b1"], HashSet::from(["entry".into()]));
        assert_eq!(preds["b2"], HashSet::from(["b1".into()]));
    }

    #[test]
    fn predecessors_branch() {
        // entry → (branch) → then / else → end
        let mut func = Function::new("::test", "entry");
        func.blocks.get_mut("entry").unwrap().terminator = Some(make_branch("$x", "then", "else"));
        func.blocks.insert("then".into(), Block::new("then"));
        func.blocks.get_mut("then").unwrap().terminator = Some(make_goto("end"));
        func.blocks.insert("else".into(), Block::new("else"));
        func.blocks.get_mut("else").unwrap().terminator = Some(make_goto("end"));
        func.blocks.insert("end".into(), Block::new("end"));
        func.blocks.get_mut("end").unwrap().terminator = Some(make_return(None));

        let preds = func.predecessors();
        assert!(preds["entry"].is_empty());
        assert_eq!(preds["then"], HashSet::from(["entry".into()]));
        assert_eq!(preds["else"], HashSet::from(["entry".into()]));
        assert_eq!(preds["end"], HashSet::from(["then".into(), "else".into()]));
    }

    #[test]
    fn reachable_blocks_simple() {
        let mut func = Function::new("::test", "entry");
        func.blocks.get_mut("entry").unwrap().terminator = Some(make_goto("b1"));
        func.blocks.insert("b1".into(), Block::new("b1"));
        func.blocks.get_mut("b1").unwrap().terminator = Some(make_return(None));
        // Unreachable block
        func.blocks
            .insert("unreachable".into(), Block::new("unreachable"));

        let reachable = func.reachable_blocks();
        assert!(reachable.contains("entry"));
        assert!(reachable.contains("b1"));
        assert!(!reachable.contains("unreachable"));
    }

    #[test]
    fn reachable_blocks_with_loop() {
        // entry → header → (branch) → body → header / end
        let mut func = Function::new("::test", "entry");
        func.blocks.get_mut("entry").unwrap().terminator = Some(make_goto("header"));
        func.blocks.insert("header".into(), Block::new("header"));
        func.blocks.get_mut("header").unwrap().terminator =
            Some(make_branch("$i < 10", "body", "end"));
        func.blocks.insert("body".into(), Block::new("body"));
        func.blocks.get_mut("body").unwrap().terminator = Some(make_goto("header"));
        func.blocks.insert("end".into(), Block::new("end"));
        func.blocks.get_mut("end").unwrap().terminator = Some(make_return(None));

        let reachable = func.reachable_blocks();
        assert_eq!(reachable.len(), 4);
        assert!(reachable.contains("entry"));
        assert!(reachable.contains("header"));
        assert!(reachable.contains("body"));
        assert!(reachable.contains("end"));
    }

    #[test]
    fn reverse_postorder_diamond() {
        // entry → (branch) → then / else → end
        let mut func = Function::new("::test", "entry");
        func.blocks.get_mut("entry").unwrap().terminator = Some(make_branch("$x", "then", "else"));
        func.blocks.insert("then".into(), Block::new("then"));
        func.blocks.get_mut("then").unwrap().terminator = Some(make_goto("end"));
        func.blocks.insert("else".into(), Block::new("else"));
        func.blocks.get_mut("else").unwrap().terminator = Some(make_goto("end"));
        func.blocks.insert("end".into(), Block::new("end"));
        func.blocks.get_mut("end").unwrap().terminator = Some(make_return(None));

        let rpo = func.reverse_postorder();
        // entry comes first, end comes last
        assert_eq!(rpo[0], "entry");
        assert_eq!(*rpo.last().unwrap(), "end");
        // then and else are between entry and end
        let then_pos = rpo.iter().position(|n| n == "then").unwrap();
        let else_pos = rpo.iter().position(|n| n == "else").unwrap();
        let end_pos = rpo.iter().position(|n| n == "end").unwrap();
        assert!(then_pos < end_pos);
        assert!(else_pos < end_pos);
    }

    #[test]
    fn reverse_postorder_loop() {
        // entry → header → (branch) → body → header / end
        let mut func = Function::new("::test", "entry");
        func.blocks.get_mut("entry").unwrap().terminator = Some(make_goto("header"));
        func.blocks.insert("header".into(), Block::new("header"));
        func.blocks.get_mut("header").unwrap().terminator =
            Some(make_branch("$i < 10", "body", "end"));
        func.blocks.insert("body".into(), Block::new("body"));
        func.blocks.get_mut("body").unwrap().terminator = Some(make_goto("header"));
        func.blocks.insert("end".into(), Block::new("end"));
        func.blocks.get_mut("end").unwrap().terminator = Some(make_return(None));

        let rpo = func.reverse_postorder();
        assert_eq!(rpo[0], "entry");
        // header before body (non-back-edge predecessor)
        let header_pos = rpo.iter().position(|n| n == "header").unwrap();
        let body_pos = rpo.iter().position(|n| n == "body").unwrap();
        assert!(header_pos < body_pos);
    }

    // Module tests

    #[test]
    fn module_construction() {
        let top = Function::new("::top", "entry_1");
        let proc = Function::new("::greet", "entry_2");
        let module = CfgModule {
            top_level: top,
            procedures: HashMap::from([("::greet".into(), proc)]),
        };
        assert_eq!(module.top_level.name, "::top");
        assert!(module.procedures.contains_key("::greet"));
    }

    // Clone and equality

    #[test]
    fn clone_preserves_equality() {
        let mut func = Function::new("::test", "entry");
        func.blocks.get_mut("entry").unwrap().terminator = Some(make_return(Some("0")));
        let cloned = func.clone();
        assert_eq!(func, cloned);
    }

    // LoopNode

    #[test]
    fn loop_node_metadata() {
        let mut func = Function::new("::test", "entry");
        func.loop_nodes.insert(
            "for_end_1".into(),
            LoopNode {
                entry_block: "for_header_1".into(),
                span: Span::new(0, 30),
            },
        );
        assert!(func.loop_nodes.contains_key("for_end_1"));
        assert_eq!(func.loop_nodes["for_end_1"].entry_block, "for_header_1");
    }
}
