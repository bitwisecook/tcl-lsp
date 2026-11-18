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

//! Method words, options and accessors of object receivers.

use super::argument_override::{ArgOverride, role_claimed_by_token_pass};
use super::legend::TokenKind;
use super::receiver_source;
use super::script_walk::ScriptCtx;
use super::variable_name_roles::VarNameArgRoles;
use rustc_hash::FxHashMap;
use tcl_compiler::analyser::{AnalysisResult, ClassHierarchy};
use tcl_compiler::segmenter::segment_commands_with_offset_and_config;
use tcl_lexer::{Token, TokenType};
use tcl_registry::CommandRegistry;

/// Object-handle → class-name map for the current document, keyed by the
/// handle text a `$var method` dispatch presents (minus the leading `$`) —
/// a scalar (`chart`) or array element (`arr(key)`).  Built once per document
/// from the [`CompilationUnit`] by
/// [`tcl_compiler::object_types::object_handle_classes`].
pub(super) type ObjectClassMap =
    std::collections::HashMap<String, std::collections::HashSet<String>>;

/// Named-instance display reports with separately retained compatibility mode.
/// Conditional original source rows never enter legacy label classification.
/// No report label supplies receiver identity, dispatch or Native authority.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NamedInstanceMap {
    rows: std::collections::HashMap<String, NamedInstanceDisplayRow>,
    pub(super) ambiguous: std::collections::HashSet<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct NamedInstanceDisplayRow {
    class: String,
    compatibility: bool,
}

impl NamedInstanceMap {
    /// Preserve the existing analysis-gated compatibility projection. Genuine
    /// conditional source-card reports are added independently below.
    #[must_use]
    pub fn from_analysis(analysis: &AnalysisResult) -> Self {
        let mut result = Self::default();
        for (name, class) in &analysis.instance_classes {
            if analysis.created_instance_commands.contains(name.as_str()) {
                result.insert_report(name.clone(), class.clone(), true);
            }
        }
        result
    }

    /// Add display-only labels from genuine final source instance cards.
    /// This never enables a compatibility row or changes source selection.
    pub fn extend_conditional_source_cards(
        &mut self,
        cards: &tcl_compiler::registry_invocation::source_structure::OriginalProjectClassCards,
    ) {
        for (name, class) in cards.named_instance_display_reports() {
            self.insert_report(name, class.qualified_name.clone(), false);
        }
    }

    /// Merge report rows without upcasting a conditional row to compatibility.
    /// Independent compatible rows retain their previous mode; conflicts abstain.
    pub fn merge_reports(&mut self, other: &Self) {
        for name in &other.ambiguous {
            self.rows.remove(name);
            self.ambiguous.insert(name.clone());
        }
        for (name, row) in &other.rows {
            self.insert_report(name.clone(), row.class.clone(), row.compatibility);
        }
    }

    /// Whether the display report contains any represented named instance.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Display label lookup only; this cannot supply a receiver/source owner.
    #[must_use]
    pub fn get(&self, label: &str) -> Option<&str> {
        self.rows.get(label).map(|row| row.class.as_str())
    }

    /// Readonly report labels for presentation. Conditional mode is not a
    /// class/command resolver and no raw-map/Deref conversion is provided.
    pub fn report_rows(&self) -> impl Iterator<Item = (&str, &str)> {
        self.rows
            .iter()
            .map(|(name, row)| (name.as_str(), row.class.as_str()))
    }

    pub(super) fn compatibility_rows(&self) -> impl Iterator<Item = (&String, &String)> {
        self.rows
            .iter()
            .filter(|(_, row)| row.compatibility)
            .map(|(name, row)| (name, &row.class))
    }

    fn insert_report(&mut self, name: String, class: String, compatibility: bool) {
        if self.ambiguous.contains(&name) {
            return;
        }
        match self.rows.entry(name.clone()) {
            std::collections::hash_map::Entry::Occupied(mut entry) => {
                if entry.get().class != class {
                    entry.remove();
                    self.ambiguous.insert(name);
                } else if compatibility {
                    // Only an independently retained pre-existing compatible
                    // row can enable the legacy projection.
                    entry.get_mut().compatibility = true;
                }
            }
            std::collections::hash_map::Entry::Vacant(entry) => {
                entry.insert(NamedInstanceDisplayRow {
                    class,
                    compatibility,
                });
            }
        }
    }
}

