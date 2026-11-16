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

//! iRules-specific static checks (non-taint).
//!
//! Hosts:
//!
//! * **IRULE3102** — `HTTP::path` / `HTTP::uri` / `HTTP::query` getters
//!   used without the `-normalized` option (URL-evasion exposure).
//! * **IRULE5002** — `drop` / `reject` / `discard` without a subsequent
//!   `event disable all` or `return` (other iRules continue executing).
//! * **IRULE5004** — `DNS::return` without a subsequent `return`
//!   (iRule processing continues after `DNS::return`).
//!
//! The cross-event diagnostics — IRULE1005 / IRULE1006 / IRULE1007 /
//! IRULE1008 (collect/release/payload pairing across `when` events),
//! IRULE1201 / IRULE1202 (HTTP-after-respond and multi-respond), and
//! IRULE4004 (per-request set hoistable to once-per-connection) — build
//! on the richer cross-event analysis in `connection_scope.rs`.

use std::collections::{HashMap, HashSet};
#[cfg(test)]
use std::sync::OnceLock;
use tcl_core_types::DiagCode;
use tcl_dialect::model::Family;

use tcl_lexer::Span;
use tcl_registry::events::{
    CollectionReleaseRequirement, DataCollectionAction, EventRegistry, PayloadCollectionRequirement,
};
use tcl_registry::side_effects::ConnectionSide;
#[cfg(test)]
use tcl_registry::side_effects::SideEffectTarget;
use tcl_registry::{CommandRegistry, Traits};

use crate::cfg_builder::build_cfg_with_registry_and_config;
use crate::compilation_unit::{CompilationUnit, FunctionUnit};
use crate::ir::{Script, Statement};
use crate::lowering::lower_to_ir_with_config;
use crate::sccp::cfg_order;
#[cfg(test)]
use crate::value_shapes::parse_command_substitution_with_config;
use tcl_dialect::model::SurfaceQuery;

/// Process-wide iRules command registry, used to derive the HTTP-flow command
/// sets below.  Built once (the data is static).
#[cfg(test)]
fn irules_registry() -> &'static CommandRegistry {
    tcl_registry::model::ingress::static_context_for("f5-irules").commands()
}

/// Commands that commit an HTTP response — derived from the registry's
/// `ResponseCommit` side-effect, so the bare iRules `redirect` is
/// included alongside `HTTP::respond` / `HTTP::redirect` without a
/// hardcoded list.
#[cfg(test)]
fn commits_response_commands() -> &'static HashSet<String> {
    static SET: OnceLock<HashSet<String>> = OnceLock::new();
    SET.get_or_init(|| {
        let reg = irules_registry();
        reg.command_names()
            .filter(|name| {
                reg.get(name).is_some_and(|spec| {
                    spec.side_effects
                        .lower_bound()
                        .iter()
                        .any(|h| h.target == SideEffectTarget::ResponseCommit && h.writes)
                })
            })
            .map(String::from)
            .collect()
    })
}

/// Whether `cmd` is a registered iRules getter that carries the
/// `-normalized` option in its [`tcl_registry::CommandSpec`].
///
/// Replaces the previous hardcoded `NORMALISED_FLAG_COMMANDS` table:
/// the registry's own option list is the single source of truth, so
/// adding `-normalized` to a new command's spec automatically
/// extends the IRULE3102 surface.
#[cfg(test)]
fn supports_normalized_flag(registry: &CommandRegistry, cmd: &str) -> bool {
    registry
        .get(cmd)
        .is_some_and(|spec| spec.options.iter().any(|opt| opt.name == "-normalized"))
}

/// How much a [`CodeFix`] changes the behaviour of the program it rewrites.
///
/// A fix's *description* says what it does; this says what it costs.  The
/// distinction matters because "Fix All Safe Issues" applies fixes
/// unattended, in bulk, without the author reading each one: only a fix
/// that is **provably** semantics-preserving for the instance it was built
/// from belongs there.  A whitelist of diagnostic *codes* cannot express
/// that, because the same code can produce an equivalent rewrite in one
/// place and a behaviour-changing one in another.
///
/// The classification is per **fix instance**, not per code.  An emitter
/// that can prove equivalence for some inputs and not others must classify
/// each fix it builds on its own evidence.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub enum FixSafety {
    /// The rewrite is provably equivalent for this instance: every input
    /// produces the same completion code, result, output, and side effects
    /// before and after.  Whitespace and delimiter repairs, and rewrites
    /// whose operands the emitter has proven inert, live here.
    ///
    /// **This is the only class "Fix All Safe Issues" applies.**
    SemanticsEquivalent,
    /// The rewrite deliberately removes a hazard, and in doing so can change
    /// what the program does.  Bracing an `expr` operand (W100) removes
    /// Tcl's second round of substitution — the point of the fix, and the
    /// reason a program relying on that substitution changes behaviour.
    /// Offered as its own named action with an explanation; never applied
    /// unattended.
    BehaviourHardening,
    /// The rewrite changes only the spelling a human reads — comment
    /// layout, a suppression directive, an alternative documented form with
    /// identical semantics that is nonetheless a matter of taste.  Safe to
    /// apply, but not a *correctness* fix, so bulk application is a style
    /// decision the author makes rather than one the server makes for them.
    StyleOnly,
    /// Equivalence is not established: the fix is a suggestion whose
    /// correctness depends on context the emitter could not check (inferred
    /// types, run-time values, traces, aliases, a rename the analyser cannot
    /// see).  The author must read it.  This is the **default** so a new
    /// emitter that has not thought about safety cannot silently opt its
    /// fixes into unattended bulk application.
    #[default]
    RequiresReview,
}

impl FixSafety {
    /// Whether a fix in this class may be applied unattended, in bulk, by
    /// "Fix All Safe Issues".
    ///
    /// The single place that decision is encoded — the server asks the fix,
    /// rather than keeping a second list of diagnostic codes that drifts
    /// from what the emitters actually prove.
    #[must_use]
    pub const fn is_bulk_applicable(self) -> bool {
        matches!(self, Self::SemanticsEquivalent)
    }

    /// A short label for the class, for diagnostics, tooling, and the
    /// `applied` list the fix-all command reports.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::SemanticsEquivalent => "semantics-equivalent",
            Self::BehaviourHardening => "behaviour-hardening",
            Self::StyleOnly => "style-only",
            Self::RequiresReview => "requires-review",
        }
    }
}

/// A suggested code-action fix — maps to an LSP `TextEdit`.
///
/// `span` is the **fix range** the edit applies to, *independent* of the
/// diagnostic's own span: an insertion is a zero-width span anchored at the
/// insertion point (e.g. the end of a `drop`), a replacement covers the text it
/// rewrites.  This is the lower-level twin of the analyser's `CodeFix`, which
/// re-exports this type (see `analyser::types`); it lives here because the
/// iRules-flow / compiler-checks layer is below the analyser.
///
/// `safety` records how much the rewrite changes behaviour — see
/// [`FixSafety`].  Prefer the classified constructors
/// ([`CodeFix::equivalent`], [`CodeFix::hardening`], [`CodeFix::style`],
/// [`CodeFix::review`]) over a struct literal: they make the classification
/// an explicit decision at every emit site.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct CodeFix {
    /// Source span the `new_text` replaces (zero-width for an insertion).
    pub span: Span,
    /// Replacement / inserted text.
    pub new_text: String,
    /// Human-readable description (`"Add 'event disable all' + 'return'"`).
    pub description: String,
    /// How much this rewrite changes the program's behaviour.
    pub safety: FixSafety,
}

impl CodeFix {
    /// A fix the emitter has proven equivalent for this instance — the only
    /// class "Fix All Safe Issues" applies.  See
    /// [`FixSafety::SemanticsEquivalent`].
    #[must_use]
    pub fn equivalent(
        span: Span,
        new_text: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self::classified(span, new_text, description, FixSafety::SemanticsEquivalent)
    }

    /// A fix that removes a hazard and may change behaviour in doing so.
    /// See [`FixSafety::BehaviourHardening`].
    #[must_use]
    pub fn hardening(
        span: Span,
        new_text: impl Into<String>,
        description: impl Into<String>,
    ) -> Self {
        Self::classified(span, new_text, description, FixSafety::BehaviourHardening)
    }

    /// A fix that changes only the spelling a human reads.  See
    /// [`FixSafety::StyleOnly`].
    #[must_use]
    pub fn style(span: Span, new_text: impl Into<String>, description: impl Into<String>) -> Self {
        Self::classified(span, new_text, description, FixSafety::StyleOnly)
    }

    /// A suggestion whose correctness the emitter could not establish.  See
    /// [`FixSafety::RequiresReview`].
    #[must_use]
    pub fn review(span: Span, new_text: impl Into<String>, description: impl Into<String>) -> Self {
        Self::classified(span, new_text, description, FixSafety::RequiresReview)
    }

    /// Shared constructor behind the four classified builders.
    #[must_use]
    pub fn classified(
        span: Span,
        new_text: impl Into<String>,
        description: impl Into<String>,
        safety: FixSafety,
    ) -> Self {
        Self {
            span,
            new_text: new_text.into(),
            description: description.into(),
            safety,
        }
    }
}

/// An IRULE3102 / iRules-check diagnostic emitted by
/// [`find_unnormalised_getter_warnings`].
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IrulesCheckWarning {
    /// Source span of the offending call.
    pub span: Span,
    /// Diagnostic code (`"IRULE3102"`).
    pub code: DiagCode,
    /// Formatted message.
    pub message: String,
    /// Optional replacement text (currently always `None`).
    pub replacement: Option<String>,
    /// Suggested fixes whose range is independent of `span` (insertions /
    /// out-of-span replacements).  Empty when the check supplies no fix.
    pub fixes: Vec<CodeFix>,
}

/// Return `true` when `args` suggests a *getter* invocation — no args,
/// or every arg starts with `-` (all flags, no positional value).
/// Resolving a getter form properly would need a form-kind model on the
/// command registry, which it does not have yet, so this is a
/// conservative approximation. `HTTP::path /foo` (setter) is excluded
/// because `/foo` is a non-flag first arg.
fn is_getter_form(args: &[String]) -> bool {
    args.iter().all(|a| a.starts_with('-'))
}

/// Format the IRULE3102 message for `cmd`.
pub(crate) fn format_message(cmd: &str) -> String {
    format!(
        "Use '{cmd} -normalized' for canonicalized request data; \
         non-normalized values may allow URL evasion patterns."
    )
}

/// A selected option vocabulary declares normalization but its exact original
/// flag-form operands do not select that option.
pub(crate) fn is_unnormalised_getter(
    options: &tcl_registry::abbrev::KeywordTable<'_>,
    args: &[String],
) -> bool {
    if !options.names().any(|name| name == "-normalized") || !is_getter_form(args) {
        return false;
    }
    !args.iter().any(|argument| {
        options.resolve(argument) == tcl_registry::abbrev::KeywordMatch::Unique("-normalized")
    })
}

/// Whether this point stands on the iRules surface.
///
/// Every check in this module is iRules-only and asked the question inline;
/// naming it once keeps the five gates reading the same way and keeps the
/// family comparison in one place.
fn is_irules(dialect: Option<SurfaceQuery<'_>>) -> bool {
    dialect.is_some_and(|q| {
        q.core
            .nearest()
            .is_some_and(|(family, _)| family == Family::F5Irules)
    })
}

/// Find IRULE3102 warnings across every function in `cu`.
///
/// Gated: returns an empty vector unless the point stands on the iRules
/// surface.
///
/// Each invocation retains its original source binding. Nested commands use
/// the complete argument substitution owner; braced data supplies no calls.
#[must_use]
pub fn find_unnormalised_getter_warnings(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<SurfaceQuery<'_>>,
) -> Vec<IrulesCheckWarning> {
    if !is_irules(dialect) {
        return Vec::new();
    }
    let Some(source) = IrulesSourceQuery::capture(cu, registry) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for function in cu.analysable_functions() {
        let Some(config) = source.function_config(function) else {
            continue;
        };
        for block in cfg_order(&function.cfg) {
            if !function.sccp.executable_blocks.contains(&block) {
                continue;
            }
            let Some(block) = function.cfg.blocks.get(&block) else {
                continue;
            };
            for statement in &block.statements {
                let Some(tokens) =
                    crate::ir::CommandBindingSites::unanimous_statement_source_tokens(
                        function.cfg.command_binding_sites.iter(),
                        statement,
                    )
                    .filter(|tokens| tokens.synthetic.is_none())
                else {
                    continue;
                };
                let Some(warnings) = original_getter_source_warnings(tokens, &source, config)
                else {
                    continue;
                };
                out.extend(warnings.into_iter().map(|mut warning| {
                    warning.span = function.abs_span(warning.span);
                    warning
                }));
            }
        }
    }
    out.sort_by(|a, b| {
        (a.span.start(), a.span.end(), &a.message).cmp(&(b.span.start(), b.span.end(), &b.message))
    });
    out.dedup();
    out
}

