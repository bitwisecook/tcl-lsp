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

//! Proc argument trait inference.
//!
//! Walks a proc body to determine how each parameter is used:
//!
//! - `Eval` — run as a script: a `Body` word of a call the registry says
//!   evaluates code or defers its script (``eval``, ``uplevel``, ``after``),
//!   or any word of one that substitutes its operand (``subst``)
//! - `Body` — used as a loop / control body
//! - `VarWrite` — names a variable the proc writes through an alias
//!   (``upvar`` and then any registry `VarWrite` of the alias)
//! - `VarRead` — names a variable the proc reads via ``upvar``
//! - `Expr` — evaluated as an expression
//! - `LoopList` — the list a list loop iterates (``foreach`` / ``lmap``)
//! - `DynamicNameLocal` — the param's *value* names a
//!   **callee-local** variable (``set $p 1`` / ``scan … $p`` /
//!   ``lassign … $p`` / ``regsub … $p``, or a registry
//!   ``VarWrite`` / ``VarRead`` role on a bare ``$param``).  Emitted
//!   with `VarRead`; refines it so caller-side dead-store / unused
//!   suppression does **not** treat a literal-name call arg as
//!   consumed (the callee uses the string to name its *own* local,
//!   not a caller-frame alias).
//!
//! Two passes are exposed:
//!
//! * [`infer_param_traits`] — shallow command scan.  Fast enough
//!   for synchronous use during analysis; detects direct patterns
//!   like ``eval $param``, ``upvar 1 $param local``, ``foreach x
//!   $list body``.  It **does** descend into ``[...]`` command
//!   substitutions (`return [set $v]` is a first-order use), and
//!   recognises variable names built from parameters — a bare
//!   ``$p``, a compound array element (``set ${v}($k)`` marks both
//!   ``v`` and ``k``), and ``namespace upvar ns $token arr``.  It
//!   does **not** recurse into braced *body* arguments (that is the
//!   deep pass's job).
//! * [`infer_param_traits_deep`] — recursive descent into
//!   braced body args, catching traits hidden one or more
//!   levels deep (`foreach item $items { uplevel 1 $body }`
//!   surfaces the `$body` Eval trait via the recursion).  More
//!   expensive than the shallow pass; intended for asynchronous
//!   analysis (call-graph / symbol-graph / dataflow-graph /
//!   semantic-graph builders).  Bounded by [`MAX_DEPTH`]
//!   (8 levels) to prevent runaway recursion on pathological
//!   input.
//!
//! [`merge_traits`] unions the two passes' results when
//! callers want both.

use std::collections::{HashMap, HashSet};

use tcl_registry::model::DocumentCommandSurface;
use tcl_registry::{CommandRegistry, arg_role::ArgRole};

use super::types::ProcArgTrait;
use crate::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::{LexerConfig, TokenType};

/// Top-level shallow trait inference.  Returns a map from
/// parameter name to a set of inferred traits.  Empty entries
/// (parameters with no detected trait) are dropped from the
/// returned map.
///
/// `env` carries the document's one command surface (the dialect-aware
/// registry generation plus whatever the document declares for itself with
/// `# tcl-lsp: stub`), the dialect lexer configuration, and the document's
/// proven command-identity facts — see [`TraitScanEnv`].
#[must_use]
pub fn infer_param_traits(
    params: &[&str],
    body_source: &str,
    env: TraitScanEnv<'_>,
) -> HashMap<String, HashSet<ProcArgTrait>> {
    if params.is_empty()
        || body_source.trim().is_empty()
        || !env.matches_source(body_source)
        || !env.matches_parameters(params)
    {
        return HashMap::new();
    }
    let param_set: HashSet<&str> = params.iter().copied().collect();
    let mut traits: HashMap<&str, HashSet<ProcArgTrait>> =
        params.iter().map(|p| (*p, HashSet::new())).collect();
    let mut aliases = Aliases::default();

    let ctx = ScanCtx {
        param_set: &param_set,
        parameters: params,
        surface: env.surface,
        config: env.config,
        identities: env.identities,
        executed_source: env.executed_source,
        original_logical_body: env.original_logical_body,
        document: None,
        original_logical_lambda: None,
        selected_source_child_body: None,
        source_base: env
            .executed_source
            .map(crate::command_binding::ExecutedScriptSource::base)
            .or_else(|| {
                env.original_logical_body
                    .map(|body| body.scope().scope_body().content_span().start())
            }),
    };
    scan_commands(body_source, &ctx, &mut traits, &mut aliases);

    finalise_traits(traits)
}

/// Maximum recursion depth for [`infer_param_traits_deep`]
/// (8 levels).  Pathological input (deeply-nested braced bodies)
/// stops descending past this bound rather than blowing the
/// stack.
pub const MAX_DEPTH: u8 = 8;

/// Recursive deep trait inference.  Same return shape as
/// [`infer_param_traits`] but additionally descends into braced
/// body arguments to surface traits hidden one or more levels
/// in.  More expensive than the shallow pass — intended for
/// asynchronous use behind the call-graph / symbol-graph /
/// dataflow-graph / semantic-graph builders.
///
/// Recursion is bounded by [`MAX_DEPTH`] and only enters braced
/// body args — `$var` or `[cmd]` references at the head of a
/// body arg are treated as opaque (their `Eval` trait is already
/// captured at the top level by the same call-site's role
/// scan).
#[must_use]
pub fn infer_param_traits_deep(
    params: &[&str],
    body_source: &str,
    env: TraitScanEnv<'_>,
) -> HashMap<String, HashSet<ProcArgTrait>> {
    infer_param_traits_deep_at_depth(params, body_source, env, 0, None, None)
}

/// Deep trait inference for a body retained inside the current complete
/// analysed document. Lambda source forms are selected from the retained
/// Registry invocation and exact original words; a stale or foreign analysis
/// simply contributes no lambda recursion.
pub(crate) fn infer_param_traits_deep_in_document(
    params: &[&str],
    body_source: &str,
    env: TraitScanEnv<'_>,
    document: &str,
    analysis: &crate::analyser::AnalysisResult,
    body_offset: Option<u32>,
) -> HashMap<String, HashSet<ProcArgTrait>> {
    infer_param_traits_deep_at_depth(
        params,
        body_source,
        env,
        0,
        Some((document, analysis, body_offset)),
        None,
    )
}

/// [`infer_param_traits_deep`] with an explicit starting depth.
///
/// The public entry points always start at `0`; `scan_deep`'s own `apply`
/// (`ArgRole::LambdaLiteral`) handling calls this with `depth + 1` instead
/// of re-entering at `0`. A lambda body is inferred in its own frame (see
/// the doc comment at that call site for why), but it is still lexically
/// nested inside the enclosing scan — starting it back at depth `0` would
/// let alternating `if {…} { apply {x {…}} … }` nesting reset the logical
/// counter on every `apply`, defeating [`MAX_DEPTH`] while the *native*
/// call stack keeps growing one `scan_deep` ↔
/// `infer_param_traits_deep_at_depth` frame group per level regardless —
/// the same "guard exists but doesn't cover every recursive edge" bug class.
fn infer_param_traits_deep_at_depth(
    params: &[&str],
    body_source: &str,
    env: TraitScanEnv<'_>,
    depth: u8,
    document: Option<(&str, &crate::analyser::AnalysisResult, Option<u32>)>,
    original_logical_lambda: Option<&crate::registry_invocation::LogicalLambdaSourceOwner>,
) -> HashMap<String, HashSet<ProcArgTrait>> {
    if params.is_empty()
        || body_source.trim().is_empty()
        || depth > MAX_DEPTH
        || !env.matches_source(body_source)
        || !env.matches_parameters(params)
        || original_logical_lambda.is_some_and(|owner| {
            let Some(input) = env
                .identities
                .source_bindings_ref()
                .original_logical_source_name_advice_input()
            else {
                return true;
            };
            let Some(image) = env.identities.original_source_image() else {
                return true;
            };
            let Some((_, _, content, _)) = owner.lambda.frame_source(input) else {
                return true;
            };
            !owner.lambda.matches_source(image, input)
                || !owner.scope.matches_input(input)
                || !owner.lambda.body().matches_original_content(
                    body_source,
                    content.start(),
                    env.config,
                )
                || !owner.lambda.formal_parameters().is_some_and(|formals| {
                    formals
                        .parameters()
                        .iter()
                        .map(|formal| formal.name.as_str())
                        .eq(params.iter().copied())
                })
        })
    {
        return HashMap::new();
    }
    let param_set: HashSet<&str> = params.iter().copied().collect();
    let mut traits: HashMap<&str, HashSet<ProcArgTrait>> =
        params.iter().map(|p| (*p, HashSet::new())).collect();
    let mut aliases = Aliases::default();

    let ctx = ScanCtx {
        param_set: &param_set,
        parameters: params,
        surface: env.surface,
        config: env.config,
        identities: env.identities,
        executed_source: env.executed_source,
        original_logical_body: env.original_logical_body,
        document,
        original_logical_lambda,
        selected_source_child_body: None,
        source_base: env
            .executed_source
            .map(crate::command_binding::ExecutedScriptSource::base)
            .or_else(|| {
                env.original_logical_body
                    .map(|body| body.scope().scope_body().content_span().start())
            })
            .or_else(|| document.and_then(|(_, _, base)| base))
            .or_else(|| {
                original_logical_lambda.and_then(|owner| {
                    owner
                        .lambda
                        .frame_source(
                            env.identities
                                .source_bindings_ref()
                                .original_logical_source_name_advice_input()?,
                        )
                        .map(|(_, _, content, _)| content.start())
                })
            }),
    };
    scan_deep(body_source, &ctx, &mut traits, &mut aliases, depth);

    finalise_traits(traits)
}

