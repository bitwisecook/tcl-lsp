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

use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use rustc_hash::FxHashMap;
use tcl_lexer::{Span, TokenType};
use tcl_registry::hooks::LoweringHookId;
use tcl_registry::model::ingress::static_context_for;
use tcl_registry::{CommandRegistry, EffectiveRegistrySemantics, Traits};

use crate::cfg::{Block, BlockId, CfgModule, Function, LoopNode, Terminator};
use crate::command_binding::ModuleCommandBindings;
use crate::expr_ast::ExprNode;
use crate::ir::{CommandBindingSite, CommandTokens, Module, Script, Statement};
use crate::ir_helpers::defs_from_ir_script;
use crate::naming::normalise_var_name;

use self::global_write_info::GlobalWriteInfo;
use self::upvar_info::{FrameReach, UpvarInfo};

/// Choose the [`CommandTokens`] for a "frozen" `while`/`for` runtime call.
///
/// The frozen-loop barrier hands the source words (the condition expression
/// text, the body script text) to the runtime `while` / `for` builtin, which
/// re-evaluates the condition and body on each iteration. Each word must be
/// pushed *as written*: a braced `{cond}` / `{body}` word verbatim
/// (substitution suppressed), a `$body` word interpolated. The codegen's
/// [`emit_call`] decides this from `argv_kinds[i] == Str && single_token_word`,
/// so the barrier needs the real per-word kinds.
///
/// `source` is the loop's recorded [`Statement::While::raw_tokens`] — the exact
/// segmenter token metadata. When present (always, for lowered loops) it is
/// used directly; the all-verbatim fallback ([`all_str_tokens`]) only covers
/// synthetically-constructed loops that carry no token metadata, matching the
/// braced-word idiom they are built from. Without this the words are pushed as
/// interpolated literals and the condition's command substitution is evaluated
/// *once* at the call site, freezing the loop (`while {[gets $f line] >= 0}`
/// happens to work, but a bare `while {[string length $x]}` would spin forever).
fn frozen_loop_tokens(cmd: &str, args: &[String], source: Option<&CommandTokens>) -> CommandTokens {
    source.cloned().unwrap_or_else(|| all_str_tokens(cmd, args))
}

/// Return `tokens` with the word at `idx` removed from every per-word vector.
///
/// Used to derive a `dict for`/`map` barrier's tokens from the source
/// `dict for {vars} $d {body}` tokens by dropping the `for`/`map` subcommand
/// word (index 1), so the remaining words line up with the barrier's
/// `::tcl::dict::for {vars} $d {body}` argv.
fn drop_word(tokens: &CommandTokens, idx: usize) -> CommandTokens {
    let mut out = tokens.clone();
    if idx < out.argv.len() {
        out.argv.remove(idx);
    }
    if idx < out.argv_texts.len() {
        out.argv_texts.remove(idx);
    }
    if idx < out.word_exprs.len() {
        out.word_exprs.remove(idx);
    }
    if idx < out.argv_kinds.len() {
        out.argv_kinds.remove(idx);
    }
    if idx < out.single_token_word.len() {
        out.single_token_word.remove(idx);
    }
    if let Some(expand) = out.expand_word.as_mut()
        && idx < expand.len()
    {
        expand.remove(idx);
    }
    debug_assert!(out.words_align_with_argv_text());
    out
}

/// Synthesise [`CommandTokens`] marking every word as a single brace-string
/// token — the verbatim fallback for a frozen loop with no recorded source
/// tokens. See [`frozen_loop_tokens`].
fn all_str_tokens(cmd: &str, args: &[String]) -> CommandTokens {
    let word_count = 1 + args.len();
    let mut argv_texts = Vec::with_capacity(word_count);
    argv_texts.push(cmd.to_owned());
    argv_texts.extend(args.iter().cloned());
    CommandTokens::from_lossy_parts(
        vec![Span::new(0, 0); word_count],
        argv_texts,
        vec![TokenType::Str; word_count],
        vec![true; word_count],
        Vec::new(),
        None,
    )
}