fn original_getter_source_warnings(
    parent: &crate::ir::CommandTokens,
    source: &IrulesSourceQuery<'_>,
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<IrulesCheckWarning>> {
    parent
        .source_binding
        .as_ref()?
        .original_lexer_config_for_tokens(parent)?;
    let children = source.argument_calls(parent, config)?;
    let mut warnings = Vec::new();
    for tokens in std::iter::once(parent).chain(
        children
            .iter()
            .map(|child| child.tokens.as_ref())
            .collect::<Option<Vec<_>>>()?,
    ) {
        let assistance =
            crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
                source.registry,
                source.metadata(),
                tokens,
            )?;
        for candidate in &assistance.candidates {
            let arguments = candidate
                .source_argument_words(source.registry, source.metadata(), tokens)?
                .iter()
                .map(|argument| {
                    Some(
                        std::str::from_utf8(argument.literal_bytes()?)
                            .ok()?
                            .to_owned(),
                    )
                })
                .collect::<Option<Vec<_>>>();
            let Some(arguments) = arguments else {
                continue;
            };
            if is_unnormalised_getter(&candidate.option_keywords, &arguments) {
                warnings.push(IrulesCheckWarning {
                    span: Span::new(tokens.argv.first()?.start(), tokens.argv.last()?.end()),
                    code: DiagCode::Irule3102,
                    message: format_message(&candidate.command),
                    replacement: None,
                    fixes: Vec::new(),
                });
            }
        }
    }
    Some(warnings)
}

/// One source-owner query shared by secondary iRules syntax consumers.
/// Supplied input and positively tagged standalone analysis are distinct routes.
struct IrulesSourceQuery<'a> {
    unit: &'a CompilationUnit,
    registry: &'a CommandRegistry,
    standalone: bool,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'a>>,
}

struct PairingHandlerInput<'a> {
    id: &'a crate::ir::ProcedureId,
    function: &'a FunctionUnit,
    procedure: &'a crate::ir::Procedure,
    commands: &'a [tcl_lexer::NativeScriptCommandWords],
    metadata: crate::registry_invocation::InvocationMetadataContext<'a>,
}

struct PairingHandlerOutput<'a> {
    original: &'a mut Vec<Vec<tcl_lexer::NativeWord>>,
    handler_flow_commands: &'a mut Vec<OriginalIrulesFlowCommand>,
    schemas: &'a mut Vec<crate::registry_invocation::RegistryInvocationShape>,
    completions: &'a mut Vec<tcl_registry::registry::InvocationCompletion>,
}

impl<'a> IrulesSourceQuery<'a> {
    fn capture(unit: &'a CompilationUnit, registry: &'a CommandRegistry) -> Option<Self> {
        let module = &unit.ir_module;
        if module.source != tcl_lexer::SourceImage::document(&unit.source) {
            return None;
        }
        let standalone = module.source_entry.metadata_context.is_standalone()
            && module.source_metadata_input.is_none();
        let metadata = if standalone {
            None
        } else {
            let metadata = crate::registry_invocation::InvocationMetadataContext::for_module(
                registry, module,
            )?;
            let input = module.source_metadata_input.as_ref()?;
            if input.lexer_config() != module.lexer_config {
                return None;
            }
            crate::realm::document_realm_bindings_for_module(module, registry)?;
            Some(metadata)
        };
        Some(Self {
            unit,
            registry,
            standalone,
            metadata,
        })
    }

    fn function_config(&self, function: &FunctionUnit) -> Option<tcl_lexer::LexerConfig> {
        irules_function_source_config(
            &self.unit.ir_module,
            function,
            self.registry,
            self.standalone,
        )
    }

    /// Distinguish ordinary functions from an unavailable original event.
    /// The actual map entry and lowering descriptor own identity; report names
    /// select neither source bodies nor conditional event frame policy.
    fn event(&self, function: &FunctionUnit) -> Option<OriginalEventPresence<'_>> {
        let module = &self.unit.ir_module;
        let Some((key, owned)) = self
            .unit
            .procedures
            .iter()
            .find(|(_, owned)| std::ptr::eq(*owned, function))
        else {
            return std::ptr::eq(function, std::ptr::from_ref(&self.unit.top_level))
                .then_some(OriginalEventPresence::Ordinary);
        };
        let Some(expected) = module.irules_event_bodies.get(key) else {
            return owned
                .irules_event_body
                .is_none()
                .then_some(OriginalEventPresence::Ordinary);
        };
        self.function_config(owned)?;
        let body = owned.irules_event_body.as_ref()?;
        let procedure = module.procedures.get(key)?;
        if !std::sync::Arc::ptr_eq(body, expected)
            || !body.owns_procedure(
                procedure,
                &module.source,
                module.lexer_config,
                self.registry,
            )
        {
            return None;
        }
        Some(OriginalEventPresence::Event(OriginalEvent {
            body,
            procedure,
        }))
    }

    fn argument_calls(
        &self,
        parent: &crate::ir::CommandTokens,
        config: tcl_lexer::LexerConfig,
    ) -> Option<Vec<crate::word_subst::LiftedCall>> {
        if let Some(metadata) = self.metadata {
            crate::word_subst::checked_original_argument_lifted_calls(
                parent,
                self.registry,
                metadata,
            )
        } else if self.standalone {
            crate::word_subst::checked_argument_lifted_calls(
                parent,
                parent.native_lexer_config(config),
            )
        } else {
            None
        }
    }

    fn original_argv(
        &self,
        tokens: &crate::ir::CommandTokens,
    ) -> Option<Vec<tcl_lexer::NativeWord>> {
        let metadata = self.metadata?;
        let binding = tokens.source_binding.as_ref()?;
        binding.original_argument_metadata_for_input(
            tokens,
            metadata.source_analysis_input()?,
            self.registry,
        )?;
        let config = binding.original_argument_lexer_config(tokens)?;
        let image = binding.original_argument_source_image(tokens)?;
        crate::registry_invocation::original_native_compiler_words(
            image,
            tokens.words(),
            tokens.words().first()?.source().span.start(),
            config,
        )
    }

    fn pairing_declaration_words(
        &self,
        metadata: crate::registry_invocation::InvocationMetadataContext<'_>,
        declaration: &crate::ir::CommandTokens,
        event: &OriginalEvent<'_>,
        context: &tcl_registry::model::ContextRegistry,
    ) -> Option<crate::registry_invocation::source_structure::OriginalRegistryWords> {
        let module = &self.unit.ir_module;
        let declaration_metadata =
            crate::registry_invocation::original_hosted_source_registry_metadata(
                self.registry,
                metadata,
                declaration,
            )?;
        let event_word =
            event
                .body
                .event_word(&module.source, module.lexer_config, self.registry)?;
        if !declaration_metadata
            .shape()
            .possible_traits()
            .contains(Traits::IS_EVENT_HANDLER)
            || !declaration_metadata
                .shape()
                .original_words()
                .contains(event_word)
            || declaration_metadata
                .authored_barriers()
                .has_nominal_command_name_hazard()
        {
            return None;
        }
        crate::registry_invocation::source_structure::original_vendor_registry_words(
            context,
            declaration,
            declaration_metadata,
        )
    }

    fn retain_pairing_handler_commands(
        &self,
        state: &CollectFlowState,
        input: &PairingHandlerInput<'_>,
        output: &mut PairingHandlerOutput<'_>,
    ) -> Option<()> {
        for statement in &input.procedure.body.statements {
            if matches!(
                statement.original_argv_inventory_disposition(),
                crate::ir::OriginalArgvInventoryDisposition::EffectOnly(_)
            ) {
                continue;
            }
            if !statement.child_scripts().is_empty() {
                // A statement with bodies is walked path by path by the flow
                // advice. The census keeps its written command so the handler
                // stays fully accounted for, and adds nothing else.
                let tokens = input
                    .procedure
                    .body
                    .retained_source_tokens_for_statement(statement)?;
                let vector = self.original_argv(tokens)?;
                if !input.commands.iter().any(|command| command.words == vector) {
                    return None;
                }
                output.original.push(vector);
                continue;
            }
            let tokens = input
                .procedure
                .body
                .retained_source_tokens_for_statement(statement)?;
            let vector = self.original_argv(tokens)?;
            if !input.commands.iter().any(|command| command.words == vector)
                || !state
                    .source_argv
                    .iter()
                    .any(|(owner, words)| owner == input.id && words == &vector)
            {
                return None;
            }
            output.original.push(vector);
            let children = self.argument_calls(tokens, self.function_config(input.function)?)?;
            for token in children
                .iter()
                .map(|child| child.tokens.as_ref())
                .collect::<Option<Vec<_>>>()?
                .into_iter()
                .chain(std::iter::once(tokens))
            {
                let vector = self.original_argv(token)?;
                if !state
                    .source_argv
                    .iter()
                    .any(|(owner, words)| owner == input.id && words == &vector)
                {
                    return None;
                }
                let possible = crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(self.registry, Some(input.metadata), token)?;
                if possible.candidates.is_empty() {
                    return None;
                }
                if let OriginalIrulesFlowPresence::Relevant(command) =
                    possible_irule_flow_command(self.registry, token, &possible)?
                {
                    output.handler_flow_commands.push(command);
                }
                for candidate in &possible.candidates {
                    let completion = candidate.with_possible_source_schema(
                        self.registry,
                        input.metadata,
                        token,
                        leaf_handler_source_completion,
                    )?;
                    output.schemas.push(candidate.clone());
                    output.completions.push(completion);
                }
            }
        }
        Some(())
    }

    fn conditional_pairing_advice<'query>(
        &'query self,
        state: &'query CollectFlowState,
    ) -> Option<ConditionalSourcePairingAdvice<'query, 'a>> {
        let metadata = self.metadata?;
        let input = metadata.source_analysis_input()?;
        input.has_hosted_source_name_context().then_some(())?;
        let context = input.borrowed_context_registry();
        let module = &self.unit.ir_module;
        let root = tcl_lexer::native_script_words_in(
            module.source.clone(),
            Span::new(0, u32::try_from(module.source.len()).ok()?),
            module.lexer_config,
        )
        .ok()?;
        if root.fatal_tail.is_some() || root.commands.len() != module.irules_event_bodies.len() {
            return None;
        }
        let mut declarations_seen = Vec::new();
        let mut handlers = Vec::new();
        let mut schemas = Vec::new();
        let mut completions = Vec::new();
        let mut flow_commands = Vec::new();
        for id in module.irules_event_bodies.keys() {
            let function = self.unit.procedures.get(id)?;
            let event = self.event(function)?.into_event()?;
            let mut declarations = module
                .top_level
                .statements
                .iter()
                .filter(|statement| statement.span() == event.procedure.span);
            let declaration = declarations.next()?;
            if declarations.next().is_some() {
                return None;
            }
            let declaration = module
                .top_level
                .retained_source_tokens_for_statement(declaration)?;
            let declaration_vector = self.original_argv(declaration)?;
            if !root
                .commands
                .iter()
                .any(|command| command.words == declaration_vector)
                || declarations_seen.contains(&declaration_vector)
            {
                return None;
            }
            declarations_seen.push(declaration_vector.clone());
            let words = self.pairing_declaration_words(metadata, declaration, &event, context)?;
            let mut bodies = words.source_script_bodies_for_mutation_coverage(context)?;
            if bodies.len() != 1 {
                return None;
            }
            let body = bodies.pop()?;
            for word in &declaration_vector {
                if word != body.original_container() {
                    tcl_syntax::word_rules::original_static_word_unicode_value(word)?;
                }
            }
            let commands = body.original_commands(context)?;
            let mut original = Vec::new();
            let mut handler_flow_commands = Vec::new();
            self.retain_pairing_handler_commands(
                state,
                &PairingHandlerInput {
                    id,
                    function,
                    procedure: event.procedure,
                    commands: &commands,
                    metadata,
                },
                &mut PairingHandlerOutput {
                    original: &mut original,
                    handler_flow_commands: &mut handler_flow_commands,
                    schemas: &mut schemas,
                    completions: &mut completions,
                },
            )?;
            if commands.len() != original.len()
                || commands
                    .iter()
                    .any(|command| !original.contains(&command.words))
            {
                return None;
            }
            handlers.push((*id, body));
            flow_commands.push((*id, handler_flow_commands));
        }
        if root
            .commands
            .iter()
            .any(|command| !declarations_seen.contains(&command.words))
        {
            return None;
        }
        (!handlers.is_empty()).then_some(ConditionalSourcePairingAdvice {
            _source: self,
            handlers,
            schemas,
            completions,
            flow_commands,
            _obligations: &state.obligations,
        })
    }

    fn metadata(&self) -> Option<crate::registry_invocation::InvocationMetadataContext<'_>> {
        self.metadata
    }
}

fn leaf_handler_source_completion(
    schema: &tcl_registry::ResolvedInvocation<'_, '_>,
) -> Option<tcl_registry::registry::InvocationCompletion> {
    let (roles, complete) = schema.authored_source_argument_roles();
    if !complete
        || roles
            .iter()
            .any(|(_, role)| *role == tcl_registry::ArgRole::LambdaLiteral)
    {
        return None;
    }
    let scripts = schema.authored_source_script_arguments()?;
    let prefixes = schema.authored_source_command_prefix_arguments()?;
    let expressions = schema.authored_source_expression_arguments()?;
    (scripts.is_empty() && prefixes.is_empty() && expressions.arguments.is_empty())
        .then(|| schema.authored_source_invocation_completion())
}

/// Closed leaf-source census under its original handler and schema premises.
/// Runtime lookup, observers, activation and completion remain obligations.
struct ConditionalSourcePairingAdvice<'query, 'unit> {
    _source: &'query IrulesSourceQuery<'unit>,
    handlers: Vec<(
        crate::ir::ProcedureId,
        crate::registry_invocation::OriginalSourceScriptBody,
    )>,
    schemas: Vec<crate::registry_invocation::RegistryInvocationShape>,
    completions: Vec<tcl_registry::registry::InvocationCompletion>,
    flow_commands: Vec<(crate::ir::ProcedureId, Vec<OriginalIrulesFlowCommand>)>,
    _obligations: &'query HashSet<CollectionSourceObligationKind>,
}

