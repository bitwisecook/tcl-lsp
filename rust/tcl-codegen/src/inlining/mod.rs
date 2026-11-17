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

//! General proc inliner (v0 + verbatim + v3 parameterised).
//!
//! Combines the decision catalogue, the IR-level splice transform, and
//! the α-rename machinery (here [`rename`]). The inliner rewrites
//! statement-position calls to *inlinable* procedures so the call
//! boundary disappears before WASM codegen consumes the IR.
//!
//! # Scope
//!
//! Four call shapes are handled:
//!
//! * **v0 — empty body.** A call to a zero-statement proc vanishes.
//! * **v1 / v2 — verbatim wrapper.** A zero-parameter proc whose every
//!   body statement is *splice-eligible* (a frame-independent, def-free,
//!   substitution-free builtin call) has its body spliced verbatim.
//! * **v3 — parameterised.** Procs with parameters and / or local writes.
//!   Every parameter and locally-written variable is α-renamed to a fresh
//!   `__inline_<id>__<name>` slot ([`rename`]), call-site args are bound
//!   to the mangled parameter slots (honouring defaults, variadic `args`,
//!   and braced-literal args), and a non-trailing `return` is lowered to
//!   `set __RESULT <v>; break` wrapped in a one-shot `while {1} { … }`
//!   loop so it doesn't short-circuit the caller.
//!
//! # Consumer
//!
//! Inlining is a pre-codegen IR transform — it dissolves call boundaries
//! so the backend emits flatter code. The LSP and CLI analysis paths
//! report on the program *as written* and never lower to codegen, so they
//! deliberately do not run the inliner. [`inline_module`], which inlines
//! the procedures a module defines, is exposed for the WASM codegen and
//! unwired, with the IR-shape unit tests here as its current verification.
//! [`inline_reference_bodies`] is the bytecode compile service's: it inlines
//! the definitions a pack gives the commands it backs with a Tcl body, and
//! no procedure the module defines, because a site that inlines one records
//! a claim on the pack that the VM attests and the WASM backend records none.
//!
//! # Soundness
//!
//! Two gates combine in [`build_inlinable_map`]: the precise
//! interprocedural [`var_escape::pure_leaf`](tcl_compiler::var_escape) proof
//! (`safe_to_inline`, the soundness predicate) and the
//! structural shape. Redefined procs are never inlined (their body is
//! ambiguous). Recursion cannot loop: the splice replaces a single static
//! call site, and the rename gives every inline a fresh slot namespace.
//! v3 declines a proc whose `return` sits inside a loop / `catch` / `try`
//! / `uplevel` body (where our `break`-based early-return lowering would
//! be trapped) and any call site using `{*}` expansion (runtime arity).
//!
//! Three more conditions keep a splice the call it replaced. The body reads only
//! what its own frame binds, through operands the rename reaches ([`frame`]), for a
//! name it never bound would be the caller's variable. It names its commands as
//! its own namespace does ([`heads`]): spliced into a third namespace each call is
//! spelled from the global one, and a body that substitutes a command there stays a
//! call. And what replaces the call is the value the call had, where anything
//! reads it ([`Tail`]): a `return` stays one only where the value is the
//! procedure's, is wrapped where nothing reads it, and where a `catch` or a `try`
//! holds the value the call stays a call. An `uplevel` body is never rewritten:
//! it is emitted from its text.

mod frame;
mod heads;
mod live;
mod live_rename;
pub(crate) mod reference;
use live::LiveEntry;
mod rename;

use std::collections::HashMap;

use tcl_lexer::LexerConfig;

use tcl_compiler::ir::{CommandTokens, Module, Procedure, Script, Statement};
use tcl_compiler::var_escape::ProcEscapeSummary;
use tcl_registry::CommandRegistry;
use tcl_syntax::word_rules::WordValueRules;

mod eligibility;
mod parameter_bindings;
mod return_routing;
mod rewrite;
mod splice;

use eligibility::{stmt_is_splice_eligible, v3_eligible};
#[cfg(test)]
use parameter_bindings::list_clean_for_splice;
use parameter_bindings::parse_params_with_defaults;
use rewrite::inline_procedures;
#[cfg(test)]
use splice::record_local_base;

