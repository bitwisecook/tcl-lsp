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

//! Lower Tcl source to structured analysis IR.
//!
//! Translates a flat token stream (via the segmenter) into the tree of
//! IR nodes defined in [`crate::ir`]. Each Tcl command is pattern-matched by
//! name and converted to a typed IR statement.

use std::collections::{HashMap, HashSet};

use tcl_lexer::TokenType;
use tcl_registry::CommandRegistry;
use tcl_registry::events::IrulesExecutionContext;

use crate::alias::CommandAliasMap;
use crate::command_binding::{RetainedSourceBodyInventory, TrustedPackageLoader};
#[cfg(test)]
use crate::command_binding::{SourceAnalysisOptions, SourceCommandBindings};
#[cfg(test)]
use crate::ir::MethodKind;
use crate::ir::{CommandBindingSite, CommandTokens, Module, Procedure, Script, Statement};
use tcl_source::segment::SegmentedCommand;
#[cfg(test)]
use tcl_syntax::word_rules::WordValueRules;

mod authored_source_bodies;
mod declared_frame_bodies;
#[doc(hidden)]
pub mod execution_regions;
pub(crate) mod hooks;
mod hosted_source_bodies;
mod installed_bodies;
#[doc(hidden)]
pub mod source_body_syntax;
pub use source_body_syntax::{
    SourceBodySyntaxCache, SourceBodySyntaxRequest, SourceBodySyntaxTemplate,
};
#[cfg(test)]
mod native_body_context_tests;
#[cfg(test)]
mod original_body_word_geometry_tests;
#[cfg(test)]
mod passive_metadata_tests;
pub use execution_regions::{declared_surface_package_loader, stock_body_provider_loader};
// `pub(crate)` for one item: `structured::parse_switch_options`, which the
// dispatch chain's lowering asks where a `switch`'s options end, and whether
// `--` closed them, rather than carrying a second copy of that rule.
pub(crate) mod structured;

mod body_cache;
mod bytecode_entry;
mod commands;
mod configuration;
mod declarations;
mod module_assembly;
mod script_bodies;

pub use body_cache::{
    body_cache_eligible, lower_proc_body_isolated, lower_to_ir_with_body_cache,
    source_may_alias_commands,
};
pub use bytecode_entry::{
    lower_original_procedure_bytes_module_for_bytecode_with_options,
    lower_proc_body_module_for_bytecode, lower_proc_body_module_for_bytecode_with_options,
    lower_procedure_bytes_module_for_bytecode_with_options,
    lower_procedure_target_module_for_bytecode_with_options,
    lower_script_bytes_module_for_bytecode_with_options, lower_script_module_for_bytecode,
    lower_script_module_for_bytecode_with_options,
};
#[cfg(test)]
use declarations::{
    declared_member_vars, parse_formal_param_names, parse_var_list_names, proc_body_namespace,
    qualify_proc_name, statement_words,
};
use script_bodies::over_depth_script;
#[cfg(test)]
use script_bodies::{
    assigned_local_body_literal, body_has_dynamic_barrier, eval_list_literal_body,
    invalidate_const_map_for,
};

#[derive(Clone, Copy, PartialEq, Eq)]
enum LoweringMetadataOrigin {
    Standalone,
    Supplied,
}

/// Join a parent namespace with a child name.
fn join_namespace(parent: &str, child: &str) -> String {
    tcl_syntax::naming::qualify(parent, child)
}

/// True iff word `idx` (full argv index) is a single braced-literal
/// (`Str`) token.
fn word_is_static_braced(kinds: &[TokenType], single: &[bool], idx: usize) -> bool {
    idx < kinds.len() && idx < single.len() && single[idx] && kinds[idx] == TokenType::Str
}

/// `SegmentedCommand` variant of [`word_is_static_braced`]: word
/// `idx` is a single braced-literal (`Str`) token.
fn seg_word_is_static_braced(seg: &SegmentedCommand, idx: usize) -> bool {
    seg.single_token_word.get(idx).copied().unwrap_or(false)
        && seg.argv.get(idx).is_some_and(|t| t.kind == TokenType::Str)
}

/// Word `idx` is a single substitution-free literal: a `{braced}` (`Str`) word
/// or a plain bareword (`Esc`) — exactly the body words C's `TclCompileIfCmd`
/// inlines. A word carrying `$var` / `[cmd]` substitution, or a multi-token
/// concatenation like `$x1$x2`, is not (it must be substituted then evaluated
/// as a script by the runtime `if` command).
fn seg_word_is_static_literal(seg: &SegmentedCommand, idx: usize) -> bool {
    seg.single_token_word.get(idx).copied().unwrap_or(false)
        && seg
            .argv
            .get(idx)
            .is_some_and(|t| matches!(t.kind, TokenType::Str | TokenType::Esc))
}

/// Word `idx` is C's `TCL_TOKEN_SIMPLE_WORD`: a [`seg_word_is_static_literal`]
/// word whose value is its text, so a bareword or quoted word with a
/// backslash is not (`\"a\"` is `"a"`). A condition or body that is not
/// simple is substituted and then evaluated by the runtime command.
fn seg_word_is_simple(seg: &SegmentedCommand, idx: usize) -> bool {
    seg_word_is_static_literal(seg, idx)
        && (seg.argv.get(idx).is_some_and(|t| t.kind == TokenType::Str)
            || seg.texts.get(idx).is_some_and(|text| !text.contains('\\')))
}