mod cfg_lower;
pub mod global_write_info;
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
    counter: u32,
    blocks: HashMap<String, MutableBlock>,
    /// Block name → [`BlockId`], assigned in block-creation order so the
    /// frozen [`Function`]'s interner reflects that order.
    block_ids: FxHashMap<String, BlockId>,
    loop_nodes: HashMap<String, LoopNode>,
    inline_loops: bool,
    /// Map from command name to upvar summary, used to pre-populate
    /// caller-side `defs` on calls to procs that use `upvar`.  Empty
    /// when the builder is constructed without an upvar context
    /// (e.g. for one-off CFGs that don't have a Module to scan).
    upvar_procs: HashMap<String, UpvarInfo>,
    /// Map from command name to parameter list, used by the upvar
    /// wiring to resolve param-based upvar sources (`upvar 1 $param
    /// local`) against the actual call-site argument.
    proc_params: HashMap<String, Vec<String>>,
    /// Map from command name to the outer-scope (global/namespace) variable
    /// names that proc's body writes (see [`global_write_info`]), used to
    /// widen caller-side `defs` at a call site exactly like `upvar_procs` —
    /// so SCCP treats a global/namespace name as overdefined across an
    /// opaque call that writes it, not as an ordinary untouched local.
    global_write_procs: HashMap<String, GlobalWriteInfo>,
    /// Closed module command state used to resolve direct and parsed embedded
    /// spellings to effective user-procedure targets, including alias chains.
    command_bindings: ModuleCommandBindings,
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
    /// Entry block to continuation block for an explicitly delimited inline
    /// structured-command region.
    command_boundary_continuations: HashMap<String, String>,
    /// Caller-frame injection the function currently being built is subject
    /// to: the union of every called procedure's
    /// [`UpvarInfo::caller_frame_barrier`].  Drained into
    /// [`crate::cfg::Function::caller_frame_barrier`], which
    /// [`crate::dynamic_names::dynamic_name_barrier`] folds into the
    /// function's blindness lattice.
    caller_frame_barrier: crate::dynamic_names::DynamicNameBarrier,
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
/// stack cost, rather than three copies of a hand-picked 256 (issue #1654).
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
        upvar_procs: HashMap<String, UpvarInfo>,
        proc_params: HashMap<String, Vec<String>>,
        global_write_procs: HashMap<String, GlobalWriteInfo>,
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
        upvar_procs: HashMap<String, UpvarInfo>,
        proc_params: HashMap<String, Vec<String>>,
        global_write_procs: HashMap<String, GlobalWriteInfo>,
        command_bindings: ModuleCommandBindings,
        registry: &'a CommandRegistry,
        command_classes: CfgCommandClasses,
    ) -> Self {
        Self {
            counter: 0,
            blocks: HashMap::new(),
            block_ids: FxHashMap::default(),
            loop_nodes: HashMap::new(),
            inline_loops,
            upvar_procs,
            proc_params,
            global_write_procs,
            command_bindings,
            invocation_namespace: crate::ir::ExecutionNamespace::exact("::"),
            widen_oo_dispatch: false,
            loop_stack: Vec::new(),
            exception_edges: Vec::new(),
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
            command_boundary_continuations: HashMap::new(),
            caller_frame_barrier: crate::dynamic_names::DynamicNameBarrier::default(),
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

    /// Registry-owned variable writes projected in the function's actual
    /// command-resolution context. A receiver-selected relative head may be
    /// shadowed by an arbitrary object-local command and therefore widens the
    /// whole variable frame.
    fn variable_write_projection(&self, stmt: &Statement) -> tcl_registry::VariableWriteProjection {
        let (Statement::Call { command: head, .. } | Statement::Barrier { command: head, .. }) =
            stmt
        else {
            return tcl_registry::VariableWriteProjection::default();
        };
        let Some(namespace) = self.invocation_namespace.for_head(head) else {
            if !statement_has_literal_head(stmt) {
                return tcl_registry::VariableWriteProjection::default();
            }
            return tcl_registry::VariableWriteProjection {
                literal_names: Vec::new(),
                opaque_variable_frame: true,
            };
        };
        self.command_bindings
            .variable_write_projection(stmt, self.registry, namespace)
    }

    /// Resolve the caller-frame effects of every retained user procedure a
    /// direct source invocation may reach. Command identity and alias-prefix
    /// composition come exclusively from [`ModuleCommandBindings`].
    fn direct_upvar_effects(&self, stmt: &Statement) -> ResolvedUpvarEffects {
        let mut combined = ResolvedUpvarEffects::default();
        let (Statement::Call { command: head, .. } | Statement::Barrier { command: head, .. }) =
            stmt
        else {
            return combined;
        };
        let Some(namespace) = self.invocation_namespace.for_head(head) else {
            if !statement_has_literal_head(stmt) {
                return combined;
            }
            combined.has_unresolvable_target = true;
            combined.opaque_arguments = true;
            combined.frame_barrier = crate::dynamic_names::DynamicNameBarrier::OPAQUE_SCRIPT;
            return combined;
        };
        self.command_bindings
            .for_each_resolved_invocation(stmt, namespace, |target, words| {
                self.extend_upvar_effects(
                    &mut combined,
                    &target.command,
                    target.registry_backed,
                    words.arguments(),
                );
            });
        combined
    }

    /// Fold one binding-resolved terminal user procedure into a caller-frame
    /// effect set. Both direct statements and parsed embedded commands enter
    /// here, so their parameter projection and barrier semantics cannot drift.
    fn extend_upvar_effects(
        &self,
        combined: &mut ResolvedUpvarEffects,
        command: &str,
        registry_backed: bool,
        arguments: tcl_registry::InvocationArguments<'_>,
    ) {
        if registry_backed {
            // TclOO self/next dispatch has no statically named method target.
            // The method-barrier pass enables this capability only for a
            // method whose reachable dispatch surface can mutate its caller.
            let is_oo_dispatch = matches!(
                self.registry.method_dispatch_keyword(command),
                Some(
                    tcl_registry::MethodDispatchKind::SelfDispatch
                        | tcl_registry::MethodDispatchKind::NextChain
                )
            );
            if self.widen_oo_dispatch && is_oo_dispatch {
                combined.has_unresolvable_target = true;
                combined.frame_barrier =
                    combined
                        .frame_barrier
                        .union(crate::dynamic_names::DynamicNameBarrier {
                            writes: true,
                            destroys: false,
                            reads: false,
                        });
            }
            return;
        }
        if !self.proc_params.contains_key(command) {
            // The binding owner can recover callable procedure bodies that
            // are absent from Module::procedures. Until summaries carry body
            // generation identities, never treat such a terminal target as
            // an effect-free retained procedure.
            combined.has_unresolvable_target = true;
            combined.opaque_arguments = true;
            combined.frame_barrier = combined
                .frame_barrier
                .union(crate::dynamic_names::DynamicNameBarrier::OPAQUE_SCRIPT);
            return;
        }
        let Some(info) = self.upvar_procs.get(command) else {
            return;
        };
        let params: &[String] = self.proc_params.get(command).map_or(&[][..], Vec::as_slice);
        let effects = info.caller_side_effects(arguments, params);
        combined.opaque_arguments |= effects.opaque;
        combined.has_unresolvable_target |= info.has_unresolvable_caller_target;
        combined.frame_barrier = combined.frame_barrier.union(info.caller_frame_barrier());
        for name in effects.defs {
            if !combined.defs.contains(&name) {
                combined.defs.push(name);
            }
        }
    }

    /// Augment a statement's effective `defs` with caller-side
    /// variable names that any callee proc will modify via `upvar`.
    /// Returns a list of statements — the original (possibly with
    /// merged `defs` for the direct-call form) plus an optional
    /// synthetic `<upvar-invalidate>` `Statement::Call` prepended
    /// when the embedded-substitution form contributes defs that
    /// can't be merged into the host statement (e.g. an
    /// `AssignValue` whose `value` text contains `[upvar_proc arg]`).
    ///
    /// Direct-call form: resolves the statement through the module command
    /// state, then maps the effective user-procedure target and its
    /// alias-prepended structured arguments through `UpvarInfo` into the
    /// call's own `defs`.
    ///
    /// Embedded-substitution form: scans the call's args / the
    /// `AssignValue`'s value text for `[command_substitution]`
    /// tokens whose head is a known upvar proc; merges those defs
    /// into the host Call when possible, or emits a synthetic
    /// `<upvar-invalidate>` Call before a non-Call host.
    ///
    /// The same two forms also widen `defs` with `global_write_procs`
    /// (a callee that writes an outer-scope name via `global`/`variable`/
    /// `upvar #0`, see [`global_write_info`]) — a global name doesn't
    /// depend on call-site arguments, so no params-based mapping is
    /// needed, just the literal name list.
    fn apply_upvar_invalidation(&mut self, stmt: Statement) -> Vec<Statement> {
        self.record_caller_frame_barrier(&stmt);
        self.record_alias_observed(&stmt);
        self.upvar_invalidated(stmt)
    }

    /// Record every caller-frame name a callee invoked by `stmt` may touch
    /// through an `upvar` alias or an `uplevel` write, so the dead-store /
    /// unused-assignment passes never delete a store such a callee can
    /// observe (`set callervar 5; get` where `get` runs `upvar 1 callervar
    /// m; return $m` — issue #1193's upvar differential).  The names land on
    /// [`crate::cfg::Function::alias_observed_vars`]; recording them as
    /// *reads* on the call statement instead would fabricate
    /// read-before-set uses (a false W210) for the pure out-param shape.
    fn record_alias_observed(&mut self, stmt: &Statement) {
        if self.upvar_procs.is_empty() {
            return;
        }
        self.alias_observed_vars
            .extend(self.direct_upvar_effects(stmt).defs);
        let embedded = crate::ir_helpers::evaluated_command_substitutions(stmt, self.registry);
        self.alias_observed_vars
            .extend(self.upvar_effects_from_commands(&embedded.commands).defs);
    }

    /// Whole-frame blindness the statement's calls impose on *this*
    /// function, as distinct from the per-name `defs` widening
    /// [`Self::upvar_invalidated`] applies.
    ///
    /// A callee that runs `uplevel 1 $body` — or aliases a caller-frame name
    /// it cannot place — reaches names no `defs` list can enumerate.
    /// Recorded flow-insensitively on the function, in the same lattice and
    /// the same abstention direction as the dynamic-name barrier
    /// ([`crate::dynamic_names`]).
    fn record_caller_frame_barrier(&mut self, stmt: &Statement) {
        let embedded = crate::ir_helpers::evaluated_command_substitutions(stmt, self.registry);
        if embedded.opaque {
            self.caller_frame_barrier = self
                .caller_frame_barrier
                .union(crate::dynamic_names::DynamicNameBarrier::OPAQUE_SCRIPT);
        }
        let direct = self.direct_upvar_effects(stmt);
        let embedded = self.upvar_effects_from_commands(&embedded.commands);
        self.caller_frame_barrier = self
            .caller_frame_barrier
            .union(direct.frame_barrier)
            .union(embedded.frame_barrier);
        if direct.opaque_arguments || embedded.opaque_arguments {
            self.caller_frame_barrier =
                self.caller_frame_barrier
                    .union(crate::dynamic_names::DynamicNameBarrier {
                        writes: true,
                        destroys: false,
                        reads: false,
                    });
        }
    }

    /// The `defs`-widening half of [`Self::apply_upvar_invalidation`], with
    /// no side effect on the function-level barrier — so a speculative query
    /// ([`Self::init_written_names`]) can ask what a statement writes without
    /// recording the statement twice.
    fn upvar_invalidated(&self, mut stmt: Statement) -> Vec<Statement> {
        // 1. Direct-call extras: command is a known upvar proc / a proc
        //    that writes outer-scope names.
        let direct_extras = self.direct_call_extras(&stmt);

        // 2. A callee whose caller-frame effect cannot be fully enumerated per
        //    name widens the call site with an opaque barrier after the call
        //    (SCCP/propagation widen every tracked value at a
        //    `Statement::Barrier`). Keep processing the call, though: a
        //    summary may contain both precise caller-side defs and an opaque
        //    remainder, and dropping the known defs loses useful facts such
        //    as `uplevel 1 [list set $parameter value]`.
        let direct_opaque_barrier = self.opaque_call_barrier(&stmt);

        // 3. Embedded-substitution extras: walk text for
        //    `[upvar_proc arg]` / `[global_write_proc arg]` substitutions.
        let (embedded_extras, embedded_opaque_global) = self.embedded_subst_extras(&stmt);

        if direct_extras.is_empty() && embedded_extras.is_empty() && !embedded_opaque_global {
            return match direct_opaque_barrier {
                Some(barrier) => vec![stmt, barrier],
                None => vec![stmt],
            };
        }

        // 2b. An embedded call to a proc that runs an unreadable script at
        //     the global frame (`set y [setter]` where `setter` does
        //     `uplevel #0 $body`, issue #1198): no def list can enumerate
        //     what it clobbers, so prepend an opaque barrier — the same
        //     program-order position the synthetic `<upvar-invalidate>`
        //     uses, so the host statement's own reads already see the
        //     widened state.
        let opaque_barrier = embedded_opaque_global.then(|| Statement::Barrier {
            span: stmt.span(),
            reason: "embedded call runs an unreadable script at the global frame".to_owned(),
            command: "<global-frame-script>".to_owned(),
            canonical_command: None,
            args: Vec::new(),
            tokens: Some(crate::ir::CommandTokens::marker(
                crate::ir::SyntheticMarker::GlobalFrameScript,
            )),
        });

        // 3. Merge into the host statement when it's a Call.
        if let Statement::Call { defs, .. } = &mut stmt {
            for d in direct_extras {
                if !defs.contains(&d) {
                    defs.push(d);
                }
            }
            for d in embedded_extras {
                if !defs.contains(&d) {
                    defs.push(d);
                }
            }
            let mut out = Vec::new();
            if let Some(barrier) = opaque_barrier {
                out.push(barrier);
            }
            out.push(stmt);
            if let Some(barrier) = direct_opaque_barrier {
                out.push(barrier);
            }
            return out;
        }

        // 4. Non-Call host (e.g. AssignValue) with embedded extras —
        //    emit a synthetic `<upvar-invalidate>` Call before the
        //    host so the affected vars are invalidated in
        //    program order.
        let mut out = Vec::new();
        if let Some(barrier) = opaque_barrier {
            out.push(barrier);
        }
        if !embedded_extras.is_empty() {
            out.push(Statement::Call {
                span: stmt.span(),
                command: "<upvar-invalidate>".to_string(),
                canonical_command: None,
                args: Vec::new(),
                defs: embedded_extras,
                reads: Vec::new(),
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: Some(crate::ir::CommandTokens::marker(
                    crate::ir::SyntheticMarker::UpvarInvalidate,
                )),
                foreach_groups: None,
            });
        }
        out.push(stmt);
        if let Some(barrier) = direct_opaque_barrier {
            out.push(barrier);
        }
        out
    }

    /// The opaque widening barrier for a direct call whose callee's
    /// caller-frame effect has no sound per-name def list: a callee whose
    /// `upvar` caller-side name is unresolvable (`upvar 1 $computed x`) can
    /// write ANY caller variable, and a callee that runs an unreadable
    /// script at the global frame (`uplevel #0 $body`, issue #1198) can
    /// write or read ANY global/namespace name.
    fn opaque_call_barrier(&self, stmt: &Statement) -> Option<Statement> {
        let Statement::Call {
            command,
            canonical_command,
            span,
            tokens,
            ..
        } = stmt
        else {
            return None;
        };
        let literal_head = tokens.as_ref().is_none_or(|tokens| {
            tokens.synthetic.is_none()
                && tokens.words().first().is_none_or(|head| {
                    crate::registry_invocation::invocation_word(head)
                        .literal()
                        .is_some()
                })
        });
        let target = canonical_command.as_deref().unwrap_or(command.as_str());
        let direct_upvar = self.direct_upvar_effects(stmt);
        let unresolvable_upvar = direct_upvar.has_unresolvable_target;
        let opaque_global = self
            .global_write_procs
            .get(target)
            .is_some_and(|info| literal_head && info.opaque_global_frame);
        let source_opaque_upvar = direct_upvar.opaque_arguments;
        let opaque_variable_write = self.variable_write_projection(stmt).opaque_variable_frame;
        if !unresolvable_upvar && !source_opaque_upvar && !opaque_global && !opaque_variable_write {
            return None;
        }
        let reason = if unresolvable_upvar || source_opaque_upvar {
            format!("{command} upvar-aliases a dynamic caller variable")
        } else if opaque_global {
            format!("{command} runs an unreadable script at the global frame")
        } else {
            format!("{command} writes a source-opaque variable name")
        };
        Some(Statement::Barrier {
            span: *span,
            // A widening *effect*, not a command to run: the call itself is
            // already in the statement stream immediately beside this barrier,
            // so naming the callee here made codegen invoke it a second time
            // (issue #1602 — `proc p {} { upvar 1 {a b} v ; puts "u=$v" }; p`
            // printed `u=…` twice on the VM where tclsh 8.6.14 / 9.0.4 print it
            // once; `proc setter {body} { uplevel #0 $body }; setter {set q 1}`
            // failed with `wrong # args` from the re-invoke). The typed
            // `SyntheticMarker` on the tokens is what stops codegen
            // dispatching it; the `command` spelling below is a label for the
            // disassembly and the explorer, and `reason` keeps the callee's
            // name.
            reason,
            command: "<caller-frame-opaque>".to_owned(),
            canonical_command: None,
            args: Vec::new(),
            tokens: Some(crate::ir::CommandTokens::marker(
                crate::ir::SyntheticMarker::CallerFrameOpaque,
            )),
        })
    }

    /// The direct-call half of [`Self::upvar_invalidated`]: the caller-side
    /// names a known upvar proc / global-writing proc defines at this call.
    /// Extras are merged as DEFS only; the "the callee may also *read* the
    /// aliased cell" half is recorded on
    /// [`crate::cfg::Function::alias_observed_vars`] by
    /// [`Self::record_alias_observed`] (a read here would fabricate
    /// read-before-set uses — a false W210 — for the pure out-param shape).
    fn direct_call_extras(&self, stmt: &Statement) -> Vec<String> {
        let Statement::Call {
            command,
            canonical_command,
            ..
        } = stmt
        else {
            return Vec::new();
        };
        let target = canonical_command.as_deref().unwrap_or(command.as_str());
        let mut extras = self.direct_upvar_effects(stmt).defs;
        for name in self.variable_write_projection(stmt).literal_names {
            if !extras.contains(&name) {
                extras.push(name);
            }
        }
        if let Some(info) = self.global_write_procs.get(target) {
            for name in &info.names {
                if !extras.contains(name) {
                    extras.push(name.clone());
                }
            }
        }
        extras
    }

    /// The embedded-substitution half of [`Self::upvar_invalidated`]: the
    /// caller-side defs contributed by `[…]` substitutions in the
    /// statement's argument words (or an assignment's value), plus whether
    /// any embedded callee runs an unreadable script at the global frame.
    fn embedded_subst_extras(&self, stmt: &Statement) -> (Vec<String>, bool) {
        let embedded = crate::ir_helpers::evaluated_command_substitutions(stmt, self.registry);
        let mut embedded_extras: Vec<String> = Vec::new();
        let mut embedded_opaque_global = embedded.opaque;
        let upvar = self.upvar_effects_from_commands(&embedded.commands);
        embedded_opaque_global |= upvar.opaque_arguments;
        for d in upvar.defs {
            if !embedded_extras.contains(&d) {
                embedded_extras.push(d);
            }
        }
        let (global_defs, opaque) = self.global_write_defs_from_commands(&embedded.commands);
        embedded_opaque_global |= opaque;
        for d in global_defs {
            if !embedded_extras.contains(&d) {
                embedded_extras.push(d);
            }
        }
        // A var-mutating builtin inside a command substitution
        // (`set y [append x b]`, `[incr x]`, `[lset l …]`) writes its
        // target variable as a side effect; record it so copy / constant
        // propagation (O100) does not propagate a stale value past the
        // mutation (FP-OPT-06).
        let writes = crate::ir_helpers::variable_write_effects_from_commands(
            &embedded.commands,
            self.registry,
        );
        embedded_opaque_global |= writes.opaque;
        for d in writes.names {
            if !embedded_extras.contains(&d) {
                embedded_extras.push(d);
            }
        }
        (embedded_extras, embedded_opaque_global)
    }

    fn upvar_effects_from_commands(
        &self,
        commands: &[Vec<crate::ir_helpers::CommandWord>],
    ) -> ResolvedUpvarEffects {
        let mut combined = ResolvedUpvarEffects::default();
        for words in commands {
            let Some(head) = words.first() else {
                continue;
            };
            if head.literal().is_none() {
                combined.opaque_arguments = true;
                continue;
            }
            let head_name = head.literal().expect("literal head checked above");
            let Some(namespace) = self.invocation_namespace.for_head(head_name) else {
                combined.opaque_arguments = true;
                combined.frame_barrier = combined
                    .frame_barrier
                    .union(crate::dynamic_names::DynamicNameBarrier::OPAQUE_SCRIPT);
                continue;
            };
            self.command_bindings.for_each_resolved_command_words(
                words,
                namespace,
                |target, invocation| {
                    self.extend_upvar_effects(
                        &mut combined,
                        &target.command,
                        target.registry_backed,
                        invocation.arguments(),
                    );
                },
            );
        }
        combined
    }

    /// Accumulate the outer-scope (global/namespace) names any embedded call
    /// to a known global-writing proc writes. A
    /// global write reached via `set y [mutate]` is just as real as one
    /// reached via a bare `mutate` statement.
    ///
    /// The second return is `true` when any embedded callee's summary
    /// carries [`GlobalWriteInfo::opaque_global_frame`] (`uplevel #0 $body`,
    /// issue #1198) — no def list can enumerate that, so the caller must
    /// widen with an opaque barrier.
    fn global_write_defs_from_commands(
        &self,
        commands: &[Vec<crate::ir_helpers::CommandWord>],
    ) -> (Vec<String>, bool) {
        let mut defs: Vec<String> = Vec::new();
        let mut opaque = false;
        for words in commands {
            let Some(cmd) = words.first() else {
                continue;
            };
            let Some(cmd_name) = cmd.literal() else {
                opaque = true;
                continue;
            };
            let Some(namespace) = self.invocation_namespace.for_head(cmd_name) else {
                opaque = true;
                continue;
            };
            opaque |= self
                .command_bindings
                .target_resolution_may_be_unknown(cmd_name, namespace);
            for target in self.command_bindings.targets(cmd_name, namespace) {
                if let Some(info) = self.global_write_procs.get(&target.command) {
                    opaque |= info.opaque_global_frame;
                    for name in &info.names {
                        if !defs.contains(name) {
                            defs.push(name.clone());
                        }
                    }
                }
            }
        }
        (defs, opaque)
    }

    /// Condition-position command-substitution out-vars (issue #923 idx
    /// 122): unions the registry's `ArgRole::VarWrite` scan
    /// ([`crate::ir_helpers::condition_command_out_vars`]) with the same
    /// known-upvar-proc /
    /// known-global-writer resolution every *other* embedded-substitution
    /// site already gets. Without this, a user
    /// proc's `upvar` write was only recognised as a bare statement or an
    /// ordinary value (`set x [getKnownOpt ...]`) — invoked from a
    /// `while`/`if` *condition* instead (`while {[getopt argv $opts opt
    /// arg]} { ... }`, tcllib's `cmdline::getoptions`), the write was
    /// invisible, producing a false W210 on the guarded body's read even
    /// though the condition's own command substitution (including the
    /// upvar write) completes before the body ever runs
    /// (tclsh9.0/8.6-verified).
    /// The second return is `true` when a condition-embedded callee runs an
    /// unreadable script at the global frame (issue #1198): the caller must
    /// then push an opaque barrier alongside the `<cond>` defs, because no
    /// def list can enumerate what the condition's evaluation clobbers.
    fn condition_out_vars(&self, condition: &ExprNode) -> (Vec<String>, bool) {
        let mut out = crate::ir_helpers::condition_command_out_vars(condition, self.registry);
        let embedded =
            crate::ir_helpers::expression_command_substitutions(condition, self.registry);
        let upvar = self.upvar_effects_from_commands(&embedded.commands);
        let opaque_upvar = upvar.opaque_arguments;
        for d in upvar.defs {
            if !out.contains(&d) {
                out.push(d);
            }
        }
        let (global_defs, mut opaque) = self.global_write_defs_from_commands(&embedded.commands);
        opaque |= embedded.opaque || opaque_upvar;
        let writes = crate::ir_helpers::variable_write_effects_from_commands(
            &embedded.commands,
            self.registry,
        );
        opaque |= writes.opaque;
        for d in writes.names {
            if !out.contains(&d) {
                out.push(d);
            }
        }
        for d in global_defs {
            if !out.contains(&d) {
                out.push(d);
            }
        }
        (out, opaque)
    }

    /// Push the `<cond>` synthetic call (and, when the condition's embedded
    /// callees demand it, an opaque barrier) for a condition that contains
    /// command substitutions — the shared tail of `lower_if`, `lower_while`,
    /// and the frozen-loop barrier.
    fn push_condition_effects(&mut self, condition: &ExprNode, span: Span, block: &str) {
        let (cond_defs, opaque) = self.condition_out_vars(condition);
        if !cond_defs.is_empty() {
            self.block_mut(block).statements.push(Statement::Call {
                span,
                command: "<cond>".into(),
                canonical_command: None,
                args: Vec::new(),
                defs: cond_defs,
                reads: Vec::new(),
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: Some(crate::ir::CommandTokens::marker(
                    crate::ir::SyntheticMarker::Condition,
                )),
                foreach_groups: None,
            });
        }
        if opaque {
            self.block_mut(block).statements.push(Statement::Barrier {
                span,
                reason: "condition runs an unreadable script at the global frame".into(),
                command: "<global-frame-script>".into(),
                canonical_command: None,
                args: Vec::new(),
                tokens: Some(crate::ir::CommandTokens::marker(
                    crate::ir::SyntheticMarker::GlobalFrameScript,
                )),
            });
        }
    }

    /// If `stmt` is a loop jump (the registry's `BREAKS_LOOP` /
    /// `CONTINUES_LOOP` classes) inside a loop, push it into
    /// `current` and set a `Goto` terminator to the loop's exit / continue
    /// target, returning `true`.  Returns `false` (no-op) otherwise.
    /// Matched against the raw command word (no `::` trimming), as the
    /// retired hardcoded comparison was.
    fn lower_loop_jump(&mut self, current: &str, stmt: &Statement) -> bool {
        if self.plain_command_dispatch {
            return false;
        }
        let Statement::Call { command, span, .. } = stmt else {
            return false;
        };
        let is_break = self.command_classes.is_loop_break_command(command);
        if !is_break && !self.command_classes.is_loop_continue_command(command) {
            return false;
        }
        let Some((brk, cont)) = self.loop_stack.last().cloned() else {
            return false;
        };
        let target_name = if is_break { brk } else { cont };
        let target = self.bid(&target_name);
        self.block_mut(current).statements.push(stmt.clone());
        self.block_mut(current).terminator = Some(Terminator::Goto {
            target,
            span: Some(*span),
        });
        true
    }

    /// Push a non-control-flow statement into `current` (after upvar
    /// invalidation), promoting `error` / `throw` / `exit` (and, in analysis
    /// builds, `tailcall`) to a `Return` terminator so any following statements
    /// become dead code (mirrors the `TERMINATES_BLOCK` registry trait).
    fn push_plain_statement(&mut self, current: &str, stmt: &Statement) {
        for s in self.apply_upvar_invalidation(stmt.clone()) {
            self.block_mut(current).statements.push(s);
        }
        if self.plain_command_dispatch {
            return;
        }
        if let Statement::Call {
            command,
            canonical_command,
            span,
            ..
        }
        | Statement::Barrier {
            command,
            canonical_command,
            span,
            ..
        } = stmt
            && self.block_mut(current).terminator.is_none()
        {
            let canon = canonical_command.as_deref().unwrap_or(command);
            // `tailcall` (Tcl 8.6+, FP-RBS-13) replaces the current frame and
            // never returns here, so it ends straight-line flow exactly like
            // `error`/`exit`.  Promote it only in analysis builds
            // (`faithful_exceptions`) so the codegen / non-faithful CFG shape
            // stays byte-identical — codegen leaves the call as a fall-through.
            let exits_proc = self.command_classes.is_block_terminating_command(canon)
                || (self.faithful_exceptions && self.command_classes.is_tailcall_command(canon));
            if exits_proc {
                // A catchable `error` / `throw` (not `exit` / `tailcall`, which
                // leave the process / pop the frame) is a throw point: record
                // the current block so an enclosing `try`'s on-error edge can be
                // sourced from here, where the body's prior defs are live.
                if self.command_classes.is_catchable_throw(canon)
                    && let Some(blocks) = self.throw_blocks.as_mut()
                {
                    blocks.push(current.to_owned());
                }
                self.block_mut(current).terminator = Some(Terminator::Return {
                    value: None,
                    span: Some(*span),
                    expr: None,
                    braced: false,
                });
            }
        }
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
    fn build_function(&mut self, name: &str, script: &Script) -> Function {
        let entry = self.new_block("entry");
        let tail = self.lower_script(script, &entry);
        if let Some(tail) = tail {
            let exit = self.new_block("exit");
            self.ensure_goto(&tail, &exit, None);
        }

        // Seed the function's interner in block-creation order (by `BlockId`),
        // so the frozen interner's ids match the ones the builder stamped into
        // every terminator / loop node / exception edge.
        let mut func = Function::new(name, &entry);
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

        func.loop_nodes = std::mem::take(&mut self.loop_nodes)
            .into_iter()
            .map(|(k, ln)| (self.bid(&k), ln))
            .collect();
        func.exception_edges = std::mem::take(&mut self.exception_edges)
            .into_iter()
            .map(|(from, to)| (self.bid(&from), self.bid(&to)))
            .collect();
        func.inline_body_error_sites = std::mem::take(&mut self.inline_body_error_sites);
        func.command_binding_sites = std::mem::take(&mut self.command_binding_sites);
        func.procedure_binding_requirements =
            std::mem::take(&mut self.procedure_binding_requirements);
        func.command_boundary_sites = std::mem::take(&mut self.command_boundary_sites)
            .into_iter()
            .map(|(block, site)| (self.bid(&block), site))
            .collect();
        func.command_boundary_continuations =
            std::mem::take(&mut self.command_boundary_continuations)
                .into_iter()
                .map(|(entry, continuation)| (self.bid(&entry), self.bid(&continuation)))
                .collect();
        func.caller_frame_barrier = self.caller_frame_barrier;
        func.alias_observed_vars = std::mem::take(&mut self.alias_observed_vars);
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
    fn lower_script(&mut self, script: &Script, block_name: &str) -> Option<String> {
        self.command_binding_sites
            .extend(script.command_binding_sites.iter().cloned());
        self.procedure_binding_requirements
            .extend(script.procedure_binding_requirements.iter().cloned());
        self.depth += 1;
        if MAX_LOWER_DEPTH.exceeded(self.depth) {
            self.depth -= 1;
            return None;
        }
        let result = self.lower_script_inner(script, block_name);
        self.depth -= 1;
        result
    }

    /// Push a "frozen" loop (`for` / `while` whose condition is a command
    /// substitution) into `current` as an opaque [`Statement::Barrier`]; the
    /// body is kept un-lowered so it is treated as an opaque effect.
    ///
    /// A `Statement::Barrier` has no `defs` field — its own
    /// [`crate::ssa::uses_of`] textually scans *both* `condition` and the
    /// un-lowered body text for `$var` reads (attributing them all to the
    /// barrier's own span), but nothing analogous computes defs for it
    /// beyond the registry's ordinary `VarWrite` role query, which `for` /
    /// `while` themselves never declare. So a `catch`/`regexp`/`scan`
    /// result var, or a known upvar/global-writing user proc's write,
    /// reached only through the frozen condition (`while {[getopt argv
    /// $opts opt arg]} { ... }`, tcllib's `cmdline::getoptions` — issue
    /// #923 idx 122) was invisible to the def-use graph: the read inside
    /// the (un-lowered, but still textually-scanned) body looked
    /// read-before-set even though the condition's own command
    /// substitution completes — including the write — before the body
    /// (or even a second evaluation of the condition) ever runs. Fixed by
    /// pushing a synthetic `<cond>` `Statement::Call` carrying those defs
    /// immediately before the barrier, exactly mirroring the non-frozen
    /// `lower_if`/`lower_while` path's own `condition_out_vars` use — the
    /// barrier's textually-scanned read then resolves to this new SSA
    /// version instead of the undef origin.
    fn push_frozen_loop_barrier(
        &mut self,
        command: &str,
        condition: &ExprNode,
        raw_args: &[String],
        raw_tokens: Option<&CommandTokens>,
        span: Span,
        current: &str,
    ) {
        self.push_condition_effects(condition, span, current);
        self.block_mut(current).statements.push(Statement::Barrier {
            span,
            reason: format!("frozen {command} (cmd-subst condition)"),
            command: command.to_owned(),
            canonical_command: None,
            args: raw_args.to_vec(),
            tokens: Some(frozen_loop_tokens(command, raw_args, raw_tokens)),
        });
    }

    /// Lower a `for`, or freeze it as an opaque barrier when its condition is a
    /// command substitution (`for {…} [cond] {…} {…}`).
    fn lower_for_or_frozen(&mut self, stmt: &Statement, current: &str) -> Option<String> {
        if let Statement::For {
            condition,
            raw_args,
            raw_tokens,
            span,
            ..
        } = stmt
            && matches!(condition, ExprNode::Command { .. })
            && !raw_args.is_empty()
        {
            self.push_frozen_loop_barrier(
                "for",
                condition,
                raw_args,
                raw_tokens.as_ref(),
                *span,
                current,
            );
            Some(current.to_owned())
        } else {
            self.lower_for(stmt, current)
        }
    }

    /// Lower a `while`, or freeze it as an opaque barrier when its condition is
    /// a command substitution (`while [cond] {…}`).
    fn lower_while_or_frozen(&mut self, stmt: &Statement, current: &str) -> String {
        if let Statement::While {
            condition,
            raw_args,
            raw_tokens,
            span,
            ..
        } = stmt
            && matches!(condition, ExprNode::Command { .. })
            && !raw_args.is_empty()
        {
            self.push_frozen_loop_barrier(
                "while",
                condition,
                raw_args,
                raw_tokens.as_ref(),
                *span,
                current,
            );
            current.to_owned()
        } else {
            self.lower_while(stmt, current)
        }
    }

    /// `return -options …` / `return {*}…args`: push the original barrier
    /// (codegen keeps its raw args) but, in `faithful_exceptions` analysis
    /// builds, also terminate `current` with a `Return` so the fall-through
    /// edge to the rest of the block / `try` join is cut.
    fn lower_return_options_barrier(&mut self, stmt: &Statement, span: Span, current: &str) {
        self.push_plain_statement(current, stmt);
        self.block_mut(current).terminator = Some(Terminator::Return {
            value: None,
            span: Some(span),
            expr: None,
            braced: false,
        });
    }

    fn lower_return_statement(&mut self, stmt: &Statement, current: &str) {
        let Statement::Return {
            span,
            value,
            expr,
            command_binding,
            braced,
        } = stmt
        else {
            unreachable!();
        };

        // Every substitution in the returned value runs before the frame
        // unwinds. Materialise its analysis effects before installing the
        // terminator.
        self.push_embedded_control_effects(
            stmt,
            current,
            "return value invokes an opaque embedded command",
        );
        if let Some(binding) = command_binding {
            self.command_binding_sites.push(CommandBindingSite {
                span: *span,
                binding: binding.clone(),
            });
        }
        self.block_mut(current).terminator = Some(Terminator::Return {
            value: value.clone(),
            span: Some(*span),
            expr: expr.clone(),
            braced: *braced,
        });
    }

    /// Materialise the variable effects of substitutions executed by a
    /// control statement before its terminator or dispatch is installed.
    fn push_embedded_control_effects(
        &mut self,
        stmt: &Statement,
        current: &str,
        opaque_reason: &str,
    ) {
        self.record_caller_frame_barrier(stmt);
        self.record_alias_observed(stmt);
        let (extras, opaque) = self.embedded_subst_extras(stmt);
        if opaque {
            self.block_mut(current).statements.push(Statement::Barrier {
                span: stmt.span(),
                reason: opaque_reason.to_owned(),
                command: "<global-frame-script>".to_owned(),
                canonical_command: None,
                args: Vec::new(),
                tokens: Some(crate::ir::CommandTokens::marker(
                    crate::ir::SyntheticMarker::GlobalFrameScript,
                )),
            });
        }
        if !extras.is_empty() {
            self.block_mut(current).statements.push(Statement::Call {
                span: stmt.span(),
                command: "<upvar-invalidate>".to_string(),
                canonical_command: None,
                args: Vec::new(),
                defs: extras,
                reads: Vec::new(),
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: Some(crate::ir::CommandTokens::marker(
                    crate::ir::SyntheticMarker::UpvarInvalidate,
                )),
                foreach_groups: None,
            });
        }
    }

    fn lower_script_statement(&mut self, stmt: &Statement, current: &str) -> Option<String> {
        self.mark_command_boundary(current, stmt);
        match stmt {
            Statement::If { .. } => Some(self.lower_if(stmt, current)),
            Statement::For { .. } => self.lower_for_or_frozen(stmt, current),
            Statement::While { .. } => Some(self.lower_while_or_frozen(stmt, current)),
            Statement::Foreach { .. } => Some(self.lower_foreach_dispatch(stmt, current)),
            Statement::Catch { .. } => {
                self.emit_opaque_catch(stmt, current);
                Some(current.to_owned())
            }
            Statement::Try { .. } => Some(self.lower_try_dispatch(stmt, current)),
            Statement::Switch { .. } => Some(self.lower_switch(stmt, current)),
            Statement::Return { .. } => {
                self.lower_return_statement(stmt, current);
                Some(current.to_owned())
            }
            // Inline block: flatten the body's statements into the current
            // control-flow stream so SSA / codegen see them as plain inline
            // statements. Preserve any registry-described Tcl error context
            // with the original command span.
            Statement::Block {
                body,
                span,
                error_context,
                ..
            } => {
                if let Some(context) = error_context {
                    self.inline_body_error_sites
                        .push(crate::cfg::InlineBodyErrorSite {
                            span: *span,
                            context: *context,
                        });
                }
                let owns_inline_region = self
                    .command_boundary_sites
                    .get(current)
                    .is_some_and(|site| site.span == *span);
                if !self.faithful_exceptions && owns_inline_region {
                    let body_block = self.new_block("inline_block_body");
                    let continuation = self.new_block("inline_block_end");
                    self.ensure_goto(current, &body_block, Some(*span));
                    self.command_boundary_continuations
                        .insert(current.to_owned(), continuation.clone());
                    if let Some(tail) = self.lower_script(body, &body_block) {
                        self.ensure_goto(&tail, &continuation, Some(*span));
                    }
                    Some(continuation)
                } else {
                    self.lower_script(body, current)
                }
            }
            // `return -options …` / `return {*}…args` lower to a
            // Statement::Barrier, but still unconditionally exit the proc
            // in analysis builds.
            Statement::Barrier { reason, span, .. }
                if self.faithful_exceptions
                    && matches!(
                        reason.as_str(),
                        "return with options" | "return with expansion"
                    ) =>
            {
                self.lower_return_options_barrier(stmt, *span, current);
                Some(current.to_owned())
            }
            other => {
                self.push_plain_statement(current, other);
                Some(current.to_owned())
            }
        }
    }

    /// Associate `block` with the explicit structured-command site retained by
    /// lowering for `stmt`. Statements without such a site are deliberately not
    /// inferred from their source text.
    fn mark_command_boundary(&mut self, block: &str, stmt: &Statement) {
        if let Some(site) = self
            .command_binding_sites
            .iter()
            .rev()
            .find(|site| site.span == stmt.span())
            .cloned()
        {
            self.command_boundary_sites.insert(block.to_owned(), site);
        }
    }

    /// Propagate an established owner to a helper block which emits the same
    /// structured command's synthetic runtime boundary.
    fn copy_command_boundary(&mut self, owner: &str, block: &str) {
        if let Some(site) = self.command_boundary_sites.get(owner).cloned() {
            self.command_boundary_sites.insert(block.to_owned(), site);
        }
    }

    fn lower_script_inner(&mut self, script: &Script, block_name: &str) -> Option<String> {
        let mut current = block_name.to_owned();
        // True once the *main* (reachable) path has hit an unconditional
        // terminator — everything after is dead code captured in orphan
        // blocks, and the script does not fall through to its caller.
        let mut main_terminated = false;

        for stmt in &script.statements {
            // If the current block is already terminated, subsequent
            // statements are dead code.  Route them into a fresh orphan
            // block with no incoming edge (rather than dropping them) so
            // SCCP marks it unreachable and O107 can flag the dead code.
            if self.block_mut(&current).terminator.is_some() {
                main_terminated = true;
                current = self.new_block("unreachable");
            }

            // `break` / `continue` inside a loop body lower to a CFG edge to
            // the loop's exit / continue target, so the loop-exit block stays
            // reachable (a `while 1 { … break }` post-loop block is live).
            if self.lower_loop_jump(&current, stmt) {
                continue;
            }
            current = self.lower_script_statement(stmt, &current)?;
        }

        // Always return the block control finally rests in, even when it is
        // terminated by a straight-line `return`/`error`.
        // `build_function` then appends a synthetic (unreachable) `exit` block
        // via `ensure_goto` (a no-op on the already-terminated block).
        // Termination is tracked separately in
        // `last_terminal_block` so `lower_try`
        // can source an on-error edge from a body that ended without an explicit
        // `error`/`throw`.  Nested control-flow lowerings still signal "no
        // continuation" by returning `None` (propagated through this loop's
        // `?` / explicit-`None` arms).
        let terminated = main_terminated || self.block_mut(&current).terminator.is_some();
        self.last_terminal_block = if terminated {
            Some(current.clone())
        } else {
            None
        };
        Some(current)
    }

    /// Dispatch `Foreach` — dict for/map, opaque top-level, or inlined.
    fn lower_foreach_dispatch(&mut self, stmt: &Statement, current: &str) -> String {
        let Statement::Foreach {
            is_dict_iteration,
            is_array_iteration,
            raw_args,
            raw_tokens,
            iterators,
            is_lmap,
            span,
            ..
        } = stmt
        else {
            unreachable!();
        };

        // `array for {k v} arr body` (Tcl 9.0): the body runs in the caller's
        // frame, so the analysis CFG inlines it (shared reaching-defs / const
        // lattice, loop vars bound) while codegen barriers it to the
        // `::tcl::array::for` ensemble invoke the array hook resolves — keeping
        // the emitted bytecode byte-identical to C Tcl.
        if *is_array_iteration {
            if self.faithful_exceptions {
                return self.lower_foreach(stmt, current);
            }
            self.block_mut(current).statements.push(Statement::Barrier {
                span: *span,
                reason: "array for".into(),
                command: "array".into(),
                canonical_command: None,
                args: raw_args.clone(),
                tokens: raw_tokens.clone(),
            });
            return current.to_owned();
        }

        if *is_dict_iteration && !raw_args.is_empty() {
            // `dict for`/`dict map {k v} $d body`: the body runs in the caller's
            // frame, so the analysis CFG inlines it (shared reaching-defs / const
            // lattice, loop vars bound, and — crucially — the body's *own* control
            // flow lowered into real blocks, so a read nested inside an `if` /
            // `while` / `catch` body is a first-class SSA use). Codegen barriers it
            // to the `::tcl::dict::for`/`::tcl::dict::map` ensemble invoke below,
            // keeping the emitted bytecode byte-identical to C Tcl. Mirrors the
            // `array for` split above. Without the inlined body, a command-name or
            // brace-nested `$var` read the shallow barrier-word scan can't see is
            // lost, so the loop's outer reads look dead (issue #833).
            if self.faithful_exceptions {
                return self.lower_foreach(stmt, current);
            }
            let sub = &raw_args[0];
            let qual_cmd = format!("::tcl::dict::{sub}");
            // The barrier re-emits `::tcl::dict::for {vars} $dict {body}` — the
            // source `dict for …` tokens with the `for`/`map` subcommand word
            // dropped — so the braced var-list / body push verbatim and the
            // `$dict` argument substitutes (else the body's command subs are
            // evaluated once, before the loop variables exist).
            let args = raw_args[1..].to_vec();
            let tokens = raw_tokens
                .as_ref()
                .map_or_else(|| all_str_tokens(&qual_cmd, &args), |t| drop_word(t, 1));
            self.block_mut(current).statements.push(Statement::Barrier {
                span: *span,
                reason: "dict for/map".into(),
                command: qual_cmd,
                canonical_command: None,
                args,
                tokens: Some(tokens),
            });
            return current.to_owned();
        }

        let has_qualified_vars = iterators
            .iter()
            .any(|it| it.vars.iter().any(|v| v.starts_with("::")));

        if (!self.inline_loops && !raw_args.is_empty()) || has_qualified_vars {
            let cmd = if *is_lmap { "lmap" } else { "foreach" };
            let loop_vars: Vec<String> = iterators.iter().flat_map(|it| it.vars.clone()).collect();
            self.block_mut(current).statements.push(Statement::Call {
                span: *span,
                command: cmd.into(),
                canonical_command: None,
                args: raw_args.clone(),
                defs: loop_vars,
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: Some(frozen_loop_tokens(cmd, raw_args, raw_tokens.as_ref())),
                foreach_groups: None,
            });
            return current.to_owned();
        }

        self.lower_foreach(stmt, current)
    }

    /// Emit an opaque `catch` call with defs for modified variables.
    fn emit_opaque_catch(&mut self, stmt: &Statement, current: &str) {
        let Statement::Catch {
            body,
            result_var,
            options_var,
            raw_args,
            span,
            tokens,
            ..
        } = stmt
        else {
            unreachable!();
        };

        let mut catch_defs = defs_from_ir_script(body);
        if let Some(rv) = result_var {
            catch_defs.push(rv.clone());
        }
        if let Some(ov) = options_var {
            catch_defs.push(ov.clone());
        }
        dedup_preserve_order(&mut catch_defs);
        // Preserve ``tokens`` on the synthetic ``Statement::Call``
        // so the codegen's eval-fallback can detect the braced
        // body word and re-wrap it in ``{…}`` when reconstructing
        // the script for ``tcl_eval``.  Without this,
        // ``catch {$undef} msg`` would lower to ``catch $undef
        // msg`` and the var-read trap would fire before catch
        // could intercept it.
        self.block_mut(current).statements.push(Statement::Call {
            span: *span,
            command: "catch".into(),
            canonical_command: None,
            args: raw_args.clone(),
            defs: catch_defs,
            reads: vec![],
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: tokens.clone(),
            foreach_groups: None,
        });
    }

    /// Dispatch `Try` — deferred opaque or inlined.
    fn lower_try_dispatch(&mut self, stmt: &Statement, current: &str) -> String {
        let Statement::Try {
            handlers,
            finally_body,
            raw_args,
            body,
            span,
            ..
        } = stmt
        else {
            unreachable!();
        };

        let defer = !self.inline_loops
            && !raw_args.is_empty()
            && (!handlers.is_empty() || finally_body.is_none());

        if defer {
            let mut try_defs = defs_from_ir_script(body);
            for handler in handlers {
                if let Some(vn) = &handler.var_name {
                    try_defs.push(vn.clone());
                }
                if let Some(ov) = &handler.options_var {
                    try_defs.push(ov.clone());
                }
                try_defs.extend(defs_from_ir_script(&handler.body));
            }
            if let Some(fb) = finally_body {
                try_defs.extend(defs_from_ir_script(fb));
            }
            dedup_preserve_order(&mut try_defs);
            self.block_mut(current).statements.push(Statement::Call {
                span: *span,
                command: "try".into(),
                canonical_command: None,
                args: raw_args.clone(),
                defs: try_defs,
                reads: vec![],
                reads_own_defs: false,
                safe_on_uninit: false,
                tokens: None,
                foreach_groups: None,
            });
            return current.to_owned();
        }

        self.lower_try(stmt, current)
    }
}

// Public API

/// Every spelling a call site may use for the procedure named `qname`.
///
/// A qualified definition is reachable by its absolute name, by its bare
/// tail, **and** by any `::`-boundary suffix in between: written from
/// `::other`, the word `demo::setdef` resolves relative to the current
/// namespace first and then falls back to `::demo::setdef`, so that
/// spelling has to key the same summary (issue #923 audit idx 59, where the
/// relative-qualified spelling silently missed the caller-frame defs while
/// the bare and absolute spellings both worked).
///
/// Registering a suffix is the same over-approximation the bare tail
/// already was: a same-named proc in a different namespace can claim the
/// key. The consequence is a *widened* `defs` list at a call site, which
/// only ever silences a warning.
#[must_use]
pub fn qualified_lookup_keys(qname: &str) -> Vec<String> {
    let mut keys = vec![qname.to_owned()];
    let mut push = |key: &str| {
        if !key.is_empty() && !keys.iter().any(|k| k == key) {
            keys.push(key.to_owned());
        }
    };
    // `::demo::setdef` → `demo::setdef` → `setdef`: drop one leading
    // namespace segment at a time, so every relative spelling of the same
    // definition keys the same summary.
    let mut rest = qname.trim_start_matches("::");
    push(rest);
    while let Some((_, tail)) = rest.split_once("::") {
        let tail = tail.trim_start_matches("::");
        push(tail);
        rest = tail;
    }
    keys
}

/// Scan a module for procedures with a caller-frame effect
/// (`upvar` / `uplevel`), returning a map from command name to its
/// [frame-effect summary][UpvarInfo].  Every spelling a call site may use
/// ([`qualified_lookup_keys`]) is registered so the lookup does not depend
/// on how the caller happened to write the name.
#[must_use]
pub fn detect_upvar_procs(module: &Module) -> HashMap<String, UpvarInfo> {
    let registry = static_context_for(module.dialect.as_deref().unwrap_or("tcl")).commands();
    detect_upvar_procs_with_registry(module, registry)
}

/// Scan caller-frame effects against the exact registry that lowered
/// `module`.
#[must_use]
pub fn detect_upvar_procs_with_registry(
    module: &Module,
    registry: &CommandRegistry,
) -> HashMap<String, UpvarInfo> {
    let command_bindings = ModuleCommandBindings::analyse(module, registry);
    detect_upvar_procs_with_bindings(module, registry, &command_bindings)
}

/// Scan caller-frame effects using the module binding summary already
/// prepared by the whole-module CFG pipeline.
fn detect_upvar_procs_with_bindings(
    module: &Module,
    registry: &CommandRegistry,
    command_bindings: &ModuleCommandBindings,
) -> HashMap<String, UpvarInfo> {
    let mut result: HashMap<String, UpvarInfo> = HashMap::new();
    // Deterministic (qualified-name) iteration order, for the same reason
    // [`prepare_cfg_context`] does it for `proc_params`: the *short* key below is
    // written by every procedure sharing that short name, so last-write-wins must
    // not be decided by the `HashMap`'s random per-process seed — this map is part
    // of the `CfgContext` folded into every procedure's `function_lattice` memo
    // key, and a nondeterministic winner makes the per-procedure cache hit or miss
    // by luck of the process start (issue #1035 follow-up).
    let mut entries: Vec<(&String, &crate::ir::Procedure)> = module.procedures.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    let mut own: Vec<(&String, &crate::ir::Procedure, UpvarInfo)> = Vec::new();
    for (qname, proc) in entries {
        let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qname);
        let namespace = if holder.is_empty() { "::" } else { holder };
        own.push((
            qname,
            proc,
            upvar_info::collect_upvar_targets_with_bindings(
                &proc.body,
                &proc.params,
                registry,
                command_bindings,
                namespace,
            ),
        ));
    }

    // One hop, no fixpoint: `uplevel <caller frame> [list callee …]` puts
    // the callee's own caller-frame effects into *this* proc's caller
    // (issue #1019). Composing against the *own-body* summaries — not the
    // composed ones — bounds the walk at a single level by construction, so
    // a recursive or mutually-recursive forward cannot diverge. A two-hop
    // chain is left to the opaque-widening path, which is the safe
    // direction.
    // `own` is already qualified-name-sorted, so a short key shared by two
    // procedures resolves to the same one every run.
    let mut by_name: HashMap<String, (&UpvarInfo, &[String])> = HashMap::new();
    for (qname, proc, info) in &own {
        for key in qualified_lookup_keys(qname) {
            by_name.insert(key, (info, proc.params.as_slice()));
        }
    }
    let mut composed: Vec<(&String, UpvarInfo)> = Vec::new();
    for (qname, _, own_info) in &own {
        let mut info = own_info.clone();
        for (callee, constructed) in &own_info.uplevel_forwarded_calls {
            if let Some((callee_info, callee_params)) = by_name.get(callee.as_str()) {
                upvar_info::compose_forwarded(callee_info, callee_params, constructed, &mut info);
            } else {
                // The forwarded command is not a procedure this unit can
                // see (a cross-file helper, a runtime-installed command).
                // It still runs in the caller's frame, so widen.
                info.caller_frame_opaque_writes = true;
                info.caller_frame_opaque_reads = true;
            }
        }
        // The second one-hop composition: an **ordinary** call to a proc that
        // reaches past its own caller lands in *this* proc's caller (issue
        // #1019 / issue #923 idx 24).  Oracle, identical on tclsh 9.0.4 and
        // 8.6.16: `proc setUp2 {var} {uplevel 2 [list set $var 99]}` /
        // `proc middle {} {setUp2 answer}` / `proc outer {} {middle; return
        // $answer}` — `outer` returns `99`, so `answer` really is assigned in
        // a frame `middle` never names.  A level-**1** effect is emphatically
        // not transitive this way (`detect_upvar_procs_does_not_propagate_
        // through_a_plain_call_wrapper` pins the tclsh transcript), which is
        // why only a `FrameReach::PastTheCaller` summary travels here.
        //
        // One hop, against the own-body summaries, for the same reason the
        // forwarded-call composition above takes exactly one: it terminates
        // by construction on any call graph, recursive or not.  A `uplevel 3`
        // two plain calls out is left to the opaque-widening path.
        for callee in &own_info.plain_calls {
            if by_name
                .get(callee.as_str())
                .is_some_and(|(callee_info, _)| {
                    callee_info.frame_reach == FrameReach::PastTheCaller
                })
            {
                info.caller_frame_opaque_writes = true;
                info.caller_frame_opaque_reads = true;
            }
        }
        // Composition input, not a caller-side effect — dropped so the
        // published summaries (which are folded into every procedure's
        // `function_lattice` memo key) carry only what a call site reads.
        info.plain_calls.clear();
        composed.push((qname, info));
    }

    for (qname, info) in composed {
        if info.is_empty() {
            continue;
        }
        for key in qualified_lookup_keys(qname) {
            if key == *qname {
                continue;
            }
            result.insert(key, info.clone());
        }
        result.insert(qname.clone(), info);
    }
    for qname in &module.redefined_procedures {
        let opaque = UpvarInfo {
            has_unresolvable_caller_target: true,
            caller_frame_opaque_writes: true,
            caller_frame_opaque_reads: true,
            frame_reach: FrameReach::PastTheCaller,
            ..UpvarInfo::default()
        };
        for key in qualified_lookup_keys(qname) {
            result.insert(key, opaque.clone());
        }
    }
    result
}

/// Module-wide CFG-determining context: the upvar-procs map, the
/// parameter-list map, the global-write-procs map, and the closed command
/// binding state
/// ([`global_write_info::detect_global_write_procs`]) [`prepare_cfg_context`]
/// returns. The single canonical definition [`crate::compilation_unit`]
/// reuses for its own `CfgContext` alias, so the two never drift apart.
pub type CfgContext = (
    HashMap<String, UpvarInfo>,
    HashMap<String, Vec<String>>,
    HashMap<String, GlobalWriteInfo>,
    ModuleCommandBindings,
);

/// Prepared module facts shared by every CFG builder in one lowering pipeline.
/// The tuple [`CfgContext`] remains the public compatibility boundary.
pub(crate) struct PreparedCfgContext {
    pub(crate) context: CfgContext,
    command_classes: CfgCommandClasses,
}

impl PreparedCfgContext {
    /// Borrow the closed binding lattice shared by downstream CFG and codegen
    /// consumers.
    pub(crate) fn command_bindings(&self) -> &ModuleCommandBindings {
        &self.context.3
    }
}

/// Return the upvar-procs map, the parameter-list map, and the
/// global-write-procs map ([`global_write_info::detect_global_write_procs`])
/// used by the CFG builder's call-site invalidation pass.  Both the
/// qualified and short forms are registered for every proc.
#[must_use]
pub fn prepare_cfg_context(module: &Module) -> CfgContext {
    let registry = static_context_for(module.dialect.as_deref().unwrap_or("tcl")).commands();
    prepare_cfg_context_with_registry(module, registry)
}

/// Prepare the module-wide CFG context against the exact registry that
/// lowered the module.
#[must_use]
pub fn prepare_cfg_context_with_registry(
    module: &Module,
    registry: &CommandRegistry,
) -> CfgContext {
    prepare_cfg_context_bundle(module, registry).context
}

#[must_use]
pub(crate) fn prepare_cfg_context_bundle(
    module: &Module,
    registry: &CommandRegistry,
) -> PreparedCfgContext {
    let command_bindings = ModuleCommandBindings::analyse(module, registry);
    let upvar_procs = detect_upvar_procs_with_bindings(module, registry, &command_bindings);
    let global_write_procs = global_write_info::detect_global_write_procs_with_bindings(
        module,
        registry,
        &command_bindings,
    );
    let mut proc_params: HashMap<String, Vec<String>> = HashMap::new();
    // Iterate procedures in a deterministic (qualified-name) order: a *short*
    // name shared by two procedures (`::a::x` and `::b::x`) is inserted by both,
    // so last-write-wins must be order-independent — otherwise `proc_params`
    // (and the `CfgContext` it feeds into the `function_lattice` memo key) would
    // depend on `HashMap` iteration order and the per-procedure lattice cache
    // would hit or miss by random seed.  Qualified names are unique, so their
    // entries are unaffected.
    let mut entries: Vec<(&String, &crate::ir::Procedure)> = module.procedures.iter().collect();
    entries.sort_by(|a, b| a.0.cmp(b.0));
    for (qname, proc) in entries {
        for key in qualified_lookup_keys(qname) {
            proc_params.insert(key, proc.params.clone());
        }
    }
    PreparedCfgContext {
        context: (
            upvar_procs,
            proc_params,
            global_write_procs,
            command_bindings,
        ),
        command_classes: CfgCommandClasses::from_registry(registry),
    }
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
    let registry = static_context_for(module.dialect.as_deref().unwrap_or("tcl")).commands();
    let config = tcl_lexer::LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(
        module.dialect.as_deref(),
    ));
    build_cfg_with_registry_and_config(module, defer_top_level, registry, config)
}

/// [`build_cfg`] with the document's exact [`tcl_lexer::LexerConfig`].
#[must_use]
pub fn build_cfg_with_config(
    module: &Module,
    defer_top_level: bool,
    config: tcl_lexer::LexerConfig,
) -> CfgModule {
    let registry = static_context_for(module.dialect.as_deref().unwrap_or("tcl")).commands();
    build_cfg_with_registry_and_config(module, defer_top_level, registry, config)
}

/// Build analysis CFGs against the exact registry that lowered `module`.
#[must_use]
pub fn build_cfg_with_registry(
    module: &Module,
    defer_top_level: bool,
    registry: &CommandRegistry,
) -> CfgModule {
    build_cfg_with_registry_and_config(
        module,
        defer_top_level,
        registry,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
    )
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
    let registry = static_context_for(module.dialect.as_deref().unwrap_or("tcl")).commands();
    let config = tcl_lexer::LexerConfig::from_grammar(tcl_dialect::grammar_of_dialect_name(
        module.dialect.as_deref(),
    ));
    build_cfg_codegen_with_registry_and_config(module, defer_top_level, registry, config)
}

/// [`build_cfg_codegen`] with the document's exact lexer configuration.
#[must_use]
pub fn build_cfg_codegen_with_config(
    module: &Module,
    defer_top_level: bool,
    config: tcl_lexer::LexerConfig,
) -> CfgModule {
    let registry = static_context_for(module.dialect.as_deref().unwrap_or("tcl")).commands();
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
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
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
pub(crate) fn build_cfg_codegen_with_registry_and_context(
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
    let (upvar_procs, proc_params, global_write_procs, command_bindings) = &context.context;

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
    let mut top_builder = new_builder(!defer_top_level)
        .with_invocation_namespace(crate::ir::ExecutionNamespace::exact(top_namespace));
    let top_cfg = top_builder.build_function("::top", &module.top_level);

    let mut proc_cfgs = HashMap::new();
    for (qname, proc) in &module.procedures {
        let mut builder = new_builder(true).with_invocation_namespace(
            crate::ir::ExecutionNamespace::exact(command_namespace(qname)),
        );
        proc_cfgs.insert(qname.clone(), builder.build_function(qname, &proc.body));
    }

    CfgModule {
        top_level: top_cfg,
        procedures: proc_cfgs,
    }
}

/// Build a CFG for a single script body against one resolved command surface.
#[must_use]
pub fn build_cfg_function(
    name: &str,
    script: &Script,
    inline_loops: bool,
    registry: &CommandRegistry,
    plain_command_dispatch: bool,
) -> Function {
    build_cfg_function_with_config(
        name,
        script,
        inline_loops,
        registry,
        plain_command_dispatch,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
    )
}

/// [`build_cfg_function`] with the exact lexer configuration used to lower
/// `script`.
#[must_use]
pub fn build_cfg_function_with_config(
    name: &str,
    script: &Script,
    inline_loops: bool,
    registry: &CommandRegistry,
    plain_command_dispatch: bool,
    config: tcl_lexer::LexerConfig,
) -> Function {
    let mut builder = CfgBuilder::new(inline_loops, registry)
        .with_lexer_config(config)
        .with_command_surface(plain_command_dispatch)
        .with_invocation_namespace(crate::ir::ExecutionNamespace::exact(command_namespace(
            name,
        )));
    builder.build_function(name, script)
}

/// Build a CFG for one body with the registry-owned module context prepared by
/// [`prepare_cfg_context`].
#[must_use]
pub fn build_cfg_function_with_upvars(
    name: &str,
    script: &Script,
    inline_loops: bool,
    registry: &CommandRegistry,
    plain_command_dispatch: bool,
    context: CfgContext,
) -> Function {
    build_cfg_function_with_upvars_and_config(
        name,
        script,
        inline_loops,
        registry,
        plain_command_dispatch,
        context,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
    )
}

/// Build a CFG for one body with both the registry-owned module context and the
/// exact dialect-aware lexer configuration used during lowering.
#[must_use]
pub fn build_cfg_function_with_upvars_and_config(
    name: &str,
    script: &Script,
    inline_loops: bool,
    registry: &CommandRegistry,
    plain_command_dispatch: bool,
    context: CfgContext,
    config: tcl_lexer::LexerConfig,
) -> Function {
    build_cfg_function_with_upvars_inner(
        name,
        script,
        crate::ir::ExecutionNamespace::exact(command_namespace(name)),
        inline_loops,
        registry,
        plain_command_dispatch,
        context,
        false,
        None,
        config,
    )
}

/// Build an ordinary body CFG using the module's already-prepared facts.
pub(crate) fn build_cfg_function_with_prepared_context(
    name: &str,
    script: &Script,
    inline_loops: bool,
    registry: &CommandRegistry,
    plain_command_dispatch: bool,
    context: &PreparedCfgContext,
    config: tcl_lexer::LexerConfig,
) -> Function {
    build_cfg_function_with_upvars_inner(
        name,
        script,
        crate::ir::ExecutionNamespace::exact(command_namespace(name)),
        inline_loops,
        registry,
        plain_command_dispatch,
        context.context.clone(),
        false,
        Some(context.command_classes.clone()),
        config,
    )
}

/// Test seam for building one method with an explicitly supplied context.
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn build_cfg_method_function_with_upvars(
    name: &str,
    script: &Script,
    execution_namespace: crate::ir::ExecutionNamespace,
    inline_loops: bool,
    registry: &CommandRegistry,
    plain_command_dispatch: bool,
    context: CfgContext,
    widen_oo_dispatch: bool,
) -> Function {
    build_cfg_function_with_upvars_inner(
        name,
        script,
        execution_namespace,
        inline_loops,
        registry,
        plain_command_dispatch,
        context,
        widen_oo_dispatch,
        None,
        tcl_lexer::LexerConfig::for_profile(registry.profile()),
    )
}

/// Build a method CFG using the module's already-prepared facts.
#[allow(clippy::too_many_arguments)]
pub(crate) fn build_cfg_method_function_with_prepared_context(
    name: &str,
    script: &Script,
    execution_namespace: crate::ir::ExecutionNamespace,
    inline_loops: bool,
    registry: &CommandRegistry,
    plain_command_dispatch: bool,
    context: &PreparedCfgContext,
    widen_oo_dispatch: bool,
    config: tcl_lexer::LexerConfig,
) -> Function {
    build_cfg_function_with_upvars_inner(
        name,
        script,
        execution_namespace,
        inline_loops,
        registry,
        plain_command_dispatch,
        context.context.clone(),
        widen_oo_dispatch,
        Some(context.command_classes.clone()),
        config,
    )
}

#[allow(clippy::too_many_arguments)]
fn build_cfg_function_with_upvars_inner(
    name: &str,
    script: &Script,
    execution_namespace: crate::ir::ExecutionNamespace,
    inline_loops: bool,
    registry: &CommandRegistry,
    plain_command_dispatch: bool,
    context: CfgContext,
    widen_oo_dispatch: bool,
    command_classes: Option<CfgCommandClasses>,
    config: tcl_lexer::LexerConfig,
) -> Function {
    let (upvar_procs, proc_params, global_write_procs, command_bindings) = context;
    let command_classes =
        command_classes.unwrap_or_else(|| CfgCommandClasses::from_registry(registry));
    let mut builder = CfgBuilder::new_with_upvars_and_classes(
        inline_loops,
        upvar_procs,
        proc_params,
        global_write_procs,
        command_bindings,
        registry,
        command_classes,
    )
    .with_faithful_exceptions()
    .with_lexer_config(config)
    .with_command_surface(plain_command_dispatch)
    .with_invocation_namespace(execution_namespace);
    if widen_oo_dispatch {
        builder = builder.with_oo_dispatch_widening();
    }
    builder.build_function(name, script)
}

fn command_namespace(qname: &str) -> String {
    let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qname);
    if holder.is_empty() {
        "::".to_owned()
    } else {
        holder.to_owned()
    }
}

/// Deduplicate a `Vec` while preserving first-occurrence order.
fn dedup_preserve_order(v: &mut Vec<String>) {
    let mut seen = std::collections::HashSet::new();
    v.retain(|item| seen.insert(item.clone()));
}

// Registry-derived command classifications
//
// These are facts of one resolved release/dialect registry, not a
// process-global union. In particular Tcl 8.4 has no builtin `throw`; a
// procedure with that name is an ordinary fall-through call in that profile.
// The owning [`CfgBuilder`] carries the classes derived for its module and
// passes them through the recursive flow-fact helpers below.

/// Name sets for the command classifications the CFG builder keys
/// control-flow shape on, each derived from one registry trait.
#[derive(Clone)]
struct CfgCommandClasses {
    /// The registry's one effective-spec index, shared with command-binding
    /// analysis. Classification stays trait-driven without rebuilding five
    /// complete name sets for every compilation unit.
    semantics: Arc<EffectiveRegistrySemantics>,
}

impl CfgCommandClasses {
    fn from_registry(registry: &CommandRegistry) -> Self {
        Self {
            semantics: registry.effective_semantics(),
        }
    }

    #[cfg(test)]
    fn for_dialect(dialect: Option<&str>) -> Self {
        Self::from_registry(static_context_for(dialect.unwrap_or("tcl")).commands())
    }

    fn is_block_terminating_command(&self, command: &str) -> bool {
        self.semantics
            .command(command.trim_start_matches(':'))
            .is_some_and(|facts| {
                facts.has_traits(Traits::TERMINATES_BLOCK)
                    && facts.lowering_hook() != Some(LoweringHookId::Return)
            })
    }

    fn is_tailcall_command(&self, command: &str) -> bool {
        self.semantics
            .command(command.trim_start_matches(':'))
            .is_some_and(|facts| facts.has_traits(Traits::REPLACES_FRAME))
    }

    fn is_loop_break_command(&self, name: &str) -> bool {
        self.semantics
            .command(name)
            .is_some_and(|facts| facts.has_traits(Traits::BREAKS_LOOP))
    }

    fn is_loop_continue_command(&self, name: &str) -> bool {
        self.semantics
            .command(name)
            .is_some_and(|facts| facts.has_traits(Traits::CONTINUES_LOOP))
    }

    fn is_catchable_throw(&self, command: &str) -> bool {
        self.semantics
            .command(command.trim_start_matches(':'))
            .is_some_and(|facts| facts.has_traits(Traits::CATCHABLE_THROW))
    }
}

impl Default for CfgCommandClasses {
    fn default() -> Self {
        Self::from_registry(&CommandRegistry::build_default())
    }
}

// Definite-assignment ("flow facts") over un-lowered IR scripts
//
// Shared by the CFG builder (to promote an opaque `switch` whose every arm
// exits the *procedure* — FP-RBS-15 — and to wire its loop-jump edges) and
// by SSA (to recover the def set an opaque switch *definitely* establishes —
// FP-RBS-14).  Both consumers agree on one model.
//
// Each statement/script is classified by how it leaves: it can complete NORMALly
// (fall through to the next statement), jump within an enclosing loop (LoopJump
// — `break`/`continue`), or exit the procedure (ProcExit —
// `return`/`error`/`throw`/`exit`/`tailcall`).  The distinction matters for an
// *opaque* switch (whose arm bodies are not lowered into the CFG, so their
// loop-jump edges are invisible): a ProcExit arm reaches no later code at all,
// but a LoopJump arm still reaches the code after the enclosing loop *without*
// the other arms' definitions — so it must NOT be treated as vacuous when
// recovering the switch's defs, and an all-LoopJump switch is NOT a procedure
// terminator.

/// How a statement/script leaves: fall through, jump to an enclosing loop, or
/// exit the procedure.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(crate) enum Completion {
    /// Falls through to the next statement.
    Normal,
    /// `break` / `continue` — leaves to an enclosing-loop target (still reaches
    /// the code after that loop).
    LoopJump,
    /// `return` / `error` / `throw` / `exit` / `tailcall` — leaves the proc.
    ProcExit,
}

