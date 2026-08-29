// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook) <https://github.com/bitwisecook>
//
// SPDX-License-Identifier: AGPL-3.0-or-later

//! A conservative, registry-driven static model of a Tk widget tree.
//!
//! This module deliberately models only source facts that can be recovered
//! without running Tcl.  In particular, a variable or command substitution in
//! a widget path is an uncertainty, never a guessed widget.  Command identity,
//! constructor shape, geometry-manager membership and nested executable bodies
//! all come from the shared registry and executable-region walker.

use tcl_dialect::model::{SurfaceQuery};
use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use tcl_compiler::realm::document_realm_bindings;
use tcl_compiler::registry_invocation::segmented_command_arguments;
use tcl_compiler::segmenter::SegmentedCommand;
use tcl_lexer::{LexerConfig, Span, TokenType};
use tcl_registry::hooks::AnalyserHookId;
use tcl_registry::hover::OptionSpec;
use tcl_registry::spec::resolve_option_prefix;
use tcl_registry::tk_geometry::TkGeometryContainerPolicy;
use tcl_registry::{CommandRegistry, CommandSpec, InvocationWord, InvocationWords, Traits};

use crate::executable_regions::{ExecutableContext, visit_executable_commands};
use tcl_dialect::model::{SpecSurface};

/// The current JSON-compatible Tk UI model schema version.
pub const TK_UI_SCHEMA_VERSION: u32 = 1;

/// Maximum number of individual abstentions retained in one UI-model
/// response. Large generated UIs can contain thousands of equivalent dynamic
/// paths; retaining a deterministic prefix keeps the response useful without
/// allowing uncertainty detail to dominate transport or editor rendering.
pub const MAX_TK_UI_UNCERTAINTIES: usize = 200;

/// Maximum number of statically constructed widgets retained in one model,
/// excluding the implicit root. This bounds the serialized response and the
/// editor's recursive DOM work for generated or otherwise untrusted source.
pub const MAX_TK_UI_WIDGETS: usize = 1_000;

/// Maximum lexical widget-path nesting retained by the recursive model and
/// editor renderers. Tk permits deeply nested windows, but an untrusted source
/// document must not be able to force unbounded recursion in an LSP/MCP host.
pub const MAX_TK_UI_PATH_DEPTH: usize = 256;

/// A source byte range, using a half-open `start..end` convention.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TkSourceSpan {
    /// First byte in the source range.
    pub start: u32,
    /// One byte past the source range.
    pub end: u32,
}

impl From<Span> for TkSourceSpan {
    fn from(span: Span) -> Self {
        Self {
            start: span.start(),
            end: span.end(),
        }
    }
}

/// A statically known option value and where it was written.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TkLiteralOption {
    /// The Tcl value after lexical unquoting.
    pub value: String,
    /// The source word that supplied `value`.
    pub source: TkSourceSpan,
}

/// The source locations that establish a widget fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TkWidgetSource {
    /// The complete constructor command.
    pub command: TkSourceSpan,
    /// The widget-path argument.
    pub path: TkSourceSpan,
}

/// A widget in the static Tk hierarchy.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TkWidget {
    /// Canonical Tcl widget path, such as `.toolbar.save`.
    pub path: String,
    /// Registry-resolved constructor identity, such as `ttk::button`.
    pub constructor: String,
    /// Literal constructor options, keyed in deterministic lexical order.
    pub options: BTreeMap<String, TkLiteralOption>,
    /// The direct geometry-manager invocation, when one is statically known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry: Option<TkGeometryPlacement>,
    /// Children whose lexical widget path makes this widget their parent.
    pub children: Vec<TkWidget>,
    /// The source that created this widget.
    pub source: TkWidgetSource,
    /// Whether this constructor is direct or appears in a potentially executed
    /// body such as a procedure, branch, callback, or lambda.
    pub certainty: TkFactCertainty,
}

/// Static execution certainty attached to model facts.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TkFactCertainty {
    /// The command is in the document's direct execution region.
    Certain,
    /// The command is in a body whose execution depends on runtime flow.
    Potential,
}

/// A direct registry-declared geometry-manager placement.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TkGeometryPlacement {
    /// Registry-resolved geometry-manager identity.
    pub manager: String,
    /// Literal manager options, keyed in deterministic lexical order.
    pub options: BTreeMap<String, TkLiteralOption>,
    /// The complete geometry-manager command.
    pub source: TkSourceSpan,
    /// The source widget-path argument claimed by this manager.
    pub target: TkSourceSpan,
    /// Effective geometry container after applying the registry-declared
    /// container option, or `None` when that option was dynamic/invalid.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container: Option<String>,
    /// Whether this placement is direct or appears in a potentially executed
    /// body.
    pub certainty: TkFactCertainty,
}

/// Why a source construct was deliberately not made into a static fact.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TkUncertaintyKind {
    /// A constructor's declared path argument was dynamic or invalid.
    DynamicWidgetPath,
    /// A geometry manager's first argument was dynamic or not a widget path.
    DynamicGeometryTarget,
    /// A geometry manager's effective container was dynamic or invalid.
    DynamicGeometryContainer,
    /// A geometry manager used a form other than its direct target form.
    UnsupportedGeometryForm,
    /// A source command was syntactically partial and cannot be trusted.
    PartialCommand,
    /// A static widget named a parent not constructed in this source model.
    UnknownWidgetParent,
    /// A direct manager targeted a widget not constructed in this source model.
    UnknownGeometryTarget,
    /// A literal `-in` container was not constructed in this source model.
    UnknownGeometryContainer,
    /// Multiple constructors named the same static widget path.
    DuplicateWidgetPath,
    /// An option's value or shape could not be represented as a literal.
    NonLiteralOption,
    /// A fact occurs inside a body whose execution is not statically proven.
    PotentialExecution,
    /// A later widget instance operation changes state the static model does
    /// not yet fold into the constructor fact.
    PostConstructorMutation,
    /// A destroy/rename-like operation changes a previously created widget's
    /// lifetime or command identity.
    WidgetLifecycleMutation,
    /// More static widget constructors were found than the bounded model can
    /// safely retain and render.
    WidgetLimitReached,
}

/// An explicit abstention made while modelling the source.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TkUiUncertainty {
    /// Classification of the unavailable fact.
    pub kind: TkUncertaintyKind,
    /// The command or word responsible for the abstention.
    pub source: TkSourceSpan,
    /// A concise, user-facing explanation.
    pub message: String,
}

/// A container claimed by more than one direct geometry manager.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TkGeometryConflict {
    /// The effective geometry container, including a literal registry-declared
    /// `-in` override when present.
    pub container: String,
    /// Distinct registry-resolved managers, in deterministic order.
    pub managers: Vec<String>,
    /// All manager calls participating in the conflict.
    pub placements: Vec<TkGeometryConflictPlacement>,
}

/// One source placement contributing to a geometry conflict.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TkGeometryConflictPlacement {
    /// The widget claimed by this manager.
    pub widget: String,
    /// Registry-resolved geometry-manager identity.
    pub manager: String,
    /// The complete manager command's source range.
    pub source: TkSourceSpan,
}

/// Versioned static Tk UI data for one source document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TkUiModel {
    /// Version of this model's serialized schema.
    pub schema_version: u32,
    /// Whether Tcl's Tk package is statically active for this document.
    pub tk_active: bool,
    /// The implicit Tk root widget (`.`), present only when [`Self::tk_active`]
    /// is true.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<TkWidget>,
    /// Number of widgets including the implicit root when Tk is active, or zero
    /// when it is not active. This includes statically recognised widgets
    /// omitted by [`MAX_TK_UI_WIDGETS`].
    pub widget_count: usize,
    /// Static widgets omitted after [`MAX_TK_UI_WIDGETS`] were retained.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub widgets_truncated: usize,
    /// Widgets whose static parent was not constructed in this source.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub orphan_widgets: Vec<TkWidget>,
    /// Facts that are dynamic, malformed, or otherwise intentionally absent.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub uncertainties: Vec<TkUiUncertainty>,
    /// Additional uncertainty records omitted after
    /// [`MAX_TK_UI_UNCERTAINTIES`] were retained.
    #[serde(default, skip_serializing_if = "is_zero")]
    pub uncertainties_truncated: usize,
    /// Mixed-manager claims grouped by lexical container.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub geometry_conflicts: Vec<TkGeometryConflict>,
    /// URI supplied by a host after analysis, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_uri: Option<String>,
    /// Document version supplied by a host after analysis, if any.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_version: Option<i32>,
    /// SHA-256 of the document text supplied by a host after analysis, if any.
    ///
    /// Some LSP clients do not expose the version they assigned to an open
    /// document. Those clients use this opaque snapshot fingerprint to reject
    /// a response produced from a different server-side text snapshot.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub document_sha256: Option<String>,
}

