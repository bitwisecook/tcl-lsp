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

mod original;

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
    /// `tcltest::test` case.  Surfaced with the LSP
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
pub fn document_symbols(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Vec<DocumentSymbol> {
    if source.is_empty() {
        return Vec::new();
    }

    let mut analyser = Analyser::new();
    let analysis = analyser.analyse(source, dialect.name);
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
    if source.is_empty()
        || crate::original_context::CurrentSourceContext::capture(source, analysis).is_none()
    {
        return Vec::new();
    }
    if let Some(symbols) = original::source_outline(source, analysis) {
        return symbols;
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
    // definition / references, all agree on `::pix::svg::parse`. Placing it
    // lexically would contradict the very same response's
    // `Namespace pix > svg` tree. Home each one under
    // the namespace node its qualified name names; a namespace this document
    // never opens has no node, so the symbol stays where it was written.
    for (home, symbol) in rehomed {
        if let Some(unplaced) = place_under_namespace(&mut symbols, "::", &home, symbol) {
            symbols.push(unplaced);
        }
    }
    let recovered = recovered_tail_symbols(source, analysis, &line_index)
        .into_iter()
        .filter(|symbol| !selects(&symbols, symbol.selection_range))
        .collect::<Vec<_>>();
    symbols.extend(recovered);
    symbols
}

/// Whether any symbol in the tree already selects `range`.
fn selects(symbols: &[DocumentSymbol], range: LineRange) -> bool {
    symbols
        .iter()
        .any(|symbol| symbol.selection_range == range || selects(&symbol.children, range))
}

/// Procedures written after an unterminated delimiter.
///
/// tclsh runs nothing of such a file, so the analysis holds no declaration
/// past the cut. The outline is a display answer: it reads the commands the
/// recovery segmenter finds there by their written words and the document's
/// registry. A command whose head the analysis reached, is not one literal
/// bareword, or is a name the document itself rebinds is not read. Nothing
/// here is a declaration for any other consumer.
fn recovered_tail_symbols(
    source: &str,
    analysis: &AnalysisResult,
    line_index: &LineIndex,
) -> Vec<DocumentSymbol> {
    let Some(current) = crate::original_context::CurrentSourceContext::capture(source, analysis)
    else {
        return Vec::new();
    };
    if tcl_lexer::script_image_is_complete(
        &tcl_lexer::SourceImage::document(source),
        current.config(),
    ) != Ok(false)
    {
        return Vec::new();
    }
    let Some(realm) = analysis.retained_command_realm() else {
        return Vec::new();
    };
    let rebound = analysis
        .all_procs
        .keys()
        .chain(analysis.command_aliases.keys())
        .chain(analysis.renamed_commands.keys())
        .chain(analysis.renamed_commands.values())
        .map(|name| tcl_syntax::naming::written_command_tail(name.as_bytes()))
        .collect();
    let tail = RecoveredTail {
        source,
        line_index,
        current: &current,
        bindings: realm.source_bindings_ref(),
        rebound,
    };
    let mut out = Vec::new();
    tail.collect(0, 0, false, &mut out);
    out
}

/// Nesting cap for unterminated braces read by [`RecoveredTail::collect`].
const MAX_RECOVERED_DEPTH: u32 = 32;

struct RecoveredTail<'a> {
    source: &'a str,
    line_index: &'a LineIndex,
    current: &'a crate::original_context::CurrentSourceContext<'a>,
    bindings: &'a tcl_compiler::command_binding::SourceCommandBindings,
    rebound: std::collections::HashSet<&'a [u8]>,
}

impl RecoveredTail<'_> {
    /// Read the script that starts at `base`: the procedures in the commands
    /// recovered after its cut, or, where the script ends in an unterminated
    /// brace word, the script inside that word.
    fn collect(&self, base: u32, depth: u32, past_cut: bool, out: &mut Vec<DocumentSymbol>) {
        use tcl_compiler::analyser::utils::{SharedNameSet, recovery_known_commands};
        let Some(text) = self.source.get(base as usize..) else {
            return;
        };
        if depth >= MAX_RECOVERED_DEPTH {
            return;
        }
        let (config, registry) = (
            self.current.config().at_depth(depth),
            self.current.registry(),
        );
        let known = recovery_known_commands(text, registry, &SharedNameSet::default(), config);
        let known: std::collections::HashSet<&str> = known.iter().collect();
        let commands =
            tcl_source::segment::segment_commands_with_recovery_and_config(text, &known, config);
        // The cut is the partial command; a script inside an unterminated
        // brace is past the cut as a whole.
        let cut = commands
            .iter()
            .find(|command| command.is_partial)
            .map(|command| command.span.start());
        if cut.is_none() && !past_cut {
            // An unterminated brace is closed by the segmenter without a
            // partial command: its word runs to the end with no closer.
            if let Some(word) = commands.last().and_then(|command| command.argv.last())
                && word.kind == tcl_lexer::TokenType::Str
                && word.content_offset == 1
                && word.span.end() as usize >= text.len().saturating_sub(1)
                && text.as_bytes().get(word.span.end() as usize) != Some(&b'}')
                && !text.ends_with('}')
            {
                self.collect(base + word.span.start() + 1, depth + 1, true, out);
            }
            return;
        }
        let readable: Vec<_> = commands
            .iter()
            .filter(|command| {
                let (Some(head), Some(word)) = (command.argv.first(), command.texts.first()) else {
                    return false;
                };
                !command.is_partial
                    && cut.is_none_or(|cut| command.span.start() > cut)
                    && head.kind == tcl_lexer::TokenType::Esc
                    && command.single_token_word.first() == Some(&true)
                    && !self.bindings.reached_site(base + head.span.start())
                    && !self
                        .rebound
                        .contains(tcl_syntax::naming::written_command_tail(word.as_bytes()))
            })
            .collect();
        let scan =
            tcl_compiler::signature_scan::extract_signatures_with_config(text, config, registry);
        for declaration in &scan.procedure_declarations {
            if !readable.iter().any(|command| {
                command.span.start() <= declaration.name_range.start()
                    && declaration.body_range.end() <= command.span.end()
            }) {
                continue;
            }
            let shift = |span: Span| Span::new(base + span.start(), base + span.end());
            let name_range =
                span_to_range(self.source, self.line_index, shift(declaration.name_range));
            let body_range =
                span_to_range(self.source, self.line_index, shift(declaration.body_range));
            out.push(DocumentSymbol {
                name: declaration.name.clone(),
                detail: Some(format_param_list(&declaration.params)),
                kind: SymbolKind::Function,
                range: merge_ranges(name_range, body_range),
                selection_range: name_range,
                children: Vec::new(),
            });
        }
    }
}

