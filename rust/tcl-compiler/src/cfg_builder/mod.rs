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

//! CFG construction from structured IR.
//!
//! Flattens structured IR (`If`, `For`, `While`, `Switch`, `Catch`,
//! `Try`) into a graph of basic blocks connected by terminators.
//! The per-construct lowering methods live in [`cfg_lower`].
//!
//! Public API:
//! - [`build_cfg`] — build CFGs for a whole module (top-level + procs).
//! - [`build_cfg_function`] — build a CFG for a single script body.

#[cfg(test)]
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::sync::Arc;

use rustc_hash::{FxHashMap, FxHashSet};
use tcl_lexer::Span;
#[cfg(test)]
use tcl_lexer::TokenType;
#[cfg(test)]
use tcl_registry::EffectiveRegistrySemantics;
#[cfg(test)]
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::{CommandRegistry, Traits};

use crate::cfg::{Block, BlockId, CfgModule, Function, LoopNode, Terminator};
use crate::command_binding::ModuleCommandBindings;
#[cfg(test)]
use crate::expr_ast::ExprNode;
use crate::ir::{CommandBindingSite, CommandTokens, Module, Script, Statement};

use self::global_write_info::GlobalWriteInfo;
pub use self::summary_candidates::OriginalProcedureSummaryOwners;
use self::upvar_info::UpvarInfo;

mod callee_frame_effects;
mod command_classes;
mod embedded_substitutions;
mod flow_facts;
mod function_entry_points;
mod guaranteed_iteration;
mod opaque_constructs;
mod prepared_context;
mod release_compiled_control;
mod script_lowering;
mod split_points;
mod unseen_code;
mod upvar_invalidation;

pub(crate) use flow_facts::{
    Completion, switch_must_defines, switch_must_defines_with_source_input,
};
#[cfg(test)]
pub(crate) use flow_facts::{escaping_loop_jumps, flow_facts_stmt};
use flow_facts::{flow_facts_stmt_with_classes, switch_escaping_jumps_with_classes};
#[cfg(test)]
use function_entry_points::build_cfg_method_function_with_upvars;
#[cfg(test)]
use function_entry_points::command_namespace;
pub(crate) use function_entry_points::{
    build_analysis_body, build_cfg_method_function_with_prepared_context,
};
pub use function_entry_points::{
    build_cfg_function, build_cfg_function_in_bpf_selected_scope,
    build_cfg_function_in_runtime_selected_scope, build_cfg_function_with_config,
    build_cfg_function_with_prepared_context, build_cfg_function_with_upvars,
    build_cfg_function_with_upvars_and_config,
};
pub(crate) use guaranteed_iteration::{foreach_never_runs, foreach_runs_at_least_once};
#[cfg(test)]
use opaque_constructs::{dedup_preserve_order, drop_word};
pub(crate) use prepared_context::entered_procedures;
pub use prepared_context::{
    detect_upvar_procs, detect_upvar_procs_with_registry, prepare_cfg_context,
    prepare_cfg_context_bundle, prepare_cfg_context_with_registry,
};
pub(super) use split_points::{NextFailure, SplitPoint, SplitRegion};

/// Registry traits by which a command declares that it runs code, or raises a
/// barrier, where it is invoked. A head that reaches such a command through a
/// binding other than its own spelling — an `interp alias`, a `rename`, the
/// registry's unresolved-command handler — runs code the graph never lowers,
/// so it is a call to code the module cannot see
/// ([`CfgBuilder::head_is_unseen`]). The command binding lattice resolves
/// aliases and unresolved-command handlers to terminal registry targets before
/// this set is consulted, so it does not name `unknown` directly.
const REGISTRY_BARRIER_TRAITS: Traits = CODE_RUNNING_TRAITS;

const CODE_RUNNING_TRAITS: Traits = Traits::EVALUATES_CODE
    .union(Traits::CREATES_BARRIER)
    .union(Traits::CREATES_DYNAMIC_BARRIER);

#[doc(hidden)]
pub mod cfg_lower;
#[doc(hidden)]
pub use cfg_lower::matches_foreach_header;
mod execution_regions;
pub(crate) mod frame_effects;
pub mod global_write_info;
#[doc(hidden)]
pub mod summary_candidates;
pub mod upvar_info;

/// Mutable block used during construction, frozen into [`Block`] at the end.
struct MutableBlock {
    name: String,
    statements: Vec<Statement>,
    terminator: Option<Terminator>,
}

#[derive(Default)]
struct ResolvedUpvarEffects {
    defs: Vec<String>,
    opaque_arguments: bool,
    has_unresolvable_target: bool,
    frame_barrier: crate::dynamic_names::DynamicNameBarrier,
}

/// The variable effects a loop or branch condition's `[…]` substitutions
/// contribute to the frame the condition is evaluated in.
#[derive(Default)]
struct ConditionEffects {
    /// Variables the condition's substitutions write.
    defs: Vec<String>,
    /// What the condition reads of [`Self::defs`] ahead of their writes, as
    /// [`EmbeddedSubstExtras::reads`] states it.
    reads: Vec<String>,
    /// An embedded callee runs an unreadable script at the global frame.
    opaque_global: bool,
    /// An embedded command is one the module cannot see.
    unseen: bool,
    /// A source-order invocation reaches a registry evaluation barrier.
    registry_barrier: bool,
}

/// The caller-frame effects a statement's `[…]` substitutions contribute.
#[derive(Default)]
struct EmbeddedSubstExtras {
    /// Variables an embedded substitution writes.
    defs: Vec<String>,
    /// What the statement reads of [`Self::defs`] ahead of their writes: the
    /// names an embedded command reads before it writes (`[incr n]`,
    /// `[append s x]`), and those the statement's words read beside a write
    /// ([`CfgBuilder::read_beside_writes`]).
    reads: Vec<String>,
    /// An embedded callee runs an unreadable script at the global frame.
    opaque_global: bool,
    /// An embedded command is one the module cannot see.
    unseen: bool,
}

