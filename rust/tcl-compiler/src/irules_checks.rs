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
                return None;
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
                    possible_irule_flow_command(token, &possible)?
                {
                    output.handler_flow_commands.push(command);
                }
                for candidate in &possible.candidates {
                    let completion = candidate.with_source_schema(
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
    completions: Vec<tcl_registry::registry::InvocationCompletion>,
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
            possible_irule_flow_command(tokens, &possible)?
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
        requires_http_context |= candidate
            .possible_traits
            .contains(Traits::REQUIRES_HTTP_CONTEXT);
    }
    let disables_all_events = !possible.unknown_residual
        && !possible.may_be_absent
        && !possible.candidates.is_empty()
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
            completions: advice.completions,
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

fn conditional_irule_source_flow_advice<'unit>(
    owner: &ConditionalIruleSourceOwner<'unit>,
) -> Option<ConditionalIruleSourceFlowAdvice<'unit>> {
    if owner
        .completions
        .iter()
        .any(|completion| *completion != tcl_registry::registry::InvocationCompletion::FallsThrough)
    {
        return None;
    }
    let source = IrulesSourceQuery::capture(owner.unit, owner.registry)?;
    let events = EventRegistry::build();
    let mut owner = owner.clone();
    owner
        .obligations
        .insert(CollectionSourceObligationKind::NormalContinuation);
    let mut findings = Vec::new();
    for (id, commands) in &owner.flow_commands {
        let function = owner.unit.procedures.get(id)?;
        let event = source.event(function)?.into_event()?;
        let http = events
            .get_props(event.body.event())
            .is_some_and(|props| props.implied_profiles.contains(&"HTTP"));
        let mut pending = DropFlowState::default();
        let mut responded = false;
        for command in commands {
            pending = drop_flow_command(&pending, command);
            if http && command.may_commit_response {
                if responded {
                    findings.push(conditional_source_flow_finding(
                        command.span, DiagCode::Irule1202,
                        &format!("Possible response operation '{}' follows another response candidate in {}.",
                            command.command, event.body.event()),
                    ));
                }
                responded = true;
            } else if http && responded && command.requires_http_context {
                findings.push(conditional_source_flow_finding(
                    command.span,
                    DiagCode::Irule1201,
                    &format!(
                        "Possible HTTP-context operation '{}' follows a response candidate in {}.",
                        command.command,
                        event.body.event()
                    ),
                ));
            }
        }
        if pending.dropped
            && let Some(span) = pending.drop_at
        {
            findings.push(conditional_source_flow_finding(
                span, DiagCode::Irule5002,
                &format!("Possible '{}' has no independently established guard against later processing.",
                    pending.drop_cmd),
            ));
        }
        if pending.dns_returned
            && let Some(span) = pending.dns_at
        {
            findings.push(conditional_source_flow_finding(
                span, DiagCode::Irule5004,
                "Possible DNS packet return has no independently established guard against later processing.",
            ));
        }
    }
    Some(ConditionalIruleSourceFlowAdvice { owner, findings })
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
            continue;
        };
        scan_when_body_for_collect_flow(fu, event, side, &mut state, source)?;
    }
    Some((state, events_seen))
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
        && !crate::registry_invocation::normal_handler_completion_route(fd.registry, None, tokens)
            .normal_possible()
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
mod tests {

    use tcl_dialect::model::{Family, SurfaceQuery};

    use super::*;

    fn registry() -> CommandRegistry {
        CommandRegistry::build_default().project_for_profile(tcl_dialect::DialectProfile::irules())
    }

