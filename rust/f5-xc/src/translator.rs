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

//! Core iRule → F5 XC static translator.
//!
//! Walks the IR produced by [`lower_to_ir`](tcl_compiler::lowering::lower_to_ir)
//! and builds an [`XCTranslationResult`] mapping iRule constructs to
//! XC-native equivalents (routes, service policies, header actions, origin
//! pools).

use std::collections::BTreeMap;
use std::sync::Arc;

use tcl_compiler::analyser::{Analyser, AnalysisResult, ResolvedAnalysisInput};
use tcl_compiler::compilation_unit::{CompilationUnit, UnitBuildOptions};
use tcl_compiler::ir::Module;
#[cfg(test)]
use tcl_compiler::ir::Statement;
#[cfg(test)]
use tcl_compiler::registry_invocation::source_structure::original_single_source_command_substitution;
use tcl_core_types::DiagCode;
#[cfg(test)]
use tcl_lexer::SourceImage;
use tcl_lexer::Span;
#[cfg(test)]
use tcl_lexer::{NativeWord, native_script_words_in};
use tcl_registry::CommandRegistry;
use tcl_syntax::expr::ast::BinOp;
#[cfg(test)]
use tcl_syntax::expr::ast::ExprNode;
#[cfg(test)]
use tcl_syntax::expr::parse_expr;

use crate::mapping::{advisory_event, translatable_event, untranslatable_event};
use crate::model::{
    TranslateStatus, TranslationItem, XCConstructKind, XCCookieMatch, XCHeaderAction,
    XCHeaderMatch, XCHostMatch, XCMethodMatch, XCOriginPool, XCPathMatch, XCQueryMatch, XCRoute,
    XCServicePolicy, XCServicePolicyRule, XCTranslationResult, XCWafExclusionRule,
};

mod ir_walk;
mod match_criteria;
mod source_advice;

use ir_walk::walk_script;
#[cfg(test)]
use match_criteria::condition_to_path_match;
#[cfg(test)]
use source_advice::SourceExpressionAdvice;

const MAX_DEPTH: u32 = 10;

const EQUALITY_OPS: [BinOp; 3] = [BinOp::StrEq, BinOp::Eq, BinOp::StrEquals];
const NEGATED_EQUALITY_OPS: [BinOp; 2] = [BinOp::StrNe, BinOp::Ne];

fn is_http_path_command(cmd: &str) -> bool {
    cmd == "HTTP::path" || cmd == "HTTP::uri"
}

// Enclosing context (threaded through from enclosing if/switch)

/// Match criteria accumulated from enclosing conditions.
#[derive(Debug, Clone, Default)]
struct EnclosingContext {
    path: Option<XCPathMatch>,
    host: Option<XCHostMatch>,
    method: Option<XCMethodMatch>,
    ip_prefix_set: Option<String>,
    header_matches: Vec<XCHeaderMatch>,
    cookie_matches: Vec<XCCookieMatch>,
    query_match: Option<XCQueryMatch>,
    ip_prefix_list: Vec<String>,
}

// Low-level helpers

/// Original words from one complete source command under the translation grammar.
#[cfg(test)]
fn original_command_words(text: &str) -> Option<Vec<NativeWord>> {
    let config = tcl_lexer::LexerConfig::from_grammar(
        tcl_registry::model::resolve_environment("f5-irules").grammar(),
    );
    let image = SourceImage::document(text);
    let region = Span::new(0, u32::try_from(text.len()).ok()?);
    let mut plan = native_script_words_in(image.clone(), region, config).ok()?;
    if plan.fatal_tail.is_some() || plan.commands.len() != 1 {
        return None;
    }
    let words = plan.commands.pop()?.words;
    if words.iter().any(|word| word.group().expand) {
        return None;
    }
    if words.len() == 1 && text.trim_start().starts_with('[') {
        return Some(
            original_single_source_command_substitution(&words[0])?
                .command()
                .words
                .clone(),
        );
    }
    Some(words)
}

