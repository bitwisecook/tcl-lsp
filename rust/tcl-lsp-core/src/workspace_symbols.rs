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
//! Lists every proc, class, method, `classmethod`, and
//! constructor recorded in the analyser's
//! `AnalysisResult` for the **current document**, filtered by
//! a query string.  This provider operates on a single
//! document; it does not yet walk every document in the
//! workspace index.

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::LineIndex;

use crate::definition::LspRange;

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

/// One entry in a workspace-symbols response.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceSymbol {
    /// Display name (unqualified).
    pub name: String,
    /// Container name (e.g. namespace), or `None`.
    pub container_name: Option<String>,
    /// Symbol kind.
    pub kind: WorkspaceSymbolKind,
    /// Definition span.
    pub range: LspRange,
}

/// Compute workspace symbols for the current document
/// matching `query`.
#[must_use]
pub fn workspace_symbols(
    source: &str,
    query: &str,
    analysis: &AnalysisResult,
) -> Vec<WorkspaceSymbol> {
    let line_index = LineIndex::new(source);
    let lower_query = query.to_lowercase();
    let mut out = Vec::new();
    for (qname, proc_def) in &analysis.all_procs {
        if matches_query(&proc_def.name, &lower_query) || matches_query(qname, &lower_query) {
            out.push(WorkspaceSymbol {
                name: proc_def.name.clone(),
                container_name: namespace_of(qname),
                kind: WorkspaceSymbolKind::Function,
                range: span_to_range(source, &line_index, proc_def.name_span),
            });
        }
    }
    for class_def in analysis.all_classes.values() {
        if matches_query(&class_def.name, &lower_query)
            || matches_query(&class_def.qualified_name, &lower_query)
        {
            out.push(WorkspaceSymbol {
                name: class_def.name.clone(),
                container_name: namespace_of(&class_def.qualified_name),
                kind: WorkspaceSymbolKind::Class,
                range: span_to_range(source, &line_index, class_def.name_span),
            });
        }
        // Instance + class methods + constructors — surface each
        // member with the class's qualified name as the
        // container, so editors render them as
        // `ClassName::methodName` and can navigate to them via
        // the symbol-list jump.
        let container = Some(class_def.qualified_name.clone());
        for method in class_def.methods.values() {
            if matches_query(&method.name, &lower_query) {
                out.push(WorkspaceSymbol {
                    name: method.name.clone(),
                    container_name: container.clone(),
                    kind: WorkspaceSymbolKind::Method,
                    range: span_to_range(source, &line_index, method.name_span),
                });
            }
        }
        for method in class_def.class_methods.values() {
            if matches_query(&method.name, &lower_query) {
                out.push(WorkspaceSymbol {
                    name: method.name.clone(),
                    container_name: container.clone(),
                    kind: WorkspaceSymbolKind::Method,
                    range: span_to_range(source, &line_index, method.name_span),
                });
            }
        }
        for ctor in &class_def.constructors {
            if matches_query("constructor", &lower_query) {
                out.push(WorkspaceSymbol {
                    name: "constructor".to_string(),
                    container_name: container.clone(),
                    kind: WorkspaceSymbolKind::Constructor,
                    range: span_to_range(source, &line_index, ctor.name_span),
                });
            }
        }
    }
    // Registry symbol-definer definitions (tcltest tests, …) — same shape as
    // procs, keyed on the resolved name and its enclosing namespace container.
    for sym in &analysis.all_defined_symbols {
        if matches_query(&sym.name, &lower_query)
            || matches_query(&sym.qualified_name, &lower_query)
        {
            out.push(WorkspaceSymbol {
                name: sym.name.clone(),
                container_name: namespace_of(&sym.qualified_name),
                kind: WorkspaceSymbolKind::from(sym.kind),
                range: span_to_range(source, &line_index, sym.name_span),
            });
        }
    }
    out
}

fn matches_query(name: &str, lower_query: &str) -> bool {
    if lower_query.is_empty() {
        return true;
    }
    name.to_lowercase().contains(lower_query)
}

fn namespace_of(qname: &str) -> Option<String> {
    let stripped = qname.strip_prefix("::").unwrap_or(qname);
    let last_sep = stripped.rfind("::")?;
    Some(format!("::{}", &stripped[..last_sep]))
}