#[derive(Debug, Clone)]
struct CollectedWidget {
    constructor: String,
    options: BTreeMap<String, TkLiteralOption>,
    source: TkWidgetSource,
    geometry: Option<TkGeometryPlacement>,
    certainty: TkFactCertainty,
}

#[derive(Debug, Clone)]
struct CollectedPlacement {
    widget: String,
    placement: TkGeometryPlacement,
    container_policy: TkGeometryContainerPolicy,
}

#[derive(Debug, Clone)]
struct PendingWidgetCall {
    method: String,
    arguments: Vec<PendingInvocationWord>,
    source: TkSourceSpan,
}

#[derive(Debug, Clone)]
enum PendingInvocationWord {
    Literal(String),
    Dynamic,
    DynamicNonOption,
    Expanded,
    Opaque,
}

impl PendingInvocationWord {
    fn from_registry(word: InvocationWord<'_>) -> Self {
        match word {
            InvocationWord::Literal(value) => Self::Literal(value.to_owned()),
            InvocationWord::Dynamic => Self::Dynamic,
            InvocationWord::DynamicNonOption => Self::DynamicNonOption,
            InvocationWord::Expanded => Self::Expanded,
            InvocationWord::Opaque => Self::Opaque,
        }
    }

    fn as_registry(&self) -> InvocationWord<'_> {
        match self {
            Self::Literal(value) => InvocationWord::Literal(value),
            Self::Dynamic => InvocationWord::Dynamic,
            Self::DynamicNonOption => InvocationWord::DynamicNonOption,
            Self::Expanded => InvocationWord::Expanded,
            Self::Opaque => InvocationWord::Opaque,
        }
    }
}

#[derive(Default)]
struct TkAnalysis {
    widgets: BTreeMap<String, CollectedWidget>,
    widgets_truncated: usize,
    active_placements: BTreeMap<String, CollectedPlacement>,
    geometry_conflicts: Vec<TkGeometryConflict>,
    pending_widget_calls: BTreeMap<String, Vec<PendingWidgetCall>>,
    pending_lifecycle: BTreeMap<String, Vec<TkSourceSpan>>,
    uncertainties: Vec<TkUiUncertainty>,
}

/// Analyse `source` into a conservative, static Tk UI model.
///
/// The caller supplies the resolved document `dialect` and its `registry` so
/// this function never guesses a command surface.  The returned root is
/// implicit; the model includes only constructor facts whose registry spec has
/// both `creates_instance_at` and `required_package == "Tk"`.
#[must_use]
pub fn analyse_tk_ui(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
) -> TkUiModel {
    let identities = document_realm_bindings(source, dialect, registry);
    let config = LexerConfig::for_file_grammar(dialect.grammar);
    let tk_active = crate::document_context_for_profile(dialect)
        .authoring_query()
        .packages
        .contains(&"Tk")
        || source_requires_tk(source, config, dialect, registry, &identities);
    if !tk_active {
        return TkUiModel {
            schema_version: TK_UI_SCHEMA_VERSION,
            tk_active: false,
            root: None,
            widget_count: 0,
            widgets_truncated: 0,
            orphan_widgets: Vec::new(),
            uncertainties: Vec::new(),
            uncertainties_truncated: 0,
            geometry_conflicts: Vec::new(),
            document_uri: None,
            document_version: None,
            document_sha256: None,
        };
    }
    let mut analysis = TkAnalysis::default();
    let tk_version = crate::document_context_for_profile(dialect)
        .placement_floor("Tk")
        .map(tcl_dialect::model::Version::as_str);

    visit_executable_commands(
        source,
        config,
        registry,
        Some(crate::document_context_for_profile(dialect).authoring_query()),
        &identities,
        &mut |command, heads, context| {
            collect_tk_command(
                command,
                heads.resolved,
                registry,
                Some(crate::document_context_for_profile(dialect).authoring_query()),
                tk_version,
                &mut analysis,
                context,
            );
            false
        },
    );

    finish_tk_analysis(analysis)
}

fn finish_tk_analysis(analysis: TkAnalysis) -> TkUiModel {
    let TkAnalysis {
        mut widgets,
        widgets_truncated,
        active_placements,
        geometry_conflicts,
        pending_widget_calls: _,
        pending_lifecycle: _,
        mut uncertainties,
    } = analysis;

    for placement in active_placements.values() {
        if let Some(container) = placement.placement.container.as_deref()
            && container != "."
            && !widgets.contains_key(container)
        {
            uncertainties.push(uncertainty(
                TkUncertaintyKind::UnknownGeometryContainer,
                placement.placement.source,
                format!(
                    "Geometry manager '{}' names an -in container '{container}' that is not statically constructed.",
                    placement.placement.manager
                ),
            ));
        }
    }

    for collected in active_placements.into_values() {
        if let Some(widget) = widgets.get_mut(&collected.widget) {
            if collected.placement.source.start < widget.source.command.start {
                uncertainties.push(uncertainty(
                    TkUncertaintyKind::WidgetLifecycleMutation,
                    collected.placement.source,
                    format!(
                        "Geometry evidence for '{}' predates the retained widget instance and is not applied to it.",
                        collected.widget
                    ),
                ));
                continue;
            }
            let mut placement = collected.placement;
            // A replacement/otherwise uncertain widget cannot acquire a
            // verified layout merely because its manager command itself
            // appears in a direct command region.
            if widget.certainty == TkFactCertainty::Potential {
                placement.certainty = TkFactCertainty::Potential;
            }
            widget.geometry = Some(placement);
        } else {
            uncertainties.push(uncertainty(
                TkUncertaintyKind::UnknownGeometryTarget,
                collected.placement.target,
                "The geometry manager targets no statically constructed widget.",
            ));
        }
    }

    let widget_count = widgets.len() + widgets_truncated + 1;
    let (root, orphan_widgets) = build_hierarchy(&widgets, &mut uncertainties);
    // Hierarchy construction can add unknown-parent abstentions, so cap only
    // after every analysis phase has contributed its records.  Capping before
    // this point would let orphan-heavy sources exceed the transport bound and
    // would under-report the omitted count.
    let uncertainties_truncated = cap_uncertainties(&mut uncertainties);
    TkUiModel {
        schema_version: TK_UI_SCHEMA_VERSION,
        tk_active: true,
        root: Some(root),
        widget_count,
        widgets_truncated,
        orphan_widgets,
        uncertainties,
        uncertainties_truncated,
        geometry_conflicts,
        document_uri: None,
        document_version: None,
        document_sha256: None,
    }
}

fn collect_tk_command(
    command: &SegmentedCommand,
    resolved_head: &str,
    registry: &CommandRegistry,
    dialect: Option<SurfaceQuery<'_>>,
    tk_version: Option<&str>,
    analysis: &mut TkAnalysis,
    context: ExecutableContext,
) {
    collect_known_widget_mutation(
        command,
        resolved_head,
        registry,
        dialect,
        &mut analysis.widgets,
        &mut analysis.pending_widget_calls,
        &mut analysis.uncertainties,
        context,
    );
    let Some(spec) = registry.get(resolved_head) else {
        return;
    };
    if command.is_partial {
        analysis.uncertainties.push(uncertainty(
            TkUncertaintyKind::PartialCommand,
            command.span,
            "The command is syntactically incomplete.",
        ));
        return;
    }
    if !spec.available_for_version(tk_version) {
        return;
    }
    if spec.creates_instance_at.is_some() && spec.required_package == Some("Tk") {
        collect_widget(
            command,
            resolved_head,
            spec,
            dialect,
            tk_version,
            &mut analysis.widgets,
            &mut analysis.widgets_truncated,
            &mut analysis.uncertainties,
            context,
        );
        apply_pending_widget_calls(command, spec, registry, dialect, analysis);
        apply_pending_lifecycle(command, spec, analysis);
    }
    if spec.traits.contains(Traits::TK_GEOMETRY_MANAGER) {
        collect_placement(
            command,
            resolved_head,
            spec,
            dialect,
            tk_version,
            &analysis.widgets,
            &mut analysis.active_placements,
            &mut analysis.geometry_conflicts,
            &mut analysis.uncertainties,
            context,
        );
    }
    collect_registry_lifecycle_effect(command, resolved_head, spec, registry, analysis, context);
}

// Serde's `skip_serializing_if` callback receives a shared reference even for
// `Copy` scalars, so this signature is constrained by the derive API.
#[allow(clippy::trivially_copy_pass_by_ref)]
fn is_zero(value: &usize) -> bool {
    *value == 0
}

