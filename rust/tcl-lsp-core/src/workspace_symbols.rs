//! Workspace-symbols provider — minimal Rust port of
//! `lsp/features/workspace_symbols.py`.
//!
//! Lists every proc, class, method, `classmethod`, and
//! constructor recorded in the analyser's
//! `AnalysisResult` for the **current document**, filtered by
//! a query string.  The Python implementation walks every
//! document in the workspace index; the workspace-index port
//! is a separate chunk under `S-workspace-init`, so this
//! provider operates on a single document until that lands
//! (deferred to `S-workspace-symbols-rich`).

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
                range: span_to_range(&line_index, proc_def.name_span),
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
                range: span_to_range(&line_index, class_def.name_span),
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
                    range: span_to_range(&line_index, method.name_span),
                });
            }
        }
        for method in class_def.class_methods.values() {
            if matches_query(&method.name, &lower_query) {
                out.push(WorkspaceSymbol {
                    name: method.name.clone(),
                    container_name: container.clone(),
                    kind: WorkspaceSymbolKind::Method,
                    range: span_to_range(&line_index, method.name_span),
                });
            }
        }
        for ctor in &class_def.constructors {
            if matches_query("constructor", &lower_query) {
                out.push(WorkspaceSymbol {
                    name: "constructor".to_string(),
                    container_name: container.clone(),
                    kind: WorkspaceSymbolKind::Constructor,
                    range: span_to_range(&line_index, ctor.name_span),
                });
            }
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

fn span_to_range(line_index: &LineIndex, span: tcl_lexer::Span) -> LspRange {
    let start = line_index.position_at(span.start());
    let end = line_index.position_at(span.end());
    LspRange {
        start_line: start.line,
        start_character: start.character,
        end_line: end.line,
        end_character: end.character,
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

    // -- S-workspace-symbols-rich: class methods ---------------------

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
}