/// `(must-defines, completion)` for a single statement.
///
/// * assignments (`set`/`incr`/`expr`-assign) contribute their target and
///   complete normally; a plain `Call` contributes its synthetic `defs` and
///   completes normally — *except* `error`/`throw`/`exit` and `tailcall`, which
///   `ProcExit`, and `break`/`continue`, which `LoopJump`; `Return` and a
///   `return -code` / expansion `Barrier` `ProcExit`;
/// * `Block` / `UpFrame` bodies always run, so recurse;
/// * an `If` *with* an else, or a `Switch` *with* a default, combine their
///   branches via [`intersect_completing`]; an else-less `If` / default-less
///   `Switch` has a fall-through path, so it completes normally assigning
///   nothing for certain;
/// * loops (`For`/`While`/`Foreach`), `Catch`/`Try` and everything else may not
///   execute / always complete (`catch` even swallows `break`), so they
///   contribute no must-define and complete normally.
fn flow_facts_stmt_with_classes(
    stmt: &Statement,
    command_classes: &CfgCommandClasses,
) -> (BTreeSet<String>, Completion) {
    match stmt {
        Statement::AssignConst { name, .. }
        | Statement::AssignExpr { name, .. }
        | Statement::AssignValue { name, .. }
        | Statement::Incr { name, .. } => {
            let mut set = BTreeSet::new();
            let n = normalise_var_name(name);
            if !n.is_empty() {
                set.insert(n.to_owned());
            }
            (set, Completion::Normal)
        }
        Statement::Return { .. } => (BTreeSet::new(), Completion::ProcExit),
        Statement::Barrier {
            reason,
            command,
            canonical_command,
            ..
        } => {
            // A `return -options …` / `return {*}…` barrier unconditionally
            // exits the proc.  An opaque callback barrier still retains its
            // host command identity; preserve registry-declared unconditional
            // control transfer such as `tailcall` rather than treating the
            // callback's unknown effects as evidence that the host falls
            // through.
            let canon = canonical_command.as_deref().unwrap_or(command);
            if matches!(
                reason.as_str(),
                "return with options" | "return with expansion"
            ) || command_classes.is_block_terminating_command(canon)
                || command_classes.is_tailcall_command(canon)
            {
                (BTreeSet::new(), Completion::ProcExit)
            } else {
                (BTreeSet::new(), Completion::Normal)
            }
        }
        Statement::Call {
            command,
            canonical_command,
            defs,
            ..
        } => {
            let canon = canonical_command.as_deref().unwrap_or(command);
            let bare = canon.trim_start_matches(':');
            let completion = if command_classes.is_loop_break_command(bare)
                || command_classes.is_loop_continue_command(bare)
            {
                // A loop jump leaves to the enclosing loop's target — it still
                // reaches the code after that loop, just without later defs.
                Completion::LoopJump
            } else if command_classes.is_block_terminating_command(canon)
                || command_classes.is_tailcall_command(canon)
            {
                Completion::ProcExit
            } else {
                Completion::Normal
            };
            (defs.iter().cloned().collect(), completion)
        }
        Statement::Block { body, .. } | Statement::UpFrame { body, .. } => {
            flow_facts_script_with_classes(body, command_classes)
        }
        Statement::If {
            clauses,
            else_body: Some(eb),
            ..
        } => {
            let mut bodies: Vec<&Script> = clauses.iter().map(|c| &c.body).collect();
            bodies.push(eb);
            intersect_completing(&bodies, command_classes)
        }
        Statement::Switch {
            arms,
            default_body: Some(default),
            ..
        } => {
            let mut bodies: Vec<&Script> = vec![default];
            bodies.extend(arms.iter().filter_map(|arm| arm.body.as_ref()));
            intersect_completing(&bodies, command_classes)
        }
        // An else-less `If` / default-less `Switch` has a fall-through path that
        // assigns nothing for certain; everything else may not run / always
        // completes.
        _ => (BTreeSet::new(), Completion::Normal),
    }
}