struct InliningSummaries<'a> {
    module: &'a Module,
    registry: &'a CommandRegistry,
    escapes: &'a HashMap<tcl_core_types::BodyFrameId, ProcEscapeSummary>,
    /// The calls a pack's reference body answers, by where each stands.
    reference_sites: Option<&'a reference::Sites>,
    /// The definitions a live compile imported, whose parameterised bodies
    /// each site lowers again from their renamed text.
    live: Option<&'a live::LiveBodies<'a>>,
}

impl InliningSummaries<'_> {
    /// Whether a call at `span` may be spliced: anywhere for the release
    /// unit, and only directly in a compiled script for a live compile.
    fn splices_at(&self, span: tcl_lexer::Span) -> bool {
        self.live.is_none_or(|bodies| bodies.at_body_level(span))
    }
    fn get(&self, id: &tcl_compiler::ir::ProcedureId) -> Option<&ProcEscapeSummary> {
        self.escapes.get(&tcl_core_types::BodyFrameId::Body(*id))
    }
    fn selected_body(&self, tokens: &CommandTokens) -> Option<tcl_compiler::ir::ProcedureId> {
        if let Some(id) = self
            .reference_sites
            .and_then(|sites| sites.get(&reference::site_key(tokens)?))
        {
            return Some(*id);
        }
        let calls = tcl_compiler::registry_invocation::original_logical_procedure_calls_for_module(
            tokens,
            self.module,
            self.registry,
        )
        .or_else(|| {
            tcl_compiler::registry_invocation::original_authored_procedure_calls_for_module(
                tokens,
                self.module,
                self.registry,
            )
        })?;
        let first = calls.first()?;
        if !first.accepts_arguments()
            || !first.effective_arguments().binding_prefix.is_empty()
            || !calls.iter().all(|call| {
                call.id() == first.id()
                    && call.accepts_arguments()
                    && call.effective_arguments() == first.effective_arguments()
            })
        {
            return None;
        }
        let written = tokens.words().get(1..)?;
        (first.effective_arguments().words.get(1..)? == written).then_some(first.id())
    }
}

/// Per-proc inlining policy decision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InlineDecision {
    /// Always inline a call to this proc (small, pure-leaf body).
    Always,
    /// Inline only when the proc has exactly one static caller. Recorded
    /// for future pruning; not acted on by the splice (the
    /// post-inline proc-pruning interaction is not yet implemented).
    IfSingleCall,
    /// Never inline.
    Never,
}

/// A body-size and static-call-count fact with no execution permission.
///
/// This type is deliberately distinct from [`InlineDecision`]. It proves no
/// escape, argument, frame, compiler, callback or currentness condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InliningProfitability {
    /// The body's size permits bounded code growth at any static call count.
    BoundedGrowth,
    /// The larger body is size-neutral only at its single static caller.
    SingleCallerOnly,
    /// The size and call count do not meet either profitability rule.
    Unprofitable,
}

/// Describe only body size and static call count, independently of eligibility.
///
/// This function accepts an IR body as data. Its result cannot authorize a
/// splice, frame removal, argument binding or native execution. Those consumers
/// still require their independent semantic proofs through [`classify_proc`]
/// and the actual transformation's existing guards.
#[must_use]
pub fn inlining_profitability(proc: &Procedure, static_call_count: usize) -> InliningProfitability {
    if count_statements(&proc.body) <= SMALL_BODY_THRESHOLD {
        InliningProfitability::BoundedGrowth
    } else if static_call_count == 1 {
        InliningProfitability::SingleCallerOnly
    } else {
        InliningProfitability::Unprofitable
    }
}

/// A pure-leaf proc whose body has at most this many flat statements is
/// unconditionally inlinable.
pub const SMALL_BODY_THRESHOLD: usize = 5;

/// Depth cap shared by every `Script`/`Statement`-tree recursion in this
/// module (`tally_calls`, `has_irreturn_in_unsafe_scope`,
/// `walk_local_writes`).
///
/// Transitively bounded today via `tcl_compiler::lowering`'s
/// `MAX_LOWER_NEST_DEPTH` (every `Script` this module walks is built by
/// lowering, which already caps its own construction at 256), capped here
/// independently for defence-in-depth and consistency with every other
/// full-tree walker in this crate.
const MAX_INLINING_WALK_DEPTH: tcl_core_types::RecursionLimit = tcl_core_types::RecursionLimit(256);

/// What an inlinable proc splices in at a call site.
#[derive(Debug, Clone)]
struct ProcedureDefinitionIdentity {
    name: String,
    namespace: String,
    parameters: String,
    body: String,
}

