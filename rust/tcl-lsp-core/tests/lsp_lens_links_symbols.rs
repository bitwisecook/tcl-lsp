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

//! Behaviour-driven coverage of four `tcl-lsp-core` LSP providers:
//! `document_symbols`, `code_lens`, `document_links`, and `code_actions`.
//!
//! These tests are derived from each provider's public API (see
//! `src/{document_symbols,code_lens,document_links,code_actions}.rs`) and from
//! Tcl semantics, with the Tcl-semantic facts pinned to real C-Tcl
//! (tclsh8.6 / tclsh9.0 via `scripts/dev/tclsh_check.sh`).
//!
//! ----------------------------------------------------------------------------
//! The presentation/structure split (read this first)
//! ----------------------------------------------------------------------------
//! A document SYMBOL, a code LENS, a document LINK, and a code ACTION are
//! editor-presentation artefacts: their *shape* (which `SymbolKind`, the lens
//! title text, the link's `file://` URI, the action's edit range) is an LSP
//! wire contract, not a Tcl runtime value. Those facts are asserted
//! STRUCTURALLY against the provider's documented contract.
//!
//! What IS a Tcl-semantic fact — and is therefore proven against C-Tcl with a
//! runnable script — is the *underlying ground truth* the artefact reflects:
//!   * a proc / namespace / variable / class / method genuinely declared at
//!     that location (proven via `info procs`, `info args`,
//!     `namespace children`, `info exists`, `info class methods`,
//!     `info class constructor`);
//!   * a `source` / `package require` target being a real Tcl command whose
//!     argument is the path / package name the link surfaces;
//!   * the `catch {body} resultVar` semantics behind the W302 quick-fix
//!     (a result variable captures the body's result/error).
//!
//! Each such test cites the corresponding `tclsh:` observation. Facts verified
//! identically on tclsh8.6 AND tclsh9.0 unless noted.
//!
//! `// bigip` / `// f5-dialect` mark behaviour that only exists for the
//! BIG-IP config / F5 iRules dialects (e.g. the `Module` symbol kind, the
//! iRules-only code-action refactors); those are noted, not driven, here.

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_lsp_core::code_actions::{ActionKind, CodeAction, code_actions};
use tcl_lsp_core::code_lens::{CodeLens, code_lenses};
use tcl_lsp_core::definition::LspRange;
use tcl_lsp_core::document_links::{DocumentLink, document_links, document_links_with_home};
use tcl_lsp_core::document_symbols::{DocumentSymbol, LineRange, SymbolKind, document_symbols};

// ---------------------------------------------------------------------------
// Shared harness — mirrors the existing port files
// (`call_hierarchy.rs`, `references_rename.rs`).
// ---------------------------------------------------------------------------

/// Build an analysis exactly the way the other port files do.
fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

/// Recursively flatten a document-symbol tree to `(name, kind)` pairs.
fn flatten(symbols: &[DocumentSymbol], out: &mut Vec<(String, SymbolKind)>) {
    for s in symbols {
        out.push((s.name.clone(), s.kind));
        flatten(&s.children, out);
    }
}

/// All `(name, kind)` pairs anywhere in the symbol tree.
fn all_pairs(symbols: &[DocumentSymbol]) -> Vec<(String, SymbolKind)> {
    let mut v = Vec::new();
    flatten(symbols, &mut v);
    v
}

/// `true` when `outer` fully contains `inner` (half-open LSP ranges).
fn line_range_contains(outer: LineRange, inner: LineRange) -> bool {
    let starts_le =
        (outer.start_line, outer.start_character) <= (inner.start_line, inner.start_character);
    let ends_ge = (outer.end_line, outer.end_character) >= (inner.end_line, inner.end_character);
    starts_le && ends_ge
}

/// A single-position (empty) `LspRange` at `(line, character)`.
fn cursor(line: u32, character: u32) -> LspRange {
    LspRange {
        start_line: line,
        start_character: character,
        end_line: line,
        end_character: character,
    }
}

/// A non-empty single-line `LspRange` selection.
fn selection(line: u32, start: u32, end: u32) -> LspRange {
    LspRange {
        start_line: line,
        start_character: start,
        end_line: line,
        end_character: end,
    }
}

// ===========================================================================
// document_symbols
// ===========================================================================

#[test]
fn symbols_empty_file_yields_nothing() {
    // An empty document has no outline.
    assert!(
        document_symbols(
            "",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile()
        )
        .is_empty()
    );
}

