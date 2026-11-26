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

use tcl_registry::CommandRegistry;
#[cfg(test)]
use tcl_registry::arg_role::ArgRole;
use tcl_registry::model::DocumentCommandSurface;

use super::types::{ProcArgTrait, ScriptParameter};
#[cfg(test)]
use crate::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::LexerConfig;

mod body_scan;
mod caller_frame;
mod command_traits;

#[cfg(test)]
use body_scan::selected_lambda_source;
use body_scan::{scan_commands, scan_deep};
pub use caller_frame::{
    caller_frame_facts, caller_frame_literal_targets, caller_frame_upvar_params,
};
#[cfg(test)]
use command_traits::is_local_scalar_name;
use command_traits::lookup_param;

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
    if params.is_empty() || !env.matches_parameters(params) {
        return HashMap::new();
    }
    shallow_param_traits(params, body_source, env, None)
}

/// Shallow parameter traits and direct caller-frame templates for one original body.
pub struct ShallowProcedureTraits {
    /// Traits inferred from the procedure arguments' use.
    pub traits: HashMap<String, HashSet<ProcArgTrait>>,
    /// Parameters naming variables in the immediate caller frame.
    pub caller_frame_params: HashSet<String>,
    /// Literal immediate caller targets and whether their aliases are written.
    pub caller_frame_literals: HashMap<String, bool>,
}

/// Project parameter traits and direct caller-frame effects during the same scan.
#[must_use]
pub fn infer_param_traits_with_caller_frame(
    params: &[&str],
    body_source: &str,
    env: TraitScanEnv<'_>,
) -> ShallowProcedureTraits {
    let params = if env.matches_parameters(params) {
        params
    } else {
        &[]
    };
    let mut direct = crate::cfg_builder::frame_effects::DirectCallerEffects::default();
    let traits = shallow_param_traits(params, body_source, env, Some(&mut direct));
    let (caller_frame_params, caller_frame_literals) = direct.into_caller_template();
    ShallowProcedureTraits {
        traits,
        caller_frame_params,
        caller_frame_literals,
    }
}

fn shallow_param_traits(
    params: &[&str],
    body_source: &str,
    env: TraitScanEnv<'_>,
    direct: Option<&mut crate::cfg_builder::frame_effects::DirectCallerEffects>,
) -> HashMap<String, HashSet<ProcArgTrait>> {
    if body_source.trim().is_empty() || !env.matches_source(body_source) {
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
    scan_commands(body_source, &ctx, &mut traits, &mut aliases, direct);

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

/// Remove the per-use [`ProcArgTrait::Script`] entries from `traits` and
/// reduce each formal's uses to its [`ScriptParameter`] contract.
/// Parameters left with no trait are dropped.
pub fn take_script_parameters<S, H>(
    traits: &mut HashMap<String, HashSet<ProcArgTrait, H>, S>,
) -> HashMap<String, ScriptParameter>
where
    S: std::hash::BuildHasher,
    H: std::hash::BuildHasher,
{
    let mut contracts = HashMap::new();
    for (param, set) in traits.iter_mut() {
        let mut script_uses = Vec::new();
        set.retain(|item| match item {
            ProcArgTrait::Script(contract) => {
                script_uses.push(*contract);
                false
            }
            _ => true,
        });
        if let Some(contract) = ScriptParameter::of_uses(script_uses) {
            contracts.insert(param.clone(), contract);
        }
    }
    traits.retain(|_, set| !set.is_empty());
    contracts
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

#[cfg(test)]
mod tests;

#[cfg(test)]
mod original_name_data_tests;