#[derive(Debug, Clone)]
enum InlineSpec {
    /// v0 — empty body; the call vanishes.
    Empty(ProcedureDefinitionIdentity, LexerConfig),
    /// v1 / v2 — splice these body statements verbatim.
    Verbatim(Script, ProcedureDefinitionIdentity, LexerConfig),
    /// v3 — parameterised; rewrite per call site from this proc, whose
    /// `params_raw` is parsed under the **module's** word-value rules
    /// ([`WordValueRules`]) rather than a re-derived C Tcl default.
    Parameterised(Procedure, WordValueRules, LexerConfig),
}

impl InlineSpec {
    /// The statements of the body the definition has.
    fn body_statements(&self) -> &[Statement] {
        match self {
            Self::Empty(_, _) => &[],
            Self::Verbatim(body, _, _) => &body.statements,
            Self::Parameterised(proc, _, _) => &proc.body.statements,
        }
    }

    fn lexer_config(&self) -> LexerConfig {
        match self {
            Self::Empty(_, config)
            | Self::Verbatim(_, _, config)
            | Self::Parameterised(_, _, config) => *config,
        }
    }

    fn definition(&self) -> ProcedureDefinitionIdentity {
        match self {
            Self::Empty(definition, _) | Self::Verbatim(_, definition, _) => definition.clone(),
            Self::Parameterised(proc, _, _) => procedure_definition(proc)
                .expect("only source-backed procedures enter the inlining catalogue"),
        }
    }
}

fn procedure_definition(proc: &Procedure) -> Option<ProcedureDefinitionIdentity> {
    Some(ProcedureDefinitionIdentity {
        name: proc.publication.as_ref()?.source_spelling()?,
        namespace: proc
            .body
            .namespace_context
            .as_ref()?
            .advisory_key()?
            .clone(),
        parameters: proc.params_raw.clone(),
        body: proc.body_source.clone()?,
    })
}

/// The prefix of the names a splice numbered `cid` gives the body's locals.
fn local_prefix(cid: usize) -> String {
    format!("__inline_{cid}__")
}

/// The first splice number (from 1) whose prefix `source` does not spell, nor
/// any later one: past the largest number any `__inline_<n>__` in it carries.
fn first_unspelled_splice(source: &[u8]) -> usize {
    const HEAD: &[u8] = b"__inline_";
    let mut first = 1;
    let mut at = 0;
    while let Some(found) = source[at..]
        .windows(HEAD.len())
        .position(|window| window == HEAD)
    {
        let digits = &source[at + found + HEAD.len()..];
        let length = digits
            .iter()
            .take_while(|byte| byte.is_ascii_digit())
            .count();
        if let Some(number) = std::str::from_utf8(&digits[..length])
            .ok()
            .and_then(|text| text.parse::<usize>().ok())
        {
            first = first.max(number.saturating_add(1));
        }
        at += found + HEAD.len();
    }
    first
}

// statement counting

/// Total number of leaf statements reachable in `script`, walking nested
/// control-flow bodies transitively (the inlined code-size cost).
#[must_use]
pub fn count_statements(script: &Script) -> usize {
    script.statements.iter().map(count_one).sum()
}

fn count_one(stmt: &Statement) -> usize {
    match stmt {
        Statement::Block { body, .. } | Statement::UpFrame { body, .. } => count_statements(body),
        Statement::If {
            clauses, else_body, ..
        } => {
            let mut n = 0;
            for c in clauses {
                n += 1 + count_statements(&c.body);
            }
            if let Some(b) = else_body {
                n += count_statements(b);
            }
            n
        }
        Statement::For {
            init, next, body, ..
        } => count_statements(init) + 1 + count_statements(next) + count_statements(body),
        Statement::While { body, .. }
        | Statement::Foreach { body, .. }
        | Statement::Catch { body, .. } => 1 + count_statements(body),
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => {
            let mut n = count_statements(body);
            for h in handlers {
                n += count_statements(&h.body);
            }
            if let Some(fb) = finally_body {
                n += count_statements(fb);
            }
            n
        }
        Statement::Switch {
            arms, default_body, ..
        } => {
            let mut n = 0;
            for a in arms {
                if let Some(b) = &a.body {
                    n += count_statements(b);
                }
            }
            if let Some(b) = default_body {
                n += count_statements(b);
            }
            n
        }
        _ => 1,
    }
}