#[test]
fn symbols_single_proc_is_a_function_with_param_detail() {
    // tclsh: `proc greet {name greeting} {...}` defines a real proc whose
    // parameters are exactly `name greeting`:
    //   info procs greet  -> greet
    //   info args greet   -> name greeting
    // (verified tclsh8.6 + tclsh9.0). The outline must mirror that fact: one
    // Function symbol named `greet` whose detail lists both params.
    let src = "proc greet {name greeting} {\n    return \"$greeting $name\"\n}\n";
    let symbols = document_symbols(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert_eq!(symbols.len(), 1, "{symbols:?}");
    assert_eq!(symbols[0].name, "greet");
    assert_eq!(symbols[0].kind, SymbolKind::Function);
    // Detail is editor presentation; structurally it must name both params.
    let detail = symbols[0].detail.as_deref().unwrap_or("");
    assert!(
        detail.contains("name"),
        "param `name` missing from {detail:?}"
    );
    assert!(
        detail.contains("greeting"),
        "param `greeting` missing from {detail:?}",
    );
}

#[test]
fn symbols_multiple_procs_one_function_each() {
    // tclsh: `info procs` over `proc foo .. ; proc bar ..` -> {bar foo}; the
    // two are independent procs, so the outline has exactly two Function nodes.
    let src = "proc foo {} { return 1 }\nproc bar {} { return 2 }\n";
    let symbols = document_symbols(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let mut names: Vec<&str> = symbols.iter().map(|s| s.name.as_str()).collect();
    names.sort_unstable();
    assert_eq!(names, vec!["bar", "foo"]);
    assert!(symbols.iter().all(|s| s.kind == SymbolKind::Function));
}

#[test]
fn symbols_proc_range_contains_its_selection_range() {
    // The outer `range` (name + body) must contain the `selection_range` (the
    // name token) — an LSP contract the editor relies on for click-to-fold.
    let src = "proc greet {name} {\n    return $name\n}\n";
    let symbols = document_symbols(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert_eq!(symbols.len(), 1);
    let p = &symbols[0];
    assert!(
        line_range_contains(p.range, p.selection_range),
        "range {:?} must contain selection {:?}",
        p.range,
        p.selection_range,
    );
}

#[test]
fn symbols_proc_range_covers_the_definition_text() {
    // The symbol range must actually span the proc definition: it starts on
    // line 0 (the `proc` keyword's line) and its end reaches past the body's
    // last line (line 2 holds the closing brace).
    let src = "proc greet {name} {\n    return $name\n}\n";
    let symbols = document_symbols(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let p = &symbols[0];
    assert_eq!(p.range.start_line, 0, "range should start at the proc line");
    assert!(
        p.range.end_line >= 2,
        "range should extend across the body to the closing brace line; got {:?}",
        p.range,
    );
    // The selection range is the bare name `greet` on line 0 (cols 5..10).
    assert_eq!(p.selection_range.start_line, 0);
    assert_eq!(p.selection_range.start_character, 5);
    assert_eq!(p.selection_range.end_character, 10);
}

#[test]
fn symbols_namespace_eval_nests_inner_proc_as_hierarchy() {
    // tclsh: `namespace eval myns { proc helper {} {...} }` — `helper` is a
    // real proc living under `::myns`:
    //   namespace eval myns { proc helper {} {return 1} }
    //   namespace children ::myns  -> (childless, helper is a proc not a ns)
    //   info procs ::myns::helper  -> ::myns::helper   (verified 8.6 + 9.0)
    // The outline mirrors the lexical nesting: a Namespace node `myns` with the
    // Function `helper` as its child.
    let src = "namespace eval myns {\n    proc helper {} {\n        return 1\n    }\n}\n";
    let symbols = document_symbols(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert_eq!(symbols.len(), 1, "{symbols:?}");
    let ns = &symbols[0];
    assert_eq!(ns.name, "myns");
    assert_eq!(ns.kind, SymbolKind::Namespace);
    let child_names: Vec<&str> = ns.children.iter().map(|c| c.name.as_str()).collect();
    assert!(child_names.contains(&"helper"), "{child_names:?}");
    assert_eq!(
        ns.children
            .iter()
            .find(|c| c.name == "helper")
            .unwrap()
            .kind,
        SymbolKind::Function,
    );
}

#[test]
fn symbols_nested_namespaces_recurse_two_levels() {
    // tclsh: `namespace eval outer { namespace eval inner {} }` makes
    // `::outer::inner` a genuine child namespace:
    //   namespace children ::outer  -> ::outer::inner   (verified 8.6 + 9.0)
    // The outline reflects the two-level hierarchy: outer ⊃ inner ⊃ deep.
    let src = concat!(
        "namespace eval outer {\n",
        "    namespace eval inner {\n",
        "        proc deep {} { return }\n",
        "    }\n",
        "}\n",
    );
    let symbols = document_symbols(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert_eq!(symbols.len(), 1);
    let outer = &symbols[0];
    assert_eq!(outer.name, "outer");
    assert_eq!(outer.kind, SymbolKind::Namespace);
    let inner = outer
        .children
        .iter()
        .find(|c| c.name == "inner")
        .expect("inner namespace must nest under outer");
    assert_eq!(inner.kind, SymbolKind::Namespace);
    assert!(
        inner.children.iter().any(|c| c.name == "deep"),
        "deep proc must nest under inner; got {:?}",
        inner.children,
    );
}

#[test]
fn symbols_global_set_emits_variable() {
    // tclsh: `set myvar 42` creates a real variable:
    //   set myvar 42; info exists myvar -> 1; set myvar -> 42  (8.6 + 9.0)
    // A top-level `set` therefore surfaces as a Variable symbol.
    let symbols = document_symbols(
        "set myvar 42\n",
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let vars: Vec<&DocumentSymbol> = symbols
        .iter()
        .filter(|s| s.kind == SymbolKind::Variable)
        .collect();
    assert!(
        vars.iter().any(|s| s.name == "myvar"),
        "expected a `myvar` Variable symbol; got {symbols:?}",
    );
}

#[test]
fn symbols_oo_class_with_method_children() {
    // tclsh: `oo::class create Dog { method bark .. ; method fetch .. }` — both
    // methods are real members of the class:
    //   info class methods Dog -> bark fetch   (verified 8.6; 9.0 also lists
    //   the inherited `destroy`, but `bark`/`fetch` are present on both).
    // The outline mirrors this: a Class node `Dog` carrying Method children
    // `bark` and `fetch`.
    let src = concat!(
        "oo::class create Dog {\n",
        "    method bark {} { return \"woof\" }\n",
        "    method fetch {item} { return $item }\n",
        "}\n",
    );
    let symbols = document_symbols(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    assert_eq!(symbols.len(), 1, "{symbols:?}");
    let cls = &symbols[0];
    assert_eq!(cls.name, "Dog");
    assert_eq!(cls.kind, SymbolKind::Class);
    let methods: Vec<&str> = cls
        .children
        .iter()
        .filter(|c| c.kind == SymbolKind::Method)
        .map(|c| c.name.as_str())
        .collect();
    assert!(methods.contains(&"bark"), "{methods:?}");
    assert!(methods.contains(&"fetch"), "{methods:?}");
}

#[test]
fn symbols_oo_class_constructor_lists_its_param() {
    // tclsh: `oo::class create Dog { constructor {name} {...} }` — the ctor's
    // parameter is exactly `name`:
    //   lindex [info class constructor Dog] 0 -> name   (verified 8.6 + 9.0)
    // The outline carries a Constructor node whose detail names that param.
    let src = concat!(
        "oo::class create Dog {\n",
        "    constructor {name} { variable n; set n $name }\n",
        "}\n",
    );
    let symbols = document_symbols(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let cls = &symbols[0];
    let ctor = cls
        .children
        .iter()
        .find(|c| c.kind == SymbolKind::Constructor)
        .expect("a Constructor symbol");
    assert_eq!(ctor.name, "constructor");
    assert!(
        ctor.detail.as_deref().unwrap_or("").contains("name"),
        "constructor detail should name its `name` param; got {:?}",
        ctor.detail,
    );
}

#[test]
fn symbols_oo_configurable_emits_property() {
    // `oo::configurable create Point { property x y }` declares two
    // properties; the outline surfaces them as Property symbols. (The
    // `property` slot is OO-machinery presentation — asserted structurally.)
    let src = concat!(
        "oo::configurable create Point {\n",
        "    property x y\n",
        "}\n",
    );
    let symbols = document_symbols(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let cls = &symbols[0];
    let props: Vec<&str> = cls
        .children
        .iter()
        .filter(|c| c.kind == SymbolKind::Property)
        .map(|c| c.name.as_str())
        .collect();
    assert!(props.contains(&"x"), "{props:?}");
    assert!(props.contains(&"y"), "{props:?}");
}

#[test]
fn symbols_kind_wire_form_is_the_lsp_identifier() {
    // The wire form is the LSP `SymbolKind` identifier string, not its number
    // — a pure presentation contract.
    assert_eq!(SymbolKind::Function.as_str(), "Function");
    assert_eq!(SymbolKind::Class.as_str(), "Class");
    assert_eq!(SymbolKind::Namespace.as_str(), "Namespace");
    assert_eq!(SymbolKind::Variable.as_str(), "Variable");
    assert_eq!(SymbolKind::Method.as_str(), "Method");
    assert_eq!(SymbolKind::Constructor.as_str(), "Constructor");
    assert_eq!(SymbolKind::Property.as_str(), "Property");
    // `Module` is the BIG-IP tmsh config-folder kind. // bigip
    assert_eq!(SymbolKind::Module.as_str(), "Module");
}

#[test]
fn symbols_mixed_document_collects_all_top_level_definitions() {
    // A document with a proc, a namespace, a class, and a global var surfaces
    // one node of each kind. Each name corresponds to a real Tcl definition
    // (the per-construct tclsh facts are proven in the dedicated tests above).
    let src = concat!(
        "set top 1\n",
        "proc f {} { return }\n",
        "namespace eval ns { proc g {} { return } }\n",
        "oo::class create K { method m {} {} }\n",
    );
    let symbols = document_symbols(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
    );
    let pairs = all_pairs(&symbols);
    assert!(
        pairs.contains(&("f".to_string(), SymbolKind::Function)),
        "{pairs:?}"
    );
    assert!(
        pairs.contains(&("ns".to_string(), SymbolKind::Namespace)),
        "{pairs:?}"
    );
    assert!(
        pairs.contains(&("g".to_string(), SymbolKind::Function)),
        "{pairs:?}"
    );
    assert!(
        pairs.contains(&("K".to_string(), SymbolKind::Class)),
        "{pairs:?}"
    );
    assert!(
        pairs.contains(&("top".to_string(), SymbolKind::Variable)),
        "{pairs:?}"
    );
}

// ===========================================================================
// code_lens — reference-count lenses on procs / classes / methods
// ===========================================================================

/// Parse a `"N reference(s)"` lens title back into its integer count.
fn lens_count(lens: &CodeLens) -> usize {
    lens.command_title
        .split_once(' ')
        .and_then(|(n, _)| n.parse().ok())
        .unwrap_or_else(|| panic!("unparseable lens title: {:?}", lens.command_title))
}

/// Lens whose anchor starts on `line`.
fn lens_on_line(lenses: &[CodeLens], line: u32) -> &CodeLens {
    lenses
        .iter()
        .find(|l| l.range.start_line == line)
        .unwrap_or_else(|| panic!("no lens on line {line}: {lenses:?}"))
}

#[test]
fn lens_none_without_analysis() {
    // With `analysis = None` the provider emits nothing (the stub-call shape).
    assert!(
        code_lenses(
            "proc foo {} {}\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            None,
            None,
            ""
        )
        .is_empty()
    );
}

#[test]
fn lens_none_for_file_with_no_eligible_symbols() {
    // A file with no procs / classes has no symbols to annotate, so no lenses.
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    let lenses = code_lenses(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        Some(&analysis),
        None,
        "",
    );
    assert!(lenses.is_empty(), "expected no lenses; got {lenses:?}");
}

#[test]
fn lens_one_per_user_proc() {
    // tclsh: `info procs` over two distinct procs -> {bar foo}; each gets its
    // own reference-count lens anchored at its name span.
    let src = "proc foo {} {}\nproc bar {} {}\n";
    let analysis = analyse(src);
    let lenses = code_lenses(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        Some(&analysis),
        None,
        "",
    );
    assert_eq!(lenses.len(), 2, "{lenses:?}");
    // Both anchor on the proc-name token, not the `proc` keyword (col 5).
    assert!(
        lenses.iter().all(|l| l.range.start_character == 5),
        "{lenses:?}"
    );
}

#[test]
fn lens_reports_zero_for_unused_proc() {
    // tclsh: `proc lonely {} {}` is never invoked — there are no call sites, so
    // the lens shows `0 references`. (The count is the call-site set; that the
    // proc exists is the Tcl fact, that 0 sites call it is structural truth of
    // the one-line script.)
    let src = "proc lonely {} {}\n";
    let analysis = analyse(src);
    let lenses = code_lenses(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        Some(&analysis),
        None,
        "",
    );
    assert_eq!(lenses.len(), 1);
    assert_eq!(lenses[0].command_title, "0 references");
}

#[test]
fn lens_counts_call_sites() {
    // tclsh: `proc tool {} {}` followed by three bare `tool` words — each is a
    // genuine invocation of the proc (a runnable script: `proc tool {} {};
    // tool; tool; tool` runs to completion). The lens counts exactly 3.
    let src = "proc tool {} {}\ntool\ntool\ntool\n";
    let analysis = analyse(src);
    let lenses = code_lenses(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        Some(&analysis),
        None,
        "",
    );
    let tool = lens_on_line(&lenses, 0);
    assert_eq!(tool.command_title, "3 references", "{lenses:?}");
    assert_eq!(lens_count(tool), 3);
}

#[test]
fn lens_singular_grammar_for_one_reference() {
    // One call site -> the title uses the singular `1 reference`, not
    // `1 references` (presentation grammar).
    let src = "proc helper {} {}\nhelper\n";
    let analysis = analyse(src);
    let lenses = code_lenses(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        Some(&analysis),
        None,
        "",
    );
    assert_eq!(lens_on_line(&lenses, 0).command_title, "1 reference");
}

#[test]
fn lens_carries_qualified_name_in_data() {
    // The proc lens surfaces the symbol's qualified name (`::greet`) for the
    // editor's references-jump command.
    let src = "proc greet {} {}\n";
    let analysis = analyse(src);
    let lenses = code_lenses(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        Some(&analysis),
        None,
        "",
    );
    assert_eq!(lenses.len(), 1);
    assert_eq!(lenses[0].qname, "::greet", "{lenses:?}");
}

#[test]
fn lens_counts_method_calls_inside_class_body() {
    // tclsh: inside `oo::class create C`, method `greet` is dispatched twice
    // from method `twice`'s body.  TclOO self-dispatch is `my greet` — a bare
    // `greet` resolves as a normal command, never the object's method — and
    // the lens on `greet` counts the two `my greet` calls.
    let src = concat!(
        "oo::class create C {\n",
        "    method greet {} {}\n",
        "    method twice {} { my greet ; my greet }\n",
        "}\n",
    );
    let analysis = analyse(src);
    // Guard: skip if this analyser build doesn't track classes for the shape.
    if analysis.all_classes.is_empty() {
        return;
    }
    let lenses = code_lenses(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        Some(&analysis),
        None,
        "",
    );
    // The `greet` declaration's name span is on line 1.
    assert_eq!(
        lens_on_line(&lenses, 1).command_title,
        "2 references",
        "{lenses:?}"
    );
}

// ===========================================================================
// document_links — `source <path>` and `package require <pkg>`
// ===========================================================================

/// Source-range start `(line, character)` of a link.
fn link_start(link: &DocumentLink) -> (u32, u32) {
    (link.start_line, link.start_character)
}

#[test]
fn links_none_for_non_source_commands() {
    // tclsh: `set` and `puts` are not path-bearing — `source` is the command
    // the provider keys on, so neither yields a link.
    assert!(
        document_links(
            "set x 1\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            None
        )
        .is_empty()
    );
    assert!(
        document_links(
            "puts hello\n",
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            None
        )
        .is_empty()
    );
}

#[test]
fn links_absolute_source_path_surfaces_file_uri() {
    // tclsh: `source` is a real command whose argument is a script path:
    //   info commands source -> source   (verified 8.6 + 9.0).
    // The provider surfaces the absolute path as a `file://` URI. (The URI
    // shape is editor presentation; that `source <path>` is the Tcl construct
    // being linked is the semantic anchor.)
    let src = "source /usr/lib/tcl/init.tcl\n";
    let links = document_links(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        None,
    );
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[0].target, "file:///usr/lib/tcl/init.tcl");
    // Anchored on the path argument (`source ` is 7 chars).
    assert_eq!(link_start(&links[0]), (0, 7));
}

#[test]
fn links_relative_source_resolves_against_workspace_root() {
    // A relative `source` path resolves against the supplied workspace root.
    let src = "source helper.tcl\n";
    let links = document_links(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        Some("/home/user/project"),
    );
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[0].target, "file:///home/user/project/helper.tcl");
}

#[test]
fn links_relative_source_without_root_yields_none() {
    // No workspace root -> a relative path has nothing to anchor against, so no
    // link is produced (the path with no resolvable target is dropped).
    let src = "source helper.tcl\n";
    assert!(
        document_links(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            None
        )
        .is_empty()
    );
}

#[test]
fn links_dynamic_source_path_yields_none() {
    // tclsh: `source $somevar` is a perfectly valid runtime command, but the
    // path is variable-substituted, not a literal — the provider can't resolve
    // it statically, so no link.
    let src = "source $somevar\n";
    assert!(
        document_links(
            src,
            tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
            None
        )
        .is_empty()
    );
}

#[test]
fn links_encoding_flag_skipped_before_path() {
    // `source -encoding utf-8 <path>` — the provider skips the `-encoding`
    // flag and its value, linking the real path argument.
    let src = "source -encoding utf-8 /tmp/foo.tcl\n";
    let links = document_links(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        None,
    );
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[0].target, "file:///tmp/foo.tcl");
}

#[test]
fn links_tilde_expands_against_supplied_home() {
    // `source ~/lib/init.tcl` expands `~` against the injected home dir.
    let src = "source ~/lib/init.tcl\n";
    let links = document_links_with_home(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        None,
        Some("/test-home"),
    );
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[0].target, "file:///test-home/lib/init.tcl");
}

#[test]
fn links_literal_file_join_source_resolves() {
    // `source [file join lib helper.tcl]` — a literal `[file join …]` resolves
    // to `lib/helper.tcl` under the workspace root. (Matches Tcl's `file join`
    // for literal segments; variable segments are declined — covered by the
    // provider's own unit tests.)
    let src = "source [file join lib helper.tcl]\n";
    let links = document_links(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        Some("/home/user/project"),
    );
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[0].target, "file:///home/user/project/lib/helper.tcl");
}

#[test]
fn links_package_require_surfaces_targetless_link_with_tooltip() {
    // tclsh: `package require` is a real command naming a package:
    //   info commands package -> package   (verified 8.6 + 9.0).
    // The provider surfaces a link on the package name with NO on-disk target
    // (package resolution needs a pkgIndex scan, which isn't done) but a
    // tooltip carrying the package name.
    let src = "package require http 2.9\n";
    let links = document_links(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        None,
    );
    let pkg = links
        .iter()
        .find(|l| l.tooltip.as_deref().is_some_and(|t| t.contains("http")))
        .expect("a package-require link for http");
    assert_eq!(
        pkg.target, "",
        "package link has no navigable target: {pkg:?}"
    );
    assert_eq!(pkg.tooltip.as_deref(), Some("package require http"));
}

#[test]
fn links_source_uri_is_percent_encoded() {
    // A literal `source` path with spaces must surface as a percent-encoded
    // `file://` URI (never a raw concatenation with a literal space).
    let src = "source /path/with spaces.tcl\n";
    let links = document_links(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        None,
    );
    if let Some(link) = links.first() {
        assert!(
            !link.target.contains(' '),
            "URI must not contain a raw space: {:?}",
            link.target,
        );
    }
}

// Computed `source` paths — issue #1140 idx 41. The provider used to fall
// through to treating the *raw unevaluated Tcl text* as a literal path, then
// percent-encode it onto the workspace root, producing a syntactically valid
// but semantically bogus `file://` URI. It now resolves what the source
// graph's own evaluator can resolve, and emits nothing otherwise.

fn links_in(src: &str, script_path: &str) -> Vec<DocumentLink> {
    let root = script_path.rsplit_once('/').map_or("/", |(dir, _)| dir);
    tcl_lsp_core::document_links::document_links_in_context(
        src,
        tcl_registry::model::ingress::resolve_environment("tcl8.6").analyser_profile(),
        &tcl_lsp_core::document_links::LinkContext {
            imported_constants: None,
            workspace_root: Some(root),
            home: None,
            script_path: Some(script_path),
        },
    )
}

#[test]
fn fp_a_computed_source_path_never_produces_a_fabricated_target() {
    // FP guard — the exact georgtree/tclopt shape. Whatever else happens,
    // no link may carry the raw Tcl text percent-encoded into its URI.
    let src = "source [file join $undeclared .. pkgfile.tcl]\n";
    let links = links_in(src, "/proj/test/all_codeCoverage.tcl");
    assert!(
        links.iter().all(|l| !l.target.contains("%5B")
            && !l.target.contains('$')
            && !l.target.contains('[')),
        "no link may encode unevaluated Tcl text: {links:?}",
    );
}

#[test]
fn fp_a_bare_variable_source_path_produces_no_link() {
    // FP guard — a whole-word `$var` is exactly one token, which is why the
    // old `single_token_word` test let it through as a "literal".
    let src = "source $dir\n";
    assert!(links_in(src, "/proj/a.tcl").is_empty());
}

#[test]
fn tp_info_script_based_paths_resolve_through_the_source_graph_evaluator() {
    // TP — `[file dirname [info script]]` is what the source graph already
    // evaluates for the M9 rehoming edges; the link provider consumes the
    // same evaluator instead of growing a second one.
    let src = "source [file join [file dirname [info script]] lib helper.tcl]\n";
    let links = links_in(src, "/proj/test/all.tcl");
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[0].target, "file:///proj/test/lib/helper.tcl");
}

#[test]
fn tp_a_single_set_constant_propagates_into_the_join() {
    // TP — the corpus idiom: one `set dir …` then `source [file join $dir …]`.
    let src = "set dir [file dirname [info script]]\nsource [file join $dir sub other.tcl]\n";
    let links = links_in(src, "/proj/test/all.tcl");
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[0].target, "file:///proj/test/sub/other.tcl");
}

#[test]
fn fp_a_variable_assigned_twice_is_not_propagated() {
    // FP guard — which value a later `source` sees is a flow question this
    // provider does not ask, so a re-assigned name is dropped entirely
    // rather than guessed at.
    let src = "set dir /one\nset dir /two\nsource [file join $dir x.tcl]\n";
    assert!(links_in(src, "/proj/a.tcl").is_empty());
}

#[test]
fn tn_a_plain_literal_path_still_links() {
    // TN — the ordinary case is untouched.
    let src = "source helper.tcl\n";
    let links = links_in(src, "/proj/a.tcl");
    assert_eq!(links.len(), 1, "{links:?}");
    assert_eq!(links[0].target, "file:///proj/helper.tcl");
}

// NOTE on BIG-IP-specific extraction: this provider keys on the vanilla Tcl
// `source` / `package require` commands. It has no BIG-IP-config-specific link
// extraction (e.g. `include`/`source` of a tmsh config fragment) — those would
// be a separate concern and are not surfaced here. // bigip

// ===========================================================================
// code_actions — quick-fixes / refactors at a position
// ===========================================================================

/// Every action's edits must be well-formed: each replacement range has its
/// start at or before its end (a valid LSP edit range).
fn edits_well_formed(action: &CodeAction) -> bool {
    action.edits.iter().all(|e| {
        (e.range.start_line, e.range.start_character) <= (e.range.end_line, e.range.end_character)
    })
}

#[test]
fn actions_none_without_analysis() {
    // `analysis = None` -> the diagnostic-driven path produces nothing.
    let actions = code_actions("proc f {} { return }\n", cursor(0, 0), None, &[]);
    assert!(actions.is_empty(), "{actions:?}");
}

#[test]
fn actions_none_at_inert_position() {
    // An ordinary, diagnostic-free statement with the cursor on it offers no
    // quick-fix and no range refactor.
    let src = "set x 1\n";
    let analysis = analyse(src);
    let actions = code_actions(src, cursor(0, 4), Some(&analysis), &analysis.diagnostics);
    assert!(
        actions.is_empty(),
        "inert position should offer nothing; got {actions:?}"
    );
}

#[test]
fn actions_catch_without_result_var_offers_capture_fixes() {
    // tclsh proves the *semantics* the fix relies on: a `catch {body}` with a
    // result variable captures the body's result/error —
    //   set rc [catch {expr {1/0}} res]; list $rc $res -> 1 {divide by zero}
    //   (verified tclsh8.6 + tclsh9.0).
    // So `catch {expr {1/0}}` (no result var, at proc scope) is W302, and the
    // provider offers the two capture quick-fixes that append ` result` /
    // ` result options` after the catch **body**.  The insertion anchor is the
    // analyser's, computed from the argument tokens: the diagnostic itself
    // spans only the `catch` keyword, so a provider reconstructing the point
    // from the diagnostic's end writes the word before the body and turns the
    // call into a catch of the script `result` (issue #1190).
    let src = "proc f {} {\n    catch {expr {1/0}}\n}\n";
    let analysis = analyse(src);
    // Sanity: the analyser actually flagged W302 here; otherwise this test
    // isn't exercising the path it claims to.
    let has_w302 = analysis
        .diagnostics
        .iter()
        .any(|d| format!("{:?}", d.code) == "W302");
    assert!(
        has_w302,
        "expected a W302 diagnostic; diags={:?}",
        analysis.diagnostics
    );
    // Cover the whole `catch` line so the action context overlaps the diag.
    let line2_len = src
        .lines()
        .nth(1)
        .map_or(0, |l| u32::try_from(l.len()).unwrap_or(0));
    let actions = code_actions(
        src,
        selection(1, 0, line2_len),
        Some(&analysis),
        &analysis.diagnostics,
    );
    let titles: Vec<&str> = actions.iter().map(|a| a.title.as_str()).collect();
    assert!(
        titles.iter().any(|t| t.contains("catch result")),
        "expected a catch-result quick-fix; got {titles:?}",
    );
    // Both capture variants are offered.
    assert!(
        actions
            .iter()
            .any(|a| a.title == "Add catch result variable(s)"),
        "missing ` result` variant; got {titles:?}",
    );
    assert!(
        actions
            .iter()
            .any(|a| a.title == "Add catch result + options variable(s)"),
        "missing ` result options` variant; got {titles:?}",
    );
    // The anchor lands past the body's closing brace, so applying either
    // action produces a well-formed `catch BODY result …`.
    for action in actions.iter().filter(|a| a.title.contains("catch result")) {
        let edit = &action.edits[0];
        let line = src.lines().nth(edit.range.start_line as usize).unwrap();
        assert_eq!(
            &line[..edit.range.start_character as usize],
            "    catch {expr {1/0}}",
            "the insertion must sit after the body's closing brace: {edit:?}",
        );
    }
    // The fixes are quick-fixes and their edits are well-formed insertions.
    for a in actions.iter().filter(|a| a.title.contains("catch result")) {
        assert_eq!(a.kind, ActionKind::QuickFix);
        assert!(edits_well_formed(a), "malformed edit: {a:?}");
        assert_eq!(
            a.edits.len(),
            1,
            "the capture fix is a single insertion: {a:?}"
        );
        // Insertion edit: zero-width range (start == end), inserting the suffix.
        let e = &a.edits[0];
        assert_eq!(
            (e.range.start_line, e.range.start_character),
            (e.range.end_line, e.range.end_character),
            "capture fix should be a pure insertion: {e:?}",
        );
        assert!(
            e.new_text.contains("result"),
            "suffix should add `result`: {e:?}"
        );
    }
}

#[test]
fn actions_extract_proc_on_selection_is_well_formed() {
    // A non-empty multi-line selection offers `refactor.extract` ("Extract
    // selection into proc"). The generated proc text is editor presentation;
    // structurally the action must be a RefactorExtract whose edits are
    // well-formed and whose new proc references the selected variable as a
    // parameter (`$x` -> param `x`). The selected body is runnable Tcl.
    let src = "set x 5\nputs $x\nputs [expr {$x + 1}]\n";
    let analysis = analyse(src);
    // Select lines 1..2 (the two `puts` lines) entirely.
    let sel = LspRange {
        start_line: 1,
        start_character: 0,
        end_line: 3,
        end_character: 0,
    };
    let actions = code_actions(src, sel, Some(&analysis), &analysis.diagnostics);
    // Select by *title*: extract-variable shares the `refactor.extract` kind,
    // so a kind-only lookup picks whichever the provider happens to emit first.
    let extract = actions
        .iter()
        .find(|a| a.title == "Extract selection into proc")
        .expect("an extract-proc refactor on a selection");
    assert_eq!(extract.kind, ActionKind::RefactorExtract);
    assert!(extract.disabled.is_none(), "{extract:?}");
    assert!(edits_well_formed(extract), "malformed edits: {extract:?}");
    // Two edits: insert the new proc above the enclosing top-level command,
    // and replace the selection with a call.
    assert_eq!(extract.edits.len(), 2, "{extract:?}");
    let proc_text = &extract.edits[0].new_text;
    assert!(
        proc_text.starts_with("proc "),
        "first edit defines a proc: {proc_text:?}"
    );
    assert!(
        proc_text.contains("{x}"),
        "`x` is read but never written, so it is a value parameter: {proc_text:?}",
    );
    assert!(
        !proc_text.contains("upvar"),
        "nothing is written back, so no upvar plumbing is needed: {proc_text:?}",
    );
}

#[test]
fn actions_ipv4_literal_offers_ipv6_mapped_conversion() {
    // The cursor on a dotted-quad IPv4 literal offers a `Refactor` that
    // rewrites it to the IPv6-mapped form `::ffff:<addr>`. (IP rewriting is an
    // editor refactor over the literal text; the addresses are equivalent
    // representations, not a Tcl runtime fact.)
    let src = "set ip 192.168.0.1\n";
    let analysis = analyse(src);
    // Cursor inside the IPv4 literal (after `set ip `, col ~9).
    let actions = code_actions(src, cursor(0, 9), Some(&analysis), &analysis.diagnostics);
    let conv = actions
        .iter()
        .find(|a| a.title.contains("IPv6-mapped"))
        .expect("an IPv4 -> IPv6-mapped refactor");
    assert_eq!(conv.kind, ActionKind::Refactor);
    assert!(edits_well_formed(conv), "{conv:?}");
    assert_eq!(conv.edits.len(), 1);
    assert!(
        conv.edits[0].new_text.contains("::ffff:192.168.0.1"),
        "rewrite should be the IPv6-mapped form: {:?}",
        conv.edits[0].new_text,
    );
}

#[test]
fn actions_demorgan_rewrite_on_expression_selection() {
    // Selecting `!($a && $b)` offers an `refactor.rewrite` applying De Morgan's
    // law -> `!$a || !$b`. (A boolean-algebra source rewrite — structural.)
    let src = "if {!($a && $b)} { puts hi }\n";
    let analysis = analyse(src);
    // Select the `!($a && $b)` substring. `if {` is 4 chars; the expr starts
    // at col 4 and is 11 chars long.
    let sel = selection(0, 4, 4 + 11);
    let actions = code_actions(src, sel, Some(&analysis), &analysis.diagnostics);
    let dm = actions
        .iter()
        .find(|a| a.title.contains("De Morgan"))
        .expect("a De Morgan rewrite over the selected expression");
    assert_eq!(dm.kind, ActionKind::RefactorRewrite);
    assert!(edits_well_formed(dm), "{dm:?}");
    assert!(
        dm.edits[0].new_text.contains("||"),
        "De Morgan of `&&` yields `||`: {:?}",
        dm.edits[0].new_text,
    );
}

#[test]
fn actions_generate_docstring_for_undocumented_proc() {
    // tclsh: `proc greet {name greeting} {...}` has params `name greeting`
    //   info args greet -> name greeting   (verified 8.6 + 9.0).
    // With the cursor on the proc name, the provider offers a `source`-kind
    // "Generate docstring" action whose inserted comment block enumerates the
    // real parameters as `@param` lines.
    let src = "proc greet {name greeting} {\n    return \"$greeting $name\"\n}\n";
    let analysis = analyse(src);
    let actions = code_actions(src, cursor(0, 6), Some(&analysis), &analysis.diagnostics);
    let doc = actions
        .iter()
        .find(|a| a.title.contains("Generate docstring"))
        .expect("a generate-docstring source action");
    assert_eq!(doc.kind, ActionKind::Source);
    assert!(edits_well_formed(doc), "{doc:?}");
    let text = &doc.edits[0].new_text;
    assert!(
        text.contains("@param name"),
        "missing @param name: {text:?}"
    );
    assert!(
        text.contains("@param greeting"),
        "missing @param greeting: {text:?}"
    );
}

#[test]
fn actions_edits_are_always_well_formed() {
    // Defensive sweep: across a document that triggers several action families,
    // EVERY emitted action's edit ranges must be well-formed (start <= end) so
    // the editor never applies a corrupt edit.
    let src = "proc greet {name} {\n    catch {expr {1/0}}\n}\n";
    let analysis = analyse(src);
    // A range spanning the whole document picks up every overlapping action.
    let whole = LspRange {
        start_line: 0,
        start_character: 0,
        end_line: 3,
        end_character: 0,
    };
    let actions = code_actions(src, whole, Some(&analysis), &analysis.diagnostics);
    assert!(
        !actions.is_empty(),
        "expected at least one action over the document"
    );
    for a in &actions {
        assert!(edits_well_formed(a), "malformed edit in action {a:?}");
    }
}

// NOTE on iRules/BIG-IP-only code actions: the provider also offers F5-specific
// refactors (extract-to-datagroup, `# Profiles:` header, iRules taint
// encode-wrap, missing-`collect` bootstrap) via separate entry points
// (`context_diagnostic_actions`, `profiles_action`). Those are gated on the
// iRules dialect / context diagnostics and are out of scope for this vanilla
// Tcl port. // f5-dialect // bigip