fn conditional_source_pairing_message(
    _advice: &ConditionalSourcePairingAdvice<'_, '_>,
    source_pairing: &str,
) -> String {
    format!("Conditional source pairing (runtime applicability unproved): {source_pairing}")
}

/// Conditional pairing under a closed retained handler-source census.
/// Its immutable owners and unresolved runtime premises remain separate from
/// actual collection, absence, handler completion and edit authority.
pub struct ConditionalCollectionSourcePairingAdvice<'unit> {
    owner: ConditionalIruleSourceOwner<'unit>,
    pairings: Vec<ConditionalCollectionSourcePairing>,
}

#[derive(Clone)]
struct ConditionalIruleSourceOwner<'unit> {
    unit: &'unit CompilationUnit,
    registry: &'unit CommandRegistry,
    _handlers: Vec<(
        crate::ir::ProcedureId,
        crate::registry_invocation::OriginalSourceScriptBody,
    )>,
    _schemas: Vec<crate::registry_invocation::RegistryInvocationShape>,
    obligations: HashSet<CollectionSourceObligationKind>,
    _completions: Vec<tcl_registry::registry::InvocationCompletion>,
    flow_commands: Vec<(crate::ir::ProcedureId, Vec<OriginalIrulesFlowCommand>)>,
}

impl<'unit> ConditionalCollectionSourcePairingAdvice<'unit> {
    /// Source-only pairing candidates; these supply no runtime outcome or fix.
    #[must_use]
    pub fn pairings(&self) -> &[ConditionalCollectionSourcePairing] {
        &self.pairings
    }

    pub(crate) fn diagnostic_source_owner_for(
        &self,
        pairing: &ConditionalCollectionSourcePairing,
    ) -> Option<(&CompilationUnit, &CommandRegistry)> {
        self.pairings
            .iter()
            .any(|current| std::ptr::eq(current, pairing))
            .then_some((self.owner.unit, self.owner.registry))
    }

    /// Independent premises that every conditional source pairing retains.
    pub fn obligations(&self) -> impl Iterator<Item = &CollectionSourceObligationKind> {
        self.owner.obligations.iter()
    }

    /// Project source flow from this same complete census and schema receipts.
    /// No second source inventory or command resolution is performed.
    #[must_use]
    pub fn conditional_source_flow(&self) -> Option<ConditionalIruleSourceFlowAdvice<'unit>> {
        conditional_irule_source_flow_advice(&self.owner)
    }
}

/// Conditional flow among complete original leaf-handler source commands.
/// Source order and descriptor shapes establish no executed continuation.
pub struct ConditionalIruleSourceFlowAdvice<'unit> {
    owner: ConditionalIruleSourceOwner<'unit>,
    findings: Vec<ConditionalIruleSourceFlowFinding>,
}

impl ConditionalIruleSourceFlowAdvice<'_> {
    /// Source-order hazards with runtime applicability explicitly unresolved.
    #[must_use]
    pub fn findings(&self) -> &[ConditionalIruleSourceFlowFinding] {
        &self.findings
    }

    /// Independent premises, including normal continuation, remain unproved.
    pub fn obligations(&self) -> impl Iterator<Item = &CollectionSourceObligationKind> {
        self.owner.obligations.iter()
    }

    pub(crate) fn diagnostic_source_owner_for(
        &self,
        finding: &ConditionalIruleSourceFlowFinding,
    ) -> Option<(&CompilationUnit, &CommandRegistry)> {
        self.findings
            .iter()
            .any(|current| std::ptr::eq(current, finding))
            .then_some((self.owner.unit, self.owner.registry))
    }
}

/// A possible source-order hazard, without a reached runtime path or fix.
pub struct ConditionalIruleSourceFlowFinding {
    warning: IrulesCheckWarning,
}

impl ConditionalIruleSourceFlowFinding {
    /// Original source advice anchor.
    #[must_use]
    pub fn span(&self) -> Span {
        self.warning.span
    }

    /// Diagnostic family; it supplies no action subject.
    #[must_use]
    pub fn code(&self) -> DiagCode {
        self.warning.code
    }

    /// Conditional source scope and unresolved runtime continuation.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.warning.message
    }
}

/// One possible source-pairing hazard, without an observed runtime outcome.
pub struct ConditionalCollectionSourcePairing {
    warning: IrulesCheckWarning,
}

impl ConditionalCollectionSourcePairing {
    /// Original source advice anchor, not an evaluated operation.
    #[must_use]
    pub fn span(&self) -> Span {
        self.warning.span
    }

    /// Diagnostic family for this conditional source advice.
    #[must_use]
    pub fn code(&self) -> DiagCode {
        self.warning.code
    }

    /// The explicit source scope and unresolved runtime applicability.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.warning.message
    }
}

/// Original conditional source event, without an executing event or worker.
struct OriginalEvent<'a> {
    body: &'a crate::ir::SourceIrulesEventBody,
    procedure: &'a crate::ir::Procedure,
}

enum OriginalEventPresence<'a> {
    Ordinary,
    Event(OriginalEvent<'a>),
}

impl<'a> OriginalEventPresence<'a> {
    fn into_event(self) -> Option<OriginalEvent<'a>> {
        match self {
            Self::Ordinary => None,
            Self::Event(event) => Some(event),
        }
    }
}

/// Select the actual function's complete config before choosing parser purpose.
/// Standalone assistance requires explicit standalone Module and CFG owners.
fn irules_function_source_config(
    module: &crate::ir::Module,
    function: &FunctionUnit,
    registry: &CommandRegistry,
    standalone_module: bool,
) -> Option<tcl_lexer::LexerConfig> {
    if standalone_module {
        if !module.source_entry.metadata_context.is_standalone()
            || module.source_metadata_input.is_some()
            || !function.cfg.metadata_context.is_standalone()
            || function.source_metadata_input().is_some()
        {
            return None;
        }
    } else {
        function.invocation_metadata_context_for_module(registry, module)?;
    }
    Some(function.source_lexer_config())
}

#[cfg(test)]
fn unnormalised_getter_source_config(
    module: &crate::ir::Module,
    function: &FunctionUnit,
    registry: &CommandRegistry,
    standalone_module: bool,
) -> Option<tcl_lexer::LexerConfig> {
    irules_function_source_config(module, function, registry, standalone_module)
}

// IRULE5002 + IRULE5004 — unguarded drop / DNS::return
//
// Path-sensitive: each original event body is walked with each branch
// of every `if` / `switch` / `try` tracked separately, emitting at
// branch joins where the drop survived unguarded.

/// One drop / DNS-return flow fact along a path.  Dedup keys on the
/// `(dropped, dns_returned)` pair only, so the first unguarded occurrence
/// on any surviving path wins.
#[derive(Clone, Default)]
struct DropFlowState {
    dropped: bool,
    drop_cmd: String,
    drop_at: Option<Span>,
    dns_returned: bool,
    dns_at: Option<Span>,
}

/// Deduplicate drop states by `(dropped, dns_returned)`, keeping the first.
fn dedupe_drop_states(states: Vec<DropFlowState>) -> Vec<DropFlowState> {
    let mut seen: HashSet<(bool, bool)> = HashSet::new();
    let mut out = Vec::new();
    for s in states {
        if seen.insert((s.dropped, s.dns_returned)) {
            out.push(s);
        }
    }
    out
}

#[derive(Clone)]
struct OriginalIrulesPacketActions {
    drops_connection: bool,
    returns_dns_packet: bool,
}

#[derive(Clone)]
struct OriginalIrulesFlowCommand {
    command: String,
    span: Span,
    packet_actions: OriginalIrulesPacketActions,
    disables_all_events: bool,
    may_commit_response: bool,
    requires_http_context: bool,
}

struct OriginalIruleFlowInventory {
    commands: Vec<OriginalIrulesFlowCommand>,
    obligations: HashSet<CollectionSourceObligationKind>,
}

/// Actual source owners retain typed-lowering carriers and all argv children.
/// Missing source is distinct from an admitted command with no flow metadata.
fn original_irules_flow_commands(
    script: &Script,
    statement: &Statement,
    source: &IrulesSourceQuery<'_>,
    config: tcl_lexer::LexerConfig,
) -> Option<Vec<OriginalIrulesFlowCommand>> {
    let inventory = original_irules_flow_inventory(script, statement, source, config)?;
    inventory
        .obligations
        .is_empty()
        .then_some(inventory.commands)
}

fn original_irules_flow_inventory(
    script: &Script,
    statement: &Statement,
    source: &IrulesSourceQuery<'_>,
    config: tcl_lexer::LexerConfig,
) -> Option<OriginalIruleFlowInventory> {
    let mut obligations = HashSet::new();
    if let crate::ir::OriginalArgvInventoryDisposition::EffectOnly(marker) =
        statement.original_argv_inventory_disposition()
    {
        obligations.insert(CollectionSourceObligationKind::EffectOnly(marker));
        return Some(OriginalIruleFlowInventory {
            commands: Vec::new(),
            obligations,
        });
    }
    let parent = script.retained_source_tokens_for_statement(statement)?;
    parent
        .source_binding
        .as_ref()?
        .original_lexer_config_for_tokens(parent)?;
    let children = source.argument_calls(parent, config)?;
    let children = children
        .iter()
        .map(|child| child.tokens.as_ref())
        .collect::<Option<Vec<_>>>()?;
    let mut calls = Vec::new();
    for tokens in children.into_iter().chain(std::iter::once(parent)) {
        let possible =
            crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
                source.registry,
                source.metadata(),
                tokens,
            )?;
        if possible.unknown_residual {
            obligations.insert(CollectionSourceObligationKind::UnknownTarget);
        }
        if possible.may_be_absent {
            obligations.insert(CollectionSourceObligationKind::MayBeAbsent);
        }
        if let OriginalIrulesFlowPresence::Relevant(command) =
            possible_irule_flow_command(source.registry, tokens, &possible)?
        {
            calls.push(command);
        }
    }
    Some(OriginalIruleFlowInventory {
        commands: calls,
        obligations,
    })
}

enum OriginalIrulesFlowPresence {
    Irrelevant,
    Relevant(OriginalIrulesFlowCommand),
}

fn possible_irule_flow_command(
    registry: &CommandRegistry,
    tokens: &crate::ir::CommandTokens,
    possible: &crate::registry_invocation::RegistryInvocationAssistance,
) -> Option<OriginalIrulesFlowPresence> {
    use tcl_registry::events::IrulesSourceFlowAction;
    let mut drops_connection = false;
    let mut returns_dns_packet = false;
    let mut may_commit_response = false;
    let mut requires_http_context = false;
    for candidate in &possible.candidates {
        drops_connection |=
            candidate.irules_source_flow_action == Some(IrulesSourceFlowAction::DropsConnection);
        returns_dns_packet |=
            candidate.irules_source_flow_action == Some(IrulesSourceFlowAction::ReturnsDnsPacket);
        may_commit_response |= candidate.source_may_commit_response;
        // The transaction requirement is declared on the command. A selected
        // subcommand (`HTTP::header insert`) resolves to its own traits, which
        // do not repeat it.
        requires_http_context |= candidate
            .possible_traits
            .contains(Traits::REQUIRES_HTTP_CONTEXT)
            || registry
                .get(&candidate.command)
                .is_some_and(|spec| spec.traits.contains(Traits::REQUIRES_HTTP_CONTEXT));
    }
    // Conditional advice keeps lookup closure as an obligation on both sides:
    // a guard counts by its candidates exactly as a drop does.
    let disables_all_events = !possible.candidates.is_empty()
        && possible.candidates.iter().all(|candidate| {
            candidate.irules_source_flow_action == Some(IrulesSourceFlowAction::DisablesAllEvents)
        });
    if !(drops_connection
        || returns_dns_packet
        || may_commit_response
        || requires_http_context
        || disables_all_events)
    {
        return Some(OriginalIrulesFlowPresence::Irrelevant);
    }
    let command = tokens.argv_texts.first()?.clone();
    Some(OriginalIrulesFlowPresence::Relevant(
        OriginalIrulesFlowCommand {
            command,
            span: Span::new(tokens.argv.first()?.start(), tokens.argv.last()?.end()),
            packet_actions: OriginalIrulesPacketActions {
                drops_connection,
                returns_dns_packet,
            },
            disables_all_events,
            may_commit_response,
            requires_http_context,
        },
    ))
}

fn drop_flow_command(state: &DropFlowState, command: &OriginalIrulesFlowCommand) -> DropFlowState {
    let mut next = state.clone();
    if command.disables_all_events {
        next.dropped = false;
        next.drop_cmd.clear();
        next.drop_at = None;
    }
    if command.packet_actions.drops_connection {
        next.dropped = true;
        next.drop_cmd.clone_from(&command.command);
        next.drop_at = Some(command.span);
    }
    if command.packet_actions.returns_dns_packet {
        next.dns_returned = true;
        next.dns_at = Some(command.span);
    }
    next
}