// static call counting

/// Return `{qname: static_call_count}` over the whole module. Walks the
/// top-level script and every proc body; an [`Statement::Call`] whose
/// command resolves (per the same namespace-walk rules the
/// interprocedural pass uses) to a known proc contributes `+1`.
fn count_static_calls(
    module: &Module,
    summaries: &InliningSummaries<'_>,
) -> HashMap<tcl_compiler::ir::ProcedureId, usize> {
    let mut counts: HashMap<_, _> = module.procedures.keys().map(|id| (*id, 0)).collect();
    for body in
        std::iter::once(&module.top_level).chain(module.procedures.values().map(|proc| &proc.body))
    {
        tcl_compiler::ir::for_each_statement(body, &mut |statement| {
            if let Statement::Call {
                tokens: Some(tokens),
                ..
            } = statement
                && let Some(id) = summaries.selected_body(tokens)
                && let Some(count) = counts.get_mut(&id)
            {
                *count += 1;
            }
        });
    }
    counts
}

/// Procedure-body namespace label in the authored inlining model.
#[cfg(test)]
fn root_namespace_of(qname: &str) -> String {
    tcl_syntax::naming::reported_qname_namespace(
        qname,
        tcl_syntax::naming::ReportedQNameNamespacePurpose::AfterLeadingRoot,
    )
}

/// `depth` is the nesting level of `script` — see
/// [`MAX_INLINING_WALK_DEPTH`].
/// Apply the inlining policy to a single proc. A proc is inlinable only if
/// its escape summary is pure-leaf (`safe_to_inline`); among that set,
/// size and static-call-count decide between [`InlineDecision::Always`]
/// (small enough that growth is bounded), [`InlineDecision::IfSingleCall`]
/// (size-neutral), and [`InlineDecision::Never`].
#[must_use]
pub fn classify_proc(
    proc: &Procedure,
    summary: Option<&ProcEscapeSummary>,
    static_call_count: usize,
) -> InlineDecision {
    if tcl_compiler::native_compilation_admission::script_requires_admission(&proc.body) {
        trace_native_inline_decline("compiler-admission", proc, summary);
        return InlineDecision::Never;
    }
    if !summary.is_some_and(ProcEscapeSummary::safe_to_inline) {
        trace_native_inline_decline("escape-summary", proc, summary);
        return InlineDecision::Never;
    }
    let body_size = count_statements(&proc.body);
    if body_size <= SMALL_BODY_THRESHOLD {
        InlineDecision::Always
    } else if static_call_count == 1 {
        InlineDecision::IfSingleCall
    } else {
        InlineDecision::Never
    }
}

fn trace_native_inline_decline(stage: &str, proc: &Procedure, summary: Option<&ProcEscapeSummary>) {
    static LOGS: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_VAR_ESCAPE_BODY_OWNERS").is_none()
        || LOGS.fetch_add(1, std::sync::atomic::Ordering::Relaxed) >= 16
    {
        return;
    }
    eprintln!(
        "NATIVE_INLINE_DECLINE proc={} stage={stage} admission={:?} legacy_failure={} summary={:?}",
        proc.qualified_name,
        proc.body
            .native_compilation_admission
            .as_ref()
            .map(|admission| (
                admission.provider_required,
                admission.failure.is_some(),
                admission.source.is_some(),
            )),
        proc.body.native_compilation_failure.is_some(),
        summary.map(|summary| (
            summary.pure_leaf,
            summary.frame_needed,
            &summary.flags,
            summary.barriers.len()
        )),
    );
}