#[cfg(test)]
fn original_literal_value(word: &NativeWord) -> Option<String> {
    String::from_utf8(tcl_syntax::word_rules::original_static_word_unicode_value(
        word,
    )?)
    .ok()
}

/// Scalar getter advice requires a complete zero-argument command or a sole
/// executable getter operand of a case conversion. Literal brackets are data.
#[cfg(test)]
fn is_http_getter(text: &str) -> Option<String> {
    scalar_getter(text).filter(|name| name.starts_with("HTTP::"))
}

#[cfg(test)]
fn scalar_getter(text: &str) -> Option<String> {
    scalar_getter_words(original_command_words(text)?)
}

#[cfg(test)]
fn scalar_getter_words(mut words: Vec<NativeWord>) -> Option<String> {
    loop {
        let command = original_literal_value(words.first()?)?;
        if words.len() == 1 {
            return Some(command);
        }
        if command != "string" || words.len() != 3 {
            return None;
        }
        let subcommand = original_literal_value(&words[1])?;
        if !matches!(subcommand.as_str(), "tolower" | "toupper") {
            return None;
        }
        words = original_single_source_command_substitution(&words[2])?
            .command()
            .words
            .clone();
    }
}

#[cfg(test)]
fn scalar_getter_operand(word: &NativeWord) -> Option<String> {
    scalar_getter_words(
        original_single_source_command_substitution(word)?
            .command()
            .words
            .clone(),
    )
}

/// Literal fields and sole executable command operands retain shared geometry.
/// Dynamic values and expanded words cannot supply static translation facts.
#[cfg(test)]
fn parse_command_parts(text: &str) -> Vec<String> {
    let Some(words) = original_command_words(text) else {
        return Vec::new();
    };
    let parts: Option<Vec<String>> = words
        .iter()
        .map(|word| {
            original_literal_value(word).or_else(|| {
                original_single_source_command_substitution(word)?;
                Some(
                    word.image()
                        .try_text()
                        .ok()?
                        .get(word.span().as_range())?
                        .to_owned(),
                )
            })
        })
        .collect();
    parts.unwrap_or_default()
}

// Translation state

/// Mutable accumulator for translation results.
struct TranslationContext<'a> {
    source: &'a str,
    registry: &'a CommandRegistry,
    analysis: &'a AnalysisResult,
    module: &'a Module,
    routes: Vec<XCRoute>,
    policy_rules: Vec<XCServicePolicyRule>,
    origin_pools: BTreeMap<String, XCOriginPool>,
    origin_pool_order: Vec<String>,
    header_actions: Vec<XCHeaderAction>,
    waf_exclusion_rules: Vec<XCWafExclusionRule>,
    items: Vec<TranslationItem>,
    rule_counter: u32,
    current_event: String,
}

impl<'a> TranslationContext<'a> {
    fn new(
        source: &'a str,
        analysis: &'a AnalysisResult,
        module: &'a Module,
        registry: &'a CommandRegistry,
    ) -> Self {
        Self {
            source,
            registry,
            analysis,
            module,
            routes: Vec::new(),
            policy_rules: Vec::new(),
            origin_pools: BTreeMap::new(),
            origin_pool_order: Vec::new(),
            header_actions: Vec::new(),
            waf_exclusion_rules: Vec::new(),
            items: Vec::new(),
            rule_counter: 0,
            current_event: String::new(),
        }
    }

    fn next_rule_name(&mut self) -> String {
        self.rule_counter += 1;
        format!("rule-{}", self.rule_counter)
    }

    fn add_origin_pool(&mut self, name: &str, source_range: Option<Span>) {
        if !self.origin_pools.contains_key(name) {
            self.origin_pools.insert(
                name.to_owned(),
                XCOriginPool {
                    name: name.to_owned(),
                    port: 80,
                },
            );
            self.origin_pool_order.push(name.to_owned());
        }
        self.items.push(TranslationItem {
            status: TranslateStatus::Translated,
            kind: XCConstructKind::OriginPool,
            irule_command: format!("pool {name}"),
            irule_range: source_range,
            xc_description: format!("Origin pool: {name}"),
            note: String::new(),
            diagnostic_code: DiagCode::Xc100,
        });
    }
}