/// Path-sensitive IRULE5002 / IRULE5004 — an unguarded `drop`/`reject`/
/// `discard` (no following `event disable all` / `return`) or a `DNS::return`
/// without a following `return`, on at least one path through an original event
/// body.
#[must_use]
pub fn find_unguarded_drop_warnings(
    cu: &CompilationUnit,
    dialect: Option<SurfaceQuery<'_>>,
) -> Vec<IrulesCheckWarning> {
    let mut out: Vec<IrulesCheckWarning> = Vec::new();
    if !is_irules(dialect) {
        return out;
    }
    let Some(source) = IrulesSourceQuery::capture(cu, cu.ir_module.resolved_registry()) else {
        return out;
    };
    // Honour the complexity guard but walk the structured
    // IR body of each analysable original event handler.
    for fu in cu.analysable_functions() {
        let Some(event) = source.event(fu) else {
            return Vec::new();
        };
        let OriginalEventPresence::Event(event) = event else {
            continue;
        };
        let proc = event.procedure;
        let Some(config) = source.function_config(fu) else {
            return Vec::new();
        };
        let missing_source = std::cell::Cell::new(false);
        let leaf = |state: &DropFlowState,
                    script: &Script,
                    statement: &Statement,
                    _out: &mut Vec<IrulesCheckWarning>| {
            let Some(commands) = original_irules_flow_commands(script, statement, &source, config)
            else {
                missing_source.set(true);
                return state.clone();
            };
            commands.iter().fold(state.clone(), |current, command| {
                drop_flow_command(&current, command)
            })
        };
        let unreached = unreached_statements(fu);
        let finals = walk_flow(
            &proc.body,
            &[DropFlowState::default()],
            &mut FlowDispatch {
                registry: cu.ir_module.resolved_registry(),
                out: &mut out,
                leaf: &leaf,
                dedupe: &dedupe_drop_states,
                unreached: &unreached,
                terminates: None,
            },
            None,
        );
        if missing_source.get() {
            return Vec::new();
        }
        for st in finals {
            if st.dropped
                && let Some(span) = st.drop_at
            {
                out.push(IrulesCheckWarning {
                    span,
                    code: DiagCode::Irule5002,
                    message: format!(
                        "'{}' without 'event disable all' or 'return' — other iRules and later \
                         priorities in this event will still execute, which may cause TCL errors.",
                        st.drop_cmd,
                    ),
                    replacement: None,
                    // Insert `event disable all` + `return` right after the drop
                    // (a zero-width edit at the drop command's end).
                    fixes: vec![CodeFix {
                        span: Span::new(span.end(), span.end()),
                        new_text: "\n    event disable all\n    return".to_owned(),
                        description: "Add 'event disable all' + 'return'".to_owned(),
                        // IRULE5002: adding `event disable all` + `return` stops later
                        // iRules and priorities running — the fix, and a control-flow change.
                        safety: crate::irules_checks::FixSafety::BehaviourHardening,
                    }],
                });
            }
            if st.dns_returned
                && let Some(span) = st.dns_at
            {
                out.push(IrulesCheckWarning {
                    span,
                    code: DiagCode::Irule5004,
                    message: "'DNS::return' must be followed by 'return' to stop iRule processing."
                        .to_owned(),
                    replacement: None,
                    // Insert `return` right after `DNS::return`.
                    fixes: vec![CodeFix {
                        span: Span::new(span.end(), span.end()),
                        new_text: "\n    return".to_owned(),
                        description: "Add 'return' after DNS::return".to_owned(),
                        // IRULE5004: adding `return` stops iRule processing after
                        // `DNS::return` — a control-flow change.
                        safety: crate::irules_checks::FixSafety::BehaviourHardening,
                    }],
                });
            }
        }
    }
    out
}

// IRULE1005 / IRULE1006 / IRULE1007 / IRULE1008 — collect/release/payload
//
// Cross-event analysis: classifies registry-declared collection operations
// across every original event body, then emits:
//
//   * IRULE1005 — `*_DATA` event handler exists but no matching
//     `*::collect` for the corresponding protocol.
//   * IRULE1006 — a payload that requires collection without a matching
//     collect (not UDP or ASM payloads, which are immediately available).
//   * IRULE1007 — a collection without an explicit or registry-declared
//     implicit release on the same connection side.
//   * IRULE1008 — a release without matching collection on the same
//     connection side.
//
// Side awareness: `EventProps::command_side` declares the event's execution
// side. Nested side-switch bodies apply their registry-declared
// `CommandSpec::side_switch_target`; no command or event spelling is decoded
// here.

#[derive(Default)]
struct CollectFlowState {
    /// Runtime lookup and widening remain independent of positive source calls.
    obligations: HashSet<CollectionSourceObligationKind>,
    source_argv: Vec<(crate::ir::ProcedureId, Vec<tcl_lexer::NativeWord>)>,
    /// `protocol -> {side, ...}` for each protocol with a collect call.
    collected: std::collections::HashMap<String, std::collections::HashSet<String>>,
    /// Same shape for release calls.
    released: std::collections::HashMap<String, std::collections::HashSet<String>>,
    /// Each collect call, including the release policy declared by its spec.
    collect_calls: Vec<CollectionCall>,
    /// Each release call.
    release_calls: Vec<CollectionCall>,
    /// Each payload call, including whether its spec requires a collect.
    payload_calls: Vec<PayloadCall>,
}

/// Unresolved runtime premises retained independently of source pairings.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CollectionSourceObligationKind {
    /// A genuine no-own-argv widening phase retains unknown effects.
    EffectOnly(crate::ir::SyntheticMarker),
    /// Runtime command selection retains an unknown alternative.
    UnknownTarget,
    /// Runtime lookup may select no command.
    MayBeAbsent,
    /// A retained event declaration does not establish activation.
    EventActivation,
    /// Authored source descriptors do not establish their actual handlers.
    SourceSchemaApplicability,
    /// Source ordering does not establish child or parent normal completion.
    NormalContinuation,
}

/// One collect or release operation observed in a `when` body.
struct CollectionCall {
    protocol: String,
    event: String,
    side: String,
    span: Span,
    release_requirement: CollectionReleaseRequirement,
}

/// One payload operation observed in a `when` body.
struct PayloadCall {
    protocol: String,
    side: String,
    span: Span,
    collection_requirement: PayloadCollectionRequirement,
}

/// Record a registry-declared data-collection operation.
///
/// This deliberately does not infer semantics from a `::payload` suffix:
/// `UDP::payload` and `ASM::payload` are immediately available, while TCP,
/// HTTP, and SSL payloads require collection.  New protocol commands join the
/// analysis by declaring [`tcl_registry::DataCollectionOperation`] in their
/// command specs.
fn classify_collect_operation(
    operation: tcl_registry::events::DataCollectionOperation,
    args: &[String],
    event: &str,
    side: &str,
    span: Span,
    state: &mut CollectFlowState,
) {
    let arg_words: Vec<&str> = args.iter().map(String::as_str).collect();
    let protocol = operation.protocol.name.to_string();
    match operation.action {
        DataCollectionAction::Collect => {
            state
                .collected
                .entry(protocol.clone())
                .or_default()
                .insert(side.to_string());
            state.collect_calls.push(CollectionCall {
                protocol,
                event: event.to_owned(),
                side: side.to_string(),
                span,
                release_requirement: operation.protocol.release_requirement,
            });
        }
        DataCollectionAction::Release => {
            state
                .released
                .entry(protocol.clone())
                .or_default()
                .insert(side.to_string());
            state.release_calls.push(CollectionCall {
                protocol,
                event: event.to_owned(),
                side: side.to_string(),
                span,
                release_requirement: operation.protocol.release_requirement,
            });
        }
        DataCollectionAction::Payload => {
            let Some(collection_requirement) = operation.payload_requirement_for_args(&arg_words)
            else {
                // Dynamic/unknown call form: it may select a payload form
                // whose availability differs, so abstain rather than guess.
                return;
            };
            state.payload_calls.push(PayloadCall {
                protocol,
                side: side.to_string(),
                span,
                collection_requirement,
            });
        }
    }
}

fn scan_when_body_for_collect_flow(
    fu: &FunctionUnit,
    event: &str,
    side: &str,
    state: &mut CollectFlowState,
    source: &IrulesSourceQuery<'_>,
) -> Option<()> {
    let config = source.function_config(fu)?;
    for bn in cfg_order(&fu.cfg) {
        if !fu.sccp.executable_blocks.contains(&bn) {
            continue;
        }
        let Some(block) = fu.cfg.blocks.get(&bn) else {
            continue;
        };
        for stmt in &block.statements {
            classify_stmt_for_collect_flow(
                stmt,
                CollectionEventSide { event, side },
                state,
                source,
                fu,
                config,
                fu.base_offset,
            )?;
        }
    }
    Some(())
}

fn retain_collection_source_obligations(
    possible: &crate::registry_invocation::RegistryInvocationAssistance,
    state: &mut CollectFlowState,
) {
    if possible.unknown_residual {
        state
            .obligations
            .insert(CollectionSourceObligationKind::UnknownTarget);
    }
    if possible.may_be_absent {
        state
            .obligations
            .insert(CollectionSourceObligationKind::MayBeAbsent);
    }
}

#[derive(Clone, Copy)]
struct CollectionEventSide<'a> {
    event: &'a str,
    side: &'a str,
}

/// Follow registry-declared side transitions with each original child owner.
/// Missing supplied child source withdraws the whole pairing query: it is not
/// evidence that a collect or release is absent.
fn scan_selected_collection_side_body(
    parent: &crate::ir::CommandTokens,
    written: usize,
    event_side: CollectionEventSide<'_>,
    state: &mut CollectFlowState,
    source: &IrulesSourceQuery<'_>,
    original_function: &FunctionUnit,
    base_offset: i64,
) -> Option<()> {
    let CollectionEventSide {
        event,
        side: inner_side,
    } = event_side;
    let registry = source.registry;
    if source.standalone {
        let binding = parent.source_binding.as_ref()?;
        let image = binding.invocation_site()?.source.source_image();
        let parent_config = binding.original_lexer_config_for_tokens(parent)?;
        let words = crate::registry_invocation::original_native_compiler_words(
            image,
            parent.words(),
            parent.argv.first()?.start(),
            parent_config,
        )?;
        let word = words.get(written.checked_add(1)?)?;
        let content = word.content_span().ok()?;
        let body = std::str::from_utf8(image.bytes().get(content.as_range())?).ok()?;
        scan_standalone_side_switch_body(
            body,
            CollectionEventSide {
                event,
                side: inner_side,
            },
            state,
            source,
            original_function,
            word.config(),
            base_offset + i64::from(content.start()),
        )?;
    } else {
        let body_unit = source.unit.conditional_original_body_unit(
            registry,
            original_function,
            parent,
            written,
        )?;
        scan_secondary_collection_body(
            &body_unit,
            CollectionEventSide {
                event,
                side: inner_side,
            },
            state,
            source,
            original_function,
            body_unit.source_lexer_config(),
            0,
        )?;
    }
    Some(())
}

fn classify_stmt_for_collect_flow(
    stmt: &Statement,
    event_side: CollectionEventSide<'_>,
    state: &mut CollectFlowState,
    source: &IrulesSourceQuery<'_>,
    original_function: &FunctionUnit,
    config: tcl_lexer::LexerConfig,
    base_offset: i64,
) -> Option<()> {
    let CollectionEventSide { event, side } = event_side;
    let registry = source.registry;
    if let crate::ir::OriginalArgvInventoryDisposition::EffectOnly(marker) =
        stmt.original_argv_inventory_disposition()
    {
        state
            .obligations
            .insert(CollectionSourceObligationKind::EffectOnly(marker));
        return Some(());
    }
    if let Some(tokens) = stmt.tokens() {
        tokens
            .source_binding
            .as_ref()?
            .original_lexer_config_for_tokens(tokens)?;
        scan_nested_collection_words(
            tokens,
            CollectionEventSide { event, side },
            state,
            source,
            original_function.body_id,
            config,
            base_offset,
        )?;
    }
    let abs = |sp: Span| -> Span {
        if base_offset == 0 {
            return sp;
        }
        let s = (i64::from(sp.start()) + base_offset).max(0);
        let e = (i64::from(sp.end()) + base_offset).max(0);
        Span::new(
            u32::try_from(s).unwrap_or(u32::MAX),
            u32::try_from(e).unwrap_or(u32::MAX),
        )
    };
    let (span, parent) = match stmt {
        Statement::Call { span, tokens, .. } | Statement::Barrier { span, tokens, .. } => {
            (*span, tokens.as_ref()?)
        }
        Statement::AssignValue { tokens: None, .. } => return None,
        _ => return Some(()),
    };
    let possible =
        crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
            registry,
            source.metadata(),
            parent,
        )?;
    retain_collection_source_obligations(&possible, state);
    for (ordinal, shape) in possible.candidates.iter().enumerate() {
        if possible.candidates[..ordinal].contains(shape) {
            continue;
        }
        if let Some(target) = shape.side_switch_target {
            if !shape.roles_complete {
                return None;
            }
            let mut bodies = shape.roles.iter().filter_map(|&(argument, role)| {
                (role == tcl_registry::ArgRole::Body)
                    .then_some(shape.argument_offset + usize::from(argument))
            });
            let effective = bodies.next()?;
            if bodies.next().is_some() {
                return None;
            }
            let written = shape.effective.written_argument(effective)?;
            if !parent.arg_is_braced_literal(written) {
                return None;
            }
            let inner_side = target.execution_side(side);
            scan_selected_collection_side_body(
                parent,
                written,
                CollectionEventSide {
                    event,
                    side: inner_side,
                },
                state,
                source,
                original_function,
                base_offset,
            )?;
        } else if let Some(operation) = shape.data_collection {
            let Some(arguments) = collection_source_arguments(parent, shape, operation, source)
            else {
                continue;
            };
            classify_collect_operation(operation, &arguments, event, side, abs(span), state);
        }
    }
    Some(())
}