fn cap_uncertainties(uncertainties: &mut Vec<TkUiUncertainty>) -> usize {
    let omitted = uncertainties.len().saturating_sub(MAX_TK_UI_UNCERTAINTIES);
    uncertainties.truncate(MAX_TK_UI_UNCERTAINTIES);
    omitted
}

fn source_requires_tk(
    source: &str,
    config: LexerConfig,
    dialect: &'static tcl_dialect::DialectProfile,
    registry: &CommandRegistry,
    identities: &tcl_compiler::realm::CommandBindingRealm,
) -> bool {
    let mut active = false;
    let available_tk = crate::document_context_for_profile(dialect)
        .placement_floor("Tk")
        .map(tcl_dialect::model::Version::as_str);
    visit_executable_commands(
        source,
        config,
        registry,
        Some(crate::document_context_for_profile(dialect).authoring_query()),
        identities,
        &mut |command, heads, context| {
            if context != ExecutableContext::Direct {
                return false;
            }
            let args: Vec<&str> = command.args().iter().map(String::as_str).collect();
            let exact = literal_word(command, 2).is_some_and(|(word, _)| word == "-exact");
            let package_index = if exact { 3 } else { 2 };
            active = registry
                .resolve_call(
                    heads.resolved,
                    &args,
                    Some(crate::document_context_for_profile(dialect).authoring_query()),
                )
                .is_some_and(|call| call.analyser_hook == Some(AnalyserHookId::PackageRequire))
                && literal_word(command, package_index).is_some_and(|(word, _)| word == "Tk")
                && tk_requirement_is_satisfied(command, package_index + 1, exact, available_tk);
            active
        },
    );
    active
}

fn tk_requirement_is_satisfied(
    command: &SegmentedCommand,
    requirement_start: usize,
    exact: bool,
    available_tk: Option<&str>,
) -> bool {
    let Some(available_tk) = available_tk else {
        return false;
    };
    let mut requirements = Vec::new();
    for index in requirement_start..command.texts.len() {
        let Some((requirement, _)) = literal_word(command, index) else {
            return false;
        };
        requirements.push(requirement);
    }
    if requirements.is_empty() {
        // `package require -exact Tk` is accepted by Tcl; without a version
        // there is no constraint for `-exact` to tighten.
        return true;
    }
    if exact {
        return requirements.len() == 1
            && tcl_dialect::version_satisfies(
                available_tk,
                &tcl_dialect::exact_requirement(&requirements[0]),
            );
    }
    requirements
        .iter()
        .any(|requirement| tcl_dialect::version_satisfies(available_tk, requirement))
}

// The collector carries one shared bounded model; grouping these mutable
// outputs would obscure which limit or fact each write affects.
#[allow(clippy::too_many_arguments)]
fn collect_widget(
    command: &SegmentedCommand,
    constructor: &str,
    spec: &CommandSpec,
    dialect: Option<SurfaceQuery<'_>>,
    tk_version: Option<&str>,
    widgets: &mut BTreeMap<String, CollectedWidget>,
    widgets_truncated: &mut usize,
    uncertainties: &mut Vec<TkUiUncertainty>,
    context: ExecutableContext,
) {
    let Some(path_index) = spec.creates_instance_at.map(usize::from) else {
        return;
    };
    let Some((path, path_span)) = literal_word(command, path_index + 1) else {
        uncertainties.push(uncertainty(
            TkUncertaintyKind::DynamicWidgetPath,
            command.span,
            "The Tk constructor's widget path is not a static literal.",
        ));
        return;
    };
    if !tcl_registry::tk_geometry::is_widget_path(&path)
        || path.split('.').skip(1).count() > MAX_TK_UI_PATH_DEPTH
    {
        uncertainties.push(uncertainty(
            TkUncertaintyKind::DynamicWidgetPath,
            path_span,
            "The Tk constructor's path is not a supported static widget path.",
        ));
        return;
    }
    if !widgets.contains_key(&path) && widgets.len() >= MAX_TK_UI_WIDGETS {
        *widgets_truncated += 1;
        if *widgets_truncated == 1 {
            uncertainties.push(uncertainty(
                TkUncertaintyKind::WidgetLimitReached,
                path_span,
                format!(
                    "The static Tk model retains at most {MAX_TK_UI_WIDGETS} widgets; additional constructor facts are counted but omitted."
                ),
            ));
        }
        return;
    }
    let option_specs = available_options(spec.options, dialect, spec.surface, tk_version);
    let options = literal_options(command, path_index + 2, &option_specs, uncertainties);
    let widget = CollectedWidget {
        constructor: constructor.to_owned(),
        options,
        source: TkWidgetSource {
            command: command.span.into(),
            path: path_span,
        },
        geometry: None,
        certainty: certainty_for_context(context),
    };
    record_potential_execution(context, command.span, "widget constructor", uncertainties);
    if widgets.insert(path.clone(), widget).is_some() {
        // A pathname can be destroyed and recreated, or a second constructor
        // can fail because the command already exists. Either way the final
        // runtime identity is not established by a path-only model. Do not
        // let a later geometry call be painted as a verified fact merely
        // because it happens to share this pathname.
        if let Some(replacement) = widgets.get_mut(&path) {
            replacement.certainty = TkFactCertainty::Potential;
        }
        uncertainties.push(uncertainty(
            TkUncertaintyKind::DuplicateWidgetPath,
            path_span,
            format!("More than one Tk constructor names the static path '{path}'."),
        ));
    }
}

#[allow(clippy::too_many_arguments)]
fn collect_known_widget_mutation(
    command: &SegmentedCommand,
    resolved_head: &str,
    registry: &CommandRegistry,
    dialect: Option<SurfaceQuery<'_>>,
    widgets: &mut BTreeMap<String, CollectedWidget>,
    pending_widget_calls: &mut BTreeMap<String, Vec<PendingWidgetCall>>,
    uncertainties: &mut Vec<TkUiUncertainty>,
    context: ExecutableContext,
) {
    let Some((method_name, _)) = literal_word(command, 1) else {
        return;
    };
    let argument_words = segmented_command_arguments(command);
    let Some(widget) = widgets.get(resolved_head) else {
        if context == ExecutableContext::PotentialBody
            && tcl_registry::tk_geometry::is_widget_path(resolved_head)
        {
            pending_widget_calls
                .entry(resolved_head.to_owned())
                .or_default()
                .push(PendingWidgetCall {
                    method: method_name,
                    arguments: argument_words
                        .iter()
                        .copied()
                        .map(PendingInvocationWord::from_registry)
                        .collect(),
                    source: command.span.into(),
                });
        }
        return;
    };
    let Some(constructor) = registry.get(&widget.constructor) else {
        return;
    };
    let Some(class) = constructor.object_class else {
        return;
    };
    let Some(invocation) = registry.resolve_structured_instance_invocation(
        class.class_name,
        InvocationWords::structured(InvocationWord::Literal(resolved_head), &argument_words),
        dialect,
    ) else {
        return;
    };
    if !invocation.semantics.mutator
        && !invocation
            .semantics
            .side_effects
            .iter()
            .any(|effect| effect.writes)
    {
        return;
    }
    let path = resolved_head.to_owned();
    if let Some(widget) = widgets.get_mut(&path) {
        widget.certainty = TkFactCertainty::Potential;
    }
    uncertainties.push(uncertainty(
        TkUncertaintyKind::PostConstructorMutation,
        command.span,
        format!(
            "Widget '{path}' is changed by instance method '{method_name}'; the static model retains its earlier constructor facts."
        ),
    ));
}

fn apply_pending_widget_calls(
    command: &SegmentedCommand,
    constructor: &CommandSpec,
    registry: &CommandRegistry,
    dialect: Option<SurfaceQuery<'_>>,
    analysis: &mut TkAnalysis,
) {
    let Some(path_index) = constructor.creates_instance_at.map(usize::from) else {
        return;
    };
    let Some((path, _)) = literal_word(command, path_index + 1) else {
        return;
    };
    let Some(class) = constructor.object_class else {
        return;
    };
    let Some(calls) = analysis.pending_widget_calls.get(&path) else {
        return;
    };
    let mut applied = Vec::new();
    for call in calls {
        let arguments: Vec<_> = call
            .arguments
            .iter()
            .map(PendingInvocationWord::as_registry)
            .collect();
        let Some(invocation) = registry.resolve_structured_instance_invocation(
            class.class_name,
            InvocationWords::structured(InvocationWord::Literal(&path), &arguments),
            dialect,
        ) else {
            continue;
        };
        if invocation.semantics.mutator
            || invocation
                .semantics
                .side_effects
                .iter()
                .any(|effect| effect.writes)
        {
            applied.push(call.clone());
        }
    }
    if applied.is_empty() {
        return;
    }
    if let Some(widget) = analysis.widgets.get_mut(&path) {
        widget.certainty = TkFactCertainty::Potential;
    }
    for call in applied {
        analysis.uncertainties.push(uncertainty(
            TkUncertaintyKind::PostConstructorMutation,
            call.source,
            format!(
                "Widget '{path}' may later be changed by deferred instance method '{}'; constructor-only facts are not asserted as final.",
                call.method
            ),
        ));
    }
}

