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

//! Integration tests for the remaining `tcl-lsp-core` LSP providers:
//! `code_lens`, `document_symbols`, `code_actions`, `declaration`,
//! `type_definition`, `implementation`, `type_hierarchy`, and
//! `linked_editing_range`.
//!
//! Harness: every test runs the real analyser
//! (`Analyser::new().analyse(src, "tcl8.6")`) over a complete, runnable Tcl
//! script and then calls the provider exactly as the server would. The
//! `code_lens` block covers the code-lens provider; the rest are fresh
//! behaviour-driven tests.
//!
//! C-Tcl proof. The facts these providers surface are Tcl-semantic, and each is
//! pinned to real `tclsh` (8.6 / 9.0) output, cited inline at each test as
//! `// tclsh:`. The ground-truth scripts, all complete and runnable, are:
//! a proc's real definition site —
//! `proc greet {name} { puts hi }; list [info args greet] [info body greet]`
//! yields `name { puts hi }`;
//! a `TclOO` class's real methods —
//! `oo::class create C { method m {} {} }; info class methods C` yields `m`;
//! a `TclOO` class's real superclasses and subclasses —
//! `oo::class create Base {}; oo::class create Sub { superclass Base }` then
//! `info class superclass Sub` yields `::Base` and `info class subclasses Base`
//! yields `::Sub`; the real namespace command tree —
//! `namespace eval outer { namespace eval inner {} }; namespace children ::outer`
//! yields `::outer::inner`.
//!
//! Editor-presentation facts (`SymbolKind` icon strings, lens title wording, the
//! order symbols are listed in, the linked-editing word-pattern regex) are
//! structural, not Tcl-runtime values — `tclsh` has no opinion on them. Those are
//! asserted structurally (kind enum, range containment, count) and called out as
//! such. No F5/iRules-only behaviour is exercised here, so there are no
//! `// f5-dialect` markers.

use tcl_compiler::analyser::{Analyser, AnalysisResult};
use tcl_registry::CommandRegistry;

use tcl_lsp_core::code_actions::{ActionKind, CodeAction, code_actions};
use tcl_lsp_core::code_lens::{CodeLens, code_lenses};
use tcl_lsp_core::declaration::declaration;
use tcl_lsp_core::definition::LspRange;
use tcl_lsp_core::document_symbols::{DocumentSymbol, LineRange, SymbolKind, document_symbols};
use tcl_lsp_core::implementation::implementation;
use tcl_lsp_core::linked_editing_range::{WORD_PATTERN, linked_editing_ranges};
use tcl_lsp_core::type_definition::type_definition;
use tcl_lsp_core::type_hierarchy::prepare as type_hierarchy_prepare;

// -- shared helpers --------------------------------------------------------

fn analyse(source: &str) -> AnalysisResult {
    let mut a = Analyser::new();
    a.analyse(source, "tcl8.6").clone()
}

fn registry() -> CommandRegistry {
    CommandRegistry::build_default()
}

/// (line, character) of the `occurrence`-th (1-based) occurrence of `needle`
/// in `src`, for cursor placement — the same convention the provider unit
/// tests use.
fn pos_of(src: &str, needle: &str, occurrence: usize) -> (u32, u32) {
    let mut start = 0;
    for _ in 0..occurrence {
        let idx = src[start..].find(needle).expect("needle not found") + start;
        start = idx + 1;
    }
    let idx = start - 1;
    let prefix = &src[..idx];
    let line = u32::try_from(prefix.matches('\n').count()).unwrap();
    let col = u32::try_from(idx - prefix.rfind('\n').map_or(0, |n| n + 1)).unwrap();
    (line, col)
}

/// `outer` (a document-symbol range) contains `inner`.
fn line_range_contains(outer: LineRange, inner: LineRange) -> bool {
    let starts_le =
        (outer.start_line, outer.start_character) <= (inner.start_line, inner.start_character);
    let ends_ge = (outer.end_line, outer.end_character) >= (inner.end_line, inner.end_character);
    starts_le && ends_ge
}

/// Recursively collect every symbol name in a document-symbol tree.
fn collect_names<'a>(symbols: &'a [DocumentSymbol], out: &mut Vec<&'a str>) {
    for s in symbols {
        out.push(s.name.as_str());
        collect_names(&s.children, out);
    }
}

// =========================================================================
// code_lens
//
// The Rust `code_lenses` provider already resolves the count eagerly, so the
// title text ("3 references", "1 reference", …) is asserted directly on the
// lens here. The lens TITLE wording and the empty
// `command` string are editor-presentation — asserted structurally. The
// reference COUNT is the Tcl-semantic fact: it is the number of real call
// sites of the proc in the script.
// =========================================================================

const TEST_URI: &str = "file:///test.tcl";

