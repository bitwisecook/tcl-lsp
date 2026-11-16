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

//! Resolving literal `source` path arguments and associating them with a declared project base.

use std::path::{Component, Path, PathBuf};

/// Resolve a literal `source` path argument written in `parent_file` to the
/// absolute path it refers to.
///
/// This project association resolves a relative `raw_path` against the
/// directory of `parent_file`; an absolute path passes through unchanged.
/// Tcl itself resolves relative `source` paths against its runtime working
/// directory, which this advisory graph does not establish.
/// The result is lexically normalised —
/// `.` and `..` segments are folded without touching the filesystem, so a
/// symlink is never followed and the call is cheap and deterministic.
#[must_use]
pub fn resolve_source_target(parent_file: &Path, raw_path: &str) -> PathBuf {
    resolve_under(
        parent_file.parent().unwrap_or_else(|| Path::new("")),
        raw_path,
    )
}

/// Associate a genuine original `SourceLoad` path under the declared project
/// script-directory policy. This is neither a reached load nor runtime cwd.
/// Dynamic paths and unsupported host-path forms preserve unknown topology.
#[must_use]
pub fn conditional_project_source_target(
    document: &Path,
    load: &tcl_compiler::registry_invocation::source_structure::OriginalProjectSourceLoad,
) -> Option<PathBuf> {
    let literal = load.literal_path()?;
    if literal.is_empty()
        || literal.starts_with('~')
        || literal.contains('\0')
        || !project_source_action_path(document)
    {
        return None;
    }
    let raw = Path::new(&literal);
    if host_absolute(raw) && !project_source_action_path(raw)
        || !host_absolute(raw) && matches!(raw.components().next(), Some(Component::Prefix(_)))
    {
        return None;
    }
    let target = resolve_source_target(document, &literal);
    project_source_action_path(&target).then_some(target)
}

/// Resolve `raw_path` relative to the directory `base_dir` (absolute paths pass
/// through), lexically normalised.  Used both for a `source` target (relative
/// to the sourcing file's directory) and for a configured project entry-point
/// path (relative to the workspace folder root).
#[must_use]
pub fn resolve_under(base_dir: &Path, raw_path: &str) -> PathBuf {
    let raw = Path::new(raw_path);
    let joined = if host_absolute(raw) {
        raw.to_path_buf()
    } else {
        base_dir.join(raw)
    };
    lexically_normalise(&joined)
}

/// Fold `.` and `..` segments lexically (no filesystem access). A leading `..`
/// with nothing to pop is preserved so a path that escapes its root still
/// resolves to a stable, comparable form.
fn lexically_normalise(path: &Path) -> PathBuf {
    let mut out: Vec<Component<'_>> = Vec::new();
    for comp in path.components() {
        match comp {
            Component::CurDir => {}
            Component::ParentDir => match out.last() {
                Some(Component::Normal(_)) => {
                    out.pop();
                }
                _ => out.push(comp),
            },
            other => out.push(other),
        }
    }
    if out.is_empty() {
        return PathBuf::from(".");
    }
    out.iter().collect()
}

/// The unique declared project base associating a literal source reference
/// with a host rename. This is an intentional source-action policy, not an
/// observation of Tcl's working directory, file loading or normal completion.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSourceAssociation {
    base: Option<PathBuf>,
}

impl ProjectSourceAssociation {
    /// Preserve absolute/relative style under this same declared project base.
    #[must_use]
    pub fn renamed_literal(&self, new_path: &Path) -> Option<String> {
        if !project_source_action_path(new_path) {
            return None;
        }
        let target = lexically_normalise(new_path);
        let value = match &self.base {
            None => target,
            Some(base) => relative_source_path(&target, base)?,
        };
        let text = value.to_str()?;
        #[cfg(windows)]
        let text = text.replace('\\', "/");
        Some(text.to_owned())
    }
}

/// Associate a whole literal FILE operand under the explicitly declared
/// script-directory/workspace-root project policy. Multiple distinct matching
/// bases refuse; candidate filesystem existence is never inferred here.
#[must_use]
pub fn project_source_association(
    document: &Path,
    workspace_roots: &[PathBuf],
    literal: &str,
    old_path: &Path,
) -> Option<ProjectSourceAssociation> {
    // Home expansion and drive-relative bases have no project-path issuer.
    if literal.is_empty()
        || literal.starts_with('~')
        || literal.contains('\0')
        || !project_source_action_path(document)
        || !project_source_action_path(old_path)
    {
        return None;
    }
    let old = lexically_normalise(old_path);
    let raw = Path::new(literal);
    if host_absolute(raw) {
        project_source_action_path(raw).then_some(())?;
        return (lexically_normalise(raw) == old)
            .then_some(ProjectSourceAssociation { base: None });
    }
    if matches!(raw.components().next(), Some(Component::Prefix(_))) {
        return None;
    }
    let mut bases = Vec::new();
    for base in
        std::iter::once(document.parent()?).chain(workspace_roots.iter().map(PathBuf::as_path))
    {
        if !project_source_action_path(base) {
            return None;
        }
        let base = lexically_normalise(base);
        if resolve_under(&base, literal) == old && !bases.contains(&base) {
            bases.push(base);
        }
    }
    let [base] = bases.as_slice() else {
        return None;
    };
    Some(ProjectSourceAssociation {
        base: Some(base.clone()),
    })
}

/// `Path::is_absolute`, plus a rooted path in a browser worker
/// (wasm32-unknown-unknown), where std reports no path as absolute and every
/// source-store path is a rooted virtual path.
fn host_absolute(path: &Path) -> bool {
    path.is_absolute()
        || cfg!(all(target_family = "wasm", target_os = "unknown")) && path.has_root()
}

fn project_source_action_path(path: &Path) -> bool {
    // POSIX double-leading slash may select an implementation-defined root.
    host_absolute(path)
        && path
            .to_str()
            .is_some_and(|text| !(text.contains('\0') || cfg!(unix) && text.starts_with("//")))
}

/// Relative spelling in the same host path domain; different roots decline.
#[must_use]
pub fn relative_source_path(target: &Path, base: &Path) -> Option<PathBuf> {
    if !host_absolute(target) || !host_absolute(base) {
        return None;
    }
    let target = lexically_normalise(target);
    let base = lexically_normalise(base);
    let target: Vec<_> = target.components().collect();
    let base: Vec<_> = base.components().collect();
    if target.first() != base.first() {
        return None;
    }
    let common = target
        .iter()
        .zip(&base)
        .take_while(|(left, right)| left == right)
        .count();
    if base[common..]
        .iter()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return None;
    }
    let mut result = PathBuf::new();
    for _ in &base[common..] {
        result.push("..");
    }
    for part in &target[common..] {
        result.push(part.as_os_str());
    }
    if result.as_os_str().is_empty() {
        result.push(".");
    }
    Some(result)
}