fn collect_registry_lifecycle_effect(
    command: &SegmentedCommand,
    resolved_head: &str,
    spec: &CommandSpec,
    registry: &CommandRegistry,
    analysis: &mut TkAnalysis,
    context: ExecutableContext,
) {
    let releases_geometry = spec.traits.contains(Traits::FIRE_AND_FORGET_TEARDOWN)
        && spec.required_package == Some("Tk");
    // Which calls move a command binding, and which word carries the moved
    // name, is registry data read through the one transition vocabulary
    // (centralisation ledger C8) — never a coarse effect word this consumer
    // then re-destructures.
    let renames_commands =
        tcl_compiler::alias::command_table_transitions(registry, resolved_head, command.args())
            .command_bindings()
            .any(|transition| {
                matches!(
                    transition,
                    tcl_registry::CommandBindingTransition::Move { .. }
                        | tcl_registry::CommandBindingTransition::Delete { .. }
                )
            });
    if !(releases_geometry || renames_commands) {
        return;
    }
    let mut affected_indices = BTreeSet::new();
    if releases_geometry {
        affected_indices.extend(1..command.texts.len());
    }
    if renames_commands {
        // `rename oldName newName` only changes the binding of `oldName`.
        // A widget-looking destination is unrelated to an existing widget
        // command (and normally makes the Tcl call fail because it exists).
        affected_indices.insert(1);
    }
    for index in affected_indices {
        let Some((path, source)) = literal_word(command, index) else {
            continue;
        };
        let affected: Vec<String> = analysis
            .widgets
            .keys()
            .filter(|candidate| widget_is_within(candidate, &path))
            .cloned()
            .collect();
        if affected.is_empty() {
            if context == ExecutableContext::PotentialBody
                && tcl_registry::tk_geometry::is_widget_path_or_root(&path)
            {
                analysis
                    .pending_lifecycle
                    .entry(path)
                    .or_default()
                    .push(source);
            }
            continue;
        }
        if context == ExecutableContext::Direct && releases_geometry {
            analysis
                .active_placements
                .retain(|candidate, _| !widget_is_within(candidate, &path));
        }
        for affected_path in affected {
            let Some(widget) = analysis.widgets.get_mut(&affected_path) else {
                continue;
            };
            widget.certainty = TkFactCertainty::Potential;
            analysis.uncertainties.push(uncertainty(
                TkUncertaintyKind::WidgetLifecycleMutation,
                source,
                format!(
                    "Widget '{affected_path}' is affected by a registry-declared destructive or command-table operation; its final lifetime is not asserted."
                ),
            ));
        }
    }
}

fn apply_pending_lifecycle(
    command: &SegmentedCommand,
    constructor: &CommandSpec,
    analysis: &mut TkAnalysis,
) {
    let Some(path_index) = constructor.creates_instance_at.map(usize::from) else {
        return;
    };
    let Some((path, _)) = literal_word(command, path_index + 1) else {
        return;
    };
    let sources: Vec<TkSourceSpan> = analysis
        .pending_lifecycle
        .iter()
        .filter(|(ancestor, _)| widget_is_within(&path, ancestor))
        .flat_map(|(_, sources)| sources.iter().copied())
        .collect();
    if sources.is_empty() {
        return;
    }
    if let Some(widget) = analysis.widgets.get_mut(&path) {
        widget.certainty = TkFactCertainty::Potential;
    }
    for source in sources {
        analysis.uncertainties.push(uncertainty(
            TkUncertaintyKind::WidgetLifecycleMutation,
            source,
            format!(
                "Widget '{path}' may later be destroyed or rebound by deferred code; its final lifetime is not asserted."
            ),
        ));
    }
}

// Placement is one ordered state transition over the live manager domain;
// keep conflict invalidation beside the registry-derived placement facts.
#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
fn collect_placement(
    command: &SegmentedCommand,
    manager: &str,
    spec: &CommandSpec,
    dialect: Option<SurfaceQuery<'_>>,
    tk_version: Option<&str>,
    widgets: &BTreeMap<String, CollectedWidget>,
    active: &mut BTreeMap<String, CollectedPlacement>,
    conflicts: &mut Vec<TkGeometryConflict>,
    uncertainties: &mut Vec<TkUiUncertainty>,
    context: ExecutableContext,
) {
    let Some(geometry_spec) = spec.tk_geometry else {
        uncertainties.push(uncertainty(
            TkUncertaintyKind::UnsupportedGeometryForm,
            command.span,
            "The registry marks this as a geometry manager but supplies no geometry descriptor.",
        ));
        return;
    };
    let Some((first_word, first_target)) = literal_word(command, 1) else {
        uncertainties.push(uncertainty(
            if command.args().is_empty() {
                TkUncertaintyKind::UnsupportedGeometryForm
            } else {
                TkUncertaintyKind::DynamicGeometryTarget
            },
            command.span,
            "The geometry manager is not in a supported direct literal-target form.",
        ));
        return;
    };

    let subcommand = spec
        .resolve_subcommand_word(&first_word, dialect, tk_version, None)
        .unique()
        .and_then(|canonical| spec.subcommands.iter().find(|sub| sub.name == canonical));
    if let Some(subcommand) = subcommand
        && geometry_spec.release_subcommands.contains(&subcommand.name)
    {
        if context == ExecutableContext::Direct {
            for index in 2..command.texts.len() {
                if let Some((path, _)) = literal_word(command, index)
                    && tcl_registry::tk_geometry::is_widget_path(&path)
                {
                    active.remove(&path);
                }
            }
        }
        record_potential_execution(context, command.span, "geometry release", uncertainties);
        return;
    }

    let (target_start, raw_option_specs, option_parent_dialects, first_path, first_target) =
        if geometry_spec.direct_form && tcl_registry::tk_geometry::is_widget_path(&first_word) {
            (1, spec.options, spec.surface, first_word, first_target)
        } else if let Some(subcommand) = subcommand
            && geometry_spec.placement_subcommand == Some(subcommand.name)
        {
            let Some((path, target)) = literal_word(command, 2) else {
                uncertainties.push(uncertainty(
                    TkUncertaintyKind::DynamicGeometryTarget,
                    command.span,
                    "The geometry placement subcommand has no static literal target.",
                ));
                return;
            };
            (
                2,
                subcommand.options,
                subcommand.surface.or(spec.surface),
                path,
                target,
            )
        } else if subcommand.is_some() {
            // A registry-known query/container-configuration form is not a
            // placement and introduces no uncertainty.
            return;
        } else {
            uncertainties.push(uncertainty(
                TkUncertaintyKind::UnsupportedGeometryForm,
                first_target,
                "This geometry-manager form does not place a widget.",
            ));
            return;
        };
    let option_specs = available_options(
        raw_option_specs,
        dialect,
        option_parent_dialects,
        tk_version,
    );
    if !tcl_registry::tk_geometry::is_widget_path(&first_path) {
        uncertainties.push(uncertainty(
            TkUncertaintyKind::UnsupportedGeometryForm,
            first_target,
            "The geometry manager target is not a supported widget path.",
        ));
        return;
    }
    let mut targets = vec![(first_path, first_target)];
    let mut index = target_start + 1;
    while let Some(word) = command.texts.get(index) {
        if declared_option(word, &option_specs).is_some() {
            break;
        }
        let Some((path, target)) = literal_word(command, index) else {
            uncertainties.push(uncertainty(
                TkUncertaintyKind::DynamicGeometryTarget,
                command
                    .argv
                    .get(index)
                    .map_or(command.span, |token| token.span),
                "A geometry-manager target is not a static literal.",
            ));
            index += 1;
            continue;
        };
        if !tcl_registry::tk_geometry::is_widget_path(&path) {
            uncertainties.push(uncertainty(
                TkUncertaintyKind::UnsupportedGeometryForm,
                target,
                "The geometry manager is not in a supported direct widget-target form.",
            ));
            return;
        }
        targets.push((path, target));
        index += 1;
    }
    let options = literal_options(command, index, &option_specs, uncertainties);
    for (path, target) in targets {
        let container = effective_geometry_container(
            command,
            index,
            &path,
            geometry_spec.container_option,
            &option_specs,
            &options,
            uncertainties,
        );
        let collected = CollectedPlacement {
            widget: path,
            placement: TkGeometryPlacement {
                manager: manager.to_owned(),
                options: options.clone(),
                source: command.span.into(),
                target,
                container,
                certainty: certainty_for_context(context),
            },
            container_policy: geometry_spec.container_policy,
        };
        record_geometry_placement(collected, active, conflicts, widgets);
    }
    record_potential_execution(context, command.span, "geometry placement", uncertainties);
}

