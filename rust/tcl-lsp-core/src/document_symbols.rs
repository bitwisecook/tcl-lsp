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

//! Document-symbol provider.
//!
//! Walks the analyser's scope tree and emits a tree of
//! `(name, kind, range, selection_range, detail, children)` records
//! for the editor outline view (Cmd+Shift+O / breadcrumbs).
//!
//! This provider handles only the analysis-driven path — the case
//! the dispatcher hits once analysis is available.  Two non-analyser
//! paths are not implemented here: a chunks fast-path (basic
//! event/proc symbols extracted while a full analysis is still
//! running, a transient outline while the analyser warms up) and a
//! conf-wrapped iRules path threading through ``embedded_rules``
//! records that aren't yet on the analyser-result shape.
//!
//! Range conversion: spans are half-open ``[start, end)`` on the
//! Rust side, and the emitted LSP `Range` is also half-open.
//! Columns are UTF-16 code-unit offsets, matching the LSP `Position`
//! contract.
//!
//! [`AnalysisResult`]: tcl_compiler::analyser::AnalysisResult

use tcl_compiler::analyser::{
    Analyser, AnalysisResult, ClassDef, MethodDef, ProcDef, PropertyDef, Scope, ScopeKind, VarDef,
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
    /// A named definition from a registry symbol-definer command — a
    /// `tcltest::test` case (issue #790).  Surfaced with the LSP
    /// `Function` wire kind: an editor has no dedicated "test" kind, and a
    /// named, runnable unit reads naturally as function-like in the outline
    /// (matching how other language servers list test definitions).
    Test,
    /// A named `tcltest::testConstraint` — a boolean test condition.  Surfaced
    /// as the LSP `Constant` kind (a named, immutable condition).
    Constant,
    /// A named `tcltest::customMatch` mode — a custom result-comparison
    /// strategy.  Surfaced as the LSP `Operator` kind.
    Operator,
    /// A BIG-IP tmsh module folder (`ltm`, `net`, `sys`, …) — the
    /// top level of the BIG-IP config outline.
    Module,
}

impl SymbolKind {
    /// Identifier name as used by `lsprotocol.types.SymbolKind`.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            // A test case has no dedicated editor kind; it shares the
            // `Function` wire form (see the `Test` variant docs).
            Self::Function | Self::Test => "Function",
            Self::Method => "Method",
            Self::Class => "Class",
            Self::Property => "Property",
            Self::Constructor => "Constructor",
            Self::Namespace => "Namespace",
            Self::Variable => "Variable",
            Self::Constant => "Constant",
            Self::Operator => "Operator",
            Self::Module => "Module",
        }
    }
}