/// Union shallow + deep trait results per parameter.  Useful
/// when callers want to run the shallow pass synchronously for
/// an initial result and then upgrade with the deep pass once it
/// completes.
///
/// Generic over the hasher so it composes with whatever
/// [`infer_param_traits`] / [`infer_param_traits_deep`] return.
#[must_use]
pub fn merge_traits<S, H>(
    shallow: HashMap<String, HashSet<ProcArgTrait, H>, S>,
    deep: HashMap<String, HashSet<ProcArgTrait, H>, S>,
) -> HashMap<String, HashSet<ProcArgTrait, H>, S>
where
    S: std::hash::BuildHasher,
    H: std::hash::BuildHasher + Default,
{
    let mut merged = shallow;
    for (param, deep_traits) in deep {
        merged.entry(param).or_default().extend(deep_traits);
    }
    merged
}

/// Drop parameters with no detected trait and convert the
/// borrowed keys back to owned `String`s.  Shared between
/// `infer_param_traits` and `infer_param_traits_deep` so both
/// pass shapes return the same kind of map.
fn finalise_traits(
    traits: HashMap<&str, HashSet<ProcArgTrait>>,
) -> HashMap<String, HashSet<ProcArgTrait>> {
    traits
        .into_iter()
        .filter(|(_, set)| !set.is_empty())
        .map(|(k, v)| (k.to_string(), v))
        .collect()
}

/// Read-only inference state threaded unchanged through the
/// shallow and deep scan family (`scan_commands` / `scan_command`
/// / `scan_deep`).  Bundling these into one borrowing context
/// keeps each scan helper at or under the 7-argument limit.
///
/// `'p` is the params' lifetime (the param-name slices borrowed
/// from the caller's `params` argument); `'r` borrows the
/// command surface / param-set for the duration of a scan.
#[derive(Clone)]
struct ScanCtx<'p, 'r> {
    param_set: &'r HashSet<&'p str>,
    parameters: &'r [&'p str],
    surface: DocumentCommandSurface<'r>,
    config: LexerConfig,
    /// The **document's** proven command-identity facts, so a
    /// role, frame-effect, or structural handler is chosen by the command a
    /// head *is* rather than the one it is spelled as.
    ///
    /// Positioned scans retain the actual evaluated body origin and its map;
    /// standalone scans require conservative unpositioned identity consensus.
    identities: &'r crate::realm::CommandBindingRealm,
    executed_source: Option<&'r crate::command_binding::ExecutedScriptSource>,
    original_logical_body:
        Option<&'r std::sync::Arc<crate::command_binding::OriginalLogicalProcedureBody>>,
    source_base: Option<u32>,
    document: Option<(&'r str, &'r crate::analyser::AnalysisResult, Option<u32>)>,
    original_logical_lambda: Option<&'r crate::registry_invocation::LogicalLambdaSourceOwner>,
    selected_source_child_body: Option<crate::registry_invocation::OriginalSourceScriptBody>,
}

impl<'p> ScanCtx<'p, '_> {
    fn body_assistance(
        &self,
        source: &str,
        command: &crate::segmenter::SegmentedCommand,
    ) -> Option<Box<crate::registry_invocation::InvocationBodyAssistance>> {
        let bindings = self.identities.source_bindings_ref();
        if let Some(owner) = self.original_logical_lambda {
            return crate::registry_invocation::logical_lambda_parameter_body_assistance(
                &self.surface,
                self.identities,
                owner,
                self.selected_source_child_body.as_ref(),
                source,
                command,
                self.source_base?,
            );
        }
        if let Some(body) = self.original_logical_body {
            return crate::registry_invocation::logical_parameter_body_assistance(
                &self.surface,
                self.identities,
                body,
                source,
                command,
                self.source_base?,
            );
        }
        if self.executed_source.is_none() {
            return crate::registry_invocation::unpositioned_body_assistance(
                &self.surface,
                bindings,
                source,
                self.config,
                command,
                self.identities.source_metadata_context().as_deref(),
            );
        }
        let selected = self
            .executed_source
            .filter(|source| bindings.source_origin() != Some(&source.origin))
            .and_then(|source| bindings.selected_source(source));
        crate::registry_invocation::segmented_body_assistance(
            &self.surface,
            selected.as_ref().unwrap_or(bindings),
            source,
            self.config,
            command,
            self.source_base.unwrap_or(0),
            self.identities.source_metadata_context().as_deref(),
        )
    }

    fn argument_values(
        &self,
        source: &str,
        command: &crate::segmenter::SegmentedCommand,
        aliases: &Aliases<'p>,
    ) -> ArgumentValues<'p> {
        let Some(executed) = self.executed_source else {
            return ArgumentValues::lexical(
                source,
                command,
                self.param_set,
                aliases,
                self.config.nested(),
                self.surface.commands(),
            );
        };
        let bindings = self.identities.source_bindings_ref();
        let selected = (bindings.source_origin() != Some(&executed.origin))
            .then(|| bindings.selected_source(executed))
            .flatten();
        let bindings = selected.as_ref().unwrap_or(bindings);
        let mut tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            self.config,
            command,
        );
        crate::lattice_rebase::rebase_command_tokens(
            &mut tokens,
            i64::from(self.source_base.unwrap_or(0)),
        );
        let parameters = self.parameters;
        let registry = self.surface.commands();
        // Parameter traits describe the original declaration's inputs. A
        // caller's entered values cannot erase or donate that symbolic input.
        let full = tokens
            .words()
            .iter()
            .map(|word| {
                let (spelling, site) = word.sole_variable_substitution()?;
                let name = bindings.symbolic_declaration_formal_value_at(
                    &executed.origin,
                    site,
                    spelling,
                    parameters,
                    (
                        registry,
                        self.identities
                            .source_bindings_ref()
                            .source_metadata_owner(),
                    ),
                )?;
                self.param_set.get(name.as_str()).copied()
            })
            .collect();
        let offset = command
            .span
            .start()
            .checked_add(self.source_base.unwrap_or(0));
        let components = tokens
            .words()
            .iter()
            .map(|word| {
                offset.map_or_else(Vec::new, |offset| {
                    bindings
                        .symbolic_declaration_formal_components_in_word(
                            &executed.origin,
                            offset,
                            word,
                            parameters,
                            (
                                registry,
                                self.identities
                                    .source_bindings_ref()
                                    .source_metadata_owner(),
                            ),
                        )
                        .into_iter()
                        .filter_map(|name| self.param_set.get(name.as_str()).copied())
                        .collect()
                })
            })
            .collect();
        ArgumentValues { full, components }
    }

    fn nested(&self, token: &tcl_lexer::Token, text: &str) -> Self {
        self.nested_at(
            token
                .span
                .start()
                .checked_add(u32::from(token.content_offset)),
            text,
        )
    }

    fn nested_at(&self, offset: Option<u32>, text: &str) -> Self {
        let source_base = self.source_base.and_then(|base| base.checked_add(offset?));
        let source_base = self.executed_source.map_or(source_base, |source| {
            let base = source_base?;
            crate::command_binding::ExecutedScriptSource::contiguous(
                std::sync::Arc::clone(&source.origin),
                text,
                base,
            )
            .map(|_| base)
        });
        Self {
            source_base,
            ..self.clone()
        }
    }

    fn source_slice(
        &self,
        text: &str,
        offset: u32,
    ) -> Option<crate::command_binding::ExecutedScriptSource> {
        let source = self.executed_source?;
        let base = self.source_base?.checked_add(offset)?;
        crate::command_binding::ExecutedScriptSource::contiguous(
            std::sync::Arc::clone(&source.origin),
            text,
            base,
        )
    }
}