fn span_to_range(source: &str, line_index: &LineIndex, span: tcl_lexer::Span) -> LspRange {
    let start = line_index.position_at_utf16(span.start(), source);
    let end = line_index.position_at_utf16(span.end(), source);
    LspRange {
        start_line: start.line,
        start_character: start.character.get(),
        end_line: end.line,
        end_character: end.character.get(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    fn analyse(source: &str) -> AnalysisResult {
        let mut a = Analyser::new();
        a.analyse(source, "tcl8.6").clone()
    }

    #[test]
    fn empty_query_returns_all_symbols() {
        let src = "proc alpha {} {}\nproc beta {} {}\n";
        let analysis = analyse(src);
        let syms = workspace_symbols(src, "", &analysis);
        assert_eq!(syms.len(), 2);
    }

    #[test]
    fn query_filters_by_substring_case_insensitive() {
        let src = "proc alpha {} {}\nproc beta {} {}\n";
        let analysis = analyse(src);
        let syms = workspace_symbols(src, "Alp", &analysis);
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].name, "alpha");
    }

    // class methods

    #[test]
    fn class_methods_surface_as_workspace_symbols() {
        let src = "oo::class create MyClass {\n\
                       method greet {} {}\n\
                       method farewell {} {}\n\
                   }\n";
        let analysis = analyse(src);
        let syms = workspace_symbols(src, "", &analysis);
        // Should include the class, both methods.
        let by_name: std::collections::HashMap<&str, &WorkspaceSymbol> =
            syms.iter().map(|s| (s.name.as_str(), s)).collect();
        assert!(by_name.contains_key("MyClass"), "{syms:?}");
        assert!(by_name.contains_key("greet"), "{syms:?}");
        assert!(by_name.contains_key("farewell"), "{syms:?}");
        // Class kind.
        assert_eq!(by_name["MyClass"].kind, WorkspaceSymbolKind::Class);
        // Methods are tagged Method and have the class's qualified
        // name as container.
        assert_eq!(by_name["greet"].kind, WorkspaceSymbolKind::Method);
        assert_eq!(
            by_name["greet"].container_name.as_deref(),
            Some("::MyClass"),
        );
    }

    #[test]
    fn classmethod_surfaces_with_method_kind() {
        let src = "oo::class create MyClass {\n\
                       classmethod factory {} {}\n\
                   }\n";
        let analysis = analyse(src);
        let syms = workspace_symbols(src, "factory", &analysis);
        assert_eq!(syms.len(), 1);
        assert_eq!(syms[0].name, "factory");
        assert_eq!(syms[0].kind, WorkspaceSymbolKind::Method);
    }

    #[test]
    fn constructor_surfaces_with_constructor_kind() {
        let src = "oo::class create MyClass {\n\
                       constructor {arg} {}\n\
                   }\n";
        let analysis = analyse(src);
        let syms = workspace_symbols(src, "constructor", &analysis);
        // At least one constructor entry, with the right kind.
        let ctors: Vec<&WorkspaceSymbol> = syms
            .iter()
            .filter(|s| s.kind == WorkspaceSymbolKind::Constructor)
            .collect();
        assert_eq!(ctors.len(), 1, "{syms:?}");
        assert_eq!(ctors[0].name, "constructor");
        assert_eq!(ctors[0].container_name.as_deref(), Some("::MyClass"));
    }

    #[test]
    fn query_matches_method_substring() {
        let src = "oo::class create MyClass {\n\
                       method greetUser {} {}\n\
                       method farewell {} {}\n\
                   }\n";
        let analysis = analyse(src);
        let syms = workspace_symbols(src, "greet", &analysis);
        let labels: Vec<&str> = syms.iter().map(|s| s.name.as_str()).collect();
        assert!(labels.contains(&"greetUser"), "{syms:?}");
        assert!(!labels.contains(&"farewell"), "{syms:?}");
    }

    // issue #790: tcltest `test` cases surface as workspace symbols.

    #[test]
    fn tcltest_test_surfaces_as_workspace_symbol() {
        let src = "package require tcltest\n\
                   namespace import ::tcltest::*\n\
                   test find-me-1 {desc} -body { set x 1 } -result 1\n";
        let analysis = analyse(src);
        let syms = workspace_symbols(src, "find-me", &analysis);
        let hit = syms
            .iter()
            .find(|s| s.name == "find-me-1")
            .unwrap_or_else(|| panic!("test not found in {syms:?}"));
        assert_eq!(hit.kind, WorkspaceSymbolKind::Test);
    }

    #[test]
    fn constraint_and_matcher_surface_with_own_kinds() {
        let src = "package require tcltest\n\
                   namespace import ::tcltest::*\n\
                   testConstraint slowNet 1\n\
                   customMatch approxEq ::approx\n";
        let analysis = analyse(src);
        let syms = workspace_symbols(src, "", &analysis);
        let by_name: std::collections::HashMap<&str, &WorkspaceSymbol> =
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
        let src = "package require tcltest\n\
                   namespace eval suite {\n\
                       tcltest::test scoped-1 {desc} -body { set x 1 } -result 1\n\
                   }\n";
        let analysis = analyse(src);
        let syms = workspace_symbols(src, "scoped", &analysis);
        let hit = syms
            .iter()
            .find(|s| s.name == "scoped-1")
            .unwrap_or_else(|| panic!("scoped test not found in {syms:?}"));
        assert_eq!(hit.kind, WorkspaceSymbolKind::Test);
        assert_eq!(hit.container_name.as_deref(), Some("::suite"));
    }
}
