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

//! Bounded lexical name ownership for diagnostic suppression.
//!
//! A conditional or opaque body can mention names that it binds internally.
//! This owner follows registry-described variable operands and same-frame
//! script arguments; it excludes declaration bodies, foreign frames and opaque
//! scripts. It supplies no executed store, current value or command identity.
//! Variable lists use the retained lexical policy's Tcl list grammar.
//! A depth limit returns no ownership advice; it never invents a binding.

use tcl_registry::{ArgRole, CommandRegistry};

/// How deep to follow nested script words.
///
/// Every level is a `{…}` the answer has to look inside, and real bodies nest
/// a handful deep — a `foreach` around an `if` around a `catch` is three. The
/// bound exists so a pathological body cannot make this walk quadratic in a
/// hot analyser path; refusing to descend further can only return `false`,
/// which is the conservative answer (the read stands).
const MAX_DEPTH: u8 = 6;

/// Which mentions of `name` count as the script owning it.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(crate) enum Ownership {
    /// Only a binding: an `ArgRole::VarWrite` operand or an
    /// `ArgRole::LoopVarList` entry.  A *read* of the name still belongs to
    /// whichever frame the script runs in, which for an undescribed
    /// command's word may be this one.
    Bindings,
    /// Binding operands evaluated as decoded names. A literal leading `$`
    /// remains part of the name; dynamic name values supply no ownership.
    DecodedBindings,
    /// A binding, or a bare-name read (`set y`, `info exists y`).  The
    /// barrier twin wants this: its body runs in a context this frame cannot
    /// see (`interp eval PATH {…}`), so a name the body names at all is that
    /// context's, and reporting it here would blame the wrong interpreter.
    BindingsOrNameReads,
    /// Names mentioned by registry-described scope-alias declarations.
    /// This supplies lexical exclusion advice, not an installed alias.
    ScopeAliases,
}

/// Conditional name ownership of one authentic original literal body.
/// Full supplied availability, grammar and installer lookup remain independent
/// of entered scripts, variable frames, successful stores or read exclusion.
pub(crate) fn original_literal_body_ownership(
    registry: &CommandRegistry,
    metadata: Option<crate::registry_invocation::InvocationMetadataContext<'_>>,
    parent_tokens: &crate::ir::CommandTokens,
    written_argument: usize,
    purpose: Ownership,
) -> Option<crate::ir_helpers::VariableWriteEffects> {
    // naming.compiler.original-analysis-metadata-context
    // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
    let metadata = metadata.filter(|metadata| metadata.matches_registry(registry))?;
    let binding = parent_tokens.source_binding.as_ref()?;
    let site = binding.invocation_site()?;
    let source = site.source.source_image().try_text().ok()?;
    let footprint =
        binding.original_materialized_footprint(parent_tokens, source, registry, Some(metadata))?;
    footprint.literal_body_name_ownership(parent_tokens, written_argument, purpose)
}