/// Profile-less compatibility entry point. Module CFG construction uses the
/// release-aware `*_with_classes` path owned by its [`CfgBuilder`].
#[cfg(test)]
pub(crate) fn flow_facts_stmt(stmt: &Statement) -> (BTreeSet<String>, Completion) {
    flow_facts_stmt_with_classes(stmt, &CfgCommandClasses::default())
}

/// `(vars definitely assigned, completion)` for an un-lowered IR script.
///
/// Walk statements in order accumulating must-defines; the first statement that
/// does not complete normally makes the rest dead and gives the script that
/// statement's completion (`ProcExit` or `LoopJump`), with the must-defines
/// being those accumulated up to (and including) it.  Only names guaranteed to
/// be assigned are returned, so it never over-claims (which would hide a real
/// read-before-set).
fn flow_facts_script_with_classes(
    script: &Script,
    command_classes: &CfgCommandClasses,
) -> (BTreeSet<String>, Completion) {
    let mut assigned = BTreeSet::new();
    for stmt in &script.statements {
        let (defs, completion) = flow_facts_stmt_with_classes(stmt, command_classes);
        assigned.extend(defs);
        if completion != Completion::Normal {
            return (assigned, completion);
        }
    }
    (assigned, Completion::Normal)
}

/// Combine branch bodies into `(must-defines, completion)`.
///
/// A `ProcExit` branch reaches no code after the construct, so it is vacuous
/// (⊤) and excluded from the must-define intersection.  A `Normal` or
/// `LoopJump` branch *does* reach later code (the fall-through successor, or —
/// for a loop jump — the code after the enclosing loop), so its must-defines
/// (those established before it leaves) ARE intersected: this keeps the result
/// sound when a `break`/`continue` arm escapes an opaque switch without the
/// other arms' defs.  The combined completion is `Normal` if any branch falls
/// through, else `LoopJump` if any jumps, else `ProcExit` (every branch exits).
fn intersect_completing(
    bodies: &[&Script],
    command_classes: &CfgCommandClasses,
) -> (BTreeSet<String>, Completion) {
    let mut common: Option<BTreeSet<String>> = None;
    let mut any_normal = false;
    let mut any_loop_jump = false;
    for body in bodies {
        let (assigned, completion) = flow_facts_script_with_classes(body, command_classes);
        match completion {
            Completion::ProcExit => continue,
            Completion::Normal => any_normal = true,
            Completion::LoopJump => any_loop_jump = true,
        }
        common = Some(match common {
            None => assigned,
            Some(acc) => acc.intersection(&assigned).cloned().collect(),
        });
    }
    let combined = if any_normal {
        Completion::Normal
    } else if any_loop_jump {
        Completion::LoopJump
    } else {
        Completion::ProcExit
    };
    (common.unwrap_or_default(), combined)
}

