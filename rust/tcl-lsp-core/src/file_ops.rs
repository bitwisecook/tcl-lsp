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

//! Intentional project source-reference rewrites for host file renames.

use std::path::{Path, PathBuf};

use tcl_compiler::analyser::AnalysisResult;
use tcl_compiler::auto_path_eval::OriginalSourcePathExpression;
use tcl_lexer::Span;

use crate::source_graph::{ProjectSourceAssociation, project_source_association};
use crate::workspace_index::WorkspaceIndex;

/// Actual host paths and the declared project association policy for a user
/// file rename. This authorizes an intentional source rewrite; it establishes
/// neither Tcl's runtime working directory nor an observed file load.
#[derive(Debug)]
pub struct FileRenameSourceAction<'a> {
    old_path: &'a Path,
    new_path: &'a Path,
    workspace_roots: &'a [PathBuf],
}

impl<'a> FileRenameSourceAction<'a> {
    #[must_use]
    pub fn new(
        old_path: &'a Path,
        new_path: &'a Path,
        workspace_roots: &'a [PathBuf],
    ) -> Option<Self> {
        let admissible = |path: &Path| {
            path.is_absolute() && path.to_str().is_some_and(|text| !text.contains('\0'))
        };
        (admissible(old_path)
            && admissible(new_path)
            && workspace_roots.iter().all(|root| admissible(root)))
        .then_some(Self {
            old_path,
            new_path,
            workspace_roots,
        })
    }
}

/// A proposed edit retaining its original source/index generation. The host
/// must join it to current analysis and document versions before publication.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenameEdit {
    pub uri: String,
    pub span: Span,
    pub new_text: String,
    revision: u64,
    original: OriginalSourcePathExpression,
    literal: String,
    document_path: PathBuf,
    old_path: PathBuf,
    new_path: PathBuf,
    workspace_roots: Vec<PathBuf>,
    association: ProjectSourceAssociation,
}

impl RenameEdit {
    #[must_use]
    pub fn matches_current(
        &self,
        index: &WorkspaceIndex,
        analysis: &AnalysisResult,
        document_path: &Path,
    ) -> bool {
        self.document_path == document_path
            && self.original.file_reference_value().is_some_and(|literal| {
                project_source_association(
                    document_path,
                    &self.workspace_roots,
                    literal,
                    &self.old_path,
                )
                .is_some_and(|association| {
                    association == self.association
                        && association.renamed_literal(&self.new_path).as_deref()
                            == Some(self.literal.as_str())
                })
            })
            && index.source_file_reference_is_current(
                &self.uri,
                self.revision,
                &self.original,
                analysis,
            )
            && self
                .original
                .file_rename_proposal(&self.literal)
                .is_some_and(|proposal| {
                    proposal.span() == self.span && proposal.replacement() == self.new_text
                })
    }
}

/// Apply the explicit script-directory/workspace-root project policy to
/// genuine current FILE-reference source inputs. A unique project association
/// and exact configured literal replacement are required; report path strings,
/// conditional source expressions and guessed ranges are never edit inputs.
#[must_use]
pub fn compute_rename_edits(
    action: &FileRenameSourceAction<'_>,
    index: &WorkspaceIndex,
    document_path: &dyn Fn(&str) -> Option<PathBuf>,
) -> Vec<RenameEdit> {
    let mut edits = Vec::new();
    for source in index.sources() {
        let Some((revision, original)) = index.source_file_reference(source) else {
            continue;
        };
        let Some(path) = document_path(&source.uri) else {
            continue;
        };
        let Some(literal) = original.file_reference_value() else {
            continue;
        };
        let Some(association) =
            project_source_association(&path, action.workspace_roots, literal, action.old_path)
        else {
            continue;
        };
        let Some(literal) = association.renamed_literal(action.new_path) else {
            continue;
        };
        let Some(proposal) = original.file_rename_proposal(&literal) else {
            continue;
        };
        edits.push(RenameEdit {
            uri: source.uri.clone(),
            span: proposal.span(),
            new_text: proposal.replacement().to_owned(),
            revision,
            original: original.clone(),
            literal,
            document_path: path,
            old_path: action.old_path.to_path_buf(),
            new_path: action.new_path.to_path_buf(),
            workspace_roots: action.workspace_roots.to_vec(),
            association,
        });
    }
    edits
}

#[cfg(test)]
mod tests {
    use super::*;
    use tcl_compiler::analyser::Analyser;