/// The optional workspace-merged facts a semantic-tokens request can enrich
/// its object-dispatch resolution with — [`ClassHierarchy`] (cross-file
/// classes), [`VarNameArgRoles`] (cross-file proc parameter roles), and
/// [`NamedInstanceMap`] (cross-file `CLASS create NAME` bindings).
/// Bundled into one `Copy` struct so the `range_*` full-arity entry
/// point stays within budget instead of growing a ninth positional
/// parameter.
#[derive(Clone, Copy, Default)]
pub struct WorkspaceTokenFacts<'a> {
    /// The workspace-merged class hierarchy, or the local single-file one.
    pub classes: Option<&'a ClassHierarchy>,
    /// Original project class/factory/instance display cards. Present means
    /// user-class roles require this typed owner, including after refusal.
    pub project_classes: Option<
        &'a [tcl_compiler::registry_invocation::source_structure::OriginalProjectClassCards],
    >,
    /// The workspace-merged (or local) inferred proc parameter roles.
    pub proc_roles: Option<&'a VarNameArgRoles>,
    /// The workspace-merged (or local) `CLASS create NAME` bareword
    /// instance-command index.
    pub named_instances: Option<&'a NamedInstanceMap>,
    /// Local analysis, when available. Package-require floors are document
    /// facts used to resolve lifecycle-gated registry methods.
    pub analysis: Option<&'a AnalysisResult>,
}

/// Build a [`NamedInstanceMap`] from `analysis` — `None` when `analysis`
/// created no named instances, so the merge in [`collect_entries`] is a
/// no-op lookup-free skip for the overwhelming majority of documents.
pub(super) fn named_instances_from_analysis(analysis: &AnalysisResult) -> NamedInstanceMap {
    NamedInstanceMap::from_analysis(analysis)
}