/// Everything a param-trait scan needs about the document it is scanning
/// inside: the command surface it resolves against, the dialect lexer
/// configuration the body re-segments under, and the document's proven
/// command-identity facts.
///
/// Bundled so the four public entry points share one parameter rather than
/// four, and so adding a document-level fact does not re-open every signature.
#[derive(Clone, Copy)]
pub struct TraitScanEnv<'a> {
    /// The one command surface this document analyses against
    /// ([`DocumentCommandSurface`]): the caller's already-built,
    /// dialect-aware registry generation plus whatever the document
    /// declares for itself with `# tcl-lsp: stub`. Building
    /// a fresh `CommandRegistry::build_default()` per proc would both be
    /// expensive and miss the dialect-specific `arg_role_resolver` /
    /// `arg_roles` the caller's generation has loaded; a stub like
    /// `# tcl-lsp: stub my_eval {script:body}` makes a `my_eval $param`
    /// invocation mark the parameter `ProcArgTrait::Body`.
    pub surface: DocumentCommandSurface<'a>,
    /// Complete parent configuration retained by the original body/metadata
    /// owner. Body parsers select `config.nested()` locally, including a body
    /// at offset zero; `{*}`, quoting and all other grammar axes remain intact.
    /// The parent configuration remains available for original word joins.
    pub config: LexerConfig,
    /// The document's proven command-identity facts — see
    /// [`crate::realm`].  Pass
    /// [`CommandBindingRealm::none()`](crate::realm::CommandBindingRealm::none)
    /// when there is no document to scan.
    pub identities: &'a crate::realm::CommandBindingRealm,
    /// Exact evaluated body bytes and origin. Missing carriers use only
    /// conservative unpositioned identity consensus.
    pub executed_source: Option<&'a crate::command_binding::ExecutedScriptSource>,
    /// Actual original Logical procedure/formal/body owner. This readonly
    /// source template is independent of evaluated body or Native entry.
    pub original_logical_body:
        Option<&'a std::sync::Arc<crate::command_binding::OriginalLogicalProcedureBody>>,
}

impl TraitScanEnv<'_> {
    fn matches_parameters(self, parameters: &[&str]) -> bool {
        self.original_logical_body.is_none_or(|body| {
            body.parameters()
                .iter()
                .map(|formal| formal.name.as_str())
                .eq(parameters.iter().copied())
        })
    }

    fn matches_source(self, text: &str) -> bool {
        self.executed_source
            .is_none_or(|source| source.text.bytes() == text.as_bytes())
            && self.original_logical_body.is_none_or(|body| {
                let Some(input) = self
                    .identities
                    .source_bindings_ref()
                    .original_logical_source_name_advice_input()
                else {
                    return false;
                };
                let Some(image) = self.identities.original_source_image() else {
                    return false;
                };
                self.config == input.lexer_config()
                    && self.identities.matches_resolved_analysis_input(input)
                    && body.matches_source(image, input)
                    && u32::try_from(text.len()).ok()
                        == Some(
                            body.scope().scope_body().content_span().end()
                                - body.scope().scope_body().content_span().start(),
                        )
                    && body.scope().scope_body().matches_original_content(
                        text,
                        body.scope().scope_body().content_span().start(),
                        self.config,
                    )
            })
    }

    #[cfg(test)]
    fn has_actual_procedure_entry(self) -> bool {
        self.executed_source.is_some_and(|source| {
            self.identities
                .source_bindings_ref()
                .has_actual_procedure_entry(source)
        })
    }
}

/// Frozen input origins for each original word, including the command head.
/// Equal spellings in different positions can read different stored values.
struct ArgumentValues<'p> {
    full: Vec<Option<&'p str>>,
    components: Vec<Vec<&'p str>>,
}

impl<'p> ArgumentValues<'p> {
    fn lexical(
        source: &str,
        command: &crate::segmenter::SegmentedCommand,
        parameters: &HashSet<&'p str>,
        aliases: &Aliases<'p>,
        config: LexerConfig,
        registry: &CommandRegistry,
    ) -> Self {
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            command,
        );
        let original = tokens
            .original_words_match_argv_text()
            .then(|| {
                crate::registry_invocation::original_native_compiler_words(
                    &tcl_lexer::SourceImage::document(source),
                    tokens.words(),
                    command.span.start(),
                    config,
                )
            })
            .flatten();
        let Some(original) = original else {
            return Self {
                full: vec![None; command.argv.len()],
                components: vec![Vec::new(); command.argv.len()],
            };
        };
        let mut scanner = crate::var_refs::VarReferenceScanner::with_config(
            crate::var_refs::VarScanOptions::default(),
            config,
        );
        Self {
            full: original
                .iter()
                .map(|word| {
                    lookup_param(original_scalar_reference_name(word)?, parameters, aliases)
                })
                .collect(),
            components: original
                .iter()
                .map(|word| {
                    original_reference_components(word, registry, &mut scanner)
                        .iter()
                        .filter_map(|name| lookup_param(name, parameters, aliases))
                        .collect()
                })
                .collect(),
        }
    }

    fn argument(&self, index: usize) -> Option<&'p str> {
        self.full.get(index + 1).copied().flatten()
    }

    fn argument_components(&self, index: usize) -> &[&'p str] {
        self.components.get(index + 1).map_or(&[], Vec::as_slice)
    }
}

/// Mutable alias / value-copy state accumulated while scanning a proc body.
/// Bundling the two maps keeps the scan helpers at or under the argument limit.
#[derive(Clone)]
struct Aliases<'p> {
    /// `myVar` (an `upvar` / `namespace upvar` alias name) → the param it
    /// aliases, so a later `set myVar …` writes the caller's variable
    /// (`VarWrite`).
    upvar: HashMap<String, &'p str>,
    retired_inputs: HashSet<String>,
    direct_inputs_known: bool,
    /// A local name → the param whose *value* it currently holds (`set n $p`),
    /// so a later `$n` in a name / command position resolves to that param.
    /// Invalidated when the local is written to anything else.
    value_copies: HashMap<String, &'p str>,
}

impl Default for Aliases<'_> {
    fn default() -> Self {
        Self {
            upvar: HashMap::new(),
            value_copies: HashMap::new(),
            retired_inputs: HashSet::new(),
            direct_inputs_known: true,
        }
    }
}

impl Aliases<'_> {
    fn clear(&mut self) {
        self.upvar.clear();
        self.value_copies.clear();
        self.direct_inputs_known = false;
    }
}

/// Local scalar geometry under this invocation's retained lookup policy.
/// Unknown policy requires agreement across source policies, not a default.
fn is_local_scalar_name(
    name: &str,
    selected: &crate::registry_invocation::ResolvedStatementInvocation,
) -> bool {
    tcl_syntax::naming::split_element_ref(name).is_none()
        && selected
            .dialect
            .and_then(|dialect| dialect.variable_lookup_policy)
            .map_or_else(
                || {
                    tcl_syntax::naming::source_variable_name_is_frame_local_for_all_policies(
                        name.as_bytes(),
                    )
                },
                |policy| {
                    tcl_syntax::naming::source_variable_name_is_frame_local(name.as_bytes(), policy)
                },
            )
}

fn written_template_literal(
    view: &crate::registry_invocation::InvocationBodyAssistance,
    selected: &crate::registry_invocation::ResolvedStatementInvocation,
    written: usize,
) -> Option<String> {
    let mut matching = (0..selected.arguments.len())
        .filter(|index| selected.effective.written_argument(*index) == Some(written));
    let argument = matching.next()?;
    matching.next().is_none().then_some(())?;
    view.caller_template_literal_data(selected, argument)
}

/// Single-level command scan shared by both passes.  Extracts
/// segmented commands from `source` and dispatches each through
/// [`scan_command`].
///
/// `source` has a lifetime independent of `'p` so callers can
/// re-enter with body-arg slices that don't outlive the
/// enclosing source.
fn scan_commands<'p>(
    source: &str,
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
) {
    scan_commands_bounded(source, ctx, traits, aliases, 0);
}

/// [`scan_commands`] plus descent into `[...]` command substitutions, bounded
/// by [`MAX_DEPTH`].  A param used inside a substitution (`return [set $v]`,
/// `set x [foo $v]`, `set ${v}($k)` wrapped in `[...]`) is a first-order use of
/// that param, not a "deep" one, so both the shallow and deep passes see it —
/// the substitution is where much ordinary Tcl computation happens.  Nested
/// substitutions are reached by the recursion.
fn scan_commands_bounded<'p>(
    source: &str,
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
    depth: u8,
) {
    let segments = segment_commands_with_offset_and_config(source, 0, ctx.config.nested());
    for seg in &segments {
        if seg.texts.is_empty() {
            continue;
        }
        scan_segment(source, seg, ctx, traits, aliases, depth);
    }
}

