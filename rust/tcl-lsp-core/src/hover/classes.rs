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

//! Cards for classes: hierarchy details, properties and counted class sources.

use super::Hover;
use tcl_compiler::analyser::{AnalysisResult, ClassDef};

fn append_class_hierarchy_details(
    analysis: &AnalysisResult,
    class_def: &ClassDef,
    details: &mut Vec<String>,
) {
    // MRO chain + direct subclasses from the class hierarchy — surfaces
    // the inheritance shape inline. Only shown when non-trivial.
    if let Some(source_name) = &class_def.source_name {
        let mut records = analysis.original_class_declarations().filter(|record| {
            record.name() == source_name && record.metadata().name_span == class_def.name_span
        });
        if let Some(record) = records.next().filter(|_| records.next().is_none()) {
            use tcl_compiler::analyser::class_hierarchy::original_metadata::original_instance_metadata_order;
            if let Some(order) =
                original_instance_metadata_order(analysis, record).filter(|order| order.len() > 1)
            {
                details.push(format!(
                    "**MRO**: {}",
                    order
                        .iter()
                        .map(|provider| provider.metadata().qualified_name.as_str())
                        .collect::<Vec<_>>()
                        .join(" → ")
                ));
            }
            let mut subclasses = analysis
                .original_class_declarations()
                .filter_map(|candidate| {
                    use tcl_compiler::analyser::types::{
                        MemberSide, OriginalSourceClassRelationKind,
                    };
                    let parents = candidate.metadata().original_relations.resolve(
                        MemberSide::Instance,
                        OriginalSourceClassRelationKind::Superclass,
                        |relation| {
                            let selected = relation.lookup().first_matching_publications(
                                analysis
                                    .original_class_declarations()
                                    .map(|record| (record.name(), record)),
                            );
                            match selected.as_slice() {
                                [record] => Some(*record),
                                _ => None,
                            }
                        },
                    )?;
                    parents
                        .iter()
                        .any(|parent| parent.declaration_site() == record.declaration_site())
                        .then_some(candidate.metadata().qualified_name.as_str())
                })
                .collect::<Vec<_>>();
            subclasses.sort_unstable();
            if !subclasses.is_empty() {
                details.push(format!("**Subclasses**: {}", subclasses.join(", ")));
            }
        }
    } else if analysis.allows_lexical_declaration_advice() {
        let hierarchy = analysis.class_hierarchy();
        let qname = &class_def.qualified_name;
        if let Some(mro) = hierarchy.mro_map.get(qname).filter(|mro| mro.len() > 1) {
            details.push(format!("**MRO**: {}", mro.join(" → ")));
        }
        if let Some(subs) = hierarchy
            .subclasses
            .get(qname)
            .filter(|subs| !subs.is_empty())
        {
            let mut names = subs.iter().map(String::as_str).collect::<Vec<_>>();
            names.sort_unstable();
            details.push(format!("**Subclasses**: {}", names.join(", ")));
        }
    }
}