/// Whether word `idx` runs a `$var` / `[cmd]` substitution when it is
/// evaluated — so its value is run-time data rather than the text as written.
///
/// Weaker than the negation of [`seg_word_is_static_literal`], and
/// deliberately so: a **compound** literal word (`{body}x`, two literal tokens
/// welded together) is still spelled out in the source, so a body walk that
/// clamps it with [`tcl_source::segment::body_text_in_region`] rebases honest
/// spans. `{*}$n` under an expansionless grammar looks the same
/// from the representative token alone — braced, multi-token — yet its second
/// fragment substitutes, so its value appears nowhere in the document.
///
/// Falls back to the representative token kind when the per-word fragments are
/// unavailable, matching the coarse view the other word gates here take.
fn seg_word_substitutes(seg: &SegmentedCommand, idx: usize) -> bool {
    match seg.word_fragments.get(idx) {
        Some(fragments) if !fragments.is_empty() => fragments
            .iter()
            .any(|fragment| matches!(fragment.token.kind, TokenType::Var | TokenType::Cmd)),
        _ => seg
            .argv
            .get(idx)
            .is_some_and(|tok| matches!(tok.kind, TokenType::Var | TokenType::Cmd)),
    }
}

/// Return `Some(true)` / `Some(false)` if *`expr_text`* is a
/// literal Tcl boolean, or `None` when the condition is not a
/// simple literal. Tolerates surrounding whitespace and is
/// case-insensitive (matches `Tcl_GetBoolean`).
pub(crate) fn static_bool(expr_text: &str) -> Option<bool> {
    let stripped = expr_text.trim();
    match stripped {
        "0" => Some(false),
        "1" => Some(true),
        // Boolean words, incl. unique-prefix spellings (`tr`, `ye`, `of`).
        _ => tcl_syntax::boolean::parse_boolean_word(stripped),
    }
}

/// A memoised per-procedure body-lowering callback: `(offset-0 body text,
/// namespace) → offset-0 body [`Script`]`.  See [`Lowerer::with_body_cache`].
type BodyCacheFn<'a> = dyn Fn(&str, &str) -> Script + 'a;

/// Actual isolated-body worker output and complete lowering recipe. The body is
/// private; independently supplied script values cannot create this template.
#[derive(Debug, Clone, PartialEq)]
pub struct OriginalBodyLoweringTemplate {
    text: String,
    namespace: String,
    registry: tcl_registry::RegistrySnapshot,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    body: Script,
}
impl OriginalBodyLoweringTemplate {
    /// Run the existing isolated lowerer once with the exact requested inputs.
    #[must_use]
    pub fn lower(
        text: &str,
        namespace: &str,
        registry: &CommandRegistry,
        config: tcl_lexer::LexerConfig,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> Self {
        let body = lower_proc_body_isolated(text, namespace, registry, config, dialect);
        Self {
            text: text.to_owned(),
            namespace: namespace.to_owned(),
            registry: registry.snapshot(),
            config,
            dialect,
            body,
        }
    }
    /// Reporting projection, without source-build or execution admission.
    #[must_use]
    pub fn body(&self) -> &Script {
        &self.body
    }
    fn matches(
        &self,
        text: &str,
        namespace: &str,
        registry: &CommandRegistry,
        config: tcl_lexer::LexerConfig,
        dialect: Option<&'static tcl_dialect::DialectProfile>,
    ) -> bool {
        self.text == text
            && self.namespace == namespace
            && self.registry == registry.snapshot()
            && self.config == config
            && self.dialect == dialect
    }
}
/// Memo callback returning an actual immutable isolated-body worker artifact.
pub type OriginalBodyLoweringCache<'a> = dyn Fn(&str, &str) -> OriginalBodyLoweringTemplate + 'a;

/// Which lowering pass a [`Lowerer`] performs.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub(crate) enum CompileTarget {
    /// The analysis IR: structured control flow kept everywhere, every
    /// registry-driven hook active. What LSP consumers (diagnostics,
    /// navigation) lower against.
    #[default]
    Analysis,
    /// The VM-faithful bytecode pass ([`lower_to_ir_for_bytecode`]):
    /// constructs the bytecode backend can't compile correctly (a bare
    /// `try`, a directly-nested `foreach`/`lmap`) barrier to a runtime
    /// dispatch instead of their structured IR; every other hook active.
    Bytecode,
    /// The bytecode pass with every registry-driven inline/structured
    /// lowering hook ALSO suppressed ([`lower_to_ir_traced`]): every command
    /// falls straight to [`Lowerer::lower_default`] — a plain runtime
    /// dispatch (`Statement::Call`/`Statement::Barrier` with a non-empty
    /// `command`, both of which `tcl_codegen::codegen` emits as `push words;
    /// INVOKE`), never a typed statement (`AssignConst`/`Incr`/`Return`/…)
    /// or a structured control-flow block (`If`/`While`/`Foreach`/`Switch`/
    /// `Catch`/`Try`/…). Used to recompile a proc/script body so execution
    /// traces (`enterstep`/`leavestep`) observe every command in it; see
    /// [`tcl_runtime_api::CompileService::compile_traced`]. Never set for
    /// the primary module compile — only for a standalone "compile this body
    /// text as a script" call, so a proc's own `proc name args body`
    /// DEFINITION always module-compiles normally; only its *body* is
    /// affected when recompiled under this target.
    BytecodeTraced,
}

impl CompileTarget {
    /// Whether this pass targets the bytecode backend (barriers
    /// backend-incompatible structured forms) — [`Self::Bytecode`] or
    /// [`Self::BytecodeTraced`].
    pub(crate) fn is_bytecode(self) -> bool {
        matches!(self, Self::Bytecode | Self::BytecodeTraced)
    }

    /// Whether this pass suppresses every inline/structured hook —
    /// [`Self::BytecodeTraced`] only.
    pub(crate) fn is_trace_visible(self) -> bool {
        self == Self::BytecodeTraced
    }
}

/// Origin of the execution entry contract used to initialise source resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SourceEntryOrigin {
    Authoring,
    Driver,
}

/// Complete source metadata retained independently of an explicitly selected
/// Standalone authoring driver. It supplies no runtime or Native entry.
#[derive(Clone, Copy)]
pub(crate) struct StandaloneSourceInput<'a> {
    registry: &'a CommandRegistry,
    input: &'a crate::analyser::ResolvedAnalysisInput,
}