fn scan_segment<'p>(
    source: &str,
    seg: &crate::segmenter::SegmentedCommand,
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
    depth: u8,
) -> (
    Option<Box<crate::registry_invocation::InvocationBodyAssistance>>,
    Aliases<'p>,
    ArgumentValues<'p>,
) {
    let cmd_args: Vec<String> = seg.texts[1..].to_vec();
    // Per-arg "braced literal" flags: a single `Str` word (`{…}`) suppresses
    // substitution, so a `$k` inside it is literal text, not a param ref.
    let braced: Vec<bool> = (1..seg.texts.len())
        .map(|i| {
            seg.argv.get(i).is_some_and(|t| t.kind == TokenType::Str)
                && seg.single_token_word.get(i).copied().unwrap_or(false)
        })
        .collect();
    let values = ctx.argument_values(source, seg, aliases);
    if depth < MAX_DEPTH {
        let config = ctx.config.nested();
        let tokens = crate::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(source),
            config,
            seg,
        );
        if let Some(words) = tokens
            .original_words_match_argv_text()
            .then(|| {
                crate::registry_invocation::original_native_compiler_words(
                    &tcl_lexer::SourceImage::document(source),
                    tokens.words(),
                    seg.span.start(),
                    config,
                )
            })
            .flatten()
        {
            for word in &words {
                let arena = word.executable_parts();
                let mut bodies: Vec<_> = arena
                    .all_parts()
                    .filter_map(|part| match part.part {
                        tcl_lexer::ExecutablePart::Command { body } => Some(body),
                        _ => None,
                    })
                    .collect();
                bodies.sort_by_key(|body| body.start());
                for body in bodies {
                    let Some(inner) = arena
                        .bytes(body)
                        .and_then(|bytes| std::str::from_utf8(bytes).ok())
                    else {
                        continue;
                    };
                    if !inner.trim().is_empty() {
                        let nested = ctx.nested_at(Some(body.start()), inner);
                        scan_commands_bounded(inner, &nested, traits, aliases, depth + 1);
                    }
                }
            }
        }
    }
    let body_aliases = aliases.clone();
    let view = ctx.body_assistance(source, seg);
    #[cfg(debug_assertions)]
    if tcl_dialect::trace_env::var_os("TCL_LSP_TRACE_ORIGINAL_PARAM_ROLES").is_some() {
        eprintln!(
            "ORIGINAL_PARAM_ROLES command local_offset={} base={:?} executed_source={} parameters={} full_values={} components={} assistance={} roles={}",
            seg.span.start(),
            ctx.source_base,
            ctx.executed_source.is_some(),
            ctx.parameters.len(),
            values.full.iter().filter(|value| value.is_some()).count(),
            values.components.iter().map(Vec::len).sum::<usize>(),
            view.is_some(),
            view.as_ref()
                .map_or(0, |view| view.parameter_role_advice().written_roles().len())
        );
    }
    scan_command(
        view.as_deref(),
        &cmd_args,
        &braced,
        &values,
        ctx,
        traits,
        aliases,
    );
    (view, body_aliases, values)
}

/// Recursive scan with depth tracking.  Walks every command at
/// the current level, then descends into the braced body
/// arguments each command declares via the registry's
/// ``ArgRole::Body`` role assignments.
///
/// `$var` / `[cmd]` body args are skipped — they aren't
/// braced bodies, and any `Eval` trait they carry is recorded
/// at the top level by the same call-site's role scan.
//
fn scan_deep<'p>(
    source: &str,
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
    depth: u8,
) {
    if depth > MAX_DEPTH {
        return;
    }

    // Each nested body receives its original pre-dispatch copy state. A
    // conditional body cannot publish its assignments into a sibling path.
    let segments = segment_commands_with_offset_and_config(source, 0, ctx.config.nested());
    for seg in segments {
        if seg.texts.is_empty() {
            continue;
        }
        let (view, body_aliases, values) = scan_segment(source, &seg, ctx, traits, aliases, depth);
        let cmd_args: Vec<&str> = seg.texts[1..].iter().map(String::as_str).collect();
        // One surface answers for both a catalogue command and a
        // document-declared `# tcl-lsp: stub` one, so a stub-defined body
        // arg recurses just like a shipped one.
        let roles = view.as_ref().map_or_else(Vec::new, |view| {
            if ctx.original_logical_lambda.is_some() {
                return view.conditional_logical_lambda_roles().unwrap_or_default();
            }
            let mut roles = view.parameter_role_advice().written_roles();
            if let Some(shape) = view.conditional_parameter_source_shape(ctx.parameters) {
                roles.extend(shape.written_argument_roles());
            }
            roles.extend(view.declared_roles.iter().copied());
            roles
        });
        let body_indices: HashSet<usize> = roles
            .iter()
            .filter_map(|&(index, role)| (role == ArgRole::Body).then_some(index))
            .collect();
        for idx in body_indices {
            let original_body = view
                .as_ref()
                .and_then(|view| view.logical_trait_caller_body(idx));
            if (ctx.original_logical_body.is_some() || ctx.original_logical_lambda.is_some())
                && original_body.is_none()
            {
                continue;
            }
            let Some(body_text) = cmd_args.get(idx) else {
                continue;
            };
            if let Some(token) = seg.argv.get(idx + 1) {
                scan_deep_body(
                    body_text,
                    token,
                    original_body.as_ref(),
                    ctx,
                    traits,
                    &body_aliases,
                    depth,
                );
            }
        }

        // `apply {argList body ?ns?} …` — an apply body runs in a *fresh*
        // call frame with its own parameters; recursing into it with the
        // enclosing proc's own `ctx`/`traits` (as a plain `Body` arg would)
        // conflates the two frames. A lambda-local variable that happens to
        // share a name with an enclosing param (`proc f {body} { apply {x
        // {eval $body}} 1 }`) would wrongly mark `f`'s `body` param as
        // evaluated, while the real forwarding case (`apply {x {eval $x}}
        // $body`) would be missed entirely. Instead: infer the lambda's own
        // traits in complete isolation — as if it were its own tiny proc —
        // then propagate a
        // lambda param's trait back onto an enclosing param only when the
        // corresponding actual argument is a bare, unadorned reference to
        // that enclosing param, i.e. only when the value genuinely flows
        // from the caller's frame into the lambda's.
        for idx in roles
            .iter()
            .filter_map(|&(index, role)| (role == ArgRole::LambdaLiteral).then_some(index))
        {
            scan_deep_lambda(
                source,
                &seg,
                view.as_deref(),
                ctx,
                traits,
                (idx, depth),
                &values,
            );
        }
    }
}

fn deep_lambda_parts(
    source: &str,
    seg: &crate::segmenter::SegmentedCommand,
    ctx: &ScanCtx<'_, '_>,
    idx: usize,
    selected_lambda: Option<crate::registry_invocation::OriginalSourceScriptBody>,
) -> Option<(Vec<crate::signature_scan::types::ParamDef>, String, u32)> {
    let parts = if let Some(body) = selected_lambda {
        let (params_text, body_text) =
            selected_logical_lambda_texts(source, ctx.source_base, &body, ctx.config)?;
        let parameter_defs = crate::signature_scan::params::parse_param_list(
            params_text,
            tcl_syntax::word_rules::WordValueRules::from_config(&ctx.config),
        );
        let body_offset = ctx
            .source_base
            .and_then(|base| body.content_span().start().checked_sub(base))?;
        (parameter_defs, body_text.to_owned(), body_offset)
    } else if let Some((document, analysis, _)) = ctx.document {
        let (params, body, offset) = selected_lambda_source(
            document,
            analysis,
            seg.argv
                .first()
                .and_then(|head| head.span.start().checked_add(ctx.source_base?)),
            idx,
            ctx.config,
        )?;
        (params, body, offset)
    } else {
        let &tok = seg.argv.get(idx + 1)?;
        if tok.kind != tcl_lexer::TokenType::Str {
            return None;
        }
        let elems = crate::lambda_literal::split_lambda_literal(source, tok)?;
        let body_span = elems.body?;
        let body_text = source.get(body_span.start() as usize..body_span.end() as usize)?;
        let params_text = source.get(elems.params.start() as usize..elems.params.end() as usize)?;
        (
            crate::signature_scan::params::parse_param_list(
                params_text,
                tcl_syntax::word_rules::WordValueRules::from_config(&ctx.config),
            ),
            body_text.to_owned(),
            ctx.source_base
                .unwrap_or(0)
                .saturating_add(body_span.start()),
        )
    };
    Some(parts)
}