fn record_geometry_placement(
    collected: CollectedPlacement,
    active: &mut BTreeMap<String, CollectedPlacement>,
    conflicts: &mut Vec<TkGeometryConflict>,
    widgets: &BTreeMap<String, CollectedWidget>,
) {
    // Potential callbacks do not overwrite proven direct state. They remain
    // visible when they are the only placement fact, but cannot manufacture a
    // definite manager conflict.
    if collected.placement.certainty == TkFactCertainty::Potential {
        active.entry(collected.widget.clone()).or_insert(collected);
        return;
    }

    // Tk_ManageGeometry first releases this content window from its previous
    // manager, so a sole widget can switch pack -> grid without conflict.
    active.remove(&collected.widget);
    if collected.container_policy == TkGeometryContainerPolicy::Exclusive
        && let Some(container) = collected.placement.container.as_deref()
        && let Some(other) = active.values().find(|other| {
            other.container_policy == TkGeometryContainerPolicy::Exclusive
                && other.placement.certainty == TkFactCertainty::Certain
                && other.placement.container.as_deref() == Some(container)
                && other.placement.manager != collected.placement.manager
        })
    {
        let mut managers = vec![
            other.placement.manager.clone(),
            collected.placement.manager.clone(),
        ];
        managers.sort();
        managers.dedup();
        let conflict = TkGeometryConflict {
            container: container.to_owned(),
            managers,
            placements: vec![
                TkGeometryConflictPlacement {
                    widget: other.widget.clone(),
                    manager: other.placement.manager.clone(),
                    source: other.placement.source,
                },
                TkGeometryConflictPlacement {
                    widget: collected.widget.clone(),
                    manager: collected.placement.manager.clone(),
                    source: collected.placement.source,
                },
            ],
        };
        // Decide certainty at the point the rejected placement occurs. A
        // later destroy/recreate changes the final UI, but cannot erase an
        // error that already stopped this execution path.
        if geometry_conflict_is_live(&conflict, widgets) {
            conflicts.push(conflict);
        }
        // The attempted placement fails; do not claim the container.
        return;
    }
    active.insert(collected.widget.clone(), collected);
}

fn certainty_for_context(context: ExecutableContext) -> TkFactCertainty {
    match context {
        ExecutableContext::Direct => TkFactCertainty::Certain,
        ExecutableContext::PotentialBody => TkFactCertainty::Potential,
    }
}

fn record_potential_execution(
    context: ExecutableContext,
    source: Span,
    fact: &str,
    uncertainties: &mut Vec<TkUiUncertainty>,
) {
    if context == ExecutableContext::PotentialBody {
        uncertainties.push(uncertainty(
            TkUncertaintyKind::PotentialExecution,
            source,
            format!("This {fact} is inside a body whose execution is not statically proven."),
        ));
    }
}

fn effective_geometry_container(
    command: &SegmentedCommand,
    option_start: usize,
    target: &str,
    container_option: Option<&str>,
    option_specs: &[OptionSpec],
    options: &BTreeMap<String, TkLiteralOption>,
    uncertainties: &mut Vec<TkUiUncertainty>,
) -> Option<String> {
    let Some(container_option) = container_option else {
        return Some(parent_path(target).to_owned());
    };
    let option_was_written = command.texts.iter().skip(option_start).any(|word| {
        declared_option(word, option_specs).is_some_and(|option| option.name == container_option)
    });
    if !option_was_written {
        return Some(parent_path(target).to_owned());
    }
    let value = options.get(container_option)?;
    if tcl_registry::tk_geometry::is_widget_path_or_root(&value.value) {
        Some(value.value.clone())
    } else {
        uncertainties.push(uncertainty(
            TkUncertaintyKind::DynamicGeometryContainer,
            value.source,
            format!(
                "Geometry option '{container_option}' does not name a supported static container."
            ),
        ));
        None
    }
}

fn literal_options(
    command: &SegmentedCommand,
    start: usize,
    specs: &[OptionSpec],
    uncertainties: &mut Vec<TkUiUncertainty>,
) -> BTreeMap<String, TkLiteralOption> {
    let mut result = BTreeMap::new();
    let mut index = start;
    while let Some(word) = command.texts.get(index) {
        let Some(option) = declared_option(word, specs) else {
            index += 1;
            continue;
        };
        if !option.takes_value() {
            result.insert(
                option.name.to_owned(),
                TkLiteralOption {
                    value: "true".to_owned(),
                    source: command.argv[index].span.into(),
                },
            );
            index += 1;
            continue;
        }
        let Some((value, source)) = literal_word(command, index + 1) else {
            uncertainties.push(uncertainty(
                TkUncertaintyKind::NonLiteralOption,
                command
                    .argv
                    .get(index)
                    .map_or(command.span, |token| token.span),
                format!(
                    "The value of option '{}' is not a static literal.",
                    option.name
                ),
            ));
            index += 2;
            continue;
        };
        result.insert(option.name.to_owned(), TkLiteralOption { value, source });
        index += 2;
    }
    result
}

fn available_options(
    specs: &[OptionSpec],
    dialect: Option<SurfaceQuery<'_>>,
    parent_surface: Option<&'static [SpecSurface]>,
    package_version: Option<&str>,
) -> Vec<OptionSpec> {
    specs
        .iter()
        .filter(|option| option.supports_dialect(dialect, parent_surface))
        .filter(|option| option.available_for_version(package_version))
        .cloned()
        .collect()
}

fn declared_option<'a>(word: &str, specs: &'a [OptionSpec]) -> Option<&'a OptionSpec> {
    resolve_option_prefix(specs, word)
}

fn literal_word(command: &SegmentedCommand, index: usize) -> Option<(String, TkSourceSpan)> {
    if !command.word_views_aligned()
        || !command
            .single_token_word
            .get(index)
            .copied()
            .unwrap_or(false)
        || command
            .expand_word
            .as_ref()
            .is_some_and(|expanded| expanded.get(index).copied().unwrap_or(false))
    {
        return None;
    }
    let fragment = command.word_fragments.get(index)?.as_slice();
    let [fragment] = fragment else { return None };
    match fragment.token.kind {
        TokenType::Str => Some((
            command.texts.get(index)?.clone(),
            fragment.token.span.into(),
        )),
        TokenType::Esc if !fragment.text.contains('$') && !fragment.text.contains('[') => Some((
            command.texts.get(index)?.clone(),
            fragment.token.span.into(),
        )),
        _ => None,
    }
}

fn parent_path(path: &str) -> &str {
    path.rfind('.')
        .filter(|index| *index > 0)
        .map_or(".", |index| &path[..index])
}

fn widget_is_within(candidate: &str, ancestor: &str) -> bool {
    tcl_registry::tk_geometry::widget_path_is_within(candidate, ancestor)
}

fn uncertainty(
    kind: TkUncertaintyKind,
    source: impl Into<TkSourceSpan>,
    message: impl Into<String>,
) -> TkUiUncertainty {
    TkUiUncertainty {
        kind,
        source: source.into(),
        message: message.into(),
    }
}

fn geometry_conflict_is_live(
    conflict: &TkGeometryConflict,
    widgets: &BTreeMap<String, CollectedWidget>,
) -> bool {
    let conflict_at = conflict
        .placements
        .iter()
        .map(|placement| placement.source.start)
        .max()
        .unwrap_or_default();
    let container_is_certain = conflict.container == "."
        || widgets.get(&conflict.container).is_some_and(|owner| {
            owner.certainty == TkFactCertainty::Certain && owner.source.command.start < conflict_at
        });
    container_is_certain
        && conflict.placements.iter().all(|placement| {
            widgets.get(&placement.widget).is_some_and(|widget| {
                widget.certainty == TkFactCertainty::Certain
                    && widget.source.command.start < placement.source.start
            })
        })
}

