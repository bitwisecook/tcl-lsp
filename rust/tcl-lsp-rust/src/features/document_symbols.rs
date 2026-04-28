//! Document-symbol provider — Rust port of
//! `lsp/features/document_symbols.py`.
//!
//! Walks the analyser's scope tree and emits a tree of
//! `(name, kind, range, selection_range, detail, children)` records
//! for the editor outline view (Cmd+Shift+O / breadcrumbs).
//!
//! The Python original also has two non-analyser paths — a
//! ``_symbols_from_chunks`` fast-path (basic event/proc symbols
//! extracted while a full analysis is still running) and a
//! ``_conf_wrapped_symbols`` path for conf-wrapped iRules.  Both
//! stay in Python for now: the chunks path is a transient outline
//! while the subprocess analyser warms up (replaced wholesale once
//! the LSP-server port lands), and the conf-wrapped path threads
//! through ``embedded_rules`` records that aren't yet on the Rust
//! analyser-result shape.  The Rust function ports the
//! analysis-driven path, which is the case the dispatcher hits
//! once analysis is available.
//!
//! Range conversion: spans are half-open ``[start, end)`` on the
//! Rust side; the LSP `Range` is also half-open, so the
//! conversion is a direct ``position_at(start)`` /
//! ``position_at(end)``.  Mirrors `to_lsp_range`'s behaviour for
//! the inclusive-end Python ranges (``end_character + 1``) without
//! the off-by-one detour.  Columns are byte offsets, matching the
//! Python LSP server's existing (non-UTF-16) convention.
//!
//! [`AnalysisResult`]: tcl_compiler::analyser::AnalysisResult

use tcl_compiler::analyser::{
    Analyser, ClassDef, MethodDef, ProcDef, PropertyDef, Scope, ScopeKind, VarDef,
};
use tcl_compiler::signature_scan::types::ParamDef;
use tcl_lexer::{LineIndex, Span};

/// LSP `SymbolKind` values used by the document-symbol provider.
///
/// The wire form ([`Self::as_str`]) is the LSP enum's identifier
/// (`"Function"`, `"Method"`, …) rather than its numeric value —
/// `lsprotocol` accepts both forms via `SymbolKind[name]`, and the
/// string is easier to read in dumps and tests than `12`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SymbolKind {
    /// `proc` definition.
    Function,
    /// `method` / `classmethod` / destructor inside a class.
    Method,
    /// `oo::class create` / `oo::define` definition.
    Class,
    /// `property` declaration in an OO class.
    Property,
    /// `constructor` inside a class.
    Constructor,
    /// `namespace eval` body.
    Namespace,
    /// Top-level `set` / `variable` definition at global / namespace
    /// scope.
    Variable,
}

impl SymbolKind {
    /// Identifier name as used by `lsprotocol.types.SymbolKind`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Function => "Function",
            Self::Method => "Method",
            Self::Class => "Class",
            Self::Property => "Property",
            Self::Constructor => "Constructor",
            Self::Namespace => "Namespace",
            Self::Variable => "Variable",
        }
    }
}

/// Line-and-character range as the LSP wire shape expects.
///
/// `start` and `end` are byte-column positions; both are
/// 0-based and the range is half-open (`end` is exclusive),
/// matching the LSP `Range` contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineRange {
    /// First line of the range (0-based).
    pub start_line: u32,
    /// Byte column on `start_line`.
    pub start_character: u32,
    /// Last line of the range (0-based, exclusive).
    pub end_line: u32,
    /// Byte column on `end_line` (exclusive).
    pub end_character: u32,
}