    fn warnings_for_irules(source: &str) -> Vec<IrulesCheckWarning> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        find_unnormalised_getter_warnings(
            &cu,
            &registry(),
            Some(SurfaceQuery::any_release(Family::F5Irules)),
        )
    }

    #[test]
    fn irule3102_warns_on_bare_http_uri_getter() {
        let w = warnings_for_irules("set u [HTTP::uri]");
        assert_eq!(w.len(), 1, "expected one IRULE3102, got {w:?}");
        assert_eq!(w[0].code, DiagCode::Irule3102);
        assert!(w[0].message.contains("HTTP::uri -normalized"));
    }

    #[test]
    fn irule3102_clean_when_normalized_flag_present() {
        let w = warnings_for_irules("set u [HTTP::uri -normalized]");
        assert_eq!(w.len(), 0, "expected no IRULE3102, got {w:?}");
    }

    #[test]
    fn irule3102_setter_form_not_flagged() {
        // `HTTP::path /x` — first arg `/x` is non-flag → setter form → no warning.
        let w = warnings_for_irules("HTTP::path /x");
        assert_eq!(w.len(), 0, "expected no IRULE3102 on setter, got {w:?}");
    }

    #[test]
    fn irule3102_warns_on_getter_nested_in_sink_argument() {
        // `HTTP::respond 200 content [HTTP::uri]` — the unnormalised
        // `[HTTP::uri]` getter is a command-substitution *argument* of the
        // HTTP sink, not a top-level call/assignment, but must still fire.
        let w = warnings_for_irules(
            "when HTTP_REQUEST {\n    HTTP::respond 200 content [HTTP::uri]\n}\n",
        );
        assert_eq!(w.len(), 1, "expected one IRULE3102, got {w:?}");
        assert_eq!(w[0].code, DiagCode::Irule3102);
        assert!(w[0].message.contains("HTTP::uri -normalized"));
    }

    #[test]
    fn irule3102_silent_for_constant_sink_argument() {
        // A constant fed into the same sink stays silent.
        let w = warnings_for_irules(
            "when HTTP_REQUEST {\n    HTTP::respond 200 content \"static body\"\n}\n",
        );
        assert_eq!(w.len(), 0, "expected no IRULE3102, got {w:?}");
    }

    #[test]
    fn irule3102_silent_for_normalized_getter_in_sink_argument() {
        // The nested getter with `-normalized` is canonicalised → silent.
        let w = warnings_for_irules(
            "when HTTP_REQUEST {\n    HTTP::respond 200 content [HTTP::uri -normalized]\n}\n",
        );
        assert_eq!(w.len(), 0, "expected no IRULE3102, got {w:?}");
    }

    #[test]
    fn irule3102_non_irules_dialect_returns_empty() {
        let cu = CompilationUnit::build_for("set u [HTTP::uri]", &registry(), false);
        assert_eq!(
            find_unnormalised_getter_warnings(&cu, &registry(), None),
            [] as [crate::irules_checks::IrulesCheckWarning; 0]
        );
        assert_eq!(
            find_unnormalised_getter_warnings(
                &cu,
                &registry(),
                Some(SurfaceQuery::any_release(Family::Tcl))
            )
            .len(),
            0
        );
    }

    #[test]
    fn irule3102_also_fires_inside_assign_value_cmd_sub() {
        // Plain `set u [HTTP::query]` — command-sub form, must still fire.
        let w = warnings_for_irules("set u [HTTP::query]");
        assert_eq!(w.len(), 1, "expected IRULE3102 for HTTP::query, got {w:?}");
        assert!(w[0].message.contains("HTTP::query"));
    }

    #[test]
    fn irule3102_fires_on_http_path_without_flag() {
        let w = warnings_for_irules("set u [HTTP::path]");
        assert_eq!(w.len(), 1);
        assert!(w[0].message.contains("HTTP::path"));
    }

    #[test]
    fn is_getter_form_recognises_flags_only() {
        assert!(is_getter_form(&[]));
        assert!(is_getter_form(&["-normalized".to_owned()]));
        assert!(!is_getter_form(&["/foo".to_owned()]));
        assert!(!is_getter_form(&[
            "foo".to_owned(),
            "-normalized".to_owned()
        ]));
    }

    /// IRULE3102 must derive its command set from the
    /// registry's `OptionSpec` table (not a hardcoded list). When
    /// the registry-side option is present, the check fires; when
    /// the registered command does not declare `-normalized`, no
    /// diagnostic is produced.
    #[test]
    fn arch3_normalised_option_is_registry_driven() {
        let registry = registry();
        // Registry-side: HTTP::uri / HTTP::path / HTTP::query carry
        // the `-normalized` option in their command spec.
        for name in ["HTTP::uri", "HTTP::path", "HTTP::query"] {
            assert!(
                supports_normalized_flag(&registry, name),
                "{name} should declare -normalized in its registry OptionSpec",
            );
        }
        // A registered iRules command without `-normalized` is not a
        // candidate (e.g. `HTTP::header`, which has no
        // `-normalized` option).
        assert!(
            !supports_normalized_flag(&registry, "HTTP::header"),
            "HTTP::header has no -normalized option in the registry",
        );

        // End-to-end proof: HTTP::uri without -normalized fires
        // IRULE3102; HTTP::header (no -normalized in registry) does not.
        let with_uri = warnings_for_irules("set u [HTTP::uri]");
        assert!(
            with_uri.iter().any(|w| w.code == DiagCode::Irule3102),
            "expected IRULE3102 on HTTP::uri, got {with_uri:?}",
        );
        let with_header = warnings_for_irules("set u [HTTP::header Content-Type]");
        assert_eq!(
            with_header.len(),
            0,
            "no IRULE3102 expected on HTTP::header (no -normalized), got {with_header:?}",
        );
    }

    // IRULE5002 / IRULE5004

    fn drop_warnings(source: &str) -> Vec<IrulesCheckWarning> {
        source_flow_warnings(source)
            .into_iter()
            .filter(|warning| matches!(warning.code, DiagCode::Irule5002 | DiagCode::Irule5004))
            .collect()
    }

    #[test]
    fn irule5002_drop_without_return_fires() {
        let ws = drop_warnings("when CLIENT_ACCEPTED { drop }");
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule5002),
            "expected IRULE5002, got {ws:?}",
        );
    }

    #[test]
    fn irule5002_drop_followed_by_return_clean() {
        let ws = drop_warnings("when CLIENT_ACCEPTED { drop; return }");
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule5002),
            "no IRULE5002 expected when `return` guards the drop, got {ws:?}",
        );
    }

    #[test]
    fn irule5002_drop_followed_by_event_disable_all_clean() {
        let ws = drop_warnings("when CLIENT_ACCEPTED { drop; event disable all }");
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule5002),
            "no IRULE5002 expected when `event disable all` guards the drop, got {ws:?}",
        );
    }

    #[test]
    fn irule5002_reject_also_fires() {
        let ws = drop_warnings("when CLIENT_ACCEPTED { reject }");
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule5002),
            "expected IRULE5002 on `reject`, got {ws:?}",
        );
    }

    #[test]
    fn irule5002_only_in_irules_dialect() {
        let cu = CompilationUnit::build_for("when CLIENT_ACCEPTED { drop }", &registry(), false);
        let none_dialect = find_unguarded_drop_warnings(&cu, None);
        assert_eq!(none_dialect.len(), 0, "got {none_dialect:?}");
        let tcl_dialect =
            find_unguarded_drop_warnings(&cu, Some(SurfaceQuery::any_release(Family::Tcl)));
        assert_eq!(tcl_dialect.len(), 0, "got {tcl_dialect:?}");
    }

    #[test]
    fn irule5004_dns_return_without_return_fires() {
        let ws = drop_warnings("when DNS_REQUEST { DNS::return }");
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule5004),
            "expected IRULE5004, got {ws:?}",
        );
    }

    #[test]
    fn irule5004_dns_return_followed_by_return_clean() {
        let ws = drop_warnings("when DNS_REQUEST { DNS::return; return }");
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule5004),
            "no IRULE5004 expected when `return` follows `DNS::return`, got {ws:?}",
        );
    }

    #[test]
    fn no_drop_warnings_for_clean_when_body() {
        let ws = drop_warnings("when CLIENT_ACCEPTED { log local0. \"connection open\" }");
        assert_eq!(ws.len(), 0, "got {ws:?}");
    }

    fn drop_codes(source: &str) -> Vec<String> {
        let mut c: Vec<String> = drop_warnings(source)
            .iter()
            .map(|w| w.code.to_string())
            .collect();
        c.sort();
        c
    }

    #[test]
    fn irule5002_fires_when_only_one_branch_guarded() {
        // The `drop` path has no following `event disable all` / `return`, so
        // it is unguarded — the path-sensitive walk keeps that path's state and
        // flags it even though the other branch returns.
        assert!(
            drop_codes("when CLIENT_ACCEPTED { if {$x} { drop } else { return } }")
                .contains(&"IRULE5002".to_string())
        );
    }

    #[test]
    fn irule5002_quiet_when_every_branch_guarded() {
        // Both branches guard their drop with `return`, so no surviving path
        // is left unguarded.
        assert_eq!(
            drop_codes("when CLIENT_ACCEPTED { if {$x} { drop; return } else { reject; return } }")
                .len(),
            0
        );
    }

    /// A drop or DNS answer in a block the solver proves unreachable leaves
    /// nothing unguarded; a live arm's still does.
    #[test]
    fn irule5002_ignores_a_drop_in_a_dead_arm() {
        for source in [
            "when CLIENT_ACCEPTED { if {0} { drop } }",
            "when CLIENT_ACCEPTED { set flag 0; if {$flag} { reject } }",
            "when CLIENT_ACCEPTED { while {0} { discard } }",
        ] {
            assert!(
                drop_codes(source).is_empty(),
                "{source}: {:?}",
                drop_codes(source)
            );
        }
        for source in [
            "when CLIENT_ACCEPTED { if {1} { drop } }",
            "when CLIENT_ACCEPTED { if {$x} { drop } }",
        ] {
            assert!(
                drop_codes(source).contains(&"IRULE5002".to_owned()),
                "{source}: {:?}",
                drop_codes(source),
            );
        }
    }

    #[test]
    fn irule5004_ignores_a_dns_return_in_a_dead_arm() {
        assert!(drop_codes("when DNS_REQUEST { if {0} { DNS::return \"1.2.3.4\" } }").is_empty());
        assert!(
            drop_codes("when DNS_REQUEST { if {$x} { DNS::return \"1.2.3.4\" } }")
                .contains(&"IRULE5004".to_owned())
        );
    }

    #[test]
    fn irule5002_drop_points_at_the_drop_command() {
        // The diagnostic span must land on the `drop` word itself (offset 23
        // in this source), not past it — the structured-IR walk carries the
        // command's absolute span.
        let ws = drop_warnings("when CLIENT_ACCEPTED { drop }");
        let w = ws
            .iter()
            .find(|w| w.code == DiagCode::Irule5002)
            .expect("IRULE5002");
        assert_eq!(w.span.start(), 23, "span should point at `drop`");
    }

    #[test]
    fn irule5002_carries_insertion_fix() {
        // The quick-fix inserts `event disable all` + `return` *after* the drop
        // (a zero-width edit anchored at the drop command's end), independent of
        // the diagnostic span.
        let ws = drop_warnings("when CLIENT_ACCEPTED { drop }");
        let w = ws
            .iter()
            .find(|w| w.code == DiagCode::Irule5002)
            .expect("IRULE5002");
        assert_eq!(w.fixes.len(), 1, "expected one fix, got {:?}", w.fixes);
        let fix = &w.fixes[0];
        assert_eq!(fix.new_text, "\n    event disable all\n    return");
        assert_eq!(fix.description, "Add 'event disable all' + 'return'");
        // Zero-width insertion at the drop command's end.
        assert_eq!(fix.span.start(), w.span.end());
        assert_eq!(fix.span.start(), fix.span.end());
    }

    #[test]
    fn irule5004_carries_insertion_fix() {
        let ws = drop_warnings("when DNS_REQUEST { DNS::return }");
        let w = ws
            .iter()
            .find(|w| w.code == DiagCode::Irule5004)
            .expect("IRULE5004");
        assert_eq!(w.fixes.len(), 1);
        let fix = &w.fixes[0];
        assert_eq!(fix.new_text, "\n    return");
        assert_eq!(fix.description, "Add 'return' after DNS::return");
        assert_eq!(fix.span.start(), fix.span.end());
    }

    #[test]
    fn irule5004_fires_through_catch_body() {
        // A `DNS::return` inside `catch { … }` still leaves the path needing a
        // following `return` (the linear scan missed the catch body).
        assert!(
            drop_codes("when DNS_REQUEST { catch { DNS::return } }")
                .contains(&"IRULE5004".to_string())
        );
    }

    #[test]
    fn irule5002_drop_and_return_inside_catch_still_warns() {
        // `catch` swallows the `return`, so it does NOT stop iRule processing —
        // the unguarded `drop` still leaks past the catch and must be flagged.
        let ws = drop_warnings("when CLIENT_ACCEPTED {\n    catch { drop; return }\n}");
        let hits: Vec<_> = ws
            .iter()
            .filter(|w| w.code == DiagCode::Irule5002)
            .collect();
        assert_eq!(hits.len(), 1, "expected one IRULE5002, got {ws:?}");
        assert!(
            hits[0].message.contains("drop"),
            "expected the unguarded drop to be flagged, got {:?}",
            hits[0].message,
        );
    }

    #[test]
    fn irule5002_catch_body_without_return_still_warns_drop() {
        // A catch whose body does not return continues normally and the
        // unguarded drop still leaks out.
        let ws = drop_warnings("when CLIENT_ACCEPTED {\n    catch { drop }\n}");
        assert_eq!(
            ws.iter().filter(|w| w.code == DiagCode::Irule5002).count(),
            1,
            "expected one IRULE5002, got {ws:?}",
        );
    }

    #[test]
    fn irule5002_catch_without_drop_no_warning() {
        // A benign catch body must not spuriously warn.
        let ws = drop_warnings("when CLIENT_ACCEPTED {\n    catch { set x 1; return }\n}");
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule5002),
            "no IRULE5002 expected for a benign catch, got {ws:?}",
        );
    }

    #[test]
    fn irule5004_dns_return_and_return_inside_catch_still_warns() {
        // `catch` swallows the `return` so it does not stop processing — the
        // unguarded `DNS::return` still leaks out.
        let ws = drop_warnings("when DNS_REQUEST {\n    catch { DNS::return; return }\n}");
        let hits: Vec<_> = ws
            .iter()
            .filter(|w| w.code == DiagCode::Irule5004)
            .collect();
        assert_eq!(hits.len(), 1, "expected one IRULE5004, got {ws:?}");
        assert!(
            hits[0].message.contains("DNS::return"),
            "expected the unguarded DNS::return to be flagged, got {:?}",
            hits[0].message,
        );
    }

    // IRULE1005 / 1006 / 1007 / 1008

    fn source_unit(
        source: &str,
    ) -> (
        CompilationUnit,
        std::sync::Arc<tcl_registry::model::ContextRegistry>,
    ) {
        let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
        let context = environment.default_context_registry();
        let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
        let input = crate::analyser::ResolvedAnalysisInput::new(
            environment.analyser_profile(),
            environment.unit_profile(),
            context.clone(),
            config,
        );
        let cu = CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(environment.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        );
        (cu, context)
    }

    fn source_flow_warnings(source: &str) -> Vec<IrulesCheckWarning> {
        let (cu, context) = source_unit(source);
        find_conditional_irule_source_flow_advice(
            &cu,
            context.commands(),
            Some(SurfaceQuery::any_release(Family::F5Irules)),
        )
        .map_or_else(Vec::new, |advice| {
            advice
                .findings
                .iter()
                .map(|finding| finding.warning.clone())
                .collect()
        })
    }

    fn flow_warnings(source: &str) -> Vec<IrulesCheckWarning> {
        let (cu, context) = source_unit(source);
        find_conditional_collection_source_pairing_advice(
            &cu,
            context.commands(),
            Some(SurfaceQuery::any_release(Family::F5Irules)),
        )
        .map_or_else(Vec::new, |advice| {
            advice
                .pairings
                .iter()
                .map(|pairing| pairing.warning.clone())
                .collect()
        })
    }

    #[test]
    fn irule1005_http_data_event_without_collect_fires() {
        // HTTP_REQUEST_DATA has no transport alternative: HTTP::collect is
        // required for it to fire.
        let ws = flow_warnings("when HTTP_REQUEST_DATA { log local0. \"data\" }");
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule1005),
            "expected IRULE1005, got {ws:?}",
        );
    }

    #[test]
    fn irule1005_retains_the_original_event_operand_instead_of_a_body_anchor() {
        // naming.consumer.original-diagnostic-source-actions
        // docs/design/analysis/name-resolution-proofs/original-diagnostic-source-actions.md
        // Source geometry, with no Native event/TMM activation claim.
        for source in [
            "set marker 😀\nwhen {HTTP_REQUEST_DATA} {set body value}\n",
            "when HTTP_REQUEST_DATA priority 123 { }\n",
        ] {
            let reg = registry();
            let cu = CompilationUnit::build_for(source, &reg, false);
            let diagnostics = find_collect_flow_warnings(
                &cu,
                &reg,
                Some(SurfaceQuery::any_release(Family::F5Irules)),
            );
            let diagnostic = diagnostics
                .iter()
                .find(|row| row.code == DiagCode::Irule1005)
                .expect("source event requiring collection");
            let body = cu.ir_module.irules_event_bodies.values().next().unwrap();
            let word = body
                .event_word(&cu.ir_module.source, cu.ir_module.lexer_config, &reg)
                .expect("genuine complete original event operand");
            assert_eq!(diagnostic.span, word.span());
            let written = &source[word.span().as_range()];
            assert!(matches!(
                written,
                "{HTTP_REQUEST_DATA}" | "HTTP_REQUEST_DATA"
            ));
            assert!(
                body.event_word(
                    &tcl_lexer::SourceImage::document(&format!("#{source}")),
                    cu.ir_module.lexer_config,
                    &reg
                )
                .is_none()
            );
            let mut stale_config = cu.ir_module.lexer_config;
            stale_config.strict_quoting ^= true;
            assert!(
                body.event_word(&cu.ir_module.source, stale_config, &reg)
                    .is_none()
            );
        }
    }

    #[test]
    fn irule1005_satisfied_by_matching_collect() {
        // HTTP_REQUEST issues a client-side HTTP::collect, satisfying the
        // HTTP_REQUEST_DATA requirement.
        let ws = flow_warnings(
            "when HTTP_REQUEST { HTTP::collect }
             when HTTP_REQUEST_DATA { log local0. \"data\" }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule1005),
            "no IRULE1005 expected — CLIENT_ACCEPTED supplies the collect, got {ws:?}",
        );
    }

    #[test]
    fn irule1006_payload_without_collect_fires() {
        let ws = flow_warnings("when HTTP_REQUEST { HTTP::payload }");
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule1006),
            "expected IRULE1006 on HTTP::payload without HTTP::collect, got {ws:?}",
        );
    }

    #[test]
    fn udp_and_asm_payloads_do_not_require_collect() {
        // F5 exposes every UDP datagram to UDP::payload, and ASM owns the
        // payload in ASM events. Neither protocol has a corresponding
        // `::collect` command, so suffix-based classification was a false
        // positive for both.
        let udp = flow_warnings("when CLIENT_DATA { UDP::payload }");
        assert!(
            !udp.iter()
                .any(|w| w.code == DiagCode::Irule1005 || w.code == DiagCode::Irule1006),
            "UDP data is available without collect, got {udp:?}",
        );
        let asm = flow_warnings("when ASM_REQUEST_VIOLATION { ASM::payload }");
        assert!(
            !asm.iter().any(|w| w.code == DiagCode::Irule1006),
            "ASM owns its payload, got {asm:?}",
        );
    }

    #[test]
    fn mqtt_payload_call_forms_have_distinct_collection_requirements() {
        let bare = flow_warnings("when MQTT_CLIENT_DATA { set bytes [MQTT::payload] }");
        assert!(
            bare.iter()
                .any(|warning| warning.code == DiagCode::Irule1006),
            "bare MQTT payload requires a prior MQTT::collect, got {bare:?}",
        );

        let append = flow_warnings("when MQTT_CLIENT_DATA { MQTT::payload append bytes }");
        assert!(
            append
                .iter()
                .any(|warning| warning.code == DiagCode::Irule1006),
            "MQTT payload append requires collected bytes, got {append:?}",
        );

        for form in ["length", "replace bytes", "prepend bytes"] {
            let source = format!("when MQTT_CLIENT_INGRESS {{ MQTT::payload {form} }}");
            let warnings = flow_warnings(&source);
            assert!(
                !warnings
                    .iter()
                    .any(|warning| warning.code == DiagCode::Irule1006),
                "MQTT payload {form} uses the current message without collect, got {warnings:?}",
            );
        }

        let dynamic = flow_warnings("when MQTT_CLIENT_DATA { MQTT::payload $operation bytes }");
        assert!(
            !dynamic
                .iter()
                .any(|warning| warning.code == DiagCode::Irule1006),
            "a dynamic MQTT payload form must abstain, got {dynamic:?}",
        );
    }

    #[test]
    fn mqtt_collection_lifecycle_balances_across_registry_events() {
        let warnings = flow_warnings(
            "when MQTT_CLIENT_INGRESS { MQTT::collect 200 }
             when MQTT_CLIENT_DATA { set bytes [MQTT::payload]; MQTT::release }",
        );
        assert!(
            !warnings.iter().any(|warning| matches!(
                warning.code,
                DiagCode::Irule1005
                    | DiagCode::Irule1006
                    | DiagCode::Irule1007
                    | DiagCode::Irule1008
            )),
            "balanced MQTT collection should be clean, got {warnings:?}",
        );
    }

    #[test]
    fn every_explicit_payload_family_warns_without_collect() {
        for (protocol, event) in [
            ("MR", "MR_DATA"),
            ("RTSP", "RTSP_REQUEST_DATA"),
            ("SCTP", "CLIENT_DATA"),
            ("WS", "WS_CLIENT_DATA"),
        ] {
            let source = format!("when {event} {{ set bytes [{protocol}::payload] }}");
            let warnings = flow_warnings(&source);
            assert!(
                warnings
                    .iter()
                    .any(|warning| warning.code == DiagCode::Irule1006),
                "{protocol} payload without collect must warn, got {warnings:?}",
            );
        }
    }

    #[test]
    fn event_owned_payload_families_do_not_invent_collect_lifecycles() {
        for (protocol, event) in [
            ("CACHE", "CACHE_RESPONSE"),
            ("DIAMETER", "DIAMETER_INGRESS"),
            ("GTP", "GTP_SIGNALLING_INGRESS"),
            ("REWRITE", "REWRITE_REQUEST_DONE"),
            ("SIP", "SIP_REQUEST"),
            ("XML", "XML_CONTENT_BASED_ROUTING"),
        ] {
            let source = format!("when {event} {{ set bytes [{protocol}::payload] }}");
            let warnings = flow_warnings(&source);
            assert!(
                !warnings
                    .iter()
                    .any(|warning| warning.code == DiagCode::Irule1006),
                "{protocol} payload is event-owned, got {warnings:?}",
            );
        }
    }

    #[test]
    fn irule1007_collect_without_release_fires() {
        let ws = flow_warnings("when CLIENT_ACCEPTED { TCP::collect }");
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule1007),
            "expected IRULE1007, got {ws:?}",
        );
    }

    #[test]
    fn irule1008_release_without_collect_fires() {
        let ws = flow_warnings("when CLIENT_ACCEPTED { TCP::release }");
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule1008),
            "expected IRULE1008, got {ws:?}",
        );
    }

    #[test]
    fn irule1007_satisfied_by_matching_release_same_side() {
        let ws = flow_warnings(
            "when CLIENT_ACCEPTED { TCP::collect }
             when CLIENT_DATA { TCP::release }",
        );
        // Both sides — same side — should NOT fire 1007 or 1008.
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule1007),
            "1007 unexpected, got {ws:?}",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule1008),
            "1008 unexpected, got {ws:?}",
        );
    }

    #[test]
    fn http_data_event_implicitly_releases_collection() {
        // F5 releases HTTP collection when the matching DATA event completes
        // unless another HTTP::collect starts. An explicit HTTP::release here
        // is optional, unlike TCP and SSL.
        let ws = flow_warnings(
            "when HTTP_REQUEST { HTTP::collect }
             when HTTP_REQUEST_DATA { set payload [HTTP::payload] }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule1007),
            "HTTP data event supplies the release, got {ws:?}",
        );
    }

    #[test]
    fn http_response_data_implicitly_releases_server_side_collection() {
        let ws = flow_warnings(
            "when HTTP_RESPONSE { HTTP::collect }
             when HTTP_RESPONSE_DATA { set payload [HTTP::payload] }",
        );
        assert!(
            !ws.iter().any(|warning| matches!(
                warning.code,
                DiagCode::Irule1005 | DiagCode::Irule1006 | DiagCode::Irule1007
            )),
            "registry side and implicit release should satisfy response collection, got {ws:?}",
        );
    }

    #[test]
    fn recollect_inside_http_data_event_still_needs_a_later_release() {
        let ws = flow_warnings(
            "when HTTP_REQUEST { HTTP::collect }
             when HTTP_REQUEST_DATA { HTTP::collect }",
        );
        let hits = ws
            .iter()
            .filter(|warning| warning.code == DiagCode::Irule1007)
            .count();
        assert_eq!(
            hits, 1,
            "the setup collect is implicit-release safe, but the DATA-event recollect is not: {ws:?}",
        );
    }

    // Side-switch body descent and peer flip.

    #[test]
    fn collect_inside_clientside_body_is_seen() {
        // The collect lives inside a `clientside { ... }` nesting script.
        // With the BODY role set, the flow check descends into the body
        // and registers the (client-side) collect, so CLIENTSSL_DATA's
        // IRULE1005 requirement is satisfied.  Without the descent the
        // collect was invisible and CLIENTSSL_DATA wrongly fired IRULE1005.
        let ws = flow_warnings(
            "when CLIENTSSL_HANDSHAKE { clientside { SSL::collect } }
             when CLIENTSSL_DATA { log local0. \"data\" }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule1005),
            "no IRULE1005 expected — the clientside body supplies the collect, got {ws:?}",
        );
    }

    #[test]
    fn peer_flips_side_for_collect_flow() {
        // `peer { SSL::collect }` evaluates under the *opposite* side of
        // the surrounding client-side event. It therefore satisfies
        // SERVERSSL_DATA but not CLIENTSSL_DATA — the inverse of a plain
        // client-side collection.
        let ws = flow_warnings(
            "when CLIENTSSL_HANDSHAKE { peer { SSL::collect } }
             when CLIENTSSL_DATA { log local0. \"c\" }
             when SERVERSSL_DATA { log local0. \"s\" }",
        );
        let irule1005: Vec<&str> = ws
            .iter()
            .filter(|w| w.code == DiagCode::Irule1005)
            .map(|w| w.message.as_str())
            .collect();
        assert_eq!(
            irule1005.len(),
            1,
            "expected exactly one IRULE1005 (CLIENTSSL_DATA only), got {ws:?}",
        );
        assert!(
            irule1005[0].contains("CLIENTSSL_DATA"),
            "the surviving IRULE1005 must be CLIENTSSL_DATA (peer flipped the collect to the server side), got {irule1005:?}",
        );
    }

    /// A collect or payload read inside a `clientside` / `serverside` body,
    /// in an arm of that body the solver proves unreachable, is not seen.
    #[test]
    fn irule1007_ignores_a_collect_in_a_dead_side_switch_arm() {
        let live =
            flow_warnings("when CLIENT_ACCEPTED { clientside { if {$x} { TCP::collect 1024 } } }");
        assert!(
            live.iter().any(|w| w.code == DiagCode::Irule1007),
            "expected IRULE1007, got {live:?}",
        );
        let dead =
            flow_warnings("when CLIENT_ACCEPTED { clientside { if {0} { TCP::collect 1024 } } }");
        assert!(
            !dead.iter().any(|w| w.code == DiagCode::Irule1007),
            "no IRULE1007 expected, got {dead:?}",
        );
        let dead_payload = flow_warnings(
            "when CLIENT_ACCEPTED { serverside { set n 0; if {$n} { TCP::payload } } }",
        );
        assert!(
            !dead_payload.iter().any(|w| w.code == DiagCode::Irule1006),
            "no IRULE1006 expected, got {dead_payload:?}",
        );
    }

    #[test]
    fn nested_side_switch_warning_keeps_its_absolute_source_span() {
        let source = "when CLIENT_ACCEPTED {\n  serverside {\n    TCP::payload\n  }\n}";
        let warning = flow_warnings(source)
            .into_iter()
            .find(|warning| warning.code == DiagCode::Irule1006)
            .expect("server-side payload without collect");
        let start = usize::try_from(warning.span.start()).unwrap();
        let end = usize::try_from(warning.span.end()).unwrap();
        assert_eq!(&source[start..end], "TCP::payload");
    }

    #[test]
    fn side_switch_specs_have_body_role_and_arity() {
        // Registry-level guard for the spec edits.
        let r = registry();
        for cmd in ["clientside", "serverside", "peer"] {
            assert!(r.is_side_switch(cmd), "{cmd} must be a side-switch");
            assert!(
                r.side_switch_target(cmd).is_some(),
                "{cmd} must declare its side-switch target",
            );
            assert_eq!(
                r.arg_indices_for_role(cmd, &["{ TCP::collect }"], tcl_registry::ArgRole::Body),
                vec![0],
                "{cmd} body script must be ArgRole::Body at index 0",
            );
        }
        // clientside/serverside accept the bare query form or one body;
        // peer requires its body; after's timer body is the trailing arg.
        let cs = r.get("clientside").unwrap().arity;
        assert_eq!((cs.min, cs.max), (0, 1), "clientside arity");
        let ss = r.get("serverside").unwrap().arity;
        assert_eq!((ss.min, ss.max), (0, 1), "serverside arity");
        let pr = r.get("peer").unwrap().arity;
        assert_eq!((pr.min, pr.max), (1, 1), "peer arity");
        // `after`'s deferred timer body runs in its own dispatch context.
        assert_eq!(
            r.get("after").unwrap().body_kind,
            tcl_registry::BodyKind::Structural,
            "after timer body must be Structural",
        );
    }

    #[test]
    fn collect_flow_only_in_irules_dialect() {
        let reg = registry();
        let cu = CompilationUnit::build_for("when CLIENT_ACCEPTED { TCP::collect }", &reg, false);
        let none = find_collect_flow_warnings(&cu, &reg, None);
        assert_eq!(none.len(), 0, "got {none:?}");
    }

    // IRULE1201 / 1202

    fn http_warnings(source: &str) -> Vec<IrulesCheckWarning> {
        source_flow_warnings(source)
            .into_iter()
            .filter(|warning| matches!(warning.code, DiagCode::Irule1201 | DiagCode::Irule1202))
            .collect()
    }

    #[test]
    fn irule1202_double_respond_fires() {
        let ws = http_warnings(
            "when HTTP_REQUEST { HTTP::respond 200 content x; HTTP::respond 404 content y }",
        );
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule1202),
            "expected IRULE1202, got {ws:?}",
        );
    }

    #[test]
    fn irule1201_http_after_respond_fires() {
        let ws = http_warnings(
            "when HTTP_REQUEST { HTTP::respond 200 content ok; HTTP::header Cache-Control no-cache }",
        );
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule1201),
            "expected IRULE1201, got {ws:?}",
        );
    }

    #[test]
    fn irule1201_uses_the_event_profile_instead_of_its_name() {
        // FN regression: ASM_RESPONSE_VIOLATION does not start with `HTTP`,
        // but its event descriptor implies the HTTP profile.
        let ws = http_warnings(
            "when ASM_RESPONSE_VIOLATION { HTTP::respond 200 content ok; HTTP::header insert X-Test yes }",
        );
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule1201),
            "expected profile-driven IRULE1201, got {ws:?}",
        );
    }

    #[test]
    fn irule1201_allows_the_registered_response_state_query() {
        // FP regression: checking whether a response has been sent is the
        // purpose of HTTP::has_responded, so its spec does not require the
        // still-live transaction context.
        let ws = http_warnings(
            "when HTTP_REQUEST { HTTP::respond 200 content ok; set sent [HTTP::has_responded] }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule1201),
            "HTTP::has_responded must remain valid after commit, got {ws:?}",
        );
    }

    #[test]
    fn irule1201_clean_when_respond_followed_by_return_only() {
        let ws = http_warnings("when HTTP_REQUEST { HTTP::respond 200 content ok; return }");
        assert!(
            !ws.iter()
                .any(|w| w.code == DiagCode::Irule1201 || w.code == DiagCode::Irule1202),
            "no IRULE1201/1202 expected, got {ws:?}",
        );
    }

    #[test]
    fn irule1201_respond_then_return_inside_catch_warns() {
        // `catch` swallows the `return`, so control continues past the catch
        // with the response already committed — the post-catch HTTP::header
        // insert must still be flagged, and the at-catch-return `responded`
        // state must propagate out.
        let ws = http_warnings(
            "when HTTP_REQUEST {\n\
            \x20   catch { HTTP::respond 200 content ok; return }\n\
            \x20   HTTP::header insert X-Debug \"yes\"\n\
            }",
        );
        let hits: Vec<_> = ws
            .iter()
            .filter(|w| w.code == DiagCode::Irule1201)
            .collect();
        assert_eq!(hits.len(), 1, "expected one IRULE1201, got {ws:?}");
        assert!(
            hits[0].message.contains("HTTP::header"),
            "expected the post-catch HTTP::header to be flagged, got {:?}",
            hits[0].message,
        );
    }

    #[test]
    fn irule1201_catch_body_without_return_still_continues() {
        // A catch whose body does not return continues normally, carrying the
        // responded state to flag the post-catch command.
        let ws = http_warnings(
            "when HTTP_REQUEST {\n\
            \x20   catch { HTTP::respond 200 content ok }\n\
            \x20   HTTP::header insert X-Debug \"yes\"\n\
            }",
        );
        assert_eq!(
            ws.iter().filter(|w| w.code == DiagCode::Irule1201).count(),
            1,
            "expected one IRULE1201, got {ws:?}",
        );
    }

    #[test]
    fn irule1201_catch_without_respond_no_warning() {
        // A benign catch body must not spuriously flag post-catch HTTP
        // commands.
        let ws = http_warnings(
            "when HTTP_REQUEST {\n\
            \x20   catch { set x 1; return }\n\
            \x20   HTTP::header insert X-Debug 1\n\
            }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule1201),
            "no IRULE1201 expected for a benign catch, got {ws:?}",
        );
    }

    #[test]
    fn irule1201_only_in_http_events() {
        // CLIENT_ACCEPTED is non-HTTP; HTTP::respond there is silly
        // but IRULE1201/1202 doesn't apply to non-HTTP events.
        let ws = http_warnings(
            "when CLIENT_ACCEPTED { HTTP::respond 200 content x; HTTP::respond 404 content y }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule1202),
            "IRULE1202 should not fire outside HTTP events, got {ws:?}",
        );
    }

    fn http_codes(source: &str) -> Vec<String> {
        let mut c: Vec<String> = http_warnings(source)
            .iter()
            .map(|w| w.code.to_string())
            .collect();
        c.sort();
        c
    }

    #[test]
    fn irule1202_quiet_on_mutually_exclusive_branches() {
        // A respond in each arm of an if/else is NOT a double-respond — only
        // one executes per path.  The path-sensitive walk keeps the branch
        // states separate (the linear scan wrongly fired IRULE1202 here).
        assert_eq!(
            http_codes(
                "when HTTP_REQUEST { if {$x} { HTTP::respond 200 content a } \
             else { HTTP::respond 404 content b } }"
            )
            .len(),
            0
        );
        // Same for mutually-exclusive switch arms.
        assert_eq!(
            http_codes(
                "when HTTP_REQUEST { switch $x { a { HTTP::respond 200 content a } \
             b { HTTP::respond 404 content b } } }"
            )
            .len(),
            0
        );
    }

    #[test]
    fn irule1202_reports_one_diagnostic_for_multiple_predecessor_proofs() {
        let ws = http_warnings(
            "when HTTP_REQUEST {\
             if {$x} { HTTP::respond 200 content a } \
             else { HTTP::respond 404 content b }; \
             HTTP::respond 500 content c \
             }",
        );
        let final_respond: Vec<_> = ws
            .iter()
            .filter(|warning| {
                warning.code == DiagCode::Irule1202
                    && warning.message.contains("Multiple 'HTTP::respond'")
            })
            .collect();
        assert_eq!(
            final_respond.len(),
            1,
            "one source command must produce one diagnostic, got {ws:?}",
        );
    }

    #[test]
    fn irule1201_quiet_when_respond_and_use_are_exclusive() {
        // respond in one branch, an HTTP command in the *other* — no path
        // both responds and then uses HTTP, so nothing fires.
        assert_eq!(
            http_codes(
                "when HTTP_REQUEST { if {$x} { HTTP::respond 200 content a } \
             else { HTTP::header insert Foo bar } }"
            )
            .len(),
            0
        );
    }

    #[test]
    fn irule1201_fires_through_catch_body() {
        // A respond inside `catch { … }` still commits the response, so a
        // following HTTP command is flagged (the linear scan missed this).
        assert!(http_codes(
            "when HTTP_REQUEST { catch { HTTP::respond 200 content a }; HTTP::header insert Foo bar }"
        )
        .contains(&"IRULE1201".to_string()));
    }

    #[test]
    fn irule1201_quiet_on_unregistered_http_subcommand() {
        // `HTTP::log` is not a registered HTTP:: command, so it must not trip
        // IRULE1201 (it carries no registry context trait).
        assert!(
            !http_codes("when HTTP_REQUEST { HTTP::respond 200 content a; HTTP::log foo }")
                .contains(&"IRULE1201".to_string())
        );
    }

    #[test]
    fn irule1202_fires_after_loop_respond() {
        // A respond in a loop body, then a respond after the loop: when the
        // loop runs at least once, the second respond is a double-respond.
        assert!(
            http_codes(
                "when HTTP_REQUEST { foreach i $list { HTTP::respond 200 content a }; \
             HTTP::respond 404 content b }"
            )
            .contains(&"IRULE1202".to_string())
        );
    }

    /// A respond or redirect in a block the solver proves unreachable commits
    /// no response, so an HTTP command after it is not issued after one, and
    /// an HTTP command in such a block is issued after none; a live arm's
    /// respond still commits.
    #[test]
    fn irule1201_ignores_a_respond_in_a_dead_arm() {
        for source in [
            "when HTTP_REQUEST { if {0} { HTTP::respond 200 }; HTTP::header insert X-Custom val }",
            "when HTTP_REQUEST { set flag 0; if {$flag} { HTTP::respond 200 }; \
             HTTP::header insert X-Custom val }",
            "when HTTP_REQUEST { if {0} { HTTP::redirect \"https://example.com/\" }; \
             HTTP::header insert X-Custom val }",
            "when HTTP_REQUEST { if {1} { log local0. ok } else { HTTP::respond 200 }; \
             HTTP::header insert X-Custom val }",
            "when HTTP_REQUEST { while {0} { HTTP::respond 200 }; \
             HTTP::header insert X-Custom val }",
            "when HTTP_REQUEST { set mode b; \
             switch $mode { a { HTTP::respond 200 } default { log local0. other } }; \
             HTTP::header insert X-Custom val }",
            "when HTTP_REQUEST { HTTP::respond 200; if {1} { return }; \
             HTTP::header insert X-Custom val }",
        ] {
            assert!(
                http_codes(source).is_empty(),
                "{source}: {:?}",
                http_codes(source)
            );
        }
        for source in [
            "when HTTP_REQUEST { if {1} { HTTP::respond 200 }; HTTP::header insert X-Custom val }",
            "when HTTP_REQUEST { set flag 1; if {$flag} { HTTP::respond 200 }; \
             HTTP::header insert X-Custom val }",
            "when HTTP_REQUEST { if {$x} { HTTP::respond 200 }; HTTP::header insert X-Custom val }",
            "when HTTP_REQUEST { if {0} { log local0. no } else { HTTP::respond 200 }; \
             HTTP::header insert X-Custom val }",
            "when HTTP_REQUEST { set mode a; \
             switch $mode { a { HTTP::respond 200 } default { log local0. other } }; \
             HTTP::header insert X-Custom val }",
        ] {
            assert!(
                http_codes(source).contains(&"IRULE1201".to_owned()),
                "{source}: {:?}",
                http_codes(source),
            );
        }
    }

    /// A second response in an arm that never runs is not a second response,
    /// and a first one that never runs commits nothing for the second to
    /// follow.
    #[test]
    fn irule1202_ignores_a_respond_in_a_dead_arm() {
        for source in [
            "when HTTP_REQUEST { HTTP::respond 200; if {0} { HTTP::respond 404 } }",
            "when HTTP_REQUEST { if {0} { HTTP::respond 200 }; HTTP::respond 404 }",
            "when HTTP_REQUEST { set flag 0; \
             if {$flag} { HTTP::redirect \"https://example.com/\" }; HTTP::respond 404 }",
        ] {
            assert!(
                http_codes(source).is_empty(),
                "{source}: {:?}",
                http_codes(source)
            );
        }
        for source in [
            "when HTTP_REQUEST { HTTP::respond 200; if {1} { HTTP::respond 404 } }",
            "when HTTP_REQUEST { HTTP::respond 200; if {$x} { HTTP::respond 404 } }",
        ] {
            assert!(
                http_codes(source).contains(&"IRULE1202".to_owned()),
                "{source}: {:?}",
                http_codes(source),
            );
        }
    }

    #[test]
    fn bare_redirect_commits_response() {
        // The bare iRules `redirect` (not just `HTTP::redirect`) commits a
        // response — derived from the registry's ResponseCommit side-effect.
        assert!(commits_response_commands().contains("redirect"));
        assert!(commits_response_commands().contains("HTTP::respond"));
    }

    // IRULE4004

    fn hoist_warnings(source: &str) -> Vec<IrulesCheckWarning> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        find_hoistable_set_warnings(&cu, Some(SurfaceQuery::any_release(Family::F5Irules)))
    }

    #[test]
    fn irule4004_literal_set_in_per_request_event_fires() {
        let ws = hoist_warnings(r#"when HTTP_REQUEST { set svc "foo" }"#);
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "expected IRULE4004, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_registry_only_per_request_event_fires() {
        // TP: the registry's per-request classification is the only source —
        // a compiler-local event list would not carry ASM_REQUEST_DONE.
        let ws = hoist_warnings(r#"when ASM_REQUEST_DONE { set svc "foo" }"#);
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "expected registry-driven IRULE4004, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_dynamic_set_clean() {
        // Value depends on per-request data — can't hoist.
        let ws = hoist_warnings("when HTTP_REQUEST { set svc [HTTP::host] }");
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "no IRULE4004 expected — value depends on request, got {ws:?}",
        );
    }

    /// A value the lattice proves reads no request data, whatever its
    /// commands: `[string range ABCDEFG 0 3]` is `ABCD` on every
    /// request and hoists like a literal, where the same range over the host
    /// header does not.
    #[test]
    fn irule4004_proven_command_value_is_hoistable() {
        let ws = hoist_warnings("when HTTP_REQUEST { set svc [string range ABCDEFG 0 3] }");
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "expected IRULE4004 on a proven value, got {ws:?}",
        );
        let ws = hoist_warnings("when HTTP_REQUEST { set svc [string range [HTTP::host] 0 3] }");
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "no IRULE4004 expected — the value reads the request, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_var_substitution_clean() {
        let ws = hoist_warnings("when HTTP_REQUEST { set svc $session }");
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "no IRULE4004 expected — value uses $session, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_per_request_reset_is_not_hoistable() {
        // FP regression from the third-party corpus: moving the initial zero
        // changes later requests after a branch sets the connection variable.
        let ws = hoist_warnings(
            "when HTTP_REQUEST { set is_reject 0; if {[HTTP::path] eq \"/deny\"} { set is_reject 1 } }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "a per-request reset must stay in the request event, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_conditional_literal_set_is_not_hoistable() {
        let ws = hoist_warnings(
            "when HTTP_REQUEST { if {[HTTP::path] eq \"/health\"} { set health_check 1 } }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "hoisting a conditional assignment would make it unconditional, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_cross_event_write_is_not_hoistable() {
        let ws =
            hoist_warnings("when HTTP_REQUEST { set state 0 }\nwhen HTTP_RESPONSE { set state 1 }");
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "a connection variable written by another event is not invariant, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_counts_writes_in_complexity_guarded_events() {
        let padding = "x".repeat(crate::ssa::DEEP_ANALYSIS_BODY_BYTES);
        let source = format!(
            "when HTTP_REQUEST {{ set state 0 }}\n\
             when HTTP_RESPONSE {{ # {padding}\n set state 1 }}"
        );
        let cu = CompilationUnit::build_for(&source, &registry(), false);
        let response = cu
            .functions()
            .find(|fu| fu.name.starts_with("::when::HTTP_RESPONSE"))
            .expect("HTTP_RESPONSE event is lowered");
        assert!(
            response.complexity_guarded,
            "the control event must be guarded"
        );

        let ws =
            find_hoistable_set_warnings(&cu, Some(SurfaceQuery::any_release(Family::F5Irules)));
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "a guarded event's second write must suppress the hoist warning, got {ws:?}",
        );
    }

    /// A second write in a block the solver proves unreachable never runs,
    /// so the first is still the variable's only write; one that runs, or
    /// might, keeps the assignment where it is.
    #[test]
    fn irule4004_counts_only_the_writes_that_run() {
        for source in [
            "when HTTP_REQUEST { set svc foo; if {0} { set svc bar } }",
            "when HTTP_REQUEST { set svc foo }\nwhen HTTP_RESPONSE { if {0} { set svc bar } }",
        ] {
            assert!(
                hoist_warnings(source)
                    .iter()
                    .any(|w| w.code == DiagCode::Irule4004),
                "{source}"
            );
        }
        for source in [
            "when HTTP_REQUEST { set svc foo; if {1} { set svc bar } }",
            "when HTTP_REQUEST { set svc foo; if {$x} { set svc bar } }",
            "when HTTP_REQUEST { set svc foo }\nwhen HTTP_RESPONSE { if {$x} { set svc bar } }",
        ] {
            assert!(
                !hoist_warnings(source)
                    .iter()
                    .any(|w| w.code == DiagCode::Irule4004),
                "{source}"
            );
        }
    }

    #[test]
    fn irule4004_array_and_static_destinations_abstain() {
        let ws = hoist_warnings(
            "when HTTP_REQUEST { set state(current) 0; set static::feature_enabled 1 }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "array and static variables need different hoist destinations, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_set_in_once_per_connection_clean() {
        // FP control: the registry classifies CLIENT_ACCEPTED as
        // once-per-connection, so there is nothing to hoist.
        let ws = hoist_warnings(r#"when CLIENT_ACCEPTED { set svc "foo" }"#);
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "no IRULE4004 expected — already once-per-connection, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_unknown_event_abstains() {
        // TN: an event absent from the registry is not guessed to be
        // per-request from its spelling.
        let ws = hoist_warnings(r#"when CUSTOM_REQUEST { set svc "foo" }"#);
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "unknown event should abstain, got {ws:?}",
        );
    }

    #[test]
    fn irule4004_only_in_irules_dialect() {
        let cu = CompilationUnit::build_for(
            r#"when HTTP_REQUEST { set svc "foo" }"#,
            &registry(),
            false,
        );
        let none = find_hoistable_set_warnings(&cu, None);
        assert_eq!(none.len(), 0, "got {none:?}");
    }

    // IRULE4002

    fn generic_warnings(source: &str) -> Vec<IrulesCheckWarning> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        find_generic_static_name_warnings(
            &cu,
            Some(SurfaceQuery::any_release(Family::F5Irules)),
            None,
        )
    }

    fn generic_warnings_with(source: &str, patterns: &[String]) -> Vec<IrulesCheckWarning> {
        let cu = CompilationUnit::build_for(source, &registry(), false);
        find_generic_static_name_warnings(
            &cu,
            Some(SurfaceQuery::any_release(Family::F5Irules)),
            Some(patterns),
        )
    }

    #[test]
    fn generic_static_warning_uses_resolved_root_keys_aliases_and_original_source_locations() {
        // naming.analysis.resolved-static-variable-patterns
        // docs/design/analysis/name-resolution-proofs/resolved-static-variable-patterns.md
        // Fixed source-analysis controls; no iRule event or physical TMM executes here.
        for operation in [
            "set static::debug 1",
            "set ::static::debug 1",
            "set ::::static::::debug 1",
            "upvar #0 ::static::debug shared; set shared 1",
        ] {
            let source = format!("when RULE_INIT {{{operation}}}");
            let warnings = generic_warnings(&source);
            let selected: Vec<_> = warnings
                .iter()
                .filter(|warning| warning.code == DiagCode::Irule4002)
                .collect();
            assert_eq!(selected.len(), 1, "{source}: {warnings:?}");
            let text = &source[selected[0].span.start() as usize..selected[0].span.end() as usize];
            assert_eq!(
                text,
                operation
                    .rsplit_once("; ")
                    .map_or(operation, |(_, write)| write)
            );
        }
        let source = "when RULE_INIT {set static::debug 1; set ::static::debug 2}";
        assert_eq!(
            generic_warnings(source)
                .iter()
                .filter(|warning| warning.code == DiagCode::Irule4002)
                .count(),
            1
        );
        for source in [
            "when RULE_INIT {set other::static::debug 1}",
            "when RULE_INIT {set ::static::inner::debug 1}",
            "when HTTP_REQUEST {set debug 1}",
            "when RULE_INIT {if {0} {set ::static::debug 1}}",
        ] {
            assert!(
                generic_warnings(source)
                    .iter()
                    .all(|warning| warning.code != DiagCode::Irule4002),
                "{source}"
            );
        }
        let patterns = vec![r"^debug\(key\)$".to_owned()];
        assert_eq!(
            generic_warnings_with("when RULE_INIT {set ::static::debug(key) 1}", &patterns)
                .iter()
                .filter(|warning| warning.code == DiagCode::Irule4002)
                .count(),
            1
        );
    }

    #[test]
    fn generic_static_warning_declines_foreign_module_source_without_spelling_fallback() {
        // naming.analysis.resolved-static-variable-patterns
        // docs/design/analysis/name-resolution-proofs/resolved-static-variable-patterns.md
        // A real built source Module loses its actual document correspondence.
        let source = "when RULE_INIT {set static::debug 1}";
        let mut unit = CompilationUnit::build_for(source, &registry(), false);
        assert_eq!(
            find_generic_static_name_warnings(
                &unit,
                Some(SurfaceQuery::any_release(Family::F5Irules)),
                None
            )
            .iter()
            .filter(|warning| warning.code == DiagCode::Irule4002)
            .count(),
            1
        );
        unit.ir_module.source = tcl_lexer::SourceImage::document("foreign source");
        assert!(
            find_generic_static_name_warnings(
                &unit,
                Some(SurfaceQuery::any_release(Family::F5Irules)),
                None
            )
            .is_empty()
        );
    }

    #[test]
    fn irule4002_fires_for_generic_static_name() {
        let ws = generic_warnings("when RULE_INIT { set static::debug 1 }");
        assert!(
            ws.iter()
                .any(|w| w.code == DiagCode::Irule4002 && w.message.contains("'static::debug'")),
            "expected IRULE4002 for static::debug, got {ws:?}",
        );
    }

    #[test]
    fn irule4002_quiet_for_specific_static_name() {
        let ws = generic_warnings("when RULE_INIT { set static::myapp_cache_ttl 60 }");
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4002),
            "no IRULE4002 expected for a specific name, got {ws:?}",
        );
    }

    #[test]
    fn irule4002_deduplicates_across_events() {
        let src = "when RULE_INIT { set static::timeout 30 }\n\
                   when HTTP_REQUEST { set static::timeout 30 }";
        let ws = generic_warnings(src);
        assert_eq!(
            ws.iter().filter(|w| w.code == DiagCode::Irule4002).count(),
            1,
            "expected a single deduplicated IRULE4002, got {ws:?}",
        );
    }

    /// A generic `static::` name written only in a block the solver proves
    /// unreachable collides with nothing; the warning lands on the write
    /// that runs.
    #[test]
    fn irule4002_ignores_a_static_in_a_dead_arm() {
        assert!(generic_warnings("when RULE_INIT { if {0} { set static::debug 1 } }").is_empty());
        assert!(
            generic_warnings("when RULE_INIT { if {$x} { set static::debug 1 } }")
                .iter()
                .any(|w| w.code == DiagCode::Irule4002)
        );
        let source = "when RULE_INIT { if {0} { set static::debug 1 }; set static::debug 2 }";
        let ws = generic_warnings(source);
        assert_eq!(ws.len(), 1, "got {ws:?}");
        let span = ws[0].span;
        assert_eq!(
            &source[span.start() as usize..span.end() as usize],
            "set static::debug 2"
        );
    }

    #[test]
    fn irule4002_only_in_irules_dialect() {
        let cu = CompilationUnit::build_for(
            "when RULE_INIT { set static::debug 1 }",
            &registry(),
            false,
        );
        assert_eq!(
            find_generic_static_name_warnings(&cu, None, None),
            [] as [crate::irules_checks::IrulesCheckWarning; 0]
        );
    }

    #[test]
    fn is_generic_static_name_matches_default_set() {
        let compiled = compile_generic_patterns(None);
        assert!(is_generic_static_name("static::debug", &compiled));
        assert!(is_generic_static_name("static::DEBUG", &compiled));
        assert!(is_generic_static_name("static::max_retries", &compiled));
        assert!(!is_generic_static_name("static::myapp_token", &compiled));
        assert!(!is_generic_static_name("static::debugger", &compiled));
    }

    #[test]
    fn irule4002_custom_patterns_replace_defaults() {
        // A custom pattern set replaces the defaults wholesale: `myapp_token`
        // fires, while `debug` — generic under the defaults — stays quiet.
        let patterns = vec![r"^myapp_".to_owned()];
        let ws = generic_warnings_with("when RULE_INIT { set static::myapp_token 1 }", &patterns);
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule4002),
            "expected IRULE4002 for static::myapp_token under custom patterns, got {ws:?}",
        );
        let quiet = generic_warnings_with("when RULE_INIT { set static::debug 1 }", &patterns);
        assert!(
            !quiet.iter().any(|w| w.code == DiagCode::Irule4002),
            "static::debug should be quiet once defaults are replaced, got {quiet:?}",
        );
    }

    #[test]
    fn irule4002_empty_patterns_disable_check() {
        // An explicit empty pattern list disables IRULE4002 entirely.
        let ws = generic_warnings_with("when RULE_INIT { set static::debug 1 }", &[]);
        assert!(
            ws.iter().all(|w| w.code != DiagCode::Irule4002),
            "empty pattern list should disable IRULE4002, got {ws:?}",
        );
    }

    /// A flag a `switch` arm sets is no constant: the `HTTP::respond` under
    /// `if {$is_api}` is reachable, so the header insert after it may follow a
    /// committed response. The same handler with no arm writing the flag
    /// leaves the respond unreachable and reports nothing.
    #[test]
    fn irule1201_reads_a_flag_a_switch_arm_sets() {
        let written = concat!(
            "when HTTP_REQUEST { set is_api 0; switch -glob [HTTP::uri] { \"/api*\" { set is_api 1 } }; ",
            "if {$is_api} { HTTP::respond 403 }; HTTP::header insert X-Seen 1 }"
        );
        let ws = http_warnings(written);
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule1201),
            "expected IRULE1201, got {ws:?}",
        );
        let untouched = concat!(
            "when HTTP_REQUEST { set is_api 0; switch -glob [HTTP::uri] { \"/api*\" { set other 1 } }; ",
            "if {$is_api} { HTTP::respond 403 }; HTTP::header insert X-Seen 1 }"
        );
        let ws = http_warnings(untouched);
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule1201),
            "the respond is unreachable, got {ws:?}",
        );
    }

    /// The same for a drop under a flag a `switch` arm sets.
    #[test]
    fn irule5002_reads_a_flag_a_switch_arm_sets() {
        let written = concat!(
            "when CLIENT_ACCEPTED { set bad 0; switch -glob [IP::client_addr] { 10.* { set bad 1 } }; ",
            "if {$bad} { drop } }"
        );
        assert!(
            drop_codes(written).contains(&"IRULE5002".to_owned()),
            "{:?}",
            drop_codes(written),
        );
        let untouched = concat!(
            "when CLIENT_ACCEPTED { set bad 0; switch -glob [IP::client_addr] { 10.* { set other 1 } }; ",
            "if {$bad} { drop } }"
        );
        assert!(
            drop_codes(untouched).is_empty(),
            "{:?}",
            drop_codes(untouched)
        );
    }

    /// A write in an arm of a `switch` the flow graph keeps as one statement
    /// counts: the `set svc foo` is one of two writes, so hoisting it to a
    /// once-per-connection event would change what a later request sees.
    #[test]
    fn irule4004_counts_the_writes_a_switch_arm_makes() {
        let ws = hoist_warnings(
            "when HTTP_REQUEST { set svc foo; switch -glob [HTTP::uri] { /a* { set svc bar } }; pool $svc }",
        );
        assert!(
            !ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "no IRULE4004 expected — an arm writes svc too, got {ws:?}",
        );
        let ws = hoist_warnings(
            "when HTTP_REQUEST { set svc foo; switch -glob [HTTP::uri] { /a* { set other bar } }; pool $svc }",
        );
        assert!(
            ws.iter().any(|w| w.code == DiagCode::Irule4004),
            "expected IRULE4004 when no arm writes svc, got {ws:?}",
        );
    }
}