/// Nested argv retains the function's full policy while the original binding
/// may select its execution grammar. Metadata and dispatch/effect admissions
/// remain independent of this syntax purpose.
fn scan_nested_collection_words(
    parent: &crate::ir::CommandTokens,
    event_side: CollectionEventSide<'_>,
    state: &mut CollectFlowState,
    source: &IrulesSourceQuery<'_>,
    body: Option<crate::ir::ProcedureId>,
    parent_config: tcl_lexer::LexerConfig,
    base_offset: i64,
) -> Option<()> {
    let CollectionEventSide { event, side } = event_side;
    let children = source.argument_calls(parent, parent_config)?;
    if !source.standalone {
        state
            .source_argv
            .push((body?, source.original_argv(parent)?));
    }
    for call in children {
        let tokens = call.tokens.as_ref()?;
        if !source.standalone {
            state
                .source_argv
                .push((body?, source.original_argv(tokens)?));
        }
        let possible =
            crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
                source.registry,
                source.metadata(),
                tokens,
            )?;
        retain_collection_source_obligations(&possible, state);
        for (ordinal, shape) in possible.candidates.iter().enumerate() {
            if possible.candidates[..ordinal].contains(shape) {
                continue;
            }
            let Some(operation) = shape.data_collection else {
                continue;
            };
            let Some(arguments) = collection_source_arguments(tokens, shape, operation, source)
            else {
                continue;
            };
            let span = Span::new(
                u32::try_from((base_offset + i64::from(tokens.argv.first()?.start())).max(0))
                    .unwrap_or(u32::MAX),
                u32::try_from((base_offset + i64::from(tokens.argv.last()?.end())).max(0))
                    .unwrap_or(u32::MAX),
            );
            classify_collect_operation(operation, &arguments, event, side, span, state);
        }
    }
    Some(())
}

fn collection_source_arguments(
    tokens: &crate::ir::CommandTokens,
    shape: &crate::registry_invocation::RegistryInvocationShape,
    operation: tcl_registry::events::DataCollectionOperation,
    source: &IrulesSourceQuery<'_>,
) -> Option<Vec<String>> {
    if operation.action != DataCollectionAction::Payload {
        return Some(Vec::new());
    }
    shape
        .source_argument_words(source.registry, source.metadata(), tokens)?
        .iter()
        .map(|word| Some(std::str::from_utf8(word.literal_bytes()?).ok()?.to_owned()))
        .collect()
}

/// Explicit standalone assistance is a separate fresh body ingress. It retains
/// the caller's complete config and never substitutes a profile's grammar.
fn scan_standalone_side_switch_body(
    body_text: &str,
    event_side: CollectionEventSide<'_>,
    state: &mut CollectFlowState,
    source: &IrulesSourceQuery<'_>,
    original_function: &FunctionUnit,
    parent_config: tcl_lexer::LexerConfig,
    base_offset: i64,
) -> Option<()> {
    let CollectionEventSide { event, side } = event_side;
    if !source.standalone {
        return None;
    }
    let config = tcl_lexer::SourceSegmentationPurpose::NestedScript.config(parent_config);
    let module = lower_to_ir_with_config(body_text, source.registry, config);
    let cfg_module = build_cfg_with_registry_and_config(&module, false, source.registry, config);
    let unit = FunctionUnit::build(
        "::side_switch",
        cfg_module.top_level,
        &[],
        source.registry,
        config,
    );
    scan_secondary_collection_body(
        &unit,
        CollectionEventSide { event, side },
        state,
        source,
        original_function,
        config,
        base_offset,
    )
}

fn scan_secondary_collection_body(
    unit: &FunctionUnit,
    event_side: CollectionEventSide<'_>,
    state: &mut CollectFlowState,
    source: &IrulesSourceQuery<'_>,
    original_function: &FunctionUnit,
    config: tcl_lexer::LexerConfig,
    base_offset: i64,
) -> Option<()> {
    let CollectionEventSide { event, side } = event_side;
    for bn in cfg_order(&unit.cfg) {
        if !unit.sccp.executable_blocks.contains(&bn) {
            continue;
        }
        let Some(block) = unit.cfg.blocks.get(&bn) else {
            continue;
        };
        for stmt in &block.statements {
            classify_stmt_for_collect_flow(
                stmt,
                CollectionEventSide { event, side },
                state,
                source,
                original_function,
                config,
                base_offset,
            )?;
        }
    }
    Some(())
}

/// Definite collect/release diagnostics require an independent runtime lifecycle owner.
/// Retained source inventories alone cannot supply that absence proof.
#[must_use]
pub fn find_collect_flow_warnings(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    dialect: Option<SurfaceQuery<'_>>,
) -> Vec<IrulesCheckWarning> {
    if !is_irules(dialect) {
        return Vec::new();
    }
    let Some(source) = IrulesSourceQuery::capture(cu, registry) else {
        return Vec::new();
    };
    let Some((state, _)) = collect_flow_state(&source) else {
        return Vec::new();
    };
    if !state.obligations.is_empty() {
        return Vec::new();
    }
    // This source-only interface has no complete runtime collection owner.
    Vec::new()
}

/// Conditional source pairing under genuine full input and handler-body owners.
/// Missing lexical phases or unsupported Body/Expr/callback inventories decline.
#[must_use]
pub fn find_conditional_collection_source_pairing_advice<'unit>(
    cu: &'unit CompilationUnit,
    registry: &'unit CommandRegistry,
    dialect: Option<SurfaceQuery<'_>>,
) -> Option<ConditionalCollectionSourcePairingAdvice<'unit>> {
    if !is_irules(dialect) {
        return None;
    }
    let source = IrulesSourceQuery::capture(cu, registry)?;
    let (state, events_seen) = collect_flow_state(&source)?;
    let advice = source.conditional_pairing_advice(&state)?;
    let events = EventRegistry::build();
    let mut out = Vec::new();
    emit_missing_data_collect_warnings(
        cu,
        registry,
        &events,
        &events_seen,
        &state,
        &advice,
        &mut out,
    );
    emit_payload_without_collect_warnings(&state, &advice, &mut out);
    emit_collect_without_release_warnings(&events, &events_seen, &state, &advice, &mut out);
    emit_release_without_collect_warnings(&state, &advice, &mut out);
    let mut obligations = state.obligations.clone();
    obligations.insert(CollectionSourceObligationKind::EventActivation);
    obligations.insert(CollectionSourceObligationKind::SourceSchemaApplicability);
    Some(ConditionalCollectionSourcePairingAdvice {
        owner: ConditionalIruleSourceOwner {
            unit: cu,
            registry,
            _handlers: advice.handlers,
            _schemas: advice.schemas,
            obligations,
            _completions: advice.completions,
            flow_commands: advice.flow_commands,
        },
        pairings: out
            .into_iter()
            .map(|warning| ConditionalCollectionSourcePairing { warning })
            .collect(),
    })
}

/// Source-order flow advice under the same complete leaf-handler census.
/// Descriptor fallthrough is only a source premise: actual lookup, effects,
/// activation and normal continuation remain unresolved in the returned owner.
#[must_use]
pub fn find_conditional_irule_source_flow_advice<'unit>(
    cu: &'unit CompilationUnit,
    registry: &'unit CommandRegistry,
    dialect: Option<SurfaceQuery<'_>>,
) -> Option<ConditionalIruleSourceFlowAdvice<'unit>> {
    find_conditional_collection_source_pairing_advice(cu, registry, dialect)?
        .conditional_source_flow()
}

/// Whether every possible source schema for the statement completes some
/// other way than falling through (`return`).
fn statement_ends_its_path(
    source: &IrulesSourceQuery<'_>,
    tokens: &crate::ir::CommandTokens,
) -> bool {
    let Some(metadata) = source.metadata() else {
        return false;
    };
    crate::registry_invocation::original_registry_invocation_assistance_with_metadata_context(
        source.registry,
        Some(metadata),
        tokens,
    )
    .is_some_and(|possible| {
        !possible.candidates.is_empty()
            && possible.candidates.iter().all(|candidate| {
                candidate
                    .with_possible_source_schema(
                        source.registry,
                        metadata,
                        tokens,
                        leaf_handler_source_completion,
                    )
                    .is_some_and(|completion| {
                        completion != tcl_registry::registry::InvocationCompletion::FallsThrough
                    })
            })
    })
}

fn unguarded_drop_findings(finals: Vec<DropFlowState>) -> Vec<IrulesCheckWarning> {
    let mut warnings = Vec::new();
    for pending in finals {
        if pending.dropped
            && let Some(span) = pending.drop_at
        {
            warnings.push(
                conditional_source_flow_finding(
                    span,
                    DiagCode::Irule5002,
                    &format!(
                        "Possible '{}' has no independently established guard against later processing.",
                        pending.drop_cmd
                    ),
                )
                .warning,
            );
        }
        if pending.dns_returned
            && let Some(span) = pending.dns_at
        {
            warnings.push(
                conditional_source_flow_finding(
                    span,
                    DiagCode::Irule5004,
                    "Possible DNS packet return has no independently established guard against later processing.",
                )
                .warning,
            );
        }
    }
    warnings
}

/// One handler's findings and the lookup obligations its statements carry.
fn conditional_handler_flow(
    unit: &CompilationUnit,
    registry: &CommandRegistry,
    source: &IrulesSourceQuery<'_>,
    events: &EventRegistry,
    id: crate::ir::ProcedureId,
) -> Option<(
    Vec<IrulesCheckWarning>,
    HashSet<CollectionSourceObligationKind>,
)> {
    let mut warnings = Vec::new();
    let function = unit.procedures.get(&id)?;
    let event = source.event(function)?.into_event()?;
    let config = source.function_config(function)?;
    let unreached = unreached_statements(function);
    let http = events
        .get_props(event.body.event())
        .is_some_and(|props| props.implied_profiles.contains(&"HTTP"));
    // The walk follows the structured body path by path: an `if` or
    // `switch` arm, a `catch` body and a loop each keep their own states,
    // and a `return` ends its path. A statement's lookup obligations stay
    // on the advice instead of refusing it.
    let missing_source = std::cell::Cell::new(false);
    let obligations = std::cell::RefCell::new(HashSet::new());
    let commands = |script: &Script, statement: &Statement| {
        let Some(inventory) = original_irules_flow_inventory(script, statement, source, config)
        else {
            missing_source.set(true);
            return Vec::new();
        };
        obligations.borrow_mut().extend(inventory.obligations);
        inventory.commands
    };
    let terminates = |tokens: &crate::ir::CommandTokens| statement_ends_its_path(source, tokens);
    if http {
        let event_name = event.body.event();
        let leaf = |state: &HttpFlowState,
                    script: &Script,
                    statement: &Statement,
                    out: &mut Vec<IrulesCheckWarning>| {
            commands(script, statement)
                .iter()
                .fold(*state, |current, command| {
                    conditional_http_flow_command(current, command, event_name, out)
                })
        };
        walk_flow(
            &event.procedure.body,
            &[HttpFlowState {
                responded: false,
                respond_at: None,
            }],
            &mut FlowDispatch {
                registry,
                out: &mut warnings,
                leaf: &leaf,
                dedupe: &dedupe_flow_states,
                unreached: &unreached,
                terminates: Some(&terminates),
            },
            None,
        );
    }
    let leaf = |state: &DropFlowState,
                script: &Script,
                statement: &Statement,
                _out: &mut Vec<IrulesCheckWarning>| {
        commands(script, statement)
            .iter()
            .fold(state.clone(), |current, command| {
                drop_flow_command(&current, command)
            })
    };
    let mut unused = Vec::new();
    let finals = walk_flow(
        &event.procedure.body,
        &[DropFlowState::default()],
        &mut FlowDispatch {
            registry,
            out: &mut unused,
            leaf: &leaf,
            dedupe: &dedupe_drop_states,
            unreached: &unreached,
            terminates: Some(&terminates),
        },
        None,
    );
    if missing_source.get() {
        return None;
    }
    warnings.extend(unguarded_drop_findings(finals));
    Some((warnings, obligations.into_inner()))
}

fn conditional_irule_source_flow_advice<'unit>(
    owner: &ConditionalIruleSourceOwner<'unit>,
) -> Option<ConditionalIruleSourceFlowAdvice<'unit>> {
    let source = IrulesSourceQuery::capture(owner.unit, owner.registry)?;
    let events = EventRegistry::build();
    let mut owner = owner.clone();
    owner
        .obligations
        .insert(CollectionSourceObligationKind::NormalContinuation);
    let mut warnings = Vec::new();
    let handlers = owner
        .flow_commands
        .iter()
        .map(|(id, _)| *id)
        .collect::<Vec<_>>();
    for id in handlers {
        let (found, obligations) =
            conditional_handler_flow(owner.unit, owner.registry, &source, &events, id)?;
        warnings.extend(found);
        owner.obligations.extend(obligations);
    }
    // Several predecessor states can reach the same command; the editor
    // receives one finding per code, range and message.
    let mut seen = HashSet::new();
    warnings.retain(|warning| {
        seen.insert((
            warning.code,
            warning.span.start(),
            warning.span.end(),
            warning.message.clone(),
        ))
    });
    Some(ConditionalIruleSourceFlowAdvice {
        owner,
        findings: warnings
            .into_iter()
            .map(|warning| ConditionalIruleSourceFlowFinding { warning })
            .collect(),
    })
}