fn build_hierarchy(
    widgets: &BTreeMap<String, CollectedWidget>,
    uncertainties: &mut Vec<TkUiUncertainty>,
) -> (TkWidget, Vec<TkWidget>) {
    let paths: BTreeSet<String> = widgets.keys().cloned().collect();
    let mut children = BTreeMap::<String, Vec<String>>::new();
    let mut orphan_roots: Vec<String> = Vec::new();
    for path in &paths {
        let parent = parent_path(path);
        if parent == "." || paths.contains(parent) {
            children
                .entry(parent.to_owned())
                .or_default()
                .push(path.clone());
        } else {
            let widget = widgets.get(path).expect("path originates in widgets");
            uncertainties.push(uncertainty(
                TkUncertaintyKind::UnknownWidgetParent,
                widget.source.path,
                format!("Widget path '{path}' has no statically constructed parent '{parent}'."),
            ));
            // A missing ancestor makes every descendant's immediate parent
            // missing too. Keep only the highest orphan root: `build_widget`
            // already includes its statically known descendants, and exposing
            // each nested orphan separately would duplicate them in the JSON
            // model and the webview tree.
            if !orphan_roots.iter().any(|root| widget_is_within(path, root)) {
                orphan_roots.push(path.clone());
            }
        }
    }
    let mut root = implicit_root();
    root.children = children
        .get(".")
        .into_iter()
        .flatten()
        .filter_map(|child| build_widget(child, widgets, &children))
        .collect();
    let orphan_widgets = orphan_roots
        .iter()
        .filter_map(|path| build_widget(path, widgets, &children))
        .collect();
    (root, orphan_widgets)
}

fn build_widget(
    path: &str,
    widgets: &BTreeMap<String, CollectedWidget>,
    children: &BTreeMap<String, Vec<String>>,
) -> Option<TkWidget> {
    let widget = widgets.get(path)?;
    Some(TkWidget {
        path: path.to_owned(),
        constructor: widget.constructor.clone(),
        options: widget.options.clone(),
        geometry: widget.geometry.clone(),
        children: children
            .get(path)
            .into_iter()
            .flatten()
            .filter_map(|child| build_widget(child, widgets, children))
            .collect(),
        source: widget.source.clone(),
        certainty: widget.certainty,
    })
}