/// Whether a call-shaped IR statement has one statically literal command
/// head. Computed dispatch already has its own dynamic-command handling; a
/// runtime-selected namespace only adds uncertainty for a literal relative
/// head that could be shadowed there.
fn statement_has_literal_head(stmt: &Statement) -> bool {
    match stmt {
        Statement::Call { tokens, .. } | Statement::Barrier { tokens, .. } => {
            tokens.as_ref().is_none_or(|tokens| {
                tokens.synthetic.is_none()
                    && tokens.words().first().is_none_or(|head| {
                        crate::registry_invocation::invocation_word(head)
                            .literal()
                            .is_some()
                    })
            })
        }
        _ => false,
    }
}

/// Builder that accumulates blocks and loop metadata for one function.
///
/// The independent flags describe separate lowering policies; grouping them
/// would hide that each can be selected by a different public CFG entry point.
#[allow(clippy::struct_excessive_bools)]
pub(crate) struct CfgBuilder<'a> {
    /// Only the sealed closed-phase entry installs this no-Tcl-effect proof.
    bpf_closed_effects: Option<Arc<crate::ir::SourceBpfClosedEffects>>,
    counter: u32,
    blocks: HashMap<String, MutableBlock>,
    statement_sources: HashMap<
        (BlockId, usize),
        Option<std::sync::Arc<crate::command_binding::ExecutedScriptSource>>,
    >,
    current_source: Option<std::sync::Arc<crate::command_binding::ExecutedScriptSource>>,
    source_may_calls: HashMap<(usize, u32), Vec<global_write_info::OriginalSourceCallMayWrites>>,
    implicit_math_invocations: Vec<crate::command_binding::SourceMathInvocation>,
    expression_preparations: Vec<crate::command_binding::SourceExpressionPreparation>,
    terminator_sources:
        HashMap<BlockId, Option<std::sync::Arc<crate::command_binding::ExecutedScriptSource>>>,
    /// Block name → [`BlockId`], assigned in block-creation order so the
    /// frozen [`Function`]'s interner reflects that order.
    block_ids: FxHashMap<String, BlockId>,
    loop_nodes: HashMap<String, LoopNode>,
    /// The bodies of the opaque `catch` calls, by span
    /// ([`Function::opaque_catch_bodies`]).
    opaque_catch_bodies: HashMap<Span, Script>,
    inline_loops: bool,
    /// Whether the function being built is a procedure body rather than the
    /// top-level script. Only a procedure has a local variable table, so only
    /// there can a `catch` be inlined (#2207): C compiles a top-level
    /// `catch {set a 1} m o` as a plain `invokeStk`, having nowhere to put the
    /// result variable's slot.
    is_proc_body: bool,
    /// Set when the module's own top level is a procedure body, so
    /// `build_function` treats `::top` as one. See
    /// [`Module::top_level_is_procedure_body`](crate::ir::Module::top_level_is_procedure_body).
    top_level_is_proc_body: bool,
    /// Map from issued body to caller-frame summary, used to pre-populate
    /// caller-side `defs` on calls to procs that use `upvar`.  Empty
    /// when the builder is constructed without an upvar context
    /// (e.g. for one-off CFGs that don't have a Module to scan).
    upvar_procs: HashMap<crate::ir::ProcedureId, UpvarInfo>,
    procedure_summary_owners: OriginalProcedureSummaryOwners,
    /// Map from issued body to parameter list, used by the upvar
    /// wiring to resolve param-based upvar sources (`upvar 1 $param
    /// local`) against the actual call-site argument.
    proc_params: HashMap<crate::ir::ProcedureId, Vec<String>>,
    /// Map from issued body to the outer-scope (global/namespace) variable
    /// names that proc's body writes (see [`global_write_info`]), used to
    /// widen caller-side `defs` at a call site exactly like `upvar_procs` —
    /// so SCCP treats a global/namespace name as overdefined across an
    /// opaque call that writes it, not as an ordinary untouched local.
    global_write_procs: HashMap<crate::ir::ProcedureId, GlobalWriteInfo>,
    /// Whether any procedure's [`Self::global_write_procs`] summary names
    /// an outer-scope write, so a module with none skips the per-statement
    /// scan [`Self::record_alias_observed`] makes for them.
    global_writers: bool,
    /// Closed module command state used to resolve direct and parsed embedded
    /// spellings to effective user-procedure targets, including alias chains.
    command_bindings: ModuleCommandBindings,
    /// Source-order snapshots for the root currently being lowered. Kept
    /// parallel to the historical module lattice so independent roots do not
    /// contaminate each other's executable entry state.
    source_binding_timeline: Option<crate::command_binding::SourceBindingTimeline>,
    /// Namespace in which direct command heads in this function resolve.
    invocation_namespace: crate::ir::ExecutionNamespace,
    /// Whether registry-declared `TclOO` self/next dispatch must conservatively
    /// invalidate this method's caller frame. Method reachability analysis
    /// owns this decision; user-procedure summaries stay identity-pure.
    widen_oo_dispatch: bool,
    /// Stack of `(break_target, continue_target)` block names for the
    /// enclosing loops, so `break` / `continue` in a body lower to a CFG
    /// edge.  Without this a `while 1 { … break }` exit block is
    /// unreachable and O107 false-fires on the code after the loop.
    loop_stack: Vec<(String, String)>,
    /// `try` body→handler exception edges (analysis builds only).
    exception_edges: Vec<(String, String)>,
    analysis_edges: Vec<(String, String)>,
    /// The exception edges that run from the block before a flattened body,
    /// with the block of the body's first command
    /// ([`crate::cfg::Function::region_entries`]).
    region_entries: Vec<(String, String, String)>,
    /// The flattened `catch` regions, each as the block before its body, the
    /// block that ends it and the `catch` written as a call
    /// ([`crate::cfg::Function::catch_ends`]).
    catch_ends: Vec<(String, String, Statement)>,
    /// The protected region being lowered in an analysis build — a flattened
    /// `catch` body or a `try` body — whose scripts give each statement a
    /// block of its own, so every point at which the body can fail ends a
    /// block an exception edge leaves from; the region's inner blocks are
    /// collected in [`Self::split_blocks`].
    split_region: Option<SplitRegion>,
    /// The points a throw may leave the region from: the blocks its scripts
    /// end a statement in other than their last.
    split_points: Vec<SplitPoint>,
    /// For each block a split began, the block the statement before it ended
    /// in: a region's first statements are the run from its first block.
    split_parent: FxHashMap<String, String>,
    /// The subset of [`Self::exception_edges`] that resume a `break` /
    /// `continue` after a `finally` clause: `(the clause's last block, jump
    /// target)`. An enclosing `try … finally` reroutes them through its own
    /// clause, as it does the jumps its own body makes.
    finally_jump_edges: Vec<(String, String)>,
    /// Blocks a plain `return` with a value that cannot substitute ends:
    /// they complete with `TCL_RETURN`, which a `try` handler may select.
    plain_return_blocks: FxHashSet<String>,
    /// Blocks that intercept *every* completion reaching them by an exception
    /// edge: a `catch`'s end block, and the end block of a `try` whose
    /// `finally` routes every exit through it. A `try` handler is not one —
    /// it selects only some codes.
    total_interceptors: FxHashSet<String>,
    /// The body block of the `try` whose handler edges are being recorded:
    /// the one block of its body that nothing inside the construct runs
    /// before, so the only one whose completion can be known exactly.
    try_entry: Option<String>,
    /// Blocks one of their `try`'s unconditional handlers catches whole: an
    /// enclosing construct must not route them past that handler.
    handler_caught: FxHashSet<String>,
    /// Last blocks of `finally` clauses that both fall through and resume an
    /// unwinding exit (a `return` or an error) once the clause is done. The
    /// fall-through `Goto` stands only for normal completion, so an enclosing
    /// construct treats such a block as an exit of its own.
    unwinding_tails: FxHashSet<String>,
    /// When `true`, record [`Self::exception_edges`] in `lower_try`.  Off for
    /// codegen builds so the default bytecode is unchanged.
    faithful_exceptions: bool,
    /// Every command must retain ordinary runtime dispatch semantics. This is
    /// the CFG half of `Module::plain_command_dispatch`: command-name traits
    /// must not promote a generic call to a builtin-specific return/loop edge,
    /// because the live command may have been replaced.
    plain_command_dispatch: bool,
    /// The exact registry that lowered the IR this builder consumes.  CFG
    /// variable-write recovery must query this registry rather than rebuilding
    /// a release-only approximation from [`Module::dialect`]: callers may add
    /// pack commands or override command descriptors without changing the
    /// dialect name.
    registry: &'a CommandRegistry,
    /// Release/dialect-specific command control-flow facts, derived from the
    /// same registry profile that lowered the module.  Keeping this on the
    /// builder prevents a Tcl 9 command (for example `throw`) from shaping a
    /// Tcl 8.4 CFG where that spelling can be a user procedure.
    command_classes: CfgCommandClasses,
    /// When `Some`, every block that raises an explicit `error` / `throw`
    /// is recorded here. `lower_try` installs a fresh list around its body
    /// so the on-error edge is sourced from each throw point (at its
    /// throw-time SSA versions) rather than the pre-`try` block — a
    /// body-set var is defined at a later throw.
    throw_blocks: Option<Vec<String>>,
    /// The block where the most recent `lower_script` call's straight-line
    /// control finally terminated, when that script had no normal
    /// fall-through (e.g. a trailing `return` / `error`). `None` when the
    /// script fell through. `lower_try` reads this to source an on-error
    /// edge from a body that terminated without an explicit `error`/`throw`
    /// (a bare `return`).
    last_terminal_block: Option<String>,
    /// Registry-described error contexts for inlined command bodies flattened
    /// into the function currently being built. Drained into
    /// [`crate::cfg::Function`] for source-text-independent error-frame
    /// emission.
    inline_body_error_sites: Vec<crate::cfg::InlineBodyErrorSite>,
    /// Structured command dependencies collected from every recursively
    /// flattened IR script in this function.
    command_binding_sites: Vec<CommandBindingSite>,
    /// User-procedure dependencies retained by executable IR inlining.
    procedure_binding_requirements: Vec<tcl_runtime_api::ProcedureBindingIdentity>,
    /// Block name to exact owning structured Tcl command for synthetic runtime
    /// revalidation boundaries.
    command_boundary_sites: HashMap<String, CommandBindingSite>,
    condition_binding_sites: HashMap<String, CommandBindingSite>,
    condition_owners: HashMap<Span, crate::cfg::ConditionOwner>,
    /// Entry block to continuation block for an explicitly delimited inline
    /// structured-command region.
    command_boundary_continuations: HashMap<String, String>,
    /// Head dispatch block to join block for a branch-lowered `if`/`switch`.
    branch_command_joins: HashMap<String, String>,
    /// The script bodies structured commands evaluate.
    body_scripts: Vec<crate::cfg::BodyScript>,
    /// Caller-frame injection the function currently being built is subject
    /// to: the union of every called procedure's
    /// [`UpvarInfo::caller_frame_barrier`].  Drained into
    /// [`crate::cfg::Function::caller_frame_barrier`], which
    /// [`crate::dynamic_names::dynamic_name_barrier`] folds into the
    /// function's blindness lattice.
    caller_frame_barrier: crate::dynamic_names::DynamicNameBarrier,
    /// [`Self::caller_frame_barrier`] by the block the raising statement goes
    /// into ([`Self::barrier_block`]); the remainder no block owns.
    caller_frame_barriers: HashMap<String, crate::dynamic_names::DynamicNameBarrier>,
    caller_frame_barrier_unplaced: crate::dynamic_names::DynamicNameBarrier,
    /// The block the statements now being invalidated are pushed into.
    barrier_block: Option<String>,
    /// Caller-frame names a callee may touch through an `upvar` alias or
    /// `uplevel` write — see [`CfgBuilder::record_alias_observed`].  Drained
    /// into [`crate::cfg::Function::alias_observed_vars`].
    alias_observed_vars: std::collections::BTreeSet<String>,
    /// Current `lower_script` recursion depth, bounded by [`MAX_LOWER_DEPTH`]
    /// so deeply-nested bodies cannot overflow the stack.
    depth: u32,
    /// The document's lexer configuration.  The embedded-substitution scans
    /// below (`command_heads_in_text`, `upvar_defs_from_text`,
    /// `global_write_defs_from_text`, `builtin_write_defs_from_text`) re-lex
    /// an IR statement's own argument text to find its `[…]` tokens and split
    /// each one into words, so they must read it under the grammar the
    /// document was lexed with — under `f5-irules` `}{` separates two words,
    /// under Jim `$(…)` is one, under 8.4 `{*}` is literal — or the head they
    /// credit is not the head that runs. Defaults to
    /// `LexerConfig::default()`; the pipeline threads the document's config
    /// through [`build_cfg_with_config`] and friends.
    config: tcl_lexer::LexerConfig,
}