/// Variables an opaque `switch` assigns on *every* non-proc-exit path.
///
/// Empty unless the switch has a `default` arm (otherwise an unmatched subject
/// falls through assigning nothing).  Intersection over the default and every
/// arm-with-a-body, excluding only arms that exit the procedure (which reach no
/// later code).  A `break`/`continue` arm is *kept* (with the defs it makes
/// before jumping), because it still reaches the code after the enclosing loop —
/// so an arm that breaks without assigning `y` correctly drops `y` rather than
/// letting it be claimed defined.
pub(crate) fn switch_must_defines(
    stmt: &Statement,
    registry: Option<&CommandRegistry>,
) -> BTreeSet<String> {
    let command_classes = registry.map_or_else(CfgCommandClasses::default, |registry| {
        CfgCommandClasses::from_registry(registry)
    });
    flow_facts_stmt_with_classes(stmt, &command_classes).0
}

/// `(can_break, can_continue)` for `break`/`continue` that escape *script* to an
/// enclosing loop.
///
/// Recurses into `if`/`switch`/`Block`/`UpFrame` bodies (which don't capture a
/// loop jump) and into `try` — a `try` does NOT capture `break`/`continue`:
/// without a matching `on`/`trap` handler a jump in the body, a handler body, or
/// the `finally` body propagates to the enclosing loop (confirmed in tclsh
/// 9.0.3, contrast `catch {break}` which yields code 3 and is absorbed).  Does
/// NOT recurse into nested loops (`for`/`while`/`foreach`) or `catch`, which
/// capture their own jumps.  Scanning stops at the first statement that cannot
/// complete normally (`return`/`error`/`exit`/`tailcall`, or `break`/`continue`
/// itself): a jump after it is dead code that never executes, so it must not
/// create a spurious loop-exit edge.
/// `depth` is the nesting level of `script` — reuses [`MAX_LOWER_DEPTH`]
/// (this walk is mutually recursive with [`switch_escaping_jumps`], and
/// both are transitively bounded today via the same cap `lower_script`
/// already builds every `Script` under, so this is defence-in-depth /
/// consistency with that cap rather than a currently-reachable path).
fn escaping_loop_jumps_with_classes(
    script: &Script,
    depth: u32,
    command_classes: &CfgCommandClasses,
) -> (bool, bool) {
    if MAX_LOWER_DEPTH.exceeded(depth) {
        return (false, false);
    }
    let mut can_break = false;
    let mut can_continue = false;
    for stmt in &script.statements {
        match stmt {
            Statement::Call {
                command,
                canonical_command,
                ..
            } => {
                let bare = canonical_command
                    .as_deref()
                    .unwrap_or(command)
                    .trim_start_matches(':');
                if command_classes.is_loop_break_command(bare) {
                    can_break = true;
                } else if command_classes.is_loop_continue_command(bare) {
                    can_continue = true;
                }
            }
            Statement::If {
                clauses, else_body, ..
            } => {
                for clause in clauses {
                    let (b, c) =
                        escaping_loop_jumps_with_classes(&clause.body, depth + 1, command_classes);
                    can_break |= b;
                    can_continue |= c;
                }
                if let Some(eb) = else_body {
                    let (b, c) = escaping_loop_jumps_with_classes(eb, depth + 1, command_classes);
                    can_break |= b;
                    can_continue |= c;
                }
            }
            Statement::Switch { .. } => {
                let (b, c) = switch_escaping_jumps_with_classes(stmt, depth, command_classes);
                can_break |= b;
                can_continue |= c;
            }
            Statement::Block { body, .. } | Statement::UpFrame { body, .. } => {
                let (b, c) = escaping_loop_jumps_with_classes(body, depth + 1, command_classes);
                can_break |= b;
                can_continue |= c;
            }
            Statement::Try {
                body,
                handlers,
                finally_body,
                ..
            } => {
                let (b, c) = escaping_loop_jumps_with_classes(body, depth + 1, command_classes);
                can_break |= b;
                can_continue |= c;
                for h in handlers {
                    let (b, c) =
                        escaping_loop_jumps_with_classes(&h.body, depth + 1, command_classes);
                    can_break |= b;
                    can_continue |= c;
                }
                if let Some(fb) = finally_body {
                    let (b, c) = escaping_loop_jumps_with_classes(fb, depth + 1, command_classes);
                    can_break |= b;
                    can_continue |= c;
                }
            }
            _ => {}
        }
        // A statement that cannot complete normally makes everything after it
        // dead code (a later `break`/`continue` never runs), so stop scanning —
        // otherwise a dead `break` after `error`/`return` would forge a
        // loop-exit edge and fire W210 on unreachable post-loop code.
        if flow_facts_stmt_with_classes(stmt, command_classes).1 != Completion::Normal {
            break;
        }
    }
    (can_break, can_continue)
}

/// Profile-less compatibility entry point. Module CFG construction uses the
/// release-aware `*_with_classes` path owned by its [`CfgBuilder`].
#[cfg(test)]
pub(crate) fn escaping_loop_jumps(script: &Script, depth: u32) -> (bool, bool) {
    escaping_loop_jumps_with_classes(script, depth, &CfgCommandClasses::default())
}

/// `(can_break, can_continue)` over all bodies of an opaque `switch`.
/// `depth` is `stmt`'s own nesting level — see
/// [`escaping_loop_jumps_with_classes`].
fn switch_escaping_jumps_with_classes(
    stmt: &Statement,
    depth: u32,
    command_classes: &CfgCommandClasses,
) -> (bool, bool) {
    let Statement::Switch {
        arms, default_body, ..
    } = stmt
    else {
        return (false, false);
    };
    let mut can_break = false;
    let mut can_continue = false;
    if let Some(default) = default_body {
        let (b, c) = escaping_loop_jumps_with_classes(default, depth + 1, command_classes);
        can_break |= b;
        can_continue |= c;
    }
    for arm in arms {
        if let Some(body) = &arm.body {
            let (b, c) = escaping_loop_jumps_with_classes(body, depth + 1, command_classes);
            can_break |= b;
            can_continue |= c;
        }
    }
    (can_break, can_continue)
}

// Guaranteed-iteration loops
//
// A loop whose body provably runs at least once does not skip its body, so a
// variable the body assigns is defined when code after the loop reads it.  The
// usual CFG models every loop as possibly running zero times (the header's exit
// edge), false-firing W210 on such a read.  In analysis builds we *rotate* a
// provably-non-empty loop so the 0-iteration skip becomes a *separate*
// entry-guard edge whose condition is statically true; SCCP marks that edge dead
// and the FP-RBS-16 dead-edge phi filter then ignores the version-0 operand it
// carried — with no synthetic def, so SCCP values are untouched.  `break` /
// `continue` stay real CFG edges, so partial-def exits remain sound.

/// True when `text` is a static, non-empty Tcl list literal.
///
/// Conservative: any `$` / `[` (a possible substitution) disqualifies it, so a
/// runtime-computed list is never claimed non-empty.  `foreach` stores a braced
/// list with its outer braces stripped (`{1 2 3}` → `1 2 3`), so a literal
/// splits directly.
fn list_literal_nonempty(text: &str, rules: tcl_syntax::word_rules::WordValueRules) -> bool {
    if text.is_empty() || text.contains('$') || text.contains('[') {
        return false;
    }
    !crate::tcl_expr_eval::split_tcl_list(text, rules).is_empty()
}

/// True when a `foreach`/`lmap` provably iterates ≥1 time.
///
/// `foreach` runs `max` over its iterator groups, so *any* non-empty iterator
/// list guarantees at least one iteration (shorter lists just pad their loop
/// vars with `""`).
pub(crate) fn foreach_runs_at_least_once(
    stmt: &Statement,
    rules: tcl_syntax::word_rules::WordValueRules,
) -> bool {
    let Statement::Foreach { iterators, .. } = stmt else {
        return false;
    };
    iterators
        .iter()
        .any(|it| list_literal_nonempty(&it.list_arg, rules))
}

impl CfgBuilder<'_> {
    /// Names a non-`AssignConst` init statement may write.  `None` means "can't
    /// tell" — the caller then drops *all* constant bindings (a write it can't
    /// characterise might clobber any of them).
    ///
    /// A `Call`'s write set includes the caller-side variables an `upvar`-using
    /// callee modifies (direct call or `[upvar_proc …]` embedded substitution),
    /// recovered via [`Self::apply_upvar_invalidation`] — the module upvar pass
    /// adds these to a call's `defs`, but they are absent from the raw IR the
    /// init clause carries.  Without this, `for {set i 0; setter} {$i < 3} …`
    /// where `setter` runs `upvar 1 i i; set i 5` would keep the stale
    /// `i = 0` binding and be wrongly judged guaranteed.
    fn init_written_names(&self, stmt: &Statement) -> Option<Vec<String>> {
        let mut names: Vec<String> = match stmt {
            Statement::AssignValue { name, .. }
            | Statement::AssignExpr { name, .. }
            | Statement::Incr { name, .. } => {
                let n = normalise_var_name(name);
                if n.is_empty() {
                    Vec::new()
                } else {
                    vec![n.to_owned()]
                }
            }
            // A plain call contributes only the (augmented) defs collected
            // below.
            Statement::Call { .. } => Vec::new(),
            // Any other statement shape in the init (a nested if/loop/…) is not
            // characterised here — clear every constant binding to stay sound.
            _ => return None,
        };
        for s in self.upvar_invalidated(stmt.clone()) {
            // A widening barrier means the callee can write ANY caller
            // variable (an `upvar` with a dynamic caller-side name) — no
            // per-name set exists, so report "can't tell" and let the
            // caller drop every constant binding.
            if matches!(s, Statement::Barrier { .. }) {
                return None;
            }
            if let Statement::Call { defs, .. } = &s {
                for d in defs {
                    if !names.contains(d) {
                        names.push(d.clone());
                    }
                }
            }
        }
        Some(names)
    }

    /// True when a `for` loop's condition is statically true on entry.
    ///
    /// Evaluates the condition against the constant bindings the init clause
    /// establishes (`for {set i 0} {$i < 3} …` → `i = 0` → `0 < 3` is true),
    /// which proves the first iteration always runs.  Init statements are
    /// processed in order: an `AssignConst` (re)binds a constant, but any
    /// *other* write *invalidates* that variable's binding — so
    /// `for {set i 0; set i $n} …` and `for {set i 0; incr i 5} …` leave `i`
    /// unknown rather than stale-constant `0`, and an `upvar`-writing call in
    /// the init invalidates the var it writes through the caller frame.
    /// Conservative: a condition referencing an unbound variable (or
    /// a command-substitution condition) evaluates to `None` → not guaranteed.
    pub(crate) fn for_runs_at_least_once(&self, stmt: &Statement) -> bool {
        use crate::tcl_expr_eval::{TclValue, eval_tcl_expr};
        let Statement::For {
            init, condition, ..
        } = stmt
        else {
            return false;
        };
        let mut env: crate::tcl_expr_eval::Env = std::collections::HashMap::new();
        for s in &init.statements {
            if let Statement::AssignConst { name, value, .. } = s {
                let n = normalise_var_name(name);
                if !n.is_empty() {
                    env.insert(n.to_owned(), coerce_scalar(value));
                }
                continue;
            }
            match self.init_written_names(s) {
                // Unknown write — every prior constant binding is now suspect.
                None => env.clear(),
                Some(names) => {
                    for n in names {
                        env.remove(&n);
                    }
                }
            }
        }
        match eval_tcl_expr(condition, &env) {
            Some(TclValue::Int(i)) => i != 0,
            Some(TclValue::Float(f)) => f != 0.0,
            // A bignum is canonical (beyond i64), hence never zero.
            Some(TclValue::Big(_)) => true,
            None => false,
        }
    }
}

