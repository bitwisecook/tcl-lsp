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
//! expression parser.

use std::collections::{HashMap, HashSet, VecDeque};

use rustc_hash::{FxHashMap, FxHashSet};
use tcl_lexer::Span;

use crate::expr_ast::ExprNode;
use crate::ir::Statement;

// Block identity

/// Interned identifier for a CFG basic block.
///
/// Block names (`"entry_1"`, `"if_then_2"`, …) are interned per
/// [`Function`] into a dense `u32` index so the hot dataflow maps
/// (predecessors, dominators, phi incoming, …) key on a cheap copyable
/// id instead of hashing and cloning the name string. The `u32`
/// reflects block-*creation* order, so [`BlockId`]'s `Ord` is creation
/// order: a deterministic, source-top-to-bottom ordering that earlier
/// code approximated by sorting block-name strings.
///
/// Resolve an id back to its display name with [`Function::block_name`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct BlockId(pub u32);

// Terminators

/// How control leaves a basic block.
#[derive(Debug, Clone, PartialEq)]
pub enum Terminator {
    /// Unconditional jump to a single successor.
    Goto {
        /// Target block.
        target: BlockId,
        /// Source span of the jump (e.g. closing brace of a body).
        span: Option<Span>,
    },

    /// Conditional jump: evaluate `condition`, then jump to
    /// `true_target` or `false_target`.
    Branch {
        /// The condition expression.
        condition: ExprNode,
        /// Block when condition is true.
        true_target: BlockId,
        /// Block when condition is false.
        false_target: BlockId,
        /// Source span of the condition.
        span: Option<Span>,
        /// Absolute source offset of the condition *text*'s first byte,
        /// when it is a verbatim source slice — see
        /// [`crate::ir::IfClause::condition_base`].  Lets consumers map
        /// expression-AST leaf offsets to absolute operand spans.
        condition_base: Option<u32>,
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
    /// Return the ids of all successor blocks.
    #[must_use]
    pub fn successors(&self) -> Vec<BlockId> {
        match self {
            Self::Goto { target, .. } => vec![*target],
            Self::Branch {
                true_target,
                false_target,
                ..
            } => vec![*true_target, *false_target],
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

    /// Return the ids of all successor blocks.
    #[must_use]
    pub fn successors(&self) -> Vec<BlockId> {
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
    /// Id of the loop header/entry block.
    pub entry_block: BlockId,
    /// Source span of the original `for` statement.
    pub span: Span,
    /// The original `for` statement ([`Statement::For`]), retained so SCCP can
    /// statically summarise a bounded loop and fold a branch that reads a
    /// loop-carried variable *after* the loop (the static-loop → SCCP fold).
    pub for_stmt: Statement,
}

/// Source site and Tcl error context for a command body flattened into a CFG.
///
/// The semantic context comes from registry resolution during lowering.  It is
/// retained separately from source text so code generators never need to
/// recover command identity by parsing the original script.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InlineBodyErrorSite {
    /// Source span of the enclosing command whose body was inlined.
    pub span: Span,
    /// Target-neutral Tcl error context contributed by the inlined body.
    pub context: tcl_registry::InlineBodyErrorContext,
}

/// A complete control-flow graph for a single procedure or top-level script.
#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    /// Fully qualified procedure name (e.g. `"::top"`, `"::ns::proc"`).
    pub name: String,
    /// Id of the entry block.
    pub entry: BlockId,
    /// All blocks in the function, keyed by block id.
    pub blocks: HashMap<BlockId, Block>,
    /// Loop metadata: exit block → loop info.
    pub loop_nodes: HashMap<BlockId, LoopNode>,
    /// `try` body→handler exception edges for control flow the single-
    /// successor terminator can't express.  Consumed by SSA (as extra phi
    /// predecessors so a handler sees the body's versions) and SCCP (as
    /// extra reachability edges so handler bodies aren't false-unreachable
    /// → O107).  `(from_block, handler_block)` pairs; empty in codegen
    /// builds so the default bytecode is unchanged.
    pub exception_edges: Vec<(BlockId, BlockId)>,
    /// Registry-described error contexts for inlined command bodies flattened
    /// into this function's statement stream. Codegen turns each into a
    /// [`tcl_bytecode::ErrorRegion`] without re-parsing command text. Empty
    /// when no inlined body contributes an error frame.
    pub inline_body_error_sites: Vec<InlineBodyErrorSite>,
    /// Caller-frame injection this function is *subject to*: a callee whose
    /// [frame-effect summary](crate::cfg_builder::upvar_info::UpvarInfo)
    /// says it writes or reads names in **this** frame that no static
    /// analysis can enumerate (`helper $x` where `helper` runs
    /// `uplevel 1 $body`, or `argparse`'s definition-list injection).
    ///
    /// Recorded here, on the CFG, because it is the CFG builder — the one
    /// stage holding the module-wide proc summaries — that can see it, while
    /// the consumers ([`crate::dynamic_names::dynamic_name_barrier`] and
    /// everything downstream of it) run per function with only the CFG in
    /// hand.  Cleared for any CFG built without an upvar context.
    pub caller_frame_barrier: crate::dynamic_names::DynamicNameBarrier,
    /// Caller-frame names some callee of this function may **touch through
    /// an `upvar` alias or an `uplevel` write** (`get` running `upvar 1
    /// callervar m` makes `callervar` here observable in both directions).
    /// The call-site widening records such names as *defs* only; this set
    /// carries the "may also be read" half, so the dead-store / unused
    /// passes (O109 / O126) must not delete a store to any name in it —
    /// recording a read on the call statement instead would fabricate
    /// read-before-set uses (a false W210) for the pure out-param shape.
    /// Populated by the CFG builder's `record_alias_observed`; empty for a
    /// CFG built without an upvar context.
    pub alias_observed_vars: std::collections::BTreeSet<String>,
    /// Block-name interner: names indexed by [`BlockId`]`.0`, in creation order.
    block_names: Vec<String>,
    /// Reverse interner index: block name → its [`BlockId`].
    name_to_id: FxHashMap<String, BlockId>,
}

impl Function {
    /// Create a new function with a single empty entry block.
    #[must_use]
    pub fn new(name: impl Into<String>, entry: impl Into<String>) -> Self {
        let mut f = Self {
            name: name.into(),
            entry: BlockId(0),
            blocks: HashMap::new(),
            loop_nodes: HashMap::new(),
            exception_edges: Vec::new(),
            inline_body_error_sites: Vec::new(),
            caller_frame_barrier: crate::dynamic_names::DynamicNameBarrier::default(),
            alias_observed_vars: std::collections::BTreeSet::new(),
            block_names: Vec::new(),
            name_to_id: FxHashMap::default(),
        };
        let entry = entry.into();
        let id = f.intern_block(entry.clone());
        f.entry = id;
        f.blocks.insert(id, Block::new(entry));
        f
    }

    /// Intern a block name, returning its [`BlockId`].
    ///
    /// Assigns the next dense id (block-creation order) the first time a
    /// name is seen and returns the existing id on re-interning, so an id
    /// is stable for the life of the function.
    pub fn intern_block(&mut self, name: impl Into<String>) -> BlockId {
        let name = name.into();
        if let Some(&id) = self.name_to_id.get(&name) {
            return id;
        }
        let id =
            BlockId(u32::try_from(self.block_names.len()).expect("CFG block count fits in u32"));
        self.block_names.push(name.clone());
        self.name_to_id.insert(name, id);
        id
    }

    /// The display name of block `id`.
    ///
    /// # Panics
    /// Panics if `id` was not produced by this function's interner.
    #[must_use]
    pub fn block_name(&self, id: BlockId) -> &str {
        &self.block_names[id.0 as usize]
    }

    /// The [`BlockId`] a name was interned to, if any.
    #[must_use]
    pub fn block_id(&self, name: &str) -> Option<BlockId> {
        self.name_to_id.get(name).copied()
    }

    /// The interned block names, indexed by [`BlockId`]`.0` (creation order).
    #[must_use]
    pub fn block_names(&self) -> &[String] {
        &self.block_names
    }

    /// Borrow a block by its display name, if present.
    #[must_use]
    pub fn block_by_name(&self, name: &str) -> Option<&Block> {
        self.block_id(name).and_then(|id| self.blocks.get(&id))
    }

    /// Mutably borrow a block by its display name, if present.
    pub fn block_by_name_mut(&mut self, name: &str) -> Option<&mut Block> {
        self.block_id(name).and_then(|id| self.blocks.get_mut(&id))
    }

    /// All successor ids of block `id`: the terminator's successors plus
    /// any `try` exception-edge handler targets sourced at `id`.
    #[must_use]
    pub fn block_successors(&self, id: BlockId) -> Vec<BlockId> {
        let mut out: Vec<BlockId> = self
            .blocks
            .get(&id)
            .map(Block::successors)
            .unwrap_or_default();
        for (from, to) in &self.exception_edges {
            if *from == id && !out.contains(to) {
                out.push(*to);
            }
        }
        out
    }

    /// Whether this function can be **ahead-of-time compiled in full** — its CFG
    /// contains no [`Statement::Barrier`], the marker for a construct that must
    /// defer to the interpreter (`eval $dynamic`, `uplevel $level`, `upvar`, …).
    ///
    /// This is the per-procedure AOT gate for the WASM backend (option A,
    /// everything in-instance WASM): a clean function lowers entirely to native
    /// WASM and can be exported as a directly-callable function, whereas one
    /// containing a barrier needs an interpreter trampoline for the dynamic part.
    /// Statically-bodied control flow (`if`/`while`/`for`/`foreach`) and a static
    /// `uplevel`/`eval` body ([`Statement::Block`]/[`Statement::UpFrame`]) are
    /// *already flattened* into the CFG by the builder, so they never appear here
    /// and do not block AOT — only a residual barrier does. (A finer gate that
    /// also accounts for computed command dispatch is a later refinement.)
    #[must_use]
    pub fn is_aot_clean(&self) -> bool {
        !self
            .blocks
            .values()
            .flat_map(|b| b.statements.iter())
            .any(|s| matches!(s, Statement::Barrier { .. }))
    }

    /// Compute the predecessor map: block → set of predecessor blocks.
    ///
    /// O(V+E), and it allocates — a caller that needs the map more than once
    /// should build it once and pass it down rather than re-deriving it
    /// (issue #1251).
    #[must_use]
    pub fn predecessors(&self) -> HashMap<BlockId, HashSet<BlockId>> {
        self.predecessor_map()
    }

    /// [`Self::predecessors`] keyed with `rustc-hash`, for the CFG-derived
    /// indices that are built per analysis run: a [`BlockId`] is a dense `u32`
    /// and needs no randomised hashing.
    #[must_use]
    pub fn predecessors_fx(&self) -> FxHashMap<BlockId, FxHashSet<BlockId>> {
        self.predecessor_map()
    }

    /// The shared predecessor-map build, generic over the hasher so
    /// [`Self::predecessors`] and [`Self::predecessors_fx`] stay one
    /// implementation.
    fn predecessor_map<S: std::hash::BuildHasher + Default>(
        &self,
    ) -> HashMap<BlockId, HashSet<BlockId, S>, S> {
        let mut preds: HashMap<BlockId, HashSet<BlockId, S>, S> = HashMap::default();
        for id in self.blocks.keys() {
            preds.entry(*id).or_default();
        }
        for id in self.blocks.keys() {
            for succ in self.block_successors(*id) {
                preds.entry(succ).or_default().insert(*id);
            }
        }
        preds
    }

    /// Return the set of blocks reachable from the entry block.
    #[must_use]
    pub fn reachable_blocks(&self) -> HashSet<BlockId> {
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();
        queue.push_back(self.entry);
        while let Some(id) = queue.pop_front() {
            if !visited.insert(id) {
                continue;
            }
            if self.blocks.contains_key(&id) {
                for succ in self.block_successors(id) {
                    if !visited.contains(&succ) {
                        queue.push_back(succ);
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
    ///
    /// Implemented with an explicit work stack rather than recursion so
    /// it cannot overflow the thread stack on a degenerate CFG — a
    /// single huge proc (e.g. a machine-generated multi-thousand-branch
    /// dispatch table) lowers to a block chain tens of thousands deep,
    /// which a recursive DFS overflows at the common 2 MB worker-thread
    /// stack size. This is the one shared RPO used by every CFG/SSA
    /// pass (dominators, SCCP, type / taint / rendered-property
    /// propagation, GVN, `cfg_order`), so keeping it iterative keeps
    /// all of them bounded.
    #[must_use]
    pub fn reverse_postorder(&self) -> Vec<BlockId> {
        // Each frame caches the block's successor list — computed once
        // when the frame is pushed — alongside the index of the next
        // successor to visit, so advancing through them doesn't
        // recompute/reallocate the list on every loop iteration (which
        // matters on the large CFGs this iterative form exists to make
        // safe). A frame moves to `postorder` once all its successors
        // have been pushed — the recursive post-order DFS order,
        // reversed.
        struct Frame {
            id: BlockId,
            succs: Vec<BlockId>,
            idx: usize,
        }
        let succs_of = |id: BlockId| -> Vec<BlockId> { self.block_successors(id) };

        let mut visited: HashSet<BlockId> = HashSet::new();
        let mut postorder: Vec<BlockId> = Vec::new();
        let mut stack: Vec<Frame> = Vec::new();
        if self.blocks.contains_key(&self.entry) {
            visited.insert(self.entry);
            stack.push(Frame {
                id: self.entry,
                succs: succs_of(self.entry),
                idx: 0,
            });
        }
        while let Some(frame) = stack.last_mut() {
            if frame.idx < frame.succs.len() {
                let next = frame.succs[frame.idx];
                frame.idx += 1;
                if visited.insert(next) {
                    let succs = succs_of(next);
                    stack.push(Frame {
                        id: next,
                        succs,
                        idx: 0,
                    });
                }
            } else {
                let id = frame.id;
                stack.pop();
                postorder.push(id);
            }
        }
        postorder.reverse();
        postorder
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

impl CfgModule {
    /// `(aot_clean, total)` — how many of this module's functions (the top-level
    /// script plus every procedure) are fully AOT-compilable per
    /// [`Function::is_aot_clean`]. The coverage signal for "as much AOT as
    /// possible": the residual `total - aot_clean` functions are the ones that
    /// still need an interpreter trampoline for a dynamic barrier.
    #[must_use]
    pub fn aot_clean_count(&self) -> (usize, usize) {
        let total = 1 + self.procedures.len();
        let clean = usize::from(self.top_level.is_aot_clean())
            + self
                .procedures
                .values()
                .filter(|f| f.is_aot_clean())
                .count();
        (clean, total)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Intern `name` into `func` and insert a fresh block for it, returning
    /// the [`BlockId`]. The shared test idiom for building a CFG by hand.
    fn block(func: &mut Function, name: &str) -> BlockId {
        let id = func.intern_block(name);
        func.blocks.insert(id, Block::new(name));
        id
    }

    fn make_goto(target: BlockId) -> Terminator {
        Terminator::Goto { target, span: None }
    }

    fn make_branch(cond_text: &str, t: BlockId, f: BlockId) -> Terminator {
        Terminator::Branch {
            condition: ExprNode::Raw {
                text: cond_text.into(),
            },
            true_target: t,
            false_target: f,
            span: None,
            condition_base: None,
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
        let t = make_goto(BlockId(2));
        assert_eq!(t.successors(), vec![BlockId(2)]);
    }

    #[test]
    fn branch_successors() {
        let t = make_branch("$x", BlockId(1), BlockId(2));
        let mut succs = t.successors();
        succs.sort_unstable();
        assert_eq!(succs, vec![BlockId(1), BlockId(2)]);
    }

    #[test]
    fn return_successors() {
        let t = make_return(Some("1"));
        assert!(t.successors().is_empty());
    }

    #[test]
    fn terminator_span_none() {
        let t = make_goto(BlockId(0));
        assert!(t.span().is_none());
    }

    #[test]
    fn terminator_span_some() {
        let t = Terminator::Goto {
            target: BlockId(0),
            span: Some(Span::new(0, 5)),
        };
        assert_eq!(t.span(), Some(Span::new(0, 5)));
    }

    // Interner tests

    #[test]
    fn intern_assigns_creation_order_ids() {
        let mut func = Function::new("::test", "entry");
        assert_eq!(func.entry, BlockId(0));
        assert_eq!(func.block_name(BlockId(0)), "entry");
        let a = func.intern_block("a");
        let b = func.intern_block("b");
        assert_eq!(a, BlockId(1));
        assert_eq!(b, BlockId(2));
        // Re-interning a known name returns the existing id.
        assert_eq!(func.intern_block("a"), a);
        assert_eq!(func.block_id("b"), Some(b));
        assert_eq!(func.block_id("missing"), None);
    }

    // AOT-clean classifier tests

    /// A dynamic barrier (`eval $x`) — the marker that blocks full AOT.
    fn barrier_stmt() -> Statement {
        Statement::Barrier {
            span: Span::new(0, 0),
            reason: "eval with dynamic body".into(),
            command: "eval".into(),
            canonical_command: None,
            args: vec!["$x".into()],
            tokens: None,
        }
    }

    #[test]
    fn aot_clean_when_no_barrier() {
        // A barrier-free function (here: just an empty entry block) is AOT-clean.
        let f = Function::new("::p", "entry");
        assert!(f.is_aot_clean());
    }

    #[test]
    fn not_aot_clean_with_barrier() {
        let mut f = Function::new("::p", "entry");
        let entry = f.entry;
        f.blocks
            .get_mut(&entry)
            .unwrap()
            .statements
            .push(barrier_stmt());
        assert!(!f.is_aot_clean());
    }

    #[test]
    fn module_aot_coverage_counts_clean_functions() {
        // top-level clean, `::p` has a barrier, `::q` clean → 2 of 3.
        let mut p = Function::new("::p", "entry");
        let p_entry = p.entry;
        p.blocks
            .get_mut(&p_entry)
            .unwrap()
            .statements
            .push(barrier_stmt());
        let mut procedures = HashMap::new();
        procedures.insert("::p".to_string(), p);
        procedures.insert("::q".to_string(), Function::new("::q", "entry"));
        let m = CfgModule {
            top_level: Function::new("::top", "entry"),
            procedures,
        };
        assert_eq!(m.aot_clean_count(), (2, 3));
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
        block.terminator = Some(make_goto(BlockId(2)));
        assert_eq!(block.successors(), vec![BlockId(2)]);
    }

    #[test]
    fn block_with_statements() {
        let mut block = Block::new("b1");
        block.statements.push(Statement::AssignConst {
            span: Span::new(0, 10),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
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
        assert_eq!(func.block_name(func.entry), "entry_1");
        assert!(func.blocks.contains_key(&func.entry));
    }

    #[test]
    fn predecessors_simple() {
        // entry → b1 → b2
        let mut func = Function::new("::test", "entry");
        let entry = func.entry;
        let b1 = block(&mut func, "b1");
        let b2 = block(&mut func, "b2");
        func.blocks.get_mut(&entry).unwrap().terminator = Some(make_goto(b1));
        func.blocks.get_mut(&b1).unwrap().terminator = Some(make_goto(b2));
        func.blocks.get_mut(&b2).unwrap().terminator = Some(make_return(None));

        let preds = func.predecessors();
        assert!(preds[&entry].is_empty());
        assert_eq!(preds[&b1], HashSet::from([entry]));
        assert_eq!(preds[&b2], HashSet::from([b1]));
    }

    #[test]
    fn predecessors_branch() {
        // entry → (branch) → then / else → end
        let mut func = Function::new("::test", "entry");
        let entry = func.entry;
        let then = block(&mut func, "then");
        let els = block(&mut func, "else");
        let end = block(&mut func, "end");
        func.blocks.get_mut(&entry).unwrap().terminator = Some(make_branch("$x", then, els));
        func.blocks.get_mut(&then).unwrap().terminator = Some(make_goto(end));
        func.blocks.get_mut(&els).unwrap().terminator = Some(make_goto(end));
        func.blocks.get_mut(&end).unwrap().terminator = Some(make_return(None));

        let preds = func.predecessors();
        assert!(preds[&entry].is_empty());
        assert_eq!(preds[&then], HashSet::from([entry]));
        assert_eq!(preds[&els], HashSet::from([entry]));
        assert_eq!(preds[&end], HashSet::from([then, els]));
    }

    #[test]
    fn reachable_blocks_simple() {
        let mut func = Function::new("::test", "entry");
        let entry = func.entry;
        let b1 = block(&mut func, "b1");
        let unreachable = block(&mut func, "unreachable");
        func.blocks.get_mut(&entry).unwrap().terminator = Some(make_goto(b1));
        func.blocks.get_mut(&b1).unwrap().terminator = Some(make_return(None));

        let reachable = func.reachable_blocks();
        assert!(reachable.contains(&entry));
        assert!(reachable.contains(&b1));
        assert!(!reachable.contains(&unreachable));
    }

    #[test]
    fn reachable_blocks_with_loop() {
        // entry → header → (branch) → body → header / end
        let mut func = Function::new("::test", "entry");
        let entry = func.entry;
        let header = block(&mut func, "header");
        let body = block(&mut func, "body");
        let end = block(&mut func, "end");
        func.blocks.get_mut(&entry).unwrap().terminator = Some(make_goto(header));
        func.blocks.get_mut(&header).unwrap().terminator = Some(make_branch("$i < 10", body, end));
        func.blocks.get_mut(&body).unwrap().terminator = Some(make_goto(header));
        func.blocks.get_mut(&end).unwrap().terminator = Some(make_return(None));

        let reachable = func.reachable_blocks();
        assert_eq!(reachable.len(), 4);
        assert!(reachable.contains(&entry));
        assert!(reachable.contains(&header));
        assert!(reachable.contains(&body));
        assert!(reachable.contains(&end));
    }

    #[test]
    fn reverse_postorder_diamond() {
        // entry → (branch) → then / else → end
        let mut func = Function::new("::test", "entry");
        let entry = func.entry;
        let then = block(&mut func, "then");
        let els = block(&mut func, "else");
        let end = block(&mut func, "end");
        func.blocks.get_mut(&entry).unwrap().terminator = Some(make_branch("$x", then, els));
        func.blocks.get_mut(&then).unwrap().terminator = Some(make_goto(end));
        func.blocks.get_mut(&els).unwrap().terminator = Some(make_goto(end));
        func.blocks.get_mut(&end).unwrap().terminator = Some(make_return(None));

        let rpo = func.reverse_postorder();
        // entry comes first, end comes last
        assert_eq!(rpo[0], entry);
        assert_eq!(*rpo.last().unwrap(), end);
        // then and else are between entry and end
        let then_pos = rpo.iter().position(|n| *n == then).unwrap();
        let else_pos = rpo.iter().position(|n| *n == els).unwrap();
        let end_pos = rpo.iter().position(|n| *n == end).unwrap();
        assert!(then_pos < end_pos);
        assert!(else_pos < end_pos);
    }

    #[test]
    fn reverse_postorder_loop() {
        // entry → header → (branch) → body → header / end
        let mut func = Function::new("::test", "entry");
        let entry = func.entry;
        let header = block(&mut func, "header");
        let body = block(&mut func, "body");
        let end = block(&mut func, "end");
        func.blocks.get_mut(&entry).unwrap().terminator = Some(make_goto(header));
        func.blocks.get_mut(&header).unwrap().terminator = Some(make_branch("$i < 10", body, end));
        func.blocks.get_mut(&body).unwrap().terminator = Some(make_goto(header));
        func.blocks.get_mut(&end).unwrap().terminator = Some(make_return(None));

        let rpo = func.reverse_postorder();
        assert_eq!(rpo[0], entry);
        // header before body (non-back-edge predecessor)
        let header_pos = rpo.iter().position(|n| *n == header).unwrap();
        let body_pos = rpo.iter().position(|n| *n == body).unwrap();
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
        let entry = func.entry;
        func.blocks.get_mut(&entry).unwrap().terminator = Some(make_return(Some("0")));
        let cloned = func.clone();
        assert_eq!(func, cloned);
    }

    // LoopNode

    #[test]
    fn loop_node_metadata() {
        let mut func = Function::new("::test", "entry");
        let for_header = func.intern_block("for_header_1");
        let for_end = func.intern_block("for_end_1");
        func.loop_nodes.insert(
            for_end,
            LoopNode {
                entry_block: for_header,
                span: Span::new(0, 30),
                for_stmt: Statement::For {
                    span: Span::new(0, 30),
                    init: crate::ir::Script::new(),
                    init_span: Span::new(0, 0),
                    condition: crate::expr_ast::ExprNode::Literal {
                        text: "1".into(),
                        start: 0,
                        end: 0,
                    },
                    condition_span: Span::new(0, 0),
                    next: crate::ir::Script::new(),
                    next_span: Span::new(0, 0),
                    body: crate::ir::Script::new(),
                    body_span: Span::new(0, 0),
                    raw_args: Vec::new(),
                    raw_tokens: None,
                    condition_base: None,
                },
            },
        );
        assert!(func.loop_nodes.contains_key(&for_end));
        assert_eq!(func.loop_nodes[&for_end].entry_block, for_header);
    }
}