#[cfg(test)]
mod original_getter_source_config_tests {

    use super::*;
    use crate::analyser::ResolvedAnalysisInput;
    use crate::compilation_unit::UnitBuildOptions;
    use std::sync::Arc;

    fn supplied(
        config: tcl_lexer::LexerConfig,
    ) -> (CompilationUnit, Arc<tcl_registry::model::ContextRegistry>) {
        supplied_source("when HTTP_REQUEST {set u [HTTP::uri]}", config)
    }

    fn supplied_source(
        source: &str,
        config: tcl_lexer::LexerConfig,
    ) -> (CompilationUnit, Arc<tcl_registry::model::ContextRegistry>) {
        let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
        let context = environment.default_context_registry();
        let input = ResolvedAnalysisInput::new(
            environment.analyser_profile(),
            environment.unit_profile(),
            Arc::clone(&context),
            config,
        );
        let cu = CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(environment.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        );
        (cu, context)
    }

    #[test]
    fn getter_source_keeps_function_config_and_selects_nested_parser_purpose() {
        // naming.source.retained-explorer-consumers
        // docs/design/analysis/name-resolution-proofs/retained-explorer-consumers.md
        // Actual source metadata and parser-purpose control, no BIG-IP execution.
        let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
        let config = tcl_lexer::LexerConfig {
            leading_bom: tcl_lexer::LeadingBom::Skip,
            base_line: 17,
            ..tcl_lexer::LexerConfig::from_grammar(environment.grammar())
        };
        let (cu, context) = supplied(config);
        let function = cu
            .functions()
            .find(|f| f.name.starts_with("::when::"))
            .expect("event function");
        let selected =
            unnormalised_getter_source_config(&cu.ir_module, function, context.commands(), false)
                .unwrap();
        assert_eq!(selected, function.source_lexer_config());
        assert_ne!(
            selected,
            tcl_lexer::LexerConfig::for_profile(context.commands().profile())
        );
        let local = tcl_lexer::SourceSegmentationPurpose::NestedScript.config(selected);
        assert_eq!(local, selected.nested());
        assert_eq!(local.leading_bom, tcl_lexer::LeadingBom::Content);
        assert_eq!(selected.leading_bom, tcl_lexer::LeadingBom::Skip);
        let warnings = find_unnormalised_getter_warnings(
            &cu,
            context.commands(),
            Some(environment.unit_profile().surface_query()),
        );
        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].code, DiagCode::Irule3102);
    }

    #[test]
    fn getter_source_keeps_original_children_and_ignores_report_values() {
        let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
        let query = Some(environment.unit_profile().surface_query());
        for body in [
            r#"set value "prefix[HTTP::uri]""#,
            "set value $a([HTTP::uri])",
            "set value 0; incr value [HTTP::uri]",
            "[HTTP::uri]",
        ] {
            let (unit, context) = supplied_source(&format!("when HTTP_REQUEST {{{body}}}"), config);
            let warnings = find_unnormalised_getter_warnings(&unit, context.commands(), query);
            assert_eq!(warnings.len(), 1, "{body}");
            assert!(warnings[0].message.contains("HTTP::uri -normalized"));
        }
        for source in [
            "when HTTP_REQUEST {set value {prefix[HTTP::uri]}; :HTTP::uri}",
            "proc HTTP::uri {} {return value}; when HTTP_REQUEST {HTTP::uri}",
        ] {
            let (unit, context) = supplied_source(source, config);
            assert!(
                find_unnormalised_getter_warnings(&unit, context.commands(), query).is_empty(),
                "{source}"
            );
        }
        let (mut unit, context) = supplied_source("when HTTP_REQUEST {HTTP::uri}", config);
        assert_eq!(
            find_unnormalised_getter_warnings(&unit, context.commands(), query).len(),
            1
        );
        let mut changed = false;
        for function in unit.procedures.values_mut() {
            for block in function.cfg.blocks.values_mut() {
                for statement in &mut block.statements {
                    if let Statement::Call {
                        command,
                        canonical_command,
                        args,
                        ..
                    }
                    | Statement::Barrier {
                        command,
                        canonical_command,
                        args,
                        ..
                    } = statement
                    {
                        *command = "HTTP::header".to_owned();
                        *canonical_command = Some("::HTTP::header".to_owned());
                        *args = vec!["-normalized".to_owned()];
                        changed = true;
                    }
                }
            }
        }
        assert!(changed);
        let warnings = find_unnormalised_getter_warnings(&unit, context.commands(), query);
        assert_eq!(warnings.len(), 1);
        assert!(warnings[0].message.contains("HTTP::uri -normalized"));
    }

    #[test]
    fn getter_source_refuses_missing_foreign_and_changed_actual_owners() {
        let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
        let (original, context) = supplied(config);
        let query = Some(environment.unit_profile().surface_query());
        assert_eq!(
            find_unnormalised_getter_warnings(&original, context.commands(), query).len(),
            1
        );
        for change in 0..4 {
            let mut cu = original.clone();
            match change {
                0 => cu.ir_module.source_metadata_input = None,
                1 => cu.ir_module.retained_source_bindings = None,
                2 => cu.ir_module.source = tcl_lexer::SourceImage::document("foreign source"),
                _ => cu.ir_module.lexer_config.leading_bom = tcl_lexer::LeadingBom::Skip,
            }
            assert!(
                find_unnormalised_getter_warnings(&cu, context.commands(), query).is_empty(),
                "Module axis {change}"
            );
        }
        let mut cu = original.clone();
        for function in cu.procedures.values_mut() {
            function.source_metadata_input = None;
        }
        assert!(find_unnormalised_getter_warnings(&cu, context.commands(), query).is_empty());
        let foreign = tcl_registry::model::resolve_known_environment("tcl9.1")
            .unwrap()
            .default_context_registry();
        assert!(find_unnormalised_getter_warnings(&original, foreign.commands(), query).is_empty());
        assert_eq!(
            find_unnormalised_getter_warnings(&original, context.commands(), query).len(),
            1
        );
    }
}