/// Apply one command to a response-flow state under conditional advice.
fn conditional_http_flow_command(
    state: HttpFlowState,
    command: &OriginalIrulesFlowCommand,
    event: &str,
    out: &mut Vec<IrulesCheckWarning>,
) -> HttpFlowState {
    if command.may_commit_response {
        if state.responded {
            out.push(
                conditional_source_flow_finding(
                    command.span,
                    DiagCode::Irule1202,
                    &format!(
                        "Possible response operation '{}' follows another response candidate in {event}.",
                        command.command
                    ),
                )
                .warning,
            );
            return state;
        }
        return HttpFlowState {
            responded: true,
            respond_at: Some(command.span.start()),
        };
    }
    if state.responded && command.requires_http_context {
        out.push(
            conditional_source_flow_finding(
                command.span,
                DiagCode::Irule1201,
                &format!(
                    "Possible HTTP-context operation '{}' follows a response candidate in {event}.",
                    command.command
                ),
            )
            .warning,
        );
    }
    state
}

fn conditional_source_flow_finding(
    span: Span,
    code: DiagCode,
    source_hazard: &str,
) -> ConditionalIruleSourceFlowFinding {
    ConditionalIruleSourceFlowFinding {
        warning: IrulesCheckWarning {
            span,
            code,
            message: format!(
                "Conditional source flow (runtime reachability and completion unproved): {source_hazard}"
            ),
            replacement: None,
            fixes: Vec::new(),
        },
    }
}

/// Existing event-table side selection remains independent of body admission.
fn collect_flow_state(source: &IrulesSourceQuery<'_>) -> Option<(CollectFlowState, Vec<String>)> {
    let mut state = CollectFlowState::default();
    let mut events_seen = Vec::new();
    let events = EventRegistry::build();
    for fu in source.unit.analysable_functions() {
        let OriginalEventPresence::Event(event) = source.event(fu)? else {
            continue;
        };
        let event = event.body.event();
        events_seen.push(event.to_owned());
        let Some(side) = events
            .get_props(event)
            .and_then(tcl_registry::events::EventProps::command_side)
            .and_then(|side| match side {
                ConnectionSide::Client => Some("client"),
                ConnectionSide::Server => Some("server"),
                _ => None,
            })
        else {
            retain_handler_source_vectors(fu, &mut state, source)?;
            continue;
        };
        scan_when_body_for_collect_flow(fu, event, side, &mut state, source)?;
    }
    Some((state, events_seen))
}

/// An event with no single connection side has no collection pairing to
/// scan, but its handler still owns its source vectors: the flow advice
/// accepts a handler's commands only from that census.
fn retain_handler_source_vectors(
    fu: &FunctionUnit,
    state: &mut CollectFlowState,
    source: &IrulesSourceQuery<'_>,
) -> Option<()> {
    if source.standalone {
        return Some(());
    }
    let config = source.function_config(fu)?;
    let body = fu.body_id?;
    for bn in cfg_order(&fu.cfg) {
        if !fu.sccp.executable_blocks.contains(&bn) {
            continue;
        }
        let Some(block) = fu.cfg.blocks.get(&bn) else {
            continue;
        };
        for stmt in &block.statements {
            if matches!(
                stmt.original_argv_inventory_disposition(),
                crate::ir::OriginalArgvInventoryDisposition::EffectOnly(_)
            ) {
                continue;
            }
            let Some(tokens) = stmt.tokens() else {
                continue;
            };
            tokens
                .source_binding
                .as_ref()?
                .original_lexer_config_for_tokens(tokens)?;
            let children = source.argument_calls(tokens, config)?;
            state
                .source_argv
                .push((body, source.original_argv(tokens)?));
            for call in children {
                state
                    .source_argv
                    .push((body, source.original_argv(call.tokens.as_ref()?)?));
            }
        }
    }
    Some(())
}

/// Project missing source collection candidates within the certified handler census.
fn emit_missing_data_collect_warnings(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    events: &EventRegistry,
    events_seen: &[String],
    state: &CollectFlowState,
    advice: &ConditionalSourcePairingAdvice<'_, '_>,
    out: &mut Vec<IrulesCheckWarning>,
) {
    // The event table owns protocol/side requirements; the retained declaration
    // owns its original event operand. Body statements supply no header geometry.
    let mut emitted_events: std::collections::HashSet<&str> = std::collections::HashSet::new();
    for event in events_seen {
        let Some((protocols, required_side)) = events.data_collect_requirement(event) else {
            continue;
        };
        if !emitted_events.insert(event.as_str()) {
            continue;
        }
        let satisfied = protocols.iter().any(|protocol_name| {
            let Some(protocol) = registry.data_collection_protocol(protocol_name) else {
                // The event table must not be treated as proof for a protocol
                // whose command surface is absent from this registry profile.
                return false;
            };
            protocol.payload_requirement == PayloadCollectionRequirement::NotRequired
                || state
                    .collected
                    .get(protocol.name)
                    .is_some_and(|s| s.contains(required_side))
        });
        if satisfied {
            continue;
        }
        let proto_hint: Vec<&str> = protocols
            .iter()
            .filter_map(|protocol| registry.data_collection_collect_command(protocol))
            .map(|spec| spec.name)
            .collect();
        if proto_hint.is_empty() {
            // The registry says each applicable protocol supplies payload
            // without an explicit collect, so the source cannot be proved to
            // miss one. This is intentionally an abstention, not a warning.
            continue;
        }
        let Some(span) = event_anchor_span(cu, registry, event) else {
            continue;
        };
        out.push(IrulesCheckWarning {
            span,
            code: DiagCode::Irule1005,
            message: conditional_source_pairing_message(advice, &format!(
                "'{event}' has no matching source {required_side} {} candidate in the retained handlers.",
                proto_hint.join(" or "),
            )),
            replacement: None,
            fixes: Vec::new(),
        });
    }
}

/// Original event operand from the authentic selected declaration generation.
fn event_anchor_span(
    cu: &CompilationUnit,
    registry: &CommandRegistry,
    event: &str,
) -> Option<Span> {
    let source = IrulesSourceQuery::capture(cu, registry)?;
    source
        .unit
        .functions()
        .filter_map(|fu| source.event(fu).and_then(OriginalEventPresence::into_event))
        .filter(|owned| owned.body.event() == event)
        .filter_map(|owned| {
            owned
                .body
                .event_word(&cu.ir_module.source, cu.ir_module.lexer_config, registry)
        })
        .map(tcl_lexer::NativeWord::span)
        .min_by_key(|span| span.start())
}

/// Emit IRULE1006 for payload commands that explicitly require collection.
fn emit_payload_without_collect_warnings(
    state: &CollectFlowState,
    advice: &ConditionalSourcePairingAdvice<'_, '_>,
    out: &mut Vec<IrulesCheckWarning>,
) {
    for payload in &state.payload_calls {
        let matched = state
            .collected
            .get(&payload.protocol)
            .is_some_and(|s| s.contains(&payload.side));
        if payload.collection_requirement == PayloadCollectionRequirement::ExplicitCollect
            && !matched
        {
            out.push(IrulesCheckWarning {
                span: payload.span,
                code: DiagCode::Irule1006,
                message: conditional_source_pairing_message(advice, &format!(
                    "Possible '{}::payload' without a matching source {} {}::collect candidate.",
                    payload.protocol, payload.side, payload.protocol,
                )),
                replacement: None,
                fixes: Vec::new(),
            });
        }
    }
}

/// Emit IRULE1007 for registered collection lifecycles not released by code or event.
fn emit_collect_without_release_warnings(
    events: &EventRegistry,
    events_seen: &[String],
    state: &CollectFlowState,
    advice: &ConditionalSourcePairingAdvice<'_, '_>,
    out: &mut Vec<IrulesCheckWarning>,
) {
    for collect in &state.collect_calls {
        let needs_release =
            collect.release_requirement != CollectionReleaseRequirement::NotApplicable;
        let released = state
            .released
            .get(&collect.protocol)
            .is_some_and(|s| s.contains(&collect.side));
        let implicitly_released = collect.release_requirement
            == CollectionReleaseRequirement::ImplicitInDataEvent
            && events_seen.iter().any(|event| {
                let side = match collect.side.as_str() {
                    "client" => ConnectionSide::Client,
                    "server" => ConnectionSide::Server,
                    _ => return false,
                };
                events.data_event_releases_collection(
                    event,
                    &collect.event,
                    &collect.protocol,
                    side,
                )
            });
        if needs_release && !released && !implicitly_released {
            out.push(IrulesCheckWarning {
                span: collect.span,
                code: DiagCode::Irule1007,
                message: conditional_source_pairing_message(advice, &format!(
                    "Possible {}::collect without a matching source {}::release candidate on the {} side.",
                    collect.protocol, collect.protocol, collect.side,
                )),
                replacement: None,
                fixes: Vec::new(),
            });
        }
    }
}

/// Project source release candidates without a matching source collection.
fn emit_release_without_collect_warnings(
    state: &CollectFlowState,
    advice: &ConditionalSourcePairingAdvice<'_, '_>,
    out: &mut Vec<IrulesCheckWarning>,
) {
    for release in &state.release_calls {
        let matched = state
            .collected
            .get(&release.protocol)
            .is_some_and(|s| s.contains(&release.side));
        if !matched {
            out.push(IrulesCheckWarning {
                span: release.span,
                code: DiagCode::Irule1008,
                message: conditional_source_pairing_message(advice, &format!(
                    "Possible {}::release without a matching source {}::collect candidate on the {} side.",
                    release.protocol, release.protocol, release.side,
                )),
                replacement: None,
                fixes: Vec::new(),
            });
        }
    }
}

// IRULE1201 / IRULE1202 — HTTP-after-respond / multi-respond
//
// Path-sensitive analysis of the *structured* IR body of each
// original HTTP-profile event body, tracking a set of per-path response states
// so that:
//
//   * IRULE1202 — a second `HTTP::respond` / `HTTP::redirect` / `redirect`
//     on a path that already responded; only the first response takes effect.
//   * IRULE1201 — a registry-declared HTTP-context command issued on a path
//     where a response is already committed.
//
// Mutually-exclusive branches keep separate states, so `if {…} { respond }
// else { respond }` is *not* a double-respond (avoids the linear-scan false
// positive), `catch { respond }; HTTP::…` *is* flagged, and a loop body runs
// zero-or-more times.  `return` terminates the current path.

/// A single response-flow fact along one path.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct HttpFlowState {
    responded: bool,
    /// Start offset of the committing response (for dedup / parity), or `None`.
    respond_at: Option<u32>,
}

/// Per-event HTTP-flow warnings. Emits IRULE1201 + IRULE1202 for each handler
/// whose event registry entry implies an HTTP profile, path-sensitively.
#[must_use]
pub fn find_http_flow_warnings(
    cu: &CompilationUnit,
    dialect: Option<SurfaceQuery<'_>>,
) -> Vec<IrulesCheckWarning> {
    let mut out = Vec::new();
    if !is_irules(dialect) {
        return out;
    }
    let Some(source) = IrulesSourceQuery::capture(cu, cu.ir_module.resolved_registry()) else {
        return out;
    };
    let events = EventRegistry::build();
    // Iterate the *analysable* functions (honouring the complexity
    // guard) but walk their structured IR body.
    for fu in cu.analysable_functions() {
        let Some(event) = source.event(fu) else {
            return Vec::new();
        };
        let OriginalEventPresence::Event(event) = event else {
            continue;
        };
        let bare_event = event.body.event();
        if !events
            .get_props(bare_event)
            .is_some_and(|props| props.implied_profiles.contains(&"HTTP"))
        {
            continue;
        }
        let proc = event.procedure;
        let unreached = unreached_statements(fu);
        let Some(config) = source.function_config(fu) else {
            return Vec::new();
        };
        let missing_source = std::cell::Cell::new(false);
        let leaf = |state: &HttpFlowState,
                    script: &Script,
                    statement: &Statement,
                    out: &mut Vec<IrulesCheckWarning>| {
            let Some(commands) = original_irules_flow_commands(script, statement, &source, config)
            else {
                missing_source.set(true);
                return *state;
            };
            commands.iter().fold(*state, |current, command| {
                apply_http_flow_command(current, command, bare_event, out)
            })
        };
        walk_flow(
            &proc.body,
            &[HttpFlowState {
                responded: false,
                respond_at: None,
            }],
            &mut FlowDispatch {
                registry: cu.ir_module.resolved_registry(),
                out: &mut out,
                leaf: &leaf,
                dedupe: &dedupe_flow_states,
                unreached: &unreached,
                terminates: None,
            },
            None,
        );
        if missing_source.get() {
            return Vec::new();
        }
    }
    // Distinct predecessor states can prove the same source command unsafe.
    // The proof multiplicity is internal; the editor should receive one
    // diagnostic per code/range/message.
    let mut seen = HashSet::new();
    out.retain(|warning| {
        seen.insert((
            warning.code,
            warning.span.start(),
            warning.span.end(),
            warning.message.clone(),
        ))
    });
    out
}