fn scan_deep_lambda<'p>(
    source: &str,
    seg: &crate::segmenter::SegmentedCommand,
    view: Option<&crate::registry_invocation::InvocationBodyAssistance>,
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    (idx, depth): (usize, u8),
    values: &ArgumentValues<'p>,
) {
    if let Some((document, _, base)) = ctx.document {
        let Some(current_base) = ctx.source_base.or(base) else {
            return;
        };
        let Ok(start) = usize::try_from(current_base) else {
            return;
        };
        let Some(end) = start.checked_add(source.len()) else {
            return;
        };
        let Some(current_image) = document.get(start..end) else {
            return;
        };
        if current_image.as_bytes() != source.as_bytes() {
            return;
        }
    }
    let selected_lambda = view.and_then(|view| view.logical_trait_caller_lambda(idx));
    let selected_lambda_owner = view.and_then(|view| view.logical_trait_caller_lambda_owner(idx));
    if selected_lambda_owner.is_some() && ctx.executed_source.is_some() {
        // A retained Logical lambda is source-role advice, not an
        // entered Native body or a replacement for that owner.
        return;
    }
    if (ctx.original_logical_body.is_some() || ctx.original_logical_lambda.is_some())
        && (selected_lambda.is_none() || selected_lambda_owner.is_none())
    {
        return;
    }
    let Some((lambda_param_defs, body_text, body_offset)) =
        deep_lambda_parts(source, seg, ctx, idx, selected_lambda)
    else {
        return;
    };
    if body_text.trim().is_empty() {
        return;
    }
    let lambda_param_names: Vec<&str> = lambda_param_defs.iter().map(|p| p.name.as_str()).collect();
    if lambda_param_names.is_empty() {
        return;
    }
    let lambda_source = if ctx.executed_source.is_some() {
        let Some(source_base) = ctx.source_base else {
            return;
        };
        let Some(relative_body_offset) = body_offset.checked_sub(source_base) else {
            return;
        };
        ctx.source_slice(&body_text, relative_body_offset)
    } else {
        None
    };
    if ctx.executed_source.is_some() && lambda_source.is_none() {
        return;
    }
    let lambda_traits = infer_param_traits_deep_at_depth(
        &lambda_param_names,
        &body_text,
        TraitScanEnv {
            surface: ctx.surface,
            config: ctx.config,
            identities: ctx.identities,
            executed_source: lambda_source.as_ref(),
            original_logical_body: None,
        },
        depth + 1,
        ctx.document
            .map(|(document, analysis, _)| (document, analysis, Some(body_offset))),
        selected_lambda_owner.as_ref(),
    );
    if lambda_traits.is_empty() {
        return;
    }
    for (param_name, outer_param) in lambda_input_bindings(view, idx, &lambda_param_defs, values) {
        if let Some(lambda_param_traits) = lambda_traits.get(&param_name) {
            traits
                .entry(outer_param)
                .or_default()
                .extend(lambda_param_traits.iter().copied());
        }
    }
}

/// Recover a lambda's formal list and body only through the complete document's
/// retained source invocation. This joins the original selected `LambdaLiteral`
/// ordinal, whole word, source-list children, lexer configuration and body
/// geometry; cooked token text is not used as an alternate source identity.
fn selected_lambda_source(
    document: &str,
    analysis: &crate::analyser::AnalysisResult,
    command_offset: Option<u32>,
    argument: usize,
    config: LexerConfig,
) -> Option<(Vec<crate::signature_scan::types::ParamDef>, String, u32)> {
    let command_offset = command_offset?;
    if analysis.body_lexer_config != Some(config) {
        return None;
    }
    let image = tcl_lexer::SourceImage::document(document);
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    let input = analysis.resolved_input.as_ref()?;
    let context = input.context_registry();
    let words = crate::registry_invocation::source_structure::source_registry_words_at(
        document,
        analysis,
        command_offset,
    )?;
    let selected = words.with_source_schema(
        &context,
        crate::registry_invocation::source_schema_queries::lambda_call,
    )??;
    if selected.lambda_argument != argument {
        return None;
    }
    let lambda = words.original_argument_word(argument)?;
    let fields = tcl_syntax::word_rules::original_static_word_list_elements(lambda)?;
    if !(2..=3).contains(&fields.len()) {
        return None;
    }
    let parameters = crate::signature_scan::params::parse_param_list(
        fields.first()?.value(),
        tcl_syntax::word_rules::WordValueRules::from_config(&config),
    );
    let body = words.original_lambda_body_geometry(&context)?;
    if body.original_container() != lambda || !body.matches_source(&image, config) {
        return None;
    }
    let span = body.content_span();
    let body_text = document.get(span.as_range())?.to_owned();
    (body_text.as_bytes()
        == body
            .original_container()
            .image()
            .bytes()
            .get(span.as_range())?)
    .then_some((parameters, body_text, span.start()))
}

/// Only an ordinary value binding forwards the original outer input. Defaults,
/// variadic list construction and Jim caller links have different value owners.
fn lambda_input_bindings<'p>(
    view: Option<&crate::registry_invocation::InvocationBodyAssistance>,
    lambda: usize,
    parameters: &[crate::signature_scan::types::ParamDef],
    values: &ArgumentValues<'p>,
) -> Vec<(String, &'p str)> {
    use tcl_syntax::formal_params::{
        FormalArgumentBinding, FormalParameter, bind_formal_arguments,
    };
    let Some(selected) = view.and_then(|view| view.caller_name_template_invocation()) else {
        return Vec::new();
    };
    let Some(grammar) = selected
        .dialect
        .and_then(tcl_registry::InvocationDialect::parameter_grammar)
    else {
        return Vec::new();
    };
    let Some(lambda) = (0..selected.arguments.len())
        .find(|index| selected.effective.written_argument(*index) == Some(lambda))
    else {
        return Vec::new();
    };
    let formals: Vec<_> = parameters
        .iter()
        .map(|parameter| FormalParameter {
            name: parameter.name.clone(),
            default: parameter.default_value.clone(),
        })
        .collect();
    let Ok(plan) = bind_formal_arguments(
        &formals,
        selected.arguments.len().saturating_sub(lambda + 1),
        grammar,
    ) else {
        return Vec::new();
    };
    plan.into_iter()
        .filter_map(|binding| {
            let FormalArgumentBinding::Value {
                parameter,
                argument,
            } = binding
            else {
                return None;
            };
            let original = selected.effective.written_argument(lambda + 1 + argument)?;
            Some((formals[parameter].name.clone(), values.argument(original)?))
        })
        .collect()
}

fn scan_deep_body<'p>(
    body_text: &str,
    token: &tcl_lexer::Token,
    original_body: Option<&crate::registry_invocation::OriginalSourceScriptBody>,
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &Aliases<'p>,
    depth: u8,
) {
    if body_text.trim().is_empty() {
        return;
    }
    let mut nested = ctx.nested(token, body_text);
    if let Some(body) = original_body {
        // The actual selected same-frame script owns these bytes. A leading
        // substitution is Tcl code inside this source body, not a computed
        // parent operand. No entered script or runtime value is supplied.
        if nested.source_base != Some(body.content_span().start())
            || !body.matches_original_content(body_text, body.content_span().start(), ctx.config)
            || u32::try_from(body_text.len()).ok()
                != Some(body.content_span().end() - body.content_span().start())
        {
            return;
        }
        if ctx.original_logical_lambda.is_some() {
            nested.selected_source_child_body = Some(body.clone());
        }
    } else {
        let head = body_text.as_bytes();
        if head
            .first()
            .is_some_and(|&byte| byte == b'$' || byte == b'[')
            || head
                .get(1)
                .is_some_and(|&byte| byte == b'$' || byte == b'[')
        {
            return;
        }
    }
    scan_deep(body_text, &nested, traits, &mut aliases.clone(), depth + 1);
}

fn selected_logical_lambda_texts<'a>(
    source: &'a str,
    source_base: Option<u32>,
    body: &crate::registry_invocation::OriginalSourceScriptBody,
    config: LexerConfig,
) -> Option<(&'a str, &'a str)> {
    let container = body.original_container();
    let fields = crate::lambda_literal::split_original_lambda_literal(container)?;
    if container.config() != config || fields.braced_body() != Some(body.content_span()) {
        return None;
    }
    let image = container.image().try_text().ok()?;
    let local_slice = |span: tcl_lexer::Span| {
        let base = source_base?;
        let start = usize::try_from(span.start().checked_sub(base)?).ok()?;
        let end = usize::try_from(span.end().checked_sub(base)?).ok()?;
        let text = source.get(start..end)?;
        (image.get(span.as_range())? == text).then_some(text)
    };
    Some((
        local_slice(fields.params)?,
        local_slice(body.content_span())?,
    ))
}

