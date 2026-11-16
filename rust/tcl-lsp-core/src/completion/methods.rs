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

//! Method candidates for class commands, object instances and registry-modelled classes.

use super::{
    CompletionEdit, CompletionItem, CompletionKind, FilteredCandidates, decorate_fuzzy_items,
    filter_candidates, insertable_source_word,
};
use rustc_hash::FxHashSet;
use tcl_compiler::analyser::AnalysisResult;
use tcl_registry::CommandRegistry;

/// Class-command declaration candidates. This does not select an executed
/// method provider or model a maker/default delegate's native attachment.
pub(super) fn original_class_method_items(
    analysis: &AnalysisResult,
    root: &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
        tcl_compiler::analyser::ClassDef,
    >,
    partial: &str,
) -> Vec<CompletionItem> {
    original_method_candidate_items(
        analysis,
        root,
        partial,
        tcl_compiler::analyser::types::MemberSide::ClassObject,
    )
}

pub(super) fn original_method_candidate_items(
    analysis: &AnalysisResult,
    root: &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
        tcl_compiler::analyser::ClassDef,
    >,
    partial: &str,
    side: tcl_compiler::analyser::types::MemberSide,
) -> Vec<CompletionItem> {
    original_method_candidate_items_at(analysis, root, partial, side, None)
}

pub(super) fn original_instance_method_candidate_items_at(
    analysis: &AnalysisResult,
    root: &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
        tcl_compiler::analyser::ClassDef,
    >,
    source: &str,
    cursor: u32,
    partial: &str,
    edit_context: Option<(&str, u32, u32)>,
) -> Vec<CompletionItem> {
    let own = match crate::original_oo::own_object_completion_methods(analysis, source, cursor) {
        std::ops::ControlFlow::Break(Some(own)) => own,
        std::ops::ControlFlow::Break(None) => return Vec::new(),
        std::ops::ControlFlow::Continue(()) => Vec::new(),
    };
    let receiver = edit_context
        .and_then(|_| crate::original_oo::instance_completion_source(analysis, source, cursor));
    if receiver
        .as_ref()
        .is_some_and(|receiver| receiver.class.declaration_site() != root.declaration_site())
    {
        return Vec::new();
    }
    rank_original_method_candidates(
        partial,
        original_method_candidate_items_with_own_at(
            analysis,
            root,
            "",
            tcl_compiler::analyser::types::MemberSide::Instance,
            OriginalMethodEditContext {
                position: edit_context,
                receiver: receiver.as_ref(),
            },
            &own,
        ),
    )
}

pub(super) fn original_method_candidate_items_at(
    analysis: &AnalysisResult,
    root: &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
        tcl_compiler::analyser::ClassDef,
    >,
    partial: &str,
    side: tcl_compiler::analyser::types::MemberSide,
    edit_context: Option<(&str, u32, u32)>,
) -> Vec<CompletionItem> {
    rank_original_method_candidates(
        partial,
        original_method_candidate_items_with_own_at(
            analysis,
            root,
            "",
            side,
            OriginalMethodEditContext {
                position: edit_context,
                receiver: None,
            },
            &[],
        ),
    )
}

/// Rank only the authenticated source method universe. Matching insertion
/// spellings retains escaped-name prefixes; shared fuzzy ranking is UI advice.
fn rank_original_method_candidates(
    partial: &str,
    items: Vec<CompletionItem>,
) -> Vec<CompletionItem> {
    let FilteredCandidates {
        mut candidates,
        fuzzy,
    } = filter_candidates(partial, items, |item| {
        if item.insert_text.starts_with(partial) {
            item.insert_text.as_str()
        } else {
            item.label.as_str()
        }
    });
    if fuzzy {
        decorate_fuzzy_items(&mut candidates, partial);
    }
    candidates
}