fn lens_on_line(lenses: &[CodeLens], line: u32) -> &CodeLens {
    lenses
        .iter()
        .find(|l| l.range.start_line == line)
        .unwrap_or_else(|| panic!("no lens on line {line}: {lenses:?}"))
}

#[test]
fn code_lens_lens_per_proc() {
    // one lens per proc.
    let src = "proc greet {} { return hi }\nproc shout {} { return HI }\n";
    let analysis = analyse(src);
    let lenses = code_lenses(src, tcl_dialect::DialectProfile::by_name("tcl8.6"), Some(&analysis), None, TEST_URI);
    let proc_lenses: Vec<&CodeLens> = lenses
        .iter()
        .filter(|l| l.range.start_line == 0 || l.range.start_line == 1)
        .collect();
    assert_eq!(proc_lenses.len(), 2, "{lenses:?}");
    for lens in &proc_lenses {
        // Informational lens: the command identifier is empty (no command
        // bound). Editor-presentation, asserted structurally.
        assert_eq!(lens.command, "");
        assert!(lens.command_title.contains("reference"));
    }
}

#[test]
fn code_lens_empty_when_no_procs() {
    let src = "set x 1\n";
    let analysis = analyse(src);
    let lenses = code_lenses(src, tcl_dialect::DialectProfile::by_name("tcl8.6"), Some(&analysis), None, TEST_URI);
    assert!(lenses.is_empty(), "{lenses:?}");
}

#[test]
fn code_lens_none_analysis_returns_empty() {
    // The Rust provider returns an empty vec for `analysis = None`; the caller
    // always threads analysis, so the contract here is empty-on-None (matching
    // `code_lens.rs::empty_lenses_when_analysis_is_none`).
    let lenses = code_lenses(
        "proc f {} {}\nproc g {} {}\n",
        tcl_dialect::DialectProfile::by_name("tcl8.6"),
        None,
        None,
        TEST_URI,
    );
    assert!(lenses.is_empty());
}

#[test]
fn code_lens_zero_references_for_unused_proc() {
    // tclsh: `greet` is never invoked → 0 call sites.
    let src = "proc greet {} { return hi }\n";
    let analysis = analyse(src);
    let lenses = code_lenses(src, tcl_dialect::DialectProfile::by_name("tcl8.6"), Some(&analysis), None, TEST_URI);
    assert_eq!(lens_on_line(&lenses, 0).command_title, "0 references");
}

#[test]
fn code_lens_singular_title_for_one_reference() {
    // singular noun.
    let src = "proc greet {} { return hi }\ngreet\n";
    let analysis = analyse(src);
    let lenses = code_lenses(src, tcl_dialect::DialectProfile::by_name("tcl8.6"), Some(&analysis), None, TEST_URI);
    assert_eq!(lens_on_line(&lenses, 0).command_title, "1 reference");
}

#[test]
fn code_lens_plural_title_for_multiple_references() {
    // multiple references (count=3).
    let src = "proc greet {} { return hi }\ngreet\ngreet\ngreet\n";
    let analysis = analyse(src);
    let lenses = code_lenses(src, tcl_dialect::DialectProfile::by_name("tcl8.6"), Some(&analysis), None, TEST_URI);
    assert_eq!(lens_on_line(&lenses, 0).command_title, "3 references");
}

#[test]
fn code_lens_anchor_is_proc_name_span() {
    // The lens anchors on the proc NAME token (the start of the name range).
    // `greet` begins at column 5.
    let src = "proc greet {} {}\n";
    let analysis = analyse(src);
    let lenses = code_lenses(src, tcl_dialect::DialectProfile::by_name("tcl8.6"), Some(&analysis), None, TEST_URI);
    let lens = lens_on_line(&lenses, 0);
    assert_eq!(lens.range.start_character, 5);
    assert_eq!(lens.qname, "::greet");
}

#[test]
fn code_lens_count_matches_forward_reference() {
    // A call BEFORE the definition is still a real call site (issue #637).
    // tclsh: `proc foo {} {}; foo` — `foo` resolves and runs regardless of
    // source order, so the forward call counts.
    let src = "foo\nproc foo {} {}\n";
    let analysis = analyse(src);
    let lenses = code_lenses(src, tcl_dialect::DialectProfile::by_name("tcl8.6"), Some(&analysis), None, TEST_URI);
    assert_eq!(lens_on_line(&lenses, 1).command_title, "1 reference");
}

#[test]
fn code_lens_count_matches_qualified_call() {
    // `::foo` is the same proc as `foo`.
    // tclsh: `proc foo {} {}; ::foo` invokes `::foo`.
    let src = "proc foo {} {}\n::foo\n";
    let analysis = analyse(src);
    let lenses = code_lenses(src, tcl_dialect::DialectProfile::by_name("tcl8.6"), Some(&analysis), None, TEST_URI);
    assert_eq!(lens_on_line(&lenses, 0).command_title, "1 reference");
}