/// Deduplicate flow states by `(responded, respond_at)`.
fn dedupe_flow_states(states: Vec<HttpFlowState>) -> Vec<HttpFlowState> {
    let mut seen: HashSet<(bool, i64)> = HashSet::new();
    let mut out = Vec::new();
    for s in states {
        let key = (s.responded, s.respond_at.map_or(-1, i64::from));
        if seen.insert(key) {
            out.push(s);
        }
    }
    out
}

/// Apply one command to a flow state, emitting IRULE1201/1202 as warranted.
fn apply_http_flow_command(
    state: HttpFlowState,
    command: &OriginalIrulesFlowCommand,
    event: &str,
    out: &mut Vec<IrulesCheckWarning>,
) -> HttpFlowState {
    let cmd = &command.command;
    let span = command.span;
    if command.may_commit_response {
        if state.responded {
            out.push(IrulesCheckWarning {
                span,
                code: DiagCode::Irule1202,
                message: format!(
                    "Multiple '{cmd}' calls possible in {event}. Only the first response takes effect.",
                ),
                replacement: None,
                fixes: Vec::new(),
            });
            return state;
        }
        return HttpFlowState {
            responded: true,
            respond_at: Some(span.start()),
        };
    }
    if state.responded && command.requires_http_context {
        out.push(IrulesCheckWarning {
            span,
            code: DiagCode::Irule1201,
            message: format!(
                "'{cmd}' used after response is committed. HTTP context is invalid after HTTP::respond/HTTP::redirect.",
            ),
            replacement: None,
            fixes: Vec::new(),
        });
    }
    state
}

/// Path-sensitive forward walk of a structured IR script, threading a set of
/// per-path flow states `S`.  Control-flow statements fan the states into each
/// branch and merge (`dedupe`) at join points; `return` terminates the current
/// paths.  Leaf statements are transferred by `leaf` (which may emit warnings
/// into `out`).  Shared by the IRULE1201/1202 and IRULE5002/5004 analyses —
/// they differ only in their state type, `leaf`, and `dedupe`.
///
/// `return_sink` is the nearest enclosing `catch` body's collection point.
/// Tcl `catch` swallows `TCL_RETURN`, so a `return` inside a catch body does
/// not stop the rule — control continues past the catch carrying the state as
/// of the swallowed `return`.  When `return_sink` is `Some`, a `return` hands
/// its at-return states there (so e.g. a committed response, or a pending
/// drop, survives past the catch) instead of discarding them; the [`Catch`]
/// arm folds them back into the continuing paths.  It is threaded through every
/// nested walk so a `return` buried in an `if`/`switch`/loop/`try` inside the
/// catch body still propagates out.  At event/proc scope it is `None` and a
/// `return` terminates the path.
///
/// [`Catch`]: Statement::Catch
/// The fixed dispatch state threaded through the flow walk: the warning sink
/// `out`, the per-leaf transfer `leaf`, and the state-set `dedupe`.  Bundled so
/// the mutually-recursive walk helpers stay under the argument limit.
struct FlowDispatch<'a, L, D> {
    registry: &'a CommandRegistry,
    out: &'a mut Vec<IrulesCheckWarning>,
    leaf: &'a L,
    dedupe: &'a D,
    /// The statements the solver proved never run ([`unreached_statements`]):
    /// a step over one leaves the states as they were.
    unreached: &'a HashSet<Span>,
    /// Whether a statement ends its path, for a walk that has no converged
    /// handler to ask: conditional advice answers from the statement's
    /// possible source schemas. `None` asks the handler's completion route.
    terminates: Option<&'a dyn Fn(&crate::ir::CommandTokens) -> bool>,
}

/// The spans of the statements of `fu` that only blocks the solver proved
/// unreachable hold. A statement the flow graph does not hold at all (a body
/// it keeps as one call) is not here, and neither is one any executable
/// block holds.
fn unreached_statements(fu: &FunctionUnit) -> HashSet<Span> {
    let mut held = HashSet::new();
    let mut reached = HashSet::new();
    for (id, block) in &fu.cfg.blocks {
        let live = fu.sccp.executable_blocks.contains(id);
        for span in block.statements.iter().map(|stmt| fu.abs_span(stmt.span())) {
            held.insert(span);
            if live {
                reached.insert(span);
            }
        }
    }
    held.retain(|span| !reached.contains(span));
    held
}

fn walk_flow<S, L, D>(
    script: &Script,
    in_states: &[S],
    fd: &mut FlowDispatch<'_, L, D>,
    mut return_sink: Option<&mut Vec<S>>,
) -> Vec<S>
where
    S: Clone,
    L: Fn(&S, &Script, &Statement, &mut Vec<IrulesCheckWarning>) -> S,
    D: Fn(Vec<S>) -> Vec<S>,
{
    let mut states = in_states.to_vec();
    for stmt in &script.statements {
        if states.is_empty() {
            break;
        }
        if fd.unreached.contains(&stmt.span()) {
            continue;
        }
        // Written argv evaluates before dispatch, including return values and
        // structured headers. Braced bodies keep their own script phases.
        states = (fd.dedupe)(
            states
                .iter()
                .map(|state| (fd.leaf)(state, script, stmt, fd.out))
                .collect(),
        );
        // `None` ⇒ the statement terminates every current path (`return`).
        match flow_step(stmt, &states, fd, return_sink.as_deref_mut()) {
            None => {
                // Hand the at-return states to the enclosing catch (if any) so
                // flow resumes past it, then terminate this script's path.
                if let Some(sink) = return_sink {
                    sink.extend(states);
                }
                return Vec::new();
            }
            Some(next) => states = next,
        }
    }
    states
}

/// Transfer one structured statement over the flow-state set, or `None` when
/// it (`return`) terminates the current paths.  Control-flow arms recurse
/// through [`walk_flow`] after their written argv has been evaluated.
fn flow_step<S, L, D>(
    stmt: &Statement,
    states: &[S],
    fd: &mut FlowDispatch<'_, L, D>,
    mut return_sink: Option<&mut Vec<S>>,
) -> Option<Vec<S>>
where
    S: Clone,
    L: Fn(&S, &Script, &Statement, &mut Vec<IrulesCheckWarning>) -> S,
    D: Fn(Vec<S>) -> Vec<S>,
{
    let one = std::slice::from_ref;
    if let Some(region) = stmt
        .tokens()
        .and_then(crate::ir::CommandTokens::evaluated_body)
    {
        return Some(flow_step_region(region, states, fd, return_sink));
    }
    if let Some(tokens) = stmt.tokens()
        && fd.terminates.map_or_else(
            || {
                !crate::registry_invocation::normal_handler_completion_route(
                    fd.registry,
                    None,
                    tokens,
                )
                .normal_possible()
            },
            |terminates| terminates(tokens),
        )
    {
        return None;
    }
    Some(match stmt {
        Statement::Return { .. } => return None,
        Statement::If {
            clauses, else_body, ..
        } => {
            let mut next = Vec::new();
            for st in states {
                for clause in clauses {
                    next.extend(walk_flow(
                        &clause.body,
                        one(st),
                        fd,
                        return_sink.as_deref_mut(),
                    ));
                }
                if let Some(eb) = else_body {
                    next.extend(walk_flow(eb, one(st), fd, return_sink.as_deref_mut()));
                } else {
                    // condition-false path keeps the incoming state.
                    next.push(st.clone());
                }
            }
            (fd.dedupe)(next)
        }
        Statement::Switch {
            arms, default_body, ..
        } => flow_step_switch(arms, default_body.as_ref(), states, fd, return_sink),
        Statement::For { body, .. }
        | Statement::While { body, .. }
        | Statement::Foreach { body, .. } => {
            let mut next = Vec::new();
            for st in states {
                next.push(st.clone()); // zero iterations
                next.extend(walk_flow(body, one(st), fd, return_sink.as_deref_mut()));
            }
            (fd.dedupe)(next)
        }
        Statement::Catch { body, .. } => {
            let mut next = Vec::new();
            for st in states {
                // `catch` swallows TCL_RETURN: both the body's normal exits and
                // any at-return states continue past the catch.  Fall back to
                // the incoming state if the body yields nothing.
                let mut catch_returns = Vec::new();
                let mut body_out = walk_flow(body, one(st), fd, Some(&mut catch_returns));
                body_out.append(&mut catch_returns);
                if body_out.is_empty() {
                    next.push(st.clone());
                } else {
                    next.extend(body_out);
                }
            }
            (fd.dedupe)(next)
        }
        Statement::Try {
            body,
            handlers,
            finally_body,
            ..
        } => flow_step_try(
            body,
            handlers,
            finally_body.as_ref(),
            states,
            fd,
            return_sink,
        ),
        // A grouping block (namespace/uplevel body) runs in sequence.
        Statement::Block { body, .. } => walk_flow(body, states, fd, return_sink),
        _ => states.to_vec(),
    })
}

/// Diagnostic projection of the same source-positioned region used by CFG
/// construction. Possible entry retains the unchanged incoming alternative.
fn flow_step_region<S, L, D>(
    region: &crate::execution_region::EvaluatedBodyRegion,
    states: &[S],
    fd: &mut FlowDispatch<'_, L, D>,
    mut return_sink: Option<&mut Vec<S>>,
) -> Vec<S>
where
    S: Clone,
    L: Fn(&S, &Script, &Statement, &mut Vec<IrulesCheckWarning>) -> S,
    D: Fn(Vec<S>) -> Vec<S>,
{
    use tcl_registry::native_compilation::PossibleBodyTopology;
    if region.selection == crate::execution_region::RegionSelection::Never {
        return states.to_vec();
    }
    let mut next = if region.selection == crate::execution_region::RegionSelection::MaySkip
        || region.possible_bodies.is_some()
    {
        states.to_vec()
    } else {
        Vec::new()
    };
    let phases: Vec<_> = region.phases.iter().collect();
    let Some(possible) = &region.possible_bodies else {
        next.extend(flow_region_phases(&phases, states, fd, return_sink));
        return (fd.dedupe)(next);
    };
    match &possible.topology {
        PossibleBodyTopology::Sequence(_) | PossibleBodyTopology::Captured(_) => {
            next.extend(flow_region_phases(&phases, states, fd, return_sink));
        }
        PossibleBodyTopology::Loop {
            initial, repeated, ..
        } => {
            let initial_phases = selected_region_phases(region, &possible.arguments, initial);
            let entry = flow_region_phases(&initial_phases, states, fd, return_sink.as_deref_mut());
            next.extend(entry.clone());
            let repeated_phases = selected_region_phases(region, &possible.arguments, repeated);
            next.extend(flow_region_phases(
                &repeated_phases,
                &entry,
                fd,
                return_sink,
            ));
        }
        PossibleBodyTopology::Alternatives(_)
        | PossibleBodyTopology::Conditional(_)
        | PossibleBodyTopology::CaseAlternatives(_) => {
            for phase in &region.phases {
                next.extend(flow_region_phases(
                    &[phase],
                    states,
                    fd,
                    return_sink.as_deref_mut(),
                ));
            }
        }
    }
    (fd.dedupe)(next)
}

fn selected_region_phases<'a>(
    region: &'a crate::execution_region::EvaluatedBodyRegion,
    arguments: &[usize],
    selected: &[usize],
) -> Vec<&'a crate::execution_region::ExecutionPhase> {
    selected
        .iter()
        .filter_map(|argument| {
            arguments
                .iter()
                .position(|candidate| candidate == argument)
                .and_then(|index| region.phases.get(index))
        })
        .collect()
}

fn flow_region_phases<S, L, D>(
    phases: &[&crate::execution_region::ExecutionPhase],
    states: &[S],
    fd: &mut FlowDispatch<'_, L, D>,
    mut return_sink: Option<&mut Vec<S>>,
) -> Vec<S>
where
    S: Clone,
    L: Fn(&S, &Script, &Statement, &mut Vec<IrulesCheckWarning>) -> S,
    D: Fn(Vec<S>) -> Vec<S>,
{
    let mut continuing = states.to_vec();
    for phase in phases {
        let mut captured = Vec::new();
        continuing = if phase.abrupt == crate::execution_region::RegionTarget::Exit {
            walk_flow(&phase.script, &continuing, fd, Some(&mut captured))
        } else {
            walk_flow(&phase.script, &continuing, fd, return_sink.as_deref_mut())
        };
        continuing.extend(captured);
    }
    continuing
}

/// The `switch` arm of [`flow_step`]: every arm body (and the default body) is
/// a feasible path, plus the no-match fall-through that keeps the incoming
/// state.
fn flow_step_switch<S, L, D>(
    arms: &[crate::ir::SwitchArm],
    default_body: Option<&Script>,
    states: &[S],
    fd: &mut FlowDispatch<'_, L, D>,
    mut return_sink: Option<&mut Vec<S>>,
) -> Vec<S>
where
    S: Clone,
    L: Fn(&S, &Script, &Statement, &mut Vec<IrulesCheckWarning>) -> S,
    D: Fn(Vec<S>) -> Vec<S>,
{
    let one = std::slice::from_ref;
    let mut next = Vec::new();
    for st in states {
        // no-match fall-through path.
        next.push(st.clone());
        for arm in arms {
            if let Some(body) = &arm.body {
                next.extend(walk_flow(body, one(st), fd, return_sink.as_deref_mut()));
            }
        }
        if let Some(db) = default_body {
            next.extend(walk_flow(db, one(st), fd, return_sink.as_deref_mut()));
        }
    }
    (fd.dedupe)(next)
}

