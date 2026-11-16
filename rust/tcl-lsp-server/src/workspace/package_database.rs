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

//! The package resolver and the `auto_path` it searches.

use super::*;

/// Build the package database (`pkgIndex.tcl` / `tclIndex`) the W120 refinement
/// consults: the workspace trees (recursive — an IDE nests packages
/// arbitrarily) plus the resolved `auto_path` ([`effective_auto_path`]) scanned
/// with C Tcl's immediate-subdir rule. Pure filesystem work, so it runs on the
/// scan worker.
pub(in super::super) fn build_package_resolver(
    store: &dyn vfs::SourceStore,
    roots: &[PathBuf],
    editor_library_paths: &[String],
    discovered: &[core_tcl_install::TclInstallation],
    dir_cap: usize,
) -> PackageResolver {
    let mut resolver = PackageResolver::new();
    for root in roots {
        workspace_scan_trace!("package.tree.begin root={root:?} dir_cap={dir_cap}");
        resolver.scan_tree_in(store, root, dir_cap);
        workspace_scan_trace!("package.tree.end root={root:?}");
    }
    workspace_scan_trace!("package.auto_path.begin");
    for dir in effective_auto_path(store, roots, editor_library_paths, discovered) {
        workspace_scan_trace!("package.path.begin path={dir:?}");
        resolver.scan_path_in(store, &dir);
        workspace_scan_trace!("package.path.end path={dir:?}");
    }
    resolver
}

/// Cap on how many distinct `auto_path` directories the workspace's own files
/// may contribute to the package database.  Each one costs a
/// [`PackageResolver::scan_path`] (one directory plus its immediate
/// subdirectories, and already-scanned directories are free), so the bound is
/// generous — but a generated file that appends in a loop must not turn the
/// startup scan into an unbounded filesystem walk.
const DOCUMENT_AUTO_PATH_DIR_CAP: usize = 64;