impl<'a> StandaloneSourceInput<'a> {
    pub(crate) fn retain(
        registry: &'a CommandRegistry,
        input: &'a crate::analyser::ResolvedAnalysisInput,
        config: tcl_lexer::LexerConfig,
        profile: Option<&tcl_dialect::DialectProfile>,
    ) -> Option<Self> {
        if config != input.lexer_config() {
            return None;
        }
        crate::registry_invocation::InvocationMetadataContext::for_source_input(
            registry, input, config, profile,
        )?;
        Some(Self { registry, input })
    }
}

use tcl_lexer::SourceSegmentationPurpose;

#[derive(Clone, Copy)]
enum SourceBodyMode {
    Ordinary,
    ReadOnly,
}

impl SourceBodyMode {
    fn is_read_only(self) -> bool {
        matches!(self, Self::ReadOnly)
    }
}

/// Whether the catalogue describes every command the host can register.
#[derive(Clone, Copy, PartialEq, Eq)]
enum HostCommandTable {
    Catalogue,
    Extensible,
}

impl HostCommandTable {
    const fn of(extensible: bool) -> Self {
        if extensible {
            Self::Extensible
        } else {
            Self::Catalogue
        }
    }

    const fn is_extensible(self) -> bool {
        matches!(self, Self::Extensible)
    }
}

#[cfg(test)]
#[derive(Clone, Copy)]
enum SourceBodySyntaxCacheVerification {
    Disabled,
    Enabled,
}

