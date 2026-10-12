// tcl-lsp — a language server and toolchain for Tcl
// Copyright (C) 2026 James Deucker (bitwisecook)
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Source outline from original naming inventories. Names are presentation of
//! retained bytes; ranges describe declarations, never execution or liveness.

use super::{
    DocumentSymbol, SymbolKind, class_member_symbols, format_param_list, merge_ranges,
    nest_by_containment, span_to_range,
};
use rustc_hash::{FxHashMap, FxHashSet};
use tcl_compiler::analyser::{AnalysisResult, ScopeKind};
use tcl_compiler::signature_scan::scope::SignatureNamespaceScope;
use tcl_lexer::{LineIndex, SourceImage};
use tcl_syntax::naming::{NamePolicyProtocol, NativeNameProtocol};

struct Node {
    symbol: DocumentSymbol,
    namespace: Option<(SignatureNamespaceScope, NamePolicyProtocol)>,
    home: Option<(SignatureNamespaceScope, NamePolicyProtocol)>,
}

pub(super) fn source_outline(
    source: &str,
    analysis: &AnalysisResult,
) -> Option<Vec<DocumentSymbol>> {
    if analysis.allows_lexical_declaration_advice() {
        return None;
    }
    let original = analysis.has_original_vendor_source_names()
        || analysis
            .command_invocations
            .iter()
            .any(|invocation| invocation.original_name_input.is_some())
        || analysis.original_procedure_declarations().next().is_some()
        || analysis.original_class_declarations().next().is_some()
        || analysis.original_symbol_declarations().next().is_some()
        || !analysis.original_variable_symbols.is_empty()
        || !analysis.original_variable_write_advice.is_empty()
        || analysis
            .namespace_refs
            .iter()
            .any(|reference| reference.original_name_input.is_some());
    #[cfg(test)]
    if std::env::var_os("TCL_LSP_TRACE_ORIGINAL_OUTLINE").is_some() {
        eprintln!(
            "ORIGINAL_OUTLINE inventories procs={} classes={} variables={} namespaces={} registry={} lexical={} original={}",
            analysis.original_procedure_declarations().count(),
            analysis.original_class_declarations().count(),
            analysis.original_variable_symbols.len(),
            analysis.namespace_refs.len(),
            analysis.all_defined_symbols.len(),
            analysis.allows_lexical_declaration_advice(),
            original
        );
    }
    if !original {
        return Some(Vec::new());
    }
    let image = SourceImage::document(source);
    let Some(config) = analysis
        .body_lexer_config
        .filter(|config| analysis.matches_original_source_image(&image, *config))
    else {
        return Some(Vec::new());
    };
    let index = LineIndex::new(source);
    let mut nodes = Vec::new();
    let mut scopes = FxHashMap::default();
    let mut lexical_scopes = Vec::new();
    let mut pending = vec![&analysis.global_scope];
    while let Some(scope) = pending.pop() {
        pending.extend(&scope.children);
        lexical_scopes.push(scope);
        if scope.kind == ScopeKind::Namespace
            && let Some(span) = scope.name_span
        {
            scopes.entry(span).or_insert_with(Vec::new).push(scope);
        }
    }
    for declaration in analysis.original_symbol_declarations() {
        if !declaration.matches_source(&image, config) {
            continue;
        }
        let input = declaration.name_input();
        let Some(name) = label(input.bytes(), input.policy(), config) else {
            continue;
        };
        let metadata = declaration.metadata();
        nodes.push(Node {
            symbol: DocumentSymbol {
                name,
                detail: metadata.detail.clone().filter(|detail| !detail.is_empty()),
                kind: SymbolKind::from(declaration.descriptor().kind),
                range: span_to_range(source, &index, metadata.full_span),
                selection_range: span_to_range(source, &index, declaration.span()),
                children: Vec::new(),
            },
            namespace: None,
            home: declaration
                .original_namespace()
                .map(|namespace| (namespace.clone(), input.policy())),
        });
    }
    if let Some(declarations) = crate::vendor_declaration::declarations(source, analysis) {
        for declaration in declarations {
            let Some(name) =
                crate::vendor_declaration::source_label(declaration.input(), declaration.purpose())
            else {
                continue;
            };
            let selection = span_to_range(source, &index, declaration.span());
            let (kind, body, detail) = if let Some(metadata) = declaration.procedure_metadata() {
                (
                    SymbolKind::Function,
                    metadata.body_span,
                    Some(format!(
                        "Source declaration {}",
                        format_param_list(&metadata.params)
                    )),
                )
            } else if let Some(metadata) = declaration.class_metadata() {
                (
                    SymbolKind::Class,
                    metadata.body_span,
                    Some("Source declaration".to_owned()),
                )
            } else if let Some(metadata) = declaration.symbol_metadata() {
                (
                    SymbolKind::from(metadata.kind),
                    metadata.full_span,
                    metadata.detail.clone(),
                )
            } else {
                continue;
            };
            nodes.push(Node {
                symbol: DocumentSymbol {
                    name,
                    detail,
                    kind,
                    range: merge_ranges(selection, span_to_range(source, &index, body)),
                    selection_range: selection,
                    children: Vec::new(),
                },
                namespace: None,
                home: None,
            });
        }
        for advice in analysis
            .original_vendor_variable_advice()
            .filter(|advice| advice.is_source_root())
        {
            let input = advice.input().original_occurrence().name_input();
            if !input.matches_source(&image, config) {
                continue;
            }
            let name = advice
                .literal_units()
                .and_then(|units| std::str::from_utf8(units).ok())
                .map(str::to_owned)
                .or_else(|| source.get(advice.span().as_range()).map(str::to_owned));
            let Some(name) = name else {
                continue;
            };
            let range = span_to_range(source, &index, advice.span());
            nodes.push(Node {
                symbol: DocumentSymbol {
                    name,
                    detail: Some("Source declaration".to_owned()),
                    kind: SymbolKind::Variable,
                    range,
                    selection_range: range,
                    children: Vec::new(),
                },
                namespace: None,
                home: None,
            });
        }
    }
    for reference in analysis
        .namespace_refs
        .iter()
        .filter(|reference| reference.declares)
    {
        let (Some(input), Some(namespace)) =
            (&reference.original_name_input, &reference.source_namespace)
        else {
            continue;
        };
        if !crate::original_name_edit::original_input_matches_source(
            source,
            analysis,
            input,
            reference.span,
        ) {
            continue;
        }
        let Some(name) = label(input.bytes(), input.policy(), config) else {
            continue;
        };
        let Some(bodies) = scopes.get(&reference.span) else {
            continue;
        };
        let Some(body) = bodies.first().and_then(|scope| scope.body_span) else {
            continue;
        };
        if bodies.iter().any(|scope| scope.body_span != Some(body)) {
            continue;
        }
        let selection = span_to_range(source, &index, reference.span);
        nodes.push(Node {
            symbol: DocumentSymbol {
                name,
                detail: None,
                kind: SymbolKind::Namespace,
                range: merge_ranges(selection, span_to_range(source, &index, body)),
                selection_range: selection,
                children: Vec::new(),
            },
            namespace: Some((namespace.clone(), input.policy())),
            home: None,
        });
    }
    for declaration in analysis.original_procedure_declarations() {
        let key = declaration.name_input();
        if key.source_image() != &image || key.lexer_config() != config {
            continue;
        }
        let Some(name) = label(
            declaration.name().slot().simple.as_bytes(),
            key.policy(),
            config,
        ) else {
            continue;
        };
        let metadata = declaration.metadata();
        let selection = span_to_range(source, &index, key.span());
        nodes.push(Node {
            symbol: DocumentSymbol {
                name,
                detail: Some(format_param_list(&metadata.params)),
                kind: SymbolKind::Function,
                range: merge_ranges(selection, span_to_range(source, &index, metadata.body_span)),
                selection_range: selection,
                children: Vec::new(),
            },
            namespace: None,
            home: (!lexical_scopes.iter().any(|scope| {
                scope.kind == ScopeKind::Proc
                    && scope.body_span.is_some_and(|body| {
                        body.start() <= key.span().start() && key.span().end() <= body.end()
                    })
            }))
            .then(|| command_home(declaration.name()))
            .flatten(),
        });
    }
    for declaration in analysis.original_class_declarations() {
        let key = declaration.name_input();
        if key.source_image() != &image || key.lexer_config() != config {
            continue;
        }
        let Some(name) = label(
            declaration.name().slot().simple.as_bytes(),
            key.policy(),
            config,
        ) else {
            continue;
        };
        let metadata = declaration.metadata();
        let selection = span_to_range(source, &index, key.span());
        // Special-member keywords report the original class body. Named
        // methods and properties use their own folded source inventories;
        // canonical declarations remain available separately for navigation.
        let named_spans: FxHashSet<_> = metadata
            .methods
            .values()
            .chain(metadata.class_methods.values())
            .map(|method| method.name_span)
            .collect();
        let mut children = class_member_symbols(source, metadata, &index);
        children.retain(|child| {
            child.kind != SymbolKind::Property
                && !named_spans
                    .iter()
                    .any(|span| span_to_range(source, &index, *span) == child.selection_range)
        });
        let mut seen = FxHashSet::default();
        for side in [
            tcl_compiler::analyser::types::MemberSide::ClassObject,
            tcl_compiler::analyser::types::MemberSide::Instance,
        ] {
            let Some(methods) = metadata.original_members.methods(side) else {
                continue;
            };
            for member in methods {
                let original = member.declaration().original_word();
                if original.image() != &image || original.config() != config {
                    continue;
                }
                let identity = (
                    original.span(),
                    member.original_name_input().bytes().to_vec(),
                );
                if member.native_class_delegate() && !seen.insert(identity) {
                    continue;
                }
                let Some(name) = crate::original_oo::method_label(&member) else {
                    continue;
                };
                let method = member.metadata();
                let name_span = member
                    .original_name_input()
                    .original_word_key()
                    .filter(|key| key.source_image() == &image && key.lexer_config() == config)
                    .map_or(original.span(), |key| key.span());
                let selection = span_to_range(source, &index, name_span);
                let parameters = format_param_list(&method.params);
                let detail = if side == tcl_compiler::analyser::types::MemberSide::ClassObject {
                    format!("classmethod {parameters}")
                } else {
                    parameters
                };
                children.push(DocumentSymbol {
                    name,
                    detail: Some(detail),
                    kind: SymbolKind::Method,
                    range: merge_ranges(selection, span_to_range(source, &index, method.body_span)),
                    selection_range: selection,
                    children: Vec::new(),
                });
            }
            if let Some(properties) = metadata.original_properties.properties(side) {
                for property in properties {
                    let original = property.declaration().name_input();
                    if original.source_image() != &image || original.lexer_config() != config {
                        continue;
                    }
                    let Some(name) = label(original.bytes(), original.policy(), config) else {
                        continue;
                    };
                    let range = span_to_range(source, &index, original.span());
                    children.push(DocumentSymbol {
                        name,
                        detail: Some(property.kind().name().to_owned()),
                        kind: SymbolKind::Property,
                        range,
                        selection_range: range,
                        children: Vec::new(),
                    });
                }
            }
        }
        let detail = super::class_detail(metadata);
        nodes.push(Node {
            symbol: DocumentSymbol {
                name,
                detail: (!detail.is_empty()).then_some(detail),
                kind: SymbolKind::Class,
                range: merge_ranges(selection, span_to_range(source, &index, metadata.body_span)),
                selection_range: selection,
                children,
            },
            namespace: None,
            home: command_home(declaration.name()),
        });
    }
    let mut variables = FxHashSet::default();
    let mut declarations: Vec<_> = analysis
        .original_variable_symbols
        .iter()
        .filter(|occurrence| occurrence.is_declaration() && occurrence.symbol().is_namespace())
        .collect();
    declarations.sort_by_key(|occurrence| occurrence.span().start());
    for occurrence in declarations {
        if variables.contains(occurrence.symbol()) {
            continue;
        }
        let input = occurrence.original_name_input();
        if !crate::original_name_edit::original_input_matches_source(
            source,
            analysis,
            input,
            occurrence.span(),
        ) {
            continue;
        }
        let Some(name) = label(input.bytes(), input.policy(), config) else {
            continue;
        };
        variables.insert(occurrence.symbol().clone());
        let range = span_to_range(source, &index, occurrence.span());
        nodes.push(Node {
            symbol: DocumentSymbol {
                name,
                detail: None,
                kind: SymbolKind::Variable,
                range,
                selection_range: range,
                children: Vec::new(),
            },
            namespace: None,
            home: None,
        });
    }
    // Conditional write-name cards need no selected cell. An existing symbol
    // declaration covers the same syntax; aliases and navigation still use
    // their independently retained symbol inventory.
    let mut written = Vec::new();
    for advice in &analysis.original_variable_write_advice {
        if advice.original_frame().is_some_and(|frame| {
            !matches!(
                frame.frame().layout(),
                tcl_compiler::var_resolve::VariableExecutionFrame::Global
                    | tcl_compiler::var_resolve::VariableExecutionFrame::Namespace(_)
                    | tcl_compiler::var_resolve::VariableExecutionFrame::NamespaceActivation { .. }
            )
        }) {
            continue;
        }
        let input = advice.original_name_input();
        if !crate::original_name_edit::original_input_matches_source(
            source,
            analysis,
            input,
            advice.span(),
        ) || analysis.original_variable_symbols.iter().any(|occurrence| {
            occurrence.is_declaration()
                && occurrence.span() == advice.span()
                && occurrence.original_name_input() == input
        }) || written
            .iter()
            .any(|(span, previous)| *span == advice.span() && *previous == input)
        {
            continue;
        }
        let protocol = input.policy().recipe();
        let Some(form) = advice.receiver_form().input_form(protocol, input.bytes()) else {
            continue;
        };
        let projection = match form {
            tcl_syntax::naming::NativeVariableInputForm::Combined(bytes) => {
                protocol.combined_variable_input(bytes)
            }
            tcl_syntax::naming::NativeVariableInputForm::Separate { root, element } => {
                protocol.separate_variable_input(root, element)
            }
        };
        let Some(name) = label(projection.root().selected(), input.policy(), config) else {
            continue;
        };
        written.push((advice.span(), input));
        let range = span_to_range(source, &index, advice.span());
        nodes.push(Node {
            symbol: DocumentSymbol {
                name,
                detail: None,
                kind: SymbolKind::Variable,
                range,
                selection_range: range,
                children: Vec::new(),
            },
            namespace: None,
            home: None,
        });
    }
    Some(assemble(nodes))
}