#[cfg(test)]
mod original_collection_source_tests {

    use super::*;
    use crate::compilation_unit::UnitBuildOptions;
    use std::sync::Arc;

    #[test]
    fn conditional_leaf_census_does_not_treat_selected_lambda_roles_as_empty_scripts() {
        let environment = tcl_registry::model::resolve_known_environment("tcl8.6").unwrap();
        let context = environment.default_context_registry();
        let dialect = tcl_registry::InvocationDialect::of_profile(environment.analyser_profile());
        let arguments = [
            tcl_registry::InvocationWord::Literal("value"),
            tcl_registry::InvocationWord::Literal("literal"),
        ];
        let selected = context.commands().resolve_structured_invocation(
            tcl_registry::InvocationWords::structured(
                tcl_registry::InvocationWord::Literal("set"),
                &arguments,
            )
            .with_dialect(dialect),
            Some(context.context().authoring_query()),
        );
        assert_eq!(
            leaf_handler_source_completion(&selected.resolved().unwrap()),
            Some(tcl_registry::registry::InvocationCompletion::FallsThrough)
        );
        for argument in [
            tcl_registry::InvocationWord::Literal("{} {drop}"),
            tcl_registry::InvocationWord::Dynamic,
        ] {
            let arguments = [argument];
            let selected = context.commands().resolve_structured_invocation(
                tcl_registry::InvocationWords::structured(
                    tcl_registry::InvocationWord::Literal("apply"),
                    &arguments,
                )
                .with_dialect(dialect),
                Some(context.context().authoring_query()),
            );
            let schema = selected
                .resolved()
                .expect("actual available Tcl8.6 Apply schema");
            assert!(schema.authored_source_lambda_call().is_some());
            assert_eq!(schema.authored_source_script_arguments(), Some(vec![0]));
            assert!(leaf_handler_source_completion(&schema).is_none());
        }
    }