/// The lowering engine — accumulates procedures and IR statements.
pub struct Lowerer<'r> {
    /// Output module being built.
    pub module: Module,
    /// Compatibility hook argument; semantic aliases are owned by source bindings.
    aliases: CommandAliasMap,
    /// Point-specific command state in the current source span space.
    source_bindings: Option<RetainedSourceBodyInventory>,
    original_native_compiler_formals:
        Option<std::sync::Arc<crate::command_binding::OriginalNativeCompilerFormals>>,
    /// Written geometry only while lowering an authentic body with its
    /// independently selected namespace; never an execution or lookup owner.
    original_body_word_source: Option<crate::command_binding::OriginalBodyWrittenWordOwner>,
    /// Exact entry inventory retained for deferred method extraction.
    module_source_bindings: Option<Box<RetainedSourceBodyInventory>>,
    compilation_scope: tcl_runtime_api::SourceCompilationScope,
    /// Explicit driver-supplied standard-distribution loader contracts.
    trusted_package_loaders: Vec<TrustedPackageLoader>,
    trusted_source_modules: Vec<crate::command_binding::TrustedSourceModuleLoader>,
    /// Whether prior interpreter history prevents a fresh entry proof.
    unknown_entry: bool,
    /// The host may register commands outside the catalogue.
    host_command_table: HostCommandTable,
    /// An explicit entry contract replaces the default authoring assumption.
    source_entry_origin: SourceEntryOrigin,
    supplied_source: Option<crate::command_binding::SuppliedSourceAnalysisIssuance>,
    /// Availability phase supplied by the source entry owner.
    invocation_realm: tcl_dialect::model::InvocationRealm,
    /// Invocation policy snapshot supplied by the driver.
    invocation_dialect: Option<tcl_native_model::InvocationDialect>,
    hosted_execution_context: Option<tcl_registry::f5::BigIpExecutionContext>,
    execution_name_policy: Option<tcl_syntax::naming::ExecutionNamePolicy>,
    metadata_input: Option<crate::analyser::ResolvedAnalysisInput>,
    logical_source_input: Option<crate::analyser::ResolvedAnalysisInput>,
    vendor_source_input: Option<crate::analyser::ResolvedAnalysisInput>,
    compiled_variable_provider:
        Option<tcl_native_model::native_compiled_variables::LogicalCompiledVariableProvider>,
    expression_parser: Option<tcl_native_model::LogicalExpressionParseProvider>,
    /// Evaluation protocol, independent of the runtime variable frame.
    native_compilation: tcl_registry::native_compilation::NativeCompilationContext,
    native_entry: Option<std::sync::Arc<tcl_runtime_api::NativeCompilationEntry>>,
    stock_tclsh_stdout: Option<crate::command_binding::StockTclshStdout>,
    source_point_entry: Option<crate::command_binding::OriginalSourcePointEntry>,
    analyser_source_settings:
        Option<std::sync::Arc<crate::analyser::OriginalAnalyserSourceSettings>>,
    source_channel: tcl_lexer::SourceChannel,
    incoming_formals: Vec<String>,
    /// `class qualified name -> every instance variable declared for it`,
    /// accumulated across **all** of that class's definition blocks.
    ///
    /// A class's state is not confined to the block that created it: `oo::class
    /// create C { method m … }` followed by `oo::define C { variable x }`
    /// makes `x` instance state for `m`, which was extracted before the
    /// declaration was seen.  Each block's [`Self::extract_oo_methods`] run
    /// computes only its *own* block's declarations, so the union is
    /// accumulated here and merged into every method of the class by
    /// [`Self::extract_oo_methods_pass`] once all blocks have been walked.
    /// Declaring `variable x` anywhere makes it instance state for every
    /// method of that class, so the union is order-free.
    class_instance_vars: HashMap<String, HashSet<String>>,
    /// Event handler occurrence counts (for `when` numbering).
    when_counts: HashMap<String, u32>,
    /// The registry-owned iRules execution placement of the script currently
    /// being lowered.  This is deliberately distinct from `proc_depth`:
    /// control-flow bodies inherit an event/procedure context, whereas a body
    /// opened by a top-level executable statement is invalid declaration
    /// territory and must not manufacture `when` / `proc` procedures.
    irules_execution_context: IrulesExecutionContext,
    /// Priority inherited by subsequent top-level iRules event declarations.
    irules_priority: u16,
    /// Monotonic counter for synthetic *body unit* names (`apply` lambdas and
    /// `namespace eval` bodies), so each registers under a unique qualified
    /// name in [`Module::body_units`]. Deterministic in source order, so the
    /// fresh and incremental lowerings produce identical names.
    body_unit_count: u32,
    /// Whether we're inside a `namespace eval` body.
    in_namespace_eval: bool,
    /// Command registry for arg-role queries.
    registry: &'r CommandRegistry,
    /// The document's own command declarations (`# tcl-lsp: stub` blocks and
    /// `<dialect>.tcl.stubs` sidecars), unioned with the catalogue's answer
    /// through [`Self::command_surface`]. `None` for a document that
    /// declares nothing, and for every non-document caller (tests, the
    /// bytecode path).
    declared_commands: Option<&'r tcl_registry::model::DeclaredSurface>,
    source_declared_commands: Option<tcl_registry::model::DeclaredSurface>,
    /// Per-script const-map stack. Each scope tracks
    /// proc-local variables assigned a brace-string literal so
    /// later `eval $var` / `uplevel 1 $var` calls can fold the
    /// body in at lowering time. Active only when
    /// `proc_depth > 0` — top-level / `namespace eval` scopes
    /// write globals or namespace vars whose values can be
    /// observed and mutated by other code, so const-propagating
    /// them is unsound.
    const_map_stack: Vec<HashMap<String, String>>,
    /// Per-recursive-script structured command dependencies. Each body owns
    /// its sites, so CFG construction receives the exact bindings for the
    /// script it flattens rather than a module-global approximation.
    command_binding_site_stack: Vec<Vec<CommandBindingSite>>,
    /// Exact original parent across structured body descent; no entry is inferred.
    structured_source_parent: Option<Box<CommandTokens>>,
    /// Depth of `proc` / `when` body lowerings currently in
    /// flight. A positive value enables the const-map.
    proc_depth: u32,
    /// `namespace import` directives observed at lowering time.
    /// Literal `(context_namespace, pattern)` dependency pairs preserve source
    /// order independently of the typed import resolver.
    namespace_imports: Vec<(String, String)>,
    /// `namespace export` directives observed at lowering time.
    /// Recorded as `(context_namespace, pattern)` pairs
    /// and copied into `Module::namespace_exports` at the end of
    /// lowering. Order is preserved.
    namespace_exports: Vec<(String, String)>,
    /// A retained body may be lowered into both a unit and a generic region.
    /// Record each original directive once per actual namespace.
    namespace_directive_sites: HashSet<(crate::command_binding::CommandAllocationSite, String)>,
    /// Depth of statically-dead branches currently being lowered.
    /// `if {0} {…}` / `if {1} {…} else {…}` bump this
    /// around the dead body so any `namespace import` /
    /// `namespace export` directives found inside don't register
    /// with the module-level tables. The IR for the dead code is
    /// still produced so consumers that walk the tree by syntactic
    /// offset see the original structure.
    pub(crate) dead_code_depth: u32,
    /// `true` while lowering a `TclOO` method body. A `proc`
    /// (or `namespace eval`-lifted proc) defined inside a method
    /// body is created at method-call time in the runtime receiver namespace,
    /// NOT at class-definition time — so it must not be lifted into
    /// `module.procedures` (codegen would otherwise emit it
    /// unconditionally at script load). The method body is still
    /// lowered for analysis; only the premature static registration is
    /// suppressed.
    suppress_proc_register: bool,
    /// Genuine readonly Logical body preparation cannot issue executed-source metadata.
    readonly_logical_body: SourceBodyMode,
    readonly_native_declaration_body: SourceBodyMode,
    native_source_children:
        Option<std::sync::Arc<crate::command_binding::OriginalNativeSourceBodyChildren>>,
    /// Hosted source cards keep their independent declaration owner.
    readonly_hosted_body: SourceBodyMode,
    readonly_authored_body: SourceBodyMode,
    authored_source_scope:
        Option<std::sync::Arc<crate::registry_invocation::OriginalAuthoredSourceBodyScope>>,
    /// Lexer config for the document's dialect, threaded into every
    /// body re-segmentation so `{*}` expansion (off for Tcl 8.4 /
    /// iRules) and the iRules `}{` ghost SEP are honoured rather than
    /// always assuming the Tcl-8.5+ default.  Defaults to
    /// `LexerConfig::default()`; production callers thread the active
    /// dialect via [`Lowerer::with_config`] / [`lower_to_ir_with_config`].
    config: tcl_lexer::LexerConfig,
    /// The document's analysis dialect (`None` for plain Tcl), threaded into
    /// every `expr` / condition parse so a dialect-only operator — an iRules
    /// word operator such as `contains` or `starts_with` — is parsed as that
    /// operator instead of degrading to
    /// [`crate::expr_ast::ExprNode::Raw`], which no downstream fold can
    /// evaluate.  Separate from `config`: the lexer config carries the
    /// dialect's *word* tokenisation, this carries its *expression* grammar.
    /// Set by [`Lowerer::with_dialect`] / [`lower_to_ir_with_dialect`].
    dialect: Option<&'static tcl_dialect::DialectProfile>,
    /// The dialect's resolved registry generation, derived beside
    /// [`Self::dialect`] — the context [`try_lower_hook`]'s selection
    /// primitive reads for the I4 binding proof (P1a): a head the
    /// environment does not provide selects no lowering hook and falls to
    /// the default `Statement::Call` path.
    dialect_context: Option<std::sync::Arc<tcl_registry::model::ContextRegistry>>,
    /// Explicit source context survives a driver transition; static authoring
    /// compatibility cannot fill an absent driver's input.
    metadata_origin: LoweringMetadataOrigin,
    /// Which lowering pass this instance performs — folds what would
    /// otherwise be two related bool fields (`for_bytecode`, `trace_visible`)
    /// into one three-state enum (`clippy::struct_excessive_bools`); see
    /// [`CompileTarget`].
    pub(crate) target: CompileTarget,
    /// Optional memoised per-procedure body lowering.
    /// When set, a **top-level** `proc`'s static literal body is lowered through
    /// this callback `(offset-0 body text, namespace) -> offset-0 body Script`
    /// (the caller rebases by the body offset) instead of `lower_body`, so an
    /// unchanged proc's body IR is reused across edits.  Set only for
    /// **context-free** files (no `namespace eval`/`import`/`export`, alias,
    /// `oo::`, `when`, or nested `proc`) where isolated lowering is byte-identical;
    /// `None` ⇒ the normal whole-file lowering.  See [`lower_to_ir_with_body_cache`].
    body_cache: Option<&'r BodyCacheFn<'r>>,
    source_body_syntax_cache: Option<&'r SourceBodySyntaxCache<'r>>,
    original_body_cache: Option<&'r OriginalBodyLoweringCache<'r>>,
    optimiser_lowering_integrity: Option<&'r std::cell::Cell<bool>>,
    #[cfg(test)]
    verify_source_body_syntax_cache: SourceBodySyntaxCacheVerification,
    /// Current `lower_script` / `lower_body` recursion depth, bounded by
    /// [`MAX_LOWER_NEST_DEPTH`] so deeply-nested bodies cannot overflow the
    /// stack. Mirrors [`crate::cfg_builder::CfgBuilder`]'s `depth` field —
    /// lowering runs *before* CFG construction, so an unguarded recursion
    /// here reaches the same crash first and makes the CFG builder's own
    /// cap moot.
    nest_depth: u32,
    /// The buffer every span this lowering emits indexes into — the document
    /// text for an ordinary lowering, and the *materialised literal* while a
    /// synthesised script is lowered at offset `0` inside one (the `eval
    /// $const` / factory-body paths that re-enter [`Self::lower_script`]).
    /// [`Self::lower_script`] swaps it for the duration of each such
    /// re-entry, because that is exactly where a new span space begins;
    /// [`Self::lower_body`] does not, since a body's spans are rebased into
    /// the space it inherits.
    ///
    /// Held so the lowerer can *check* a body word's rebase against the text
    /// it claims to slice ([`tcl_source::segment::body_text_in_region`] in
    /// [`Self::lower_body_from_tok`]) rather than trusting the arithmetic —
    /// the same guard the analyser applies.  Empty until a
    /// script is lowered, which the guard reads as "nothing to compare
    /// against" and leaves every body text alone.
    source: String,
    /// The buffer each word model is read from — see [`WordSpace`].
    word_space: WordSpace,
}

