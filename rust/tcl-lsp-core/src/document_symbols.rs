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
/// (`"Function"`, `"Method"`, …) rather than its numeric value: the
/// string is easier to read in dumps and tests than `12`, and it is
/// mapped to the numeric value on the way out.
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
    /// An event handler bound by a registry event-handler command — a
    /// `when EVENT { … }` block in an iRule.  Surfaced with the LSP `Event`
    /// wire kind, which editors render with their event glyph.
    Event,
}

impl SymbolKind {
    /// Identifier name of the LSP `SymbolKind` enum member.
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
            Self::Event => "Event",
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
            tcl_registry::DefinedSymbolKind::Event => Self::Event,
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
    let mut rehomed: Vec<(String, DocumentSymbol)> = Vec::new();
    let mut ctx = SymbolCtx {
        rehomed: &mut rehomed,
    };
    let mut symbols = scope_symbols(
        source,
        &analysis.global_scope,
        &line_index,
        ScopePos {
            depth: 0,
            namespace: "::",
        },
        &mut ctx,
    );
    // A proc written with a qualified name outside any `namespace eval` block
    // (`proc pix::svg::parse {…} {…}` at file top level) really lands in the
    // namespace its own name spells — tclsh, and this LSP's own hover /
    // definition / references, all agree on `::pix::svg::parse`. The outline
    // used to place it lexically, contradicting the very same response's
    // `Namespace pix > svg` tree (issue #1140 idx 67). Home each one under
    // the namespace node its qualified name names; a namespace this document
    // never opens has no node, so the symbol stays where it was written.
    for (home, symbol) in rehomed {
        if let Some(unplaced) = place_under_namespace(&mut symbols, "::", &home, symbol) {
            symbols.push(unplaced);
        }
    }
    symbols
}

/// Enclosing namespace of a fully-qualified name (`"::ns::foo"` → `"::ns"`,
/// `"::foo"` → `"::"`) — the same pure-string rule
/// `ItemTree::from_analysis` uses, so the outline and the item tree agree
/// about where a definition lives.
fn enclosing_namespace(qualified: &str) -> String {
    let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qualified);
    if holder.is_empty() {
        "::".to_string()
    } else {
        holder.to_string()
    }
}

/// Join a namespace prefix with a child namespace name as written, honouring
/// the absolute-reset rule (`namespace eval ::a { namespace eval ::b {…} }`
/// creates `::b`, not `::a::b`).
fn join_namespace(prefix: &str, name: &str) -> String {
    tcl_syntax::naming::qualify(prefix, name)
}

/// Insert `symbol` under the namespace node named `home`, descending
/// `symbols` (whose own qualified prefix is `prefix`).  Returns `Some(symbol)`
/// unchanged when no such namespace node exists.
fn place_under_namespace(
    symbols: &mut [DocumentSymbol],
    prefix: &str,
    home: &str,
    symbol: DocumentSymbol,
) -> Option<DocumentSymbol> {
    let mut carried = symbol;
    for node in symbols.iter_mut() {
        if node.kind != SymbolKind::Namespace {
            continue;
        }
        let qualified = join_namespace(prefix, &node.name);
        if qualified == home {
            node.children.push(carried);
            return None;
        }
        carried = place_under_namespace(&mut node.children, &qualified, home, carried)?;
    }
    Some(carried)
}

/// Per-walk state [`scope_symbols`] threads through the scope tree.
struct SymbolCtx<'a> {
    /// Procs whose semantic home namespace differs from the scope they were
    /// lexically written in, paired with that home (issue #1140 idx 67).
    rehomed: &'a mut Vec<(String, DocumentSymbol)>,
}