    #[test]
    fn conditional_source_flow_retains_phase_and_normal_continuation_obligations() {
        let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
        let source = "when HTTP_REQUEST {set value \"prefix[drop]\"; HTTP::respond 200; HTTP::header insert X yes}";
        let (unit, context) = supplied(source, config);
        let surface = Some(environment.unit_profile().surface_query());
        let advice = find_conditional_irule_source_flow_advice(&unit, context.commands(), surface)
            .expect("complete leaf source retains possible actions without executing them");
        for code in [DiagCode::Irule5002, DiagCode::Irule1201] {
            assert!(
                advice
                    .findings()
                    .iter()
                    .any(|finding| finding.code() == code)
            );
        }
        assert!(advice.findings().iter().all(|finding| {
            finding
                .message()
                .contains("runtime reachability and completion unproved")
                && finding.warning.replacement.is_none()
                && finding.warning.fixes.is_empty()
        }));
        for obligation in [
            CollectionSourceObligationKind::UnknownTarget,
            CollectionSourceObligationKind::NormalContinuation,
        ] {
            assert!(advice.obligations().any(|current| *current == obligation));
        }
        assert!(
            advice
                .obligations()
                .any(|current| matches!(current, CollectionSourceObligationKind::EffectOnly(_)))
        );
        assert!(find_unguarded_drop_warnings(&unit, surface).is_empty());
        assert!(find_http_flow_warnings(&unit, surface).is_empty());
        for source in [
            "when HTTP_REQUEST {set value [return]suffix[drop]}",
            "when HTTP_REQUEST {if {$condition} {drop}}",
            "when HTTP_REQUEST {for {} {$condition} {drop} {HTTP::respond 200}}",
            "when HTTP_REQUEST {eval {drop}}",
            "when HTTP_REQUEST {HTTP::respond 200; return; HTTP::header insert X yes}",
        ] {
            let (unit, context) = supplied(source, config);
            assert!(
                find_conditional_irule_source_flow_advice(&unit, context.commands(), surface)
                    .is_none(),
                "{source}"
            );
        }
        let mut missing = unit.clone();
        for function in missing.procedures.values_mut() {
            for block in function.cfg.blocks.values_mut() {
                for statement in &mut block.statements {
                    if let Some(tokens) = statement.tokens_mut()
                        && tokens.synthetic.is_none()
                    {
                        tokens.source_binding = None;
                    }
                }
            }
        }
        assert!(
            find_conditional_irule_source_flow_advice(&missing, context.commands(), surface)
                .is_none()
        );
    }