/// One node in the document-symbol tree.
///
/// Mirrors `lsprotocol.types.DocumentSymbol`: a labelled, ranged
/// outline entry that may carry nested children.  The `PyO3` binding
/// renders this as a dict the Python dispatcher materialises into
/// the lsprotocol type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSymbol {
    /// Display name (proc / class / namespace / property name).
    pub name: String,
    /// Optional one-line detail (parameter list, metaclass, …).
    /// `None` when the Python original wouldn't have set the
    /// field.
    pub detail: Option<String>,
    /// LSP symbol kind.
    pub kind: SymbolKind,
    /// Outer range of the symbol — the merged span over name +
    /// body.  Used by editors for click-to-fold.
    pub range: LineRange,
    /// Selection range — the name token, used for the
    /// breadcrumb / outline highlight.
    pub selection_range: LineRange,
    /// Nested symbols (proc-local definitions, class members,
    /// nested namespaces).  Empty when there are none.
    pub children: Vec<DocumentSymbol>,
}

/// Compute document symbols for a Tcl source document.
///
/// Runs the Rust analyser internally and walks its scope tree.
/// Mirrors `get_document_symbols` in
/// `lsp/features/document_symbols.py` for the
/// `analysis is not None` (and `embedded_rules is None`) path —
/// which is the case the LSP server hits once analysis is
/// available.
#[must_use]
pub fn document_symbols(source: &str, dialect: &str) -> Vec<DocumentSymbol> {
    if source.is_empty() {
        return Vec::new();
    }

    let mut analyser = Analyser::new();
    let analysis = analyser.analyse(source, dialect);
    let line_index = LineIndex::new(source);

    scope_symbols(&analysis.global_scope, &line_index)
}

fn span_to_range(line_index: &LineIndex, span: Span) -> LineRange {
    let start = line_index.position_at(span.start());
    let end = line_index.position_at(span.end());
    LineRange {
        start_line: start.line,
        start_character: start.character,
        end_line: end.line,
        end_character: end.character,
    }
}

fn pos_leq(a_line: u32, a_char: u32, b_line: u32, b_char: u32) -> bool {
    if a_line == b_line {
        a_char <= b_char
    } else {
        a_line < b_line
    }
}

/// Merge two ranges into a single outer range covering both.
///
/// Mirrors `_merge_symbol_range`: take the earlier start and the
/// later end so the resulting range contains both inputs.
fn merge_ranges(first: LineRange, second: LineRange) -> LineRange {
    let (start_line, start_character) = if pos_leq(
        first.start_line,
        first.start_character,
        second.start_line,
        second.start_character,
    ) {
        (first.start_line, first.start_character)
    } else {
        (second.start_line, second.start_character)
    };
    let (end_line, end_character) = if pos_leq(
        second.end_line,
        second.end_character,
        first.end_line,
        first.end_character,
    ) {
        (first.end_line, first.end_character)
    } else {
        (second.end_line, second.end_character)
    };
    LineRange {
        start_line,
        start_character,
        end_line,
        end_character,
    }
}

fn format_param_list(params: &[ParamDef]) -> String {
    if params.is_empty() {
        return "()".to_string();
    }
    let parts: Vec<String> = params
        .iter()
        .map(|p| {
            if p.has_default {
                format!(
                    "{{{} {}}}",
                    p.name,
                    p.default_value.as_deref().unwrap_or(""),
                )
            } else {
                p.name.clone()
            }
        })
        .collect();
    format!("({})", parts.join(" "))
}

fn class_detail(class_def: &ClassDef) -> String {
    let mut parts: Vec<String> = Vec::new();
    if class_def.metaclass != "oo::class" {
        parts.push(class_def.metaclass.clone());
    }
    if !class_def.superclasses.is_empty() {
        parts.push(format!(": {}", class_def.superclasses.join(", ")));
    }
    parts.join(" ")
}

