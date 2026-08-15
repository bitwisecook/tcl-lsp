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

//! Workspace-symbols provider.
//!
//! Answers `workspace/symbol` from the [`crate::workspace_index::WorkspaceIndex`]:
//! every proc, class, method, `classmethod`, constructor, and registry
//! symbol-definer definition (a `tcltest::test` case, a `testConstraint`, a
//! `customMatch` mode, an iRules `when EVENT` handler) the **whole workspace**
//! holds, filtered by a query string.
//!
//! This module owns the wire contract — what counts as a workspace symbol,
//! what kind it reports, and how a query matches.  The scan itself is
//! [`crate::workspace_index::WorkspaceIndex::symbols_matching`], which walks
//! the index document by document so that a capped answer is a slice of the
//! workspace rather than a slice of one symbol table.

use tcl_lexer::Span;

/// The most symbols one `workspace/symbol` answer carries.
///
/// VS Code re-issues the request on every keystroke in its Ctrl+T box, and a
/// short query matches most of a large workspace — on the order of 100 000
/// symbols on tcllib, none of which a picker can usefully show.  The cap is
/// applied *while scanning the index*, so it bounds the work and not merely
/// the payload.
pub const MAX_WORKSPACE_SYMBOL_RESULTS: usize = 1000;

/// Workspace-symbol kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WorkspaceSymbolKind {
    /// User-defined `proc`.
    Function,
    /// `TclOO` class.
    Class,
    /// `TclOO` instance / class method.
    Method,
    /// `TclOO` constructor — a member method whose role is
    /// instance construction.
    Constructor,
    /// A named definition from a registry symbol-definer command — a
    /// `tcltest::test` case (issue #790).
    Test,
    /// A named `tcltest::testConstraint` — a boolean test condition.
    Constant,
    /// A named `tcltest::customMatch` mode — a custom result matcher.
    Operator,
    /// An iRules `when EVENT { … }` event handler.
    Event,
}

impl From<tcl_registry::DefinedSymbolKind> for WorkspaceSymbolKind {
    /// Map a registry outline category to its workspace-symbol kind.
    fn from(kind: tcl_registry::DefinedSymbolKind) -> Self {
        match kind {
            tcl_registry::DefinedSymbolKind::Test => Self::Test,
            tcl_registry::DefinedSymbolKind::Constraint => Self::Constant,
            tcl_registry::DefinedSymbolKind::Matcher => Self::Operator,
            tcl_registry::DefinedSymbolKind::Event => Self::Event,
        }
    }
}

/// One entry in a `workspace/symbol` response.
///
/// The location is a byte [`Span`] in `uri`'s source, not an LSP range: a
/// range needs the target document's text, which the index does not hold, so
/// the server resolves it at delivery time — the rule every other
/// cross-document index answer follows.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IndexedWorkspaceSymbol {
    /// Document the definition is in.
    pub uri: String,
    /// Display name (unqualified).
    pub name: String,
    /// Container name — the enclosing namespace, or the owning class for a
    /// method or constructor — or `None`.
    pub container_name: Option<String>,
    /// Symbol kind.
    pub kind: WorkspaceSymbolKind,
    /// Byte span of the definition's name token in `uri`'s source.
    pub name_span: Span,
}

/// Whether `name` satisfies `lower_query`: a case-insensitive substring
/// match, with an empty query matching everything.
pub(crate) fn matches_query(name: &str, lower_query: &str) -> bool {
    if lower_query.is_empty() {
        return true;
    }
    name.to_lowercase().contains(lower_query)
}