/// Where in the scope tree [`scope_symbols`] currently is.
#[derive(Clone, Copy)]
struct ScopePos<'a> {
    /// Recursion depth, bounded by `MAX_SCOPE_WALK_DEPTH`.
    depth: u32,
    /// The `::`-rooted qualified namespace this scope's definitions home to.
    namespace: &'a str,
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
    pos: ScopePos<'_>,
    ctx: &mut SymbolCtx<'_>,
) -> Vec<DocumentSymbol> {
    let ScopePos { depth, namespace } = pos;
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
        let symbol = proc_symbol(source, proc_def, scope, line_index, depth, ctx);
        // Only a namespace-level definition can be re-homed: a proc nested
        // inside another proc's body belongs under that proc in the outline
        // whatever its qualified name spells, because that is where the user
        // wrote — and reads — it.
        let home = enclosing_namespace(&proc_def.qualified_name);
        if matches!(scope.kind, ScopeKind::Global | ScopeKind::Namespace) && home != namespace {
            ctx.rehomed.push((home, symbol));
        } else {
            symbols.push(symbol);
        }
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
            let body_range = span_to_range(source, line_index, span);
            let child_ns = join_namespace(namespace, &child.name);
            let child_syms = scope_symbols(
                source,
                child,
                line_index,
                ScopePos {
                    depth: depth + 1,
                    namespace: &child_ns,
                },
                ctx,
            );
            // `selectionRange` is "the range that should be selected and
            // revealed when this symbol is picked" — the *name*, exactly as
            // `proc_symbol` does.  A namespace used to answer its whole body
            // for both ranges, so clicking it in the outline selected the
            // entire block (issue #1218).  `range` then widens to cover the
            // name **and** the body, keeping the LSP containment invariant
            // (`selectionRange` ⊆ `range`) that the narrowing would otherwise
            // break — the name word sits before the body's opening brace.
            let (ns_range, selection_range) =
                child
                    .name_span
                    .map_or((body_range, body_range), |name_span| {
                        let name_range = span_to_range(source, line_index, name_span);
                        (merge_ranges(name_range, body_range), name_range)
                    });
            symbols.push(DocumentSymbol {
                name: child.name.clone(),
                detail: None,
                kind: SymbolKind::Namespace,
                range: ns_range,
                selection_range,
                children: child_syms,
            });
        }
    }

    nest_by_containment(symbols)
}

/// Does `outer` strictly contain `inner` — covering it, but not equal to it?
fn strictly_contains(outer: LineRange, inner: LineRange) -> bool {
    outer != inner
        && pos_leq(
            outer.start_line,
            outer.start_character,
            inner.start_line,
            inner.start_character,
        )
        && pos_leq(
            inner.end_line,
            inner.end_character,
            outer.end_line,
            outer.end_character,
        )
}

/// Re-parent siblings that lexically nest inside one another.
///
/// A scope's symbol list is flat by construction, because a body that does
/// *not* open a scope contributes its definitions to the enclosing one: the
/// `set`s inside an iRules `when EVENT { … }` handler (or a
/// `tcltest::test` case) land in the same scope as the handler itself, so
/// they arrive as siblings whose ranges sit inside the handler's.  The LSP
/// contract wants a tree — and VS Code's outline, breadcrumbs and sticky
/// scroll all assume one — so each symbol moves under the innermost sibling
/// whose range strictly contains it.
///
/// Symbols that contain nothing are left exactly as they were, so every
/// document without this shape (which is every plain Tcl file, where scope
/// bodies *are* scopes) round-trips unchanged.
fn nest_by_containment(mut symbols: Vec<DocumentSymbol>) -> Vec<DocumentSymbol> {
    let count = symbols.len();
    if count < 2 {
        return symbols;
    }
    // `parent[i]` = index of the innermost sibling containing `i`, found by
    // one containment sweep: visiting outermost-first (start ascending, then
    // the wider range first) means the enclosing symbols are exactly the ones
    // still on the stack, so this is O(n log n) rather than a pairwise scan
    // over a document that can carry thousands of top-level symbols.
    let mut sweep: Vec<usize> = (0..count).collect();
    sweep.sort_by_key(|&i| {
        let r = symbols[i].range;
        (
            r.start_line,
            r.start_character,
            std::cmp::Reverse((r.end_line, r.end_character)),
        )
    });
    let mut parent: Vec<Option<usize>> = vec![None; count];
    let mut open: Vec<usize> = Vec::new();
    let mut nested = false;
    for &i in &sweep {
        // Equal ranges do not contain one another, so the earlier of a tied
        // pair is popped and both stay siblings.
        while open
            .last()
            .is_some_and(|&p| !strictly_contains(symbols[p].range, symbols[i].range))
        {
            open.pop();
        }
        if let Some(&p) = open.last() {
            parent[i] = Some(p);
            nested = true;
        }
        open.push(i);
    }
    if !nested {
        return symbols;
    }
    // Move each nested symbol into its parent.  A child always sorts after
    // its parent in the sweep, so walking the sweep backwards assembles a
    // chain (`when` → `test` → `set`) from the inside out.  Indices stay
    // valid because entries are taken, never removed.
    let mut slots: Vec<Option<DocumentSymbol>> = symbols.drain(..).map(Some).collect();
    for &i in sweep.iter().rev() {
        let Some(p) = parent[i] else { continue };
        let Some(child) = slots[i].take() else {
            continue;
        };
        if let Some(host) = slots[p].as_mut() {
            host.children.push(child);
        }
    }
    for slot in slots.iter_mut().flatten() {
        sort_nested_children(slot);
    }
    slots.into_iter().flatten().collect()
}