fn command_home(
    command: &tcl_compiler::signature_scan::scope::SignatureSourceCommand,
) -> Option<(SignatureNamespaceScope, NamePolicyProtocol)> {
    match command.policy().recipe() {
        NativeNameProtocol::C(_) => Some((
            SignatureNamespaceScope::C(command.slot().namespace.clone()),
            command.policy(),
        )),
        // Jim's command table is flat. A reported qualifier cannot supply its
        // separate namespace owner; the source containment remains available.
        NativeNameProtocol::Jim084 => None,
    }
}

fn label(
    bytes: &[u8],
    _policy: NamePolicyProtocol,
    _config: tcl_lexer::LexerConfig,
) -> Option<String> {
    Some(tcl_syntax::native_string::resident_name_label(bytes))
}

fn assemble(nodes: Vec<Node>) -> Vec<DocumentSymbol> {
    let mut homes = FxHashMap::default();
    for (index, node) in nodes.iter().enumerate() {
        if let Some(namespace) = &node.namespace {
            homes.entry(namespace.clone()).or_insert(index);
        }
    }
    let destinations: Vec<_> = nodes
        .iter()
        .enumerate()
        .map(|(index, node)| {
            node.home
                .as_ref()
                .and_then(|home| homes.get(home))
                .copied()
                .filter(|target| *target != index)
                .filter(|target| {
                    !super::strictly_contains(nodes[*target].symbol.range, node.symbol.range)
                })
                .map(|target| nodes[target].symbol.selection_range)
        })
        .collect();
    let mut rehomed = Vec::new();
    let mut symbols = Vec::new();
    for (node, destination) in nodes.into_iter().zip(destinations) {
        if let Some(destination) = destination {
            rehomed.push((destination, node.symbol));
        } else {
            symbols.push(node.symbol);
        }
    }
    // Preserve the existing category order for unrelated top-level cards.
    // Containment independently groups bodies and source-orders their children.
    symbols.sort_by_key(|symbol| {
        (
            outline_category(symbol.kind),
            symbol.range.start_line,
            symbol.range.start_character,
        )
    });
    let mut symbols = nest_by_containment(symbols);
    // Source containment is assembled before ranges are widened for semantic
    // homes, so that rehoming cannot invent lexical nesting between siblings.
    for (destination, symbol) in rehomed {
        if let Some(unplaced) = place_by_selection(&mut symbols, destination, symbol) {
            symbols.push(unplaced);
        }
    }
    for symbol in &mut symbols {
        super::sort_nested_children(symbol);
    }
    symbols
}