/// A whole original scalar substitution, including one inside quotes.
fn original_scalar_reference_name(word: &tcl_lexer::NativeWord) -> Option<&str> {
    if word.group().expand {
        return None;
    }
    let arena = word.executable_parts();
    let [component] = arena.list(arena.root()) else {
        return None;
    };
    let tcl_lexer::ExecutablePart::Variable { index: None, .. } = component.part else {
        return None;
    };
    let spelling = std::str::from_utf8(arena.bytes(component.span)?).ok()?;
    tcl_syntax::naming::whole_scalar_variable_reference_name(spelling, word.config())
}

fn original_reference_components(
    word: &tcl_lexer::NativeWord,
    registry: &CommandRegistry,
    scanner: &mut crate::var_refs::VarReferenceScanner,
) -> Vec<String> {
    use tcl_lexer::ExecutablePart;
    enum PendingReference {
        Word(tcl_lexer::NativeWord),
        Name(String),
    }
    let mut pending = vec![PendingReference::Word(word.clone())];
    let mut names = Vec::new();
    while let Some(reference) = pending.pop() {
        let word = match reference {
            PendingReference::Name(name) => {
                names.push(name);
                continue;
            }
            PendingReference::Word(word) => word,
        };
        let arena = word.executable_parts();
        let components: Vec<_> = arena.all_parts().collect();
        for component in components.into_iter().rev() {
            match component.part {
                ExecutablePart::Variable { name, .. } => {
                    if let Some(name) = arena
                        .bytes(name)
                        .and_then(|name| std::str::from_utf8(name).ok())
                    {
                        // The lexer already separates an array's index. A scalar
                        // name retains literal parentheses and dollar bytes.
                        pending.push(PendingReference::Name(name.to_owned()));
                    }
                }
                ExecutablePart::Command { body } => {
                    let Ok(plan) = tcl_lexer::native_script_words_in_with_purpose(
                        word.image().clone(),
                        body,
                        word.config(),
                        tcl_lexer::SourceSegmentationPurpose::NestedScript,
                    ) else {
                        continue;
                    };
                    if plan.fatal_tail.is_some() {
                        continue;
                    }
                    for command in plan.commands.into_iter().rev() {
                        pending.extend(command.words.into_iter().rev().map(PendingReference::Word));
                    }
                }
                ExecutablePart::Expression { expression } => {
                    if let Some(script) = arena
                        .bytes(expression)
                        .and_then(|script| std::str::from_utf8(script).ok())
                    {
                        pending.extend(
                            scanner
                                .scan_lexical_script(script, registry)
                                .into_iter()
                                .rev()
                                .map(PendingReference::Name),
                        );
                    }
                }
                ExecutablePart::Text(_) | ExecutablePart::ParseError(_) => {}
            }
        }
    }
    names
}

/// Trait navigation is a May projection; alias and value-copy state require
/// the separate accepted, definite invocation and original written operands.
fn scan_command<'p>(
    view: Option<&crate::registry_invocation::InvocationBodyAssistance>,
    cmd_args: &[String],
    braced: &[bool],
    values: &ArgumentValues<'p>,
    ctx: &ScanCtx<'p, '_>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
) {
    if let Some(p) = values.full.first().copied().flatten()
        && let Some(set) = traits.get_mut(p)
    {
        set.insert(ProcArgTrait::Command);
    }
    let Some(view) = view else {
        aliases.clear();
        return;
    };
    if let Some(shape) = view.conditional_logical_lambda_shape() {
        apply_arg_role_traits(
            &shape.written_argument_roles(),
            cmd_args,
            braced,
            values,
            traits,
        );
        apply_possible_evaluation_traits(
            std::slice::from_ref(shape),
            cmd_args,
            braced,
            values,
            traits,
        );
        aliases.clear();
        return;
    }
    if let Some(shape) = view.conditional_parameter_source_shape(ctx.parameters)
        && let Some(selected) = view.conditional_parameter_source_invocation(ctx.parameters)
    {
        // Conditional symbolic source usage is a separate May report. Actual
        // observer/lookup debt remains on its genuine prefix; no closed,
        // definite, successful alias or runtime write follows from this path.
        apply_arg_role_traits(
            &selected.written_argument_roles(),
            cmd_args,
            braced,
            values,
            traits,
        );
        apply_possible_evaluation_traits(
            std::slice::from_ref(shape),
            cmd_args,
            braced,
            values,
            traits,
        );
        let original = selected.retained_invocation();
        record_selected_aliases(view, original, values, traits, aliases);
        apply_definite_writes(view, original, values, traits, aliases);
        if shape.may_execute_parameter_body() {
            aliases.clear();
        }
        return;
    }
    if !view.parameter_role_advice().is_closed() {
        aliases.clear();
        apply_arg_role_traits(&view.declared_roles, cmd_args, braced, values, traits);
        return;
    }
    // An unresolved replacement may have no parameter-evaluation grammar.
    // Preserve declaration navigation independently of that execution residual.
    let advice = view.parameter_role_advice();
    apply_arg_role_traits(&advice.written_roles(), cmd_args, braced, values, traits);
    apply_arg_role_traits(&view.declared_roles, cmd_args, braced, values, traits);
    apply_possible_evaluation_traits(advice.candidates(), cmd_args, braced, values, traits);
    if let Some(selected) = view.parameter_invocation(ctx.parameters) {
        record_selected_aliases(view, selected, values, traits, aliases);
        apply_definite_writes(view, selected, values, traits, aliases);
        if advice.may_execute_body() {
            aliases.clear();
        }
    } else {
        aliases.clear();
    }
}

fn apply_possible_evaluation_traits<'p>(
    candidates: &[crate::registry_invocation::RegistryInvocationShape],
    args: &[String],
    braced: &[bool],
    values: &ArgumentValues<'p>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
) {
    use tcl_registry::Traits;
    for candidate in candidates {
        for (index, role) in candidate.written_argument_roles() {
            if role == ArgRole::Body
                && !braced.get(index).copied().unwrap_or(false)
                && candidate.possible_traits.intersects(
                    Traits::EVALUATES_CODE | Traits::DYNAMIC_EVAL_BODY | Traits::DEFERS_BODY,
                )
                && let Some(param) = values.argument(index)
                && let Some(set) = traits.get_mut(param)
            {
                set.insert(ProcArgTrait::Eval);
            }
        }
        if candidate
            .possible_traits
            .contains(Traits::SCRIPT_CONCATENATES_ARGS)
            && candidate
                .possible_traits
                .intersects(Traits::EVALUATES_CODE | Traits::DEFERS_BODY)
            && let Some(first) = candidate
                .roles
                .iter()
                .filter_map(|&(index, role)| {
                    (role == ArgRole::Body)
                        .then_some(candidate.argument_offset + usize::from(index))
                })
                .min()
        {
            for effective in first..candidate.effective.words.len() {
                if let Some(index) = candidate.effective.written_argument(effective)
                    && !braced.get(index).copied().unwrap_or(false)
                    && let Some(param) = values.argument(index)
                    && let Some(set) = traits.get_mut(param)
                {
                    set.insert(ProcArgTrait::Eval);
                }
            }
        }
        if candidate
            .possible_traits
            .contains(Traits::PERFORMS_SUBSTITUTION)
        {
            for index in 0..args.len() {
                if braced.get(index).copied().unwrap_or(false) {
                    continue;
                }
                if let Some(param) = values.argument(index)
                    && let Some(set) = traits.get_mut(param)
                {
                    set.insert(ProcArgTrait::Eval);
                }
            }
        }
        if !candidate.possible_traits.contains(Traits::LOOP_LIST_HEADER) {
            continue;
        }
        for &(index, role) in &candidate.roles {
            if role != ArgRole::LoopVarList {
                continue;
            }
            let effective = candidate.argument_offset + usize::from(index) + 1;
            if let Some(param) = candidate
                .effective
                .written_argument(effective)
                .and_then(|original| values.argument(original))
                && let Some(set) = traits.get_mut(param)
            {
                set.insert(ProcArgTrait::LoopList);
            }
        }
    }
}

fn record_selected_aliases<'p>(
    view: &crate::registry_invocation::InvocationBodyAssistance,
    selected: &crate::registry_invocation::ResolvedStatementInvocation,
    values: &ArgumentValues<'p>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
) {
    let Some((_, first)) = selected_alias_layout(view, selected) else {
        return;
    };
    for source in (first..selected.arguments.len()).step_by(2) {
        let destination = source + 1;
        let input = selected
            .effective
            .written_argument(source)
            .and_then(|original| values.argument(original));
        if let Some(param) = input
            && let Some(set) = traits.get_mut(param)
        {
            set.insert(ProcArgTrait::VarRead);
        }
        if let Some(param) = selected
            .effective
            .written_argument(destination)
            .and_then(|original| values.argument(original))
            && let Some(set) = traits.get_mut(param)
        {
            mark_dynamic_name_local(set);
        }
        let Some(local) = view.caller_template_literal_data(selected, destination) else {
            aliases.clear();
            continue;
        };
        aliases.value_copies.remove(&local);
        aliases.upvar.remove(&local);
        aliases.retired_inputs.insert(local.clone());
        if let Some(param) = input
            && is_local_scalar_name(&local, selected)
        {
            aliases.upvar.insert(local, param);
        }
    }
}