/// Put a symbol's children back into source order after re-parenting.
fn sort_nested_children(symbol: &mut DocumentSymbol) {
    symbol
        .children
        .sort_by_key(|c| (c.range.start_line, c.range.start_character));
    for child in &mut symbol.children {
        sort_nested_children(child);
    }
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
    ctx: &mut SymbolCtx<'_>,
) -> DocumentSymbol {
    // Find the proc's body scope to recurse into for nested definitions by its
    // body span, which is identical between the `ProcDef` and its `Scope`.
    // Matching on the name would drop nested symbols for a namespace-qualified
    // proc, because the proc scope is keyed by the qualified name
    // (`ns::outer`) while `proc_def.name` is the bare tail (`outer`) — issue
    // 185.
    let body_namespace = enclosing_namespace(&proc_def.qualified_name);
    let child_symbols: Vec<DocumentSymbol> = scope
        .children
        .iter()
        .find(|child| {
            matches!(child.kind, ScopeKind::Proc) && child.body_span == Some(proc_def.body_span)
        })
        .map(|child| {
            scope_symbols(
                source,
                child,
                line_index,
                ScopePos {
                    depth: depth + 1,
                    namespace: &body_namespace,
                },
                ctx,
            )
        })
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

    #[test]
    fn outline_homes_colon_run_qualified_names_consistently() {
        let source = concat!(
            "namespace eval a:::b {}\n",
            "proc a::b::q {} {}\n",
            "proc : {} {}\n",
            "proc ::a:::b {} {}\n",
            "proc foo::: {} {}\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        let names = flat(&symbols);
        assert!(names.iter().any(|(name, _)| name == "q"), "{names:?}");
        assert!(names.iter().any(|(name, _)| name == ":"), "{names:?}");
        assert!(names.iter().any(|(name, _)| name.is_empty()), "{names:?}");
        let ns = find(&symbols, "a:::b").expect("a:::b namespace");
        assert!(find(&ns.children, "q").is_some(), "{symbols:?}");
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
    fn oo_literal_foreach_installed_methods_appear_in_the_outline() {
        // Issue #1277: a literal `foreach`-installed member's *name* is
        // statically knowable even though its signature is not, so it must
        // still show up in `documentSymbol` alongside an ordinary method.
        let source = concat!(
            "oo::class create Widget {\n",
            "    foreach m {alpha beta gamma} {\n",
            "        method $m {args} { return $args }\n",
            "    }\n",
            "    method fetch {item} { return $item }\n",
            "}\n",
        );
        let symbols = document_symbols(source, "tcl9.0");
        let cls = &symbols[0];
        let method_names: Vec<&str> = cls
            .children
            .iter()
            .filter(|c| c.kind == SymbolKind::Method)
            .map(|c| c.name.as_str())
            .collect();
        for name in ["alpha", "beta", "gamma", "fetch"] {
            assert!(method_names.contains(&name), "{method_names:?}");
        }
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
    fn oo_self_block_form_emits_class_side_method_symbols() {
        // Issue #1081 — TP. `self { method … }` is `self method …` spelled as
        // a block; both declare a method on the class *object*. Oracle
        // (tclsh 9.0.4 / 8.6.16, identical):
        //   oo::class create ::C { self { method make {n} {…} } }
        //   ::C make 7               -> made-7
        //   info object methods ::C  -> make      (class-object side)
        //   info class methods ::C   -> {}        (instance side untouched)
        // Only the prefix spelling reached the outline before the fix.
        for source in [
            concat!(
                "oo::class create Counter {\n",
                "    self {\n",
                "        method make {n} { return $n }\n",
                "        method reset {} { return 0 }\n",
                "    }\n",
                "    method tick {} { return 1 }\n",
                "}\n",
            ),
            concat!(
                "oo::class create Counter {}\n",
                "oo::define Counter {\n",
                "    self {\n",
                "        method make {n} { return $n }\n",
                "        method reset {} { return 0 }\n",
                "    }\n",
                "    method tick {} { return 1 }\n",
                "}\n",
            ),
        ] {
            for dialect in ["tcl8.6", "tcl9.0"] {
                let symbols = document_symbols(source, dialect);
                let cls = &symbols[0];
                let make = cls
                    .children
                    .iter()
                    .find(|c| c.name == "make")
                    .unwrap_or_else(|| panic!("expected `make` in outline for {dialect}"));
                assert_eq!(make.kind, SymbolKind::Method);
                assert_eq!(
                    make.detail.as_deref(),
                    Some("classmethod (n)"),
                    "block-form `self method` must carry the same detail the \
                     prefix form does",
                );
                assert!(
                    cls.children.iter().any(|c| c.name == "reset"),
                    "every member of the block must be emitted, not just the first",
                );
                // The plain instance method alongside it is untouched.
                let tick = cls
                    .children
                    .iter()
                    .find(|c| c.name == "tick")
                    .expect("expected instance method `tick`");
                assert_eq!(tick.detail.as_deref(), Some("()"));
            }
        }
    }

    #[test]
    fn oo_self_block_method_selection_range_covers_the_inner_name() {
        // The symbol must select the member's own name word inside the block,
        // not the enclosing `self` keyword — otherwise the outline jump lands
        // on the wrapper.
        let source = concat!(
            "oo::class create Counter {\n",
            "    self {\n",
            "        method make {n} { return $n }\n",
            "    }\n",
            "}\n",
        );
        let symbols = document_symbols(source, "tcl9.0");
        let make = symbols[0]
            .children
            .iter()
            .find(|c| c.name == "make")
            .expect("expected `make`");
        // Line 2 (0-based): `        method make {n} { return $n }`.
        assert_eq!(make.selection_range.start_line, 2);
        assert_eq!(make.selection_range.start_character, 15);
        assert_eq!(make.selection_range.end_character, 19);
    }

    #[test]
    fn self_introspection_call_in_a_method_body_emits_no_symbol() {
        // Issue #1081 — TN. A `self` *introspection* call inside a method body
        // is not a definer member at all; it must contribute nothing to the
        // outline. (`self class` there is an ordinary command substitution —
        // tclsh returns the defining class, it declares nothing.)
        let source = concat!(
            "oo::class create Counter {\n",
            "    method whoami {} {\n",
            "        set c [self class]\n",
            "        return [self object]\n",
            "    }\n",
            "}\n",
        );
        let symbols = document_symbols(source, "tcl9.0");
        let names: Vec<&str> = symbols[0]
            .children
            .iter()
            .map(|c| c.name.as_str())
            .collect();
        assert_eq!(
            names,
            ["whoami"],
            "only the method itself is a symbol; `self class`/`self object` are not",
        );
    }

    #[test]
    fn oo_self_block_deleted_member_is_not_in_the_outline() {
        // Issue #1095 review. A member the same block goes on to delete must
        // not appear in the outline — a stale entry navigates to a name the
        // interpreter does not have. Oracle (tclsh 9.0.4 / 8.6.16, identical):
        //   oo::class create ::C1 {
        //       self { method gone {} {…} ; method kept {} {…} ; deletemethod gone }
        //   }
        //   info object methods ::C1  ->  kept
        //   ::C1 gone                 ->  unknown method "gone"
        let source = concat!(
            "oo::class create Counter {\n",
            "    self {\n",
            "        method gone {} { return 1 }\n",
            "        method kept {} { return 2 }\n",
            "        deletemethod gone\n",
            "    }\n",
            "}\n",
        );
        for dialect in ["tcl8.6", "tcl9.0"] {
            let symbols = document_symbols(source, dialect);
            let names: Vec<&str> = symbols[0]
                .children
                .iter()
                .map(|c| c.name.as_str())
                .collect();
            assert_eq!(names, ["kept"], "{dialect}: stale member in outline");
        }
    }

    #[test]
    fn oo_self_block_non_destructive_reference_keeps_the_member_listed() {
        // TN for the same mechanism: `export` / `unexport` / `filter` name a
        // method without removing it, so the member stays in the outline.
        // Oracle: `self { method a {} {…} ; unexport a ; export a }` leaves
        // `info object methods ::A` -> a, and `::A a` -> 1.
        let source = concat!(
            "oo::class create Counter {\n",
            "    self {\n",
            "        method a {} { return 1 }\n",
            "        unexport a\n",
            "        export a\n",
            "    }\n",
            "}\n",
        );
        let symbols = document_symbols(source, "tcl9.0");
        assert!(
            symbols[0].children.iter().any(|c| c.name == "a"),
            "export/unexport must not drop the outline entry",
        );
    }

    #[test]
    fn oo_unwrapped_deleted_member_is_not_in_the_outline() {
        // Issue #1101 — TP, and the user-visible symptom: an *unwrapped*
        // `deletemethod` (no `self` / `private` wrapper) really removes the
        // method, so a retained outline entry navigates to a name the
        // interpreter does not have. Oracle (tclsh 9.0.4 / 8.6.14, identical):
        //   oo::class create ::I1 { method gone {} {…}; method kept {} {…}
        //                           deletemethod gone }
        //   info class methods ::I1  ->  kept
        //   oo::class create ::I4 { method gone {} {…}; method kept {} {…} }
        //   oo::define ::I4 { deletemethod gone }
        //   info class methods ::I4  ->  kept
        for source in [
            concat!(
                "oo::class create Counter {\n",
                "    method gone {} { return 1 }\n",
                "    method kept {} { return 2 }\n",
                "    deletemethod gone\n",
                "}\n",
            ),
            concat!(
                "oo::class create Counter {\n",
                "    method gone {} { return 1 }\n",
                "    method kept {} { return 2 }\n",
                "}\n",
                "oo::define Counter {\n",
                "    deletemethod gone\n",
                "}\n",
            ),
        ] {
            for dialect in ["tcl8.6", "tcl9.0"] {
                let symbols = document_symbols(source, dialect);
                let names: Vec<&str> = symbols[0]
                    .children
                    .iter()
                    .map(|c| c.name.as_str())
                    .collect();
                assert_eq!(names, ["kept"], "{dialect}: stale member in outline");
            }
        }
    }

    #[test]
    fn oo_unwrapped_renamed_member_is_listed_under_its_new_name() {
        // Issue #1121 (the residual #1101/#1118 left). `renamemethod old new`
        // is a move: `old` goes and `new` takes its place as a fully navigable
        // outline entry. Oracle, byte-identical on tclsh 9.0.4 and 8.6.14:
        //   oo::class create ::I3 { method old {} {return o}; renamemethod old new }
        //   info class methods ::I3        ->  new     ; ::I3 old -> unknown method
        //   info class definition ::I3 new ->  {} { return o }
        //   [::I3 new] new                 ->  o
        let source = concat!(
            "oo::class create Counter {\n",
            "    method old {} { return 1 }\n",
            "    method kept {} { return 2 }\n",
            "    renamemethod old new\n",
            "}\n",
        );
        for dialect in ["tcl8.6", "tcl9.0"] {
            let members = &document_symbols(source, dialect)[0].children;
            let mut names: Vec<&str> = members.iter().map(|c| c.name.as_str()).collect();
            names.sort_unstable();
            assert_eq!(names, ["kept", "new"], "{dialect}");
            // The entry's selection range is the `renamemethod` destination
            // word — the only place `new` is written — so clicking the outline
            // row lands on a real token rather than on the old declaration.
            let new_entry = members
                .iter()
                .find(|c| c.name == "new")
                .expect("`new` listed");
            let line = new_entry.selection_range.start_line as usize;
            assert!(
                source
                    .lines()
                    .nth(line)
                    .is_some_and(|l| l.contains("renamemethod")),
                "{dialect}: selection range should sit on the renamemethod word, got line {line}",
            );
        }
    }

    #[test]
    fn oo_unwrapped_deletemethod_leaves_the_class_side_member_listed() {
        // Issue #1101 — TN. The unwrapped word is instance-scoped, so a
        // class-object-side member of the same name keeps its outline entry.
        let source = concat!(
            "oo::class create Counter {\n",
            "    self {\n",
            "        method cm {} { return 1 }\n",
            "    }\n",
            "}\n",
            "oo::define Counter {\n",
            "    deletemethod cm\n",
            "}\n",
        );
        for dialect in ["tcl8.6", "tcl9.0"] {
            assert!(
                document_symbols(source, dialect)[0]
                    .children
                    .iter()
                    .any(|c| c.name == "cm"),
                "{dialect}: an unwrapped delete must not reach the class side",
            );
        }
    }

    #[test]
    fn oo_self_scoped_unexport_keeps_both_same_named_members_listed() {
        // Issue #1098 — TN at the outline level. Side-scoping the visibility
        // flip must not disturb which members are *listed*: the class-side and
        // instance-side `m` are separate members and both stay in the outline.
        let source = concat!(
            "oo::class create Counter {\n",
            "    method m {} { return 1 }\n",
            "    self {\n",
            "        method m {} { return 2 }\n",
            "        unexport m\n",
            "    }\n",
            "}\n",
        );
        for dialect in ["tcl8.6", "tcl9.0"] {
            let symbols = document_symbols(source, dialect);
            let details: Vec<Option<&str>> = symbols[0]
                .children
                .iter()
                .filter(|c| c.name == "m")
                .map(|c| c.detail.as_deref())
                .collect();
            assert_eq!(
                details.len(),
                2,
                "{dialect}: both sides' `m` must stay listed, got {details:?}",
            );
            assert!(details.contains(&Some("()")), "{dialect}: {details:?}");
            assert!(
                details.contains(&Some("classmethod ()")),
                "{dialect}: {details:?}",
            );
        }
    }

    #[test]
    fn oo_private_block_form_emits_instance_method_symbols() {
        // Issue #1081, symmetric half: `private` is the other registry member
        // marked wrapper-with-block-body, so the same normalisation gives it
        // the outline node its prefix form already had.
        let source = concat!(
            "oo::class create Counter {\n",
            "    private {\n",
            "        method secret {k} { return $k }\n",
            "    }\n",
            "}\n",
        );
        let symbols = document_symbols(source, "tcl9.0");
        let secret = symbols[0]
            .children
            .iter()
            .find(|c| c.name == "secret")
            .expect("expected private method `secret` in the outline");
        assert_eq!(secret.kind, SymbolKind::Method);
        assert_eq!(secret.detail.as_deref(), Some("(k)"));
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

    /// Issue #1218: `namespace eval` used to answer its whole body for
    /// `selectionRange`, so picking the namespace in the outline selected the
    /// entire block instead of its name.
    #[test]
    fn namespace_selection_range_is_the_name_word() {
        let source = concat!(
            "# lead-in\n",
            "namespace eval ::myns {\n",
            "    proc helper {} {\n",
            "        return 1\n",
            "    }\n",
            "}\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        let ns = find(&symbols, "::myns").expect("namespace symbol");
        assert_eq!(
            (
                ns.selection_range.start_line,
                ns.selection_range.start_character,
                ns.selection_range.end_line,
                ns.selection_range.end_character,
            ),
            (1, 15, 1, 21),
            "selectionRange must span exactly `::myns`, got {:?}",
            ns.selection_range,
        );
        assert_ne!(
            ns.selection_range, ns.range,
            "selectionRange must narrow to the name, not repeat the range",
        );
        assert!(
            range_contains(ns.range, ns.selection_range),
            "range {:?} must contain selection {:?}",
            ns.range,
            ns.selection_range,
        );
        // `range` covers the name *and* the body — it starts at the name word
        // (before the body's opening brace) and ends where the body does.
        assert_eq!(ns.range.start_line, 1);
        assert_eq!(ns.range.start_character, 15);
        assert_eq!(ns.range.end_line, 5);
    }

    /// The containment invariant must hold at every nesting depth, not just
    /// for a top-level namespace.
    #[test]
    fn nested_namespace_selection_ranges_stay_inside_their_ranges() {
        fn check(symbols: &[DocumentSymbol]) {
            for sym in symbols {
                assert!(
                    range_contains(sym.range, sym.selection_range),
                    "{}: range {:?} must contain selection {:?}",
                    sym.name,
                    sym.range,
                    sym.selection_range,
                );
                check(&sym.children);
            }
        }
        let source = concat!(
            "namespace eval outer {\n",
            "    namespace eval inner {\n",
            "        proc deep {} { return }\n",
            "    }\n",
            "}\n",
        );
        let symbols = document_symbols(source, "tcl8.6");
        check(&symbols);
        let outer = find(&symbols, "outer").expect("outer namespace");
        assert_eq!(
            (
                outer.selection_range.start_line,
                outer.selection_range.start_character,
                outer.selection_range.end_character,
            ),
            (0, 15, 20),
            "{:?}",
            outer.selection_range,
        );
        let inner = find(&symbols, "inner").expect("inner namespace");
        assert_eq!(
            (
                inner.selection_range.start_line,
                inner.selection_range.start_character,
                inner.selection_range.end_character,
            ),
            (1, 19, 24),
            "{:?}",
            inner.selection_range,
        );
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

    #[test]
    fn opt_proc_outline_shows_the_real_args_only_signature() {
        // TP — issue #923 idx 90: before the fix, the missing analyser hook
        // left the stub's `{}`-arity `ProcDef` in place, so the outline
        // showed an empty (or missing) signature instead of the real
        // `(args)` one.
        let src = "::tcl::OptProc greet {child -use -display} { return $child }\n";
        let symbols = document_symbols(src, "tcl8.6");
        let greet = find(&symbols, "greet").expect("greet symbol");
        assert_eq!(greet.detail.as_deref(), Some("(args)"), "{greet:?}");
    }

    const IRULES: &str = "f5-irules";

    #[test]
    fn irule_event_handlers_are_outline_symbols() {
        // An iRule's structure is its `when` blocks; before the registry
        // `defines_symbol` on `when`, the outline listed only the variables
        // the handlers happened to set.
        let src = concat!(
            "when HTTP_REQUEST {\n",
            "    set host [HTTP::host]\n",
            "}\n",
            "when HTTP_RESPONSE priority 500 {\n",
            "    HTTP::header insert X-Served-By $host\n",
            "}\n",
        );
        let symbols = document_symbols(src, IRULES);
        assert_eq!(names(&symbols), vec!["HTTP_REQUEST", "HTTP_RESPONSE"]);
        assert!(
            symbols.iter().all(|s| s.kind == SymbolKind::Event),
            "{:?}",
            flat(&symbols)
        );
    }

    #[test]
    fn irule_event_outline_uses_resolved_head_identity() {
        let src = concat!(
            "interp alias {} event {} when\n",
            "event HTTP_REQUEST {}\n",
            "proc when {args} {}\n",
            "when CLIENT_DATA {}\n",
        );
        let symbols = document_symbols(src, IRULES);
        assert_eq!(
            symbols
                .iter()
                .filter(|symbol| symbol.kind == SymbolKind::Event)
                .map(|symbol| symbol.name.as_str())
                .collect::<Vec<_>>(),
            ["HTTP_REQUEST"]
        );
    }

    #[test]
    fn event_handler_range_spans_the_body_and_selects_the_event_name() {
        let src = "when HTTP_REQUEST {\n    set host [HTTP::host]\n}\n";
        let symbols = document_symbols(src, IRULES);
        let handler = &symbols[0];
        // Selection is the event name on line 0; the range reaches the
        // closing brace so the outline can fold (and stick) the handler.
        assert_eq!(handler.selection_range.start_line, 0);
        assert_eq!(handler.selection_range.start_character, 5);
        assert_eq!(handler.range.end_line, 2);
        assert!(range_contains(handler.range, handler.selection_range));
    }

    #[test]
    fn variables_set_in_a_handler_nest_under_it() {
        // A `when` body is structural, not a scope, so its `set`s land in
        // the global scope alongside the handler.  They belong *under* it.
        let src = concat!(
            "set global_one 1\n",
            "when HTTP_REQUEST {\n",
            "    set inner 2\n",
            "}\n",
        );
        let symbols = document_symbols(src, IRULES);
        assert_eq!(names(&symbols), vec!["global_one", "HTTP_REQUEST"]);
        let handler = find(&symbols, "HTTP_REQUEST").expect("handler");
        assert_eq!(names(&handler.children), vec!["inner"]);
    }

    #[test]
    fn nested_handlers_nest_in_the_outline() {
        // A `when` inside a `when` is legal iRules; the inner handler is a
        // child, not a sibling with an overlapping range.
        let src = concat!(
            "when CLIENT_ACCEPTED {\n",
            "    when HTTP_REQUEST {\n",
            "        set deep 1\n",
            "    }\n",
            "}\n",
        );
        let symbols = document_symbols(src, IRULES);
        assert_eq!(names(&symbols), vec!["CLIENT_ACCEPTED"]);
        let outer = &symbols[0];
        assert_eq!(names(&outer.children), vec!["HTTP_REQUEST"]);
        assert_eq!(names(&outer.children[0].children), vec!["deep"]);
    }

    #[test]
    fn plain_tcl_outline_is_unchanged_by_containment_nesting() {
        // Every plain-Tcl body opens a scope, so nothing re-parents: the
        // nesting pass must leave these documents byte-identical.
        let src = concat!(
            "set top 1\n",
            "proc greet {name} {\n",
            "    set msg \"hi\"\n",
            "}\n",
            "namespace eval ns {\n",
            "    variable v 1\n",
            "}\n",
        );
        let symbols = document_symbols(src, "tcl8.6");
        assert_eq!(names(&symbols), vec!["greet", "top", "ns"]);
        // `msg` is proc-local, so it is not an outline symbol at all — and
        // `greet` must not swallow `top` or `ns`, which sit outside it.
        assert!(names(&find(&symbols, "greet").unwrap().children).is_empty());
        assert_eq!(names(&find(&symbols, "ns").unwrap().children), vec!["v"]);
    }

    #[test]
    fn dynamic_event_name_is_not_an_outline_symbol() {
        // `when $evt { … }` has no statically-known event; the definer walk
        // skips a non-constant name rather than listing `$evt`.
        let src = "set evt HTTP_REQUEST\nwhen $evt {\n    set x 1\n}\n";
        let all = flat(&document_symbols(src, IRULES));
        assert!(
            !all.iter().any(|(name, _)| name.contains('$')),
            "dynamic event name leaked into the outline: {all:?}"
        );
    }

    /// Assert every symbol range lands inside the document the client holds.
    ///
    /// An outline symbol whose `range.end` addresses a line the client does
    /// not have is dropped wholesale by the outline-model sticky-scroll
    /// provider — `StickyRange(selectionRange.start, range.end)` fails
    /// `TextModel.isValidRange` — while breadcrumbs, which use the same
    /// symbols but no such check, keep working.  That asymmetry is exactly
    /// what masked issue #1122, so the bound is pinned here.
    fn assert_symbol_ranges_in_bounds(source: &str, label: &str) {
        fn walk(symbol: &DocumentSymbol, last_line: u32, label: &str) {
            assert!(
                symbol.range.end_line <= last_line,
                "{label}: symbol {} range ends on line {} but the last line is {last_line}",
                symbol.name,
                symbol.range.end_line,
            );
            assert!(
                symbol.selection_range.end_line <= last_line,
                "{label}: symbol {} selectionRange ends on line {} but the last line is \
                 {last_line}",
                symbol.name,
                symbol.selection_range.end_line,
            );
            for child in &symbol.children {
                walk(child, last_line, label);
            }
        }
        let last_line = u32::try_from(tcl_lexer::LineIndex::new_lsp(source).line_count())
            .expect("line count fits u32")
            - 1;
        let symbols = document_symbols(source, "tcl8.6");
        assert!(!symbols.is_empty(), "{label}: expected outline symbols");
        for symbol in &symbols {
            walk(symbol, last_line, label);
        }
    }

    /// Definitions closing on the document's final line, in the four shapes
    /// the issue #1122 report can take: LF and CRLF (the reporter is on
    /// Windows), each with and without a final newline.  The class fixture
    /// keeps the reported module's structure — a top-level `oo::class
    /// create` with a superclass, an instance variable, a constructor, and
    /// methods — under invented names.
    #[test]
    fn symbol_ranges_stay_inside_the_document_when_definitions_end_at_eof() {
        let sources: [(&str, &str); 3] = [
            (
                "class",
                concat!(
                    "oo::class create Widget {\n",
                    "    superclass WidgetBase\n",
                    "    variable label\n",
                    "    constructor {text} {\n",
                    "        set label $text\n",
                    "    }\n",
                    "    method render {} {\n",
                    "        puts \"widget $label\"\n",
                    "    }\n",
                    "}",
                ),
            ),
            ("proc", "proc demo {} {\n    set x 1\n    set y 2\n}"),
            (
                "namespace",
                "namespace eval ns {\n    proc a {} {\n        set x 1\n    }\n}",
            ),
        ];
        for (kind, base) in sources {
            for (eol, source) in [
                ("lf", base.to_owned()),
                ("crlf", base.replace('\n', "\r\n")),
            ] {
                assert_symbol_ranges_in_bounds(
                    &source,
                    &format!("{kind} at EOF, {eol}, no trailing newline"),
                );
                assert_symbol_ranges_in_bounds(
                    &format!("{source}\n"),
                    &format!("{kind} at EOF, {eol}, trailing newline"),
                );
            }
        }
    }

    // Namespace-resolved proc homing (issue #1140 idx 67).

    #[test]
    fn tp_a_qualified_proc_written_outside_its_namespace_block_nests_under_it() {
        // TP — issue #1140 idx 67, the nico-robert/pix shape reduced: the
        // `namespace eval` blocks have already closed when the qualified
        // `proc` is written, yet tclsh puts it at `::pix::svg::parse` and
        // this LSP's own hover / definition / references all say so. The
        // outline used to contradict them by placing it at top level.
        let src = concat!(
            "namespace eval ::pix {\n",
            "    namespace eval svg {\n",
            "    }\n",
            "}\n",
            "proc pix::svg::parse {a} {\n",
            "    return $a\n",
            "}\n",
        );
        let symbols = document_symbols(src, "tcl8.6");
        assert_eq!(names(&symbols), vec!["::pix"], "{symbols:#?}");
        let svg = find(&symbols, "svg").expect("svg namespace node");
        assert_eq!(
            names(&svg.children),
            vec!["parse"],
            "the proc must nest under `pix > svg`: {symbols:#?}",
        );
    }

    #[test]
    fn tn_a_proc_written_inside_its_namespace_block_still_nests_lexically() {
        // TN — the ordinary case must be untouched: lexical and semantic
        // home agree, so nothing moves.
        let src = "namespace eval ::a {\n    proc caller {} {\n        return 1\n    }\n}\n";
        let symbols = document_symbols(src, "tcl8.6");
        let a = find(&symbols, "::a").expect("namespace node");
        assert_eq!(names(&a.children), vec!["caller"], "{symbols:#?}");
    }

    #[test]
    fn tn_an_unqualified_top_level_proc_stays_at_top_level() {
        // TN — a plain `proc greet` homes to `::`, which is the scope it was
        // written in, so it is not re-homed anywhere.
        let src = "proc greet {} { return 1 }\n";
        let symbols = document_symbols(src, "tcl8.6");
        assert_eq!(names(&symbols), vec!["greet"], "{symbols:#?}");
    }

    #[test]
    fn fp_a_qualified_proc_whose_namespace_is_never_opened_stays_where_written() {
        // FP guard — re-homing must never invent a namespace node. With no
        // `namespace eval ::nowhere` in the document there is nothing to
        // nest under, so the symbol keeps its written position.
        let src = "proc nowhere::helper {} { return 1 }\n";
        let symbols = document_symbols(src, "tcl8.6");
        assert_eq!(names(&symbols), vec!["helper"], "{symbols:#?}");
    }

    #[test]
    fn fp_a_proc_nested_in_another_procs_body_is_never_re_homed() {
        // FP guard — a `proc` written inside another proc's body belongs
        // under that proc in the outline, whatever its qualified name
        // spells: that is where the reader finds it.
        let src = concat!(
            "namespace eval ::ns {\n}\n",
            "proc outer {} {\n",
            "    proc ns::inner {} { return 1 }\n",
            "}\n",
        );
        let symbols = document_symbols(src, "tcl8.6");
        let outer = find(&symbols, "outer").expect("outer proc node");
        assert_eq!(names(&outer.children), vec!["inner"], "{symbols:#?}");
    }
}