/// Build the Module-issued body-ID map of inlinable procedures. Only
/// [`InlineDecision::Always`] procs qualify; `IfSingleCall` is not
/// eligible, because its profitability depends on post-inline pruning
/// this pass does not perform. Redefined procs are never inlined.
fn build_inlinable_map(
    module: &Module,
    summaries: &InliningSummaries<'_>,
    registry: &CommandRegistry,
) -> HashMap<tcl_compiler::ir::ProcedureId, InlineSpec> {
    // The transform binds ordinary positional cells, ordered defaults and a
    // final rest slot. Name-only Jim headers cannot issue that activation;
    // unknown axes preserve dispatch instead of borrowing C Tcl binding.
    if module.parameter_grammar() != Some(tcl_dialect::ParameterGrammar::Tcl) {
        return HashMap::new();
    }
    let Some(source) = frame::SourceContext::for_module(module, registry) else {
        return HashMap::new();
    };
    let counts = count_static_calls(module, summaries);
    // The module's own dialect, resolved once here — the lowered module names
    // it, and this is the only point in the inliner's recursion that still
    // holds the module.
    let word_rules = WordValueRules::from_config(&module.lexer_config);
    let mut map = HashMap::new();
    for (id, proc) in &module.procedures {
        let qname = &proc.qualified_name;
        if module.redefined_procedures.contains(qname)
            || !source.matches_procedure(proc)
            || tcl_compiler::native_compilation_admission::script_requires_admission(&proc.body)
        {
            continue;
        }
        let count = counts.get(id).copied().unwrap_or(0);
        if classify_proc(proc, summaries.get(id), count) != InlineDecision::Always {
            continue;
        }
        // Every binding strategy uses the same selected strict ParamList
        // syntax, including exact-arity and empty-body calls. A displayed name
        // vector cannot bypass the original declaration's validation.
        let Some(parameters) = parse_params_with_defaults(&proc.params_raw, word_rules) else {
            continue;
        };
        if !parameters
            .iter()
            .map(|(name, _)| name)
            .eq(proc.params.iter())
        {
            continue;
        }
        // Executable inlining is valid only when codegen can retain an exact
        // definition identity for runtime admission. Synthetic procedures have
        // no Tcl body source to validate, so leave their calls intact.
        let Some(definition) = procedure_definition(proc) else {
            continue;
        };

        // A body runs in a frame of its own. Spliced into a caller's, a name it
        // reads without having bound it would answer the caller's variable of
        // that spelling, so such a body stays a call.
        if !frame::reads_only_bound_names(proc, source) {
            continue;
        }

        // v0 — empty body.
        if proc.body.statements.is_empty() {
            map.insert(*id, InlineSpec::Empty(definition, source.config));
            continue;
        }

        // v1 / v2 — zero-param wrapper, every body statement splice-safe.
        if proc.params.is_empty()
            && proc
                .body
                .statements
                .iter()
                .all(|s| stmt_is_splice_eligible(s, qname, summaries, source))
        {
            map.insert(
                *id,
                InlineSpec::Verbatim(proc.body.clone(), definition, source.config),
            );
            continue;
        }

        // v3 — parameterised inline with α-renaming.
        if v3_eligible(proc, qname, summaries, source) {
            map.insert(
                *id,
                InlineSpec::Parameterised(proc.clone(), word_rules, source.config),
            );
        }
    }
    map
}

/// What the one procedure of `definition` splices in, decided in the module the
/// definition was lowered in: its own text, bindings and frame.
fn reference_inline_spec(definition: &Module, registry: &CommandRegistry) -> Option<InlineSpec> {
    let escape =
        tcl_compiler::var_escape::analyse_var_escape_with_registry(definition, true, registry);
    let summaries = InliningSummaries {
        module: definition,
        registry,
        escapes: &escape,
        reference_sites: None,
        live: None,
    };
    build_inlinable_map(definition, &summaries, registry)
        .into_values()
        .next()
}

impl InlineSpec {
    /// This decision for `procedure`, the same definition where it now stands.
    fn moved_to(self, procedure: &Procedure) -> Self {
        match self {
            Self::Empty(definition, config) => Self::Empty(definition, config),
            Self::Verbatim(_, definition, config) => {
                Self::Verbatim(procedure.body.clone(), definition, config)
            }
            Self::Parameterised(_, rules, config) => {
                Self::Parameterised(procedure.clone(), rules, config)
            }
        }
    }
}

// the pass

/// Inline eligible statement-position calls throughout `module`,
/// returning the rewritten module. Idempotent: a module with no remaining
/// inlinable sites is returned structurally unchanged.
///
/// Dead-proc elimination is intentionally omitted: the [`Procedure`] type
/// carries no `compiler_synthetic` flag to key it on. User procs stay in
/// the module so host eval / `info procs` / `rename` observers still see
/// them.
#[must_use]
pub fn inline_module(module: Module, registry: &CommandRegistry) -> Module {
    inline_procedures(module, registry, &Inlining::Everything)
}