fn selected_alias_layout(
    view: &crate::registry_invocation::InvocationBodyAssistance,
    selected: &crate::registry_invocation::ResolvedStatementInvocation,
) -> Option<(tcl_registry::frame_effect::FrameLevel, usize)> {
    use tcl_registry::frame_effect::{FrameArgLayout, FrameArgumentResolution, FrameLevel};
    if let Some(layout) = view.conditional_parameter_source_alias_layout(selected) {
        return layout.into_option();
    }
    let namespace_alias = selected
        .facts
        .state_transitions
        .declared()
        .into_iter()
        .flat_map(tcl_registry::StateTransitions::facts)
        .filter_map(|fact| match &fact.transition {
            tcl_registry::StateTransition::VariableCellAlias(alias)
                if matches!(
                    alias.target,
                    tcl_registry::VariableAliasTarget::Namespace { .. }
                ) =>
            {
                Some(alias.words.target)
            }
            _ => None,
        })
        .min();
    if let Some(first) = namespace_alias {
        return Some((FrameLevel::Dynamic, first));
    }
    let effect = selected.facts.frame_effect?;
    if effect.layout != FrameArgLayout::AliasPairs {
        return None;
    }
    let words: Vec<_> = selected
        .evaluated_words
        .iter()
        .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
        .collect();
    let arguments =
        tcl_registry::InvocationArguments::structured(&words).with_dialect(selected.dialect?);
    match effect.successful_layout(arguments).layout {
        FrameArgumentResolution::Valid {
            level,
            level_word_len,
        } => Some((level, level_word_len)),
        FrameArgumentResolution::Invalid | FrameArgumentResolution::Unknown => None,
    }
}

fn apply_definite_writes<'p>(
    view: &crate::registry_invocation::InvocationBodyAssistance,
    selected: &crate::registry_invocation::ResolvedStatementInvocation,
    values: &ArgumentValues<'p>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
    aliases: &mut Aliases<'p>,
) {
    if selected_alias_layout(view, selected).is_some() {
        return;
    }
    // Freeze the read-side copy before applying this command's writes.
    let copy = (selected.facts.operation
        == tcl_registry::SemanticOperationId::StructuredLowering(
            tcl_registry::hooks::LoweringHookId::Set,
        )
        && selected.arguments.len() == 2)
        .then(|| {
            let target = view.caller_template_literal_data(selected, 0)?;
            if !is_local_scalar_name(&target, selected) {
                return None;
            }
            let param = selected
                .effective
                .written_argument(1)
                .and_then(|index| values.argument(index))?;
            Some((target, param))
        })
        .flatten();
    for (index, role) in selected.written_argument_roles() {
        if role != ArgRole::VarWrite {
            continue;
        }
        let Some(name) = written_template_literal(view, selected, index) else {
            aliases.clear();
            continue;
        };
        if let Some(param) = aliases.upvar.get(name.as_str())
            && let Some(set) = traits.get_mut(param)
        {
            set.insert(ProcArgTrait::VarWrite);
        }
        aliases.value_copies.remove(name.as_str());
        aliases.retired_inputs.insert(name.clone());
    }
    if let Some((target, param)) = copy {
        aliases.value_copies.insert(target, param);
    }
}