/// The outline of a **declaration document** — a `.sslictcl` file, whose
/// blocks are its structure (see [`crate::declaration_outline`]).
///
/// Empty for a script document, whose outline is the analyser's scope tree
/// ([`document_symbols_from_analysis`]); the two never both answer, so the
/// dispatcher picks one and no outline is ever doubled.
///
/// A named block (`endpoint /Common/www`) is a [`SymbolKind::Class`] — the
/// same kind the BIG-IP config outline gives a configured object — and an
/// unnamed one (`hsts`, `grade`) a [`SymbolKind::Property`] of the block that
/// contains it.
#[must_use]
pub fn declaration_symbols(
    source: &str,
    dialect: &'static tcl_dialect::DialectProfile,
) -> Vec<DocumentSymbol> {
    let declarations = crate::declaration_outline::declarations(source, dialect);
    if declarations.is_empty() {
        return Vec::new();
    }
    let line_index = LineIndex::new(source);
    lift_declarations(source, &line_index, &declarations)
}

/// Build declaration-document symbols from the server's current configured
/// analysis, retaining its selected workspace registry and full lexer config.
#[must_use]
pub fn declaration_symbols_from_analysis(
    source: &str,
    analysis: &AnalysisResult,
) -> Vec<DocumentSymbol> {
    let declarations = crate::declaration_outline::declarations_from_analysis(source, analysis);
    if declarations.is_empty() {
        return Vec::new();
    }
    let line_index = LineIndex::new(source);
    lift_declarations(source, &line_index, &declarations)
}

fn lift_declarations(
    source: &str,
    line_index: &LineIndex,
    declarations: &[crate::declaration_outline::BlockDeclaration],
) -> Vec<DocumentSymbol> {
    declarations
        .iter()
        .map(|declaration| DocumentSymbol {
            name: declaration.label(),
            detail: None,
            kind: if declaration.name.is_some() {
                SymbolKind::Class
            } else {
                SymbolKind::Property
            },
            range: span_to_range(source, line_index, declaration.range),
            selection_range: span_to_range(source, line_index, declaration.selection),
            children: lift_declarations(source, line_index, &declaration.children),
        })
        .collect()
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
    /// lexically written in, paired with that home.
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
            name: ctor.kind.clone(),
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
            name: dtor.kind.clone(),
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
            // `proc_symbol` does.  Answering the whole body for both ranges
            // would make clicking a namespace in the outline select the
            // entire block.  `range` widens to cover the
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
        .map_or_default(|child| {
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
        });

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
mod tests;