/// Maximum nesting depth for the recursive `lower_script` descent.
/// `lower_script` ↔ `lower_if`/`lower_for`/`lower_while`/`lower_foreach`/
/// `lower_switch`/`lower_try` are mutually recursive with one Rust frame
/// per nesting level; deeply-nested (generated / minified) bodies would
/// otherwise overflow the stack — an uncatchable SIGABRT that takes down
/// the LSP worker or the `tcl` CLI. At the cap we stop descending and
/// treat the over-deep script as a non-fall-through tail, yielding a
/// truncated-but-valid CFG instead of a crash. No real source nests
/// anywhere near this.
///
/// Shares [`crate::depth_guard::MAX_SOURCE_NEST_DEPTH`] with the lowering
/// and analyser body walks — one budget, derived from a measured per-level
/// stack cost, rather than three copies of a hand-picked 256.
///
/// Through the normal pipeline this cap is **unreachable**: `lowering`'s
/// copy of the same number already bounds the IR handed here, so a
/// `Module` built from source cannot nest deeper than the cap in the first
/// place. It matters for a host that builds a `Module` some other way, and
/// its 8,288 bytes a level is what the budget must still afford for that
/// caller — the same defence-in-depth role the optimiser and codegen walks
/// play over the same IR.
const MAX_LOWER_DEPTH: tcl_core_types::RecursionLimit = crate::depth_guard::MAX_SOURCE_NEST_DEPTH;