/// Precise `$obj method …` highlighting via the registry's object-class model.
///
/// When the command head is an object handle whose class is known — a `$var`
/// bound by `set var [Class new]` (tracked by [`tcl_compiler::object_types`]),
/// or a direct `[Class new] method …` dispatch — and the class declares
/// `method`, the method word is highlighted as a function and the method's
/// declared options / option values are coloured exactly like a built-in's
/// (`Decorator` for the switch, `EnumMember` for a closed-set value, else
/// `OptionValue`).
///
/// Recognised method roles precede generic source options: a method's
/// options are claimed precisely first, and anything it leaves (an option not
/// in the spec, an un-provenanced receiver) is picked up by the generic
/// shape-based fallback.  A `$var` / `[cmd]` option value keeps its own
/// highlight; only literal (`Esc`/`Str`) values are recoloured.
pub(super) fn insert_object_method_overrides(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    classes: Option<&ClassHierarchy>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if classes.is_some()
        && let Some(providers) = ctx.project_classes
        && receiver_source::insert_project_method(ctx, seg, providers, overrides)
    {
        return;
    }
    if insert_registered_instance_method(ctx, seg, overrides) {
        return;
    }
    // Typed project mode cannot recover a refused source class from maps.
    let classes = ctx.project_classes.is_none().then_some(classes).flatten();
    let registry = ctx.registry;
    let object_classes = ctx.object_classes;
    let document_floor = ctx
        .analysis
        .map(|analysis| crate::document_floor::DocumentFloor::new(analysis, ctx.dialect));
    let dialect = Some(ctx.context.authoring_query());
    let (Some(head_tok), Some(head_text), Some(method)) =
        (seg.argv.first(), seg.texts.first(), seg.texts.get(1))
    else {
        return;
    };
    // Candidate receiver classes implied by the head's shape: a `$var` object
    // handle, a direct `[Class new] …` constructor, a `[dict get $coll $k]`
    // / `[lindex $coll $i]` retrieval resolved through the retained original
    // collection occurrences, or a bareword instance-command name bound by a
    // positional create call (`ttk::treeview .t` / a registry naming factory;
    // `object_classes` is name-keyed regardless of whether the name came from a
    // `set` LHS or a bareword factory, so the same map already carries these —
    // see `object_types::harvest_unit`'s `Statement::Call` arm).
    let original_words = tcl_compiler::ir::CommandTokens::from_segmented(
        &tcl_lexer::SourceMap::new(ctx.full_source),
        ctx.config,
        seg,
    );
    let variable = original_words
        .word_exprs
        .first()
        .and_then(tcl_compiler::ir::WordExpr::sole_variable_substitution)
        .and_then(|(spelling, _)| object_handle_name(spelling, Some(ctx.config)));
    let mut candidates: Vec<String> = if let Some(name) = variable {
        object_classes
            .get(name)
            .map_or_default(|classes| classes.iter().cloned().collect())
    } else {
        match head_tok.kind {
            // The genuine whole receiver word, child source binding and selected
            // family determine conditional candidates. Missing canonical class
            // receipts remain typed unavailable inside the shared receiver path.
            TokenType::Cmd => match receiver_source::candidates(ctx, seg) {
                Ok(receiver_source::ReceiverSourceCandidates::ClassNames(classes)) => classes,
                Ok(receiver_source::ReceiverSourceCandidates::LogicalClass(class)) => {
                    class.insert_method(ctx, seg, classes, overrides);
                    return;
                }
                Ok(receiver_source::ReceiverSourceCandidates::CollectionElement) => {
                    receiver_source::insert_collection_method(ctx, seg, overrides);
                    return;
                }
                Err(_) => Vec::new(),
            },
            TokenType::Esc => object_classes
                .get(head_text.as_str())
                .map_or_default(|s| s.iter().cloned().collect()),
            _ => Vec::new(),
        }
    };
    if candidates.is_empty() {
        return;
    }
    // The candidate sets come from `HashSet` iteration, whose order varies per
    // map instance (random hash seed). When a receiver has several candidate
    // classes that resolve differently, an order-dependent pick would make the
    // incrementally-edited buffer and a fresh open disagree on the token — so
    // sort to a stable order before selecting (see the `edit_tracking_stress`
    // incremental-vs-fresh parity tests).
    candidates.sort_unstable();
    candidates.dedup();
    // 1. Registry-modelled class — precise, declared method options.
    if let Some(method_sub) = candidates.iter().find_map(|cls| {
        let package_version = registry
            .get(cls)
            .and_then(|spec| document_floor.and_then(|floor| floor.for_spec(spec)));
        registry.instance_method_at(cls, method, package_version, dialect)
    }) {
        mark_method_word(seg, overrides);
        insert_registry_method_options(seg, method_sub, overrides);
        return;
    }
    // 2. User-defined class — resolve the method through the class hierarchy
    //    (workspace-wide when a project index is supplied, so a class defined in
    //    another file resolves too); for a configurable receiver, colour the
    //    generated accessor's property options.
    if let Some(hierarchy) = classes
        && let Some(cls) = candidates
            .iter()
            .find(|c| user_class_provides_method(hierarchy, registry, c, method))
    {
        mark_method_word(seg, overrides);
        insert_user_configure_options(seg, hierarchy, registry, cls, method, overrides);
    }
}

/// A proved registered instance (a Tk widget path or a handle variable bound
/// by its constructor) takes its method from the factory's own descriptor
/// under the document floor, ahead of any name-keyed class candidates.
fn insert_registered_instance_method(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) -> bool {
    let (Some(analysis), Some(head)) = (ctx.analysis, seg.argv.first()) else {
        return false;
    };
    let Some(words) =
        crate::registered_instance::words_at(ctx.full_source, analysis, head.span.start())
    else {
        return false;
    };
    let Some((_spec, method)) =
        crate::registered_instance::selected_method(analysis, &words, ctx.dialect)
    else {
        return false;
    };
    mark_method_word(seg, overrides);
    insert_registry_method_options(seg, method, overrides);
    true
}

/// Highlight a dispatched object method's name word (`seg.argv[1]`) as a
/// [`TokenKind::Method`].
///
/// The *call site* of a method (`$obj add …`, `my Cleanup`, `[Class new] m …`)
/// is the same entity as its declaration, so it takes the same type — a method
/// is not a free procedure.  This is the one place a dispatched
/// method name is typed, so declaration and call site cannot drift apart.
pub(super) fn mark_method_word(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if let Some(mtok) = seg.argv.get(1) {
        overrides
            .entry(mtok.span.start())
            .or_insert(ArgOverride::Kind(TokenKind::Method));
    }
}