fn outline_category(kind: SymbolKind) -> u8 {
    match kind {
        SymbolKind::Class => 0,
        SymbolKind::Function => 1,
        SymbolKind::Variable => 2,
        SymbolKind::Test | SymbolKind::Constant | SymbolKind::Operator | SymbolKind::Event => 3,
        SymbolKind::Namespace | SymbolKind::Module => 4,
        SymbolKind::Method | SymbolKind::Constructor | SymbolKind::Property => 5,
    }
}

fn place_by_selection(
    symbols: &mut [DocumentSymbol],
    destination: super::LineRange,
    symbol: DocumentSymbol,
) -> Option<DocumentSymbol> {
    let mut carried = symbol;
    for node in symbols {
        if node.kind == SymbolKind::Namespace && node.selection_range == destination {
            node.range = merge_ranges(node.range, carried.range);
            node.children.push(carried);
            return None;
        }
        let range = carried.range;
        match place_by_selection(&mut node.children, destination, carried) {
            Some(unplaced) => carried = unplaced,
            None => {
                node.range = merge_ranges(node.range, range);
                return None;
            }
        }
    }
    Some(carried)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn collect<'a>(symbols: &'a [DocumentSymbol], out: &mut Vec<&'a DocumentSymbol>) {
        for symbol in symbols {
            out.push(symbol);
            collect(&symbol.children, out);
        }
    }

    #[test]
    fn original_vendor_outline_keeps_source_cards_after_reporting_maps_are_erased() {
        // Implementation contract: naming.vendor.original-source-declaration-consumers
        // docs/design/analysis/name-resolution-proofs/vendor-original-source-declaration-consumers.md
        let source = "proc helper {argument} {return $argument}\nproc p\\uD800 {} {}\nwhen HTTP_REQUEST {}\n";
        let mut analysis = Analyser::new().analyse(source, "f5-irules");
        assert_eq!(analysis.original_vendor_procedure_declarations().count(), 2);
        assert_eq!(analysis.original_vendor_symbol_declarations().count(), 1);
        analysis.all_procs.clear();
        analysis.all_defined_symbols.clear();
        analysis.global_scope.procs.clear();
        analysis.global_scope.defined_symbols.clear();
        let symbols = super::super::document_symbols_from_analysis(source, &analysis);
        let mut flat = Vec::new();
        collect(&symbols, &mut flat);
        for name in ["helper", r"p\uD800", "HTTP_REQUEST"] {
            let selected = flat
                .iter()
                .find(|symbol| symbol.name == name)
                .unwrap_or_else(|| panic!("{name}: {symbols:?}"));
            assert!(selected.range.start_line <= selected.selection_range.start_line);
            assert!(selected.selection_range.end_line <= selected.range.end_line);
        }
        assert_eq!(
            flat.iter()
                .filter(|symbol| symbol.kind == SymbolKind::Function)
                .count(),
            2
        );
        assert!(
            super::super::document_symbols_from_analysis(
                &format!("# displaced\n{source}"),
                &analysis
            )
            .is_empty()
        );
    }

    #[test]
    fn original_outline_preserves_opaque_declarations_and_canonical_moved_members_without_ui_maps()
    {
        let source = "proc p\\uD800 {} {return A}\nproc p\\uD801 {} {return B}\noo::class create C\\uD800 {method m\\uD800 {} {return A}; renamemethod m\\uD800 moved}\noo::class create C\\uD801 {method m\\uD801 {} {return B}}\nset v\\uD800 1\nset v\\uD801 2\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_procs.clear();
        analysis.all_classes.clear();
        analysis.global_scope.procs.clear();
        analysis.global_scope.classes.clear();
        analysis.global_scope.variables.clear();
        let symbols = super::super::document_symbols_from_analysis(source, &analysis);
        let mut flat = Vec::new();
        collect(&symbols, &mut flat);
        for (kind, count) in [
            (SymbolKind::Function, 2),
            (SymbolKind::Class, 2),
            (SymbolKind::Method, 2),
            (SymbolKind::Variable, 2),
        ] {
            let matching = flat
                .iter()
                .filter(|symbol| symbol.kind == kind)
                .collect::<Vec<_>>();
            assert_eq!(matching.len(), count, "{kind:?}: {symbols:?}");
            assert_ne!(matching[0].name, matching[1].name);
        }
        assert!(
            super::super::document_symbols_from_analysis(&format!("{source} "), &analysis)
                .is_empty()
        );
    }

    #[test]
    // Implementation contract: naming.core.original-variable-outline-advice
    // docs/design/analysis/name-resolution-proofs/original-variable-outline-advice.md
    fn original_write_advice_cards_do_not_require_or_donate_storage_symbols() {
        let source = "set v\\uD800 1\nset v\\uD801 2\nproc p {} {set local 3}\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        assert!(analysis.original_variable_write_advice.len() >= 3);
        analysis.original_variable_symbols.clear();
        analysis.all_procs.clear();
        analysis.global_scope.variables.clear();
        let symbols = super::super::document_symbols_from_analysis(source, &analysis);
        let mut flat = Vec::new();
        collect(&symbols, &mut flat);
        let variables = flat
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::Variable)
            .collect::<Vec<_>>();
        assert_eq!(variables.len(), 2, "{symbols:?}");
        assert_ne!(variables[0].name, variables[1].name);
        assert!(analysis.original_variable_symbols.is_empty());
        assert!(
            super::super::document_symbols_from_analysis(
                &format!("# displaced\n{source}"),
                &analysis
            )
            .is_empty()
        );
        analysis.original_variable_write_advice.clear();
        let symbols = super::super::document_symbols_from_analysis(source, &analysis);
        let mut flat = Vec::new();
        collect(&symbols, &mut flat);
        assert!(
            flat.iter()
                .all(|symbol| symbol.kind != SymbolKind::Variable)
        );
    }

    #[test]
    // Implementation contract: naming.core.readonly-member-source-candidates
    // docs/design/analysis/name-resolution-proofs/readonly-member-source-candidates.md
    fn original_readonly_loop_names_are_outline_cards_without_static_declaration_keys() {
        let source =
            "oo::class create C {foreach item {left right} {method $item {} {return body}}}";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_classes.clear();
        analysis.global_scope.classes.clear();
        let symbols = super::super::document_symbols_from_analysis(source, &analysis);
        let mut flat = Vec::new();
        collect(&symbols, &mut flat);
        let mut names = flat
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::Method)
            .map(|symbol| symbol.name.as_str())
            .collect::<Vec<_>>();
        names.sort_unstable();
        assert_eq!(names, ["left", "right"]);
        let declarations = analysis
            .original_class_declarations()
            .next()
            .unwrap()
            .metadata()
            .original_members
            .declarations()
            .collect::<Vec<_>>();
        assert!(
            declarations
                .iter()
                .all(|method| method.declaration().static_occurrence().is_none())
        );
    }

    #[test]
    fn original_property_outline_keeps_per_name_kinds_and_opaque_redefinitions() {
        let source = "oo::configurable create C {property p\\uD800 -kind readable q\\uD801 -kind writable; property p\\uD800 -kind readwrite}";
        let mut analysis = Analyser::new().analyse(source, "tcl9.0").clone();
        analysis.all_classes.clear();
        analysis.global_scope.classes.clear();
        let symbols = super::super::document_symbols_from_analysis(source, &analysis);
        let mut flat = Vec::new();
        collect(&symbols, &mut flat);
        let properties = flat
            .iter()
            .filter(|symbol| symbol.kind == SymbolKind::Property)
            .collect::<Vec<_>>();
        assert_eq!(properties.len(), 2, "{symbols:?}");
        assert_ne!(properties[0].name, properties[1].name);
        let mut kinds = properties
            .iter()
            .map(|property| property.detail.as_deref().unwrap())
            .collect::<Vec<_>>();
        kinds.sort_unstable();
        assert_eq!(kinds, ["readwrite", "writable"]);
    }

    #[test]
    fn original_outline_groups_qualified_publications_by_typed_namespace_home() {
        let source = "namespace eval N\\uD800 {}\nnamespace eval N\\uD801 {}\nproc ::N\\uD800::p {} {}\nproc ::N\\uD801::p {} {}\n";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        analysis.all_procs.clear();
        analysis.global_scope.procs.clear();
        for child in &mut analysis.global_scope.children {
            child.name = "collapsed".into();
            child.procs.clear();
        }
        for reference in &mut analysis.namespace_refs {
            reference.qualified_name = "::wrong".into();
            reference.original_name.clear();
        }
        let symbols = super::super::document_symbols_from_analysis(source, &analysis);
        assert_eq!(symbols.len(), 2, "{symbols:?}");
        assert_ne!(symbols[0].name, symbols[1].name);
        for namespace in &symbols {
            assert_eq!(namespace.kind, SymbolKind::Namespace);
            assert_eq!(namespace.children.len(), 1);
            assert_eq!(namespace.children[0].name, "p");
        }
    }
}

#[cfg(test)]
mod registry_ledger_tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    #[test]
    fn original_registry_outline_uses_sealed_roles_and_names_after_reporting_maps_clear() {
        // Implementation contract: naming.compiler.original-symbol-declaration-advice
        // docs/design/analysis/name-resolution-proofs/original-symbol-declaration-advice.md
        let source = r"package require tcltest; tcltest::test case\uD800 first -body {} -result {}; tcltest::test case\uD801 second -body {} -result {}";
        let mut analysis = Analyser::new().analyse(source, "tcl8.6");
        assert_eq!(analysis.original_symbol_declarations().count(), 2);
        analysis.all_defined_symbols.clear();
        analysis.global_scope.defined_symbols.clear();
        analysis.global_scope.children.clear();
        let symbols = source_outline(source, &analysis).unwrap();
        assert_eq!(symbols.len(), 2);
        assert_ne!(symbols[0].name, symbols[1].name);
        assert!(
            symbols.iter().all(
                |symbol| symbol.kind == SymbolKind::from(tcl_registry::DefinedSymbolKind::Test)
            )
        );
        assert!(
            source_outline(&format!("# changed\n{source}"), &analysis)
                .unwrap()
                .is_empty()
        );
    }
}