impl<'a> CfgBuilder<'a> {
    fn with_procedure_summary_owners(mut self, owners: OriginalProcedureSummaryOwners) -> Self {
        self.procedure_summary_owners = owners;
        self
    }

    fn original_tokens_for_statement<'s>(
        &'s self,
        statement: &'s Statement,
    ) -> Option<&'s CommandTokens> {
        crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
            self.command_binding_sites.iter(),
            statement,
        )
    }

    fn original_tokens_at(&self, span: Span) -> Option<&CommandTokens> {
        let mut candidates = self
            .command_binding_sites
            .iter()
            .filter(|site| site.span == span)
            .filter_map(|site| site.source_tokens.as_deref());
        let first = candidates.next()?;
        candidates
            .all(|candidate| candidate == first)
            .then_some(first)
    }

    fn with_source_binding_timeline(
        mut self,
        timeline: crate::command_binding::SourceBindingTimeline,
    ) -> Self {
        self.source_binding_timeline = Some(timeline);
        self
    }

    fn new(inline_loops: bool, registry: &'a CommandRegistry) -> Self {
        Self::new_with_upvars(
            inline_loops,
            HashMap::new(),
            HashMap::new(),
            HashMap::new(),
            ModuleCommandBindings::default(),
            registry,
        )
    }

    fn new_with_upvars(
        inline_loops: bool,
        upvar_procs: HashMap<crate::ir::ProcedureId, UpvarInfo>,
        proc_params: HashMap<crate::ir::ProcedureId, Vec<String>>,
        global_write_procs: HashMap<crate::ir::ProcedureId, GlobalWriteInfo>,
        command_bindings: ModuleCommandBindings,
        registry: &'a CommandRegistry,
    ) -> Self {
        let command_classes = CfgCommandClasses::from_registry(registry);
        Self::new_with_upvars_and_classes(
            inline_loops,
            upvar_procs,
            proc_params,
            global_write_procs,
            command_bindings,
            registry,
            command_classes,
        )
    }

    fn new_with_upvars_and_classes(
        inline_loops: bool,
        upvar_procs: HashMap<crate::ir::ProcedureId, UpvarInfo>,
        proc_params: HashMap<crate::ir::ProcedureId, Vec<String>>,
        global_write_procs: HashMap<crate::ir::ProcedureId, GlobalWriteInfo>,
        command_bindings: ModuleCommandBindings,
        registry: &'a CommandRegistry,
        command_classes: CfgCommandClasses,
    ) -> Self {
        let global_writers = global_write_procs
            .values()
            .any(|info| !info.names.is_empty());
        Self {
            bpf_closed_effects: None,
            counter: 0,
            blocks: HashMap::new(),
            statement_sources: HashMap::new(),
            current_source: None,
            source_may_calls: HashMap::new(),
            implicit_math_invocations: Vec::new(),
            expression_preparations: Vec::new(),
            terminator_sources: HashMap::new(),
            block_ids: FxHashMap::default(),
            loop_nodes: HashMap::new(),
            opaque_catch_bodies: HashMap::new(),
            inline_loops,
            is_proc_body: false,
            top_level_is_proc_body: false,
            upvar_procs,
            procedure_summary_owners: OriginalProcedureSummaryOwners::default(),
            proc_params,
            global_write_procs,
            global_writers,
            command_bindings,
            source_binding_timeline: None,
            invocation_namespace: crate::ir::ExecutionNamespace::exact("::"),
            widen_oo_dispatch: false,
            loop_stack: Vec::new(),
            exception_edges: Vec::new(),
            analysis_edges: Vec::new(),
            region_entries: Vec::new(),
            catch_ends: Vec::new(),
            split_region: None,
            split_points: Vec::new(),
            split_parent: FxHashMap::default(),
            finally_jump_edges: Vec::new(),
            plain_return_blocks: FxHashSet::default(),
            total_interceptors: FxHashSet::default(),
            try_entry: None,
            handler_caught: FxHashSet::default(),
            unwinding_tails: FxHashSet::default(),
            faithful_exceptions: false,
            plain_command_dispatch: false,
            registry,
            command_classes,
            throw_blocks: None,
            last_terminal_block: None,
            inline_body_error_sites: Vec::new(),
            command_binding_sites: Vec::new(),
            procedure_binding_requirements: Vec::new(),
            command_boundary_sites: HashMap::new(),
            condition_binding_sites: HashMap::new(),
            condition_owners: HashMap::new(),
            command_boundary_continuations: HashMap::new(),
            branch_command_joins: HashMap::new(),
            body_scripts: Vec::new(),
            caller_frame_barrier: crate::dynamic_names::DynamicNameBarrier::default(),
            caller_frame_barriers: HashMap::new(),
            caller_frame_barrier_unplaced: crate::dynamic_names::DynamicNameBarrier::default(),
            barrier_block: None,
            alias_observed_vars: std::collections::BTreeSet::new(),
            depth: 0,
            // dialect-drift-ok: the builder's own default; the pipeline
            // replaces it via `with_lexer_config` from the unit's config.
            config: tcl_lexer::LexerConfig::default(),
        }
    }

    /// Enable `try` exception-edge recording (analysis builds).
    fn with_faithful_exceptions(mut self) -> Self {
        self.faithful_exceptions = true;
        self
    }

    /// Read every embedded `[…]` substitution under the document's own
    /// grammar (see [`Self::config`]).
    fn with_lexer_config(mut self, config: tcl_lexer::LexerConfig) -> Self {
        if matches!(&self.command_classes.metadata, CfgMetadataContext::SuppliedSource(input) if input.lexer_config().normalized() != config.normalized())
        {
            self.command_classes.metadata = CfgMetadataContext::Unavailable;
        }

        self.config = config;
        self
    }

    /// Bind every command-sensitive CFG decision to one resolved command
    /// surface and dispatch mode. Single-function consumers cannot recover
    /// either fact from a detached [`Script`], so their public entry points
    /// require them and funnel through this seam.
    fn with_command_surface(mut self, plain_command_dispatch: bool) -> Self {
        self.plain_command_dispatch = plain_command_dispatch;
        self
    }

    fn with_invocation_namespace(mut self, namespace: crate::ir::ExecutionNamespace) -> Self {
        self.invocation_namespace = namespace;
        self
    }

    fn with_oo_dispatch_widening(mut self) -> Self {
        self.widen_oo_dispatch = true;
        self
    }

    /// Allocate a new empty block with a unique name, interning its
    /// [`BlockId`] in creation order.
    fn new_block(&mut self, prefix: &str) -> String {
        self.counter += 1;
        let name = format!("{prefix}_{}", self.counter);
        let id = BlockId(u32::try_from(self.block_ids.len()).expect("CFG block count fits in u32"));
        self.block_ids.insert(name.clone(), id);
        self.blocks.insert(
            name.clone(),
            MutableBlock {
                name: name.clone(),
                statements: Vec::new(),
                terminator: None,
            },
        );
        name
    }

    /// The [`BlockId`] interned for a created block name. Panics if the
    /// name was never created by [`Self::new_block`].
    fn bid(&self, name: &str) -> BlockId {
        *self
            .block_ids
            .get(name)
            .unwrap_or_else(|| panic!("block {name} has no id"))
    }

    /// Borrow a mutable block by name. Panics if missing.
    fn block_mut(&mut self, name: &str) -> &mut MutableBlock {
        self.blocks
            .get_mut(name)
            .unwrap_or_else(|| panic!("block {name} not found"))
    }

    /// Set a `Goto` terminator on a block only if it doesn't already
    /// have one.
    fn ensure_goto(&mut self, block_name: &str, target: &str, span: Option<Span>) {
        let target = self.bid(target);
        let block = self.block_mut(block_name);
        if block.terminator.is_none() {
            block.terminator = Some(Terminator::Goto { target, span });
        }
    }

    /// Build a [`Function`] by lowering a script starting at a fresh
    /// entry block, then freezing all mutable blocks.
    /// Mark this builder's `::top` as a procedure body.
    fn with_top_level_proc_body(mut self, is_proc_body: bool) -> Self {
        self.top_level_is_proc_body = is_proc_body;
        self
    }

    fn build_function(&mut self, name: &str, script: &Script) -> Function {
        self.is_proc_body = name != "::top" || self.top_level_is_proc_body;
        let entry = self.new_block("entry");
        let tail = self.lower_script(script, &entry);
        if let Some(tail) = tail {
            let exit = self.new_block("exit");
            self.ensure_goto(&tail, &exit, None);
        }

        let mut func = self.freeze_function_blocks(name, &entry, script);

        func.loop_nodes = std::mem::take(&mut self.loop_nodes)
            .into_iter()
            .map(|(k, ln)| (self.bid(&k), ln))
            .collect();
        func.opaque_catch_bodies = std::mem::take(&mut self.opaque_catch_bodies);
        self.finally_jump_edges.clear();
        self.plain_return_blocks.clear();
        self.split_parent.clear();
        self.total_interceptors.clear();
        self.handler_caught.clear();
        self.unwinding_tails.clear();
        func.exception_edges = std::mem::take(&mut self.exception_edges)
            .into_iter()
            .map(|(from, to)| (self.bid(&from), self.bid(&to)))
            .collect();
        func.analysis_edges = std::mem::take(&mut self.analysis_edges)
            .into_iter()
            .map(|(from, to)| (self.bid(&from), self.bid(&to)))
            .collect();
        func.region_entries = std::mem::take(&mut self.region_entries)
            .into_iter()
            .map(|(source, handler, first)| crate::cfg::RegionEntry {
                source: self.bid(&source),
                handler: self.bid(&handler),
                first: self.bid(&first),
            })
            .collect();
        func.catch_ends = std::mem::take(&mut self.catch_ends)
            .into_iter()
            .map(|(entry, end, call)| crate::cfg::CatchEnd {
                entry: self.bid(&entry),
                end: self.bid(&end),
                call,
            })
            .collect();
        func.inline_body_error_sites = std::mem::take(&mut self.inline_body_error_sites);
        func.command_binding_sites = std::mem::take(&mut self.command_binding_sites);
        func.procedure_binding_requirements =
            std::mem::take(&mut self.procedure_binding_requirements);
        func.condition_owners = std::mem::take(&mut self.condition_owners);
        func.command_boundary_sites = std::mem::take(&mut self.command_boundary_sites)
            .into_iter()
            .map(|(block, site)| (self.bid(&block), site))
            .collect();
        func.condition_binding_sites = std::mem::take(&mut self.condition_binding_sites)
            .into_iter()
            .map(|(block, site)| (self.bid(&block), site))
            .collect();
        func.command_boundary_continuations =
            std::mem::take(&mut self.command_boundary_continuations)
                .into_iter()
                .map(|(entry, continuation)| (self.bid(&entry), self.bid(&continuation)))
                .collect();
        func.branch_command_joins = std::mem::take(&mut self.branch_command_joins)
            .into_iter()
            .map(|(head, join)| (self.bid(&head), self.bid(&join)))
            .collect();
        func.body_scripts = std::mem::take(&mut self.body_scripts);
        func.caller_frame_barrier = self.caller_frame_barrier;
        func.caller_frame_barrier_unplaced = self.caller_frame_barrier_unplaced;
        for (block, barrier) in std::mem::take(&mut self.caller_frame_barriers) {
            match self.block_ids.get(&block) {
                Some(id) => {
                    let own = func.caller_frame_barriers.entry(*id).or_default();
                    *own = own.union(barrier);
                }
                None => {
                    func.caller_frame_barrier_unplaced =
                        func.caller_frame_barrier_unplaced.union(barrier);
                }
            }
        }
        func.declared_frame_effects = self.command_bindings.declared_frame_effects();
        func.alias_observed_vars = std::mem::take(&mut self.alias_observed_vars);
        func
    }

    fn freeze_function_blocks(&mut self, name: &str, entry: &str, script: &Script) -> Function {
        // Seed the function's interner in block-creation order (by `BlockId`),
        // so the frozen interner's ids match the ones the builder stamped into
        // every terminator / loop node / exception edge.
        let mut func = Function::new(name, entry);
        func.metadata_context = self.command_classes.metadata.clone();
        func.namespace_context =
            script
                .namespace_context
                .clone()
                .or_else(|| match &self.invocation_namespace {
                    crate::ir::ExecutionNamespace::SourceContext(key) => {
                        Some(Box::new(key.clone()))
                    }
                    _ => None,
                });
        func.executed_source.clone_from(&script.executed_source);
        func.implicit_math_invocations = std::mem::take(&mut self.implicit_math_invocations);
        func.expression_preparations = std::mem::take(&mut self.expression_preparations);
        func.statement_sources = std::mem::take(&mut self.statement_sources);
        func.source_may_calls = std::mem::take(&mut self.source_may_calls);
        func.terminator_sources = std::mem::take(&mut self.terminator_sources);
        func.native_compilation_failure
            .clone_from(&script.native_compilation_failure);
        func.native_compilation_admission =
            crate::native_compilation_admission::retained_script_admission(script);
        let mut ordered: Vec<(BlockId, &String)> =
            self.block_ids.iter().map(|(n, id)| (*id, n)).collect();
        ordered.sort_by_key(|(id, _)| *id);
        for (_, n) in &ordered {
            func.intern_block((*n).clone());
        }

        func.blocks = self
            .blocks
            .drain()
            .map(|(k, mb)| {
                let id = func
                    .block_id(&k)
                    .unwrap_or_else(|| panic!("frozen block {k} not interned"));
                (
                    id,
                    Block {
                        name: mb.name,
                        statements: mb.statements,
                        terminator: mb.terminator,
                    },
                )
            })
            .collect();

        func
    }

    /// Lower a script (sequence of IR statements) into CFG blocks.
    ///
    /// `block_name` is the block where the first statement lands.
    /// Returns `Some(tail_block)` — the block where subsequent code
    /// should go — or `None` if control doesn't fall through (e.g.
    /// the script ends with a `return`).
    /// Depth-guarded entry to the recursive lowering. Every
    /// nested body re-enters here, so bounding this one point caps the
    /// whole `lower_*` recursion. At the cap we stop descending and report
    /// "no fall-through" — `build_function` already handles a `None` tail,
    /// so the result is a truncated-but-valid CFG rather than a stack
    /// overflow.
    /// Append under the exact source instance active during child lowering.
    fn push_statement(&mut self, block: &str, statement: Statement) {
        let id = self.block_ids[block];
        let index = self.blocks[block].statements.len();
        self.statement_sources
            .insert((id, index), self.current_source.clone());
        self.block_mut(block).statements.push(statement);
    }

    fn set_terminator(&mut self, block: &str, terminator: Terminator) {
        self.terminator_sources
            .insert(self.bid(block), self.current_source.clone());
        self.block_mut(block).terminator = Some(terminator);
    }
}

