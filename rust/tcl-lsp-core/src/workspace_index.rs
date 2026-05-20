//! Workspace index — a cross-document symbol aggregate.
//!
//! The per-document providers (definition, references, rename,
//! completion, code-lens) answer queries against a single
//! document's [`AnalysisResult`].  The workspace index lifts
//! the proc / class *definitions* of every analysed document
//! into one searchable structure so cross-document features
//! can resolve a symbol that lives in a sibling file.
//!
//! The server owns one index, rebuilt (or incrementally
//! updated) as documents open / change / close from its cached
//! `AnalysisResult` map.  The index stores owned data (so it
//! can move into a `spawn_blocking` worker) and keeps the byte
//! [`Span`] of each definition; converting a span to an LSP
//! range needs the *target* document's source, which the
//! server resolves at query time.
//!
//! This is the foundation for:
//!
//! * workspace-wide proc enumeration in completion;
//! * cross-document go-to-definition;
//! * cross-document references / rename / call-hierarchy
//!   (later strips).
//!
//! What is *deferred*:
//!
//! * On-disk scanning of workspace-folder files that haven't
//!   been opened in the editor — the index only covers
//!   documents the server has analysed.  A folder-walk that
//!   analyses unopened `.tcl` files lands with the
//!   workspace-folder enumeration follow-up.
//! * Variable / namespace indexing — only procs and classes
//!   are indexed today (the cross-document features that need
//!   them).

use tcl_compiler::analyser::AnalysisResult;
use tcl_lexer::Span;

/// One proc definition recorded in the workspace index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceProc {
    /// Document the proc is defined in (the `analyses` map key).
    pub uri: String,
    /// Simple (tail) name, e.g. `greet`.
    pub name: String,
    /// Fully-qualified name, e.g. `::myns::greet`.
    pub qualified_name: String,
    /// Declared parameter count (for completion detail).
    pub param_count: usize,
    /// Byte span of the proc's name token in `uri`'s source.
    /// The server resolves this to an LSP range against the
    /// target document at query time.
    pub name_span: Span,
}

/// One class definition recorded in the workspace index.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorkspaceClass {
    /// Document the class is defined in.
    pub uri: String,
    /// Simple (tail) name.
    pub name: String,
    /// Fully-qualified name.
    pub qualified_name: String,
    /// Byte span of the class's name token.
    pub name_span: Span,
}

/// Cross-document aggregate of proc / class definitions.
#[derive(Debug, Clone, Default)]
pub struct WorkspaceIndex {
    procs: Vec<WorkspaceProc>,
    classes: Vec<WorkspaceClass>,
}

impl WorkspaceIndex {
    /// Empty index.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Build an index from an iterator of `(uri, analysis)`
    /// pairs — typically the server's cached-analysis map.
    #[must_use]
    pub fn from_documents<'a, I>(documents: I) -> Self
    where
        I: IntoIterator<Item = (&'a str, &'a AnalysisResult)>,
    {
        let mut index = Self::new();
        for (uri, analysis) in documents {
            index.add_document(uri, analysis);
        }
        index
    }

    /// Add (or refresh) one document's proc / class definitions.
    ///
    /// Call [`Self::remove_document`] first when re-indexing a
    /// changed document to avoid stale duplicates.
    pub fn add_document(&mut self, uri: &str, analysis: &AnalysisResult) {
        for proc_def in analysis.all_procs.values() {
            self.procs.push(WorkspaceProc {
                uri: uri.to_owned(),
                name: proc_def.name.clone(),
                qualified_name: proc_def.qualified_name.clone(),
                param_count: proc_def.params.len(),
                name_span: proc_def.name_span,
            });
        }
        for class_def in analysis.all_classes.values() {
            self.classes.push(WorkspaceClass {
                uri: uri.to_owned(),
                name: class_def.name.clone(),
                qualified_name: class_def.qualified_name.clone(),
                name_span: class_def.name_span,
            });
        }
    }

    /// Drop every entry that came from `uri` (used before
    /// re-indexing a changed document, or on `did_close`).
    pub fn remove_document(&mut self, uri: &str) {
        self.procs.retain(|p| p.uri != uri);
        self.classes.retain(|c| c.uri != uri);
    }