/// True when `word`, read as a script, owns `name` under `ownership` — by any
/// command the registry says takes a variable operand, at any nesting depth
/// up to [`MAX_DEPTH`].
///
/// `name` is a literal variable root, with reference sigils kept separate.
pub(crate) fn script_binds_name(
    word: &str,
    name: &str,
    ownership: Ownership,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> bool {
    script_image_binds_name(
        &tcl_lexer::SourceImage::document(word),
        name,
        ownership,
        registry,
        config,
    )
}

/// Lexical ownership in an unchanged original image. Opaque bytes, incomplete
/// scripts and unavailable segmentation supply no ownership advice. Nested
/// literal body values use native-value channel semantics after their original
/// enclosing word has been evaluated by the shared word owner.
pub(crate) fn script_image_binds_name(
    image: &tcl_lexer::SourceImage,
    name: &str,
    ownership: Ownership,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
) -> bool {
    let Ok(text) = image.try_text() else {
        return false;
    };
    if !text.contains(name) {
        return false;
    }
    binds(image, name, ownership, registry, config, MAX_DEPTH)
}

fn binds(
    script: &tcl_lexer::SourceImage,
    name: &str,
    ownership: Ownership,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    depth: u8,
) -> bool {
    if depth == 0 {
        return false;
    }
    let list_rules = tcl_syntax::word_rules::WordValueRules::from_config(&config);
    let Some(segments) =
        crate::segmenter::segment_commands_image_with_offset_and_config(script, 0, config)
    else {
        return false;
    };
    for segment in segments {
        if segment.is_partial {
            return false;
        }
        let Some((command, args)) = segment.texts.split_first() else {
            continue;
        };
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        let resolution = registry.resolve_structured_invocation(
            tcl_registry::InvocationWords::literals(command, &args),
            registry.profile().and_then(|profile| {
                tcl_registry::InvocationDialect::of_profile(profile).authoring_query()
            }),
        );
        let Some(resolved) = resolution.resolved() else {
            continue;
        };
        let facts = resolved.facts();
        let at = |role: ArgRole| {
            facts
                .arg_roles
                .iter()
                .filter_map(|(index, found)| {
                    (*found == role).then_some(facts.argument_offset + usize::from(*index))
                })
                .collect::<Vec<_>>()
        };

        // An output operand: `catch … err`, `scan … out`, `binary scan … v`,
        // `lassign`'s tail, `incr`, `append`, `lappend`, `upvar`'s locals.
        // `VarRead` joins it for a barrier body — a bare-name read such as
        // the one-argument `set y` names that context's variable too.
        let scope_alias = facts.traits.intersects(
            tcl_registry::Traits::CREATES_SCOPE_ALIAS | tcl_registry::Traits::ALIASES_GLOBAL,
        );
        let name_roles: &[ArgRole] = match ownership {
            Ownership::Bindings | Ownership::DecodedBindings => &[ArgRole::VarWrite],
            Ownership::BindingsOrNameReads => &[ArgRole::VarWrite, ArgRole::VarRead],
            Ownership::ScopeAliases if scope_alias => &[ArgRole::VarWrite],
            Ownership::ScopeAliases => &[],
        };
        if name_roles.iter().copied().flat_map(&at).any(|index| {
            decoded_binding_root(script, &segment, index + 1, config)
                .is_some_and(|root| root == name)
        }) {
            return true;
        }
        // A loop's own variables: `foreach {k v} $pairs …` binds `k` and `v`.
        //
        // Split with the dialect's own list grammar, as the SSA
        // loop-variable harvester does, not on whitespace:
        // `foreach {{first last}} …` binds one variable whose name contains a
        // space, and a whitespace split reads it as the two fragments
        // `{first` and `last}` and finds neither.
        if ownership != Ownership::ScopeAliases
            && at(ArgRole::LoopVarList)
                .into_iter()
                .filter_map(|index| args.get(index))
                .filter_map(|list| list_rules.split_list(list).ok())
                .flatten()
                .any(|word| {
                    tcl_syntax::naming::split_element_ref(word).map_or(word, |(root, _)| root)
                        == name
                })
        {
            return true;
        }
        // Recurse only into a body that runs in *this* script's own frame.
        //
        // `plain_body_arg_indices` is the registry's generic answer to "is a
        // dispatch nested in this body argument still the same context as the
        // caller": it keeps `if` / `while` / `foreach` / `catch` / `eval` and
        // drops every `BodyKind::Structural` body. That distinction is the
        // whole question here. `proc p {} {set x 1}; puts $x` binds `x` in
        // *p's* frame, so the `$x` beside it is still a read of the enclosing
        // one and must keep its W210 — descending into `proc`'s body would
        // silence a real finding. `uplevel`, `namespace eval` and the `oo::`
        // definers are excluded for the same reason.
        //
        // `ArgRole::OpaqueScript` goes with it: the registry's contract is
        // that such a word is not executed here at all.
        if registry
            .plain_body_arg_indices(command, &args)
            .into_iter()
            .filter_map(|index| literal_body_image(script, &segment, index + 1, config))
            .any(|body| binds(&body, name, ownership, registry, config, depth - 1))
        {
            return true;
        }
    }
    false
}

fn decoded_binding_root(
    image: &tcl_lexer::SourceImage,
    segment: &crate::segmenter::SegmentedCommand,
    word: usize,
    config: tcl_lexer::LexerConfig,
) -> Option<String> {
    let tokens = crate::ir::CommandTokens::from_segmented(&image.source_map(), config, segment);
    match crate::registry_invocation::effective_invocation_word(
        tokens.words().get(word)?,
        config.escapes,
        tcl_syntax::word_rules::WordValueRules::from_config(&config),
    ) {
        crate::registry_invocation::EffectiveInvocationWord::Literal(value) => Some(
            tcl_syntax::naming::normalise_var_name_braced_for_style(
                &value,
                true,
                config.braced_var,
            )
            .to_owned(),
        ),
        crate::registry_invocation::EffectiveInvocationWord::ArrayElementName { root } => {
            Some(root)
        }
        _ => None,
    }
}

/// Original straight-line declaration layout available to missing-read advice.
/// It excludes unknown heads, nested script evaluation and unavailable operand
/// roles. This is a syntax-domain receipt, not a completion or effect proof.
#[cfg(test)]
pub(crate) struct LinearDeclarationReadLayout;

#[cfg(test)]
pub(crate) fn linear_declaration_read_layout(
    image: &tcl_lexer::SourceImage,
    registry: &CommandRegistry,
    config: tcl_lexer::LexerConfig,
    dialect: tcl_registry::InvocationDialect,
) -> Option<LinearDeclarationReadLayout> {
    let segments =
        crate::segmenter::segment_commands_image_with_offset_and_config(image, 0, config)?;
    for segment in segments {
        if segment.is_partial {
            return None;
        }
        let tokens =
            crate::ir::CommandTokens::from_segmented(&image.source_map(), config, &segment);
        if !tokens
            .words()
            .iter()
            .all(|word| linear_operand(word, config))
        {
            return None;
        }
        let values: Vec<_> = tokens
            .words()
            .iter()
            .map(|word| {
                crate::registry_invocation::effective_invocation_word(
                    word,
                    config.escapes,
                    dialect.word_values,
                )
            })
            .collect();
        let words: Vec<_> = values
            .iter()
            .map(crate::registry_invocation::EffectiveInvocationWord::as_registry_word)
            .collect();
        let (head, arguments) = words.split_first()?;
        let resolved = registry.resolve_structured_invocation(
            tcl_registry::InvocationWords::structured(*head, arguments).with_dialect(dialect),
            dialect.authoring_query(),
        );
        let facts = resolved.resolved()?.facts();
        if !facts.arg_roles_complete
            || facts.arity_accepts_frozen_arguments() != Some(true)
            || !matches!(
                tcl_registry::case_bodies::script_body_flow_in_registry(
                    registry,
                    &facts,
                    tcl_registry::InvocationWords::structured(*head, arguments)
                        .with_dialect(dialect)
                        .arguments()
                ),
                tcl_registry::script_body_flow::ScriptBodyFlow::None
            )
            || facts
                .arg_roles
                .iter()
                .any(|(_, role)| role.carries_script() || *role == ArgRole::OpaqueScript)
        {
            return None;
        }
    }
    Some(LinearDeclarationReadLayout)
}

#[cfg(test)]
fn linear_operand(word: &crate::ir::WordExpr, config: tcl_lexer::LexerConfig) -> bool {
    use crate::ir::{WordExpr, WordPart};
    let variable = |spelling: &str| {
        tcl_lexer::word_parts::whole_var_ref(spelling.as_bytes(), config)
            .ok()
            .flatten()
            .is_some_and(|reference| reference.index.is_none())
    };
    match word {
        WordExpr::Literal { .. } | WordExpr::BracedLiteral { .. } => true,
        WordExpr::Variable { spelling, .. } => variable(spelling),
        WordExpr::Template { parts, .. } => parts.iter().all(|part| match part {
            WordPart::Text { .. } => true,
            WordPart::Variable { spelling, .. } => variable(spelling),
            WordPart::CommandSubstitution { .. } | WordPart::Opaque { .. } => false,
        }),
        WordExpr::CommandSubstitution { .. }
        | WordExpr::Expand { .. }
        | WordExpr::Opaque { .. } => false,
    }
}

fn literal_body_image(
    image: &tcl_lexer::SourceImage,
    segment: &crate::segmenter::SegmentedCommand,
    word: usize,
    config: tcl_lexer::LexerConfig,
) -> Option<tcl_lexer::SourceImage> {
    let tokens = crate::ir::CommandTokens::from_segmented(&image.source_map(), config, segment);
    let value = crate::registry_invocation::effective_invocation_word(
        tokens.words().get(word)?,
        config.escapes,
        tcl_syntax::word_rules::WordValueRules::from_config(&config),
    );
    let crate::registry_invocation::EffectiveInvocationWord::Literal(value) = value else {
        return None;
    };
    Some(tcl_lexer::SourceImage::native(value.into_bytes()))
}

/// An unchanged authored procedure's potential local-read domain. This is
/// existential diagnostic advice, including a declaration whose runtime
/// publication fails; it supplies no activation, physical absence or dispatch.
pub(crate) struct AuthoredProcedureReadAdvice {
    body: tcl_lexer::Span,
}

impl AuthoredProcedureReadAdvice {
    /// Original diagnostic read span, without a physical contents verdict.
    pub(crate) fn owns(&self, span: tcl_lexer::Span) -> bool {
        span.start() >= self.body.start() && span.end() <= self.body.end()
    }
}

/// Validate the original declaration/body/formal geometry using shared word
/// and parameter owners. A materialised, substituted or malformed declaration
/// cannot donate an authored local frame to this diagnostic projection.
pub(crate) fn authored_procedure_read_advice(
    image: &tcl_lexer::SourceImage,
    procedure: &crate::ir::Procedure,
    analysis: &crate::analyser::AnalysisResult,
    context: &tcl_registry::model::ContextRegistry,
) -> Option<AuthoredProcedureReadAdvice> {
    let config = analysis.body_lexer_config?;
    let input = analysis.resolved_input.as_ref()?;
    let realm = analysis.retained_command_realm()?;
    if !realm.matches_resolved_analysis_input(input)
        || !realm.matches_original_source_image(image, config)
        || input.lexer_config() != config
    {
        return None;
    }
    let binding = realm.invocation_at_source("", procedure.span.start());
    let (_, tokens) = binding.original_recorded_command()?;
    let recipe = realm
        .source_bindings_ref()
        .original_procedure_read_declaration(&tokens, context.commands(), input)?;
    let source = recipe.source();
    let body = procedure.body_source.as_ref()?;
    if source.origin.source_image() != image
        || source.base() != procedure.body_offset
        || source.text.try_text().ok()? != body
        || recipe.parameters_text()? != procedure.params_raw
        || recipe.parameter_names() != procedure.params
    {
        return None;
    }
    let base = usize::try_from(source.base()).ok()?;
    if image.bytes().get(base..base.checked_add(body.len())?) != Some(body.as_bytes()) {
        return None;
    }
    Some(AuthoredProcedureReadAdvice {
        body: tcl_lexer::Span::new(
            source.base(),
            source.base().checked_add(u32::try_from(body.len()).ok()?)?,
        ),
    })
}

#[cfg(test)]
mod tests {
    use super::{Ownership, script_binds_name};

    fn binds(script: &str, name: &str) -> bool {
        script_binds_name(
            script,
            name,
            Ownership::Bindings,
            tcl_registry::default_registry(),
            tcl_lexer::LexerConfig::default(),
        )
    }

    fn owns(script: &str, name: &str) -> bool {
        script_binds_name(
            script,
            name,
            Ownership::BindingsOrNameReads,
            tcl_registry::default_registry(),
            tcl_lexer::LexerConfig::default(),
        )
    }

    #[test]
    fn decoded_binding_names_do_not_strip_reference_sigils() {
        let registry = tcl_registry::default_registry();
        let config = tcl_lexer::LexerConfig::default();
        for (script, name, expected) in [
            ("set {$n} 1; return ${$n}", "$n", true),
            ("set {$n} 1; return $n", "n", false),
            ("set $n 1", "n", false),
            ("set map($key) 1", "map", true),
            ("foreach {{$n}} $items {}", "$n", true),
        ] {
            assert_eq!(
                super::script_image_binds_name(
                    &tcl_lexer::SourceImage::native(script.as_bytes()),
                    name,
                    Ownership::DecodedBindings,
                    registry,
                    config,
                ),
                expected,
                "{script}: {name}"
            );
        }
    }

    #[test]
    fn linear_read_layout_declines_unmodelled_completion_and_nested_effects() {
        let registry = tcl_registry::model::ingress::static_context_for("tcl8.6").commands();
        let dialect = tcl_registry::InvocationDialect::of_profile(registry.profile().unwrap());
        let config = tcl_lexer::LexerConfig::from_grammar(dialect.lexer_grammar);
        for (script, expected) in [
            ("namespace upvar ::ns a alias; return $missing", true),
            ("set {$n} 1; return ${$n}", true),
            ("if {[catch {operation} err]} {puts $other}", false),
            (
                "switch $x {a {return} default {error stop}}; puts $missing",
                false,
            ),
            ("operation; puts $missing", false),
            ("puts $a([operation])", false),
        ] {
            assert_eq!(
                super::linear_declaration_read_layout(
                    &tcl_lexer::SourceImage::native(script.as_bytes()),
                    registry,
                    config,
                    dialect,
                )
                .is_some(),
                expected,
                "{script}"
            );
        }
    }

    /// The shape the old top-level-`set` reading already answered, kept so a
    /// rewrite cannot lose it.
    #[test]
    fn a_top_level_set_still_binds() {
        assert!(binds("set a 1\nputs $a", "a"));
        assert!(!binds("puts $a", "a"));
    }

    /// The #2117 body: three names, three spellings, none of them a
    /// top-level `set`.
    #[test]
    fn a_nested_set_a_loop_variable_and_a_catch_operand_all_bind() {
        let body = "foreach it $items { set last $it }\ncatch {risky} err\nlist $last $err $it";
        for name in ["it", "last", "err"] {
            assert!(binds(body, name), "`{name}` is bound by this body");
        }
        assert!(!binds(body, "items"), "`items` is only read");
    }

    /// Depth, in both directions: a write far enough down is not found, which
    /// is the conservative answer rather than a wrong one.
    #[test]
    fn nesting_is_followed_to_a_bound() {
        assert!(binds("if {1} { while {1} { set deep 1 } }", "deep"));
        let too_deep = "if {1} { if {1} { if {1} { if {1} { if {1} { if {1} { set x 1 } } } } } }";
        assert!(!binds(too_deep, "x"));
    }

    /// An array element write binds the array, as `normalise_var_name` reads
    /// it everywhere else.
    #[test]
    fn an_array_element_write_binds_the_array() {
        assert!(binds("foreach k $ks { set map($k) 1 }\nparray map", "map"));
    }

    /// A definition body binds in the frame it will later run in, not in the
    /// script that defines it — so the `$x` beside the `proc` is still a read
    /// of the enclosing frame and must keep its finding. Descending into a
    /// `BodyKind::Structural` body silenced it.
    #[test]
    fn a_definition_body_does_not_bind_in_the_defining_script() {
        assert!(!binds("proc p {} {set x 1}\nputs $x", "x"));
        assert!(!binds("namespace eval ns { set y 1 }\nputs $y", "y"));
        assert!(!binds("uplevel 1 { set z 1 }\nputs $z", "z"));
        // The same-frame bodies next to them still do bind.
        assert!(binds("if {1} { set a 1 }\nputs $a", "a"));
        assert!(binds("catch { set b 1 }\nputs $b", "b"));
    }

    /// A `foreach` variable list is a Tcl list, so one element may itself
    /// contain whitespace. Splitting the word on spaces found neither
    /// fragment.
    #[test]
    fn a_loop_variable_list_is_split_as_a_tcl_list() {
        assert!(binds("foreach {{first last}} $rows { }", "first last"));
        assert!(!binds("foreach {{first last}} $rows { }", "first"));
        // The ordinary spelling is unchanged.
        assert!(binds("foreach {k v} $pairs { }", "k"));
        assert!(binds("foreach {k v} $pairs { }", "v"));
    }

    /// A one-argument `set` reads its operand rather than binding it, so it
    /// is a binding for nobody — but it *names* the variable, which is what
    /// an opaque barrier body (`interp eval i {set y} 7`) needs: the name
    /// belongs to the child interpreter, not to this frame.
    #[test]
    fn a_bare_name_read_is_ownership_only_for_a_barrier_body() {
        assert!(!binds("set y", "y"));
        assert!(owns("set y", "y"));
    }
    #[test]
    fn original_source_channel_is_retained_before_body_value_evaluation() {
        let raw = b"set \\\r\n x 1";
        let registry = tcl_registry::default_registry();
        let config = tcl_lexer::LexerConfig::default();
        assert!(super::script_image_binds_name(
            &tcl_lexer::SourceImage::from_bytes(raw.as_slice(), tcl_lexer::SourceChannel::Document),
            "x",
            Ownership::Bindings,
            registry,
            config,
        ));
        assert!(!super::script_image_binds_name(
            &tcl_lexer::SourceImage::native(raw.as_slice()),
            "x",
            Ownership::Bindings,
            registry,
            config,
        ));
        assert!(!super::script_image_binds_name(
            &tcl_lexer::SourceImage::native(b"set \xff 1".as_slice()),
            "x",
            Ownership::Bindings,
            registry,
            config,
        ));
    }
    #[test]
    fn declaration_read_advice_requires_original_unchanged_body_and_formals() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Authored read applicability, no installed procedure or entered frame.
        let source = "proc p {arg} {puts $missing}";
        let analysis = crate::analyser::Analyser::new().analyse(source, "tcl");
        let input = analysis.resolved_input.as_ref().unwrap();
        let context = input.context_registry();
        let unit = crate::compilation_unit::CompilationUnit::build_with_analysis_input(
            source,
            crate::compilation_unit::UnitBuildOptions {
                registry: context.commands(),
                defer_top_level: false,
                config: input.lexer_config(),
                dialect: Some(input.unit_profile()),
                external_call_sites: None,
                declared_commands: None,
            },
            None,
            input,
        );
        let procedure = &unit.ir_module.procedures["::p"];
        let image = tcl_lexer::SourceImage::document(source);
        let advice =
            super::authored_procedure_read_advice(&image, procedure, &analysis, &context).unwrap();
        assert!(advice.owns(tcl_lexer::Span::new(
            procedure.body_offset,
            procedure.body_offset + 4
        )));
        assert!(!advice.owns(procedure.span));
        let mut changed = procedure.clone();
        changed.params.push("invented".into());
        assert!(
            super::authored_procedure_read_advice(&image, &changed, &analysis, &context).is_none()
        );
        assert!(
            super::authored_procedure_read_advice(
                &tcl_lexer::SourceImage::document("proc p {arg} {puts $changed}"),
                procedure,
                &analysis,
                &context,
            )
            .is_none()
        );
        let mut missing = analysis.clone();
        missing.resolved_input = None;
        assert!(
            super::authored_procedure_read_advice(&image, procedure, &missing, &context).is_none()
        );
        let foreign =
            tcl_registry::model::ingress::resolve_environment("tcl9.0").default_context_registry();
        assert!(
            super::authored_procedure_read_advice(&image, procedure, &analysis, &foreign).is_none()
        );
    }

    #[test]
    fn selected_formal_read_scope_keeps_native_dialects_and_failed_publication_separate() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Original header topology only; no Native body entry or call outcome.
        for (dialect, formals) in [
            ("tcl8.4", "arg"),
            ("tcl8.5", "arg"),
            ("tcl8.6", "arg"),
            ("tcl9.0", "arg"),
            ("tcl9.1", "arg"),
            ("jim", "&link"),
        ] {
            let source = format!("proc ::missing::p {{{formals}}} {{puts $missing}}");
            let profile = tcl_dialect::DialectProfile::find(dialect).unwrap();
            let (_owner, captured) =
                crate::environment_ingress::captured_native_entry_with_owner(profile);
            let entry = crate::command_binding::SourceAnalysisEntry {
                native_entry: Some(std::sync::Arc::new(captured)),
                invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
                ..Default::default()
            };
            let analysis = crate::analyser::Analyser::new()
                .with_source_analysis_entry(std::sync::Arc::new(entry))
                .analyse(&source, dialect);
            let input = analysis.resolved_input.as_ref().unwrap();
            let context = input.context_registry();
            let realm = analysis.retained_command_realm().unwrap();
            let binding = realm.invocation_at_source("", 0);
            let (_, tokens) = binding.original_recorded_command().unwrap();
            let recipe = realm
                .source_bindings_ref()
                .original_procedure_read_declaration(&tokens, context.commands(), input)
                .unwrap_or_else(|| panic!("selected {dialect} formal source recipe"));
            assert_eq!(recipe.parameters_text(), Some(formals));
            assert_eq!(recipe.source().text.try_text().unwrap(), "puts $missing");
            assert_eq!(
                recipe.parameter_names(),
                if dialect == "jim" {
                    vec!["&link"]
                } else {
                    vec!["arg"]
                }
            );
            let missing = crate::analyser::Analyser::new().analyse(&source, dialect);
            let absent = missing
                .retained_command_realm()
                .unwrap()
                .invocation_at_source("", 0);
            if let Some((_, tokens)) = absent.original_recorded_command() {
                assert!(
                    missing
                        .retained_command_realm()
                        .unwrap()
                        .source_bindings_ref()
                        .original_procedure_read_declaration(&tokens, context.commands(), input)
                        .is_none()
                );
            }
        }
    }

    fn native_read_scope(source: &str, declaration: &str) -> bool {
        let profile = tcl_dialect::DialectProfile::find("tcl9.0").unwrap();
        let (_owner, captured) =
            crate::environment_ingress::captured_native_entry_with_owner(profile);
        let entry = crate::command_binding::SourceAnalysisEntry {
            native_entry: Some(std::sync::Arc::new(captured)),
            invocation_dialect: Some(tcl_registry::InvocationDialect::of_profile(profile)),
            ..Default::default()
        };
        let analysis = crate::analyser::Analyser::new()
            .with_source_analysis_entry(std::sync::Arc::new(entry))
            .analyse(source, profile.name);
        let input = analysis.resolved_input.as_ref().unwrap();
        let context = input.context_registry();
        let realm = analysis.retained_command_realm().unwrap();
        let binding = realm.invocation_at_source(
            "",
            u32::try_from(source.rfind(declaration).unwrap()).unwrap(),
        );
        let Some((_, tokens)) = binding.original_recorded_command() else {
            return false;
        };
        realm
            .source_bindings_ref()
            .original_procedure_read_declaration(&tokens, context.commands(), input)
            .is_some()
    }

    #[test]
    fn selected_formal_read_scope_follows_original_factory_aliases_and_replacements() {
        // naming.compiler.original-analysis-metadata-context
        // docs/design/analysis/name-resolution-proofs/original-analysis-metadata-context.md
        // Read applicability from original declaration operands, no installation.
        for (source, declaration, expected) in [
            (
                "rename proc stock; stock p {arg} {puts $missing}",
                "stock p",
                true,
            ),
            (
                "interp alias {} define {} proc p; define {arg} {puts $missing}",
                "define {arg}",
                true,
            ),
            (
                "interp alias {} define {} proc p {arg}; define {puts $missing}",
                "define {puts",
                true,
            ),
            (
                "proc proc {name params body} {}; proc p {arg} {puts $missing}",
                "proc p",
                false,
            ),
            (
                "rename proc {}; proc p {arg} {puts $missing}",
                "proc p",
                false,
            ),
        ] {
            assert_eq!(native_read_scope(source, declaration), expected, "{source}");
        }
    }
}