/// Apply a registry object-method's declared options to the words after the
/// method (`skip(2)`): the switch → [`ArgOverride::Decorator`], a closed-set
/// value → [`TokenKind::EnumMember`], else [`TokenKind::OptionValue`].
fn insert_registry_method_options(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    method_sub: &tcl_registry::SubCommand,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    for (i, text) in seg.texts.iter().enumerate().skip(2) {
        // registry-axis-ok: irreducible — generic across every object
        // method's option set; only `switch` and `regexp` have
        // `OptionEffectKind::EndsOptions` populated on a "--" row today, so
        // reading `method_sub`'s own row here would stop recognising "--"
        // for virtually every other method (`formatting/keywords.rs`'s
        // `scan_options` has the same gap, for the same reason); until never
        if text == "--" {
            // End-of-options marker — colour it, then stop (Tcl convention).
            if let Some(tok) = seg.argv.get(i) {
                overrides
                    .entry(tok.span.start())
                    .or_insert(ArgOverride::Decorator);
            }
            break;
        }
        if !text.starts_with('-') {
            continue;
        }
        let Some(opt) = method_sub.options.iter().find(|o| o.matches(text)) else {
            continue;
        };
        if let Some(tok) = seg.argv.get(i) {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::Decorator);
        }
        // Colour the option's value word(s).  A value whose role is claimed by
        // a later token pass is left alone (parity with the command path).
        if opt.takes_value() && !role_claimed_by_token_pass(opt.value_role()) {
            let values = opt.value_values();
            for vi in opt.value_indices(&seg.texts, i) {
                let (Some(val_tok), Some(val_text)) = (seg.argv.get(vi), seg.texts.get(vi)) else {
                    continue;
                };
                if !matches!(val_tok.kind, TokenType::Esc | TokenType::Str) {
                    continue;
                }
                let kind =
                    if !values.is_empty() && values.iter().any(|v| v.value == val_text.as_str()) {
                        TokenKind::EnumMember
                    } else {
                        TokenKind::OptionValue
                    };
                overrides
                    .entry(val_tok.span.start())
                    .or_insert(ArgOverride::Kind(kind));
            }
        }
    }
}

/// Whether a *user-defined* class provides `method` for an instance dispatch:
/// the class hierarchy's MRO resolves it (a declared method on the class or an
/// ancestor), it is an `TclOO` builtin every instance answers (`destroy`), or
/// it is one the class system generates from the class's declared properties
/// — decided by the same registry-driven
/// [`ClassHierarchy::property_accessor_methods`] the analyser's W308
/// existence check uses, so colouring and diagnostics cannot disagree about
/// whether a method exists.  `hierarchy` is the local file's hierarchy or a
/// workspace-merged project index.
fn user_class_provides_method(
    hierarchy: &ClassHierarchy,
    registry: &CommandRegistry,
    class: &str,
    method: &str,
) -> bool {
    if hierarchy.method_target(class, method).is_some() {
        return true;
    }
    // registry-axis-ok: irreducible — `oo::object`'s own `SUBCOMMANDS` doc
    // comment spells out why this is `destroy` alone: it is "the [only]
    // exported method" reachable through an instance's public name (`new`
    // and `create` are class-level, not instance methods, despite being
    // registered on the same command); no trait yet distinguishes
    // instance-exported from class-level registry subcommands generically,
    // so asking that query would take a new registry concept, not a
    // mechanical read of an existing one; until never
    if method == "destroy" {
        return true;
    }
    hierarchy.is_property_accessor(Some(registry), class, method)
}