    /// Every indexed proc.
    #[must_use]
    pub fn procs(&self) -> &[WorkspaceProc] {
        &self.procs
    }

    /// Every indexed class.
    #[must_use]
    pub fn classes(&self) -> &[WorkspaceClass] {
        &self.classes
    }

    /// Procs whose simple *or* qualified name starts with
    /// `prefix`, excluding any defined in `exclude_uri` (the
    /// caller's current document, whose procs the single-doc
    /// provider already surfaces).  Empty `prefix` matches all.
    #[must_use]
    pub fn procs_matching<'a>(&'a self, prefix: &str, exclude_uri: &str) -> Vec<&'a WorkspaceProc> {
        self.procs
            .iter()
            .filter(|p| p.uri != exclude_uri)
            .filter(|p| {
                prefix.is_empty()
                    || p.name.starts_with(prefix)
                    || p.qualified_name.starts_with(prefix)
            })
            .collect()
    }

    /// Proc definitions matching `name` (simple, qualified, or
    /// `::`-prefixed simple form), excluding `exclude_uri`.
    /// Used by cross-document go-to-definition: when the
    /// current document has no matching proc, the index
    /// resolves one defined elsewhere.
    #[must_use]
    pub fn proc_definitions<'a>(&'a self, name: &str, exclude_uri: &str) -> Vec<&'a WorkspaceProc> {
        let qualified = format!("::{name}");
        self.procs
            .iter()
            .filter(|p| p.uri != exclude_uri)
            .filter(|p| p.name == name || p.qualified_name == name || p.qualified_name == qualified)
            .collect()
    }

    /// Class definitions matching `name`, excluding
    /// `exclude_uri`.
    #[must_use]
    pub fn class_definitions<'a>(
        &'a self,
        name: &str,
        exclude_uri: &str,
    ) -> Vec<&'a WorkspaceClass> {
        let qualified = format!("::{name}");
        self.classes
            .iter()
            .filter(|c| c.uri != exclude_uri)
            .filter(|c| c.name == name || c.qualified_name == name || c.qualified_name == qualified)
            .collect()
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
    fn indexes_procs_from_multiple_documents() {
        let a = analyse("proc greet {name} {}\n");
        let b = analyse("proc farewell {} {}\nproc greet2 {x y} {}\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        assert_eq!(index.procs().len(), 3);
        // Param counts captured.
        let greet = index.procs().iter().find(|p| p.name == "greet").unwrap();
        assert_eq!(greet.param_count, 1);
        assert_eq!(greet.uri, "file:///a.tcl");
    }

    #[test]
    fn procs_matching_excludes_current_doc_and_filters_prefix() {
        let a = analyse("proc alpha {} {}\n");
        let b = analyse("proc alphabet {} {}\nproc beta {} {}\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        // From a.tcl's perspective, only b.tcl procs with `alph`.
        let got = index.procs_matching("alph", "file:///a.tcl");
        let names: Vec<&str> = got.iter().map(|p| p.name.as_str()).collect();
        assert_eq!(names, vec!["alphabet"]);
    }

    #[test]
    fn proc_definitions_resolves_cross_document() {
        let a = analyse("proc helper {} {}\n");
        let b = analyse("helper\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a), ("file:///b.tcl", &b)]);
        // From b.tcl, `helper` resolves to a.tcl's definition.
        let defs = index.proc_definitions("helper", "file:///b.tcl");
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].uri, "file:///a.tcl");
        // The same-document exclusion drops it when querying
        // from a.tcl itself.
        assert!(index.proc_definitions("helper", "file:///a.tcl").is_empty());
    }

    #[test]
    fn remove_document_drops_its_entries() {
        let a = analyse("proc a {} {}\n");
        let b = analyse("proc b {} {}\n");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///a.tcl", &a);
        index.add_document("file:///b.tcl", &b);
        assert_eq!(index.procs().len(), 2);
        index.remove_document("file:///a.tcl");
        assert_eq!(index.procs().len(), 1);
        assert_eq!(index.procs()[0].name, "b");
    }

    #[test]
    fn indexes_classes() {
        let a = analyse("oo::class create Widget {}\n");
        let index = WorkspaceIndex::from_documents([("file:///a.tcl", &a)]);
        let defs = index.class_definitions("Widget", "file:///other.tcl");
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0].qualified_name, "::Widget");
    }
}