/// The `try` arm of [`flow_step`]: the body and each handler are feasible
/// paths; a `finally` body runs in sequence after each; an empty result keeps
/// the incoming state.
fn flow_step_try<S, L, D>(
    body: &Script,
    handlers: &[crate::ir::TryHandler],
    finally_body: Option<&Script>,
    states: &[S],
    fd: &mut FlowDispatch<'_, L, D>,
    mut return_sink: Option<&mut Vec<S>>,
) -> Vec<S>
where
    S: Clone,
    L: Fn(&S, &Script, &Statement, &mut Vec<IrulesCheckWarning>) -> S,
    D: Fn(Vec<S>) -> Vec<S>,
{
    let one = std::slice::from_ref;
    let mut next = Vec::new();
    for st in states {
        let mut branch = walk_flow(body, one(st), fd, return_sink.as_deref_mut());
        for handler in handlers {
            branch.extend(walk_flow(
                &handler.body,
                one(st),
                fd,
                return_sink.as_deref_mut(),
            ));
        }
        if let Some(fb) = finally_body {
            let mut final_out = Vec::new();
            for mid in &branch {
                final_out.extend(walk_flow(fb, one(mid), fd, return_sink.as_deref_mut()));
            }
            branch = final_out;
        }
        if branch.is_empty() {
            next.push(st.clone());
        } else {
            next.extend(branch);
        }
    }
    (fd.dedupe)(next)
}

// IRULE4004 — `set var value` in per-request event hoistable to once-per-connection
//
// The event registry classifies per-request and once-per-connection events.
// An `AssignConst` whose value does not depend on per-request data could be
// hoisted to a once-per-connection event, saving work per request.
//
// A warning is only safe when the literal scalar assignment is unconditional
// and is the variable's sole write across the rule. Repeated initialisation is
// commonly a deliberate per-request reset; moving it changes connection-state
// semantics after the first request. Array elements and namespace/static
// variables likewise need a different destination and are excluded.

/// Hoistable-set warnings for IRULE4004.
#[must_use]
pub fn find_hoistable_set_warnings(
    cu: &CompilationUnit,
    dialect: Option<SurfaceQuery<'_>>,
) -> Vec<IrulesCheckWarning> {
    let mut out = Vec::new();
    if !is_irules(dialect) {
        return out;
    }
    let events = EventRegistry::build();
    let registry = cu.ir_module.resolved_registry();
    let Some(source) = IrulesSourceQuery::capture(cu, registry) else {
        return out;
    };
    let Some(write_counts) = event_mutation_counts(&source) else {
        return out;
    };
    for fu in cu.analysable_functions() {
        let Some(event) = source.event(fu) else {
            return Vec::new();
        };
        let OriginalEventPresence::Event(event) = event else {
            continue;
        };
        if !events.is_per_request(event.body.event()) {
            continue;
        }
        let Some(block) = fu.cfg.blocks.get(&fu.cfg.entry) else {
            continue;
        };
        let contexts = crate::place_bridge::build_point_resolve_contexts_with_entry(
            &fu.cfg,
            event.body.conditional_entry(),
            registry,
        );
        for (index, stmt) in block.statements.iter().enumerate() {
            let Some((name, value)) = hoistable_literal_assignment(stmt, registry) else {
                continue;
            };
            let places = crate::place_bridge::def_places_with_continuation(
                stmt,
                contexts.before_statement(fu.cfg.entry, index),
                contexts.after_statement(fu.cfg.entry, index),
                registry,
            );
            let [place] = places.as_slice() else {
                continue;
            };
            let Some(cell) = crate::connection_scope::cell_from_place(place) else {
                continue;
            };
            if !cell.is_connection() {
                continue;
            }
            if name.is_empty()
                || value.is_empty()
                || place.observed
                || place.kind != crate::place::PlaceKind::Scalar
                || write_counts.get(&cell) != Some(&1)
            {
                continue;
            }
            let span = stmt.span();
            out.push(IrulesCheckWarning {
                span: fu.abs_span(span),
                code: DiagCode::Irule4004,
                message: format!(
                    "`set {name} ...` runs on every request — consider hoisting to a once-per-connection event.",
                ),
                replacement: None,
                fixes: Vec::new(),
            });
        }
    }
    out
}

/// Count physical mutations rather than displayed operands. An unbounded
/// mutation cannot prove that another event leaves a proposed destination alone.
fn event_mutation_counts(
    source: &IrulesSourceQuery<'_>,
) -> Option<HashMap<crate::connection_scope::EventCell, usize>> {
    let mut counts = HashMap::new();
    let registry = source.registry;
    for fu in source.unit.functions() {
        let OriginalEventPresence::Event(event) = source.event(fu)? else {
            continue;
        };
        let contexts = crate::place_bridge::build_point_resolve_contexts_with_entry(
            &fu.cfg,
            event.body.conditional_entry(),
            registry,
        );
        for (&id, block) in &fu.cfg.blocks {
            for (index, stmt) in block.statements.iter().enumerate() {
                for place in crate::place_bridge::statement_mutation_places_with_continuation(
                    stmt,
                    contexts.before_statement(id, index),
                    contexts.after_statement(id, index),
                    registry,
                ) {
                    let cell = crate::connection_scope::cell_from_place(&place)?;
                    *counts.entry(cell).or_insert(0) += 1;
                }
            }
        }
    }
    Some(counts)
}

fn hoistable_literal_assignment(
    statement: &Statement,
    registry: &CommandRegistry,
) -> Option<(String, String)> {
    if let Some(tokens) = statement.tokens() {
        let normal =
            crate::registry_invocation::normal_representation_invocation(registry, None, tokens)?;
        let assignment = normal.value_assignment()?;
        let dialect = tokens
            .source_binding
            .as_ref()?
            .variable_context
            .invocation_dialect?;
        let value = crate::registry_invocation::effective_invocation_word(
            &assignment.value,
            dialect.lexer_grammar.escapes,
            dialect.word_values,
        );
        return match value {
            crate::registry_invocation::EffectiveInvocationWord::Literal(value) => {
                Some((assignment.name, value))
            }
            _ => None,
        };
    }
    match statement {
        Statement::AssignConst { name, value, .. } => Some((name.clone(), value.clone())),
        _ => None,
    }
}

// IRULE4002 — generic `static::` variable name that will collide
//
// Walks every original event CFG body for statements that define a
// `static::` variable and flags bare names matching a set of generic
// regex patterns. Deduplicated across events — the first occurrence's
// span wins. The pattern set is configurable (`tclLsp.diagnostics.
// genericVariablePatterns`): pass `None` for the built-in default set,
// `Some(&[])` to disable the check, or `Some(custom)` to replace the
// defaults.

/// Default generic `static::` variable-name patterns. Each is matched case-insensitively
/// (the bare name is lowercased) against the part after the `static::`
/// prefix. A user-supplied list replaces this set wholesale.
pub const DEFAULT_GENERIC_VARIABLE_PATTERNS: &[&str] = &[
    // Debug / logging
    r"^debug(_level|_enabled)?$",
    r"^dbg$",
    r"^log_(level|server|enabled)$",
    r"^logging$",
    r"^verbose$",
    r"^trace$",
    // Configuration
    r"^(response_)?timeout$",
    r"^(max_)?retr(y|ies)$",
    r"^config$",
    r"^(enabled|disabled|active)$",
    r"^mode$",
    r"^(port|host|server|pool)$",
    // Counters / limits
    r"^count(er)?$",
    r"^(limit|max_connections|threshold|rate|interval)$",
    // Generic state
    r"^(flag|level|status|state|version|name|value|data|result|test|init|default)$",
];

/// Compile the generic-name patterns for IRULE4002. `None` selects the
/// built-in [`DEFAULT_GENERIC_VARIABLE_PATTERNS`]; `Some(custom)` uses the
/// caller's list verbatim (an empty slice compiles to no patterns, which
/// disables the check). Invalid regexes are skipped.
fn compile_generic_patterns(patterns: Option<&[String]>) -> Vec<regex::Regex> {
    let build = |src: &str| {
        regex::RegexBuilder::new(src)
            .case_insensitive(true)
            .build()
            .ok()
    };
    match patterns {
        None => DEFAULT_GENERIC_VARIABLE_PATTERNS
            .iter()
            .filter_map(|p| build(p))
            .collect(),
        Some(pats) => pats.iter().filter_map(|p| build(p)).collect(),
    }
}

/// Configured lexical patterns apply only after the namespace root is selected.
fn matches_generic_variable_pattern(simple_name: &str, compiled: &[regex::Regex]) -> bool {
    let lower = simple_name.to_ascii_lowercase();
    compiled.iter().any(|re| re.is_match(&lower))
}

#[cfg(test)]
fn is_generic_static_name(var_name: &str, compiled: &[regex::Regex]) -> bool {
    matches_generic_variable_pattern(
        var_name.strip_prefix("static::").unwrap_or(var_name),
        compiled,
    )
}

/// Generic-static-name warnings for IRULE4002. `generic_patterns` follows the
/// [`compile_generic_patterns`] convention (`None` → default set, `Some(&[])`
/// → disabled, `Some(custom)` → replace defaults).
#[must_use]
pub fn find_generic_static_name_warnings(
    cu: &CompilationUnit,
    dialect: Option<SurfaceQuery<'_>>,
    generic_patterns: Option<&[String]>,
) -> Vec<IrulesCheckWarning> {
    let mut out = Vec::new();
    if !is_irules(dialect) {
        return out;
    }
    let compiled = compile_generic_patterns(generic_patterns);
    if compiled.is_empty() {
        return out;
    }
    let registry = cu.ir_module.resolved_registry();
    let Some(source) = IrulesSourceQuery::capture(cu, registry) else {
        return out;
    };
    let mut seen = HashSet::new();
    for fu in cu.analysable_functions() {
        let Some(event) = source.event(fu) else {
            return Vec::new();
        };
        let OriginalEventPresence::Event(event) = event else {
            continue;
        };
        let contexts = crate::place_bridge::build_point_resolve_contexts_with_entry(
            &fu.cfg,
            event.body.conditional_entry(),
            registry,
        );
        for bn in cfg_order(&fu.cfg) {
            if !fu.sccp.executable_blocks.contains(&bn) {
                continue;
            }
            let Some(block) = fu.cfg.blocks.get(&bn) else {
                continue;
            };
            for (index, stmt) in block.statements.iter().enumerate() {
                let labels: Vec<&str> = match stmt {
                    Statement::AssignConst { name, .. }
                    | Statement::AssignValue { name, .. }
                    | Statement::Incr { name, .. } => vec![name],
                    Statement::Call { defs, .. } => defs.iter().map(String::as_str).collect(),
                    _ => continue,
                };
                let places =
                    crate::place_bridge::def_places_at(stmt, bn, index, &contexts, registry);
                for place in places {
                    if crate::side_effects::namespace_storage_scope_for_place(
                        &place,
                        registry.profile(),
                    ) != crate::side_effects::StorageScope::Static
                    {
                        continue;
                    }
                    let Some(key) = crate::var_resolve::canonical_place_key(&place) else {
                        continue;
                    };
                    // A single assignment's label reports that same operation. Multi-output
                    // calls join their authored labels through the actual SSA cell map.
                    let label = if matches!(stmt, Statement::Call { .. }) {
                        fu.ssa.source_symbols_at(bn, index).and_then(|symbols| {
                            labels.iter().copied().find(|label| {
                                symbols.get(*label).is_some_and(|symbol| {
                                    fu.ssa
                                        .cell_key(*symbol)
                                        .is_some_and(|cell| cell.root() == key.root())
                                })
                            })
                        })
                    } else {
                        labels.first().copied()
                    };
                    let Some(label) = label else {
                        continue;
                    };
                    check_generic_static(
                        &place,
                        &key,
                        label,
                        fu.abs_span(stmt.span()),
                        &compiled,
                        &mut seen,
                        &mut out,
                    );
                }
            }
        }
    }
    out
}

fn check_generic_static(
    place: &crate::place::Place,
    key: &crate::var_resolve::VariableCellKey,
    source_name: &str,
    span: Span,
    compiled: &[regex::Regex],
    seen: &mut HashSet<crate::var_resolve::VariableCellKey>,
    out: &mut Vec<IrulesCheckWarning>,
) {
    let root = key.root().clone();
    if seen.contains(&root) {
        return;
    }
    let Some(bare) = crate::var_resolve::namespace_variable_simple_name(place) else {
        return;
    };
    let pattern_name = if let Some(index) = &place.index {
        if index.kind != crate::place::IndexKind::Literal {
            return;
        }
        let Ok(element) = index.value.try_utf8() else {
            return;
        };
        std::borrow::Cow::Owned(format!("{bare}({element})"))
    } else {
        std::borrow::Cow::Borrowed(bare)
    };
    if !matches_generic_variable_pattern(&pattern_name, compiled) {
        return;
    }
    seen.insert(root);
    out.push(IrulesCheckWarning {
        span,
        code: DiagCode::Irule4002,
        message: format!(
            "'{source_name}' is a generic name that can collide with other iRules. \
             static:: names share storage across rule owners on each executing TMM — \
             prefix with the application or rule name (e.g. 'static::<app>_{pattern_name}')."
        ),
        replacement: None,
        fixes: Vec::new(),
    });
}

// Tests

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_getter_source_config_tests;

#[cfg(test)]
mod original_collection_source_tests;

#[cfg(test)]
mod original_event_owner_tests;