/// Colour the `-property` options of a generated property-accessor dispatch
/// (`oo::configurable`'s `configure`) on a user class: a word `-<name>` whose
/// `name` is a property declared on the class or an ancestor becomes a
/// [`ArgOverride::Decorator`], and a following literal value an
/// [`TokenKind::OptionValue`].  A non-property `-word` is left to the generic
/// option fallback.  No-op for any method the class system does not generate
/// from properties.
pub(super) fn insert_user_configure_options(
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    hierarchy: &ClassHierarchy,
    registry: &CommandRegistry,
    class: &str,
    method: &str,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    if !hierarchy.is_property_accessor(Some(registry), class, method) {
        return;
    }
    // Property names across the whole MRO (`-node`, `-name`, inherited …).
    let props: std::collections::HashSet<String> = hierarchy
        .mro_or_self(class)
        .iter()
        .filter_map(|c| hierarchy.classes.get(c))
        .flat_map(|cd| cd.properties.keys().cloned())
        .collect();
    if props.is_empty() {
        return;
    }
    for (i, text) in seg.texts.iter().enumerate().skip(2) {
        // registry-axis-ok: irreducible — `props` (above) is derived
        // entirely from the class's own declared properties, not from any
        // registered `OptionSpec` row — a generated property accessor has no
        // command-level "--" fact in the registry to read at all; until never
        if text == "--" {
            if let Some(tok) = seg.argv.get(i) {
                overrides
                    .entry(tok.span.start())
                    .or_insert(ArgOverride::Decorator);
            }
            break;
        }
        let Some(prop) = text.strip_prefix('-') else {
            continue;
        };
        if !props.contains(prop) {
            continue;
        }
        if let Some(tok) = seg.argv.get(i) {
            overrides
                .entry(tok.span.start())
                .or_insert(ArgOverride::Decorator);
        }
        // The immediately-following literal word is this property's value.
        let next_word = seg.texts.get(i + 1).map(String::as_str);
        // registry-axis-ok: irreducible — same reason as this loop's other
        // "--" check above; until never
        let next_word_is_a_value = next_word.is_some_and(|w| !w.starts_with('-') && w != "--");
        if let Some(val_tok) = seg.argv.get(i + 1)
            && matches!(val_tok.kind, TokenType::Esc | TokenType::Str)
            && next_word_is_a_value
        {
            overrides
                .entry(val_tok.span.start())
                .or_insert(ArgOverride::Kind(TokenKind::OptionValue));
        }
    }
}

/// Project a complete original variable spelling to its metadata-map name.
/// Supplied configuration validates the whole reference before decoration is
/// removed; the caller separately requires a sole substitution in its word.
/// An absent configuration is the explicit standalone decoration convention.
pub(super) fn object_handle_name(
    head_text: &str,
    config: Option<tcl_lexer::LexerConfig>,
) -> Option<&str> {
    if !head_text.starts_with('$') {
        return None;
    }
    if let Some(config) = config {
        let reference =
            tcl_lexer::word_parts::whole_var_ref(head_text.as_bytes(), config).ok()??;
        if reference.next != head_text.len() {
            return None;
        }
        return Some(tcl_syntax::naming::var_reference_for_style(
            head_text,
            config.braced_var,
        ));
    }
    // Explicit standalone decoration metadata retains its prior convention.
    Some(tcl_syntax::naming::var_reference(head_text))
}

/// Resolve a class name *as written* at a definer head to a qualified key in
/// `hierarchy` through retained root source receipts. Missing or ambiguous
/// byte geometry withdraws the class identity.
pub(super) fn resolve_class_in_hierarchy(hierarchy: &ClassHierarchy, name: &str) -> Option<String> {
    tcl_compiler::analyser::class_hierarchy::resolve_written_class_name(name, &hierarchy.classes)
}

/// Resolve a self-call inside a class body against the enclosing class's MRO:
/// colour the method a callable, and — for `configure` / `cget` on an
/// `oo::configurable` class — its `-property` options.  The self-receiver is
/// `my` (`TclOO`), `[self]`/`[self object]` (`TclOO`), `$self`
/// (snit), or `$this` (itcl) — each of which dispatches on the enclosing
/// object.  No-op outside a class body, without a hierarchy, or for any
/// other head.
pub(super) fn insert_self_method_overrides(
    ctx: ScriptCtx<'_>,
    seg: &tcl_compiler::segmenter::SegmentedCommand,
    original: Option<&crate::original_invocation::OriginalRegistryWords>,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    let (Some(hierarchy), Some(class_name), Some(head)) =
        (ctx.classes, ctx.enclosing_class, seg.argv.first())
    else {
        return;
    };
    let selected_self = original.and_then(|words| {
        words.with_source_schema(ctx.generation, |schema| {
            schema
                .semantics
                .traits
                .contains(tcl_registry::Traits::TCLOO_SELF_DISPATCH)
        })
    }) == Some(true);
    let method_index = if selected_self {
        let Some(tcl_compiler::registry_invocation::InvocationWordOrigin::Written(written)) =
            original.and_then(|words| words.origins.as_slice().get(1))
        else {
            return;
        };
        *written
    } else {
        let original_words = tcl_compiler::ir::CommandTokens::from_segmented(
            &tcl_lexer::SourceMap::new(ctx.full_source),
            ctx.config,
            seg,
        );
        let implicit_self = original_words
            .word_exprs
            .first()
            .and_then(tcl_compiler::ir::WordExpr::sole_variable_substitution)
            .and_then(|(spelling, _)| object_handle_name(spelling, Some(ctx.config)))
            .is_some_and(|name| {
                ctx.receiver_grammar
                    .is_some_and(|grammar| grammar.is_implicit_receiver_variable(name))
            });
        if !implicit_self && !original_self_accessor(ctx, *head) {
            return;
        }
        1
    };
    let Some(method) = seg.texts.get(method_index) else {
        return;
    };
    let Some(class) = resolve_class_in_hierarchy(hierarchy, class_name) else {
        return;
    };
    if !user_class_provides_method(hierarchy, ctx.registry, &class, method) {
        return;
    }
    if let Some(word) = seg.argv.get(method_index) {
        overrides.insert(word.span.start(), ArgOverride::Kind(TokenKind::Method));
    }
    // This presentation helper addresses direct written selector/options only.
    // Captures never borrow its positional argument coordinates.
    if method_index == 1
        && original.is_none_or(|words| {
            words
                .origins
                .as_slice()
                .iter()
                .enumerate()
                .all(|(ordinal, origin)| {
                    *origin
                        == tcl_compiler::registry_invocation::InvocationWordOrigin::Written(ordinal)
                })
        })
    {
        insert_user_configure_options(seg, hierarchy, ctx.registry, &class, method, overrides);
    }
}