#[test]
fn code_lens_count_matches_ns_qualified_call() {
    // tclsh: `namespace eval ns { proc foo {} {} }; ns::foo` runs `::ns::foo`.
    let src = "namespace eval ns { proc foo {} {} }\nns::foo\n";
    let analysis = analyse(src);
    let lenses = code_lenses(src, tcl_dialect::DialectProfile::by_name("tcl8.6"), Some(&analysis), None, TEST_URI);
    let foo = lenses
        .iter()
        .find(|l| l.qname.contains("foo"))
        .unwrap_or_else(|| panic!("no foo lens: {lenses:?}"));
    assert_eq!(foo.command_title, "1 reference", "{lenses:?}");
}

#[test]
fn code_lens_count_matches_call_inside_body() {
    let src = "proc foo {} {}\nproc bar {} { foo }\n";
    let analysis = analyse(src);
    let lenses = code_lenses(src, tcl_dialect::DialectProfile::by_name("tcl8.6"), Some(&analysis), None, TEST_URI);
    assert_eq!(lens_on_line(&lenses, 0).command_title, "1 reference");
}

#[test]
fn code_lens_count_matches_cmd_substitution() {
    // `[foo]` — command substitution.
    // tclsh: `proc foo {} {}; set x [foo]` invokes `foo` inside the
    // command substitution.
    let src = "proc foo {} {}\nset x [foo]\n";
    let analysis = analyse(src);
    let lenses = code_lenses(src, tcl_dialect::DialectProfile::by_name("tcl8.6"), Some(&analysis), None, TEST_URI);
    assert_eq!(lens_on_line(&lenses, 0).command_title, "1 reference");
}

#[test]
fn code_lens_degenerate_inputs_do_not_panic() {
    // Empty / whitespace / lone-comment inputs must not panic and yield no
    // proc lenses.
    for src in ["", "   ", "\n\n", "# just a comment\n"] {
        let analysis = analyse(src);
        let lenses = code_lenses(src, tcl_dialect::DialectProfile::by_name("tcl8.6"), Some(&analysis), None, TEST_URI);
        assert!(lenses.is_empty(), "src {src:?} produced {lenses:?}");
    }
}

// =========================================================================
// document_symbols — proc / namespace / variable / class symbol tree
//
// The symbol TREE STRUCTURE (which names nest under which) mirrors the real
// Tcl command structure: a proc inside `namespace eval ns { ... }` is a child
// of `ns`, a method is a child of its class. Those nesting facts are pinned to
// `namespace children` / `info class methods` below. SymbolKind icon strings
// ("Function", "Namespace", …) are editor-presentation, asserted structurally
// via the kind enum.
// =========================================================================

#[test]
fn doc_symbols_empty_file_is_empty() {
    assert!(document_symbols("", tcl_dialect::DialectProfile::by_name("tcl8.6")).is_empty());
    assert!(document_symbols("   \n\t\n", tcl_dialect::DialectProfile::by_name("tcl8.6")).is_empty());
}