    // Preserve the original URI-facing test ingress through the real protocol
    // provider; production receives paths checked by the server URI owner.
    fn uri_to_path(uri: &str) -> Option<String> {
        let uri: tower_lsp_server::ls_types::Uri = uri.parse().ok()?;
        (uri.scheme().as_str() == "file" && uri.query().is_none() && uri.fragment().is_none())
            .then_some(())?;
        let authority = uri
            .authority()
            .map(|authority| authority.as_str())
            .unwrap_or_default();
        if !authority.is_empty() && !(cfg!(unix) && authority.eq_ignore_ascii_case("localhost")) {
            return None;
        }
        let decoded = uri.path().decode().to_string().ok()?;
        (!decoded.contains('\0')).then_some(())?;
        uri.to_file_path()?.to_str().map(str::to_owned)
    }

    fn compute_rename_edits(
        old_uri: &str,
        new_uri: &str,
        index: &WorkspaceIndex,
        roots: &[String],
    ) -> Vec<RenameEdit> {
        let (Some(old), Some(new)) = (uri_to_path(old_uri), uri_to_path(new_uri)) else {
            return Vec::new();
        };
        let Some(roots) = roots
            .iter()
            .map(|root| uri_to_path(root).map(PathBuf::from))
            .collect::<Option<Vec<_>>>()
        else {
            return Vec::new();
        };
        let Some(action) = FileRenameSourceAction::new(Path::new(&old), Path::new(&new), &roots)
        else {
            return Vec::new();
        };
        super::compute_rename_edits(&action, index, &|uri| uri_to_path(uri).map(PathBuf::from))
    }

    fn normpath(path: &str) -> String {
        crate::source_graph::resolve_under(Path::new(""), path)
            .to_str()
            .unwrap()
            .to_owned()
    }

    fn posix_dirname(path: &str) -> &str {
        Path::new(path)
            .parent()
            .and_then(Path::to_str)
            .unwrap_or("")
    }

    fn relpath(target: &str, base: &str) -> String {
        crate::source_graph::relative_source_path(Path::new(target), Path::new(base))
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned()
    }

    fn index_of(uri: &str, src: &str) -> WorkspaceIndex {
        let mut a = Analyser::new();
        let analysis = a.analyse(src, "tcl8.6").clone();
        let mut idx = WorkspaceIndex::new();
        idx.add_document(uri, &analysis);
        idx
    }