pub(super) fn positioned_class_hover(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Option<Hover> {
    crate::original_oo::positioned_class_at(source, analysis, cursor)
        .map(|class| Hover::markdown(class_hover_text(analysis, class.metadata())))
        .or_else(|| relation_class_hover(source, analysis, cursor))
        .or_else(|| counted_class_source_hover(source, analysis, cursor))
}

/// A `superclass` / `mixin` word hovers as the class its own recorded lookup
/// selects among the document's class declarations (current namespace, then
/// global). Display only.
fn relation_class_hover(source: &str, analysis: &AnalysisResult, cursor: u32) -> Option<Hover> {
    let relation = crate::original_oo::relation_word_at(source, analysis, cursor)?;
    let selected = relation.lookup().first_matching_publications(
        analysis
            .original_class_declarations()
            .map(|record| (record.name(), record)),
    );
    let [class] = selected.as_slice() else {
        return None;
    };
    Some(Hover::markdown(class_hover_text(
        analysis,
        class.metadata(),
    )))
}

pub(super) fn original_property_source_hover(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> std::ops::ControlFlow<Option<Hover>> {
    use std::ops::ControlFlow;
    let ControlFlow::Break(Some(identity)) =
        crate::original_declaration::select_at_offset("", source, analysis, cursor)
    else {
        return ControlFlow::Continue(());
    };
    if !(matches!(
        identity.role(),
        crate::original_declaration::OriginalDeclarationRole::Property(_)
    ) && identity.span().start() <= cursor
        && cursor < identity.span().end())
    {
        return ControlFlow::Continue(());
    }
    let Some(class) = identity.class_metadata(analysis) else {
        return ControlFlow::Break(None);
    };
    let mut properties = class
        .metadata()
        .original_properties
        .declarations()
        .filter(|property| {
            property.matches_original_declaration_source(source, analysis, class)
                && crate::original_declaration::OriginalDeclarationIdentity::for_property(
                    "", source, analysis, class, property,
                )
                .as_ref()
                    == Some(&identity)
        });
    let Some(property) = properties.next() else {
        return ControlFlow::Break(None);
    };
    if properties.next().is_some() {
        return ControlFlow::Break(None);
    }
    let mut label = tcl_syntax::naming::native_command_full_name_bytes(class.name().slot());
    label.extend_from_slice(b"::");
    label.extend_from_slice(property.original_name_input().bytes());
    let label = tcl_syntax::native_string::resident_name_label(&label);
    let longest = label
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    ControlFlow::Break(Some(Hover::markdown(format!(
        "**property**\n\n{fence}text\n{label}\n{fence}\n\nSource declaration; property availability depends on execution."
    ))))
}

pub(super) fn counted_initial_method_hover(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Option<Hover> {
    let config = analysis.body_lexer_config?;
    let image = tcl_lexer::SourceImage::document(source);
    analysis
        .matches_original_source_image(&image, config)
        .then_some(())?;
    let context = analysis.resolved_input.as_ref()?.context_registry();
    let image = &image;
    let mut declarations = analysis
        .original_counted_class_metadata
        .iter()
        .flat_map(|class| {
            let body = class.metadata().body();
            let current = class.name_input().source_image() == image
                && class.name_input().lexer_config() == config
                && body.matches_source(image, config)
                && body.matches_context(&context);
            class
                .metadata()
                .initial_method_declarations()
                .unwrap_or_default()
                .iter()
                .filter(move |method| {
                    let key = method.original_name_input();
                    let word = method.original_word();
                    let site = method.site();
                    let body_span = body.content_span();
                    current
                        && key.original_word() == word
                        && key.source_image() == image
                        && key.lexer_config() == config
                        && key.policy() == class.name().policy()
                        && !word.group().expand
                        && site.source == class.declaration_site().source
                        && body_span.start() <= site.offset
                        && site.offset <= word.span().start()
                        && word.span().end() <= body_span.end()
                        && key.span().start() <= cursor
                        && cursor < key.span().end()
                })
        });
    let declaration = declarations.next()?;
    declarations.next().is_none().then_some(())?;
    let word = declaration.original_word().try_text().ok()?;
    let longest = word
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    Some(Hover::markdown(format!(
        "**Method source declaration**\n\n{fence}tcl\n{word}\n{fence}\n\nMethod availability depends on execution."
    )))
}

fn counted_class_source_hover(
    source: &str,
    analysis: &AnalysisResult,
    cursor: u32,
) -> Option<Hover> {
    let mut rows = analysis.command_invocations.iter().filter(|invocation| {
        invocation.lookup.is_execution_site()
            && invocation.range.start() <= cursor
            && cursor < invocation.range.end()
    });
    let invocation = rows.next()?;
    rows.next().is_none().then_some(())?;
    let record = crate::original_oo::counted_class_for_invocation(source, analysis, invocation)?;
    let key = record.name_input();
    let name = record.name();
    let label = tcl_syntax::naming::native_command_source_word(
        name.policy().recipe(),
        name.slot(),
        key.source_image().channel(),
        key.lexer_config(),
    )?;
    let longest = label
        .split(|character| character != '`')
        .map(str::len)
        .max()
        .unwrap_or(0);
    let fence = "`".repeat(longest.max(2) + 1);
    Some(Hover::markdown(format!(
        "**Class declaration**\n\n{fence}tcl\n{label}\n{fence}\n\nClass availability depends on execution."
    )))
}

pub(super) fn class_hover_text(analysis: &AnalysisResult, class_def: &ClassDef) -> String {
    let original_word = class_def.source_name.as_ref().and_then(|name| {
        let mut records = analysis.original_class_declarations().filter(|record| {
            record.name() == name && record.metadata().name_span == class_def.name_span
        });
        let record = records.next()?;
        if records.next().is_some() || record.name_input().display().is_some() {
            return None;
        }
        tcl_syntax::naming::native_command_source_word(
            name.policy().recipe(),
            name.slot(),
            record.name_input().source_image().channel(),
            record.name_input().lexer_config(),
        )
    });
    let mut sig = format!(
        "{} create {}",
        class_def.metaclass,
        original_word
            .as_deref()
            .unwrap_or(&class_def.qualified_name)
    );
    if !class_def.superclasses.is_empty() {
        use std::fmt::Write as _;
        let _ = write!(sig, " (superclass: {})", class_def.superclasses.join(", "));
    }
    if !class_def.mixins.is_empty() {
        use std::fmt::Write as _;
        let _ = write!(sig, " (mixin: {})", class_def.mixins.join(", "));
    }
    let mut parts = vec![format!("```tcl\n{sig}\n```")];
    let mut details: Vec<String> = Vec::new();
    for (side, title) in [
        (
            tcl_compiler::analyser::types::MemberSide::Instance,
            "Methods",
        ),
        (
            tcl_compiler::analyser::types::MemberSide::ClassObject,
            "Class methods",
        ),
    ] {
        let names = if class_def.source_name.is_some() {
            crate::original_oo::own_method_labels(class_def, side)
        } else if analysis.allows_lexical_declaration_advice() {
            let table = match side {
                tcl_compiler::analyser::types::MemberSide::Instance => &class_def.methods,
                tcl_compiler::analyser::types::MemberSide::ClassObject => &class_def.class_methods,
            };
            let mut names = table.keys().cloned().collect::<Vec<_>>();
            names.sort();
            Some(names)
        } else {
            None
        };
        if let Some(names) = names.filter(|names| !names.is_empty()) {
            details.push(format!("**{title}**: {}", names.join(", ")));
        }
    }
    if !class_def.variables.is_empty() {
        details.push(format!(
            "**Instance variables**: {}",
            class_def.variables.join(", ")
        ));
    }
    append_class_hierarchy_details(analysis, class_def, &mut details);
    if !details.is_empty() {
        parts.push(details.join("  \n"));
    }
    if !class_def.doc.is_empty() {
        parts.push(class_def.doc.clone());
    }
    parts.join("\n\n")
}