fn implicit_root() -> TkWidget {
    TkWidget {
        path: ".".to_owned(),
        constructor: "root".to_owned(),
        options: BTreeMap::new(),
        geometry: None,
        children: Vec::new(),
        source: TkWidgetSource {
            command: TkSourceSpan { start: 0, end: 0 },
            path: TkSourceSpan { start: 0, end: 0 },
        },
        certainty: TkFactCertainty::Certain,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn model(source: &str) -> TkUiModel {
        let dialect = crate::profile_for_dialect("tk");
        analyse_tk_ui(
            source,
            dialect,
            crate::registry_for_dialect_profile(dialect),
        )
    }

    fn model_in_dialect(source: &str, name: &str) -> TkUiModel {
        let dialect = crate::profile_for_dialect(name);
        analyse_tk_ui(
            source,
            dialect,
            crate::registry_for_dialect_profile(dialect),
        )
    }

    #[test]
    fn plain_tcl_requires_a_resolved_literal_tk_package_request() {
        let inactive = model_in_dialect("frame .looks_like_a_widget", "tcl8.6");
        assert!(!inactive.tk_active);
        assert!(inactive.root.is_none());
        assert_eq!(inactive.widget_count, 0);

        let active = model_in_dialect("package require Tk\nframe .real_widget", "tcl8.6");
        assert!(active.tk_active);
        assert_eq!(active.widget_count, 2);

        let exact = model_in_dialect("package require -exact Tk\nframe .exact_widget", "tcl8.6");
        assert!(exact.tk_active);
        assert_eq!(exact.widget_count, 2);

        let dynamic = model_in_dialect("package require $package\nframe .not_proven", "tcl8.6");
        assert!(!dynamic.tk_active);

        let impossible = model_in_dialect("package require Tk 99\nframe .not_proven", "tcl8.6");
        assert!(!impossible.tk_active);

        let compatible = model_in_dialect("package require Tk 8.5\nframe .proven", "tcl8.6");
        assert!(compatible.tk_active);

        let impossible_exact =
            model_in_dialect("package require -exact Tk 8.5\nframe .not_proven", "tcl8.6");
        assert!(!impossible_exact.tk_active);
    }

    #[test]
    fn finds_multiline_semicolon_and_quoted_literal_widgets_with_sources() {
        let source = "# frame .commented\nframe .top -background {navy blue}; button \".top.ok\" -text \"Save\"\npack .top.ok\n";
        let ui = model(source);
        assert_eq!(ui.schema_version, TK_UI_SCHEMA_VERSION);
        assert_eq!(ui.widget_count, 3);
        let root = ui.root.as_ref().expect("Tk dialect is active");
        assert_eq!(root.children[0].path, ".top");
        let button = &root.children[0].children[0];
        assert_eq!(button.path, ".top.ok");
        assert_eq!(button.options["-text"].value, "Save");
        assert_eq!(
            button.geometry.as_ref().map(|g| g.manager.as_str()),
            Some("pack")
        );
        assert!(button.source.command.end > button.source.command.start);
    }

    #[test]
    fn dynamic_paths_and_manager_forms_abstain() {
        let ui = model("frame $path\npack $path\npack configure .missing\n");
        assert_eq!(ui.widget_count, 1);
        assert!(
            ui.uncertainties
                .iter()
                .any(|u| u.kind == TkUncertaintyKind::DynamicWidgetPath)
        );
        assert!(
            ui.uncertainties
                .iter()
                .any(|u| u.kind == TkUncertaintyKind::DynamicGeometryTarget)
        );
        assert!(
            ui.uncertainties
                .iter()
                .any(|u| u.kind == TkUncertaintyKind::UnknownGeometryTarget)
        );
    }

    #[test]
    fn visits_registry_declared_nested_bodies() {
        let ui = model("proc make {} {\n frame .inside\n pack .inside\n}\n");
        assert_eq!(ui.widget_count, 2);
        let root = ui.root.as_ref().expect("Tk dialect is active");
        assert_eq!(root.children[0].path, ".inside");
        assert_eq!(root.children[0].certainty, TkFactCertainty::Potential);
        assert_eq!(
            root.children[0]
                .geometry
                .as_ref()
                .map(|g| g.manager.as_str()),
            Some("pack")
        );
        assert!(
            ui.uncertainties
                .iter()
                .any(|uncertainty| uncertainty.kind == TkUncertaintyKind::PotentialExecution)
        );
    }

    #[test]
    fn potential_bodies_do_not_activate_tk_or_create_definite_conflicts() {
        let inactive = model_in_dialect(
            "proc maybe {} { package require Tk; frame .inside }",
            "tcl8.6",
        );
        assert!(!inactive.tk_active);

        let alternatives =
            model("frame .holder\nif {$mode} {pack .a -in .holder} else {grid .b -in .holder}");
        assert!(alternatives.geometry_conflicts.is_empty());
    }

    #[test]
    fn follows_imported_and_qualified_heads_but_not_rebound_ones() {
        let ui = model(
            "namespace import ::ttk::button\nbutton .imported\n::frame .qualified\nproc frame args {}\nframe .rebound\n",
        );
        let paths: Vec<_> = ui
            .root
            .as_ref()
            .expect("Tk dialect is active")
            .children
            .iter()
            .map(|widget| widget.path.as_str())
            .collect();
        assert_eq!(paths, vec![".imported", ".qualified"]);
    }

    #[test]
    fn all_registry_geometry_managers_are_consumed_without_a_command_list() {
        let dialect = crate::profile_for_dialect("tk");
        let registry = crate::registry_for_dialect_profile(dialect);
        let managers = registry.commands_with_trait(Traits::TK_GEOMETRY_MANAGER);
        assert!(!managers.is_empty());
        for manager in managers {
            let source = format!("frame .w; {manager} .w");
            let ui = analyse_tk_ui(&source, dialect, registry);
            let widget = &ui.root.as_ref().expect("Tk dialect is active").children[0];
            assert_eq!(
                widget.geometry.as_ref().map(|item| item.manager.as_str()),
                Some(manager),
                "registry-declared manager {manager} was not consumed"
            );
        }
    }

    #[test]
    fn one_direct_manager_call_places_every_literal_leading_target() {
        let ui = model("frame .a\nframe .b\npack .a .b -side left\n");
        let root = ui.root.as_ref().expect("Tk dialect is active");
        assert_eq!(root.children.len(), 2);
        for widget in &root.children {
            let placement = widget.geometry.as_ref().expect("direct pack placement");
            assert_eq!(placement.manager, "pack");
            assert_eq!(placement.options["-side"].value, "left");
        }
    }

    #[test]
    fn registry_option_abbreviations_keep_literal_options() {
        let ui = model("button .save -und 0");
        assert_eq!(
            ui.root.as_ref().expect("Tk dialect is active").children[0].options["-underline"].value,
            "0"
        );
    }

    #[test]
    fn preview_filters_constructors_by_the_resolved_tk_floor() {
        let old = model_in_dialect(
            "package require Tk\nttk::toggleswitch .switch\npack .switch",
            "tcl8.6",
        );
        assert!(old.tk_active);
        assert_eq!(old.widget_count, 1);
        assert!(old.root.as_ref().expect("Tk is active").children.is_empty());
        assert!(old.geometry_conflicts.is_empty());

        let current = model_in_dialect("package require Tk\nttk::toggleswitch .switch", "tcl9.1");
        assert_eq!(current.widget_count, 2);
        assert_eq!(
            current.root.as_ref().expect("Tk is active").children[0].path,
            ".switch"
        );
    }

    #[test]
    fn preview_filters_options_before_resolving_abbreviations() {
        let old = model_in_dialect(
            "package require Tk\nlabel .label -text old -texta 45",
            "tcl8.6",
        );
        let old_options = &old.root.as_ref().expect("Tk is active").children[0].options;
        assert_eq!(old_options["-text"].value, "old");
        assert!(!old_options.contains_key("-textangle"));

        let current = model_in_dialect(
            "package require Tk\nlabel .label -text old -texta 45",
            "tcl9.1",
        );
        let current_options = &current.root.as_ref().expect("Tk is active").children[0].options;
        assert_eq!(current_options["-text"].value, "old");
        assert_eq!(current_options["-textangle"].value, "45");
    }

    #[test]
    fn reports_mixed_geometry_managers_and_unknown_parent() {
        let ui = model(
            "frame .a\nframe .a.one\nframe .a.two\npack .a.one\ngrid .a.two\nframe .lost.child\n",
        );
        assert_eq!(ui.geometry_conflicts.len(), 1);
        assert_eq!(ui.geometry_conflicts[0].container, ".a");
        assert!(
            ui.uncertainties
                .iter()
                .any(|u| u.kind == TkUncertaintyKind::UnknownWidgetParent)
        );
        assert_eq!(ui.orphan_widgets[0].path, ".lost.child");
    }

    #[test]
    fn nested_missing_parents_are_rendered_once_as_one_orphan_subtree() {
        let ui = model("frame .lost.child\nframe .lost.child.grandchild\n");
        assert_eq!(ui.orphan_widgets.len(), 1);
        assert_eq!(ui.orphan_widgets[0].path, ".lost.child");
        assert_eq!(
            ui.orphan_widgets[0].children[0].path,
            ".lost.child.grandchild"
        );
    }

    #[test]
    fn one_widget_can_switch_managers_without_a_conflict() {
        let ui = model("frame .a\npack .a\ngrid .a\n");
        let root = ui.root.as_ref().expect("Tk dialect is active");
        assert_eq!(
            root.children[0]
                .geometry
                .as_ref()
                .map(|placement| placement.manager.as_str()),
            Some("grid")
        );
        assert!(ui.geometry_conflicts.is_empty());
    }

    #[test]
    fn geometry_release_and_query_forms_do_not_place_widgets() {
        let released = model(
            "frame .holder\nframe .holder.a\nframe .holder.b\npack .holder.a\npack forget .holder.a\ngrid .holder.b\n",
        );
        assert!(released.geometry_conflicts.is_empty(), "{released:#?}");

        let query = model(
            "frame .holder\nframe .holder.a\nframe .holder.b\npack .holder.a\ngrid info .holder.b\n",
        );
        assert!(query.geometry_conflicts.is_empty(), "{query:#?}");
        assert!(
            !query
                .uncertainties
                .iter()
                .any(|entry| entry.kind == TkUncertaintyKind::UnsupportedGeometryForm)
        );

        let conflict = model(
            "frame .holder\nframe .holder.a\nframe .holder.b\npack .holder.a .holder.b\ngrid .holder.a\n",
        );
        assert_eq!(conflict.geometry_conflicts.len(), 1, "{conflict:#?}");
    }

    #[test]
    fn geometry_conflicts_use_registry_policy_and_effective_in_container() {
        let place = model("frame .a\nframe .b\nplace .a\ngrid .b\n");
        assert!(place.geometry_conflicts.is_empty());

        let separate = model(
            "frame .left\nframe .right\nframe .a\nframe .b\npack .a -in .left\ngrid .b -in .right\n",
        );
        assert!(separate.geometry_conflicts.is_empty());
        let a = separate
            .root
            .as_ref()
            .unwrap()
            .children
            .iter()
            .find(|widget| widget.path == ".a")
            .unwrap();
        assert_eq!(
            a.geometry.as_ref().unwrap().container.as_deref(),
            Some(".left")
        );

        let same =
            model("frame .holder\nframe .a\nframe .b\npack .a -in .holder\ngrid .b -in .holder\n");
        assert_eq!(same.geometry_conflicts.len(), 1);
        assert_eq!(same.geometry_conflicts[0].container, ".holder");

        let missing = model("frame .a\npack .a -in .not-created\n");
        assert!(
            missing
                .uncertainties
                .iter()
                .any(|entry| { entry.kind == TkUncertaintyKind::UnknownGeometryContainer })
        );

        let later_destroy = model(
            "frame .holder\nframe .holder.a\nframe .holder.b\npack .holder.a\ngrid .holder.b\ndestroy .holder\n",
        );
        assert_eq!(
            later_destroy.geometry_conflicts.len(),
            1,
            "later lifecycle changes must not erase an already-triggered Tk error: {later_destroy:#?}"
        );
    }

    #[test]
    fn accepts_real_tk_path_components_and_marks_lifecycle_mutations_uncertain() {
        let ui =
            model("ttk::frame .main-pane\n.main-pane configure -padding 4\ndestroy .main-pane\n");
        let widget = &ui.root.as_ref().unwrap().children[0];
        assert_eq!(widget.path, ".main-pane");
        assert_eq!(widget.certainty, TkFactCertainty::Potential);
        assert!(
            ui.uncertainties
                .iter()
                .any(|entry| { entry.kind == TkUncertaintyKind::PostConstructorMutation })
        );
        assert!(
            ui.uncertainties
                .iter()
                .any(|entry| { entry.kind == TkUncertaintyKind::WidgetLifecycleMutation })
        );
    }

    #[test]
    fn deferred_mutation_before_constructor_downgrades_the_later_widget() {
        let ui =
            model("proc later {} {.w configure -text changed}\ncheckbutton .w -text initial\n");
        let widget = &ui.root.as_ref().unwrap().children[0];
        assert_eq!(widget.path, ".w");
        assert_eq!(widget.certainty, TkFactCertainty::Potential);
        assert!(ui.uncertainties.iter().any(|entry| {
            entry.kind == TkUncertaintyKind::PostConstructorMutation
                && entry.message.contains("configure")
        }));

        let destroyed = model("proc later {} {destroy .w}\ncheckbutton .w\n");
        let widget = &destroyed.root.as_ref().unwrap().children[0];
        assert_eq!(widget.certainty, TkFactCertainty::Potential);
        assert!(
            destroyed
                .uncertainties
                .iter()
                .any(|entry| entry.kind == TkUncertaintyKind::WidgetLifecycleMutation)
        );
    }

    #[test]
    fn command_table_lifecycle_uses_only_rename_old_name() {
        for source in ["frame .w\nproc .w {} {}\n", "frame .w\nrename other .w\n"] {
            let ui = model(source);
            assert_eq!(
                ui.root.as_ref().unwrap().children[0].certainty,
                TkFactCertainty::Certain,
                "an unrelated command-table operand must not mutate the widget: {source}"
            );
            assert!(
                !ui.uncertainties
                    .iter()
                    .any(|entry| entry.kind == TkUncertaintyKind::WidgetLifecycleMutation),
                "an unrelated command-table operand must not add lifecycle uncertainty: {ui:#?}"
            );
        }

        let renamed = model("frame .w\nrename .w other\n");
        assert_eq!(
            renamed.root.as_ref().unwrap().children[0].certainty,
            TkFactCertainty::Potential
        );
        assert!(
            renamed
                .uncertainties
                .iter()
                .any(|entry| entry.kind == TkUncertaintyKind::WidgetLifecycleMutation)
        );
    }

    #[test]
    fn argument_sensitive_widget_queries_do_not_look_like_mutations() {
        let cases = [
            ("button", "configure", "configure -text Save"),
            ("entry", "configure -width", "configure -width 20"),
            ("ttk::button", "state", "state disabled"),
            ("ttk::combobox", "current", "current 2"),
            ("ttk::notebook", "select", "select .page"),
            ("ttk::treeview", "focus", "focus item"),
            ("ttk::treeview", "selection", "selection set item"),
            ("ttk::toggleswitch", "switchstate", "switchstate 1"),
            ("entry", "selection present", "selection clear"),
            ("spinbox", "selection element", "selection element buttonup"),
            ("listbox", "selection includes 0", "selection set 0"),
            ("canvas", "select item", "select clear"),
            ("panedwindow", "proxy coord", "proxy forget"),
            ("text", "edit canundo", "edit undo"),
            ("text", "tag ranges hot", "tag raise hot"),
            ("text", "peer names", "peer create .peer"),
            ("ttk::treeview", "tag has hot item", "tag add hot item"),
        ];
        for (constructor, query, mutation) in cases {
            let query_ui = model(&format!("{constructor} .w\n.w {query}\n"));
            let query_widget = &query_ui.root.as_ref().unwrap().children[0];
            assert_eq!(
                query_widget.certainty,
                TkFactCertainty::Certain,
                "query must retain constructor certainty: {constructor} {query}"
            );
            assert!(
                !query_ui
                    .uncertainties
                    .iter()
                    .any(|entry| { entry.kind == TkUncertaintyKind::PostConstructorMutation }),
                "query must not report mutation: {constructor} {query}"
            );

            let mutation_ui = model(&format!("{constructor} .w\n.w {mutation}\n"));
            let mutation_widget = &mutation_ui.root.as_ref().unwrap().children[0];
            assert_eq!(
                mutation_widget.certainty,
                TkFactCertainty::Potential,
                "setter must downgrade constructor certainty: {constructor} {mutation}"
            );
            assert!(
                mutation_ui
                    .uncertainties
                    .iter()
                    .any(|entry| { entry.kind == TkUncertaintyKind::PostConstructorMutation }),
                "setter must report mutation: {constructor} {mutation}"
            );
        }

        let deferred_query = model("proc later {} {.w configure}\nframe .w\n");
        assert_eq!(
            deferred_query.root.as_ref().unwrap().children[0].certainty,
            TkFactCertainty::Certain,
            "a deferred getter before construction must stay a getter"
        );
        assert!(
            !deferred_query
                .uncertainties
                .iter()
                .any(|entry| entry.kind == TkUncertaintyKind::PostConstructorMutation)
        );

        let deferred_operation_query = model("proc later {} {.w selection present}\nentry .w\n");
        assert_eq!(
            deferred_operation_query.root.as_ref().unwrap().children[0].certainty,
            TkFactCertainty::Certain,
            "deferred calls must retain literal operation facts"
        );
    }

    #[test]
    fn computed_unknown_and_ambiguous_widget_operations_keep_parent_mutation_fallback() {
        for source in [
            "entry .w\nset op present\n.w selection $op\n",
            "entry .w\n.w selection unknown\n",
            "text .w\n.w tag ra hot\n",
            "ttk::treeview .w\n.w tag c hot\n",
        ] {
            let ui = model(source);
            assert_eq!(
                ui.root.as_ref().unwrap().children[0].certainty,
                TkFactCertainty::Potential,
                "indeterminate operation must not be guessed as a query: {source}"
            );
            assert!(
                ui.uncertainties
                    .iter()
                    .any(|entry| entry.kind == TkUncertaintyKind::PostConstructorMutation),
                "parent fallback must remain conservative: {source}"
            );
        }

        let abbreviated = model("entry .w\n.w selection pres\n");
        assert_eq!(
            abbreviated.root.as_ref().unwrap().children[0].certainty,
            TkFactCertainty::Certain,
            "a unique operation prefix resolves to the query form"
        );
    }

    #[test]
    fn recreated_widget_cannot_inherit_an_earlier_instances_geometry() {
        let ui = model("frame .item\npack .item\ndestroy .item\nframe .item\n");
        let widget = &ui.root.as_ref().expect("Tk dialect is active").children[0];
        assert_eq!(widget.path, ".item");
        assert_eq!(widget.certainty, TkFactCertainty::Potential);
        assert!(
            widget.geometry.is_none(),
            "old pack evidence must not attach to the recreated instance: {ui:#?}"
        );
        assert!(ui.geometry_conflicts.is_empty());
        assert!(
            ui.uncertainties
                .iter()
                .any(|entry| entry.kind == TkUncertaintyKind::WidgetLifecycleMutation)
        );
    }

    #[test]
    fn destroying_the_root_releases_descendant_geometry_and_lifetimes() {
        let ui = model(
            "frame .holder\nframe .holder.a\npack .holder.a\ndestroy .\nframe .holder\nframe .holder.b\ngrid .holder.b\n",
        );
        assert!(ui.geometry_conflicts.is_empty(), "{ui:#?}");
        let holder = ui
            .root
            .as_ref()
            .unwrap()
            .children
            .iter()
            .find(|widget| widget.path == ".holder")
            .unwrap();
        assert_eq!(holder.certainty, TkFactCertainty::Potential);
        assert!(
            ui.uncertainties
                .iter()
                .any(|entry| entry.kind == TkUncertaintyKind::WidgetLifecycleMutation)
        );
    }

    #[test]
    fn absent_or_lifecycle_ambiguous_containers_cannot_form_definite_conflicts() {
        let absent = model("frame .a\nframe .b\npack .a -in .missing\ngrid .b -in .missing\n");
        assert!(absent.geometry_conflicts.is_empty(), "{absent:#?}");
        assert!(
            absent
                .uncertainties
                .iter()
                .any(|entry| entry.kind == TkUncertaintyKind::UnknownGeometryContainer)
        );

        let recreated = model(
            "frame .holder\nframe .a\nframe .b\ndestroy .holder\npack .a -in .holder\ngrid .b -in .holder\n",
        );
        assert!(recreated.geometry_conflicts.is_empty(), "{recreated:#?}");
    }

    #[test]
    fn serializes_versioned_host_metadata_only_when_stamped() {
        let mut ui = model("frame .x");
        let json = serde_json::to_value(&ui).expect("model is serializable");
        assert!(json.get("document_uri").is_none());
        ui.document_uri = Some("file:///tmp/example.tcl".to_owned());
        ui.document_version = Some(7);
        let stamped = serde_json::to_value(ui).expect("model is serializable");
        assert_eq!(stamped["document_version"], 7);
    }

    #[test]
    fn caps_repetitive_uncertainty_detail_and_reports_the_omitted_count() {
        let source = (0..(MAX_TK_UI_UNCERTAINTIES + 17))
            .map(|index| format!("frame $dynamic{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        let ui = model(&source);
        assert_eq!(ui.uncertainties.len(), MAX_TK_UI_UNCERTAINTIES);
        assert_eq!(ui.uncertainties_truncated, 17);

        let json = serde_json::to_value(ui).expect("model is serializable");
        assert_eq!(json["uncertainties_truncated"], 17);
    }

    #[test]
    fn cap_includes_unknown_parent_uncertainties_added_during_hierarchy_building() {
        let source = (0..(MAX_TK_UI_UNCERTAINTIES + 9))
            .map(|index| format!("frame .missing{index}.child"))
            .collect::<Vec<_>>()
            .join("\n");
        let ui = model(&source);
        assert_eq!(ui.uncertainties.len(), MAX_TK_UI_UNCERTAINTIES);
        assert_eq!(ui.uncertainties_truncated, 9);
        assert!(
            ui.uncertainties
                .iter()
                .all(|entry| entry.kind == TkUncertaintyKind::UnknownWidgetParent)
        );
    }

    #[test]
    fn caps_retained_widgets_and_reports_the_complete_static_count() {
        let extra = 19;
        let source = (0..(MAX_TK_UI_WIDGETS + extra))
            .map(|index| format!("frame .widget{index}"))
            .collect::<Vec<_>>()
            .join("\n");
        let ui = model(&source);
        assert_eq!(ui.widget_count, MAX_TK_UI_WIDGETS + extra + 1);
        assert_eq!(ui.widgets_truncated, extra);
        assert_eq!(
            ui.root
                .as_ref()
                .expect("Tk dialect is active")
                .children
                .len(),
            MAX_TK_UI_WIDGETS
        );
        assert!(
            ui.uncertainties
                .iter()
                .any(|entry| entry.kind == TkUncertaintyKind::WidgetLimitReached)
        );
    }
}