/// The text a [`tcl_source::words::WordExpr`] is decomposed from, and the offset its
/// first byte sits at in the span space the segmentation used.
///
/// Deliberately *not* [`Lowerer::source`]: a word is read from the very text
/// that was segmented into it, which is not always a slice of the enclosing
/// buffer.  A `{body}x` word's body is clamped to the part that does map;
/// an `eval $const` / `eval [list …]` body is a literal
/// materialised at lowering time and lives in a span space of its own; and
/// the `TclOO` method extraction runs as a post-pass, after the document's
/// span space has been restored to the (empty) enclosing one.  Reading the
/// segmented text itself makes the word model right in all three, and the
/// base keeps every span it reports in the space the statements' spans are
/// in.
struct WordSpace {
    text: String,
    base: u32,
    /// `text`'s line index, held so the per-command [`tcl_lexer::SourceMap`]
    /// is a refcount bump (`LineIndex` is `Arc`-backed) rather than a rescan.
    lines: tcl_lexer::LineIndex,
    channel: tcl_lexer::SourceChannel,
}

impl WordSpace {
    fn new(text: &str, base: u32, channel: tcl_lexer::SourceChannel) -> Self {
        Self {
            text: text.to_owned(),
            base,
            lines: tcl_lexer::LineIndex::new(text),
            channel,
        }
    }

    /// A [`tcl_lexer::SourceMap`] over this text, carrying its base so a word
    /// built from it reports spans in the enclosing span space.
    fn map(&self) -> tcl_lexer::SourceMap<'_> {
        tcl_lexer::SourceMap::from_bytes_with_line_index(
            self.text.as_bytes(),
            self.channel,
            self.lines.clone(),
        )
        .with_base(self.base, 0, 0)
    }
}

/// Maximum nesting depth for the recursive `lower_script` / `lower_body`
/// descent (`lower_script` ↔ `lower_body` ↔ `lower_segmented` ↔
/// `lower_command` are mutually recursive with one Rust frame group per
/// braced-body nesting level). Matches
/// [`crate::cfg_builder::MAX_LOWER_DEPTH`] and
/// `tcl_compiler::analyser::commands::MAX_BODY_DEPTH` — all three
/// independently-recursive walkers over the same source cap at the same
/// depth, so no consumer of this crate depends on one pass reaching a
/// deeper nesting level than another. All three read that depth from
/// [`tcl_source::limits::MAX_SOURCE_NEST_DEPTH`], so it is a shared
/// *budget* rather than a shared *convention*.
///
/// This walk is the one the budget is sized by: at eight Rust frames per
/// nesting level it costs about 18.4 KiB a level, more than twice either
/// sibling, and it is the walk whose recursion the ~400-nested-`foreach`
/// reproducer aborted on.
const MAX_LOWER_NEST_DEPTH: tcl_core_types::RecursionLimit =
    tcl_source::limits::MAX_SOURCE_NEST_DEPTH;

/// Shared construction bound for consumers deriving nested source bodies.
/// A subset frontend must decline further descent when this limit is exceeded.
#[must_use]
pub const fn source_nesting_limit() -> tcl_core_types::RecursionLimit {
    MAX_LOWER_NEST_DEPTH
}