    #[test]
    fn rewrites_relative_source_literal() {
        // /proj/main.tcl sources lib/old.tcl → rename to lib/new.tcl
        // rewrites the literal to `lib/new.tcl`.
        let idx = index_of("file:///proj/main.tcl", "source lib/old.tcl\n");
        let edits = compute_rename_edits(
            "file:///proj/lib/old.tcl",
            "file:///proj/lib/new.tcl",
            &idx,
            &[],
        );
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].uri, "file:///proj/main.tcl");
        assert_eq!(edits[0].new_text, "lib/new.tcl");
    }

    #[test]
    fn braced_and_quoted_literals_rewrite_content_only() {
        // A braced `source {lib/old.tcl}` must rewrite only the path
        // content, leaving the `{` / `}` delimiters intact (else the
        // edit would orphan the closing brace).
        for src in [
            "source {lib/old.tcl}\n",
            "source \"lib/old.tcl\"\n",
            "source lib/old.tcl\n",
        ] {
            let idx = index_of("file:///proj/main.tcl", src);
            let edits = compute_rename_edits(
                "file:///proj/lib/old.tcl",
                "file:///proj/lib/new.tcl",
                &idx,
                &[],
            );
            assert_eq!(edits.len(), 1, "{src:?}");
            // The edit's span must cover exactly the old path content.
            let covered = &src[edits[0].span.as_range()];
            assert_eq!(covered, "lib/old.tcl", "span content for {src:?}");
            assert_eq!(edits[0].new_text, "lib/new.tcl", "{src:?}");
            // Applying the edit preserves any surrounding delimiters.
            let mut applied = src.to_string();
            applied.replace_range(edits[0].span.as_range(), &edits[0].new_text);
            let expected = src.replace("lib/old.tcl", "lib/new.tcl");
            assert_eq!(applied, expected, "rewrite for {src:?}");
        }
    }

    #[test]
    fn rewrites_absolute_source_literal_to_new_absolute() {
        let idx = index_of("file:///proj/main.tcl", "source /proj/lib/old.tcl\n");
        let edits = compute_rename_edits(
            "file:///proj/lib/old.tcl",
            "file:///proj/lib/new.tcl",
            &idx,
            &[],
        );
        assert_eq!(edits.len(), 1);
        assert_eq!(edits[0].new_text, "/proj/lib/new.tcl");
    }

    #[test]
    fn ignores_unrelated_and_substituted_sources() {
        // A source of a different file, and a `$var`-substituted source,
        // are both left alone.
        let idx = index_of(
            "file:///proj/main.tcl",
            "source other.tcl\nsource $dynamic\n",
        );
        let edits = compute_rename_edits(
            "file:///proj/lib/old.tcl",
            "file:///proj/lib/new.tcl",
            &idx,
            &[],
        );
        assert!(edits.is_empty());
    }

    #[test]
    fn resolves_relative_source_via_workspace_root() {
        // `/proj/sub/main.tcl` does `source helper.tcl`, but the file lives
        // at the workspace root `/proj/helper.tcl`, not next to the script.
        // Without a workspace root it isn't matched; with one it is, and the
        // rewrite re-relativises against the *same* base it matched under (the
        // root) — so `helper.tcl` → `helper2.tcl`, staying root-relative, not
        // `../helper2.tcl` (which would resolve elsewhere at runtime).
        let idx = index_of("file:///proj/sub/main.tcl", "source helper.tcl\n");
        let no_root = compute_rename_edits(
            "file:///proj/helper.tcl",
            "file:///proj/helper2.tcl",
            &idx,
            &[],
        );
        assert!(no_root.is_empty(), "no root → no match: {no_root:?}");

        let with_root = compute_rename_edits(
            "file:///proj/helper.tcl",
            "file:///proj/helper2.tcl",
            &idx,
            &["file:///proj".to_string()],
        );
        assert_eq!(with_root.len(), 1, "{with_root:?}");
        assert_eq!(with_root[0].new_text, "helper2.tcl");
    }

    #[test]
    fn script_dir_match_still_rewrites_relative_to_script() {
        // When the literal resolves via the script's *own* directory, the
        // rewrite stays script-relative (the base that matched).
        let idx = index_of("file:///proj/sub/main.tcl", "source helper.tcl\n");
        let edits = compute_rename_edits(
            "file:///proj/sub/helper.tcl",
            "file:///proj/sub/renamed.tcl",
            &idx,
            &["file:///proj".to_string()],
        );
        assert_eq!(edits.len(), 1, "{edits:?}");
        assert_eq!(edits[0].new_text, "renamed.tcl");
    }

    #[test]
    fn path_helpers() {
        assert_eq!(normpath("/a/b/../c"), "/a/c");
        assert_eq!(posix_dirname("/a/b/c.tcl"), "/a/b");
        assert_eq!(relpath("/proj/lib/new.tcl", "/proj"), "lib/new.tcl");
        assert_eq!(relpath("/proj/new.tcl", "/proj/sub"), "../new.tcl");
        assert_eq!(uri_to_path("file:///a/b.tcl").as_deref(), Some("/a/b.tcl"));
        assert_eq!(uri_to_path("untitled:foo"), None);
    }
    #[test]
    fn file_action_edits_require_the_original_index_and_analysis_generation() {
        let source = "source {lib/old.tcl}\n";
        let analysis = Analyser::new().analyse(source, "tcl8.6");
        let mut index = WorkspaceIndex::new();
        index.add_document("file:///project/main.tcl", &analysis);
        let edits = compute_rename_edits(
            "file:///project/lib/old.tcl",
            "file:///project/lib/new.tcl",
            &index,
            &[],
        );
        assert_eq!(edits.len(), 1);
        assert!(edits[0].matches_current(&index, &analysis, Path::new("/project/main.tcl")));
        let mut altered = edits[0].clone();
        altered.new_text.push_str("; unknown");
        assert!(!altered.matches_current(&index, &analysis, Path::new("/project/main.tcl")));
        let foreign = Analyser::new().analyse(source, "tcl8.6");
        assert!(!edits[0].matches_current(&index, &foreign, Path::new("/project/main.tcl")));
        index.replace_document("file:///project/main.tcl", &analysis);
        assert!(!edits[0].matches_current(&index, &analysis, Path::new("/project/main.tcl")));
    }
}