/// A single genuine receiver substitution under the current original schema.
/// This is conditional source syntax, not the result of executing the child.
fn original_self_accessor(ctx: ScriptCtx<'_>, head: Token) -> bool {
    let Some(analysis) = ctx.analysis else {
        return false;
    };
    let regions =
        crate::executable_regions::command_substitution_regions(ctx.full_source, ctx.config, head);
    let [(start, end)] = regions.as_slice() else {
        return false;
    };
    let Ok(offset) = u32::try_from(*start) else {
        return false;
    };
    let commands =
        segment_commands_with_offset_and_config(&ctx.full_source[*start..*end], offset, ctx.config);
    let [command] = commands.as_slice() else {
        return false;
    };
    let Some(words) = tcl_compiler::registry_invocation::source_structure::source_registry_words(
        ctx.full_source,
        analysis,
        command,
    ) else {
        return false;
    };
    words.with_source_schema(ctx.generation, |schema| {
        let spec = schema.authored_source_descriptors().command;
        if spec.self_receiver_words.is_empty() {
            return false;
        }
        match schema.words.arguments().exact_argv_len() {
            Some(0) => spec.arity.min == 0,
            Some(1) => schema
                .words
                .arguments()
                .get(0)
                .and_then(tcl_registry::InvocationWord::literal)
                .is_some_and(|word| spec.self_receiver_words.contains(&word)),
            _ => false,
        }
    }) == Some(true)
}

/// Paint literal name fragments only under the caller's retained variable role.
/// Original word geometry supplies display extents, independently of runtime
/// names, variable places, cells or successful argument evaluation.
pub(super) fn retag_original_variable_argument(
    command: &tcl_compiler::segmenter::SegmentedCommand,
    original: &tcl_lexer::NativeWord,
    kind: ArgOverride,
    overrides: &mut FxHashMap<u32, ArgOverride>,
) {
    use tcl_syntax::word_rules::OriginalVariableNameWordForm;
    if !original
        .tokens()
        .iter()
        .all(|token| command.all_tokens.contains(token))
    {
        return;
    }
    let Some(geometry) = tcl_syntax::word_rules::original_variable_name_word_geometry(original)
    else {
        return;
    };
    if matches!(geometry.form(), OriginalVariableNameWordForm::Dynamic) {
        return;
    }
    if original.group().kind == tcl_lexer::WordKind::Quoted
        || original.tokens().iter().any(|token| token.in_quote)
    {
        if matches!(geometry.form(), OriginalVariableNameWordForm::Static { .. })
            && let [token] = original.tokens()
            && token.kind == TokenType::Esc
        {
            overrides.entry(token.span.start()).or_insert(kind);
        }
        return;
    }
    for token in geometry.original_word().tokens() {
        let literal = token.kind == TokenType::Esc
            && geometry
                .literal_spans()
                .iter()
                .any(|span| span.start() <= token.span.start() && token.span.end() <= span.end());
        let braced = token.kind == TokenType::Str
            && matches!(geometry.form(), OriginalVariableNameWordForm::Static { .. });
        if literal || braced {
            overrides
                .entry(token.span.start())
                .or_insert_with(|| kind.clone());
        }
    }
}