#[derive(Clone, Copy)]
struct OriginalMethodEditContext<'a, 'b> {
    position: Option<(&'a str, u32, u32)>,
    receiver: Option<&'a crate::original_oo::instance_completion::InstanceCompletionSource<'b>>,
}

fn original_method_candidate_items_with_own_at(
    analysis: &AnalysisResult,
    root: &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
        tcl_compiler::analyser::ClassDef,
    >,
    partial: &str,
    side: tcl_compiler::analyser::types::MemberSide,
    edit_context: OriginalMethodEditContext<'_, '_>,
    own: &[crate::original_oo::OwnObjectMethodCandidate<'_>],
) -> Vec<CompletionItem> {
    use tcl_compiler::analyser::class_hierarchy::original_metadata::original_instance_metadata_order;
    use tcl_compiler::analyser::types::MemberSide;
    let providers = original_instance_metadata_order(analysis, root).unwrap_or_else(|| vec![root]);
    if side == MemberSide::Instance
        && providers.iter().any(|provider| {
            !provider
                .metadata()
                .original_properties
                .side_matches_producer_input(side, analysis)
        })
    {
        return Vec::new();
    }
    let mut names = Vec::<(
        tcl_core_types::NameBytes,
        tcl_syntax::naming::NamePolicyProtocol,
    )>::new();
    let mut items = Vec::new();
    extend_original_own_method_items(analysis, partial, edit_context, own, &mut names, &mut items);
    for provider in providers {
        for method in provider
            .metadata()
            .original_members
            .methods(side)
            .into_iter()
            .flatten()
        {
            if !method.exported()
                || (side == MemberSide::ClassObject
                    && provider.declaration_site() != root.declaration_site()
                    && match method.name_purpose() {
                        tcl_registry::definer::DefinitionMemberNamePurpose::TclOoMethod => {
                            !method.native_class_delegate()
                        }
                        tcl_registry::definer::DefinitionMemberNamePurpose::SourceValue => {
                            method.metadata().is_self_method
                        }
                    })
            {
                continue;
            }
            let input = method.original_name_input();
            let key = (
                tcl_core_types::NameBytes::from(input.bytes()),
                input.policy(),
            );
            if names.contains(&key) {
                continue;
            }
            let Some(spelling) = crate::original_oo::method_word(&method) else {
                continue;
            };
            let label = crate::original_oo::method_label(&method);
            if !label.starts_with(partial) && !spelling.starts_with(partial) {
                continue;
            }
            let Some(item) = original_method_candidate_item(
                analysis,
                input.bytes(),
                input.policy(),
                label,
                spelling,
                edit_context,
                "method — source declaration candidate",
            ) else {
                continue;
            };
            names.push(key);
            items.push(item);
        }
        extend_original_property_method_items(
            analysis,
            provider,
            side,
            partial,
            edit_context,
            &mut names,
            &mut items,
        );
    }
    items.sort_by(|left, right| left.label.cmp(&right.label));
    items
}

fn original_method_candidate_item(
    analysis: &AnalysisResult,
    bytes: &[u8],
    policy: tcl_syntax::naming::NamePolicyProtocol,
    label: String,
    spelling: String,
    edit_context: OriginalMethodEditContext<'_, '_>,
    detail: &str,
) -> Option<CompletionItem> {
    let text_edit = if let Some((source, cursor, line)) = edit_context.position {
        let (span, new_text) = if let Some(receiver) = edit_context.receiver {
            receiver.selector_replacement(source, cursor, bytes, policy)?
        } else {
            crate::original_oo::method_selector_replacement(
                analysis, source, cursor, bytes, policy,
            )?
        };
        let index = tcl_lexer::LineIndex::new(source);
        let start = index.position_at_utf16(span.start(), source);
        let end = index.position_at_utf16(span.end(), source);
        if start.line != line || end.line != line {
            return None;
        }
        Some(CompletionEdit {
            start_char: start.character.get(),
            end_char: end.character.get(),
            new_text,
        })
    } else {
        None
    };
    Some(CompletionItem {
        label,
        insert_text: spelling,
        text_edit,
        kind: CompletionKind::Function,
        detail: Some(detail.to_owned()),
        ..CompletionItem::default()
    })
}

pub(super) fn method_completions(
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    class_q: &str,
    bucket: crate::definition::MethodBucket,
    partial: &str,
    object_state: Option<&tcl_compiler::analyser::ObjectMemberState>,
    profile: &'static tcl_dialect::DialectProfile,
) -> Option<Vec<CompletionItem>> {
    let all = method_items(
        analysis,
        Some(registry),
        class_q,
        bucket,
        object_state,
        profile,
    )?;
    let FilteredCandidates {
        candidates: items,
        fuzzy,
    } = filter_candidates(partial, all, |item| item.label.as_str());
    (!fuzzy && !items.is_empty()).then_some(items)
}

/// Readonly method suggestions preserve source side, provider order and visibility.
/// Named source members require their current selected original name operand;
/// registry, generated and per-object suggestions retain separate producers.
pub(super) fn method_items(
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    class_q: &str,
    bucket: crate::definition::MethodBucket,
    object_state: Option<&tcl_compiler::analyser::ObjectMemberState>,
    profile: &'static tcl_dialect::DialectProfile,
) -> Option<Vec<CompletionItem>> {
    use crate::definition::MethodBucket;
    let object_members = if bucket == MethodBucket::Instance {
        match object_state {
            Some(state) => Some(analysis.logical_source_object_member_state(state)?),
            None => None,
        }
    } else {
        None
    };
    if !analysis.all_classes.contains_key(class_q) {
        // An external catalogue cannot supply the source object's own masks.
        if object_members.is_some() {
            return None;
        }
        return registry_method_items(analysis, registry?, class_q, profile);
    }
    let exports = object_members
        .as_ref()
        .into_iter()
        .flat_map(tcl_compiler::analyser::LogicalSourceObjectMembers::export_names)
        .collect::<FxHashSet<_>>();
    let unexports = object_members
        .as_ref()
        .into_iter()
        .flat_map(tcl_compiler::analyser::LogicalSourceObjectMembers::unexport_names)
        .collect::<FxHashSet<_>>();
    let hierarchy = analysis.class_hierarchy();
    let mro = hierarchy
        .mro_map
        .get(class_q)
        .cloned()
        .unwrap_or_else(|| vec![class_q.to_string()]);
    let mut seen: FxHashSet<String> = FxHashSet::default();
    let mut items: Vec<CompletionItem> = Vec::new();
    // Per-object members first — `TclOO` layers an `oo::objdefine`d method
    // ahead of the class chain, so the receiver's own members claim their
    // names before any class provider.
    if bucket == MethodBucket::Instance
        && let (Some(state), Some(members)) = (object_state, object_members.as_ref())
    {
        layer_per_object_members(state, members, &mut seen, &mut items)?;
    }
    for cls in &mro {
        let Some(cd) = analysis.all_classes.get(cls) else {
            continue;
        };
        let bucketed: Vec<(&String, &tcl_compiler::analyser::MethodDef)> = match bucket {
            MethodBucket::Instance => cd.methods.iter().collect(),
            MethodBucket::Class => cd
                .class_methods
                .iter()
                .filter(|(_, m)| cls.as_str() == class_q || !m.is_self_method)
                .collect(),
        };
        // Per-object visibility flips override the declared state for this
        // receiver: an `oo::objdefine … export` revives an unexported class
        // member, an `… unexport` masks an exported one.
        let flipped = |name: &str, declared_public: bool| {
            if exports.contains(name) {
                true
            } else if unexports.contains(name) {
                false
            } else {
                declared_public
            }
        };
        let side = match bucket {
            MethodBucket::Instance => tcl_compiler::analyser::MemberSide::Instance,
            MethodBucket::Class => tcl_compiler::analyser::MemberSide::ClassObject,
        };
        let mut methods = bucketed
            .into_iter()
            .filter(|(name, method)| flipped(name, method.visibility == "public"))
            .filter_map(|(_, method)| analysis.logical_source_member_insertion(cd, side, method))
            .collect::<Vec<_>>();
        methods.sort_by(|left, right| left.name().cmp(right.name()));
        for method in methods {
            let name = method.name();
            let provider = cls.as_str();
            if !seen.insert(name.to_owned()) {
                continue;
            }
            let detail = if provider == class_q {
                format!("method — {class_q}")
            } else {
                format!("method — inherited from {provider}")
            };
            items.push(CompletionItem {
                label: name.to_owned(),
                insert_text: method.source_word().to_owned(),
                kind: CompletionKind::Function,
                detail: Some(detail),
                ..CompletionItem::default()
            });
        }
    }
    extend_generated_method_items(analysis, registry, class_q, bucket, &mut seen, &mut items);
    items.sort_by(|a, b| a.label.cmp(&b.label));
    Some(items)
}

/// Own names and visibility masks require the same current original source state.
fn layer_per_object_members(
    state: &tcl_compiler::analyser::ObjectMemberState,
    members: &tcl_compiler::analyser::LogicalSourceObjectMembers<'_>,
    seen: &mut FxHashSet<String>,
    items: &mut Vec<CompletionItem>,
) -> Option<()> {
    let mut methods = state.methods.values().collect::<Vec<_>>();
    methods.sort_by(|left, right| left.name.cmp(&right.name));
    for method in methods {
        let spelling = members.source_word(method)?;
        if method.name.contains('\0') {
            return None;
        }
        if !seen.insert(method.name.clone()) || method.visibility != "public" {
            continue;
        }
        items.push(CompletionItem {
            label: method.name.clone(),
            insert_text: spelling,
            kind: CompletionKind::Function,
            detail: Some("method — per-object (oo::objdefine)".to_owned()),
            ..CompletionItem::default()
        });
    }
    Some(())
}

/// Catalogue name data uses the actual retained source grammar, not a runtime name recipe.
pub(super) fn source_method_name_word(analysis: &AnalysisResult, name: &str) -> Option<String> {
    let input = analysis.resolved_input.as_ref()?;
    if analysis.body_lexer_config != Some(input.lexer_config()) || name.contains('\0') {
        return None;
    }
    tcl_syntax::word_rules::lexical_unicode_source_word(name, input.lexer_config())
}

pub(super) fn generated_property_method_items(
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    class_q: &str,
) -> Vec<CompletionItem> {
    let Some(registry) = registry else {
        return Vec::new();
    };
    if analysis
        .resolved_registry()
        .is_none_or(|actual| actual.snapshot().semantic_key() != registry.snapshot().semantic_key())
    {
        return Vec::new();
    }
    let Some(class) = analysis.all_classes.get(class_q) else {
        return Vec::new();
    };
    if analysis.logical_source_class_declaration(class).is_none() {
        return Vec::new();
    }
    let hierarchy = analysis.class_hierarchy();
    let providers = hierarchy
        .mro_map
        .get(class_q)
        .cloned()
        .unwrap_or_else(|| vec![class_q.to_owned()]);
    if providers
        .iter()
        .filter_map(|name| analysis.all_classes.get(name))
        .any(|provider| {
            analysis
                .logical_source_class_declaration(provider)
                .is_none()
        })
    {
        return Vec::new();
    }
    hierarchy
        .property_accessor_methods(Some(registry), class_q)
        .into_iter()
        .filter_map(|name| {
            let insert_text = source_method_name_word(analysis, name)?;
            Some(CompletionItem {
                label: name.to_owned(),
                insert_text,
                kind: CompletionKind::Function,
                detail: Some("method — generated property accessor".to_owned()),
                ..CompletionItem::default()
            })
        })
        .collect()
}

/// Every instance method/subcommand item for a *registry*-modelled class:
/// a tcllib-style factory (`report::report`, `struct::graph` — a distinct
/// `ObjectClassSpec`) or a Tk/ttk widget (self-referential — `class_q`
/// names its own `CommandSpec`, whose `subcommands` *is* its instance
/// dispatch table).  Tries the dedicated object class first since that is
/// the precise, intended shape; the self-referential widget case falls
/// back to the class's own `CommandSpec` when it has no separate
/// `ObjectClassSpec` (matching [`CommandRegistry::instance_methods`]). The
/// registry also applies the owning package's lifecycle floor before returning
/// a row, so Tk 9.1-only methods cannot leak into an older resolved profile.
/// Fragment matching happens in [`method_completions`].
pub(super) fn registry_method_items(
    analysis: &AnalysisResult,
    registry: &CommandRegistry,
    class_q: &str,
    profile: &'static tcl_dialect::DialectProfile,
) -> Option<Vec<CompletionItem>> {
    // naming.consumer.original-receiver-completion-domain
    // docs/design/analysis/name-resolution-proofs/original-receiver-completion-domain.md
    if !analysis.allows_lexical_declaration_advice() {
        return None;
    }
    let floor = crate::document_floor::DocumentFloor::new(analysis, profile);
    let context = floor.context()?;
    let spec = context.resolve_spec(registry, class_q)?;
    let methods = CommandRegistry::instance_method_providers_for_descriptor_at(
        registry,
        spec,
        Some(context.authoring_query()),
        |owner| {
            context
                .resolve_spec(registry, owner.name)
                .filter(|selected| std::ptr::eq(*selected, owner))
                .map(|selected| floor.for_spec(selected))
        },
    );
    let mut items: Vec<CompletionItem> = methods
        .into_iter()
        .filter_map(|(_owner, method)| {
            let insert_text = source_method_name_word(analysis, method.name)?;
            Some(CompletionItem {
                label: method.name.to_owned(),
                insert_text,
                kind: CompletionKind::Function,
                detail: Some(format!("method — {class_q}")),
                documentation: (!method.detail.is_empty()).then(|| method.detail.to_owned()),
                ..CompletionItem::default()
            })
        })
        .collect();
    if items.is_empty() {
        return None;
    }
    items.sort_by(|a, b| a.label.cmp(&b.label));
    Some(items)
}

fn extend_original_own_method_items(
    analysis: &AnalysisResult,
    partial: &str,
    edit_context: OriginalMethodEditContext<'_, '_>,
    own: &[crate::original_oo::OwnObjectMethodCandidate<'_>],
    names: &mut Vec<(
        tcl_core_types::NameBytes,
        tcl_syntax::naming::NamePolicyProtocol,
    )>,
    items: &mut Vec<CompletionItem>,
) {
    for candidate in own {
        let input = &candidate.input;
        // A private own name blocks the identical class candidate, even when
        // its source reporting label or insertion spelling collides.
        names.push((
            tcl_core_types::NameBytes::from(input.bytes()),
            input.policy(),
        ));
        if !candidate.exported
            || !candidate
                .metadata
                .name_purpose()
                .admits(input.policy().recipe(), input.bytes())
        {
            continue;
        }
        let declaration = candidate.metadata.declaration().original_word();
        let Some(spelling) = tcl_syntax::backslash::native_literal_source_word(
            input.bytes(),
            declaration.image().channel(),
            declaration.config(),
            input.policy().string_protocol(),
        ) else {
            continue;
        };
        let spelling = insertable_source_word(spelling, input.bytes(), declaration.config());
        let label =
            std::str::from_utf8(input.bytes()).map_or_else(|_| spelling.clone(), str::to_owned);
        if !label.starts_with(partial) && !spelling.starts_with(partial) {
            continue;
        }
        if let Some(item) = original_method_candidate_item(
            analysis,
            input.bytes(),
            input.policy(),
            label,
            spelling,
            edit_context,
            "method — per-object declaration candidate",
        ) {
            items.push(item);
        }
    }
}

fn extend_generated_method_items(
    analysis: &AnalysisResult,
    registry: Option<&CommandRegistry>,
    class_q: &str,
    bucket: crate::definition::MethodBucket,
    seen: &mut FxHashSet<String>,
    items: &mut Vec<CompletionItem>,
) {
    use crate::definition::MethodBucket;
    if bucket == MethodBucket::Instance {
        for item in generated_property_method_items(analysis, registry, class_q) {
            if seen.insert(item.label.clone()) {
                items.push(item);
            }
        }
    }
    // The universal object method (present on every instance, never on a
    // class command).
    if bucket == MethodBucket::Instance && seen.insert("destroy".to_string()) {
        items.push(CompletionItem {
            label: "destroy".to_string(),
            insert_text: "destroy".to_string(),
            kind: CompletionKind::Function,
            detail: Some("method — oo::object builtin".to_string()),
            ..CompletionItem::default()
        });
    }
}

fn extend_original_property_method_items(
    analysis: &AnalysisResult,
    provider: &tcl_compiler::signature_scan::original_name::SourceDeclarationMetadata<
        tcl_compiler::analyser::ClassDef,
    >,
    side: tcl_compiler::analyser::types::MemberSide,
    partial: &str,
    edit_context: OriginalMethodEditContext<'_, '_>,
    names: &mut Vec<(
        tcl_core_types::NameBytes,
        tcl_syntax::naming::NamePolicyProtocol,
    )>,
    items: &mut Vec<CompletionItem>,
) {
    use tcl_compiler::analyser::types::MemberSide;
    if side == MemberSide::Instance
        && let Some(properties) = provider
            .metadata()
            .original_properties
            .properties_for_analysis(side, analysis)
    {
        for property in properties {
            let key = property.declaration().name_input();
            for name in property.accessor_methods() {
                // This is selected Registry-authored metadata, separate
                // from the property's counted name and native table state.
                let identity = (
                    tcl_core_types::NameBytes::from(name.as_bytes()),
                    key.policy(),
                );
                if names.contains(&identity) || !name.starts_with(partial) {
                    continue;
                }
                let Some(spelling) = tcl_syntax::backslash::native_literal_source_word(
                    name.as_bytes(),
                    key.source_image().channel(),
                    key.lexer_config(),
                    key.policy().string_protocol(),
                ) else {
                    continue;
                };
                let spelling =
                    insertable_source_word(spelling, name.as_bytes(), key.lexer_config());
                let Some(item) = original_method_candidate_item(
                    analysis,
                    name.as_bytes(),
                    key.policy(),
                    (*name).to_owned(),
                    spelling,
                    edit_context,
                    "method — property declaration candidate",
                ) else {
                    continue;
                };
                names.push(identity);
                items.push(item);
            }
        }
    }
}