/// Per-arg role-driven trait recording — apply
/// ``ArgRole::Body`` / ``Expr`` / ``VarWrite`` / ``VarRead`` to
/// the matching parameter trait set when an arg is a simple
/// ``$param`` reference (or aliases an upvar'd one).
///
/// `braced[i]` is `true` when argument `i` was a braced (`{…}`) literal word,
/// in which case **no substitution occurs**: `set {arr($k)} 1` names the literal
/// variable `arr($k)`, so the `$k` inside must not be read as a substitution.
/// The braced guard applies only to the variable-name
/// roles; a braced `Body` / `Expr` word still substitutes internally when Tcl
/// evaluates it, and the deep pass recurses into it.
fn apply_arg_role_traits<'p>(
    roles: &[(usize, ArgRole)],
    cmd_args: &[String],
    braced: &[bool],
    values: &ArgumentValues<'p>,
    traits: &mut HashMap<&'p str, HashSet<ProcArgTrait>>,
) {
    for idx in 0..cmd_args.len() {
        for role in roles
            .iter()
            .filter_map(|&(index, role)| (index == idx).then_some(role))
        {
            match role {
                // Body / Expr apply only to a bare `$param` (or upvar-aliased) word:
                // the whole argument *is* the param's value, evaluated as a script /
                // expression.
                ArgRole::Body => {
                    if let Some(p) = values.argument(idx)
                        && let Some(set) = traits.get_mut(p)
                    {
                        set.insert(ProcArgTrait::Body);
                    }
                }
                ArgRole::Expr => {
                    if let Some(p) = values.argument(idx)
                        && let Some(set) = traits.get_mut(p)
                    {
                        set.insert(ProcArgTrait::Expr);
                    }
                }
                // A registry / stub `VarWrite` / `VarRead` role names a variable.
                // Every param whose value flows into that name — the whole `$p`, or
                // a `$p` component of a compound dynamic name (`${p}($k)`,
                // `$p($k)`) — names a CALLEE-LOCAL variable (`set $p 1`,
                // `set ${v}($k)`, `incr $p`), NOT a caller-frame alias.  Record the
                // callee-local refinement (never `VarWrite`; only an `upvar`-aliased
                // write-back is a genuine caller write).  A braced word is a literal
                // name — no substitution — so no param flows into it.
                ArgRole::VarWrite | ArgRole::VarRead
                    if !braced.get(idx).copied().unwrap_or(false) =>
                {
                    for p in values.argument_components(idx) {
                        if let Some(set) = traits.get_mut(p) {
                            mark_dynamic_name_local(set);
                        }
                    }
                }
                // A registry / stub `CommandPrefix` role names a command — a callback
                // command prefix (`tcltest::customMatch`, `selection handle`, a stub
                // `:command_prefix` arg).  A `$param` (or component of a dynamic
                // name) flowing there means the param's value is a command name.
                // Braced words are literal — no substitution.
                ArgRole::CommandPrefix if !braced.get(idx).copied().unwrap_or(false) => {
                    for p in values.argument_components(idx) {
                        if let Some(set) = traits.get_mut(p) {
                            set.insert(ProcArgTrait::Command);
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Map a substituted name to a parameter value, directly or through an
/// accepted value copy. An upvar's contents belong to the referenced cell;
/// they are not the parameter string that named that cell.
fn lookup_param<'p>(
    var_name: &str,
    param_set: &HashSet<&'p str>,
    aliases: &Aliases<'p>,
) -> Option<&'p str> {
    aliases.value_copies.get(var_name).copied().or_else(|| {
        (aliases.direct_inputs_known && !aliases.retired_inputs.contains(var_name))
            .then(|| param_set.get(var_name).copied())
            .flatten()
    })
}

/// Record the callee-local dynamic-name use of a param — its **value**
/// is used as a variable *name* in the proc's own scope (``set $p 1``,
/// ``scan $s %d $p``, ``lassign $l $p``, ``regsub … $p``).  Emits
/// [`ProcArgTrait::DynamicNameLocal`] (the refinement that stops
/// caller-side dead-store / unused-variable suppression from treating
/// the call site as consuming the caller's variable) plus
/// [`ProcArgTrait::VarRead`] (the param's string *is* read).  Never
/// `VarWrite` — only an ``upvar``-aliased write-back is a genuine
/// caller-frame write.
fn mark_dynamic_name_local(set: &mut HashSet<ProcArgTrait>) {
    set.insert(ProcArgTrait::DynamicNameLocal);
    set.insert(ProcArgTrait::VarRead);
}

/// The parameters whose **value names a variable in the immediate caller's
/// frame** — `upvar 1 $param local` and its default-level spelling `upvar
/// $param local`, and nothing else.
///
/// This symbolic layout is conditional on entry to the retained declaration.
/// It names no caller cell and grants no read, store or successful activation.
/// Call-site navigation selects the actual callee and argument independently.
/// It is strictly narrower
/// than [`ProcArgTrait::VarWrite`] / [`ProcArgTrait::VarRead`], which record
/// only *that* a parameter's value is used as a variable name through an
/// `upvar`, never *which frame* the alias lands in.  Every other level names a
/// different frame, pinned identical on tclsh 9.0.4 and 8.6.14:
///
/// | callee body | `p x` from a proc writes … |
/// |---|---|
/// | `upvar 1 $n a; set a 1` | the caller's `x` |
/// | `upvar 0 $n a; set a 1` | the **callee's own** `x` — caller's `x` never exists |
/// | `upvar #0 $n a; set a 1` | the **global** `::x` |
/// | `upvar 2 $n a; set a 1` | the caller's **caller** — the immediate caller is skipped |
///
/// `namespace upvar ns $token local` is deliberately absent too: it aliases a
/// *namespace* variable, so a call site passing `token` creates nothing in the
/// calling frame either.
///
/// Only the top-level command scan is walked, matching the shallow trait pass
/// exactly — a consumer that requires both facts is then never widened by a
/// disagreement between them, only narrowed.
#[must_use]
pub fn caller_frame_upvar_params(
    params: &[&str],
    body_source: &str,
    env: TraitScanEnv<'_>,
) -> HashSet<String> {
    caller_frame_upvar_params_over(params, body_source, env, &std::cell::OnceCell::new())
}

/// One top-level command of a body with its assistance.
type TopLevelView = (
    crate::segmenter::SegmentedCommand,
    Option<Box<crate::registry_invocation::InvocationBodyAssistance>>,
);

/// Every top-level command of `body_source` with its assistance. The
/// assistance reads the scan environment and never the parameters, so the
/// caller-frame scans of one body share it.
fn top_level_views(body_source: &str, ctx: &ScanCtx<'_, '_>) -> Vec<TopLevelView> {
    segment_commands_with_offset_and_config(body_source, 0, ctx.config.nested())
        .into_iter()
        .map(|seg| {
            let view = ctx.body_assistance(body_source, &seg);
            (seg, view)
        })
        .collect()
}

/// [`caller_frame_upvar_params`] and [`caller_frame_literal_targets`] of one
/// body, reading each top-level command's assistance once.
#[must_use]
pub fn caller_frame_facts(
    params: &[&str],
    body_source: &str,
    env: TraitScanEnv<'_>,
) -> (HashSet<String>, HashMap<String, bool>) {
    let views = std::cell::OnceCell::new();
    (
        caller_frame_upvar_params_over(params, body_source, env, &views),
        caller_frame_literal_targets_over(body_source, env, &views),
    )
}

fn caller_frame_upvar_params_over(
    params: &[&str],
    body_source: &str,
    env: TraitScanEnv<'_>,
    views: &std::cell::OnceCell<Vec<TopLevelView>>,
) -> HashSet<String> {
    let mut out = HashSet::new();
    if params.is_empty()
        || body_source.trim().is_empty()
        || !env.matches_source(body_source)
        || !env.matches_parameters(params)
    {
        return out;
    }
    let param_set: HashSet<&str> = params.iter().copied().collect();
    let ctx = ScanCtx {
        param_set: &param_set,
        parameters: params,
        surface: env.surface,
        config: env.config,
        identities: env.identities,
        executed_source: env.executed_source,
        original_logical_body: env.original_logical_body,
        document: None,
        original_logical_lambda: None,
        selected_source_child_body: None,
        source_base: env
            .executed_source
            .map(crate::command_binding::ExecutedScriptSource::base)
            .or_else(|| {
                env.original_logical_body
                    .map(|body| body.scope().scope_body().content_span().start())
            }),
    };
    for (seg, view) in views.get_or_init(|| top_level_views(body_source, &ctx)) {
        let Some(view) = view.as_deref() else {
            continue;
        };
        let source = view.conditional_parameter_source_invocation(params);
        let Some(selected) = view.parameter_invocation(params).or_else(|| {
            source
                .as_ref()
                .map(super::super::registry_invocation::ConditionalParameterSourceInvocation::retained_invocation)
        }) else {
            continue;
        };
        let Some((level, first)) = selected_alias_layout(view, selected) else {
            continue;
        };
        if !level.is_caller_frame() {
            continue;
        }
        let values = ctx.argument_values(body_source, seg, &Aliases::default());
        for source in (first..selected.arguments.len()).step_by(2) {
            if let Some(param) = selected
                .effective
                .written_argument(source)
                .and_then(|original| values.argument(original))
            {
                out.insert(param.to_owned());
            }
        }
    }
    out
}

/// The **literal caller-frame targets** a proc body binds through `upvar` —
/// caller-frame names the callee spells in its *own* text, with no call-site
/// argument word to key on (`upvar 1 name name`).  Returns
/// `caller_name → written-through-alias`.
///
/// Strictly the complement of [`caller_frame_upvar_params`]: that fact keys
/// on a `$param` **source** (the name arrives from the call site); this one
/// keys on a *literal* source (the name is fixed by the callee).  tclsh
/// 9.0.4 / 8.6.14, identical: with `proc np {} {upvar name name; set name
/// W1}`, a caller running `np; puts $name` prints `W1` — `name` exists in
/// the caller's frame although the caller never assigns it and no call-site
/// word spells it.
///
/// This is an owned callee template, conditional on its entry and normal alias
/// operations. It identifies no actual caller cell and proves no completed
/// write. Navigation instantiates it only with a separate original
/// call/allocation/frame receipt; execution and value consumers use actual
/// source access and continuation proofs instead.
///
/// Exclusions, each the abstaining direction:
///
/// * a non-caller frame level (`upvar 0` / `#0` / `2` / `$lvl`) — a
///   different frame entirely, see [`caller_frame_upvar_params`]'s table;
/// * a `::`-qualified source (`upvar 1 ::ns::x local`) — a fixed global/
///   namespace cell, level-independent, already linked by the analyser's
///   alias consumer (`apply_state_transitions`) `otherVar` link;
/// * an array element or any substituted/computed source — not a plain
///   caller-frame scalar name this scan can claim;
/// * a dynamic **local** side — with no alias name, the write-through scan
///   below has nothing to match, so the pair contributes nothing here (the
///   compiler-side summary still widens the call site).
///
/// The write-through upgrade mirrors [`record_upvar_pairs`]'s rule for
/// params: a later registry-declared `VarWrite` on the alias local (`set
/// name …`, `lassign … name`) marks the caller name as *created* by a call;
/// an alias only ever read leaves it `false` (a referencing call site, not a
/// creating one).  Only the top-level command scan is walked, matching the
/// shallow trait pass exactly.
#[must_use]
pub fn caller_frame_literal_targets(
    body_source: &str,
    env: TraitScanEnv<'_>,
) -> HashMap<String, bool> {
    caller_frame_literal_targets_over(body_source, env, &std::cell::OnceCell::new())
}

fn caller_frame_literal_targets_over(
    body_source: &str,
    env: TraitScanEnv<'_>,
    views: &std::cell::OnceCell<Vec<TopLevelView>>,
) -> HashMap<String, bool> {
    if !env.matches_source(body_source) {
        return HashMap::new();
    }
    let params = HashSet::new();
    let ctx = ScanCtx {
        param_set: &params,
        parameters: &[],
        surface: env.surface,
        config: env.config,
        identities: env.identities,
        executed_source: env.executed_source,
        original_logical_body: env.original_logical_body,
        document: None,
        original_logical_lambda: None,
        selected_source_child_body: None,
        source_base: env
            .executed_source
            .map(crate::command_binding::ExecutedScriptSource::base)
            .or_else(|| {
                env.original_logical_body
                    .map(|body| body.scope().scope_body().content_span().start())
            }),
    };
    let mut targets = HashMap::new();
    if body_source.trim().is_empty() || !env.matches_source(body_source) {
        return targets;
    }
    let mut alias_to_target = HashMap::new();
    for (_, view) in views.get_or_init(|| top_level_views(body_source, &ctx)) {
        let Some(view) = view.as_deref() else {
            continue;
        };
        let Some(selected) = view.caller_name_template_invocation() else {
            continue;
        };
        if let Some((level, first)) = selected_alias_layout(view, selected) {
            if !level.is_caller_frame() {
                continue;
            }
            for source in (first..selected.arguments.len()).step_by(2) {
                let (Some(name), Some(local)) = (
                    view.caller_template_literal_data(selected, source),
                    view.caller_template_literal_data(selected, source + 1),
                ) else {
                    continue;
                };
                if is_local_scalar_name(&name, selected) && is_local_scalar_name(&local, selected) {
                    targets.entry(name.clone()).or_insert(false);
                    alias_to_target.insert(local, name);
                }
            }
            continue;
        }
        for (index, role) in selected.written_argument_roles() {
            if role != ArgRole::VarWrite {
                continue;
            }
            if let Some(name) = written_template_literal(view, selected, index)
                && let Some(caller_name) = alias_to_target.get(&name)
            {
                targets.insert(caller_name.clone(), true);
            }
        }
    }
    targets
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_name_data_tests;