/// Fold the `auto_path` mutations the workspace's own files perform into
/// `resolver`'s search path.
///
/// Real Tcl resolves `package require` against the `auto_path` **as the
/// program builds it**, and the near-universal idiom for a repo-local package
/// is for a script to add its own tree:
///
/// ```tcl
/// lappend auto_path [file dirname [file dirname [info script]]]
/// package require mypix
/// ```
///
/// A resolver seeing only configured roots and `libraryPaths` would answer
/// this *by luck* — a workspace root enclosing the package directory gets
/// swept up by [`PackageResolver::scan_tree`]'s blunt recursive descent — and
/// fail outright when it does not (opening just `examples/`, or a single
/// file): go-to-definition and hover on a command the package provides
/// answered nothing, though tclsh resolves it deterministically.
///
/// Only **statically resolvable** entries contribute:
/// [`tcl_compiler::auto_path_eval::evaluate_auto_path_expr`] folds literals,
/// `~`, `[info script]`, `[file dirname …]`, and `[file join …]` — the exact
/// evaluator the `source`-graph tier (`resolve_source_edge`) already trusts —
/// and returns `None` for anything else, including any `$var`.  A dynamic
/// mutation therefore adds nothing here; it stays handled the established way,
/// by [`AnalysisResult::has_dynamic_providers`] widening the document's
/// command availability to conditional so the unknown-command pass abstains
/// rather than guessing.
///
/// Cost is one pass over the already-computed per-file analyses, bounded by
/// [`DOCUMENT_AUTO_PATH_DIR_CAP`] directories; it runs once per workspace
/// scan, never per request.
pub(super) fn extend_resolver_with_document_auto_paths(
    store: &dyn vfs::SourceStore,
    mut resolver: PackageResolver,
    analysed: &[(Uri, String, String, AnalysisResult)],
) -> PackageResolver {
    let mut dirs: Vec<PathBuf> = Vec::new();
    for (uri, _, _, analysis) in analysed {
        for dir in document_auto_path_dirs(uri, analysis) {
            if dirs.len() >= DOCUMENT_AUTO_PATH_DIR_CAP {
                break;
            }
            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
    }
    for dir in &dirs {
        resolver.scan_path_in(store, dir);
    }
    resolver
}

/// Candidate package directories under the same actual retained analysis.
/// The selected mutation and genuine whole argument own list parsing:
/// assignment splits the completed value; list append keeps one element.
/// Unknown/foreign source, local receiver ambiguity and unsupported operations
/// contribute no directories. Relative advice is anchored on the explicit
/// document directory, independently of the interpreter's unknown cwd.
pub(in super::super) fn document_auto_path_dirs(
    uri: &Uri,
    analysis: &AnalysisResult,
) -> Vec<PathBuf> {
    if analysis.auto_path_entries.is_empty() {
        return Vec::new();
    }
    let Some(file_path) = uri.document_path() else {
        return Vec::new();
    };
    // The document's own single-assignment constants, chain-folded, so the
    // corpus idiom `set libDir [file join $dir lib]; lappend auto_path
    // $libDir` contributes its directory instead of nothing.
    let constants = tcl_compiler::auto_path_eval::fold_constant_assignments(
        &analysis.path_constant_assignments,
        file_path.to_str(),
    );
    let mut dirs: Vec<PathBuf> = Vec::new();
    for entry in &analysis.auto_path_entries {
        if dirs.len() >= DOCUMENT_AUTO_PATH_DIR_CAP {
            break;
        }
        let Some(expression) =
            tcl_compiler::auto_path_eval::capture_source_auto_path_expression(analysis, entry)
        else {
            continue;
        };
        let Some(values) = expression.evaluate(file_path.to_str(), &|name| {
            tcl_compiler::auto_path_eval::PathConstantLookup::path_constant(
                &constants.at(entry.range.start()),
                name,
            )
        }) else {
            continue;
        };
        for folded in values {
            if dirs.len() >= DOCUMENT_AUTO_PATH_DIR_CAP {
                break;
            }
            // Anchor a rootless fold on the document's own directory (see
            // this function's doc comment); an already-rooted one passes
            // through unchanged.
            let dir = tcl_lsp_core::source_graph::resolve_source_target(&file_path, &folded);
            if !dirs.contains(&dir) {
                dirs.push(dir);
            }
        }
    }
    dirs
}

/// The effective `auto_path` for the package database, layering the configured
/// sources plus on-disk discovery (deduped, in priority order):
///
/// 1. editor `tclLsp.libraryPaths`,
/// 2. user config `config.ini` `[global] libraryPaths`,
/// 3. per-workspace `.tcl-lsp.ini` `[project] libraryPaths`,
/// 4. discovered Tcl installations' `auto_path`,
/// 5. `TCLLIBPATH`.
pub(in super::super) fn effective_auto_path(
    store: &dyn vfs::SourceStore,
    roots: &[PathBuf],
    editor_library_paths: &[String],
    discovered: &[core_tcl_install::TclInstallation],
) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = Vec::new();
    let push = |p: PathBuf, out: &mut Vec<PathBuf>| {
        if !out.contains(&p) {
            out.push(p);
        }
    };
    for p in editor_library_paths {
        push(PathBuf::from(p), &mut out);
    }
    if let Some(cfg) = core_tcl_install::user_config_path()
        && let Ok(content) = store.read_to_string(&cfg)
    {
        for p in core_tcl_install::library_paths_from_ini(&content, "global") {
            push(PathBuf::from(p), &mut out);
        }
    }
    for root in roots {
        let proj = core_tcl_install::project_config_path(root);
        if let Ok(content) = store.read_to_string(&proj) {
            for p in core_tcl_install::library_paths_from_ini(&content, "project") {
                push(PathBuf::from(p), &mut out);
            }
        }
    }
    for inst in discovered {
        for p in &inst.auto_path {
            push(p.clone(), &mut out);
        }
    }
    for p in core_tcl_install::tcllibpath_dirs() {
        push(p, &mut out);
    }
    out
}