// Public API

/// The profile-stamped iRules [`CommandRegistry`], suitable for lowering and
/// translating an iRule (the §9 operator-head exclusion applies to every
/// dialect-availability query made through it, and every spec's own surface
/// decides the rest).
fn irules_registry() -> &'static CommandRegistry {
    tcl_registry::model::ingress::static_context_for_profile(tcl_dialect::DialectProfile::irules())
        .commands()
}

fn translation_source(
    source: &str,
    registry: &CommandRegistry,
) -> (Arc<CompilationUnit>, AnalysisResult) {
    let environment = tcl_registry::model::resolve_environment("f5-irules");
    let config = tcl_lexer::LexerConfig::from_grammar(environment.grammar());
    let availability = Arc::new(
        environment
            .default_context_registry()
            .with_command_store(registry.snapshot().shared_registry()),
    );
    let input = ResolvedAnalysisInput::new(
        environment.analyser_profile(),
        environment.unit_profile(),
        availability,
        config,
    );
    let mut analyser = Analyser::new().with_resolved_input(input);
    let input = analyser
        .prepare_analysis_input("f5-irules")
        .expect("explicit registry context is available");
    let unit = Arc::new(CompilationUnit::build_with_analysis_input(
        source,
        UnitBuildOptions {
            registry,
            defer_top_level: false,
            config,
            dialect: Some(environment.unit_profile()),
            external_call_sites: None,
            declared_commands: None,
        },
        None,
        &input,
    ));
    analyser.set_cu_override(Arc::clone(&unit));
    let analysis = analyser.analyse(source, "f5-irules");
    (unit, analysis)
}

/// Translate an iRule to F5 XC configuration. Analyses the iRule source,
/// walks the IR, and returns an [`XCTranslationResult`] with routes,
/// service policies, origin pools, header actions, and a coverage report.
#[must_use]
pub fn translate_irule(source: &str) -> XCTranslationResult {
    translate_irule_with_registry(source, irules_registry())
}

/// [`translate_irule`] against a caller-provided registry (must have the
/// iRules dialect loaded).
#[must_use]
pub fn translate_irule_with_registry(
    source: &str,
    registry: &CommandRegistry,
) -> XCTranslationResult {
    let (unit, analysis) = translation_source(source, registry);
    translate_unit(source, &unit, &analysis, registry)
}

/// Translate an open document's iRule under the document's own retained
/// analysis and compilation unit, rather than reparsing its text under the
/// standalone iRules defaults [`translate_irule`] uses.
///
/// The registry is the one the analysis retained, so the document's
/// configured availability, packs and lexer configuration are the ones the
/// translation walks. `None` when the analysis does not describe exactly
/// `source` as an iRules document, or `unit` was built from other text: a
/// document translation never falls back to the standalone profile.
#[must_use]
pub fn translate_irule_for_document(
    source: &str,
    unit: &CompilationUnit,
    analysis: &AnalysisResult,
) -> Option<XCTranslationResult> {
    if analysis.analysis_context_unavailable.is_some() || unit.source != source {
        return None;
    }
    tcl_compiler::source_graph::current_analysis(source, analysis)?;
    let input = analysis.resolved_input.as_ref()?;
    if !input.unit_profile().is_irules() {
        return None;
    }
    let registry = analysis.resolved_registry()?;
    Some(translate_unit(source, unit, analysis, registry))
}