/// Body-keyed caller-frame, parameter and outer-write summaries, closed
/// command bindings, and the original Module owner selecting summary edges.
/// [`crate::compilation_unit`] shares this canonical tuple.
pub type CfgContext = (
    HashMap<crate::ir::ProcedureId, UpvarInfo>,
    HashMap<crate::ir::ProcedureId, Vec<String>>,
    HashMap<crate::ir::ProcedureId, GlobalWriteInfo>,
    ModuleCommandBindings,
    OriginalProcedureSummaryOwners,
);

/// Prepared module facts shared by every CFG builder in one lowering pipeline.
/// The tuple [`CfgContext`] remains the public compatibility boundary.
pub struct PreparedCfgContext {
    pub(crate) context: CfgContext,
    command_classes: CfgCommandClasses,
}

/// Build CFGs for a whole module: top-level script + each procedure.
///
/// When `defer_top_level` is `true`, `foreach`/`catch`/`try` at the
/// top level are compiled as opaque calls (matching tclsh bytecode
/// output). Analysis passes should leave this `false` to get full
/// inlining of loop bodies.
///
/// The builder also applies the upvar-invalidation pass — calls to
/// procedures whose bodies use `upvar` have their `defs` augmented
/// with the caller-side variable names the callee will modify.  See
/// [`prepare_cfg_context`] for the per-module scan.
#[must_use]
pub fn build_cfg(module: &Module, defer_top_level: bool) -> CfgModule {
    let registry = module.resolved_registry();
    let config = module.lexer_config;
    build_cfg_with_registry_and_config(module, defer_top_level, registry, config)
}