    #[test]
    fn conditional_collection_pairings_require_a_closed_source_census() {
        let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
        let source = "when HTTP_REQUEST {HTTP::release}";
        let (unit, context) = supplied(source, config);
        {
            let advice = find_conditional_collection_source_pairing_advice(
                &unit,
                context.commands(),
                Some(environment.unit_profile().surface_query()),
            )
            .expect("the complete original leaf handler has conditional source advice");
            assert!(
                advice
                    .pairings()
                    .iter()
                    .any(|pairing| pairing.code() == DiagCode::Irule1008)
            );
            assert!(
                advice
                    .pairings()
                    .iter()
                    .all(|pairing| pairing.message().contains("Conditional source pairing"))
            );
            assert!(
                advice
                    .obligations()
                    .any(|obligation| *obligation == CollectionSourceObligationKind::UnknownTarget)
            );
            assert!(
                advice.obligations().any(
                    |obligation| *obligation == CollectionSourceObligationKind::EventActivation
                )
            );
            assert!(advice.obligations().any(|obligation| *obligation
                == CollectionSourceObligationKind::SourceSchemaApplicability));
        }
        for source in [
            "when HTTP_REQUEST {HTTP::release}\nwhen $event {HTTP::collect}",
            "when HTTP_REQUEST priority [unknown_header] {HTTP::release}",
            "when HTTP_REQUEST priority [eval {when HTTP_RESPONSE {HTTP::collect}}] {HTTP::release}",
            "when HTTP_REQUEST {eval {HTTP::collect}; HTTP::release}",
            "when HTTP_REQUEST {expr {[HTTP::collect]}; HTTP::release}",
            "when HTTP_REQUEST {after 0 {HTTP::collect}; HTTP::release}",
            "unknown_constructor\nwhen HTTP_REQUEST {HTTP::release}",
            "when HTTP_REQUEST {HTTP::release}\nwhen HTTP_RESPONSE {",
            "when HTTP_REQUEST {HTTP::release; return; HTTP::collect}",
        ] {
            let (unit, context) = supplied(source, config);
            assert!(
                find_conditional_collection_source_pairing_advice(
                    &unit,
                    context.commands(),
                    Some(environment.unit_profile().surface_query()),
                )
                .is_none(),
                "{source}"
            );
        }
        for axis in 0..3 {
            let mut stale = unit.clone();
            match axis {
                0 => stale.ir_module.source = tcl_lexer::SourceImage::document("foreign source"),
                1 => stale.ir_module.lexer_config.leading_bom = tcl_lexer::LeadingBom::Skip,
                _ => {
                    for function in stale.procedures.values_mut() {
                        for block in function.cfg.blocks.values_mut() {
                            for statement in &mut block.statements {
                                if let Some(tokens) = statement.tokens_mut() {
                                    if tokens.synthetic.is_none() {
                                        tokens.source_binding = None;
                                    }
                                }
                            }
                        }
                    }
                }
            }
            assert!(
                find_conditional_collection_source_pairing_advice(
                    &stale,
                    context.commands(),
                    Some(environment.unit_profile().surface_query()),
                )
                .is_none(),
                "axis {axis}"
            );
        }
    }