fn translate_unit(
    source: &str,
    unit: &CompilationUnit,
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
) -> XCTranslationResult {
    let module = &unit.ir_module;
    let mut ctx = TranslationContext::new(source, analysis, module, registry);

    // Original event ownership selects handlers; display names only order output.
    let mut event_procs = Vec::new();
    let mut regular_procs = Vec::new();
    for (id, procedure) in module.procedures.iter() {
        if let Some(event) =
            tcl_compiler::source_graph::event_body_for_procedure(module, id, procedure, registry)
        {
            event_procs.push((id, procedure, event));
        } else {
            regular_procs.push((id, procedure));
        }
    }
    event_procs.sort_by(|a, b| {
        a.1.qualified_name
            .cmp(&b.1.qualified_name)
            .then(a.0.cmp(b.0))
    });
    regular_procs.sort_by(|a, b| {
        a.1.qualified_name
            .cmp(&b.1.qualified_name)
            .then(a.0.cmp(b.0))
    });

    for (_, proc, event) in event_procs {
        let event_name = event.event().to_owned();
        ctx.current_event.clone_from(&event_name);

        if translatable_event(&event_name).is_some() {
            walk_script(
                &proc.body,
                &mut ctx,
                registry,
                0,
                &EnclosingContext::default(),
            );
        } else if let Some(desc) = untranslatable_event(&event_name) {
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Untranslatable,
                kind: XCConstructKind::Note,
                irule_command: format!("when {event_name}"),
                irule_range: Some(proc.span),
                xc_description: desc.to_owned(),
                note: "This entire event handler has no XC equivalent.".to_owned(),
                diagnostic_code: DiagCode::Xc201,
            });
        } else if let Some(desc) = advisory_event(&event_name) {
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Advisory,
                kind: XCConstructKind::Note,
                irule_command: format!("when {event_name}"),
                irule_range: Some(proc.span),
                xc_description: desc.to_owned(),
                note: "This event maps to a separate XC feature.".to_owned(),
                diagnostic_code: DiagCode::Xc250,
            });
        } else {
            ctx.items.push(TranslationItem {
                status: TranslateStatus::Untranslatable,
                kind: XCConstructKind::Note,
                irule_command: format!("when {event_name}"),
                irule_range: Some(proc.span),
                xc_description: format!("Event '{event_name}' has no known XC equivalent."),
                note: String::new(),
                diagnostic_code: DiagCode::Xc201,
            });
        }
    }

    // Also walk regular procs that may be called from event handlers.
    for (_, proc) in regular_procs {
        ctx.current_event = String::new();
        walk_script(
            &proc.body,
            &mut ctx,
            registry,
            0,
            &EnclosingContext::default(),
        );
    }

    // Build service policy from collected rules.
    let service_policies = if ctx.policy_rules.is_empty() {
        Vec::new()
    } else {
        vec![XCServicePolicy {
            name: "translated-policy".to_owned(),
            algo: "FIRST_MATCH".to_owned(),
            rules: std::mem::take(&mut ctx.policy_rules),
        }]
    };

    // Coverage.
    let total = ctx.items.len();
    let coverage = if total == 0 {
        100.0
    } else {
        let translated = ctx
            .items
            .iter()
            .filter(|i| {
                matches!(
                    i.status,
                    TranslateStatus::Translated | TranslateStatus::Advisory
                )
            })
            .count();
        let partial = ctx
            .items
            .iter()
            .filter(|i| i.status == TranslateStatus::Partial)
            .count();
        // Small item counts; the f64 coverage percentage is display-only.
        #[allow(clippy::cast_precision_loss)]
        let cov = ((translated as f64 + partial as f64 * 0.5) / total as f64) * 100.0;
        cov
    };

    let origin_pools: Vec<XCOriginPool> = ctx
        .origin_pool_order
        .iter()
        .filter_map(|n| ctx.origin_pools.get(n).cloned())
        .collect();

    XCTranslationResult {
        routes: ctx.routes,
        service_policies,
        origin_pools,
        header_actions: ctx.header_actions,
        waf_exclusion_rules: ctx.waf_exclusion_rules,
        items: ctx.items,
        coverage_pct: round_1dp(coverage),
    }
}

/// Round to one decimal place using round-half-to-even (banker's rounding).
fn round_1dp(x: f64) -> f64 {
    let scaled = x * 10.0;
    let rounded = scaled.round_ties_even();
    rounded / 10.0
}

#[cfg(test)]
mod original_getter_geometry_tests;

#[cfg(test)]
mod original_call_owner_tests;

#[cfg(test)]
mod original_expression_owner_tests;

#[cfg(test)]
mod supplied_source_registry_tests;