/// [`build_cfg`] with the document's exact [`tcl_lexer::LexerConfig`].
#[must_use]
pub fn build_cfg_with_config(
    module: &Module,
    defer_top_level: bool,
    config: tcl_lexer::LexerConfig,
) -> CfgModule {
    let registry = module.resolved_registry();
    build_cfg_with_registry_and_config(module, defer_top_level, registry, config)
}

/// Build analysis CFGs against the exact registry that lowered `module`.
#[must_use]
pub fn build_cfg_with_registry(
    module: &Module,
    defer_top_level: bool,
    registry: &CommandRegistry,
) -> CfgModule {
    build_cfg_with_registry_and_config(module, defer_top_level, registry, module.lexer_config)
}

/// Build analysis CFGs against both the exact command surface and lexer
/// configuration that lowered `module`.
#[must_use]
pub fn build_cfg_with_registry_and_config(
    module: &Module,
    defer_top_level: bool,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> CfgModule {
    build_cfg_inner(module, defer_top_level, true, registry, config)
}

/// Build analysis CFGs from an already-prepared registry-owned module context.
pub(crate) fn build_cfg_with_registry_and_context(
    module: &Module,
    defer_top_level: bool,
    registry: &CommandRegistry,
    context: &PreparedCfgContext,
    config: tcl_lexer::LexerConfig,
) -> CfgModule {
    build_cfg_inner_with_context(module, defer_top_level, true, registry, context, config)
}

/// Build CFGs for codegen (bytecode / WASM): the plain, byte-identical loop /
/// switch shape with no analysis-only transforms.
#[must_use]
pub fn build_cfg_codegen(module: &Module, defer_top_level: bool) -> CfgModule {
    let registry = module.resolved_registry();
    let config = module.lexer_config;
    build_cfg_codegen_with_registry_and_config(module, defer_top_level, registry, config)
}

/// [`build_cfg_codegen`] with the document's exact lexer configuration.
#[must_use]
pub fn build_cfg_codegen_with_config(
    module: &Module,
    defer_top_level: bool,
    config: tcl_lexer::LexerConfig,
) -> CfgModule {
    let registry = module.resolved_registry();
    build_cfg_codegen_with_registry_and_config(module, defer_top_level, registry, config)
}

/// Build code-generation CFGs against the exact registry that lowered
/// `module`.
#[must_use]
pub fn build_cfg_codegen_with_registry(
    module: &Module,
    defer_top_level: bool,
    registry: &CommandRegistry,
) -> CfgModule {
    build_cfg_codegen_with_registry_and_config(
        module,
        defer_top_level,
        registry,
        module.lexer_config,
    )
}

/// Build code-generation CFGs against both the exact command surface and lexer
/// configuration that lowered `module`.
#[must_use]
pub fn build_cfg_codegen_with_registry_and_config(
    module: &Module,
    defer_top_level: bool,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> CfgModule {
    build_cfg_inner(module, defer_top_level, false, registry, config)
}

/// Build code-generation CFGs from an already-prepared registry-owned module
/// context.
#[doc(hidden)]
pub fn build_cfg_codegen_with_registry_and_context(
    module: &Module,
    defer_top_level: bool,
    registry: &CommandRegistry,
    context: &PreparedCfgContext,
    config: tcl_lexer::LexerConfig,
) -> CfgModule {
    build_cfg_inner_with_context(module, defer_top_level, false, registry, context, config)
}

fn build_cfg_inner(
    module: &Module,
    defer_top_level: bool,
    faithful: bool,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> CfgModule {
    let context = prepare_cfg_context_bundle(module, registry);
    build_cfg_inner_with_context(
        module,
        defer_top_level,
        faithful,
        registry,
        &context,
        config,
    )
}

fn build_cfg_inner_with_context(
    module: &Module,
    defer_top_level: bool,
    faithful: bool,
    registry: &CommandRegistry,
    context: &PreparedCfgContext,
    config: tcl_lexer::LexerConfig,
) -> CfgModule {
    let (upvar_procs, proc_params, global_write_procs, command_bindings, procedure_summary_owners) =
        &context.context;

    let new_builder = |inline: bool| {
        let b = CfgBuilder::new_with_upvars_and_classes(
            inline,
            upvar_procs.clone(),
            proc_params.clone(),
            global_write_procs.clone(),
            command_bindings.clone(),
            registry,
            context.command_classes.clone(),
        )
        .with_procedure_summary_owners(procedure_summary_owners.clone())
        .with_lexer_config(config)
        .with_command_surface(module.plain_command_dispatch);
        if faithful {
            b.with_faithful_exceptions()
        } else {
            b
        }
    };

    let top_namespace = if module.top_level_namespace.is_empty() {
        "::".to_owned()
    } else {
        module.top_level_namespace.clone()
    };
    let top_context = module.top_level.execution_namespace(
        module.top_level_namespace_context.as_ref().map_or_else(
            || crate::ir::ExecutionNamespace::exact(top_namespace),
            |key| crate::ir::ExecutionNamespace::SourceContext(key.clone()),
        ),
    );
    let top_timeline =
        command_bindings.source_binding_timeline(&module.top_level, registry, &top_context, true);
    let mut top_builder = new_builder(!defer_top_level)
        .with_invocation_namespace(top_context)
        .with_source_binding_timeline(top_timeline.clone())
        .with_top_level_proc_body(module.top_level_kind == crate::ir::TopLevelKind::ProcedureBody);
    let top_cfg = top_builder.build_function("::top", &module.top_level);

    let mut proc_cfgs = HashMap::new();
    for (qname, proc) in &module.procedures {
        let namespace = proc
            .body
            .execution_namespace(crate::ir::ExecutionNamespace::RuntimeSelected);
        let timeline = top_timeline
            .entry_after(
                first_possible_invocation(module, registry, proc.span),
                command_bindings,
            )
            .map_or_else(
                || {
                    command_bindings
                        .source_binding_timeline(&proc.body, registry, &namespace, false)
                },
                |initial| {
                    ModuleCommandBindings::source_binding_timeline_from_initial(
                        &proc.body, registry, &namespace, &initial,
                    )
                },
            );
        let mut builder = new_builder(true)
            .with_invocation_namespace(namespace)
            .with_source_binding_timeline(timeline);
        proc_cfgs.insert(
            *qname,
            builder.build_function(&proc.qualified_name, &proc.body),
        );
    }

    CfgModule {
        top_level: top_cfg,
        procedures: proc_cfgs,
    }
}

/// The point before which the file's own top-level run cannot invoke the
/// procedure defined at `definition`: the first statement after the definition
/// that may run code, or past the end when none does. A body runs only once its
/// procedure is invoked, so a command defined before that point is bound when it
/// runs, and a call to it is the module's own (#2407). A statement is inert
/// when it is a definition, a literal assignment, or a literal invocation whose
/// registry footprint re-enters Tcl only through a trace. Under a trace an
/// inert statement may run code too, so there the definition itself is the
/// point.
fn first_possible_invocation(
    module: &Module,
    registry: &CommandRegistry,
    definition: Span,
) -> Span {
    let traced = module.has_dynamic_trace
        || module.has_dynamic_variable_trace
        || !module.traced_commands.is_empty()
        || !module.traced_variables.is_empty();
    if traced {
        return definition;
    }
    let literal = |word: &String| !word.contains(['$', '[', '\\']);
    let defines = |span: Span| module.procedures.values().any(|proc| proc.span == span);
    let inert = |stmt: &Statement| match stmt {
        Statement::AssignConst { .. } => true,
        Statement::AssignValue { value, .. } => literal(value),
        Statement::Call { command, args, .. } | Statement::Barrier { command, args, .. } => {
            defines(stmt.span())
                || (args.iter().all(literal) && reenters_tcl_only_by_trace(registry, command, args))
        }
        _ => false,
    };
    // A definition nested in a loop, a branch or a body may be reached again
    // after code that invokes it, so only a top-level one moves the point.
    if !module
        .top_level
        .statements
        .iter()
        .any(|stmt| stmt.span() == definition)
    {
        return definition;
    }
    let first = module
        .top_level
        .statements
        .iter()
        .filter(|stmt| stmt.span().start() >= definition.end())
        .find(|stmt| !inert(stmt))
        .map(Statement::span);
    first.map_or(Span::new(u32::MAX, u32::MAX), |at| {
        let before = at.start().saturating_sub(1);
        Span::new(before, before)
    })
}

/// Whether the registry's effect footprint for this literal invocation declares
/// that a variable trace is the only way it can re-enter Tcl. The caller rules
/// traces out for the module. An undeclared footprint is unknown, so the
/// invocation may run code.
fn reenters_tcl_only_by_trace(registry: &CommandRegistry, command: &str, args: &[String]) -> bool {
    let words: Vec<&str> = args.iter().map(String::as_str).collect();
    registry
        .resolve_invocation(command, &words, None)
        .is_some_and(|invocation| {
            invocation.semantics.world_effects.is_declared()
                && tcl_registry::CallbackKinds::TRACE
                    .contains(invocation.effects().callback().kinds)
        })
}

// Registry-derived command classifications
//
// These are facts of one resolved release/dialect registry, not a
// process-global union. In particular Tcl 8.4 has no builtin `throw`; a
// procedure with that name is an ordinary fall-through call in that profile.
// The owning [`CfgBuilder`] carries the classes derived for its module and
// passes them through the recursive flow-fact helpers below.

/// The actual source metadata owner, separate from explicit compatibility.
pub(crate) use crate::registry_invocation::OwnedInvocationMetadataContext as CfgMetadataContext;

#[derive(Clone)]
struct CfgCommandClasses {
    /// Actual immutable command surface used by the shared invocation owner.
    registry: tcl_registry::RegistrySnapshot,
    metadata: CfgMetadataContext,
    /// The registry's one effective-spec index, shared with command-binding
    /// analysis. Classification stays trait-driven without rebuilding five
    /// complete name sets for every compilation unit.
    #[cfg(test)]
    semantics: Arc<EffectiveRegistrySemantics>,
}

#[cfg(test)]
mod tests;