    #[test]
    fn unknown_collection_effects_keep_source_calls_without_certifying_absence() {
        let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
        // Unknown effects need unknown prior history, which a supplied
        // document no longer implies: a fresh rule load resolves
        // `HTTP::release` (user decision 2026-10-21T00:10Z). The explicit
        // unknown-history entry keeps this contract's premise.
        let (mut unit, context) =
            supplied_with_history("when HTTP_REQUEST {HTTP::release}", config, true);
        let query = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
        let state = collect_flow_state(&query).unwrap().0;
        assert_eq!(state.release_calls.len(), 1);
        assert!(
            state
                .obligations
                .contains(&CollectionSourceObligationKind::UnknownTarget)
        );
        assert!(
            find_collect_flow_warnings(
                &unit,
                context.commands(),
                Some(environment.unit_profile().surface_query())
            )
            .is_empty()
        );
        let mut changed = false;
        for function in unit.procedures.values_mut() {
            for block in function.cfg.blocks.values_mut() {
                for statement in &mut block.statements {
                    if matches!(
                        statement.original_argv_inventory_disposition(),
                        crate::ir::OriginalArgvInventoryDisposition::EffectOnly(
                            crate::ir::SyntheticMarker::GlobalFrameScript
                                | crate::ir::SyntheticMarker::CallerFrameOpaque
                        )
                    ) {
                        statement
                            .tokens_mut()
                            .unwrap()
                            .argv_texts
                            .push("unowned argv".to_owned());
                        changed = true;
                        break;
                    }
                }
                if changed {
                    break;
                }
            }
            if changed {
                break;
            }
        }
        assert!(changed);
        let query = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
        assert!(collect_flow_state(&query).is_none());
    }

    fn supplied(
        source: &str,
        config: tcl_lexer::LexerConfig,
    ) -> (CompilationUnit, Arc<tcl_registry::model::ContextRegistry>) {
        supplied_with_history(source, config, false)
    }

    /// `unknown_history` supplies the explicit unknown-history entry; without
    /// it a supplied document is a fresh rule load (user decision
    /// 2026-10-21T00:10Z).
    fn supplied_with_history(
        source: &str,
        config: tcl_lexer::LexerConfig,
        unknown_history: bool,
    ) -> (CompilationUnit, Arc<tcl_registry::model::ContextRegistry>) {
        let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
        let context = environment.default_context_registry();
        let input = crate::analyser::ResolvedAnalysisInput::new(
            environment.analyser_profile(),
            environment.unit_profile(),
            Arc::clone(&context),
            config,
        );
        let unit = CompilationUnit::build_with_analysis_input(
            source,
            UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(environment.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            unknown_history
                .then(|| {
                    crate::command_binding::SourceAnalysisEntry::unknown_history_for_test(&input)
                })
                .as_ref(),
            &input,
        );
        (unit, context)
    }

    #[test]
    fn collection_query_uses_actual_function_config_and_separates_standalone() {
        // naming.source.original-collection-body-owner
        // docs/design/analysis/name-resolution-proofs/original-collection-body-owner.md
        // Actual source/config + extracted syntax purpose, not BIG-IP execution.
        let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
        let config = tcl_lexer::LexerConfig {
            leading_bom: tcl_lexer::LeadingBom::Skip,
            base_line: 17,
            ..tcl_lexer::LexerConfig::from_grammar(environment.grammar())
        };
        let (unit, context) = supplied("when HTTP_REQUEST {HTTP::collect; HTTP::release}", config);
        let source = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
        assert!(!source.standalone);
        assert!(source.metadata().is_some());
        let function = unit
            .procedures
            .values()
            .find(|fu| fu.name.starts_with("::when::"))
            .unwrap();
        let actual = source.function_config(function).unwrap();
        assert_eq!(actual, function.source_lexer_config());
        let local = tcl_lexer::SourceSegmentationPurpose::NestedScript.config(actual);
        assert_eq!(local.leading_bom, tcl_lexer::LeadingBom::Content);
        let (head, _) =
            parse_command_substitution_with_config("[\u{feff}HTTP::payload]", local).unwrap();
        assert_eq!(head, "\u{feff}HTTP::payload");
        let (head, _) = parse_command_substitution_with_config("[HTTP::payload]", local).unwrap();
        assert_eq!(head, "HTTP::payload");
        let standalone = CompilationUnit::build_for_with_config(
            "when HTTP_REQUEST {HTTP::collect; HTTP::release}",
            context.commands(),
            false,
            config,
        );
        let separate = IrulesSourceQuery::capture(&standalone, context.commands()).unwrap();
        assert!(separate.standalone);
        assert!(separate.metadata().is_none());
        assert!(collect_flow_state(&separate).is_some());
    }

    #[test]
    fn collection_source_metadata_keeps_original_names_and_ignores_report_labels() {
        let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
        let (mut unit, context) = supplied(
            "when HTTP_REQUEST {HTTP::release; :HTTP::collect; set value {[HTTP::collect]}}",
            config,
        );
        let query = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
        let original = collect_flow_state(&query).unwrap().0;
        assert!(original.collect_calls.is_empty());
        assert_eq!(original.release_calls.len(), 1);
        let mut changed = false;
        for function in unit.procedures.values_mut() {
            for block in function.cfg.blocks.values_mut() {
                for statement in &mut block.statements {
                    if let Statement::Call {
                        command,
                        canonical_command,
                        tokens: Some(tokens),
                        ..
                    }
                    | Statement::Barrier {
                        command,
                        canonical_command,
                        tokens: Some(tokens),
                        ..
                    } = statement
                        && tokens
                            .argv_texts
                            .first()
                            .is_some_and(|word| word == "HTTP::release")
                    {
                        *command = "HTTP::collect".to_owned();
                        *canonical_command = Some("::HTTP::collect".to_owned());
                        changed = true;
                    }
                }
            }
        }
        assert!(changed);
        let query = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
        let actual = collect_flow_state(&query).unwrap().0;
        assert!(actual.collect_calls.is_empty());
        assert_eq!(actual.release_calls.len(), 1);
    }

    #[test]
    fn collection_source_keeps_original_template_index_and_head_substitutions() {
        let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
        for body in [
            r#"set value "prefix[HTTP::collect]""#,
            "set value $a([HTTP::collect])",
            "[HTTP::collect]",
        ] {
            let (unit, context) = supplied(
                &format!("when HTTP_REQUEST {{{body}; HTTP::release}}"),
                config,
            );
            let query = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
            let state = collect_flow_state(&query).unwrap().0;
            assert_eq!(state.collect_calls.len(), 1, "{body}");
            assert_eq!(state.release_calls.len(), 1, "{body}");
        }
        let (unit, context) = supplied(
            "when HTTP_REQUEST {set value {prefix[HTTP::collect]}; HTTP::release}",
            config,
        );
        let query = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
        let state = collect_flow_state(&query).unwrap().0;
        assert!(state.collect_calls.is_empty());
        assert_eq!(state.release_calls.len(), 1);
    }

    #[test]
    fn collection_source_retains_admitted_nonregistry_commands() {
        let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
        let (unit, context) = supplied(
            "proc ordinary {} {return value}; when HTTP_REQUEST {ordinary; HTTP::release}",
            config,
        );
        let query = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
        let state = collect_flow_state(&query).unwrap().0;
        assert!(state.collect_calls.is_empty());
        assert_eq!(state.release_calls.len(), 1);
    }

    #[test]
    fn assigned_collection_substitution_uses_its_original_child_once() {
        let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
        let (mut unit, context) = supplied(
            "when HTTP_REQUEST {set value [HTTP::collect]; HTTP::release}",
            config,
        );
        let query = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
        let state = collect_flow_state(&query).unwrap().0;
        assert_eq!(state.collect_calls.len(), 1);
        assert_eq!(state.release_calls.len(), 1);
        let mut removed = false;
        for function in unit.procedures.values_mut() {
            for block in function.cfg.blocks.values_mut() {
                for statement in &mut block.statements {
                    if statement.tokens().is_some_and(|tokens| {
                        tokens.argv_texts.first().is_some_and(|head| head == "set")
                    }) {
                        match statement {
                            Statement::AssignValue { tokens, .. }
                            | Statement::Call { tokens, .. }
                            | Statement::Barrier { tokens, .. } => {
                                *tokens = None;
                                removed = true;
                            }
                            _ => panic!("unexpected original set carrier"),
                        }
                    }
                }
            }
        }
        assert!(removed);
        let query = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
        assert!(collect_flow_state(&query).is_none());
    }

    #[test]
    fn collection_withdrawal_is_unavailable_instead_of_an_empty_pairing_inventory() {
        // naming.source.original-collection-body-owner
        // docs/design/analysis/name-resolution-proofs/original-collection-body-owner.md
        let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
        let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
        let (unit, context) = supplied(
            "when HTTP_REQUEST {clientside {HTTP::collect}; HTTP::release}",
            config,
        );
        let source = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
        assert!(collect_flow_state(&source).is_some());
        for axis in 0..6 {
            let mut changed = unit.clone();
            match axis {
                0 => changed.source.push_str("foreign"),
                1 => changed.ir_module.source_metadata_input = None,
                2 => changed.ir_module.retained_source_bindings = None,
                3 => changed.ir_module.lexer_config.leading_bom = tcl_lexer::LeadingBom::Skip,
                4 => changed.source_class_facts = None,
                _ => {
                    for function in changed.procedures.values_mut() {
                        function.source_metadata_input = None;
                    }
                }
            }
            let query = IrulesSourceQuery::capture(&changed, context.commands());
            assert!(
                query
                    .as_ref()
                    .is_none_or(|query| collect_flow_state(query).is_none()),
                "axis {axis}"
            );
            assert!(
                find_collect_flow_warnings(
                    &changed,
                    context.commands(),
                    Some(environment.unit_profile().surface_query())
                )
                .is_empty(),
                "axis {axis}"
            );
        }
        let mut wrong = unit.clone();
        for function in wrong.procedures.values_mut() {
            for block in function.cfg.blocks.values_mut() {
                for statement in &mut block.statements {
                    if let Some(tokens) = statement.tokens_mut()
                        && tokens
                            .argv_texts
                            .first()
                            .is_some_and(|head| head == "clientside")
                    {
                        tokens.argv_texts[1].push_str("foreign");
                    }
                }
            }
        }
        let source = IrulesSourceQuery::capture(&wrong, context.commands()).unwrap();
        assert!(collect_flow_state(&source).is_none());
    }
}

#[cfg(test)]
mod original_event_owner_tests {
    use crate::interprocedural::test_body_identity::fixture_id_if_present;

    use super::*;
    use crate::compilation_unit::UnitBuildOptions;
    use std::sync::Arc;