/// The enclosing namespace of a qualified name, or `None` when it names a
/// global.
pub(crate) fn namespace_of(qname: &str) -> Option<String> {
    let (holder, _) = tcl_syntax::naming::key_holder_and_tail(qname);
    (!holder.is_empty() && holder != "::").then(|| holder.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workspace_index::WorkspaceIndex;
    use tcl_compiler::analyser::{Analyser, AnalysisResult};

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    /// One document indexed under `file:///a.tcl`, queried for `query`.
    fn symbols(source: &str, query: &str) -> Vec<IndexedWorkspaceSymbol> {
        let analysis = analyse(source);
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &analysis);
        index.symbols_matching(query, MAX_WORKSPACE_SYMBOL_RESULTS)
    }

    #[test]
    fn empty_query_returns_all_symbols() {
        let syms = symbols("proc alpha {} {}\nproc beta {} {}\n", "");
        assert_eq!(syms.len(), 2);
    }

    #[test]
    fn query_filters_by_substring_case_insensitive() {
        let syms = symbols("proc alpha {} {}\nproc beta {} {}\n", "Alp");
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].name, "alpha");
    }

    #[test]
    fn namespace_container_uses_constructed_name_codec() {
        assert_eq!(namespace_of("::foo"), None);
        assert_eq!(namespace_of("::a::b"), Some("::a".to_string()));
        assert_eq!(namespace_of("::::::"), Some(":::".to_string()));
        assert_eq!(namespace_of("::foo::"), Some("::foo".to_string()));
    }

    #[test]
    fn a_qualified_name_matches_the_query_too() {
        let syms = symbols("namespace eval util { proc helper {} {} }\n", "util::help");
        assert_eq!(syms.len(), 1, "{syms:?}");
        assert_eq!(syms[0].name, "helper");
        assert_eq!(syms[0].container_name.as_deref(), Some("::util"));
    }

    #[test]
    fn qualified_colon_run_names_keep_their_workspace_container() {
        let syms = symbols(
            "namespace eval a:::b {}\nproc a::b::q {} {}\nproc : {} {}\nproc foo::: {} {}\n",
            "q",
        );
        assert_eq!(syms.len(), 1, "{syms:?}");
        assert_eq!(syms[0].name, "q");
        assert_eq!(syms[0].container_name.as_deref(), Some("::a::b"));
    }

    // class methods

    #[test]
    fn class_methods_surface_as_workspace_symbols() {
        let syms = symbols(
            "oo::class create MyClass {\n\
                 method greet {} {}\n\
                 method farewell {} {}\n\
             }\n",
            "",
        );
        let by_name: std::collections::HashMap<&str, &IndexedWorkspaceSymbol> =
            syms.iter().map(|s| (s.name.as_str(), s)).collect();
        assert!(by_name.contains_key("MyClass"), "{syms:?}");
        assert!(by_name.contains_key("greet"), "{syms:?}");
        assert!(by_name.contains_key("farewell"), "{syms:?}");
        assert_eq!(by_name["MyClass"].kind, WorkspaceSymbolKind::Class);
        assert_eq!(by_name["greet"].kind, WorkspaceSymbolKind::Method);
        assert_eq!(
            by_name["greet"].container_name.as_deref(),
            Some("::MyClass")
        );
    }

    #[test]
    fn classmethod_surfaces_with_method_kind() {
        let syms = symbols(
            "oo::class create MyClass {\n\
                 classmethod factory {} {}\n\
             }\n",
            "factory",
        );
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].name, "factory");
        assert_eq!(syms[0].kind, WorkspaceSymbolKind::Method);
    }

    #[test]
    fn constructor_surfaces_with_constructor_kind() {
        let syms = symbols(
            "oo::class create MyClass {\n\
                 constructor {arg} {}\n\
             }\n",
            "constructor",
        );
        let ctors: Vec<&IndexedWorkspaceSymbol> = syms
            .iter()
            .filter(|s| s.kind == WorkspaceSymbolKind::Constructor)
            .collect();
        assert_eq!(ctors.len(), 1, "{syms:?}");
        assert_eq!(ctors[0].name, "constructor");
        assert_eq!(ctors[0].container_name.as_deref(), Some("::MyClass"));
    }

    #[test]
    fn query_matches_method_substring() {
        let syms = symbols(
            "oo::class create MyClass {\n\
                 method greetUser {} {}\n\
                 method farewell {} {}\n\
             }\n",
            "greet",
        );
        let labels: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(labels.contains(&"greetUser"), "{syms:?}");
        assert!(!labels.contains(&"farewell"), "{syms:?}");
    }

    // issue #790: tcltest `test` cases surface as workspace symbols.

    #[test]
    fn tcltest_test_surfaces_as_workspace_symbol() {
        let syms = symbols(
            "package require tcltest\n\
             namespace import ::tcltest::*\n\
             test find-me-1 {desc} -body { set x 1 } -result 1\n",
            "find-me",
        );
        let hit = syms
            .iter()
            .find(|s| s.name == "find-me-1")
            .unwrap_or_else(|| panic!("test not found in {syms:?}"));
        assert_eq!(hit.kind, WorkspaceSymbolKind::Test);
    }

    #[test]
    fn constraint_and_matcher_surface_with_own_kinds() {
        let syms = symbols(
            "package require tcltest\n\
             namespace import ::tcltest::*\n\
             testConstraint slowNet 1\n\
             customMatch approxEq ::approx\n",
            "",
        );
        let by_name: std::collections::HashMap<&str, &IndexedWorkspaceSymbol> =
            syms.iter().map(|s| (s.name.as_str(), s)).collect();
        assert_eq!(
            by_name.get("slowNet").map(|s| s.kind),
            Some(WorkspaceSymbolKind::Constant),
            "{syms:?}"
        );
        assert_eq!(
            by_name.get("approxEq").map(|s| s.kind),
            Some(WorkspaceSymbolKind::Operator),
            "{syms:?}"
        );
    }

    #[test]
    fn test_case_in_namespace_has_container() {
        let syms = symbols(
            "package require tcltest\n\
             namespace eval suite {\n\
                 tcltest::test scoped-1 {desc} -body { set x 1 } -result 1\n\
             }\n",
            "scoped",
        );
        let hit = syms
            .iter()
            .find(|s| s.name == "scoped-1")
            .unwrap_or_else(|| panic!("scoped test not found in {syms:?}"));
        assert_eq!(hit.kind, WorkspaceSymbolKind::Test);
        assert_eq!(hit.container_name.as_deref(), Some("::suite"));
    }

    /// Issue #1156: a document the editor never opened — only scanned into the
    /// index — is searchable.  The previous handler walked the *open-document*
    /// map, so every symbol in an unopened file was invisible to Ctrl+T.
    #[test]
    fn symbols_come_from_every_indexed_document_not_only_open_ones() {
        let a = analyse("proc opened_one {} {}\n");
        let b = analyse("proc never_opened {} {}\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///open.tcl", &a);
        index.add_document("file:///scanned.tcl", &b);
        let syms = index.symbols_matching("never_opened", MAX_WORKSPACE_SYMBOL_RESULTS);
        assert_eq!(syms.len(), 1, "{syms:?}");
        assert_eq!(syms[0].uri, "file:///scanned.tcl");
    }

    /// Issue #1156: the cap bounds the answer, and it bounds it *by document*
    /// — a truncated result is a prefix of the workspace, not a prefix of one
    /// symbol table.
    #[test]
    fn the_result_cap_holds_and_truncates_by_document() {
        let mut index = WorkspaceIndex::new();
        for doc in 0..10 {
            let src = (0..10).fold(String::new(), |mut src, n| {
                use std::fmt::Write;
                let _ = writeln!(src, "proc p{doc}_{n} {{}} {{}}");
                src
            });
            let analysis = analyse(&src);
            index.add_document(&format!("file:///d{doc}.tcl"), &analysis);
        }
        assert_eq!(index.symbols_matching("", 100).len(), 100);
        let capped = index.symbols_matching("", 25);
        assert_eq!(capped.len(), 25);
        let uris: std::collections::BTreeSet<&str> =
            capped.iter().map(|s| s.uri.as_str()).collect();
        assert_eq!(
            uris,
            ["file:///d0.tcl", "file:///d1.tcl", "file:///d2.tcl"]
                .into_iter()
                .collect(),
        );
    }
}