fn class_member_symbols(class_def: &ClassDef, line_index: &LineIndex) -> Vec<DocumentSymbol> {
    let mut children: Vec<DocumentSymbol> = Vec::new();

    for ctor in &class_def.constructors {
        let ctor_range = span_to_range(line_index, ctor.body_span);
        children.push(DocumentSymbol {
            name: "constructor".to_string(),
            detail: Some(format_param_list(&ctor.params)),
            kind: SymbolKind::Constructor,
            range: ctor_range,
            selection_range: ctor_range,
            children: Vec::new(),
        });
    }

    if let Some(dtor) = &class_def.destructor {
        let dtor_range = span_to_range(line_index, dtor.body_span);
        children.push(DocumentSymbol {
            name: "destructor".to_string(),
            detail: None,
            kind: SymbolKind::Method,
            range: dtor_range,
            selection_range: dtor_range,
            children: Vec::new(),
        });
    }

    let mut method_pairs: Vec<(&String, &MethodDef)> = class_def.methods.iter().collect();
    method_pairs.sort_by_key(|(_, md)| md.name_span.start());
    for (_, md) in method_pairs {
        children.push(method_symbol(md, line_index, false));
    }

    let mut classmethod_pairs: Vec<(&String, &MethodDef)> =
        class_def.class_methods.iter().collect();
    classmethod_pairs.sort_by_key(|(_, md)| md.name_span.start());
    for (_, md) in classmethod_pairs {
        children.push(method_symbol(md, line_index, true));
    }

    let mut property_pairs: Vec<(&String, &PropertyDef)> = class_def.properties.iter().collect();
    property_pairs.sort_by_key(|(_, pd)| pd.name_span.start());
    for (_, pd) in property_pairs {
        let prop_range = span_to_range(line_index, pd.name_span);
        children.push(DocumentSymbol {
            name: pd.name.clone(),
            detail: None,
            kind: SymbolKind::Property,
            range: prop_range,
            selection_range: prop_range,
            children: Vec::new(),
        });
    }

    children
}

fn method_symbol(md: &MethodDef, line_index: &LineIndex, classmethod: bool) -> DocumentSymbol {
    let body_range = span_to_range(line_index, md.body_span);
    let name_range = span_to_range(line_index, md.name_span);
    let symbol_range = merge_ranges(name_range, body_range);
    let params = format_param_list(&md.params);
    let detail = if classmethod {
        format!("classmethod {params}")
    } else {
        params
    };
    DocumentSymbol {
        name: md.name.clone(),
        detail: Some(detail),
        kind: SymbolKind::Method,
        range: symbol_range,
        selection_range: name_range,
        children: Vec::new(),
    }
}

/// Recursively collect symbols from a scope and its children.
///
/// Mirrors `_scope_symbols`: classes, then procs, then variables
/// (global / namespace scopes only), then nested namespace
/// scopes.
fn scope_symbols(scope: &Scope, line_index: &LineIndex) -> Vec<DocumentSymbol> {
    let mut symbols: Vec<DocumentSymbol> = Vec::new();

    let mut class_pairs: Vec<(&String, &ClassDef)> = scope.classes.iter().collect();
    class_pairs.sort_by_key(|(_, cd)| cd.name_span.start());
    for (_, class_def) in class_pairs {
        symbols.push(class_symbol(class_def, line_index));
    }

    let mut proc_pairs: Vec<(&String, &ProcDef)> = scope.procs.iter().collect();
    proc_pairs.sort_by_key(|(_, pd)| pd.name_span.start());
    for (_, proc_def) in proc_pairs {
        symbols.push(proc_symbol(proc_def, scope, line_index));
    }

    if matches!(scope.kind, ScopeKind::Global | ScopeKind::Namespace) {
        let mut var_pairs: Vec<(&String, &VarDef)> = scope.variables.iter().collect();
        var_pairs.sort_by_key(|(_, vd)| vd.definition_span.start());
        for (_, var_def) in var_pairs {
            let var_range = span_to_range(line_index, var_def.definition_span);
            symbols.push(DocumentSymbol {
                name: var_def.name.clone(),
                detail: None,
                kind: SymbolKind::Variable,
                range: var_range,
                selection_range: var_range,
                children: Vec::new(),
            });
        }
    }

    for child in &scope.children {
        if matches!(child.kind, ScopeKind::Namespace) {
            if let Some(span) = child.body_span {
                let ns_range = span_to_range(line_index, span);
                let child_syms = scope_symbols(child, line_index);
                symbols.push(DocumentSymbol {
                    name: child.name.clone(),
                    detail: None,
                    kind: SymbolKind::Namespace,
                    range: ns_range,
                    selection_range: ns_range,
                    children: child_syms,
                });
            }
        }
    }

    symbols
}