    fn supplied(
        text: &str,
        custom_definer: bool,
    ) -> (CompilationUnit, Arc<tcl_registry::model::ContextRegistry>) {
        let environment = tcl_registry::model::resolve_known_environment("f5-irules").unwrap();
        let context = if custom_definer {
            let mut store = CommandRegistry::build_default()
                .project_for_profile(environment.analyser_profile());
            let selected = store.get("when").unwrap().clone();
            store.insert(tcl_registry::CommandSpec {
                name: "on",
                ..selected
            });
            Arc::new(
                environment
                    .default_context_registry()
                    .with_command_store(Arc::new(store)),
            )
        } else {
            environment.default_context_registry()
        };
        let config = tcl_lexer::LexerConfig {
            leading_bom: tcl_lexer::LeadingBom::Skip,
            base_line: 23,
            ..tcl_lexer::LexerConfig::from_grammar(environment.grammar())
        };
        let input = crate::analyser::ResolvedAnalysisInput::new(
            environment.analyser_profile(),
            environment.unit_profile(),
            Arc::clone(&context),
            config,
        );
        let unit = CompilationUnit::build_with_analysis_input(
            text,
            UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config,
                dialect: Some(environment.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            &input,
        );
        (unit, context)
    }

    fn events(source: &IrulesSourceQuery<'_>) -> Option<Vec<String>> {
        let mut names = Vec::new();
        for function in source.unit.functions() {
            if let OriginalEventPresence::Event(event) = source.event(function)? {
                names.push(event.body.event().to_owned());
            }
        }
        names.sort();
        Some(names)
    }

    fn event_warnings(
        unit: &CompilationUnit,
        registry: &CommandRegistry,
    ) -> Vec<IrulesCheckWarning> {
        let surface = Some(SurfaceQuery::any_release(Family::F5Irules));
        let mut warnings = Vec::new();
        if let Some(advice) =
            find_conditional_collection_source_pairing_advice(unit, registry, surface)
        {
            if let Some(flow) = advice.conditional_source_flow() {
                warnings.extend(flow.findings.iter().map(|finding| finding.warning.clone()));
            }
            warnings.extend(
                advice
                    .pairings
                    .iter()
                    .map(|pairing| pairing.warning.clone()),
            );
        }
        warnings.extend(find_hoistable_set_warnings(unit, surface));
        warnings.extend(find_generic_static_name_warnings(unit, surface, None));
        warnings
    }

    #[test]
    fn source_flow_requires_actual_selected_commands_and_original_children() {
        let surface = Some(SurfaceQuery::any_release(Family::F5Irules));
        for text in [
            "proc drop args {}; when HTTP_REQUEST {drop}",
            "proc HTTP::respond args {}; when HTTP_REQUEST {HTTP::respond 200; HTTP::header insert X yes}",
            "when HTTP_REQUEST {set value {prefix[drop]}; set response {[HTTP::respond 200]}; HTTP::header insert X yes}",
        ] {
            let (unit, _) = supplied(text, false);
            assert!(
                find_unguarded_drop_warnings(&unit, surface).is_empty(),
                "{text}"
            );
            assert!(find_http_flow_warnings(&unit, surface).is_empty(), "{text}");
        }
        for text in [
            "when HTTP_REQUEST {::drop}",
            r#"when HTTP_REQUEST {set value "prefix[drop]"}"#,
            "when HTTP_REQUEST {drop; event disable $target}",
            "when HTTP_REQUEST {drop; event disable all extra}",
        ] {
            let (unit, _) = supplied(text, false);
            assert!(
                find_unguarded_drop_warnings(&unit, surface)
                    .iter()
                    .any(|warning| warning.code == DiagCode::Irule5002),
                "{text}"
            );
        }
        for text in [
            r#"when HTTP_REQUEST {set value "prefix[HTTP::respond 200]"; HTTP::header insert X yes}"#,
            "when HTTP_REQUEST {set value $a([HTTP::respond 200]); HTTP::header insert X yes}",
        ] {
            let (unit, _) = supplied(text, false);
            assert!(
                find_http_flow_warnings(&unit, surface)
                    .iter()
                    .any(|warning| warning.code == DiagCode::Irule1201),
                "{text}"
            );
        }
    }

    #[test]
    fn source_flow_evaluates_original_header_and_return_arguments_before_dispatch() {
        let surface = Some(SurfaceQuery::any_release(Family::F5Irules));
        for tail in [
            "return [HTTP::header value X]",
            "if [HTTP::header exists X] {}",
            "while [HTTP::header exists X] {break}",
            "foreach name [HTTP::header names] {}",
            "switch [HTTP::header value X] {yes {}}",
        ] {
            let text = format!("when HTTP_REQUEST {{HTTP::respond 200; {tail}}}");
            let (unit, _) = supplied(&text, false);
            let warnings = find_http_flow_warnings(&unit, surface);
            assert_eq!(
                warnings
                    .iter()
                    .filter(|warning| warning.code == DiagCode::Irule1201)
                    .count(),
                1,
                "{text}: {warnings:?}"
            );
        }

        let (unit, _) = supplied(
            "when HTTP_REQUEST {catch {return [HTTP::respond 200]}; HTTP::header value X}",
            false,
        );
        assert!(
            find_http_flow_warnings(&unit, surface)
                .iter()
                .any(|warning| warning.code == DiagCode::Irule1201)
        );

        for tail in [
            "return {[HTTP::header value X]}",
            "if 0 {HTTP::header value X}",
            "return; HTTP::header value X",
        ] {
            let text = format!("when HTTP_REQUEST {{HTTP::respond 200; {tail}}}");
            let (unit, _) = supplied(&text, false);
            assert!(find_http_flow_warnings(&unit, surface).is_empty(), "{text}");
        }
    }

    #[test]
    fn ordinary_procedure_with_an_event_shaped_name_has_no_event_semantics() {
        // naming.variable.registry-event-frame-identity
        // docs/design/analysis/name-resolution-proofs/variable-registry-event-frame-identity.md
        // Source classification only; this does not execute an iRule or Tcl proc.
        let text = "namespace eval ::when {}; proc ::when::HTTP_REQUEST {} {\n\
            HTTP::respond 200; HTTP::header insert X-Test yes; drop;\n\
            set static::debug 1; set svc foo; HTTP::payload\n}";
        let (unit, context) = supplied(text, false);
        assert!(fixture_id_if_present(&unit.procedures, "::when::HTTP_REQUEST").is_some());
        let source = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
        assert_eq!(events(&source), Some(Vec::new()));
        assert_eq!(event_mutation_counts(&source), Some(HashMap::new()));
        assert!(collect_flow_state(&source).unwrap().1.is_empty());
        assert!(event_warnings(&unit, context.commands()).is_empty());
        assert!(event_anchor_span(&unit, context.commands(), "HTTP_REQUEST").is_none());
    }

    #[test]
    fn selected_custom_event_definer_and_report_names_use_the_original_descriptor() {
        // naming.variable.registry-event-frame-identity
        // docs/design/analysis/name-resolution-proofs/variable-registry-event-frame-identity.md
        // Event-table policy and source descriptor only, without entered event/TMM facts.
        let text = "on ASM_RESPONSE_VIOLATION {HTTP::respond 200; HTTP::header insert X-Test yes}\n\
            on HTTP_REQUEST {}\nwhen HTTP_REQUEST {}";
        let (unit, context) = supplied(text, true);
        let source = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
        let expected = vec![
            "ASM_RESPONSE_VIOLATION".to_owned(),
            "HTTP_REQUEST".to_owned(),
            "HTTP_REQUEST".to_owned(),
        ];
        assert_eq!(events(&source), Some(expected.clone()));
        let baseline = event_warnings(&unit, context.commands());
        assert!(
            baseline
                .iter()
                .any(|warning| warning.code == DiagCode::Irule1201)
        );
        let anchor = event_anchor_span(&unit, context.commands(), "HTTP_REQUEST").unwrap();
        assert_eq!(
            &text[anchor.start() as usize..anchor.end() as usize],
            "HTTP_REQUEST"
        );
        let mut labelled = unit.clone();
        for (index, function) in labelled.procedures.values_mut().enumerate() {
            function.name = format!("::ordinary::report{index}");
        }
        let source = IrulesSourceQuery::capture(&labelled, context.commands()).unwrap();
        assert_eq!(events(&source), Some(expected));
        assert_eq!(event_warnings(&labelled, context.commands()), baseline);
        assert_eq!(
            event_anchor_span(&labelled, context.commands(), "HTTP_REQUEST"),
            Some(anchor)
        );
        for text in [
            "rename on moved; moved HTTP_REQUEST {}",
            "interp alias {} fire {} on; fire HTTP_REQUEST {}",
        ] {
            let (selected, context) = supplied(text, true);
            let source = IrulesSourceQuery::capture(&selected, context.commands()).unwrap();
            assert_eq!(events(&source), Some(vec!["HTTP_REQUEST".to_owned()]));
        }
        let function = source.unit.procedures.values().next().unwrap().clone();
        assert!(
            source.event(&function).is_none(),
            "a copied FU is not the actual CU-owned function"
        );
    }

    #[test]
    fn event_projection_withdraws_changed_or_missing_complete_owners() {
        // naming.variable.registry-event-frame-identity
        // docs/design/analysis/name-resolution-proofs/variable-registry-event-frame-identity.md
        let (unit, context) = supplied(
            "when HTTP_REQUEST {drop; HTTP::payload; set static::debug 1}",
            false,
        );
        let original = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
        assert_eq!(events(&original), Some(vec!["HTTP_REQUEST".to_owned()]));
        assert!(!event_warnings(&unit, context.commands()).is_empty());
        for axis in 0..10 {
            let mut changed = unit.clone();
            match axis {
                0 => changed.source.push_str("foreign"),
                1 => changed.ir_module.source_metadata_input = None,
                2 => changed.ir_module.retained_source_bindings = None,
                3 => changed.ir_module.lexer_config.leading_bom = tcl_lexer::LeadingBom::Content,
                4 => {
                    changed.ir_module.lexer_config.strict_quoting =
                        !changed.ir_module.lexer_config.strict_quoting
                }
                5 => changed
                    .procedures
                    .values_mut()
                    .for_each(|function| function.source_metadata_input = None),
                6 => changed.procedures.values_mut().for_each(|function| {
                    function.source_config.strict_quoting = !function.source_config.strict_quoting
                }),
                7 => changed
                    .procedures
                    .values_mut()
                    .for_each(|function| function.irules_event_body = None),
                8 => changed.ir_module.irules_event_bodies.clear(),
                _ => changed
                    .ir_module
                    .procedures
                    .values_mut()
                    .for_each(|procedure| procedure.body.statements.clear()),
            }
            let source = IrulesSourceQuery::capture(&changed, context.commands());
            assert!(
                source
                    .as_ref()
                    .is_none_or(|source| events(source).is_none()),
                "axis {axis}"
            );
            assert!(
                source
                    .as_ref()
                    .is_none_or(|source| collect_flow_state(source).is_none()),
                "pairing axis {axis}"
            );
            assert!(
                source
                    .as_ref()
                    .is_none_or(|source| event_mutation_counts(source).is_none()),
                "mutations axis {axis}"
            );
            assert!(
                event_warnings(&changed, context.commands()).is_empty(),
                "warnings axis {axis}"
            );
            assert!(
                event_anchor_span(&changed, context.commands(), "HTTP_REQUEST").is_none(),
                "anchor axis {axis}"
            );
        }
        let foreign = tcl_registry::model::resolve_known_environment("tcl9.1")
            .unwrap()
            .default_context_registry();
        assert!(IrulesSourceQuery::capture(&unit, foreign.commands()).is_none());
        let (other, _) = supplied("when CLIENT_ACCEPTED {drop}", false);
        let mut donated = unit.clone();
        donated
            .procedures
            .values_mut()
            .next()
            .unwrap()
            .irules_event_body = other
            .procedures
            .values()
            .next()
            .unwrap()
            .irules_event_body
            .clone();
        let source = IrulesSourceQuery::capture(&donated, context.commands()).unwrap();
        assert!(events(&source).is_none());
        assert!(event_warnings(&donated, context.commands()).is_empty());
    }

    #[test]
    fn explicitly_standalone_source_retains_events_without_report_name_assistance() {
        // naming.variable.registry-event-frame-identity
        // docs/design/analysis/name-resolution-proofs/variable-registry-event-frame-identity.md
        let (original, context) = supplied("when HTTP_REQUEST {drop}", false);
        let mut unit = CompilationUnit::build_for_with_config(
            &original.source,
            context.commands(),
            false,
            original.ir_module.lexer_config,
        );
        for function in unit.procedures.values_mut() {
            function.name = "::plain::report".to_owned();
        }
        let source = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
        assert!(source.standalone);
        assert_eq!(events(&source), Some(vec!["HTTP_REQUEST".to_owned()]));
        assert!(
            find_unguarded_drop_warnings(&unit, Some(SurfaceQuery::any_release(Family::F5Irules)))
                .iter()
                .any(|warning| warning.code == DiagCode::Irule5002)
        );
        unit.procedures
            .values_mut()
            .next()
            .unwrap()
            .irules_event_body = None;
        let source = IrulesSourceQuery::capture(&unit, context.commands()).unwrap();
        assert!(events(&source).is_none());
        assert!(
            find_unguarded_drop_warnings(&unit, Some(SurfaceQuery::any_release(Family::F5Irules)))
                .is_empty()
        );
    }
}