/// Inline the calls `module` makes to commands a pack declares
/// `TclBody`-backed, and to no procedure the module defines.
///
/// The definition each such command's `runtime_backing` names is brought into
/// the module ([`ReferenceBodies`](tcl_compiler::ir::ReferenceBodies)) and inlined under
/// the policy a procedure the module defines is: a pure leaf of at most
/// [`SMALL_BODY_THRESHOLD`] statements, never at a site the runtime could not
/// replay. Each site records the procedure binding the runtime holds the live
/// command to, and codegen records a
/// [`tcl_runtime_api::SiteClaim::ReferenceBody`] beside it, so the unit is
/// admitted only while the live command is that definition and the pack that
/// declared it is the one the runtime holds. A module that calls none, a plain
/// dispatch compile, a release that keeps the commands running bytecode
/// compiled (`TclVersion::revalidates_running_bytecode`), and a registry that
/// holds no pack with a body are returned unchanged.
///
/// `config` and `profile` are the compile's own, so a definition is read under
/// the grammar its caller is.
#[must_use]
pub fn inline_reference_bodies(
    mut module: Module,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    profile: Option<&'static tcl_dialect::DialectProfile>,
) -> Module {
    // A procedure body is a unit of its own that a release keeping the
    // commands running bytecode compiled enters afresh at each call, which a
    // body spliced into its caller would not be.
    let release = profile.map_or(tcl_dialect::TclVersion::V9_0, |profile| {
        profile.vm_runtime_version
    });
    if module.plain_command_dispatch || !release.revalidates_running_bytecode() {
        return module;
    }
    let imported = reference::import(&mut module, registry, config, profile);
    if imported.ids.is_empty() {
        return module;
    }
    let mut module = inline_procedures(module, registry, &Inlining::Only(&imported));
    module
        .procedures
        .retain(|name, _| !imported.ids.contains(name));
    module
}

/// [`inline_reference_bodies`] for a procedure body the runtime compiles under
/// its live native entry (`options`): the definitions are spliced with the
/// native selections that entry gives their commands, so the runtime can
/// replay each. `fresh` reads a definition as a fresh interpreter of the
/// release does, to decide what it splices in. `None` when the module calls
/// no command a pack defines with a body it can import.
#[must_use]
pub(crate) fn inline_reference_bodies_live(
    module: &Module,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    profile: &'static tcl_dialect::DialectProfile,
    options: tcl_compiler::command_binding::SourceAnalysisOptions<'_>,
    fresh: impl FnOnce() -> Option<tcl_compiler::analyser::ResolvedAnalysisInput>,
) -> Option<Module> {
    if module.plain_command_dispatch || !profile.vm_runtime_version.revalidates_running_bytecode() {
        return None;
    }
    let called = live::called_commands(module, registry)?;
    let fresh = fresh()?;
    let mut module = module.clone();
    let entry = LiveEntry {
        options,
        fresh: &fresh,
    };
    let (imported, bodies) = live::import(&mut module, registry, config, profile, entry, &called);
    if imported.ids.is_empty() {
        return None;
    }
    let mut module = inline_procedures(module, registry, &Inlining::Live(&imported, &bodies));
    module
        .procedures
        .retain(|name, _| !imported.ids.contains(name));
    module.source = bodies.into_image();
    Some(module)
}

/// Which procedures a pass inlines the calls to.
enum Inlining<'a> {
    /// Every procedure the module defines that the policy admits.
    Everything,
    /// Only the procedures the importer added: the module's own are neither
    /// inlined nor rewritten.
    Only(&'a reference::Imported),
    /// [`Self::Only`], for a module lowered under a live native entry.
    Live(&'a reference::Imported, &'a live::LiveBodies<'a>),
}

impl Inlining<'_> {
    fn admits(&self, id: &tcl_compiler::ir::ProcedureId) -> bool {
        match self {
            Self::Everything => true,
            Self::Only(imported) | Self::Live(imported, _) => imported.ids.contains(id),
        }
    }

    /// Whether the module's top level is rewritten, and whether it may be Tcl
    /// frame zero. A script's global level is frame zero's, and a body inlined
    /// there would leave its renamed parameters behind as global variables, so
    /// a pass that inlines a pack's definition leaves it alone; a
    /// procedure-body compile's top level is a procedure's, with a frame of its
    /// own.
    fn top_level(&self, module: &Module) -> Option<bool> {
        match self {
            Self::Everything => Some(true),
            Self::Only(_) | Self::Live(..) => (module.top_level_kind
                == tcl_compiler::ir::TopLevelKind::ProcedureBody)
                .then_some(false),
        }
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod reported_namespace_tests;