fn class_symbol(class_def: &ClassDef, line_index: &LineIndex) -> DocumentSymbol {
    let body_range = span_to_range(line_index, class_def.body_span);
    let name_range = span_to_range(line_index, class_def.name_span);
    let symbol_range = merge_ranges(name_range, body_range);
    let detail = class_detail(class_def);
    DocumentSymbol {
        name: class_def.name.clone(),
        detail: if detail.is_empty() {
            None
        } else {
            Some(detail)
        },
        kind: SymbolKind::Class,
        range: symbol_range,
        selection_range: name_range,
        children: class_member_symbols(class_def, line_index),
    }
}

fn proc_symbol(proc_def: &ProcDef, scope: &Scope, line_index: &LineIndex) -> DocumentSymbol {
    // Find the proc's body scope to recurse into for nested
    // definitions.  Mirrors the Python loop that matches by
    // ``child.kind == "proc"`` and ``child.name == proc_def.name``.
    let child_symbols: Vec<DocumentSymbol> = scope
        .children
        .iter()
        .find(|child| matches!(child.kind, ScopeKind::Proc) && child.name == proc_def.name)
        .map(|child| scope_symbols(child, line_index))
        .unwrap_or_default();

    let body_range = span_to_range(line_index, proc_def.body_span);
    let name_range = span_to_range(line_index, proc_def.name_span);
    let symbol_range = merge_ranges(name_range, body_range);
    DocumentSymbol {
        name: proc_def.name.clone(),
        detail: Some(format_param_list(&proc_def.params)),
        kind: SymbolKind::Function,
        range: symbol_range,
        selection_range: name_range,
        children: child_symbols,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(symbols: &[DocumentSymbol]) -> Vec<&str> {
        symbols.iter().map(|s| s.name.as_str()).collect()
    }

    fn range_contains(outer: LineRange, inner: LineRange) -> bool {
        let starts_after = pos_leq(
            outer.start_line,
            outer.start_character,
            inner.start_line,
            inner.start_character,
        );
        let ends_after = pos_leq(
            inner.end_line,
            inner.end_character,
            outer.end_line,
            outer.end_character,
        );
        starts_after && ends_after
    }

    #[test]
    fn symbol_kind_wire_form() {
        assert_eq!(SymbolKind::Function.as_str(), "Function");
        assert_eq!(SymbolKind::Method.as_str(), "Method");
        assert_eq!(SymbolKind::Class.as_str(), "Class");
        assert_eq!(SymbolKind::Property.as_str(), "Property");
        assert_eq!(SymbolKind::Constructor.as_str(), "Constructor");
        assert_eq!(SymbolKind::Namespace.as_str(), "Namespace");
        assert_eq!(SymbolKind::Variable.as_str(), "Variable");
    }

    #[test]
    fn empty_source_yields_no_symbols() {
        assert!(document_symbols("", "tcl8.6").is_empty());
    }

    #[test]
    fn single_proc_emits_function_symbol() {
        // Mirrors `test_single_proc` in tests/test_document_symbols.py.
        let source = "proc greet {name} {\n    puts \"Hello $name\"\n}\n";
        let symbols = document_symbols(source, "tcl8.6");
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "greet");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
        assert_eq!(symbols[0].detail.as_deref(), Some("(name)"));
    }

    #[test]
    fn proc_with_default_param_renders_brace_form() {
        // Mirrors `test_proc_with_defaults`.
        let source = "proc greet {name {greeting Hello}} {\n    puts \"$greeting $name\"\n}\n";
        let symbols = document_symbols(source, "tcl8.6");
        assert_eq!(symbols.len(), 1);
        assert_eq!(
            symbols[0].detail.as_deref(),
            Some("(name {greeting Hello})")
        );
    }

    #[test]
    fn multiple_procs_emit_one_symbol_each() {
        // Mirrors `test_multiple_procs`.
        let source = "proc foo {} { return 1 }\nproc bar {} { return 2 }\n";
        let symbols = document_symbols(source, "tcl8.6");
        let mut got = names(&symbols);
        got.sort_unstable();
        assert_eq!(got, vec!["bar", "foo"]);
    }

    #[test]
    fn proc_with_no_params_renders_empty_parens() {
        // Mirrors `test_proc_no_params`.
        let symbols = document_symbols("proc nop {} { return }\n", "tcl8.6");
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].detail.as_deref(), Some("()"));
    }

    #[test]
    fn proc_symbol_range_contains_selection_range() {
        // Mirrors `test_proc_symbol_range_contains_selection`.
        let source = "proc greet {name} {\n    puts \"Hello $name\"\n}\n";
        let symbols = document_symbols(source, "tcl8.6");
        assert_eq!(symbols.len(), 1);
        let proc = &symbols[0];
        assert!(
            range_contains(proc.range, proc.selection_range),
            "range {:?} must contain selection {:?}",
            proc.range,
            proc.selection_range,
        );
    }

    #[test]
    fn namespace_eval_nests_inner_proc() {
        // Mirrors `test_namespace_with_proc`.
        let source = concat!(
            "namespace eval myns {\n",
            "    proc helper {} {\n",
            "        return 1\n",
            "    }\n",
            "}\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        assert_eq!(symbols.len(), 1);
        let ns = &symbols[0];
        assert_eq!(ns.name, "myns");
        assert_eq!(ns.kind, SymbolKind::Namespace);
        assert_eq!(ns.children.len(), 1);
        assert_eq!(ns.children[0].name, "helper");
        assert_eq!(ns.children[0].kind, SymbolKind::Function);
    }

    #[test]
    fn nested_namespaces_recurse_two_levels() {
        // Mirrors `test_nested_namespace`.
        let source = concat!(
            "namespace eval outer {\n",
            "    namespace eval inner {\n",
            "        proc deep {} { return }\n",
            "    }\n",
            "}\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        assert_eq!(symbols.len(), 1);
        let outer = &symbols[0];
        assert_eq!(outer.name, "outer");
        let inner: Vec<&DocumentSymbol> = outer
            .children
            .iter()
            .filter(|c| c.name == "inner")
            .collect();
        assert_eq!(inner.len(), 1);
        let deep: Vec<&DocumentSymbol> = inner[0]
            .children
            .iter()
            .filter(|c| c.name == "deep")
            .collect();
        assert_eq!(deep.len(), 1);
    }

    #[test]
    fn global_set_emits_variable_symbol() {
        // Mirrors `test_global_variable`.
        let symbols = document_symbols("set myvar 42\n", "tcl8.6");
        let vars: Vec<&DocumentSymbol> = symbols
            .iter()
            .filter(|s| s.kind == SymbolKind::Variable)
            .collect();
        assert!(!vars.is_empty(), "expected at least one variable symbol");
        assert!(vars.iter().any(|s| s.name == "myvar"));
    }

    #[test]
    fn oo_class_emits_class_symbol_with_method_children() {
        // Mirrors `test_class_symbol_emitted` and
        // `test_methods_nested_under_class`.
        let source = concat!(
            "oo::class create Dog {\n",
            "    method bark {} { return \"woof\" }\n",
            "    method fetch {item} { return $item }\n",
            "}\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        assert_eq!(symbols.len(), 1);
        let cls = &symbols[0];
        assert_eq!(cls.name, "Dog");
        assert_eq!(cls.kind, SymbolKind::Class);
        let method_names: Vec<&str> = cls
            .children
            .iter()
            .filter(|c| c.kind == SymbolKind::Method)
            .map(|c| c.name.as_str())
            .collect();
        assert!(method_names.contains(&"bark"));
        assert!(method_names.contains(&"fetch"));
    }

    #[test]
    fn oo_class_constructor_emits_constructor_symbol() {
        // Mirrors `test_constructor_symbol`.
        let source = concat!(
            "oo::class create Dog {\n",
            "    constructor {name} { set n $name }\n",
            "}\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        let cls = &symbols[0];
        let ctor: Vec<&DocumentSymbol> = cls
            .children
            .iter()
            .filter(|c| c.kind == SymbolKind::Constructor)
            .collect();
        assert_eq!(ctor.len(), 1);
        assert_eq!(ctor[0].name, "constructor");
        let detail = ctor[0].detail.as_deref().unwrap_or("");
        assert!(
            detail.contains("(name)"),
            "expected constructor detail to contain '(name)', got {detail:?}",
        );
    }

    #[test]
    fn oo_configurable_property_emits_property_symbol() {
        // Mirrors `test_property_symbol`.
        let source = concat!(
            "oo::configurable create Point {\n",
            "    property x y\n",
            "}\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        let cls = &symbols[0];
        let prop_names: Vec<&str> = cls
            .children
            .iter()
            .filter(|c| c.kind == SymbolKind::Property)
            .map(|c| c.name.as_str())
            .collect();
        assert!(prop_names.contains(&"x"), "expected property x");
        assert!(prop_names.contains(&"y"), "expected property y");
    }

    #[test]
    fn oo_class_detail_lists_superclass() {
        // Mirrors `test_class_detail_shows_superclass`.
        let source = concat!("oo::class create Dog {\n", "    superclass Animal\n", "}\n",);
        let symbols = document_symbols(source, "tcl8.6");
        let detail = symbols[0].detail.as_deref().unwrap_or("");
        assert!(
            detail.contains(": Animal"),
            "expected superclass in detail, got {detail:?}",
        );
    }

    #[test]
    fn oo_class_detail_lists_non_default_metaclass() {
        // Mirrors `test_class_detail_shows_metaclass`.
        let source = concat!(
            "oo::abstract create Shape {\n",
            "    method area {} {}\n",
            "}\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        let detail = symbols[0].detail.as_deref().unwrap_or("");
        assert!(
            detail.contains("oo::abstract"),
            "expected metaclass in detail, got {detail:?}",
        );
    }

    #[test]
    fn oo_classmethod_detail_lists_classmethod_keyword() {
        // Mirrors `test_classmethod_detail`.
        let source = concat!(
            "oo::class create Counter {\n",
            "    classmethod count {} { return 0 }\n",
            "}\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        let cls = &symbols[0];
        let method = cls
            .children
            .iter()
            .find(|c| c.name == "count")
            .expect("expected classmethod 'count'");
        let detail = method.detail.as_deref().unwrap_or("");
        assert!(
            detail.contains("classmethod"),
            "expected 'classmethod' in detail, got {detail:?}",
        );
    }

    #[test]
    fn merge_ranges_takes_outer_bounds() {
        let a = LineRange {
            start_line: 1,
            start_character: 4,
            end_line: 1,
            end_character: 8,
        };
        let b = LineRange {
            start_line: 0,
            start_character: 2,
            end_line: 3,
            end_character: 1,
        };
        let merged = merge_ranges(a, b);
        assert_eq!(merged.start_line, 0);
        assert_eq!(merged.start_character, 2);
        assert_eq!(merged.end_line, 3);
        assert_eq!(merged.end_character, 1);
    }

    #[test]
    fn format_param_list_handles_empty_list() {
        assert_eq!(format_param_list(&[]), "()");
    }

    #[test]
    fn format_param_list_handles_default_with_empty_value() {
        let params = vec![ParamDef {
            name: "x".to_string(),
            has_default: true,
            default_value: None,
        }];
        assert_eq!(format_param_list(&params), "({x })");
    }
}