/// The substitutions [`crate::subst_nocommands`] performs: `[cmd]` off,
/// `$var` and backslashes on.
///
/// A `subst` call the registry answers with exactly this set is one that
/// evaluator reproduces, however the call spells it —
/// `subst -nocommands {…}` and Tcl 9.1's `subst -variables -backslashes {…}`
/// both land here (tclsh 9.1b0: `subst -variables -backslashes {hello
/// $name\n[format X]}` with `name` set to `world` yields "hello world",
/// a newline, then the untouched `[format X]`). The registry answers a call
/// it cannot read with every kind, which is not this set, so a computed
/// switch word folds nothing.
pub(crate) const SUBST_NOCOMMANDS_KINDS: tcl_registry::substitution::SubstitutionKinds =
    tcl_registry::substitution::SubstitutionKinds {
        backslashes: true,
        commands: false,
        variables: true,
    };

impl Lowerer<'_> {
    /// Build a `CommandTokens` snapshot from a segmented command, decomposing
    /// each word under the document's own grammar (`self.config`) from the
    /// text that was segmented into it (see [`WordSpace`]).
    fn cmd_tokens(&self, seg: &SegmentedCommand) -> CommandTokens {
        let mut tokens = CommandTokens::from_segmented(&self.word_space.map(), self.config, seg);
        if self.readonly_authored_body.is_read_only() {
            if let Some(scope) = &self.authored_source_scope {
                let mut scalar = None;
                if let Some(bindings) = &self.source_bindings
                    && let Some(mut declared) = bindings
                        .original_authored_declaration_source_tokens(&tokens, scope, self.registry)
                {
                    let original = declared.clone();
                    let retained = declared.source_binding.as_mut().and_then(|binding| {
                        binding.retain_original_authored_scalar_body_scope(
                            std::sync::Arc::clone(scope),
                            &original,
                        )
                    });
                    if retained.is_some()
                        && crate::registry_invocation::original_authored_scalar_invocation(
                            &declared,
                            self.registry,
                        )
                        .is_some()
                    {
                        scalar = Some(declared);
                    }
                }
                let original = tokens.clone();
                let _ = tokens
                    .source_binding
                    .get_or_insert_with(Default::default)
                    .retain_original_authored_source_body_scope(
                        std::sync::Arc::clone(scope),
                        &original,
                    );
                if let Some(declared) = scalar {
                    tokens = declared;
                }
            }
        } else if let Some(bindings) = &self.source_bindings {
            bindings.stamp_original_tokens(&mut tokens);
            if let (Some(body), Some(metadata)) = (
                self.original_body_word_source.as_ref(),
                self.invocation_metadata_context(),
            ) && let Some(input) = metadata.source_analysis_input()
            {
                let _ = body.stamp_original_tokens(self.registry, input, &mut tokens);
            }
        }
        if let Some(owner) = &self.native_source_children {
            let _ = owner.stamp_tokens(&mut tokens);
        }
        // Implementation contract: naming.consumer.hosted-source-taint-descriptor
        // docs/design/analysis/name-resolution-proofs/hosted-source-taint-descriptor.md
        // The invocation owns the original whole image. A retained body's
        // positioned lowering buffer can contain padding before this command;
        // those bytes are not the source owner of conditional hosted metadata.
        tokens.hosted_taint_context = tokens
            .source_binding
            .as_ref()
            .and_then(|binding| binding.invocation_site())
            .zip(self.dialect_context.as_ref())
            .and_then(|(site, context)| {
                crate::registry_invocation::HostedSourceTaintContext::from_lowering(
                    site.source.source_image().clone(),
                    self.config,
                    std::sync::Arc::clone(context),
                )
                .map(|owner| {
                    owner.retain_source_structure_input(
                        self.invocation_metadata_context()
                            .and_then(crate::registry_invocation::InvocationMetadataContext::source_analysis_input),
                    )
                })
            })
            .map(std::sync::Arc::new);
        tokens
    }

    /// Lower a complete source string to an IR module.
    pub fn lower(&mut self, source: &str) -> &Module {
        self.start_module();
        self.module.source =
            tcl_lexer::SourceImage::from_bytes(source.as_bytes(), self.source_channel);

        if self.source_point_entry.as_ref().is_some_and(|point| {
            self.target.is_bytecode()
                || self.module.source_entry.unknown_entry
                || self
                    .module
                    .source_entry
                    .metadata_context
                    .source_analysis_input()
                    != Some(point.input())
        }) && self.refuse_inherited_runtime_target(source)
        {
            return &self.module;
        }
        let namespace = self.runtime_target_namespace("");
        self.module.top_level_namespace.clone_from(&namespace);
        self.module.top_level =
            self.lower_script(source, &namespace, SourceSegmentationPurpose::WholeInput);

        self.retain_module_source_bindings();
        &self.module
    }

    /// Lower a runtime procedure body as this module's top-level script.
    ///
    /// This is the procedure-target counterpart of [`Self::lower`].  It uses
    /// the same fresh frame as a static `proc` body, while retaining all
    /// module-wide side effects (nested procedures, aliases, namespaces, OO
    /// definitions, and traces) for the bytecode backend.
    pub fn lower_procedure_target(&mut self, source: &str, namespace: &str) -> &Module {
        self.start_module();
        self.module.source =
            tcl_lexer::SourceImage::from_bytes(source.as_bytes(), self.source_channel);
        // The runtime ABI carries an unrooted constructed namespace key while
        // compiler IR records rooted constructed keys.  Convert at the shared
        // naming owner without re-parsing the key as a written Tcl word: a
        // leading `::` here may belong to a literal-colon namespace segment.
        if self.refuse_inherited_runtime_target(source) {
            return &self.module;
        }
        let namespace = self.runtime_target_namespace(namespace);
        self.module.top_level_namespace.clone_from(&namespace);
        // This module's top level *is* a procedure body, which the `::top`
        // name cannot convey to anything downstream (#2207).
        self.module.top_level_kind = crate::ir::TopLevelKind::ProcedureBody;
        self.module.source_entry.native_compilation.frame =
            tcl_registry::native_compilation::NativeCompilationFrame::ProcedureCode;
        self.module.top_level =
            self.in_procedure_frame(Some(IrulesExecutionContext::ProcedureBody), |lowerer| {
                lowerer.lower_script(source, &namespace, SourceSegmentationPurpose::NestedScript)
            });
        self.retain_module_source_bindings();
        &self.module
    }

    /// Lower a runtime script in an explicit constructed namespace.
    ///
    /// Unlike [`Self::lower_procedure_target`], this keeps script-frame
    /// variable and return semantics. Both target forms share the exact same
    /// namespace conversion and module provenance field.
    pub fn lower_script_target(&mut self, source: &str, namespace: &str) -> &Module {
        self.start_module();
        self.module.source =
            tcl_lexer::SourceImage::from_bytes(source.as_bytes(), self.source_channel);
        if self.refuse_inherited_runtime_target(source) {
            return &self.module;
        }
        let namespace = self.runtime_target_namespace(namespace);
        self.module.top_level_namespace.clone_from(&namespace);
        self.module.top_level =
            self.lower_script(source, &namespace, SourceSegmentationPurpose::WholeInput);
        self.retain_module_source_bindings();
        &self.module
    }
}