/// Coerce a literal assignment value to int/float when possible (else str),
/// for the `for`-init constant environment.
fn coerce_scalar(text: &str) -> crate::tcl_expr_eval::EnvValue {
    use crate::tcl_expr_eval::EnvValue;
    if let Ok(i) = text.parse::<i64>() {
        return EnvValue::Int(i);
    }
    if let Ok(f) = text.parse::<f64>() {
        return EnvValue::Float(f);
    }
    EnvValue::Str(text.to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{ForeachIterator, IfClause, Script, SwitchArm, SwitchMode};
    use tcl_lexer::Span;

    fn build_test_cfg_function(name: &str, script: &Script, inline_loops: bool) -> Function {
        build_cfg_function(
            name,
            script,
            inline_loops,
            &CommandRegistry::build_default(),
            false,
        )
    }

    /// The registry-derived commands which leave a procedure when they appear
    /// as ordinary calls. Keep test data tied to traits rather than duplicating
    /// command names alongside the production lookup.
    fn proc_exit_host_commands() -> Vec<String> {
        let classes = CfgCommandClasses::for_dialect(Some("tcl9.0"));
        let mut commands: Vec<_> = classes
            .semantics
            .command_names_with_traits(Traits::TERMINATES_BLOCK)
            .filter(|name| {
                classes
                    .semantics
                    .command(name)
                    .is_some_and(|facts| facts.lowering_hook() != Some(LoweringHookId::Return))
            })
            .chain(
                classes
                    .semantics
                    .command_names_with_traits(Traits::REPLACES_FRAME),
            )
            .map(str::to_owned)
            .collect();
        commands.sort_unstable();
        commands.dedup();
        commands
    }

    fn host_call(command: String) -> Statement {
        Statement::Call {
            span: Span::new(0, 1),
            command,
            canonical_command: None,
            args: Vec::new(),
            defs: Vec::new(),
            reads: Vec::new(),
            reads_own_defs: false,
            safe_on_uninit: false,
            tokens: None,
            foreach_groups: None,
        }
    }

    fn host_barrier(command: String) -> Statement {
        Statement::Barrier {
            span: Span::new(0, 1),
            reason: "opaque host command".into(),
            command,
            canonical_command: None,
            args: Vec::new(),
            tokens: None,
        }
    }

    fn build_analysis_cfg(name: &str, script: &Script) -> Function {
        CfgBuilder::new(true, tcl_registry::default_registry())
            .with_faithful_exceptions()
            .build_function(name, script)
    }

    fn proc_exit_barrier_switch(mode: SwitchMode) -> Script {
        let commands = proc_exit_host_commands();
        let arms = commands
            .iter()
            .enumerate()
            .map(|(i, command)| SwitchArm {
                pattern: i.to_string(),
                pattern_braced: true,
                pattern_span: Span::new(0, 1),
                // Exercise absolute spellings inside both flattened and opaque
                // switch paths; classification must normalise the `::` prefix.
                body: Some(Script::from_statements(vec![host_barrier(format!(
                    "::{command}"
                ))])),
                body_span: Some(Span::new(0, 1)),
                fallthrough: false,
            })
            .collect();
        Script::from_statements(vec![Statement::Switch {
            subject_braced: false,
            raw_arg_braced: Vec::new(),
            span: Span::new(0, 1),
            subject: "$which".into(),
            subject_span: Span::new(0, 1),
            arms,
            default_body: Some(Script::from_statements(vec![host_barrier(format!(
                "::{}",
                commands[0]
            ))])),
            default_span: Some(Span::new(0, 1)),
            mode,
            nocase: false,
            raw_args: Vec::new(),
            patterns_braced: true,
        }])
    }

    /// Drift guard: the registry-derived classification sets must equal
    /// the name lists this file used to hardcode, so a future trait
    /// stamping change is a conscious CFG-shape decision rather than a
    /// silent one.
    #[test]
    fn registry_derived_cfg_classes_match_previous_hardcodes() {
        fn sorted(classes: &CfgCommandClasses, traits: Traits) -> Vec<&str> {
            let mut v: Vec<&str> = classes
                .semantics
                .command_names_with_traits(traits)
                .collect();
            v.sort_unstable();
            v
        }
        let classes = CfgCommandClasses::for_dialect(Some("tcl9.0"));
        // `return` carries TERMINATES_BLOCK but lowers to
        // `Statement::Return`, so the name-keyed terminator set excludes it.
        let mut terminates = sorted(&classes, Traits::TERMINATES_BLOCK);
        terminates.retain(|name| {
            classes
                .semantics
                .command(name)
                .is_some_and(|facts| facts.lowering_hook() != Some(LoweringHookId::Return))
        });
        assert_eq!(terminates, ["error", "exit", "throw"]);
        assert_eq!(
            sorted(&classes, Traits::CATCHABLE_THROW),
            ["error", "throw"]
        );
        assert_eq!(sorted(&classes, Traits::REPLACES_FRAME), ["tailcall"]);
        assert_eq!(sorted(&classes, Traits::BREAKS_LOOP), ["break"]);
        assert_eq!(sorted(&classes, Traits::CONTINUES_LOOP), ["continue"]);

        let tcl84 = CfgCommandClasses::for_dialect(Some("tcl8.4"));
        assert!(
            !tcl84.is_block_terminating_command("throw") && !tcl84.is_catchable_throw("throw"),
            "Tcl 8.4 must select its visible spec set, not inherit later `throw` traits"
        );
    }

    #[test]
    fn cfg_and_binding_analysis_share_the_registry_semantic_index() {
        let registry = CommandRegistry::build_default();
        let module = crate::lowering::lower_to_ir("set value 1", &registry);
        let bindings = ModuleCommandBindings::analyse(&module, &registry);
        let classes = CfgCommandClasses::from_registry(&registry);

        assert!(Arc::ptr_eq(
            bindings.effective_semantics(),
            &classes.semantics
        ));
    }

    #[test]
    fn whole_module_cfg_uses_the_exact_custom_registry_traits() {
        let mut registry = CommandRegistry::build_default();
        let mut custom = registry.get("puts").expect("puts spec").clone();
        custom.traits |= Traits::TERMINATES_BLOCK;
        registry.insert(custom);

        let module = crate::lowering::lower_to_ir("puts before\nset reached 1", &registry);
        let cfg = build_cfg_with_registry(&module, false, &registry);
        let entry = &cfg.top_level.blocks[&cfg.top_level.entry];
        assert!(
            matches!(entry.terminator, Some(Terminator::Return { .. })),
            "the custom registry's terminating command must shape the whole-module CFG: {entry:?}"
        );
        assert!(
            cfg.top_level.blocks.values().any(|block| {
                block.name.starts_with("unreachable")
                    && block.statements.iter().any(|statement| {
                        matches!(statement, Statement::AssignConst { name, .. } if name == "reached")
                    })
            }),
            "the statement after the custom terminator must be unreachable"
        );
    }

    #[test]
    fn condition_var_writes_use_the_exact_custom_registry_roles() {
        let mut registry = CommandRegistry::build_default();
        let mut custom = registry.get("list").expect("list spec").clone();
        custom.arg_roles = &[(0, tcl_registry::ArgRole::VarWrite)];
        custom.arg_role_resolver = None;
        registry.insert(custom);

        let module = crate::lowering::lower_to_ir("if {[list slot]} {puts yes}", &registry);
        let cfg = build_cfg_with_registry(&module, false, &registry);
        let condition_defs = cfg
            .top_level
            .blocks
            .values()
            .flat_map(|block| &block.statements)
            .find_map(|statement| match statement {
                Statement::Call { command, defs, .. } if command == "<cond>" => Some(defs),
                _ => None,
            })
            .expect("condition command substitutions produce a synthetic CFG call");
        assert!(
            condition_defs.iter().any(|name| name == "slot"),
            "custom VarWrite role was replaced by a default Tcl registry: {condition_defs:?}"
        );
    }

    /// The classification helpers keep each site's historical name
    /// normalisation: the terminator / throw / tailcall checks trim
    /// leading `:` runs (so canonical `::error` classifies), while
    /// [`CfgBuilder::lower_loop_jump`] matches the raw word (so
    /// `::break` stays a plain call there).
    #[test]
    fn cfg_class_helpers_keep_site_normalisation() {
        let classes = CfgCommandClasses::for_dialect(Some("tcl9.0"));
        assert!(classes.is_block_terminating_command("error"));
        assert!(classes.is_block_terminating_command("::throw"));
        assert!(classes.is_block_terminating_command("exit"));
        assert!(!classes.is_block_terminating_command("return"));
        assert!(!classes.is_block_terminating_command("break"));
        assert!(classes.is_catchable_throw("::error"));
        assert!(!classes.is_catchable_throw("exit"));
        assert!(classes.is_tailcall_command("::tailcall"));
        assert!(!classes.is_tailcall_command("error"));
        assert!(classes.is_loop_break_command("break"));
        assert!(!classes.is_loop_break_command("::break"));
        assert!(classes.is_loop_continue_command("continue"));
        assert!(!classes.is_loop_continue_command("break"));
    }

    #[test]
    fn opaque_host_barriers_match_calls_for_every_registry_proc_exit_command() {
        // This is deliberately registry-driven: today the traits yield
        // `error`, `exit`, `throw`, and `tailcall`, but a trait change must
        // extend this test automatically. Test both source spellings the CFG
        // accepts, including the global-qualified form.
        for command in proc_exit_host_commands() {
            for spelling in [command.clone(), format!("::{command}")] {
                for (kind, statement) in [
                    ("Call", host_call(spelling.clone())),
                    ("Barrier", host_barrier(spelling.clone())),
                ] {
                    assert_eq!(
                        flow_facts_stmt(&statement).1,
                        Completion::ProcExit,
                        "{kind} {spelling} must leave the procedure",
                    );

                    let script = Script::from_statements(vec![
                        statement,
                        Statement::AssignConst {
                            span: Span::new(2, 3),
                            name: "dead".into(),
                            name_braced: false,
                            value: "1".into(),
                            value_span: None,
                        },
                    ]);
                    let func = build_analysis_cfg("::test", &script);
                    let entry = &func.blocks[&func.entry];
                    assert_eq!(
                        entry.statements.len(),
                        1,
                        "{kind} {spelling} must keep following statements unreachable",
                    );
                    assert!(
                        matches!(entry.terminator, Some(Terminator::Return { .. })),
                        "{kind} {spelling} must terminate its CFG block, got {:?}",
                        entry.terminator,
                    );
                }
            }
        }
    }

    #[test]
    fn proc_exit_host_barriers_terminate_direct_and_opaque_switch_forms() {
        // Exact switches lower each arm directly; glob switches remain an
        // opaque Statement::Switch. In both representations, barriers whose
        // host command is a registry-described proc exit must retain the same
        // completion fact as a Call.
        let commands = proc_exit_host_commands();
        let direct = build_analysis_cfg("::direct", &proc_exit_barrier_switch(SwitchMode::Exact));
        assert!(matches!(
            direct.blocks[&direct.entry].terminator,
            Some(Terminator::Branch { .. })
        ));
        let direct_barriers: Vec<_> = direct
            .blocks
            .values()
            .filter(|block| {
                block
                    .statements
                    .iter()
                    .any(|stmt| matches!(stmt, Statement::Barrier { .. }))
            })
            .collect();
        assert_eq!(direct_barriers.len(), commands.len() + 1);
        assert!(
            direct_barriers
                .iter()
                .all(|block| { matches!(block.terminator, Some(Terminator::Return { .. })) })
        );

        let opaque = build_analysis_cfg("::opaque", &proc_exit_barrier_switch(SwitchMode::Glob));
        let entry = &opaque.blocks[&opaque.entry];
        assert!(matches!(
            entry.statements.as_slice(),
            [Statement::Switch { .. }]
        ));
        assert!(
            matches!(entry.terminator, Some(Terminator::Return { .. })),
            "an all-proc-exit opaque switch must not fall through, got {:?}",
            entry.terminator,
        );
    }

    #[test]
    fn empty_script_produces_entry_exit() {
        let func = build_test_cfg_function("::test", &Script::new(), true);
        assert_eq!(func.name, "::test");
        assert!(func.blocks.contains_key(&func.entry));
        // entry → exit
        assert!(func.blocks.len() >= 2);
    }

    #[test]
    fn linear_script() {
        let script = Script::from_statements(vec![
            Statement::AssignConst {
                span: Span::new(0, 7),
                name: "x".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            },
            Statement::AssignConst {
                span: Span::new(8, 15),
                name: "y".into(),
                name_braced: false,
                value: "2".into(),
                value_span: None,
            },
        ]);
        let func = build_test_cfg_function("::test", &script, true);
        // Entry block should have both statements.
        let entry = &func.blocks[&func.entry];
        assert_eq!(entry.statements.len(), 2);
    }

    #[test]
    fn return_terminates() {
        let script = Script::from_statements(vec![
            Statement::AssignConst {
                span: Span::new(0, 7),
                name: "x".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            },
            Statement::Return {
                span: Span::new(8, 16),
                value: Some("$x".into()),
                expr: None,
                command_binding: None,
                braced: false,
            },
            Statement::AssignConst {
                span: Span::new(17, 24),
                name: "y".into(),
                name_braced: false,
                value: "2".into(), // dead code
                value_span: None,
            },
        ]);
        let func = build_test_cfg_function("::test", &script, true);
        let entry = &func.blocks[&func.entry];
        // Only one statement before the return terminator.
        assert_eq!(entry.statements.len(), 1);
        assert!(matches!(entry.terminator, Some(Terminator::Return { .. })));
    }

    #[test]
    fn if_creates_branches() {
        let script = Script::from_statements(vec![Statement::If {
            span: Span::new(0, 30),
            clauses: vec![IfClause {
                condition: ExprNode::Var {
                    text: "$x".into(),
                    name: "x".into(),
                    start: 0,
                    end: 2,
                },
                condition_span: Span::new(3, 5),
                body: Script::from_statements(vec![Statement::AssignConst {
                    span: Span::new(7, 14),
                    name: "y".into(),
                    name_braced: false,
                    value: "1".into(),
                    value_span: None,
                }]),
                body_span: Span::new(6, 15),
                condition_base: None,
            }],
            else_body: None,
            else_span: None,
        }]);
        let func = build_test_cfg_function("::test", &script, true);
        // Should have at least: entry, if_then, if_next, if_end, exit
        assert!(func.blocks.len() >= 4);
        // Entry block should have a Branch terminator.
        let entry = &func.blocks[&func.entry];
        assert!(
            matches!(entry.terminator, Some(Terminator::Branch { .. })),
            "entry should branch; got {:?}",
            entry.terminator
        );
    }

    #[test]
    fn build_cfg_module() {
        let module = Module::default();
        let cfg = build_cfg(&module, false);
        assert_eq!(cfg.top_level.name, "::top");
        assert!(cfg.procedures.is_empty());
    }

    #[test]
    fn prepared_context_cfg_matches_compatibility_entry_point() {
        let registry = CommandRegistry::build_default();
        let module = crate::lowering::lower_to_ir(
            "proc writer {} { global answer; set answer 42 }\n\
             interp alias {} write-answer {} writer\n\
             write-answer\n",
            &registry,
        );
        let context = prepare_cfg_context_bundle(&module, &registry);

        assert_eq!(
            build_cfg_with_registry_and_context(
                &module,
                false,
                &registry,
                &context,
                tcl_lexer::LexerConfig::for_profile(registry.profile()),
            ),
            build_cfg_with_registry(&module, false, &registry),
        );
    }

    #[test]
    fn drop_word_keeps_structured_words_aligned() {
        let tokens = CommandTokens::from_lossy_parts(
            vec![Span::new(0, 4), Span::new(5, 8), Span::new(9, 13)],
            vec!["dict".into(), "for".into(), "body".into()],
            vec![TokenType::Esc, TokenType::Esc, TokenType::Str],
            vec![true, true, true],
            Vec::new(),
            None,
        );
        let dropped = drop_word(&tokens, 1);
        assert_eq!(dropped.argv_texts, vec!["dict", "body"]);
        assert!(dropped.words_align_with_argv_text());
        assert_eq!(dropped.words()[1].legacy_text(), "body");
    }

    #[test]
    fn catch_emits_opaque_call() {
        let script = Script::from_statements(vec![Statement::Catch {
            span: Span::new(0, 30),
            body: Script::from_statements(vec![Statement::AssignConst {
                span: Span::new(7, 14),
                name: "inner".into(),
                name_braced: false,
                value: "1".into(),
                value_span: None,
            }]),
            body_span: Span::new(6, 15),
            result_var: Some("result".into()),
            options_var: None,
            raw_args: vec!["{set inner 1}".into(), "result".into()],
            tokens: None,
        }]);
        let func = build_test_cfg_function("::test", &script, true);
        let entry = &func.blocks[&func.entry];
        // Should have a Call to "catch" with defs.
        assert!(entry.statements.iter().any(|s| matches!(
            s,
            Statement::Call {
                command, defs, ..
            } if command == "catch" && !defs.is_empty()
        )));
    }

    #[test]
    fn foreach_synthetic_call_records_iterator_group_sizes() {
        // ``foreach {a b} L1 c L2 { … }`` has two iterator groups
        // — the first binds two vars, the second binds one.  The
        // synthesised header `Statement::Call` must record the
        // group sizes via ``foreach_groups`` so codegen can
        // reconstruct the original pairing.
        let body = Script::from_statements(vec![Statement::AssignConst {
            span: Span::new(0, 0),
            name: "x".into(),
            name_braced: false,
            value: "1".into(),
            value_span: None,
        }]);
        let script = Script::from_statements(vec![Statement::Foreach {
            span: Span::new(0, 0),
            iterators: vec![
                ForeachIterator {
                    vars: vec!["a".into(), "b".into()],
                    list_arg: "L1".into(),
                    list_braced: false,
                },
                ForeachIterator {
                    vars: vec!["c".into()],
                    list_arg: "L2".into(),
                    list_braced: false,
                },
            ],
            body,
            body_span: Span::new(0, 0),
            is_lmap: false,
            raw_args: vec![],
            is_dict_iteration: false,
            is_array_iteration: false,
            raw_tokens: None,
        }]);
        let func = build_test_cfg_function("::test", &script, true);
        let mut found_groups: Option<Vec<usize>> = None;
        for block in func.blocks.values() {
            for stmt in &block.statements {
                if let Statement::Call {
                    command,
                    foreach_groups,
                    ..
                } = stmt
                    && command == "foreach"
                {
                    found_groups = foreach_groups.clone();
                }
            }
        }
        assert_eq!(found_groups, Some(vec![2, 1]));
    }

    #[test]
    fn dedup_preserves_order() {
        let mut v = vec!["a".into(), "b".into(), "a".into(), "c".into(), "b".into()];
        dedup_preserve_order(&mut v);
        assert_eq!(v, vec!["a", "b", "c"]);
    }

    // Uplevel-literal-set wiring tests.
    //
    // Each test drives the full pipeline:
    // `lower_to_ir` → `build_cfg` (which calls `prepare_cfg_context`).
    // The assertions inspect the resulting CFG to confirm that calls
    // to upvar-using procs carry the expected caller-side defs.

    fn lower_module(src: &str) -> Module {
        use tcl_registry::CommandRegistry;
        crate::lowering::lower_to_ir(src, &CommandRegistry::build_default())
    }

    // --- escaping_loop_jumps: try propagation and
    // dead-code early-stop
    //
    // These exercise the helper directly: their end-to-end W210 effect is
    // masked by a separate, pre-existing `while 1` exit-reachability behaviour
    // (Rust treats the post-loop block of an infinite loop as reachable), so we
    // assert the `(can_break, can_continue)` result the wiring keys off.

    #[test]
    fn escaping_loop_jumps_follows_try_but_not_catch() {
        // `try { break }` (no matching handler) propagates the break to the
        // enclosing loop; `catch { break }` absorbs it (yields code 3). Confirmed
        // against tclsh 9.0.3.
        assert_eq!(
            escaping_loop_jumps(&lower_module("try { break }").top_level, 0),
            (true, false),
            "try propagates break",
        );
        assert_eq!(
            escaping_loop_jumps(&lower_module("try { continue } finally { }").top_level, 0),
            (false, true),
            "try (with finally) propagates continue",
        );
        assert_eq!(
            escaping_loop_jumps(
                &lower_module("try { puts x } on error {} { break }").top_level,
                0
            ),
            (true, false),
            "a break in a try handler body propagates",
        );
        assert_eq!(
            escaping_loop_jumps(&lower_module("catch { break }").top_level, 0),
            (false, false),
            "catch absorbs break (does not propagate)",
        );
    }

    #[test]
    fn escaping_loop_jumps_stops_after_non_completing_stmt() {
        // A `break`/`continue` after a statement that cannot complete normally
        // is dead code and must not be collected as an escaping jump (else it
        // forges a spurious loop-exit edge).
        assert_eq!(
            escaping_loop_jumps(&lower_module("error bad\nbreak").top_level, 0),
            (false, false),
            "dead break after error is not collected",
        );
        assert_eq!(
            escaping_loop_jumps(&lower_module("return\ncontinue").top_level, 0),
            (false, false),
            "dead continue after return is not collected",
        );
        // A live jump before dead code IS collected (and stops the scan there).
        assert_eq!(
            escaping_loop_jumps(&lower_module("break\nset x 1").top_level, 0),
            (true, false),
            "live break is collected",
        );
    }

    /// Regression coverage for issue #996: `escaping_loop_jumps` and the
    /// mutually-recursive `switch_escaping_jumps` recurse once per nested
    /// `if`/`switch`/`Block`/`UpFrame`/`try` body, with no depth cap of
    /// their own before this fix. Transitively bounded to
    /// `MAX_LOWER_DEPTH` (256) by the lowering pass today, so this is
    /// defence-in-depth / consistency with every other full-tree walker in
    /// this crate, not a currently-reproducible crash. 1000 levels of
    /// source nesting is comfortably past this new cap; the assertion is
    /// that `escaping_loop_jumps` returns at all, not what it returns.
    /// Spawns its own big-stack thread since the lexer/CST/segmenter
    /// stages upstream of the lowering cap still walk the full
    /// un-truncated source nesting before that cap trims it — same
    /// rationale as
    /// `codegen::structured::tests::deeply_nested_if_survives_structured_walk`.
    #[test]
    fn deeply_nested_if_survives_escaping_loop_jumps() {
        const DEPTH: usize = 1000;
        const STACK_SIZE: usize = 64 * 1024 * 1024;
        let mut src = String::new();
        for _ in 0..DEPTH {
            src.push_str("if {1} {\n");
        }
        src.push_str("break\n");
        for _ in 0..DEPTH {
            src.push_str("}\n");
        }
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || {
                let _ = escaping_loop_jumps(&lower_module(&src).top_level, 0);
            })
            .unwrap()
            .join()
            .unwrap();
    }

    fn find_call_defs<'a>(func: &'a Function, command: &str) -> Option<&'a [String]> {
        for block in func.blocks.values() {
            for stmt in &block.statements {
                if let Statement::Call {
                    command: c, defs, ..
                } = stmt
                    && c == command
                {
                    return Some(defs);
                }
            }
        }
        None
    }

    #[test]
    fn rooted_regexp_value_form_does_not_widen_runtime_selected_method_frame() {
        let registry = CommandRegistry::build_default();
        let module = crate::lowering::lower_to_ir(
            "oo::class create C {\n\
                 method value {re s} {::regexp $re $s}\n\
                 method capture {re s target} {::regexp $re $s $target}\n\
             }",
            &registry,
        );
        let context = prepare_cfg_context_with_registry(&module, &registry);
        let build = |name: &str| {
            let method = &module.methods[name];
            build_cfg_method_function_with_upvars(
                name,
                &method.body,
                method.execution_namespace.clone(),
                true,
                &registry,
                module.plain_command_dispatch,
                context.clone(),
                false,
            )
        };
        let has_opaque_variable_barrier = |function: &Function| {
            function.blocks.values().any(|block| {
                block.statements.iter().any(|statement| {
                    matches!(
                        statement,
                        Statement::Barrier { reason, .. }
                            if reason.contains("writes a source-opaque variable name")
                    )
                })
            })
        };

        let value = build("::C::value");
        assert!(
            !has_opaque_variable_barrier(&value),
            "an absolute two-operand regexp has no capture variable"
        );

        let capture = build("::C::capture");
        assert!(
            has_opaque_variable_barrier(&capture),
            "a dynamic capture name must still widen the variable frame"
        );
    }

    #[test]
    fn detect_upvar_procs_registers_short_and_qualified() {
        let module = lower_module("proc ::ns::p {} { upvar 1 caller_x x }\nproc ::ns::p2 {} {}");
        let upvar_procs = detect_upvar_procs(&module);
        assert!(upvar_procs.contains_key("::ns::p"));
        assert!(upvar_procs.contains_key("p"));
        // p2 has no upvar — not registered.
        assert!(!upvar_procs.contains_key("::ns::p2"));
        assert!(!upvar_procs.contains_key("p2"));
    }

    #[test]
    fn redefined_procedure_generation_is_an_opaque_frame_effect() {
        let module = lower_module(
            "proc p {} {}\n\
             rename p old\n\
             proc p {} {upvar 1 x y; set y 2}\n\
             proc caller {} {p}",
        );
        let summaries = detect_upvar_procs(&module);
        assert!(summaries["::p"].caller_frame_barrier().writes);
        let cfg = build_cfg(&module, false);
        assert!(cfg.procedures["::caller"].caller_frame_barrier.writes);
    }

    #[test]
    fn recovered_procedure_without_a_published_summary_is_opaque() {
        let module = lower_module(
            "namespace eval ::n [list proc writer {} {upvar 1 x y; ::set y 2}]\n\
             proc caller {} {::n::writer}",
        );
        assert!(!module.procedures.contains_key("::n::writer"));
        let cfg = build_cfg(&module, false);
        assert!(cfg.procedures["::caller"].caller_frame_barrier.writes);
    }

    // Issue #923 idx 18 (revisited after PR #1020 review): a wrapper proc
    // that reaches an already-known upvar proc through a *plain* call
    // (`real_worker $fvar $nvar $script`, not `uplevel`) does NOT itself
    // become an upvar-write target for its own caller — tclsh9.0/8.6-
    // verified (`can't read "myf": no such variable"` when the caller reads
    // the variable in a statement separate from the call, i.e. genuinely
    // outside any `uplevel`'d script argument). A plain call only shares
    // *values*, not stack frames: `real_worker`'s own `upvar 1` reaches the
    // wrapper's frame, not the wrapper's caller's frame — an earlier
    // version of this fix treated every such pass-through as transitive,
    // which was disproven by re-testing with the read moved outside the
    // uplevel'd script (see the reverted commit's own follow-up fix for
    // the story). The real tcllib idiom this finding was mined from
    // (`page::util::flow`) reaches its own worker via `uplevel 1 [list
    // ... ]`, not a plain call — genuinely propagating one frame further,
    // confirmed separately against tclsh9.0 — but modelling that shape
    // soundly (accounting for the wrapper's own uplevel level composed
    // with the callee's own upvar level) is out of scope here; tracked at
    // https://github.com/bitwisecook/tcl-lsp/issues/1019.

    #[test]
    fn detect_upvar_procs_does_not_propagate_through_a_plain_call_wrapper() {
        // TN — `wrapper` has no `upvar` of its own, and its call to the
        // known upvar proc `real_worker` is a plain call (not `uplevel`),
        // so it must NOT be registered as a transitive upvar proc.
        let module = lower_module(
            "proc real_worker {fvar nvar script} {\n\
             upvar 1 $fvar f\n\
             upvar 1 $nvar n\n\
             set f 1\n\
             set n 2\n\
             uplevel 1 $script\n\
             }\n\
             proc wrapper {fvar nvar script} {\n\
             real_worker $fvar $nvar $script\n\
             }",
        );
        let upvar_procs = detect_upvar_procs(&module);
        assert!(
            !upvar_procs.contains_key("wrapper"),
            "a plain-call wrapper must not be treated as a transitive upvar proc",
        );
    }

    #[test]
    fn detect_upvar_procs_does_not_propagate_when_args_are_not_passed_through() {
        // TN — `unrelated_wrapper` calls the known upvar proc `real_worker`,
        // but with literal args ("x"/"y"), not its own parameters passed
        // through unchanged — real tclsh9.0/8.6 confirms this genuinely
        // errors ("can't read \"myf\": no such variable"), so it must NOT
        // be registered.
        let module = lower_module(
            "proc real_worker {fvar nvar script} {\n\
             upvar 1 $fvar f\n\
             upvar 1 $nvar n\n\
             set f 1\n\
             set n 2\n\
             uplevel 1 $script\n\
             }\n\
             proc unrelated_wrapper {a b c} {\n\
             real_worker x y $c\n\
             }",
        );
        let upvar_procs = detect_upvar_procs(&module);
        assert!(
            !upvar_procs.contains_key("unrelated_wrapper"),
            "unrelated_wrapper passes literal args, not its own params — must not propagate",
        );
    }

    /// Issue #1019 / issue #923 audit idx 24: the *other* half of the rule
    /// the two tests above pin. A level-**1** effect stops at the wrapper's
    /// own frame, but a level that lands past the callee's caller reaches
    /// this proc's caller through an ordinary call, so the wrapper really
    /// does become a caller-frame writer.
    ///
    /// Oracle, byte-identical on tclsh 9.0.4 and 8.6.16:
    ///
    /// ```tcl
    /// proc setUp2 {var} { uplevel 2 [list set $var 99] }
    /// proc middle {}    { setUp2 answer }
    /// proc outer  {}    { middle ; return $answer }
    /// puts [outer]        ;# -> 99
    /// ```
    #[test]
    fn detect_upvar_procs_propagates_a_beyond_caller_effect_one_plain_hop() {
        let module = lower_module(
            "proc setUp2 {var} {\n             uplevel 2 [list set $var 99]\n             }\n             proc middle {} {\n             setUp2 answer\n             }",
        );
        let upvar_procs = detect_upvar_procs(&module);
        let middle = upvar_procs
            .get("middle")
            .expect("a plain call to a beyond-caller writer registers the wrapper");
        assert!(
            middle.caller_frame_opaque_writes,
            "setUp2's `uplevel 2` lands in middle's caller: {middle:?}"
        );
        assert!(
            middle.caller_frame_opaque_reads,
            "the same script may read there too: {middle:?}"
        );
    }

    /// Tcl 9.0.4 oracle:
    ///
    /// ```tcl
    /// proc far {} {uplevel 2 {set x 2}}
    /// interp alias {} via {} far
    /// proc middle {} {via}
    /// proc outer {} {set x 1; middle; return $x}
    /// puts [outer] ;# -> 2
    /// ```
    #[test]
    fn alias_terminal_proc_propagates_a_beyond_caller_effect() {
        let module = lower_module(
            "proc far {} {uplevel 2 {set x 2}}\n\
             interp alias {} via {} far\n\
             proc middle {} {via}",
        );
        let upvar_procs = detect_upvar_procs(&module);
        let middle = upvar_procs
            .get("::middle")
            .expect("the alias resolves to far's beyond-caller effect");
        assert!(middle.caller_frame_opaque_writes, "{middle:?}");
        assert!(middle.caller_frame_opaque_reads, "{middle:?}");
    }

    /// TN — `uplevel #0` (global) and `uplevel 0` (the callee's own frame)
    /// reach no caller at all, so neither is a beyond-caller effect and
    /// neither makes a plain-call wrapper one.
    #[test]
    fn detect_upvar_procs_does_not_propagate_a_global_or_own_frame_level() {
        for level in ["#0", "0"] {
            let module = lower_module(&format!(
                "proc setSomewhere {{var}} {{\n                 uplevel {level} [list set $var 99]\n                 }}\n                 proc wrapper {{}} {{\n                 setSomewhere answer\n                 }}"
            ));
            let upvar_procs = detect_upvar_procs(&module);
            assert!(
                !upvar_procs.contains_key("wrapper"),
                "`uplevel {level}` never touches a caller frame: {upvar_procs:?}"
            );
        }
    }

    /// The published summaries carry no composition bookkeeping — they are
    /// folded into every procedure's `function_lattice` memo key, so only
    /// what a call site actually reads may travel in them.
    #[test]
    fn detect_upvar_procs_publishes_no_plain_call_bookkeeping() {
        let module = lower_module(
            "proc setUp2 {var} {\n             uplevel 2 [list set $var 99]\n             }\n             proc middle {} {\n             setUp2 answer\n             }",
        );
        for (name, info) in detect_upvar_procs(&module) {
            assert!(
                info.plain_calls.is_empty(),
                "{name} published composition input: {info:?}"
            );
        }
    }

    #[test]
    fn prepare_cfg_context_registers_params_for_all_procs() {
        let module = lower_module("proc ::ns::p {a b} { upvar 1 $a x }\nproc q {c} {}");
        let (_upvar_procs, proc_params, _global_write_procs, _command_bindings) =
            prepare_cfg_context(&module);
        assert_eq!(
            proc_params.get("::ns::p"),
            Some(&vec!["a".to_string(), "b".to_string()]),
        );
        assert_eq!(
            proc_params.get("p"),
            Some(&vec!["a".to_string(), "b".to_string()]),
        );
        // q has no upvar but its params should still be in proc_params.
        assert_eq!(proc_params.get("q"), Some(&vec!["c".to_string()]));
    }

    #[test]
    fn literal_upvar_proc_call_augments_defs() {
        // Caller invokes a proc whose body has `upvar 1 caller_x x`.
        // The call should land with `caller_x` in its defs.
        let module = lower_module(
            "proc setter {} { upvar 1 caller_x x; set x 1 }\n\
             setter",
        );
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "setter")
            .expect("setter call should be in top-level CFG");
        assert!(
            defs.contains(&"caller_x".to_string()),
            "expected caller_x in defs, got {defs:?}",
        );
    }

    #[test]
    fn alias_to_upvar_uses_its_prepended_frame_level() {
        // Tcl 9.0.4: `bind` executes `upvar 1 target local`; assigning
        // `local` in `setter` therefore assigns `target` in its caller.
        // The frame descriptor and argv are both supplied by the closed
        // command binding, not by the source spelling `bind`.
        let module = lower_module(
            "interp alias {} bind {} upvar 1\n\
             proc setter {} { bind target local; set local 99 }\n\
             setter",
        );
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "setter")
            .expect("setter call should be in top-level CFG");
        assert!(
            defs.contains(&"target".to_owned()),
            "the aliased upvar target must invalidate its caller: {defs:?}"
        );
    }

    #[test]
    fn param_upvar_proc_call_resolves_call_site_arg() {
        // `proc setter {name} { upvar 1 $name x }` aliased to whatever
        // the caller passes for `name`.  Call `setter my_var` should
        // augment the call with `my_var` in defs.
        let module = lower_module(
            "proc setter {name} { upvar 1 $name x; set x 1 }\n\
             setter my_var",
        );
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "setter")
            .expect("setter call should be in top-level CFG");
        assert!(
            defs.contains(&"my_var".to_string()),
            "expected my_var in defs, got {defs:?}",
        );
    }

    #[test]
    fn early_and_late_aliases_to_upvar_proc_prepend_the_caller_target() {
        for source in [
            "interp alias {} assign {} setter target\n\
             proc setter {name} { upvar 1 $name x; set x 1 }\n\
             proc outer {} { assign }",
            "proc setter {name} { upvar 1 $name x; set x 1 }\n\
             proc outer {} { assign }\n\
             interp alias {} assign {} setter target",
        ] {
            let module = lower_module(source);
            let cfg = build_cfg(&module, false);
            let outer = cfg
                .procedures
                .get("::outer")
                .expect("outer proc CFG should exist");
            let defs = find_call_defs(outer, "assign").expect("alias call should be in outer CFG");
            assert!(
                defs.contains(&"target".to_owned()),
                "alias definition order must not hide its prepended upvar target: {defs:?}",
            );
        }
    }

    #[test]
    fn alias_chain_preserves_prefix_order_for_upvar_proc_arguments() {
        let module = lower_module(
            "proc setter {first name} { upvar 1 $name x; set x 1 }\n\
             interp alias {} inner {} setter first\n\
             interp alias {} outer {} inner second\n\
             outer",
        );
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "outer")
            .expect("outer alias call should be in top-level CFG");
        assert!(
            defs.contains(&"second".to_owned()),
            "inner then outer alias prefixes must remain in runtime argv order: {defs:?}",
        );
        assert!(
            !defs.contains(&"first".to_owned()),
            "the first fixed argument must not be projected onto the second parameter: {defs:?}",
        );
    }

    #[test]
    fn substituted_upvar_actual_is_opaque_not_a_literal_def() {
        // The source spelling `$caller_var` is not the variable name passed
        // at runtime. No precise def may be manufactured from that spelling.
        let module = lower_module(
            "proc setter {name} { upvar 1 $name x }\n\
             setter $caller_var",
        );
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "setter")
            .expect("setter call should be in top-level CFG");
        assert!(
            !defs.contains(&"caller_var".to_string()),
            "substituted source text became a literal caller def: {defs:?}",
        );
        assert_eq!(
            cfg.top_level.caller_frame_barrier,
            crate::dynamic_names::DynamicNameBarrier {
                writes: true,
                destroys: false,
                reads: false,
            }
        );
    }

    #[test]
    fn trailing_expansion_preserves_a_known_upvar_actual_prefix() {
        let module = lower_module(
            "proc setter {name args} { upvar 1 $name x; set x 1 }\n\
             setter my_var {*}$rest",
        );
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "setter")
            .expect("setter call should be in top-level CFG");
        assert!(defs.contains(&"my_var".to_string()), "defs={defs:?}");
    }

    #[test]
    fn expanded_upvar_actual_is_opaque_not_a_literal_def() {
        let module = lower_module(
            "proc setter {name} { upvar 1 $name x; set x 1 }\n\
             setter {*}$actual",
        );
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "setter")
            .expect("setter call should be in top-level CFG");
        assert!(
            defs.is_empty(),
            "expanded source text became defs: {defs:?}"
        );
        assert_eq!(
            cfg.top_level.caller_frame_barrier,
            crate::dynamic_names::DynamicNameBarrier {
                writes: true,
                destroys: false,
                reads: false,
            }
        );
    }

    #[test]
    fn computed_or_expanded_direct_head_is_not_misidentified_as_a_static_callee() {
        for call in ["$command target", "{*}$invocation"] {
            let module = lower_module(&format!(
                "proc setter {{name}} {{ upvar 1 $name x; set x 1 }}\n{call}"
            ));
            let cfg = build_cfg(&module, false);
            assert_eq!(
                cfg.top_level.caller_frame_barrier,
                crate::dynamic_names::DynamicNameBarrier::default(),
                "call={call}"
            );
        }
    }

    #[test]
    fn non_upvar_proc_call_unchanged() {
        // No upvar in the callee — the call's defs should be empty.
        let module = lower_module("proc no_upvar {} { set x 1 }\nno_upvar");
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "no_upvar")
            .expect("no_upvar call should be in top-level CFG");
        assert!(defs.is_empty(), "expected no augmented defs, got {defs:?}");
    }

    #[test]
    fn qualified_call_resolves_via_qualified_key() {
        // `proc ::ns::setter` is registered under both `::ns::setter`
        // and `setter`.  A qualified call site `::ns::setter` should
        // resolve via the qualified key.
        let module = lower_module(
            "proc ::ns::setter {} { upvar 1 caller_x x }\n\
             ::ns::setter",
        );
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "::ns::setter")
            .expect("::ns::setter call should be in top-level CFG");
        assert!(
            defs.contains(&"caller_x".to_string()),
            "expected caller_x in defs (qualified call), got {defs:?}",
        );
    }

    #[test]
    fn cross_proc_call_inside_proc_body_augments_defs() {
        // Outer proc calls an inner upvar-using proc.  The outer
        // proc's CFG should reflect the augmented defs.
        let module = lower_module(
            "proc inner {} { upvar 1 caller_x x }\n\
             proc outer {} { inner }",
        );
        let cfg = build_cfg(&module, false);
        let outer = cfg
            .procedures
            .get("::outer")
            .expect("outer proc CFG should exist");
        let defs = find_call_defs(outer, "inner").expect("inner call should be in outer CFG");
        assert!(
            defs.contains(&"caller_x".to_string()),
            "expected caller_x in inner's defs (called from outer), got {defs:?}",
        );
    }

    #[test]
    fn late_alias_call_inside_proc_uses_projected_global_write_summary() {
        let module = lower_module(
            "proc ::mutate {} { global x; set x 99 }\n\
             proc ::z::mutate {} {}\n\
             proc ::outer {} { call_mutate }\n\
             interp alias {} call_mutate {} mutate",
        );
        let cfg = build_cfg(&module, false);
        let outer = cfg
            .procedures
            .get("::outer")
            .expect("outer proc CFG should exist");
        let defs =
            find_call_defs(outer, "call_mutate").expect("late alias call should be in outer CFG");
        assert!(
            defs.contains(&"x".to_owned()),
            "projected alias summary must invalidate x at the inner call: {defs:?}",
        );
    }

    #[test]
    fn upvar_call_inside_if_branch_augments_defs() {
        // Calls inside structured constructs (if branches, while
        // bodies, ...) must also be augmented — the wiring runs in
        // `lower_script`, which is invoked recursively for every
        // body.
        let module = lower_module(
            "proc setter {} { upvar 1 caller_y y }\n\
             if {1} { setter }",
        );
        let cfg = build_cfg(&module, false);
        let defs = find_call_defs(&cfg.top_level, "setter")
            .expect("setter call in if-branch should be in top-level CFG");
        assert!(
            defs.contains(&"caller_y".to_string()),
            "expected caller_y in defs (call inside if branch), got {defs:?}",
        );
    }

    #[test]
    fn empty_module_no_upvar_context_no_panic() {
        // Empty module → empty upvar_procs and proc_params.  Building
        // a CFG should still succeed.
        let module = Module::default();
        let cfg = build_cfg(&module, false);
        assert_eq!(cfg.top_level.name, "::top");
        assert!(cfg.procedures.is_empty());
    }

    #[test]
    fn build_cfg_function_does_not_apply_upvar_wiring() {
        // `build_cfg_function` is the no-context variant used by
        // tests and one-off CFG construction.  It MUST NOT augment
        // defs even when the script calls a known upvar proc —
        // the function only sees the script, not a module.
        let module = lower_module("proc setter {} { upvar 1 caller_x x }\nsetter");
        // Build only the top-level CFG via the no-context API.
        let func = build_cfg_function(
            "::top",
            &module.top_level,
            true,
            &CommandRegistry::build_default(),
            module.plain_command_dispatch,
        );
        let defs = find_call_defs(&func, "setter").expect("setter call should be in top-level CFG");
        assert!(
            defs.is_empty(),
            "build_cfg_function without context should leave defs empty, got {defs:?}",
        );
    }

    // Embedded-substitution form: `[upvar_proc arg]` in every evaluated
    // statement value, argument, expression, return, or condition surface.

    /// Walk every block looking for a Call whose `defs` contain
    /// *def*.  Returns the command name when found.
    fn find_call_with_def<'a>(func: &'a Function, def: &str) -> Option<&'a str> {
        for block in func.blocks.values() {
            for stmt in &block.statements {
                if let Statement::Call { command, defs, .. } = stmt
                    && defs.iter().any(|d| d == def)
                {
                    return Some(command);
                }
            }
        }
        None
    }

    #[test]
    fn embedded_subst_in_assign_value_emits_synthetic_invalidate() {
        // `set foo [setter]` where setter upvars caller_x.  The
        // resulting CFG should have a synthetic `<upvar-invalidate>`
        // Call with `caller_x` in its defs, emitted BEFORE the
        // `set foo ...` AssignValue.
        let module = lower_module(
            "proc setter {} { upvar 1 caller_x x; return $x }\n\
             set foo [setter]",
        );
        let cfg = build_cfg(&module, false);
        let cmd = find_call_with_def(&cfg.top_level, "caller_x")
            .expect("expected a Call carrying caller_x in defs");
        assert_eq!(cmd, "<upvar-invalidate>");
    }

    #[test]
    fn embedded_subst_in_return_emits_synthetic_invalidate_before_terminator() {
        let module = lower_module(
            "proc setter {} { upvar 1 caller_x x; set x 1; return 0 }\n\
             proc outer {} { return [setter] }",
        );
        let cfg = build_cfg(&module, false);
        let outer = cfg.procedures.get("::outer").expect("::outer CFG");
        assert_eq!(
            find_call_with_def(outer, "caller_x"),
            Some("<upvar-invalidate>")
        );
        assert!(
            outer
                .blocks
                .values()
                .any(|block| { matches!(block.terminator, Some(Terminator::Return { .. })) })
        );
    }

    #[test]
    fn switch_word_substitutions_invalidate_before_dispatch() {
        let module = lower_module(
            "proc setter {name} { upvar 1 $name value; set value 1; return 0 }\n\
             proc outer {} {\n\
                 set subject 1\n\
                 set pattern 1\n\
                 switch [setter subject] [setter pattern] { return ok } default { return no }\n\
             }",
        );
        let cfg = build_cfg(&module, false);
        let outer = cfg.procedures.get("::outer").expect("::outer CFG");
        for name in ["subject", "pattern"] {
            assert_eq!(
                find_call_with_def(outer, name),
                Some("<upvar-invalidate>"),
                "the unbraced switch {name} substitution must invalidate before dispatch"
            );
        }
        assert!(outer.blocks.values().any(|block| {
            block.statements.iter().any(|stmt| {
                matches!(
                    stmt,
                    Statement::Call { command, defs, .. }
                        if command == "<upvar-invalidate>"
                            && defs.iter().any(|name| name == "subject")
                            && defs.iter().any(|name| name == "pattern")
                )
            }) && matches!(block.terminator, Some(Terminator::Branch { .. }))
        }));
    }

    #[test]
    fn braced_switch_words_do_not_invalidate_before_dispatch() {
        let module = lower_module(
            "proc setter {name} { upvar 1 $name value; set value 1; return 0 }\n\
             proc outer {} {\n\
                 switch {[setter subject]} {[setter pattern]} { return ok }\n\
             }",
        );
        let cfg = build_cfg(&module, false);
        let outer = cfg.procedures.get("::outer").expect("::outer CFG");
        assert_eq!(find_call_with_def(outer, "subject"), None);
        assert_eq!(find_call_with_def(outer, "pattern"), None);
    }

    #[test]
    fn embedded_subst_in_assign_expr_and_expr_eval_emits_invalidation() {
        let module = lower_module(
            "proc setter {} { upvar 1 caller_x x; set x 1; return 0 }\n\
             proc assign_expr {} { set result [expr {[setter] + 1}] }\n\
             proc expr_eval {} { expr {[setter] + 1} }",
        );
        let cfg = build_cfg(&module, false);
        for proc_name in ["::assign_expr", "::expr_eval"] {
            let function = cfg.procedures.get(proc_name).expect("procedure CFG");
            assert_eq!(
                find_call_with_def(function, "caller_x"),
                Some("<upvar-invalidate>"),
                "missing embedded invalidation in {proc_name}"
            );
        }
    }

    #[test]
    fn embedded_subst_in_call_arg_merges_into_call_defs() {
        // `puts [setter]` — Call host with embedded substitution.
        // The defs should merge into the existing Call's defs (no
        // synthetic invalidate needed since the host is a Call).
        let module = lower_module(
            "proc setter {} { upvar 1 caller_x x; return $x }\n\
             puts [setter]",
        );
        let cfg = build_cfg(&module, false);
        let defs =
            find_call_defs(&cfg.top_level, "puts").expect("puts call should be in top-level CFG");
        assert!(
            defs.contains(&"caller_x".to_string()),
            "expected caller_x merged into puts's defs, got {defs:?}",
        );
        // No synthetic invalidate should appear (the Call branch
        // merged in place).
        let synthetic = find_call_with_def(&cfg.top_level, "caller_x");
        assert_eq!(
            synthetic,
            Some("puts"),
            "embedded extras should merge into the Call host, not a synthetic",
        );
    }

    #[test]
    fn embedded_subst_param_form_resolves_call_site_arg() {
        // setter takes a parameter `name`; its upvar source is `$name`.
        // `set foo [setter myvar]` — the embedded call passes "myvar",
        // which becomes the caller-side def.
        let module = lower_module(
            "proc setter {name} { upvar 1 $name x; set x 1 }\n\
             set foo [setter myvar]",
        );
        let cfg = build_cfg(&module, false);
        let cmd = find_call_with_def(&cfg.top_level, "myvar")
            .expect("expected synthetic invalidate carrying myvar");
        assert_eq!(cmd, "<upvar-invalidate>");
    }

    /// Tcl 9.0.4 oracle:
    ///
    /// ```tcl
    /// proc far {name} {upvar 1 $name v; set v 2; return ok}
    /// interp alias {} via {} far x
    /// proc p {} {set x 1; set y [via]; return $x}
    /// puts [p] ;# -> 2
    /// ```
    #[test]
    fn embedded_alias_prefix_terminal_proc_invalidates_its_caller_target() {
        let module = lower_module(
            "proc far {name} {upvar 1 $name v; set v 2; return ok}\n\
             interp alias {} via {} far x\n\
             proc p {} {set x 1; set y [via]; return $x}",
        );
        let cfg = build_cfg(&module, false);
        let p = cfg.procedures.get("::p").expect("::p CFG");
        assert_eq!(find_call_with_def(p, "x"), Some("<upvar-invalidate>"));
    }

    #[test]
    fn embedded_terminal_proc_honours_namespace_and_unknown_resolution() {
        let namespace_module = lower_module(
            "namespace eval ::a {\n\
                 proc mutate {} {upvar 1 x v; set v 2; return ok}\n\
                 proc p {} {set x 1; set y [mutate]; return $x}\n\
             }\n\
             namespace eval ::b {\n\
                 proc mutate {} {upvar 1 y v; set v 3; return ok}\n\
             }",
        );
        let namespace_cfg = build_cfg(&namespace_module, false);
        let p = namespace_cfg.procedures.get("::a::p").expect("::a::p CFG");
        assert_eq!(find_call_with_def(p, "x"), Some("<upvar-invalidate>"));
        assert_eq!(find_call_with_def(p, "y"), None);

        let unknown_module = lower_module(
            "proc unknown {name args} {upvar 1 $name v; set v 2; return ok}\n\
             proc p {} {set missing 1; set y [missing]; return $missing}",
        );
        let unknown_cfg = build_cfg(&unknown_module, false);
        let p = unknown_cfg.procedures.get("::p").expect("::p CFG");
        assert_eq!(find_call_with_def(p, "missing"), Some("<upvar-invalidate>"));
    }

    #[test]
    fn embedded_subst_unknown_command_ignored() {
        // `[not_upvar]` — unknown command, should produce no
        // synthetic invalidate.
        let module = lower_module("proc setter {} { set x 1 }\nset foo [setter]");
        let cfg = build_cfg(&module, false);
        // setter has no upvar, so neither direct nor embedded form
        // contributes — the only Call in the CFG should be
        // for the literal `setter` lookup (none here) or nothing.
        for block in cfg.top_level.blocks.values() {
            for stmt in &block.statements {
                if let Statement::Call { command, .. } = stmt {
                    assert_ne!(
                        command, "<upvar-invalidate>",
                        "no synthetic invalidate should appear for non-upvar embedded calls",
                    );
                }
            }
        }
    }

    #[test]
    fn embedded_subst_no_bracket_in_text_short_circuits() {
        // `set foo "plain string"` — value has no `[`, so the
        // embedded-substitution scan should short-circuit on the
        // `text.contains('[')` guard and produce no extras.
        let module = lower_module(
            "proc setter {} { upvar 1 caller_x x }\n\
             set foo plain",
        );
        let cfg = build_cfg(&module, false);
        let synthetic = find_call_with_def(&cfg.top_level, "caller_x");
        assert!(
            synthetic.is_none(),
            "no synthetic invalidate expected when text has no `[`, got {synthetic:?}",
        );
    }

    #[test]
    fn embedded_subst_synthetic_appears_before_host_assign() {
        // The synthetic invalidate must land BEFORE the host
        // AssignValue in program order, so SSA / dataflow correctly
        // see the invalidation before any later use of the variable.
        let module = lower_module(
            "proc setter {} { upvar 1 caller_x x }\n\
             set foo [setter]",
        );
        let cfg = build_cfg(&module, false);
        let entry = &cfg.top_level.blocks[&cfg.top_level.entry];
        // Find the synthetic invalidate's index and the AssignValue's
        // index; assert ordering.
        let mut synthetic_idx = None;
        let mut assign_idx = None;
        for (i, stmt) in entry.statements.iter().enumerate() {
            match stmt {
                Statement::Call { command, .. } if command == "<upvar-invalidate>" => {
                    synthetic_idx = Some(i);
                }
                Statement::AssignValue { name, .. } if name == "foo" => {
                    assign_idx = Some(i);
                }
                _ => {}
            }
        }
        let s = synthetic_idx.expect("synthetic <upvar-invalidate> should be in entry block");
        let a = assign_idx.expect("set foo AssignValue should be in entry block");
        assert!(
            s < a,
            "synthetic invalidate at {s} should precede assign at {a}",
        );
    }

    // Issue #923 idx 122: a known upvar proc's call-by-name write reached
    // only through a nested `[...]` (a wrapping command around it, or a
    // loop/branch condition) must still be recovered — real tcllib
    // `cmdline::getoptions` repro: `while {[set err [getopt argv $opts
    // opt arg]]} { ... }`.

    #[test]
    fn embedded_subst_recovers_a_wrapped_upvar_proc_call() {
        // TP — `set x [set err [setter]]`: `setter` is nested one bracket
        // deeper than the embedded-substitution scan's own top-level word
        // check reaches (`set` itself is never a known upvar proc), so
        // recovering it requires recursing into the inner text rather than
        // pattern-matching the (bracket-stripped) word list.
        let module = lower_module(
            "proc setter {} { upvar 1 caller_x x; set x 1 }\n\
             set out [set err [setter]]",
        );
        let cfg = build_cfg(&module, false);
        let cmd = find_call_with_def(&cfg.top_level, "caller_x")
            .expect("expected a synthetic invalidate recovering caller_x through the wrapper");
        assert_eq!(cmd, "<upvar-invalidate>");
    }

    #[test]
    fn condition_out_vars_recovers_catch_scan_gets_regexp_and_upvar_procs() {
        // TP — `condition_out_vars` unions the hardcoded builtin scan with
        // the upvar-proc / global-write-proc resolution; a known upvar
        // proc's write reached through a condition must appear in the
        // union exactly like a `catch` result var already does.
        let module = lower_module(
            "proc setter {} { upvar 1 caller_x x; set x 1 }\n\
             if {[setter]} { puts $caller_x }",
        );
        let cfg = build_cfg(&module, false);
        let defs =
            find_call_defs(&cfg.top_level, "<cond>").expect("expected a <cond> Call in the CFG");
        assert!(
            defs.contains(&"caller_x".to_string()),
            "expected caller_x in the <cond> Call's defs, got {defs:?}",
        );
    }

    #[test]
    fn frozen_while_condition_carries_upvar_proc_defs() {
        // TP — `while {[getopt ...]}`'s condition is purely a command
        // substitution, so `lower_while_or_frozen` freezes the whole loop
        // as an opaque `Statement::Barrier` (no `defs` field at all) rather
        // than calling `lower_while`. Without a separate synthetic `<cond>`
        // Call carrying the condition's defs, `getopt`'s upvar write to
        // `opt` would be invisible to the def-use graph even though the
        // barrier's own `uses_of` textually scans the (un-lowered) body
        // for `$opt` and attributes the read to the same statement.
        let module = lower_module(
            "proc getopt {ovar} { upvar 1 $ovar opt; set opt 1 }\n\
             while {[getopt opt]} { puts $opt }",
        );
        let cfg = build_cfg(&module, false);
        let defs =
            find_call_defs(&cfg.top_level, "<cond>").expect("expected a <cond> Call in the CFG");
        assert!(
            defs.contains(&"opt".to_string()),
            "expected opt in the frozen while's <cond> Call defs, got {defs:?}",
        );
        // The barrier itself must still be there too — freezing, not
        // ordinary `lower_while`, is exactly the shape under test.
        let has_barrier = cfg.top_level.blocks.values().any(|b| {
            b.statements
                .iter()
                .any(|s| matches!(s, Statement::Barrier { command, .. } if command == "while"))
        });
        assert!(
            has_barrier,
            "expected the loop to still freeze as a Barrier"
        );
    }

    #[test]
    fn frozen_for_condition_carries_upvar_proc_defs() {
        // TP — the `for` loop's identical frozen-barrier path (issue #923
        // idx 122 applies equally to `for {...} [cond] {...} {...}`).
        let module = lower_module(
            "proc getopt {ovar} { upvar 1 $ovar opt; set opt 1 }\n\
             for {} {[getopt opt]} {} { puts $opt }",
        );
        let cfg = build_cfg(&module, false);
        let defs =
            find_call_defs(&cfg.top_level, "<cond>").expect("expected a <cond> Call in the CFG");
        assert!(
            defs.contains(&"opt".to_string()),
            "expected opt in the frozen for's <cond> Call defs, got {defs:?}",
        );
    }

    #[test]
    fn frozen_while_condition_with_no_upvar_proc_omits_synthetic_cond() {
        // TN — a frozen loop whose condition calls an *ordinary* (non-
        // upvar, non-`catch`/`scan`/`gets`/`regexp`) command must not gain
        // a synthetic `<cond>` Call at all — nothing for it to carry.
        let module = lower_module("proc pureCheck {} { return 1 }\nwhile {[pureCheck]} { }");
        let cfg = build_cfg(&module, false);
        assert!(
            find_call_defs(&cfg.top_level, "<cond>").is_none(),
            "no synthetic <cond> Call expected when the condition defines nothing",
        );
    }

    #[test]
    fn try_handler_return_options_terminates() {
        // A `try` handler ending in `return -code error …` (lowered to a
        // "return with options" barrier) returns from the proc, so it must
        // *terminate* its block rather than fall through to the post-`try`
        // join. Regression for the spurious `try_handler → try_end` edge +
        // phi (e.g. `auto_mkindex`). Analysis builds only (`build_cfg` sets
        // `faithful_exceptions`); codegen leaves the barrier as a plain stmt.
        let module =
            lower_module("proc f {} { try { set a 1 } on error {} { return -code error boom } }");
        let cfg = build_cfg(&module, false);
        let func = cfg.procedures.get("::f").expect("::f cfg");
        let handler = func
            .blocks
            .values()
            .find(|b| b.name.starts_with("try_handler"))
            .expect("try_handler block");
        assert!(
            matches!(handler.terminator, Some(Terminator::Return { .. })),
            "return-options handler must terminate (no fall-through to try_end), got {:?}",
            handler.terminator,
        );
    }
}