impl From<tcl_registry::DefinedSymbolKind> for SymbolKind {
    /// Map a registry outline category to the LSP symbol kind the provider
    /// emits.  Centralises the "which editor kind" decision on the provider
    /// side of the wire contract, keeping the registry LSP-agnostic.
    fn from(kind: tcl_registry::DefinedSymbolKind) -> Self {
        match kind {
            tcl_registry::DefinedSymbolKind::Test => Self::Test,
            tcl_registry::DefinedSymbolKind::Constraint => Self::Constant,
            tcl_registry::DefinedSymbolKind::Matcher => Self::Operator,
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
    /// Line of the start position (0-based).
    pub start_line: u32,
    /// Byte column of the start position on `start_line` (0-based).
    pub start_character: u32,
    /// Line of the end position (0-based).
    pub end_line: u32,
    /// Byte column of the end position on `end_line` (0-based, with
    /// the overall end position exclusive — `(end_line,
    /// end_character)` is the first position *after* the range).
    pub end_character: u32,
}

/// One node in the document-symbol tree.
///
/// A labelled, ranged outline entry that may carry nested children.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentSymbol {
    /// Display name (proc / class / namespace / property name).
    pub name: String,
    /// Optional one-line detail (parameter list, metaclass, …).
    /// `None` when there is no detail to show.
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
#[must_use]
pub fn document_symbols(source: &str, dialect: &str) -> Vec<DocumentSymbol> {
    if source.is_empty() {
        return Vec::new();
    }

    let mut analyser = Analyser::new();
    let analysis = analyser.analyse(source, dialect);
    document_symbols_from_analysis(source, &analysis)
}

/// Build the outline from an *already-computed* [`AnalysisResult`].
///
/// The server caches a per-document analysis (populated by didOpen /
/// didChange); reusing it here avoids a full re-analysis on every
/// `textDocument/documentSymbol` request — the standalone
/// [`document_symbols`] re-runs the analyser, which dominates the
/// request cost on large files.
#[must_use]
pub fn document_symbols_from_analysis(
    source: &str,
    analysis: &AnalysisResult,
) -> Vec<DocumentSymbol> {
    if source.is_empty() {
        return Vec::new();
    }
    let line_index = LineIndex::new(source);
    scope_symbols(source, &analysis.global_scope, &line_index, 0)
}

fn span_to_range(source: &str, line_index: &LineIndex, span: Span) -> LineRange {
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    LineRange {
        start_line: start.line,
        start_character: start.character.get(),
        end_line: end.line,
        end_character: end.character.get(),
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
/// Take the earlier start and the later end so the resulting range
/// contains both inputs.
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

fn class_member_symbols(
    source: &str,
    class_def: &ClassDef,
    line_index: &LineIndex,
) -> Vec<DocumentSymbol> {
    let mut children: Vec<DocumentSymbol> = Vec::new();

    for ctor in &class_def.constructors {
        // `selectionRange` is the `constructor` keyword (its `name_span`), not
        // the whole body — matching `method_symbol` so the outline/breadcrumb
        // reveals the keyword rather than highlighting the entire body
        // (issue 184).
        let name_range = span_to_range(source, line_index, ctor.name_span);
        let body_range = span_to_range(source, line_index, ctor.body_span);
        children.push(DocumentSymbol {
            name: "constructor".to_string(),
            detail: Some(format_param_list(&ctor.params)),
            kind: SymbolKind::Constructor,
            range: merge_ranges(name_range, body_range),
            selection_range: name_range,
            children: Vec::new(),
        });
    }

    if let Some(dtor) = &class_def.destructor {
        let name_range = span_to_range(source, line_index, dtor.name_span);
        let body_range = span_to_range(source, line_index, dtor.body_span);
        children.push(DocumentSymbol {
            name: "destructor".to_string(),
            detail: None,
            kind: SymbolKind::Method,
            range: merge_ranges(name_range, body_range),
            selection_range: name_range,
            children: Vec::new(),
        });
    }

    let mut method_pairs: Vec<(&String, &MethodDef)> = class_def.methods.iter().collect();
    method_pairs.sort_by_key(|(_, md)| md.name_span.start());
    for (_, md) in method_pairs {
        children.push(method_symbol(source, md, line_index, false));
    }

    let mut classmethod_pairs: Vec<(&String, &MethodDef)> =
        class_def.class_methods.iter().collect();
    classmethod_pairs.sort_by_key(|(_, md)| md.name_span.start());
    for (_, md) in classmethod_pairs {
        children.push(method_symbol(source, md, line_index, true));
    }

    let mut property_pairs: Vec<(&String, &PropertyDef)> = class_def.properties.iter().collect();
    property_pairs.sort_by_key(|(_, pd)| pd.name_span.start());
    for (_, pd) in property_pairs {
        let prop_range = span_to_range(source, line_index, pd.name_span);
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

fn method_symbol(
    source: &str,
    md: &MethodDef,
    line_index: &LineIndex,
    classmethod: bool,
) -> DocumentSymbol {
    let body_range = span_to_range(source, line_index, md.body_span);
    let name_range = span_to_range(source, line_index, md.name_span);
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
/// Classes, then procs, then variables (global / namespace scopes
/// only), then nested namespace scopes.
fn scope_symbols(
    source: &str,
    scope: &Scope,
    line_index: &LineIndex,
    depth: u32,
) -> Vec<DocumentSymbol> {
    if crate::MAX_SCOPE_WALK_DEPTH.exceeded(depth) {
        return Vec::new();
    }
    let mut symbols: Vec<DocumentSymbol> = Vec::new();

    let mut class_pairs: Vec<(&String, &ClassDef)> = scope.classes.iter().collect();
    class_pairs.sort_by_key(|(_, cd)| cd.name_span.start());
    for (_, class_def) in class_pairs {
        symbols.push(class_symbol(source, class_def, line_index));
    }

    let mut proc_pairs: Vec<(&String, &ProcDef)> = scope.procs.iter().collect();
    proc_pairs.sort_by_key(|(_, pd)| pd.name_span.start());
    for (_, proc_def) in proc_pairs {
        symbols.push(proc_symbol(source, proc_def, scope, line_index, depth));
    }

    if matches!(scope.kind, ScopeKind::Global | ScopeKind::Namespace) {
        let mut var_pairs: Vec<(&String, &VarDef)> = scope.variables.iter().collect();
        var_pairs.sort_by_key(|(_, vd)| vd.definition_span.start());
        for (_, var_def) in var_pairs {
            let var_range = span_to_range(source, line_index, var_def.definition_span);
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

    // Registry symbol-definer definitions (tcltest tests, …).  Kept in
    // declaration order (already source order) and emitted in every scope kind
    // — a `test` inside a proc body or `namespace eval` nests under its parent.
    for sym in &scope.defined_symbols {
        let name_range = span_to_range(source, line_index, sym.name_span);
        let full_range = span_to_range(source, line_index, sym.full_span);
        symbols.push(DocumentSymbol {
            name: sym.name.clone(),
            detail: sym.detail.clone().filter(|d| !d.is_empty()),
            kind: SymbolKind::from(sym.kind),
            range: full_range,
            selection_range: name_range,
            children: Vec::new(),
        });
    }

    for child in &scope.children {
        if matches!(child.kind, ScopeKind::Namespace)
            && let Some(span) = child.body_span
        {
            let ns_range = span_to_range(source, line_index, span);
            let child_syms = scope_symbols(source, child, line_index, depth + 1);
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

    symbols
}

fn class_symbol(source: &str, class_def: &ClassDef, line_index: &LineIndex) -> DocumentSymbol {
    let body_range = span_to_range(source, line_index, class_def.body_span);
    let name_range = span_to_range(source, line_index, class_def.name_span);
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
        children: class_member_symbols(source, class_def, line_index),
    }
}

fn proc_symbol(
    source: &str,
    proc_def: &ProcDef,
    scope: &Scope,
    line_index: &LineIndex,
    depth: u32,
) -> DocumentSymbol {
    // Find the proc's body scope to recurse into for nested definitions by its
    // body span, which is identical between the `ProcDef` and its `Scope`.
    // Matching on the name would drop nested symbols for a namespace-qualified
    // proc, because the proc scope is keyed by the qualified name
    // (`ns::outer`) while `proc_def.name` is the bare tail (`outer`) — issue
    // 185.
    let child_symbols: Vec<DocumentSymbol> = scope
        .children
        .iter()
        .find(|child| {
            matches!(child.kind, ScopeKind::Proc) && child.body_span == Some(proc_def.body_span)
        })
        .map(|child| scope_symbols(source, child, line_index, depth + 1))
        .unwrap_or_default();

    let body_range = span_to_range(source, line_index, proc_def.body_span);
    let name_range = span_to_range(source, line_index, proc_def.name_span);
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
    use std::fmt::Write as _;

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
        // A test case has no dedicated editor kind — it lists as a function.
        assert_eq!(SymbolKind::Test.as_str(), "Function");
    }

    /// Flatten a symbol tree (depth-first) into `(name, kind)` pairs.
    fn flat(symbols: &[DocumentSymbol]) -> Vec<(String, SymbolKind)> {
        let mut out = Vec::new();
        for s in symbols {
            out.push((s.name.clone(), s.kind));
            out.extend(flat(&s.children));
        }
        out
    }

    fn find<'a>(symbols: &'a [DocumentSymbol], name: &str) -> Option<&'a DocumentSymbol> {
        for s in symbols {
            if s.name == name {
                return Some(s);
            }
            if let Some(found) = find(&s.children, name) {
                return Some(found);
            }
        }
        None
    }

    #[test]
    fn constructor_selection_range_is_the_keyword_not_the_body() {
        // The `constructor`/`destructor` outline symbol's selectionRange must
        // be the keyword span (like a method's name), not the whole body span
        // (issue 184).
        let source = concat!(
            "oo::class create C {\n",
            "    constructor {a} { set x $a }\n",
            "    destructor { cleanup }\n",
            "}\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        let ctor = find(&symbols, "constructor").expect("constructor symbol");
        // The keyword sits at line 1, columns 4..15.
        assert_eq!(ctor.selection_range.start_line, 1);
        assert_eq!(ctor.selection_range.start_character, 4);
        assert_eq!(ctor.selection_range.end_character, 15);
        // The full range strictly contains the (smaller) selection range.
        assert!(range_contains(ctor.range, ctor.selection_range));
        assert_ne!(ctor.range, ctor.selection_range);

        let dtor = find(&symbols, "destructor").expect("destructor symbol");
        assert_eq!(dtor.selection_range.start_line, 2);
        assert_eq!(dtor.selection_range.start_character, 4);
        assert!(range_contains(dtor.range, dtor.selection_range));
        assert_ne!(dtor.range, dtor.selection_range);
    }

    #[test]
    fn nested_proc_inside_namespace_qualified_proc_is_kept() {
        // `proc ns::outer {} { proc inner {} {} }` — the inner proc must still
        // appear nested under the outer one, even though the outer proc's body
        // scope is keyed by its qualified name while `proc_def.name` is the
        // bare tail (issue 185).
        let source = "proc ns::outer {} { proc inner {} {} }\n";
        let symbols = document_symbols(source, "tcl8.6");
        let inner = find(&symbols, "inner").expect("nested inner proc must be listed");
        assert_eq!(inner.kind, SymbolKind::Function);
        // It is a *child* of the outer proc, not a top-level symbol.
        let outer = symbols
            .iter()
            .find(|s| s.name == "outer" || s.name == "ns::outer")
            .expect("outer proc symbol");
        assert!(
            find(&outer.children, "inner").is_some(),
            "inner should be nested under outer"
        );
    }

    // ---- issue #790: tcltest `test` names in the outline ----

    #[test]
    fn tp_imported_test_name_is_a_symbol() {
        // TP: after `namespace import ::tcltest::*`, a bare `test` resolves to
        // the tcltest spec and its name becomes a `Test` outline symbol.
        let source = concat!(
            "package require tcltest\n",
            "namespace import ::tcltest::*\n",
            "test my-case-1 {verifies the widget} -body { set x 1 } -result 1\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        let sym = find(&symbols, "my-case-1").expect("test name should be a symbol");
        assert_eq!(sym.kind, SymbolKind::Test);
        assert_eq!(sym.detail.as_deref(), Some("verifies the widget"));
    }

    #[test]
    fn tp_qualified_test_name_is_a_symbol() {
        // TP: the fully-qualified `tcltest::test` call resolves directly, no
        // import needed.
        let source = concat!(
            "package require tcltest\n",
            "tcltest::test qualified-1 {desc} -body { expr 1 } -result 1\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        let names = names(&symbols);
        assert!(names.contains(&"qualified-1"), "got {names:?}");
    }

    #[test]
    fn tp_legacy_positional_test_name_is_a_symbol() {
        // TP: the legacy positional form `test name desc body result` also
        // names a test case.
        let source = concat!(
            "package require tcltest\n",
            "tcltest::test legacy-1 {desc} { set x 1 } 1\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        let names = names(&symbols);
        assert!(names.contains(&"legacy-1"), "got {names:?}");
    }

    #[test]
    fn tp_constant_var_test_name_resolves_via_propagation() {
        // TP: a test name given as a *constant* `$var` is resolved through the
        // constant-propagation lattice, not recorded as the literal `$name`.
        let source = concat!(
            "package require tcltest\n",
            "set name resolved-1.1\n",
            "tcltest::test $name {desc} -body { set x 1 } -result 1\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        let names = names(&symbols);
        assert!(
            names.contains(&"resolved-1.1"),
            "constant-propagated name expected, got {names:?}"
        );
        assert!(
            !names.contains(&"$name"),
            "the raw substitution text must never be recorded, got {names:?}"
        );
    }

    #[test]
    fn fn_guard_dynamic_test_name_is_not_recorded() {
        // FN-guard / TN: a genuinely dynamic name (no known constant value)
        // must be skipped, not recorded as the literal substitution text.
        let source = concat!(
            "package require tcltest\n",
            "tcltest::test $undefined {desc} -body { set x 1 } -result 1\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        let names = names(&symbols);
        assert!(
            !names
                .iter()
                .any(|n| n.contains("undefined") || n.contains('$')),
            "dynamic test name must not appear, got {names:?}"
        );
    }

    #[test]
    fn fp_guard_bare_test_without_tcltest_is_not_a_symbol() {
        // FP-guard: a bare `test` with no tcltest import is an ordinary unknown
        // user command, not a tcltest case — it must NOT list as a symbol.
        let source = "test not-a-tcltest-case {desc} { set x 1 } 1\n";
        let symbols = document_symbols(source, "tcl8.6");
        let names = names(&symbols);
        assert!(
            !names.contains(&"not-a-tcltest-case"),
            "un-imported `test` must not produce a symbol, got {names:?}"
        );
    }

    #[test]
    fn tn_variable_named_test_is_not_a_test_symbol() {
        // TN: `set test 5` defines a *variable* named `test`; it must list as a
        // Variable, never as a Test case.
        let source = "set test 5\n";
        let kinds = flat(&document_symbols(source, "tcl8.6"));
        assert!(
            kinds
                .iter()
                .any(|(n, k)| n == "test" && *k == SymbolKind::Variable),
            "expected a Variable named test, got {kinds:?}"
        );
        assert!(
            !kinds.iter().any(|(_, k)| *k == SymbolKind::Test),
            "no Test symbol expected, got {kinds:?}"
        );
    }

    #[test]
    fn tn_plain_proc_file_has_no_test_symbols() {
        // TN: a file with only a proc yields no Test symbols at all.
        let kinds = flat(&document_symbols("proc greet {} { return 1 }\n", "tcl8.6"));
        assert!(
            !kinds.iter().any(|(_, k)| *k == SymbolKind::Test),
            "plain proc file must have no Test symbols, got {kinds:?}"
        );
    }

    #[test]
    fn tp_test_inside_namespace_eval_nests_under_it() {
        // TP: a test defined inside `namespace eval` nests under the namespace
        // symbol, mirroring how procs nest.
        let source = concat!(
            "package require tcltest\n",
            "namespace eval suite {\n",
            "    tcltest::test suite-1 {desc} -body { set x 1 } -result 1\n",
            "}\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        let ns = find(&symbols, "suite").expect("namespace symbol");
        assert_eq!(ns.kind, SymbolKind::Namespace);
        assert!(
            ns.children
                .iter()
                .any(|c| c.name == "suite-1" && c.kind == SymbolKind::Test),
            "test should nest under the namespace: {:?}",
            ns.children
        );
    }

    #[test]
    fn fp_guard_local_proc_named_test_shadows_imported_definer() {
        // FP-guard (PR #821 review): a user `proc test` shadows the imported
        // `::tcltest::test` under Tcl's command resolution, so bare `test`
        // calls invoke the local proc, not the definer — they must not be
        // recorded as tcltest test cases.  The proc itself still lists.
        let source = concat!(
            "package require tcltest\n",
            "namespace import ::tcltest::*\n",
            "proc test {args} {}\n",
            "test not-a-case\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        let kinds = flat(&symbols);
        assert!(
            kinds.contains(&("test".to_string(), SymbolKind::Function)),
            "the local proc test should list as a Function: {kinds:?}"
        );
        assert!(
            !kinds.iter().any(|(n, _)| n == "not-a-case"),
            "a shadowed local call must not be a test symbol: {kinds:?}"
        );
        assert!(
            !kinds.iter().any(|(_, k)| *k == SymbolKind::Test),
            "no Test symbol expected when the definer is shadowed: {kinds:?}"
        );
    }

    #[test]
    fn tp_qualified_test_still_records_when_local_proc_shadows_bare_name() {
        // TP companion: a local `proc test` shadows only the *bare* name; an
        // explicit `tcltest::test` call is unaffected and still records.
        let source = concat!(
            "package require tcltest\n",
            "namespace import ::tcltest::*\n",
            "proc test {args} {}\n",
            "tcltest::test real-1 {desc} -body { set x 1 } -result 1\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        let names = names(&symbols);
        assert!(
            names.contains(&"real-1"),
            "qualified test should record: {names:?}"
        );
    }

    #[test]
    fn tp_test_constraint_setter_is_a_constant_symbol() {
        // TP: `testConstraint NAME value` (setter) defines a constraint symbol,
        // filed under the Constant kind with the condition as detail.
        let source = concat!(
            "package require tcltest\n",
            "namespace import ::tcltest::*\n",
            "testConstraint needsRoot 1\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        let sym = find(&symbols, "needsRoot").expect("constraint should be a symbol");
        assert_eq!(sym.kind, SymbolKind::Constant);
        assert_eq!(sym.detail.as_deref(), Some("1"));
    }

    #[test]
    fn fp_guard_test_constraint_getter_is_not_a_symbol() {
        // FP-guard: the one-arg `testConstraint NAME` getter only *reads* the
        // constraint, so it must not produce an outline symbol.
        let source = concat!(
            "package require tcltest\n",
            "namespace import ::tcltest::*\n",
            "testConstraint needsRoot\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        let names = names(&symbols);
        assert!(
            !names.contains(&"needsRoot"),
            "constraint getter must not define a symbol, got {names:?}"
        );
    }

    #[test]
    fn tp_custom_match_is_an_operator_symbol() {
        // TP: `customMatch MODE command` defines a match-mode symbol, filed
        // under the Operator kind with the backing command as detail.
        let source = concat!(
            "package require tcltest\n",
            "tcltest::customMatch dictMatch ::my::dictComparer\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        let sym = find(&symbols, "dictMatch").expect("match mode should be a symbol");
        assert_eq!(sym.kind, SymbolKind::Operator);
        assert_eq!(sym.detail.as_deref(), Some("::my::dictComparer"));
    }

    #[test]
    fn tp_constraint_matcher_test_distinct_kinds() {
        // TP: the three tcltest definers each land under a distinct kind in the
        // same file.
        let source = concat!(
            "package require tcltest\n",
            "namespace import ::tcltest::*\n",
            "testConstraint slow 1\n",
            "customMatch approx ::approxEq\n",
            "test t-1 {desc} -body { set x 1 } -result 1\n",
        );
        let kinds = flat(&document_symbols(source, "tcl8.6"));
        assert!(
            kinds.contains(&("slow".to_string(), SymbolKind::Constant)),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&("approx".to_string(), SymbolKind::Operator)),
            "{kinds:?}"
        );
        assert!(
            kinds.contains(&("t-1".to_string(), SymbolKind::Test)),
            "{kinds:?}"
        );
    }

    #[test]
    fn tp_multiple_tests_each_listed() {
        // TP: every test case in a suite is listed independently.
        let source = concat!(
            "package require tcltest\n",
            "namespace import ::tcltest::*\n",
            "test alpha-1 {a} -body { set x 1 } -result 1\n",
            "test beta-2 {b} -body { set y 2 } -result 2\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        let names = names(&symbols);
        assert!(names.contains(&"alpha-1"), "got {names:?}");
        assert!(names.contains(&"beta-2"), "got {names:?}");
    }

    #[test]
    fn empty_source_yields_no_symbols() {
        assert!(document_symbols("", "tcl8.6").is_empty());
    }

    #[test]
    fn single_proc_emits_function_symbol() {
        let source = "proc greet {name} {\n    puts \"Hello $name\"\n}\n";
        let symbols = document_symbols(source, "tcl8.6");
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "greet");
        assert_eq!(symbols[0].kind, SymbolKind::Function);
        assert_eq!(symbols[0].detail.as_deref(), Some("(name)"));
    }

    #[test]
    fn proc_with_default_param_renders_brace_form() {
        let source = "proc greet {name {greeting Hello}} {\n    puts \"$greeting $name\"\n}\n";
        let symbols = document_symbols(source, "tcl8.6");
        assert_eq!(symbols.len(), 1);
        assert_eq!(
            symbols[0].detail.as_deref(),
            Some("(name {greeting Hello})")
        );
    }

    #[test]
    fn midword_quote_in_unterminated_bracket_still_recovers_tail() {
        // `[foo abc"` — the `"` is mid-word, an ordinary literal, so the
        // following line is a genuine command-break that must still recover.
        // The `"` toggles the command-substitution `in_quotes` counter, so
        // the E201 recovery ghost `]` only closes the bracket because a
        // recovery ghost is an unconditional closer.
        let source = "set x [foo abc\"\nproc recovered_after_midword {} {}\n";
        let symbols = document_symbols(source, "tcl8.6");
        assert!(
            names(&symbols).contains(&"recovered_after_midword"),
            "tail proc not recovered: {:?}",
            names(&symbols)
        );
    }

    #[test]
    fn multiple_procs_emit_one_symbol_each() {
        let source = "proc foo {} { return 1 }\nproc bar {} { return 2 }\n";
        let symbols = document_symbols(source, "tcl8.6");
        let mut got = names(&symbols);
        got.sort_unstable();
        assert_eq!(got, vec!["bar", "foo"]);
    }

    #[test]
    fn proc_with_no_params_renders_empty_parens() {
        let symbols = document_symbols("proc nop {} { return }\n", "tcl8.6");
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].detail.as_deref(), Some("()"));
    }

    /// Regression coverage for issue #996: `scope_symbols`/`proc_symbol`
    /// recurse once per nested namespace/proc scope, with no depth cap
    /// before this fix (`MAX_SCOPE_WALK_DEPTH`, `crate::lib`). A `Scope`
    /// tree built by the real analyser can never exceed its own
    /// `MAX_BODY_DEPTH` (256) in practice, so this exercises deep-but-valid
    /// nesting rather than this crate's own cap tripping — that cap is
    /// defence-in-depth against a scope tree built/received some other way.
    ///
    /// `namespace eval` nesting costs meaningfully more native stack per
    /// level than `if`-nesting does (empirically: 300 levels overflows
    /// `cargo test`'s bare ~2 MiB per-test default, unlike the 2000-level
    /// `if` case elsewhere in this codebase) — so, like
    /// `tcl-compiler`'s own `deeply_nested_if_survives_full_optimiser_pipeline`,
    /// this spawns its own production-sized (64 MiB) thread rather than
    /// asserting on the test harness's thread directly; every real
    /// consumer already wraps analysis in one (issue #996's primary fix).
    /// The assertion is that this returns at all, not what it returns.
    #[test]
    fn deeply_nested_namespaces_produce_a_symbol_tree() {
        const DEPTH: usize = 300;
        const STACK_SIZE: usize = 64 * 1024 * 1024;
        let mut source = String::new();
        for i in 0..DEPTH {
            let _ = writeln!(source, "namespace eval ns{i} {{");
        }
        source.push_str("proc leaf {} { return 1 }\n");
        for _ in 0..DEPTH {
            source.push_str("}\n");
        }
        std::thread::Builder::new()
            .stack_size(STACK_SIZE)
            .spawn(move || {
                let symbols = document_symbols(&source, "tcl8.6");
                assert!(!symbols.is_empty());
            })
            .unwrap()
            .join()
            .unwrap();
    }

    #[test]
    fn proc_symbol_range_contains_selection_range() {
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

    #[test]
    fn foreach_rename_reinstall_idiom_outline_has_no_garbled_dollar_symbol() {
        // TP — issue #923 idx 86: the finding's own outline complaint
        // (`tk/library/accessibility.tcl`'s rename-and-reinstall idiom
        // showing a `Function ${wtype}(args)` outline entry — the raw,
        // unresolved dynamic-name text — instead of the real per-element
        // wrapper names). Every symbol name in the outline must be real
        // Tcl identifier text; none may contain the literal `$` of an
        // unresolved substitution.
        let src = "proc button {args} {return orig_button}\n\
                   proc entry {args} {return orig_entry}\n\
                   namespace eval ::tk::accessible {\n    \
                   foreach wtype {button entry} {\n        \
                   rename ::$wtype ::tk::accessible::orig_$wtype\n        \
                   proc ::$wtype {args} {return wrapped}\n    \
                   }\n\
                   }\n";
        let symbols = document_symbols(src, "tcl8.6");
        let all = flat(&symbols);
        assert!(
            !all.iter().any(|(name, _)| name.contains('$')),
            "garbled dynamic-name symbol leaked into the outline: {all:?}"
        );
        assert!(find(&symbols, "button").is_some(), "{all:?}");
        assert!(find(&symbols, "entry").is_some(), "{all:?}");
    }
}