// Public API

/// Lower Tcl source to an IR module.
///
/// Lexes, parses expressions and resolves commands under `registry`'s own
/// profile; a profile-less registry abstains from resolution. Use
/// [`lower_to_ir_with_dialect`] to honour a document's dialect.
#[must_use]
pub fn lower_to_ir(source: &str, registry: &CommandRegistry) -> Module {
    // dialect-drift-ok: the test-only entry point — every production caller
    // goes through `lower_to_ir_with_config` / `_with_dialect` with the
    // document's own config. The command world fails closed on a lexer
    // config that differs from its profile's, so the config follows the
    // registry's profile.
    // Its IR is executed by hosts that register their own commands (a VM's
    // `enable_threads`), so the catalogue does not close its command table.
    let profile = registry.profile();
    lower_with(
        Lowerer::with_config(registry, tcl_lexer::LexerConfig::for_profile(profile))
            .with_dialect(profile)
            .with_host_extensible_command_table(),
        source,
    )
}

#[cfg(test)]
mod body_cache_eligible_tests;

/// Like [`lower_to_ir`] but with an explicit dialect
/// [`tcl_lexer::LexerConfig`], threaded into every body re-segmentation
/// so `{*}` expansion (off for Tcl 8.4 / iRules) and the iRules `}{`
/// ghost SEP are honoured.
///
/// Expressions are still parsed in the plain-Tcl grammar — a caller that
/// knows the dialect *name* should use [`lower_to_ir_with_dialect`], which
/// also gives `if` / `while` / `for` / `expr` the dialect's operator set.
#[must_use]
pub fn lower_to_ir_with_config(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Module {
    lower_with(Lowerer::with_config(registry, config), source)
}

/// Like [`lower_to_ir_with_config`] but also naming the document's analysis
/// `dialect` (`""` for plain Tcl), so every `if` / `while` / `for` condition
/// and every inlined `expr` is parsed with that dialect's operator set.
///
/// This is what makes an iRules word-operator condition — `if {$x contains
/// "cd"}` — reach the IR as a real
/// [`BinOp::Contains`](crate::expr_ast::BinOp::Contains) comparison instead of
/// an opaque [`ExprNode::Raw`](crate::expr_ast::ExprNode::Raw), which is the
/// only shape the constant folder (and the I230 constant-condition
/// diagnostic behind it) can evaluate.
#[must_use]
pub fn lower_to_ir_with_dialect(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Module {
    lower_with(
        Lowerer::with_config(registry, config).with_dialect(dialect),
        source,
    )
}

/// Lower `source` with an already-configured [`Lowerer`].
///
/// The seam a host uses when it has more to install than the fixed-argument
/// entry points above accept — the document's own
/// [`with_declared_commands`](Lowerer::with_declared_commands) surface, above
/// all. Every other `lower_to_ir_*` function is this one with a `Lowerer`
/// built for it.
#[must_use]
pub fn lower_to_ir_with(lowerer: Lowerer<'_>, source: &str) -> Module {
    lower_with(lowerer, source)
}

/// Like [`lower_to_ir`] but for the bytecode/VM compile path: constructs the
/// backend can't compile correctly (`try`, and a `foreach`/`lmap` directly
/// nesting another) are lowered to runtime-command barriers (see
/// [`Lowerer::for_bytecode`]). Analysis callers keep the structured IR via
/// [`lower_to_ir`].
#[must_use]
pub fn lower_to_ir_for_bytecode(source: &str, registry: &CommandRegistry) -> Module {
    let profile = registry.profile();
    lower_to_ir_for_bytecode_with_dialect(
        source,
        registry,
        tcl_lexer::LexerConfig::for_profile(profile),
        profile,
    )
}

/// Like [`lower_to_ir_for_bytecode`] but also naming the document's `dialect`
/// (`""` for plain Tcl) — the bytecode-path counterpart of
/// [`lower_to_ir_with_dialect`], so a dialect expression such as an iRules
/// word-operator condition compiles to its dedicated opcode rather than the
/// generic runtime-`expr` fallback.
#[must_use]
pub fn lower_to_ir_for_bytecode_with_dialect(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Module {
    let lowerer = || {
        Lowerer::with_config(registry, config)
            .with_dialect(dialect)
            .for_bytecode_backend()
    };
    let module = lower_with(lowerer(), source);
    if module.top_level.native_compilation_failure.is_none() || !sources_files_directly(dialect) {
        return module;
    }
    // 8.4 never compiles a sourced file whole: `Tcl_EvalEx` compiles each
    // command as it runs, so a command its compiler refuses fails where it
    // stands, after the commands before it, as a direct script's does.
    let mut module = lower_with(lowerer().evaluated_directly(), source);
    module.top_level_evaluation = tcl_runtime_api::CommandEvaluation::Direct;
    module
}

/// Whether `dialect`'s `source` hands a file to `Tcl_EvalEx`.
fn sources_files_directly(dialect: Option<&'static tcl_dialect::DialectProfile>) -> bool {
    dialect
        .and_then(tcl_dialect::DialectProfile::runtime_version)
        .is_some_and(|version| {
            tcl_registry::release_compilation::evaluates_script_directly(version, "source", 1)
        })
}

/// Like [`lower_to_ir_for_bytecode`] but with every inline/structured
/// lowering hook suppressed (see [`Lowerer::trace_visible`]) — every command
/// in `source` compiles to a plain runtime dispatch, so an execution trace
/// observes it. For recompiling a proc/script body once a step-capable
/// execution trace targets it; see
/// [`tcl_runtime_api::CompileService::compile_traced`].
#[must_use]
pub fn lower_to_ir_traced(source: &str, registry: &CommandRegistry) -> Module {
    let profile = registry.profile();
    lower_to_ir_traced_with_dialect(
        source,
        registry,
        tcl_lexer::LexerConfig::for_profile(profile),
        profile,
    )
}

/// Like [`lower_to_ir_traced`] but with an explicit dialect
/// [`tcl_lexer::LexerConfig`], so a version-pinned host's traced recompiles
/// parse under the same grammar as its ordinary compiles.
#[must_use]
pub fn lower_to_ir_traced_with_config(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> Module {
    lower_to_ir_traced_with_dialect(source, registry, config, None)
}

/// Like [`lower_to_ir_traced_with_config`] but also records the exact dialect
/// selected by the host.  Runtime compile services for a named profile must
/// use this entry point so the resulting bytecode artifact retains the
/// profile identity that its lexer, registry, and expression grammar used.
#[must_use]
pub fn lower_to_ir_traced_with_dialect(
    source: &str,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: Option<&'static tcl_dialect::DialectProfile>,
) -> Module {
    lower_with(
        Lowerer::with_config(registry, config)
            .with_dialect(dialect)
            .for_bytecode_backend()
            .trace_visible(),
        source,
    )
}

/// Return the first hard parse-error message in `source`, in source order, or
/// `None` if `source` parses cleanly.
///
/// A VM compile front-end (`CompileService::compile`) calls this so that a
/// malformed script becomes a catchable runtime error carrying the **exact**
/// C Tcl message (e.g. `extra characters after close-quote`), matching C Tcl's
/// deferral of a compile-time parse error to a runtime instruction. The lexer
/// otherwise recovers from these for the benefit of editor diagnostics, so the
/// IR / bytecode never sees them.
///
/// Lexes with the default (Tcl-8.5+) dialect config — the same one
/// [`lower_to_ir_for_bytecode`] uses.
#[must_use]
pub fn first_fatal_parse_error(source: &str) -> Option<String> {
    first_fatal_parse_error_with_config(source, tcl_lexer::LexerConfig::default())
}

/// Like [`first_fatal_parse_error`] but lexing under an explicit dialect
/// [`tcl_lexer::LexerConfig`], so a version-pinned host rejects exactly what
/// its emulated release rejects — under the Tcl 8.4 grammar `{*}{a b}` is a
/// hard `extra characters after close-brace`, which the default (8.5+) config
/// lexes as an ordinary expansion.
#[must_use]
pub fn first_fatal_parse_error_with_config(
    source: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    first_fatal_parse_cut(source, config).map(|cut| cut.message.to_owned())
}

/// The full [`tcl_lexer::ParseCut`] for `source` — the message *and* the
/// index of the top-level command it was found under.
///
/// The command index is what separates this from
/// [`first_fatal_parse_error_with_config`]: C runs every command before the
/// cut and only then raises, so a front-end that wants C's behaviour has to
/// know where the clean prefix ends. The message alone cannot say.
///
/// This is a thin re-export of [`tcl_lexer::first_parse_cut`] — the shared
/// owner — and exists so `tcl-compiler`'s consumers have one import for the
/// whole parse-error question rather than reaching past it into the lexer.
/// The lexer's flat **warning stream** cannot answer it: filtering that
/// stream by message and taking the lowest-offset hit disagrees with C
/// whenever the failure is nested — `list [sfx one] [list "oops]` warns
/// `missing close-bracket` where C reports `missing "` — and a welded
/// close-brace (`set y {a}b`) does not appear in the warning stream at all.
#[must_use]
pub fn first_fatal_parse_cut(
    source: &str,
    config: tcl_lexer::LexerConfig,
) -> Option<tcl_lexer::ParseCut> {
    tcl_lexer::first_parse_cut(source, config)
}

/// Segment `source` using `config`, retaining only complete commands before a
/// fatal command and the original structured cut, including its kind and term.
#[doc(hidden)]
pub fn command_at_time_script_with_config(
    source: &str,
    config: tcl_lexer::LexerConfig,
) -> Result<tcl_source::segment::SegmentedScript, tcl_lexer::ParseCutUnavailable> {
    command_at_time_script_image(&tcl_lexer::SourceImage::document(source), config)
}

/// Complete command prefix of an original source image. Unavailable lexical
/// ownership remains distinct from an authentic malformed command tail.
#[doc(hidden)]
pub fn command_at_time_script_image(
    source: &tcl_lexer::SourceImage,
    config: tcl_lexer::LexerConfig,
) -> Result<tcl_source::segment::SegmentedScript, tcl_lexer::ParseCutUnavailable> {
    tcl_source::segment::segment_script_image_with_offset_and_config(source, 0, config)
}

/// Drive a configured [`Lowerer`] to a finished [`Module`] (the shared tail of
/// the `lower_to_ir*` entry points).
fn lower_with(mut lowerer: Lowerer<'_>, source: &str) -> Module {
    lowerer.lower(source);
    lowerer.finish_module(source)
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_script_purpose_tests;