#[test]
fn doc_symbols_proc_is_a_function_with_param_detail() {
    // tclsh: `proc greet {name} { puts hi }; info args greet` → `name`,
    // so the proc's single parameter is `name`. The detail string `(name)`
    // is editor-presentation; the parameter fact comes from `info args`.
    let src = "proc greet {name} {\n    puts \"Hello $name\"\n}\n";
    let symbols = document_symbols(src, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    assert_eq!(symbols.len(), 1, "{symbols:?}");
    assert_eq!(symbols[0].name, "greet");
    assert_eq!(symbols[0].kind, SymbolKind::Function);
    assert_eq!(symbols[0].detail.as_deref(), Some("(name)"));
}

#[test]
fn doc_symbols_proc_range_contains_selection_range() {
    // The outer (fold) range must contain the name selection range —
    // structural invariant for the breadcrumb/outline.
    let src = "proc greet {name} {\n    puts \"Hello $name\"\n}\n";
    let symbols = document_symbols(src, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let proc = &symbols[0];
    assert!(
        line_range_contains(proc.range, proc.selection_range),
        "range {:?} must contain selection {:?}",
        proc.range,
        proc.selection_range,
    );
    // The selection range covers exactly the name token `greet` (cols 5..10).
    assert_eq!(proc.selection_range.start_line, 0);
    assert_eq!(proc.selection_range.start_character, 5);
}

#[test]
fn doc_symbols_namespace_nests_inner_proc() {
    // tclsh: `namespace eval myns { proc helper {} {} }` makes `helper`
    // live under `myns` — `namespace children ::myns`'s proc is reachable as
    // `::myns::helper`. The outline mirrors that: `helper` is a child of the
    // `myns` Namespace symbol.
    let src = concat!(
        "namespace eval myns {\n",
        "    proc helper {} {\n",
        "        return 1\n",
        "    }\n",
        "}\n",
    );
    let symbols = document_symbols(src, tcl_dialect::DialectProfile::by_name("tcl8.6"));
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
        SymbolKind::Function
    );
}

#[test]
fn doc_symbols_nested_namespaces_recurse() {
    // tclsh: `namespace eval outer { namespace eval inner {} }` →
    // `namespace children ::outer` → `::outer::inner`. The outline nests
    // `inner` under `outer`, and `deep` under `inner`.
    let src = concat!(
        "namespace eval outer {\n",
        "    namespace eval inner {\n",
        "        proc deep {} { return }\n",
        "    }\n",
        "}\n",
    );
    let symbols = document_symbols(src, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    assert_eq!(symbols.len(), 1);
    let outer = &symbols[0];
    assert_eq!(outer.name, "outer");
    let inner = outer
        .children
        .iter()
        .find(|c| c.name == "inner")
        .expect("inner namespace");
    assert_eq!(inner.kind, SymbolKind::Namespace);
    assert!(inner.children.iter().any(|c| c.name == "deep"));
}

#[test]
fn doc_symbols_global_set_emits_variable() {
    let src = "set myvar 42\n";
    let symbols = document_symbols(src, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let vars: Vec<&DocumentSymbol> = symbols
        .iter()
        .filter(|s| s.kind == SymbolKind::Variable)
        .collect();
    assert!(vars.iter().any(|s| s.name == "myvar"), "{symbols:?}");
}

#[test]
fn doc_symbols_oo_class_lists_its_methods() {
    // tclsh: `oo::class create Dog { method bark {} {}; method fetch {item} {} }`
    // `info class methods Dog` → `bark fetch`. The outline shows `Dog` as a
    // Class symbol whose Method children are exactly those methods.
    let src = concat!(
        "oo::class create Dog {\n",
        "    method bark {} { return \"woof\" }\n",
        "    method fetch {item} { return $item }\n",
        "}\n",
    );
    let symbols = document_symbols(src, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    assert_eq!(symbols.len(), 1, "{symbols:?}");
    let cls = &symbols[0];
    assert_eq!(cls.name, "Dog");
    assert_eq!(cls.kind, SymbolKind::Class);
    let mut method_names: Vec<&str> = cls
        .children
        .iter()
        .filter(|c| c.kind == SymbolKind::Method)
        .map(|c| c.name.as_str())
        .collect();
    method_names.sort_unstable();
    assert_eq!(method_names, vec!["bark", "fetch"], "{cls:?}");
}

#[test]
fn doc_symbols_oo_class_detail_lists_superclass() {
    // tclsh: `oo::class create Dog { superclass Animal }; info class superclass Dog`
    // → `::Animal`. The Class symbol's detail surfaces the superclass.
    let src = "oo::class create Dog {\n    superclass Animal\n}\n";
    let symbols = document_symbols(src, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let detail = symbols[0].detail.as_deref().unwrap_or("");
    assert!(
        detail.contains("Animal"),
        "expected superclass; got {detail:?}"
    );
}

#[test]
fn doc_symbols_no_panic_on_unbalanced_braces() {
    // A genuinely malformed script (unclosed proc body) must not panic.
    for src in ["proc broken {} {", "namespace eval ns {", "}}}}\n", "{{{{"] {
        let _ = document_symbols(src, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    }
}

#[test]
fn doc_symbols_multiple_top_level_procs() {
    let src = "proc foo {} { return 1 }\nproc bar {} { return 2 }\n";
    let symbols = document_symbols(src, tcl_dialect::DialectProfile::by_name("tcl8.6"));
    let mut names = Vec::new();
    collect_names(&symbols, &mut names);
    assert!(names.contains(&"foo"));
    assert!(names.contains(&"bar"));
}

// =========================================================================
// code_actions — quick-fixes offered for diagnostics
//
// These exercise the analyser→quick-fix path. The TRIGGER conditions are
// Tcl-semantic: `catch { ... }` with no result variable silently swallows
// errors (W302); `unset somevar` on a possibly-undefined variable would raise
// a runtime error without `-nocomplain` (W213). The quick-fix EDIT (the
// inserted text, the title wording) is editor-presentation, asserted
// structurally on the resulting `TextEdit`.
// =========================================================================

/// A code-action range spanning a whole line — the editor sends the cursor's
/// line/selection; a quick-fix is offered when the diagnostic overlaps it.
fn whole_line(line: u32) -> LspRange {
    LspRange {
        start_line: line,
        start_character: 0,
        end_line: line,
        end_character: u32::MAX,
    }
}

#[test]
fn code_actions_none_analysis_is_empty() {
    assert!(code_actions("catch { puts hi }\n", whole_line(0), None, &[]).is_empty());
}

#[test]
fn code_actions_catch_without_result_var_offers_fix() {
    // tclsh: `catch { error boom }` returns a status code and discards the
    // error message unless a result variable is given — `catch { error boom } r`
    // captures it. The provider offers the two W302 quick-fixes that splice a
    // result (and result+options) variable in.
    let src = "catch { puts hi }\n";
    let analysis = analyse(src);
    // Sanity: the analyser actually emitted W302 for this script.
    assert!(
        analysis
            .diagnostics
            .iter()
            .any(|d| d.code == tcl_compiler::compiler_checks::DiagCode::W302),
        "expected a W302 diagnostic; got {:?}",
        analysis.diagnostics,
    );
    let actions = code_actions(src, whole_line(0), Some(&analysis), &analysis.diagnostics);
    let titles: Vec<&str> = actions.iter().map(|a| a.title.as_str()).collect();
    assert!(
        titles.iter().any(|t| t.contains("result")),
        "expected a catch-result quick-fix; got {titles:?}",
    );
    // Both fixes are quickfix-kind insertions (empty replaced range).
    let result_fix: &CodeAction = actions
        .iter()
        .find(|a| a.title == "Add catch result variable(s)")
        .unwrap_or_else(|| panic!("missing result fix; got {titles:?}"));
    assert_eq!(result_fix.kind, ActionKind::QuickFix);
    assert_eq!(result_fix.edits.len(), 1);
    assert_eq!(result_fix.edits[0].new_text, " result");
    // Insertion: zero-width range.
    let r = result_fix.edits[0].range;
    assert_eq!(
        (r.start_line, r.start_character),
        (r.end_line, r.end_character)
    );
    // …and it lands *after the body*, not after the `catch` keyword: applying
    // it must yield `catch { puts hi } result`, never
    // `catch result { puts hi }` (issue #1190).
    let column = r.start_character as usize;
    assert_eq!(&src[..column], "catch { puts hi }", "range {r:?}");
}

#[test]
fn code_actions_catch_with_result_var_offers_no_w302_fix() {
    // tclsh: `catch { puts hi } result` already captures the result — no
    // silent-swallow, so no W302 and no catch-result quick-fix.
    let src = "catch { puts hi } result\n";
    let analysis = analyse(src);
    let actions = code_actions(src, whole_line(0), Some(&analysis), &analysis.diagnostics);
    assert!(
        !actions.iter().any(|a| a.title.contains("catch result")),
        "no catch-result fix expected; got {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>(),
    );
}

#[test]
fn code_actions_unset_possibly_undef_offers_nocomplain() {
    // tclsh: `unset xs` on a never-set variable raises
    // `can't unset "xs": no such variable`; `unset -nocomplain xs` does not.
    // The provider offers the `-nocomplain` quick-fix for W213.
    let src = "proc foo {} { unset xs }\n";
    let analysis = analyse(src);
    if !analysis
        .diagnostics
        .iter()
        .any(|d| d.code == tcl_compiler::compiler_checks::DiagCode::W213)
    {
        // The RBS pass didn't flag it on this build; nothing to assert.
        return;
    }
    // The W213 diag is on line 0 (narrowed to the `xs` variable word); the
    // whole-line request range still overlaps it and the `unset` keyword.
    let (l, _c) = pos_of(src, "unset", 1);
    let actions = code_actions(src, whole_line(l), Some(&analysis), &analysis.diagnostics);
    let nocomplain = actions
        .iter()
        .find(|a| a.title.contains("-nocomplain"))
        .unwrap_or_else(|| {
            panic!(
                "expected an -nocomplain quick-fix; got {:?}",
                actions.iter().map(|a| &a.title).collect::<Vec<_>>()
            )
        });
    assert_eq!(nocomplain.kind, ActionKind::QuickFix);
    assert_eq!(nocomplain.edits.len(), 1);
    assert_eq!(nocomplain.edits[0].new_text, " -nocomplain");
}

#[test]
fn code_actions_clean_script_outside_diag_offers_no_quickfix() {
    // A clean line with no overlapping diagnostic yields no quick-fix actions
    // (refactor/source actions may still appear, but no `quickfix`).
    let src = "set x 1\nset y 2\n";
    let analysis = analyse(src);
    let actions = code_actions(src, whole_line(0), Some(&analysis), &analysis.diagnostics);
    assert!(
        !actions.iter().any(|a| a.kind == ActionKind::QuickFix),
        "no quick-fix expected on a clean line; got {:?}",
        actions.iter().map(|a| &a.title).collect::<Vec<_>>(),
    );
}

#[test]
fn code_actions_degenerate_inputs_do_not_panic() {
    for src in ["", "   ", "\n", "catch {"] {
        let analysis = analyse(src);
        let _ = code_actions(src, whole_line(0), Some(&analysis), &analysis.diagnostics);
    }
}

// =========================================================================
// declaration — jump to a proc / var declaration
//
// The DECLARATION SITE is a Tcl-semantic fact. `global counter` inside a proc
// body declares the name `counter` in that frame (binding it to the global
// variable); a `$counter` reference resolves there. A bare proc call resolves
// to the proc's definition site. tclsh proof: `info args` confirms a proc has
// a real definition with a real parameter list.
// =========================================================================

#[test]
fn declaration_global_decl_in_proc_body() {
    // tclsh: inside `proc bump {} { global counter; incr counter }`, the
    // `global counter` statement is what binds `counter` to the global var —
    // its declaration site is line 1.
    let src = "proc bump {} {\n\
               global counter\n\
               incr counter\n\
               puts $counter\n\
               }\n";
    let analysis = analyse(src);
    let (l, c) = pos_of(src, "$counter", 1);
    let locs = declaration(src, l, c + 1, tcl_dialect::DialectProfile::by_name("tcl8.6"), &analysis, &registry());
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1);
}

#[test]
fn declaration_variable_decl_in_namespace_proc() {
    // tclsh: `namespace eval ns { variable config 1; proc get {} { variable
    // config; return $config } }` — the in-proc `variable config` re-declares
    // the namespace variable so `$config` reads it.
    let src = "namespace eval ns {\n\
               variable config 1\n\
               proc get {} {\n\
               variable config\n\
               return $config\n\
               }\n\
               }\n";
    let analysis = analyse(src);
    let (l, c) = pos_of(src, "$config", 1);
    let locs = declaration(src, l, c + 1, tcl_dialect::DialectProfile::by_name("tcl8.6"), &analysis, &registry());
    assert!(!locs.is_empty(), "{locs:?}");
}

#[test]
fn declaration_upvar_alias_is_a_declaration() {
    // tclsh: `upvar 1 other local` makes `local` an alias for the caller's
    // `other`; that `upvar` line is `local`'s declaration site.
    let src = "proc wrap {} {\n\
               upvar 1 other local\n\
               return $local\n\
               }\n";
    let analysis = analyse(src);
    let (l, c) = pos_of(src, "$local", 1);
    let locs = declaration(src, l, c + 1, tcl_dialect::DialectProfile::by_name("tcl8.6"), &analysis, &registry());
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1);
}

#[test]
fn declaration_non_variable_falls_back_to_definition() {
    // tclsh: `proc greet {} {}; greet` — `info args greet` → `` (a real proc
    // exists). The cursor on the `greet` call is not a variable, so
    // declaration falls back to go-to-definition: the proc name span on line 0.
    let src = "proc greet {} {}\ngreet\n";
    let analysis = analyse(src);
    let locs = declaration(src, 1, 2, tcl_dialect::DialectProfile::by_name("tcl8.6"), &analysis, &registry());
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 0);
}

#[test]
fn declaration_on_blank_position_does_not_panic() {
    // Cursor on whitespace / past EOL → no declaration, no panic.
    let src = "set x 1\n";
    let _ = declaration(src, 0, 0, tcl_dialect::DialectProfile::by_name("tcl8.6"), &analyse(src), &registry());
    let _ = declaration(src, 5, 99, tcl_dialect::DialectProfile::by_name("tcl8.6"), &analyse(src), &registry());
    let _ = declaration("", 0, 0, tcl_dialect::DialectProfile::by_name("tcl8.6"), &analyse(""), &registry());
}

// =========================================================================
// type_definition — TclOO: jump to the class that types a symbol
//
// tclsh proof: `[Dog new]` constructs a Dog instance; `info object class $d`
// → `::Dog`. So an inferred `$d` of class Dog has its type at the `Dog` class
// definition. A method word inside a class body belongs to that class.
// =========================================================================

#[test]
fn type_definition_var_jumps_to_inferred_class() {
    // tclsh: `oo::class create Dog { method bark {} {} }; set d [Dog new];
    // info object class $d` → `::Dog`. type-definition on `$d` jumps to the
    // Dog class definition (line 0).
    let src = "oo::class create Dog { method bark {} {} }\n\
               set d [Dog new]\n\
               $d bark\n";
    let analysis = analyse(src);
    let (l, c) = pos_of(src, "$d bark", 1);
    let locs = type_definition(src, l, c + 1, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 0);
}

#[test]
fn type_definition_method_word_in_class_body_jumps_to_class() {
    // tclsh: `oo::class create Greeter { method greet {} {}; method again {}
    // { greet } }; info class methods Greeter` → `again greet`. The `greet`
    // call inside `again` names a method of the enclosing class, so
    // type-definition jumps to Greeter (line 0).
    let src = "oo::class create Greeter {\n\
               method greet {} { return hi }\n\
               method again {} { greet }\n\
               }\n";
    let analysis = analyse(src);
    let (l, c) = pos_of(src, "greet", 2); // the call inside `again`
    let locs = type_definition(src, l, c, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 0);
}

#[test]
fn type_definition_var_without_known_class_is_empty() {
    // `$x` is a plain integer, not an object — no class types it.
    let src = "set x 42\nputs $x\n";
    let analysis = analyse(src);
    let (l, c) = pos_of(src, "$x", 1);
    assert!(type_definition(src, l, c + 1, &analysis).is_empty());
}

#[test]
fn type_definition_unrelated_word_is_empty_and_no_panic() {
    let src = "puts hello\n";
    let analysis = analyse(src);
    let (l, c) = pos_of(src, "hello", 1);
    assert!(type_definition(src, l, c, &analysis).is_empty());
    // Degenerate positions.
    assert!(type_definition("", 0, 0, &analyse("")).is_empty());
    assert!(type_definition(src, 9, 9, &analysis).is_empty());
}

// =========================================================================
// implementation — TclOO subclass / method-override fan-out
//
// tclsh proof: `info class subclasses Base` → `::Sub` (who realises Base);
// `info class methods` per class tells which classes define a method. The
// provider answers "who realises this": subclasses for a class name, all
// definers for a free method name, self + descendant overrides inside a class.
// =========================================================================

#[test]
fn implementation_class_name_returns_direct_subclasses() {
    // tclsh: `oo::class create Base {}; oo::class create Derived { superclass
    // Base }; oo::class create Other {}; info class subclasses Base` → `::Derived`.
    // The cursor on `Base` returns exactly its one subclass (line 1), not Other.
    let src = "oo::class create Base {}\n\
               oo::class create Derived {\n\
               superclass Base\n\
               }\n\
               oo::class create Other {}\n";
    let analysis = analyse(src);
    let (l, c) = pos_of(src, "Base", 1); // on `Base` in its own definition
    let locs = implementation(src, l, c, &analysis);
    assert_eq!(locs.len(), 1, "{locs:?}");
    assert_eq!(locs[0].start_line, 1); // Derived's name span
}

#[test]
fn implementation_method_outside_class_returns_all_definers() {
    // tclsh: both A and B define `run` —
    // `oo::class create A { method run {} {} }; info class methods A` → `run`,
    // likewise for B. A `run` call outside any class returns both definers.
    let src = "oo::class create A {\n\
               method run {} {}\n\
               }\n\
               oo::class create B {\n\
               method run {} {}\n\
               }\n\
               set x [a run]\n";
    let analysis = analyse(src);
    let (l, c) = pos_of(src, "run", 3); // the top-level `a run`
    let locs = implementation(src, l, c, &analysis);
    assert_eq!(locs.len(), 2, "{locs:?}");
}

#[test]
fn implementation_method_inside_class_returns_self_and_descendant() {
    // tclsh: `info class subclasses Base` → `::Sub`, and both Base and Sub
    // define `greet` (`info class methods` on each → `greet`). The cursor on
    // `greet` inside Base returns Base's own def plus Sub's override (2).
    let src = "oo::class create Base {\n\
               method greet {} { return hi }\n\
               }\n\
               oo::class create Sub {\n\
               superclass Base\n\
               method greet {} { return yo }\n\
               }\n";
    let analysis = analyse(src);
    let (l, c) = pos_of(src, "greet", 1); // inside Base's method def
    let locs = implementation(src, l, c, &analysis);
    assert_eq!(locs.len(), 2, "{locs:?}");
}

#[test]
fn implementation_unknown_word_is_empty_and_no_panic() {
    let src = "puts hello\n";
    let analysis = analyse(src);
    let (l, c) = pos_of(src, "hello", 1);
    assert!(implementation(src, l, c, &analysis).is_empty());
    assert!(implementation("", 0, 0, &analyse("")).is_empty());
    assert!(implementation(src, 8, 8, &analysis).is_empty());
}

// =========================================================================
// type_hierarchy — resolve the TclOO class at the cursor (prepare)
//
// tclsh proof: `info class superclass` / `info class subclasses` confirm the
// class is a real TclOO type. The `prepare` step resolves the class whose
// name is under the cursor; super/subtype walks are stubbed in this provider,
// so this covers the prepare surface (the part that is implemented).
// =========================================================================

#[test]
fn type_hierarchy_resolves_class_at_cursor() {
    // tclsh: `oo::class create Greeter {}; info class superclass Greeter`
    // → `::oo::object` (Greeter is a real class). prepare on `Greeter`
    // returns one item naming it.
    let src = "oo::class create Greeter {}\n";
    let analysis = analyse(src);
    let (l, c) = pos_of(src, "Greeter", 1);
    let items = type_hierarchy_prepare(src, l, c, &analysis);
    assert_eq!(items.len(), 1, "{items:?}");
    assert!(items[0].name.contains("Greeter"));
    // The full range must contain the name selection range (structural).
    let it = &items[0];
    assert!(
        (it.range.start_line, it.range.start_character)
            <= (
                it.selection_range.start_line,
                it.selection_range.start_character
            )
    );
}

#[test]
fn type_hierarchy_class_with_superclass_resolves() {
    // tclsh: `oo::class create Base {}; oo::class create Sub { superclass
    // Base }; info class superclass Sub` → `::Base`. prepare on `Sub` resolves
    // the Sub class.
    let src = "oo::class create Base {}\noo::class create Sub { superclass Base }\n";
    let analysis = analyse(src);
    let (l, c) = pos_of(src, "Sub", 1);
    let items = type_hierarchy_prepare(src, l, c, &analysis);
    assert_eq!(items.len(), 1, "{items:?}");
    assert!(items[0].name.contains("Sub"));
}

#[test]
fn type_hierarchy_non_class_word_is_empty() {
    // A plain command word names no class.
    let src = "puts hello\n";
    let analysis = analyse(src);
    let (l, c) = pos_of(src, "hello", 1);
    assert!(type_hierarchy_prepare(src, l, c, &analysis).is_empty());
}

#[test]
fn type_hierarchy_degenerate_inputs_do_not_panic() {
    assert!(type_hierarchy_prepare("", 0, 0, &analyse("")).is_empty());
    let src = "oo::class create C {}\n";
    let analysis = analyse(src);
    let _ = type_hierarchy_prepare(src, 9, 9, &analysis);
}

// =========================================================================
// linked_editing_range — link a proc declaration to its recursive self-calls
//
// The LINKED set is a Tcl-semantic fact: a recursive proc calls *itself*, so
// renaming the proc must rename the self-call too. tclsh proof: a real proc
// with a self-call exists (`info body factorial` contains `factorial`). The
// WORD_PATTERN regex is editor-presentation, asserted structurally.
// =========================================================================

#[test]
fn linked_editing_links_recursive_self_call() {
    // tclsh: `proc factorial {n} { return [factorial 1] }; info body factorial`
    // contains the recursive `factorial` call — declaration + self-call edit
    // together.
    let src = "proc factorial {n} {\n    return [factorial 1]\n}\n";
    let analysis = analyse(src);
    let result = linked_editing_ranges(src, 0, 6, &analysis)
        .expect("recursive self-call should link to declaration");
    assert!(result.ranges.len() >= 2, "{result:?}");
    // Editor-presentation: the validating word pattern.
    assert_eq!(result.word_pattern, WORD_PATTERN);
    // Both linked ranges name `factorial` — the declaration (line 0) and the
    // self-call (line 1).
    assert!(result.ranges.iter().any(|r| r.start_line == 0));
    assert!(result.ranges.iter().any(|r| r.start_line == 1));
}

#[test]
fn linked_editing_cursor_inside_body_also_links() {
    let src = "proc factorial {n} {\n    return [factorial 1]\n}\n";
    let analysis = analyse(src);
    // Cursor on the self-call inside the body.
    let (l, c) = pos_of(src, "factorial", 2);
    let result = linked_editing_ranges(src, l, c + 1, &analysis)
        .expect("cursor inside body on the recursive call should link");
    assert!(result.ranges.len() >= 2, "{result:?}");
}

#[test]
fn linked_editing_non_recursive_proc_is_none() {
    // tclsh: `info body greet` has no self-call — a single range can't be
    // linked-edited, so the provider returns None.
    let src = "proc greet {name} { return $name }\n";
    let analysis = analyse(src);
    assert!(linked_editing_ranges(src, 0, 6, &analysis).is_none());
}

#[test]
fn linked_editing_on_builtin_is_none() {
    // `puts` is a built-in, not a user proc.
    let src = "set x 1\nputs $x\n";
    let analysis = analyse(src);
    assert!(linked_editing_ranges(src, 1, 1, &analysis).is_none());
}

#[test]
fn linked_editing_degenerate_inputs_do_not_panic() {
    assert!(linked_editing_ranges("", 0, 0, &analyse("")).is_none());
    let src = "proc factorial {n} { return [factorial 1] }\n";
    let analysis = analyse(src);
    let _ = linked_editing_ranges(src, 9, 99, &analysis);
}
